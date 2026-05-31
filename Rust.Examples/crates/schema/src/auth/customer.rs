use chrono::NaiveDateTime;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, JsonSchema)]
pub struct Customer {
    pub id: i32,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub industry_sector: Option<String>,
    pub account_number: String,
    pub balance: f64,
    pub currency: String,
    pub credit_score: Option<i32>,
    pub risk_level: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl Customer {
    #[inline]
    pub fn select_base() -> &'static str {
        "SELECT * FROM app.customers"
    }
}
