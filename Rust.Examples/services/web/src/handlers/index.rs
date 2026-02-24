use app_error::AppError;
use askama::Template;
use axum::response::Html;

#[derive(Template)]
#[template(path = "index.html")]
struct IndexPage;

pub async fn get_index() -> Result<Html<String>, AppError> {
    let page = IndexPage;
    Ok(Html(page.render()?))
}

pub async fn post_index() -> Result<String, AppError> {
    Err(AppError::new(
        "Test",
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        1,
    ))
}
