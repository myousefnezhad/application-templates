use app_schema::customer::Customer;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub enum RiskType {
    LOW,
    MEDIUM,
    HIGH,
}

impl RiskType {
    pub fn to_string(&self) -> String {
        match self {
            Self::LOW => "low",
            Self::MEDIUM => "medium",
            Self::HIGH => "high",
        }
        .to_string()
    }
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct McpRiskToolInput {
    pub risk: RiskType,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct McpRiskToolOutput {
    pub customer_list: Vec<Customer>,
}
