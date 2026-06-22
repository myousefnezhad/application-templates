mod helper;

use crate::helper::*;
use adk_core::{ReadonlyContext, Toolset};
use adk_tool::McpToolset;
use anyhow::{Result, anyhow};
use app_adk_utils::content::SimpleContext;
use app_config::AppConfig;
use app_log::init_tracing;
use rmcp::{
    ServiceExt,
    model::CallToolRequestParams,
    transport::streamable_http_client::{
        StreamableHttpClientTransport, StreamableHttpClientTransportConfig,
    },
};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct DebugContext {
    pub app_name: String,
    pub user_id: String,
    pub agent_name: String,
    pub session_id: String,
    pub invocation_id: String,
    pub branch: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = AppConfig::new();
    init_tracing(config.log_level.clone());
    // CTX
    let debug_ctx = DebugContext {
        app_name: ask_default("app_name", "mcp-debugger-app")?,
        user_id: ask_default("user_id", "debug-user")?,
        agent_name: ask_default("agent_name", "mcp-debugger")?,
        session_id: ask_default("session_id", &Uuid::new_v4().to_string())?,
        invocation_id: Uuid::new_v4().to_string(),
        branch: "main".to_string(),
    };
    // Connect to MCP
    println!("Connecting to");
    println!("{}", config.mcp_base_url);
    println!();
    let mcp_cfg = StreamableHttpClientTransportConfig::with_uri(config.mcp_base_url.clone())
        .auth_header(&config.mcp_token);
    let transport = StreamableHttpClientTransport::from_config(mcp_cfg.clone());
    let client = ().serve(transport).await?;
    println!("Connected");
    // Reading Tools
    let toolset = McpToolset::new(client);
    let ctx: Arc<dyn ReadonlyContext> = Arc::new(SimpleContext {
        app_name: Some(debug_ctx.app_name.clone()),

        ..Default::default()
    });
    println!();
    println!("Loading tools ...");
    let tools = toolset.tools(ctx).await?;
    println!();
    println!("Found {} tools", tools.len());
    println!();
    for (i, t) in tools.iter().enumerate() {
        println!("{}. {}", i + 1, t.name());
    }
    println!();
    let input = ask("Select tool: ")?;

    let tool = if let Ok(idx) = input.parse::<usize>() {
        tools.get(idx - 1).ok_or_else(|| anyhow!("Invalid index"))?
    } else {
        tools
            .iter()
            .find(|t| t.name() == input)
            .ok_or_else(|| anyhow!("Tool not found"))?
    };
    println!();
    println!("Name");
    println!("{}", tool.name());
    println!();
    println!("Description");
    println!("{}", tool.description());
    println!();
    println!("================================");
    println!("Parameters Schema");
    println!("================================");
    match tool.parameters_schema() {
        Some(schema) => {
            println!("{}", serde_json::to_string_pretty(&schema)?);
        }
        None => {
            println!("No schema");
        }
    }
    println!();
    println!("Fill tool arguments");
    println!();
    let args = match tool.parameters_schema() {
        Some(schema) => ask_from_schema(&schema)?,
        None => {
            serde_json::json!({})
        }
    };
    let mut args_obj = args
        .as_object()
        .cloned()
        .ok_or_else(|| anyhow!("Arguments must be JSON object"))?;
    args_obj.insert(
        "_adk".to_string(),
        json!({
            "app_name":
                debug_ctx.app_name,
            "user_id":
                debug_ctx.user_id,
            "session_id":
                debug_ctx.session_id,
            "agent_name":
                debug_ctx.agent_name,
            "invocation_id":
                debug_ctx.invocation_id,
            "branch":
                debug_ctx.branch
        }),
    );
    let tool_name = tool.name().to_string();
    println!();
    println!("================================");
    println!("Calling tool:");
    println!("================================");
    println!("{}", &tool_name);
    println!();
    println!("================================");
    println!("Generated Arguments");
    println!("================================");
    println!("{}", serde_json::to_string_pretty(&args)?);
    let req = CallToolRequestParams::new(tool_name).with_arguments(args_obj);
    let transport = StreamableHttpClientTransport::from_config(mcp_cfg.clone());
    let mut client = ().serve(transport).await?;
    let result = client.call_tool(req).await?;
    print_result(&result)?;
    client.close().await?;
    Ok(())
}
