use app_cryptography::hash::{hash, verify};
use app_dto::auth::user::{PatchChangePasswordInput, PostUserInput};
use app_error::AppError;
use app_middleware::get_session;
use app_redis::Redis;
use app_schema::auth::users::User;
use app_state::AppState;
use axum::{
    extract::{Json, State},
    http::{HeaderMap, StatusCode},
};
use std::sync::Arc;
use tracing::*;

pub async fn get_user(State(state): State<Arc<AppState>>) -> Result<Json<Vec<User>>, AppError> {
    let pg = state.pg.clone();
    let res = sqlx::query_as::<_, User>(&User::select_query())
        .fetch_all(&pg)
        .await?;
    debug!("{:?}", &res);
    Ok(Json(res))
}

const AUTH_FAILD_MESSAGE: &'static str = "Provided information is wrong!";

pub async fn post_user(
    State(state): State<Arc<AppState>>,
    Json(args): Json<PostUserInput>,
) -> Result<StatusCode, AppError> {
    let pg = state.pg.clone();
    let hash_password = hash(&args.password)?.to_string();
    sqlx::query(&User::insert_query())
        .bind(&args.name)
        .bind(&args.email)
        .bind(&hash_password)
        .execute(&pg)
        .await?;
    debug!("{}", format!("INSERT user: {:#?}", &args));
    Ok(StatusCode::CREATED)
}

pub async fn patch_change_password(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Json(args): Json<PatchChangePasswordInput>,
) -> Result<StatusCode, AppError> {
    let pg = state.pg.clone();
    let redis = state.redis.clone();
    // Search for user
    let user_info =
        match sqlx::query_as::<_, User>(&format!("{} WHERE email = $1", User::select_query()))
            .bind(&args.email)
            .fetch_optional(&pg)
            .await?
        {
            Some(user) => user,
            None => {
                return Err(AppError::new(AUTH_FAILD_MESSAGE, StatusCode::FORBIDDEN, 2));
            }
        };
    // Verify current password
    if !verify(&args.password, &user_info.password_hash)? {
        return Err(AppError::new(AUTH_FAILD_MESSAGE, StatusCode::FORBIDDEN, 2));
    }
    // Hash new password
    let hash_password = hash(&args.new_password)?.to_string();
    // Update record
    sqlx::query(&User::change_password_query())
        .bind(&hash_password)
        .bind(&args.email)
        .execute(&pg)
        .await?;
    // Clean Redis login sessions
    let session = match get_session(&headers) {
        None => {
            return Err(AppError::internal(
                "Password has been changed, but session information is worng!",
            ));
        }
        Some(s) => s,
    };
    let _ = Redis::del(&redis, vec![&format!("{}:{}", &args.email, &session)]).await?;
    Ok(StatusCode::ACCEPTED)
}
