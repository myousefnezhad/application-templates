use axum::{
    Json, Router,
    body::{Body, to_bytes},
    extract::{Path, State},
    http::{
        Request, Response, StatusCode,
        header::{self, HeaderName},
    },
    response::IntoResponse,
    routing::{any, delete, get, post},
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    io::{self, Write},
    sync::Arc,
    time::Duration,
};
use tokio::sync::RwLock;
use web_proxy::prompt_port;

const MAX_REQUEST_BODY_SIZE: usize = 10 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Backend {
    /// Unique backend name.
    ///
    /// Example: users-service
    name: String,

    /// Public prefix exposed by the proxy.
    ///
    /// Example: /users
    public_prefix: String,

    /// Backend base address.
    ///
    /// Example: http://127.0.0.1:4000
    upstream_url: String,
}

#[derive(Clone)]
struct AppState {
    client: Client,
    backends: Arc<RwLock<Vec<Backend>>>,
}

#[tokio::main]
async fn main() {
    // Always asks for a port.
    // PROXY_PORT changes the suggested default.
    let port = prompt_port("PROXY_PORT", "proxy", 3000);

    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("failed to create HTTP client");

    // The proxy always starts with an empty routing table.
    let backends = Arc::new(RwLock::new(Vec::<Backend>::new()));

    let state = AppState {
        client,
        backends: Arc::clone(&backends),
    };

    let app = Router::new()
        // Proxy management endpoints
        .route("/health", get(proxy_health))
        .route("/backends", get(list_backends))
        .route("/backends", post(add_backend))
        .route("/backends/{name}", delete(remove_backend))
        // All other requests are checked against the backend list
        .fallback(any(proxy_request))
        .with_state(state);

    let address = format!("0.0.0.0:{port}");

    let listener = tokio::net::TcpListener::bind(&address)
        .await
        .unwrap_or_else(|error| panic!("Failed to bind proxy to {address}: {error}"));

    println!();
    println!("Proxy started at http://127.0.0.1:{port}");
    println!("The proxy currently has no registered backends.");
    println!();

    // Run the terminal menu on a blocking thread so it does not
    // block the Axum/Tokio server.
    let menu_backends = Arc::clone(&backends);

    tokio::task::spawn_blocking(move || {
        run_proxy_menu(menu_backends);
    });

    axum::serve(listener, app)
        .await
        .expect("proxy server failed");
}

/* ---------------------------------------------------------
   HTTP management endpoints
--------------------------------------------------------- */

async fn proxy_health(State(state): State<AppState>) -> Json<serde_json::Value> {
    let backends = state.backends.read().await;

    Json(json!({
        "status": "healthy",
        "backend_count": backends.len()
    }))
}

async fn list_backends(State(state): State<AppState>) -> Json<Vec<Backend>> {
    let backends = state.backends.read().await;
    Json(backends.clone())
}

async fn add_backend(
    State(state): State<AppState>,
    Json(mut backend): Json<Backend>,
) -> Result<(StatusCode, Json<Backend>), ProxyError> {
    validate_and_normalize_backend(&mut backend).map_err(ProxyError::BadRequest)?;

    let mut backends = state.backends.write().await;

    if backends.iter().any(|current| current.name == backend.name) {
        return Err(ProxyError::Conflict(format!(
            "Backend '{}' already exists",
            backend.name
        )));
    }

    if backends
        .iter()
        .any(|current| current.public_prefix == backend.public_prefix)
    {
        return Err(ProxyError::Conflict(format!(
            "Public prefix '{}' is already registered",
            backend.public_prefix
        )));
    }

    backends.push(backend.clone());

    sort_backends(&mut backends);

    println!();
    println!(
        "HTTP: added '{}' mapping {} -> {}",
        backend.name, backend.public_prefix, backend.upstream_url
    );

    Ok((StatusCode::CREATED, Json(backend)))
}

