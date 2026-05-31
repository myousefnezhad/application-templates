use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, JsonSchema)]
pub struct CounterRow {
    pub app_id: String,
    pub user_id: String,
    pub session_id: String,
    pub counter: i64,
}

#[derive(Debug, Deserialize, FromRow)]
pub struct CounterPgResult {
    pub counter: i64,
}

impl CounterRow {
    #[inline]
    pub fn select() -> &'static str {
        include_str!("../../../../SQL/counter_select.sql")
    }

    #[inline]
    pub fn insert() -> &'static str {
        include_str!("../../../../SQL/counter_insert.sql")
    }
}
