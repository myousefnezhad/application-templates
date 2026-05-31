use app_cryptography::jwt::{Algorithm, Claims, RedisInfo, generate_token, validate_token};
use app_error::AppError;
use app_redis::Redis;
use app_state::AppState;
use axum::{
    extract::{Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header::AUTHORIZATION},
    middleware::Next,
    response::{IntoResponse, Response},
};
use chrono::{Duration, Utc};
use std::sync::Arc;
use tracing::*;

pub async fn web_auth_middleware(
    State(state): State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Response {
    info!(
        "Web Athorization Middleware Running for {} {}",
        req.method(),
        req.uri()
    );
    let redis = state.redis.clone();
    let config = state.config.clone();
    let jwt_access_key = config.jwt_access_key.clone();
    let jwt_access_session_minutes = config.jwt_access_session_minutes;
    let jwt_refresh_key = config.jwt_refresh_key.clone();
    let unauthorized =
        || AppError::new("Unauthorized", StatusCode::UNAUTHORIZED, 0).into_response();

    // IMPORTANT: make the header value owned so we don't keep borrowing `req`
    let auth_header: String = match req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
    {
        Some(v) => v,
        None => return unauthorized(),
    };

    // Expect: "Authorization: Bearer <access_token> <refresh_token>"
    let parts: Vec<&str> = auth_header.split_whitespace().collect();
    if parts.len() != 3 || !parts[0].eq_ignore_ascii_case("bearer") {
        return unauthorized();
    }
    let access_token = parts[1];
    let refresh_token = parts[2];

    // 1) Access token valid
    if let Ok(access_token_data) = validate_token(Algorithm::HS256, &jwt_access_key, access_token) {
        let access_claim = access_token_data.claims;
        add_req_headers(&mut req, &access_claim, access_token);

        info!("User {} approved using access token", access_claim.email);

        let mut res = next.run(req).await;
        add_res_headers(&mut res, access_token);
        return res;
    }

    // 2) Otherwise try refresh token
    if let Ok(refresh_token_data) =
        validate_token(Algorithm::HS256, &jwt_refresh_key, refresh_token)
    {
        let refresh_claim: Claims = refresh_token_data.claims;

        if let Ok(redis_info_str) = Redis::get::<String>(
            &redis,
            &format!("{}:{}", refresh_claim.email, refresh_claim.session),
        )
        .await
        {
            if let Ok(redis_info) = serde_json::from_str::<RedisInfo>(&redis_info_str) {
                if refresh_token == redis_info.token {
                    // Regenerate access token
                    let iat = Utc::now().timestamp();
                    let exp =
                        (Utc::now() + Duration::minutes(jwt_access_session_minutes)).timestamp();

                    let access_claim = Claims {
                        iat,
                        exp,
                        id: refresh_claim.id.clone(),
                        name: refresh_claim.name.clone(),
                        email: refresh_claim.email.clone(),
                        session: refresh_claim.session,
                        source: refresh_claim.source.clone(),
                    };

                    if let Ok(new_access_token) =
                        generate_token(Algorithm::HS256, &jwt_access_key, access_claim)
                    {
                        add_req_headers(&mut req, &refresh_claim, &new_access_token);

                        info!("User {} approved using refresh token", refresh_claim.email);

                        let mut res = next.run(req).await;
                        add_res_headers(&mut res, &new_access_token);
                        return res;
                    }
                }
            }
        }
    }
    unauthorized()
}

fn add_req_headers(req: &mut Request, claim: &Claims, access_token: &str) {
    // If any value is invalid for headers, we just skip it (don’t panic middleware)
    let _ = req.headers_mut().insert(
        "x-auth-name",
        HeaderValue::from_str(&claim.name).unwrap_or_else(|_| HeaderValue::from_static("")),
    );
    let _ = req.headers_mut().insert(
        "x-auth-email",
        HeaderValue::from_str(&claim.email).unwrap_or_else(|_| HeaderValue::from_static("")),
    );
    let _ = req.headers_mut().insert(
        "x-auth-id",
        HeaderValue::from_str(&claim.id).unwrap_or_else(|_| HeaderValue::from_static("")),
    );
    let _ = req.headers_mut().insert(
        "x-auth-session",
        HeaderValue::from_str(&claim.session.to_string())
            .unwrap_or_else(|_| HeaderValue::from_static("")),
    );
    let _ = req.headers_mut().insert(
        "x-auth-access-token",
        HeaderValue::from_str(access_token).unwrap_or_else(|_| HeaderValue::from_static("")),
    );
}

fn add_res_headers(res: &mut Response, access_token: &str) {
    let _ = res.headers_mut().insert(
        "x-auth-access-token",
        HeaderValue::from_str(access_token).unwrap_or_else(|_| HeaderValue::from_static("")),
    );
}

pub fn get_email(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-auth-email")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

pub fn get_session(headers: &HeaderMap) -> Option<u64> {
    match headers.get("x-auth-session").and_then(|v| v.to_str().ok()) {
        None => None,
        Some(x) => match x.parse::<u64>() {
            Err(e) => {
                error!(
                    "Middleware cannot parse session information from header. {:?}",
                    &e
                );
                None
            }
            Ok(x) => Some(x),
        },
    }
}

pub async fn mcp_auth_middleware(
    State(state): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Response {
    info!(
        "MCP Authorization Middleware Running for {} {}",
        req.method(),
        req.uri()
    );
    let config = state.config.clone();
    let unauthorized =
        || AppError::new("Unauthorized", StatusCode::UNAUTHORIZED, 0).into_response();

    // IMPORTANT: make the header value owned so we don't keep borrowing `req`
    let auth_header: String = match req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
    {
        Some(v) => v,
        None => return unauthorized(),
    };
    let parts: Vec<&str> = auth_header.split_whitespace().collect();
    if parts.len() != 2 || !parts[0].eq_ignore_ascii_case("bearer") {
        return unauthorized();
    }
    if &config.mcp_token != parts[1] {
        error!("MCP token is not valid: {}", &parts[1]);
        return unauthorized();
    }
    next.run(req).await
}
