use app_error::AppError;
use app_schema::kb::KnowledgeBased;
use app_state::AppState;
use askama::Template;
use axum::extract::{Json, State};
use axum::response::Html;
use std::sync::Arc;
use tracing::*;

#[derive(Template)]
#[template(path = "kb.html")]
struct KbPage;

pub async fn get_kb() -> Result<Html<String>, AppError> {
    let page = KbPage;
    Ok(Html(page.render()?))
}

pub async fn post_kb(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<KnowledgeBased>>, AppError> {
    let pg = state.pg.clone();
    let res = sqlx::query_as::<_, KnowledgeBased>(&KnowledgeBased::select_base())
        .fetch_all(&pg)
        .await?;
    debug!("{:#?}", &res);
    Ok(Json(res))
}
