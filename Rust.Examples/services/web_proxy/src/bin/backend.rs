use axum::{
    Json, Router,
    extract::{Path, Query, Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use web_proxy::{prompt_port, prompt_text};

#[derive(Clone)]
struct AppState {
    service_name: Arc<String>,
    port: u16,
}

#[derive(Debug, Deserialize)]
struct MessageRequest {
    message: String,
}

#[derive(Debug, Serialize)]
struct MessageResponse {
    service: String,
    message: String,
}

#[tokio::main]
async fn main() {
    // Always asks for a port.
    // BACKEND_PORT changes the suggested default.
    let port = prompt_port("BACKEND_PORT", "backend", 4000);

    // Always asks for the service name.
    // SERVICE_NAME changes the suggested default.
    let service_name = prompt_text(
        "SERVICE_NAME",
        "Enter backend service name",
        "backend-service",
    );

    let state = AppState {
        service_name: Arc::new(service_name),
        port,
    };

    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/hello", get(hello))
        .route("/hello/{name}", get(hello_name))
        .route("/message", post(create_message))
        .route("/echo", post(echo))
        .route("/headers", get(show_headers))
        .layer(middleware::from_fn(log_request))
        .with_state(state);

    let address = format!("0.0.0.0:{port}");

    let listener = tokio::net::TcpListener::bind(&address)
        .await
        .unwrap_or_else(|error| panic!("Failed to bind backend to {address}: {error}"));

    println!();
    println!("Backend service started");
    println!("Service: {}", listener.local_addr().unwrap());
    println!("Name: {}", state_for_display(&listener));
    println!("URL: http://127.0.0.1:{port}");
    println!();

    axum::serve(listener, app)
        .await
        .expect("backend server failed");
}

// This function exists only to keep the startup output simple.
fn state_for_display(_listener: &tokio::net::TcpListener) -> &'static str {
    "configured backend"
}

async fn log_request(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let started = Instant::now();

    let response = next.run(request).await;

    println!(
        "[BACKEND] {} {} -> {} | {:.2} ms",
        method,
        path,
        response.status().as_u16(),
        started.elapsed().as_secs_f64() * 1000.0,
    );

    response
}

async fn root(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "service": state.service_name.as_str(),
        "port": state.port,
        "message": "Backend service is running",
        "endpoints": [
            "GET /",
            "GET /health",
            "GET /hello",
            "GET /hello/{name}",
            "POST /message",
            "POST /echo",
            "GET /headers"
        ]
    }))
}

async fn health(State(state): State<AppState>) -> Json<Value> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    Json(json!({
        "service": state.service_name.as_str(),
        "status": "healthy",
        "port": state.port,
        "timestamp": timestamp
    }))
}

async fn hello(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "service": state.service_name.as_str(),
        "message": "Hello from the backend"
    }))
}

async fn hello_name(State(state): State<AppState>, Path(name): Path<String>) -> Json<Value> {
    Json(json!({
        "service": state.service_name.as_str(),
        "message": format!("Hello, {name}!")
    }))
}

async fn create_message(
    State(state): State<AppState>,
    Json(payload): Json<MessageRequest>,
) -> (StatusCode, Json<MessageResponse>) {
    (
        StatusCode::CREATED,
        Json(MessageResponse {
            service: state.service_name.to_string(),
            message: payload.message,
        }),
    )
}

async fn echo(State(state): State<AppState>, Json(payload): Json<Value>) -> Json<Value> {
    Json(json!({
        "service": state.service_name.as_str(),
        "received": payload
    }))
}

async fn show_headers(
    State(state): State<AppState>,
    Query(query): Query<HashMap<String, String>>,
    headers: axum::http::HeaderMap,
) -> Json<Value> {
    let header_values: HashMap<String, String> = headers
        .iter()
        .map(|(name, value)| {
            (
                name.to_string(),
                value.to_str().unwrap_or("<binary value>").to_string(),
            )
        })
        .collect();

    Json(json!({
        "service": state.service_name.as_str(),
        "query": query,
        "headers": header_values
    }))
}
