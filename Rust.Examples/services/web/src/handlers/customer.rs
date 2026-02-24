use app_error::AppError;
use app_schema::customer::Customer;
use app_state::AppState;
use askama::Template;
use axum::extract::{Json, State};
use axum::response::Html;
use std::sync::Arc;
use tracing::*;

#[derive(Template)]
#[template(path = "customer.html")]
struct CustomerPage;

pub async fn get_customer() -> Result<Html<String>, AppError> {
    let page = CustomerPage;
    Ok(Html(page.render()?))
}

pub async fn post_customer(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Customer>>, AppError> {
    let pg = state.pg.clone();
    let res = sqlx::query_as::<_, Customer>(&Customer::select_base())
        .fetch_all(&pg)
        .await?;
    debug!("{:#?}", &res);
    Ok(Json(res))
}
