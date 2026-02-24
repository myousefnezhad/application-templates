use adk_runner::Runner;
use adk_rust::session::SessionService;
use app_config::AppConfig;
use deadpool_redis::Pool as RedisPool;
use sqlx::Pool as PostgresPool;
use sqlx::postgres::Postgres;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub redis: RedisPool,
    pub config: AppConfig,
    pub pg: PostgresPool<Postgres>,
    pub agent_runner: Option<Arc<Runner>>,
    pub agent_session: Option<Arc<dyn SessionService>>,
}
