use app_schema::customer::Customer;
use app_schema::kb::KnowledgeBased;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct McpCustomerInformationToolInput {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub account_number: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct CustomerInformation {
    pub customer_record: Customer,
    pub public_information: Vec<KnowledgeBased>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct McpCustomerInformationToolOutput {
    pub customer_list: Vec<CustomerInformation>,
}