async fn remove_backend(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, ProxyError> {
    let mut backends = state.backends.write().await;
    let previous_length = backends.len();

    backends.retain(|backend| backend.name != name);

    if backends.len() == previous_length {
        return Err(ProxyError::NotFound(format!(
            "Backend '{name}' was not found"
        )));
    }

    println!();
    println!("HTTP: removed backend '{name}'");

    Ok(Json(json!({
        "message": "Backend removed",
        "name": name
    })))
}

/* ---------------------------------------------------------
   Dynamic proxy
--------------------------------------------------------- */

async fn proxy_request(
    State(state): State<AppState>,
    request: Request<Body>,
) -> Result<Response<Body>, ProxyError> {
    let request_path = request.uri().path();

    let backend = {
        let backends = state.backends.read().await;

        backends
            .iter()
            .filter(|backend| prefix_matches(request_path, &backend.public_prefix))
            .max_by_key(|backend| backend.public_prefix.len())
            .cloned()
    };

    let backend = backend.ok_or_else(|| {
        ProxyError::NotFound(format!(
            "No backend is configured for path '{request_path}'"
        ))
    })?;

    forward_request(state, backend, request).await
}

fn prefix_matches(path: &str, prefix: &str) -> bool {
    if prefix == "/" {
        return true;
    }

    path == prefix
        || path
            .strip_prefix(prefix)
            .is_some_and(|remaining| remaining.starts_with('/'))
}

async fn forward_request(
    state: AppState,
    backend: Backend,
    request: Request<Body>,
) -> Result<Response<Body>, ProxyError> {
    let (parts, body) = request.into_parts();

    let remaining_path = if backend.public_prefix == "/" {
        parts.uri.path()
    } else {
        parts
            .uri
            .path()
            .strip_prefix(&backend.public_prefix)
            .unwrap_or(parts.uri.path())
    };

    let upstream_path = if remaining_path.is_empty() {
        "/"
    } else if remaining_path.starts_with('/') {
        remaining_path
    } else {
        return Err(ProxyError::InternalServerError(
            "Generated upstream path is invalid".to_string(),
        ));
    };

    let mut destination = format!("{}{}", backend.upstream_url, upstream_path);

    if let Some(query) = parts.uri.query() {
        destination.push('?');
        destination.push_str(query);
    }

    println!();
    println!("PROXY: {} {} -> {}", parts.method, parts.uri, destination);

    let request_body = to_bytes(body, MAX_REQUEST_BODY_SIZE)
        .await
        .map_err(|error| ProxyError::BadRequest(format!("Cannot read request body: {error}")))?;

    let mut upstream_request = state
        .client
        .request(parts.method, &destination)
        .body(request_body);

    for (name, value) in &parts.headers {
        if should_forward_request_header(name) {
            upstream_request = upstream_request.header(name, value);
        }
    }

    upstream_request = upstream_request
        .header("x-forwarded-proto", "http")
        .header("x-proxy-backend", &backend.name);

    let upstream_response = upstream_request.send().await.map_err(|error| {
        ProxyError::BadGateway(format!(
            "Cannot connect to backend '{}': {error}",
            backend.name
        ))
    })?;

    let status = upstream_response.status();
    let response_headers = upstream_response.headers().clone();

    let response_body = upstream_response.bytes().await.map_err(|error| {
        ProxyError::BadGateway(format!(
            "Cannot read response from backend '{}': {error}",
            backend.name
        ))
    })?;

    let mut response_builder = Response::builder().status(status);

    for (name, value) in &response_headers {
        if should_forward_response_header(name) {
            response_builder = response_builder.header(name, value);
        }
    }

    response_builder
        .header("x-proxy-backend", backend.name)
        .body(Body::from(response_body))
        .map_err(|error| ProxyError::InternalServerError(error.to_string()))
}

/* ---------------------------------------------------------
   Terminal menu
--------------------------------------------------------- */

fn run_proxy_menu(backends: Arc<RwLock<Vec<Backend>>>) {
    show_menu();

    loop {
        let choice = prompt_menu_value("proxy> ");

        match choice.to_lowercase().as_str() {
            "1" | "add" => {
                menu_add_backend(&backends);
            }

            "2" | "delete" | "remove" => {
                menu_delete_backend(&backends);
            }

            "3" | "show" | "list" => {
                menu_show_backends(&backends);
            }

            "4" | "menu" | "help" => {
                show_menu();
            }

            "0" | "hide" => {
                println!("Menu hidden. Type 'menu' to display it again.");
            }

            "" => {}

            _ => {
                println!("Unknown command. Type 'menu' for help.");
            }
        }
    }
}

fn show_menu() {
    println!();
    println!("==============================");
    println!(" Proxy management menu");
    println!("==============================");
    println!("1. Add backend");
    println!("2. Delete backend");
    println!("3. Show backends");
    println!("4. Show menu");
    println!("0. Hide menu");
    println!();
}

fn prompt_menu_value(message: &str) -> String {
    print!("{message}");

    io::stdout().flush().expect("failed to flush stdout");

    let mut input = String::new();

    io::stdin()
        .read_line(&mut input)
        .expect("failed to read terminal input");

    input.trim().to_string()
}

fn menu_add_backend(backends: &Arc<RwLock<Vec<Backend>>>) {
    println!();
    println!("Add backend");

    let name = prompt_menu_value("Backend name: ");

    let public_prefix = prompt_menu_value("Public prefix, for example /users: ");

    let upstream_url = prompt_menu_value("Upstream URL, for example http://127.0.0.1:4000: ");

    let mut backend = Backend {
        name,
        public_prefix,
        upstream_url,
    };

    if let Err(message) = validate_and_normalize_backend(&mut backend) {
        println!("Cannot add backend: {message}");
        return;
    }

    let mut current_backends = backends.blocking_write();

    if current_backends
        .iter()
        .any(|current| current.name == backend.name)
    {
        println!("Backend '{}' already exists.", backend.name);

        return;
    }

    if current_backends
        .iter()
        .any(|current| current.public_prefix == backend.public_prefix)
    {
        println!("Prefix '{}' is already registered.", backend.public_prefix);

        return;
    }

    current_backends.push(backend.clone());
    sort_backends(&mut current_backends);

    println!(
        "Added '{}' mapping {} -> {}",
        backend.name, backend.public_prefix, backend.upstream_url
    );
}

fn menu_delete_backend(backends: &Arc<RwLock<Vec<Backend>>>) {
    let name = prompt_menu_value("Backend name to delete: ");

    let mut current_backends = backends.blocking_write();

    let previous_length = current_backends.len();

    current_backends.retain(|backend| backend.name != name);

    if current_backends.len() == previous_length {
        println!("Backend '{name}' was not found.");
    } else {
        println!("Backend '{name}' was removed.");
    }
}

fn menu_show_backends(backends: &Arc<RwLock<Vec<Backend>>>) {
    let current_backends = backends.blocking_read();

    println!();

    if current_backends.is_empty() {
        println!("No backends are currently registered.");
        return;
    }

    println!("Registered backends:");

    for backend in current_backends.iter() {
        println!(
            "- {}: {} -> {}",
            backend.name, backend.public_prefix, backend.upstream_url
        );
    }
}

/* ---------------------------------------------------------
   Backend validation
--------------------------------------------------------- */

fn validate_and_normalize_backend(backend: &mut Backend) -> Result<(), String> {
    backend.name = backend.name.trim().to_string();

    backend.public_prefix = backend.public_prefix.trim().to_string();

    backend.upstream_url = backend.upstream_url.trim().to_string();

    if backend.name.is_empty() {
        return Err("backend name cannot be empty".to_string());
    }

    if !backend.public_prefix.starts_with('/') {
        return Err("public prefix must start with '/'".to_string());
    }

    if !backend.upstream_url.starts_with("http://") && !backend.upstream_url.starts_with("https://")
    {
        return Err("upstream URL must start with http:// or https://".to_string());
    }

    if backend.public_prefix != "/" {
        backend.public_prefix = backend.public_prefix.trim_end_matches('/').to_string();
    }

    backend.upstream_url = backend.upstream_url.trim_end_matches('/').to_string();

    Ok(())
}

fn sort_backends(backends: &mut [Backend]) {
    backends.sort_by(|left, right| right.public_prefix.len().cmp(&left.public_prefix.len()));
}

/* ---------------------------------------------------------
   HTTP headers
--------------------------------------------------------- */

fn should_forward_request_header(name: &HeaderName) -> bool {
    name != header::HOST && !is_hop_by_hop_header(name)
}

fn should_forward_response_header(name: &HeaderName) -> bool {
    !is_hop_by_hop_header(name)
}

fn is_hop_by_hop_header(name: &HeaderName) -> bool {
    matches!(
        name.as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
    )
}

/* ---------------------------------------------------------
   Proxy errors
--------------------------------------------------------- */

enum ProxyError {
    BadRequest(String),
    Conflict(String),
    NotFound(String),
    BadGateway(String),
    InternalServerError(String),
}

impl IntoResponse for ProxyError {
    fn into_response(self) -> Response<Body> {
        let (status, error) = match self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, message),

            Self::Conflict(message) => (StatusCode::CONFLICT, message),

            Self::NotFound(message) => (StatusCode::NOT_FOUND, message),

            Self::BadGateway(message) => (StatusCode::BAD_GATEWAY, message),

            Self::InternalServerError(message) => (StatusCode::INTERNAL_SERVER_ERROR, message),
        };

        (
            status,
            Json(json!({
                "status": status.as_u16(),
                "error": error
            })),
        )
            .into_response()
    }
}
