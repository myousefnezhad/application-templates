mod handlers;
mod routings;

use crate::routings::router;
use app_config::AppConfig;
use app_log::init_tracing;
use app_redis::Redis;
use app_state::AppState;
use dotenv::dotenv;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, session::never::NeverSessionManager,
};
use sqlx::postgres::PgPoolOptions;
use std::{env, sync::Arc};
use tracing::*;

pub async fn mcp_service() {
    dotenv().ok();
    let config = AppConfig::new();
    let bind = config.mcp_bind.clone();
    let pg_connection = config.pg_connection;
    let redis_url = config.redis_url.clone();
    init_tracing(config.log_level.clone());
    // PostgreSQL
    let database_url = match env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(e) => {
            debug!("{}", e);
            panic!("Cannot locate DATABASE_URL env variable");
        }
    };
    let pg = PgPoolOptions::new()
        .max_connections(pg_connection.try_into().unwrap())
        .connect(&database_url)
        .await
        .unwrap();
    // Redis
    let redis = match Redis::new(&redis_url) {
        Ok(redis_pool) => redis_pool,
        Err(err) => panic!("Cannot connect Redis\n{}", err),
    };
    // Generating AppState
    let app_state = Arc::new(AppState {
        config: config.clone(),
        pg,
        redis,
        agent_runner: None,
        agent_session: None,
    });
    // Loading Routes
    let mcp_config = StreamableHttpServerConfig {
        stateful_mode: false,
        ..Default::default()
    };
    let routes = router(app_state, NeverSessionManager::default().into(), mcp_config);
    // Setup TCP Port
    let tcp_listener = tokio::net::TcpListener::bind(&bind).await.unwrap();
    // Running Server ...
    info!("Serving web server on {}", &bind);
    let _ = axum::serve(tcp_listener, routes).await;
}
