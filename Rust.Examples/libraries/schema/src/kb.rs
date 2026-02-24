use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, JsonSchema)]
pub struct KnowledgeBased {
    pub id: i64,
    pub chunk: String,
    pub created_at: DateTime<Utc>,
}

impl KnowledgeBased {
    #[inline]
    pub fn select_base() -> &'static str {
        "SELECT * FROM rag.knowledge_based"
    }
}
