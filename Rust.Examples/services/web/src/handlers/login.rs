use app_cryptography::{
    hash::verify,
    jwt::{Algorithm, Claims, RedisInfo, generate_token},
};
use app_dto::auth::login::{PostLoginInput, PostLoginOutput};
use app_error::AppError;
use app_middleware::{get_email, get_session};
use app_redis::Redis;
use app_schema::auth::users::User;
use app_state::AppState;
use axum::{
    extract::{Json, State},
    http::{HeaderMap, StatusCode},
};
use chrono::{Duration, Utc};
use rand::Rng;
use std::sync::Arc;
use tracing::error;

const AUTH_FAILD_MESSAGE: &'static str = "Provided information is wrong!";

use askama::Template;
use axum::response::Html;

#[derive(Template)]
#[template(path = "login.html")]
struct LoginPage;

pub async fn get_login() -> Result<Html<String>, AppError> {
    let page = LoginPage;
    Ok(Html(page.render()?))
}

pub async fn post_login(
    State(state): State<Arc<AppState>>,
    Json(args): Json<PostLoginInput>,
) -> Result<Json<PostLoginOutput>, AppError> {
    // Init configs
    let config = state.config.clone();
    let pg = state.pg.clone();
    let redis = state.redis.clone();
    let jwt_access_key = config.jwt_access_key.clone();
    let jwt_access_session_min = config.jwt_access_session_minutes;
    let jwt_refresh_key = config.jwt_refresh_key.clone();
    let jwt_refresh_session_day = config.jwt_refresh_session_days;
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
    if !verify(&args.password, &user_info.password_hash)? {
        return Err(AppError::new(AUTH_FAILD_MESSAGE, StatusCode::FORBIDDEN, 2));
    }
    // User authentication is successed, generating login
    // Access Token
    let session: u64 = {
        let mut rng = rand::thread_rng();
        rng.r#gen()
    };
    let iat = Utc::now().timestamp();
    let exp = (Utc::now() + Duration::minutes(jwt_access_session_min)).timestamp();
    let email = user_info.email.to_string().clone();
    let name = user_info.name.clone();
    let id = format!("{:?}", &user_info.id);
    let access_claim = Claims {
        iat,
        exp,
        id: id.clone(),
        name: name.clone(),
        email: email.clone(),
        session,
        source: "local".to_string(),
    };
    // Refresh Token
    let exp = (Utc::now() + Duration::days(jwt_refresh_session_day)).timestamp();
    let refresh_claim = Claims {
        iat,
        exp,
        id: id.clone(),
        name: name.clone(),
        email: email.clone(),
        session,
        source: "local".to_string(),
    };
    // Generate Tokens
    let access_token = generate_token(Algorithm::HS256, &jwt_access_key, &access_claim)?;
    let refresh_token = generate_token(Algorithm::HS256, &jwt_refresh_key, &refresh_claim)?;
    // Saving Refresh Token in Redis
    let redis_key = format!("{}:{}", &email, &session);
    let redis_info = RedisInfo {
        id,
        name,
        email,
        session,
        token: refresh_token.clone(),
        source: "local".to_string(),
    };
    let redis_info_str = serde_json::to_string(&redis_info)?;
    Redis::set(&redis, &redis_key, &redis_info_str).await?;
    Ok(Json(PostLoginOutput {
        access_token,
        refresh_token,
    }))
}

pub async fn post_logout(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, AppError> {
    let redis = state.redis.clone();
    let email = match get_email(&headers) {
        None => {
            error!("Email header not found!");
            return Err(AppError::internal(AUTH_FAILD_MESSAGE));
        }
        Some(e) => e,
    };
    let session = match get_session(&headers) {
        None => {
            error!("Session header not found!");
            return Err(AppError::internal(AUTH_FAILD_MESSAGE));
        }
        Some(s) => s,
    };
    let _ = Redis::del(&redis, vec![&format!("{}:{}", &email, &session)]).await?;
    Ok(StatusCode::OK)
}
