use adk_core::{ReadonlyContext, Toolset};
use adk_model::{OpenAIClient, OpenAIConfig};
use adk_runner::Runner;
use adk_rust::prelude::{GoogleSearchTool, LlmAgentBuilder, McpToolset, RunnerConfig};
use app_adk_utils::{
    content::SimpleContext,
    session::{postgres::PgSessionService, tools::AdkInjectSessionTool},
};
use app_config::AppConfig;
use app_error::AppError;
use rmcp09::{
    ServiceExt,
    transport::streamable_http_client::{
        StreamableHttpClientTransport, StreamableHttpClientTransportConfig,
    },
};
use sqlx::postgres::PgPoolOptions;
use std::{env, sync::Arc};

// This is default sqlx parameter
// It can be replace with some param from config or other env
// if we want to use seperate postgresql database for agent session
const DATABASE_URL: &'static str = "DATABASE_URL";

pub async fn agent_builder(
    config: &AppConfig,
) -> Result<(Arc<Runner>, Arc<PgSessionService>), AppError> {
    // Agent makes its own postgresql connection
    let database_url = env::var(DATABASE_URL)?;
    let pg_connection = config.pg_connection;
    let pg = PgPoolOptions::new()
        .max_connections(pg_connection.try_into()?)
        .connect(&database_url)
        .await?;
    // Make PostgreSQL session Manager
    // Can replace with build-in in memory session manager
    let agent_sessions = Arc::new(PgSessionService::new(pg).await?);
    // This deploy agent session schema if not exists
    agent_sessions.migrate().await?;
    // LLM Load
    let model_config = OpenAIConfig {
        api_key: config.llm_token.clone(),
        model: config.llm_model.clone(),
        base_url: Some(config.llm_base_url.clone()),
        project_id: None,
        organization_id: None,
    };
    let llm_model = OpenAIClient::new(model_config).unwrap();
    // MCP Tool Config
    let mcp_cfg = StreamableHttpClientTransportConfig::with_uri(config.mcp_base_url.clone())
        .auth_header(&config.mcp_token.clone());
    let transport = StreamableHttpClientTransport::from_config(mcp_cfg);
    let mcp_client = ().serve(transport).await?;
    let toolset = McpToolset::new(mcp_client);
    let _cancel = toolset.cancellation_token().await;
    let ctx: Arc<dyn ReadonlyContext> = Arc::new(SimpleContext {
        app_name: Some(config.agent_app_name.clone()),
        ..Default::default()
    });
    let mcp_tools = toolset.tools(ctx).await?;
    // Agent Builder
    let mut builder = LlmAgentBuilder::new(config.agent_app_name.clone())
        .description(config.agent_description.clone())
        .instruction(config.agent_instruction.clone())
        .model(Arc::new(llm_model));
    for t in mcp_tools {
        builder = builder.tool(Arc::new(AdkInjectSessionTool::new(t)));
    }
    // Add Google Search Tools
    // Note: compatible with Gemini 2 models
    // Need export GOOGLE_API_KEY="YOUR_GOOGLE_KEY"
    builder = builder.tool(Arc::new(GoogleSearchTool::new()));
    let agent = Arc::new(builder.build()?);
    // Agent Runner
    // let artifacts = Arc::new(InMemoryArtifactService::new());
    let agnet_runner = Arc::new(Runner::new(RunnerConfig {
        app_name: config.agent_app_name.to_string(),
        agent: agent,
        session_service: agent_sessions.clone(),
        artifact_service: None,
        memory_service: None,
        run_config: None, // Uses default SSE streaming
    })?);
    Ok((agnet_runner, agent_sessions))
}
