use crate::handlers::{
    agent::*, customer::*, google::*, index::*, knowledge_based::*, login::*, ping::*, user::*,
};
use app_middleware::web_auth_middleware;
use app_state::AppState;
use axum::{
    Router, middleware,
    routing::{get, patch, post},
};
use std::sync::Arc;
use tower_http::{services::ServeDir, trace::TraceLayer};

pub fn router(state: Arc<AppState>) -> Router {
    let asset_path = state.config.asset_path.clone();
    let asset_service = ServeDir::new(&asset_path).append_index_html_on_directories(true);

    Router::new()
        .route("/", get(get_index).post(post_index))
        .route("/customer", get(get_customer))
        .route("/kb", get(get_kb))
        .route("/agent", get(get_agent))
        .route("/login", get(get_login).post(post_login))
        .route("/ping", get(ping).post(ping))
        .nest(
            "/google",
            Router::new()
                .route("/auth", get(get_google_auth))
                .route("/callback", get(get_google_callback))
                .route("/token", get(page_google_token)),
        )
        // .layer(TraceLayer::new_for_http())
        // .with_state(state.clone())
        .nest(
            "/auth",
            Router::new()
                .route("/ping", get(ping).post(ping))
                .route("/logout", post(post_logout))
                .route("/user", get(get_user).post(post_user))
                .route("/change_password", patch(patch_change_password))
                .route("/customer", post(post_customer))
                .route("/kb", post(post_kb))
                .route("/agent", post(post_agent))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    web_auth_middleware,
                )),
        )
        .nest_service("/assets", asset_service)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
