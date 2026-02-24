use app_schema::kb::KnowledgeBased;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct McpAddKnowledgeBasedToolInput {
    pub content: String,
}
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct McpAddKnowledgeBasedToolOutput {
    pub status: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct McpSearchKnowledgeBasedToolInput {
    pub content: String,
    pub confident: Option<f32>,
}
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct McpSearchKnowledgeBasedToolOutput {
    pub results: Vec<KnowledgeBased>,
}
