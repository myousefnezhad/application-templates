use app_schema::customer::Customer;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub enum CreditScoreOperation {
    LESS,
    EQUAL,
    MORE,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct McpCreditScoreToolInput {
    pub score: i32,
    pub operation: CreditScoreOperation,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct McpCreditScoreToolOutput {
    pub customer_list: Vec<Customer>,
}
