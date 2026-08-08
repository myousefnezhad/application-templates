#[utoipa::path(
    post,
    path = "/auth/ping",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "pong"),
        (status = 401, description = "Unauthorized access"),
    ),
    tag = "Authentication"
)]
pub async fn ping() -> String {
    "pong".to_string()
}
