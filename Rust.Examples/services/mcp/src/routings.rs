use crate::handlers::McpHandler;
use app_middleware::mcp_auth_middleware;
use app_state::AppState;
use axum::{Router, middleware, routing::get};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::SessionManager,
};
use std::sync::Arc;
use tower_http::trace::TraceLayer;

pub fn router<M>(
    state: Arc<AppState>,
    session: Arc<M>,
    mcp_config: StreamableHttpServerConfig,
) -> Router
where
    M: SessionManager,
{
    let service = StreamableHttpService::new(
        {
            let state = state.clone();
            move || Ok(McpHandler::new(state.clone()))
        },
        session,
        mcp_config,
    );

    Router::new()
        .route("/", get(index))
        .nest(
            "/v1",
            axum::Router::new().nest_service("/mcp", service).layer(
                middleware::from_fn_with_state(state.clone(), mcp_auth_middleware),
            ),
        )
        .layer(TraceLayer::new_for_http())
}

async fn index() -> String {
    "[ok]".to_owned()
}
