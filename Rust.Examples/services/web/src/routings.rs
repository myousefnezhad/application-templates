use crate::handlers::{
    agent::*, customer::*, google::*, index::*, knowledge_based::*, login::*, ping::*, user::*,
};
use app_middleware::web_auth_middleware;
use app_state::AppState;
use axum::{
    Router, middleware,
    routing::{get, patch, post},
};
use std::{sync::Arc};
use tower_http::{services::ServeDir, trace::TraceLayer};
use utoipa::{OpenApi, openapi::security::{
        HttpAuthScheme,
        HttpBuilder,
        SecurityScheme,
    },Modify};
use utoipa_swagger_ui::SwaggerUi;
use app_dto::auth::{login::{PostLoginInput, PostLoginOutput},user::{PatchChangePasswordInput, PostUserInput}};
use app_schema::{customer::Customer, kb::KnowledgeBased, auth::users::User};
use app_log::LogLevel;
use tracing::debug;

pub fn router(state: Arc<AppState>) -> Router {
    let log_level = state.config.log_level.clone();
    let asset_path = state.config.asset_path.clone();
    let asset_service = ServeDir::new(&asset_path).append_index_html_on_directories(true);

    let mut router = Router::new()
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
        .with_state(state);

    if log_level == LogLevel::Debug {
        router = router.merge(
            SwaggerUi::new("/swagger-ui")
                .url("/api-docs/openapi.json", ApiDoc::openapi())
        );
        debug!("Swagger UI enabled at /swagger-ui");
    }
    router
}


#[derive(OpenApi)]
#[openapi(
    paths(
        post_login,
        post_logout,
        post_customer,
        post_kb,
        ping,
        get_user,
        post_user,
        patch_change_password,
        post_agent,
    ),
    components(
        schemas(
            PostLoginInput,
            PostLoginOutput,
            Customer,
            KnowledgeBased,
            User,
            PatchChangePasswordInput,
            PostUserInput,
            ChatPostInput,
            ChatPostOutput,
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "Authentication", description = "Authentication related endpoints"),
        (name = "User Management", description = "User management related endpoints"),
        (name = "Customer", description = "Customer related endpoints"),
        (name = "KnowledgeBased", description = "Knowledge based related endpoints"),
        (name = "Agent", description = "Agent related endpoints"),
    )
)]
struct ApiDoc;

pub struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .description(Some("JWT <access token> <refresh token>"))
                        .build(),
                ),
            );
        }
    }
}