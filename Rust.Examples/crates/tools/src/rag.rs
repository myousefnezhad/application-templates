#![allow(dead_code)]
use app_dto::rag::{
    McpAddKnowledgeBasedToolInput, McpAddKnowledgeBasedToolOutput,
    McpSearchKnowledgeBasedToolInput, McpSearchKnowledgeBasedToolOutput,
};
use app_error::AppError;
use app_llama_cpp::embedding::embedding;
use app_schema::kb::KnowledgeBased;
use app_state::AppState;
use rmcp::{
    ErrorData as McpError,
    handler::server::{
        router::tool::ToolRouter,
        wrapper::{Json, Parameters},
    },
    tool, tool_router,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct RagTools {
    state: Arc<AppState>,
    pub tool_router: ToolRouter<Self>,
}

#[tool_router]
impl RagTools {
    pub fn new(state: Arc<AppState>) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Add provided String content to knowledge based")]
    async fn add_content_knowledge_based(
        &self,
        Parameters(args): Parameters<McpAddKnowledgeBasedToolInput>,
    ) -> Result<Json<McpAddKnowledgeBasedToolOutput>, McpError> {
        let pg = self.state.pg.clone();
        let content_embd = embedding(&self.state.config, &args.content)
            .await
            .map_err(AppError::from)?;
        sqlx::query("INSERT INTO rag.knowledge_based (chunk, embedding) VALUES ($1, $2)")
            .bind(&args.content)
            .bind(&content_embd)
            .execute(&pg)
            .await
            .map_err(AppError::from)?;

        Ok(Json(McpAddKnowledgeBasedToolOutput {
            status: "Ok".to_string(),
        }))
    }

    #[tool(
        description = "Search provided String content in the knowledge based return top 5 matches; confident is an optional value between 0 and 1; default is 0.6"
    )]
    async fn search_content_knowledge_based(
        &self,
        Parameters(args): Parameters<McpSearchKnowledgeBasedToolInput>,
    ) -> Result<Json<McpSearchKnowledgeBasedToolOutput>, McpError> {
        let pg = self.state.pg.clone();
        let content_embd = embedding(&self.state.config, &args.content)
            .await
            .map_err(AppError::from)?;
        let confident = match &args.confident {
            None => &0.6,
            Some(x) => x,
        };

        let results = sqlx::query_as::<_, KnowledgeBased>(&format!("SELECT id, chunk, created_at FROM rag.knowledge_based WHERE 1 - (embedding <=> $1::vector) >= {} ORDER BY embedding <=> $1::vector LIMIT 5;", confident))
            .bind(&content_embd)
            .fetch_all(&pg)
            .await
            .map_err(AppError::from)?;

        Ok(Json(McpSearchKnowledgeBasedToolOutput { results }))
    }
}
