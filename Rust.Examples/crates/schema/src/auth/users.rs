use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Deserialize, Clone, FromRow, Serialize, JsonSchema)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    #[inline]
    pub fn select_query() -> &'static str {
        include_str!("../../../../SQL/auth/users/select_base.sql")
    }
    #[inline]
    pub fn insert_query() -> &'static str {
        include_str!("../../../../SQL/auth/users/insert_query.sql")
    }
    #[inline]
    pub fn change_password_query() -> &'static str {
        include_str!("../../../../SQL/auth/users/change_password.sql")
    }
}
