use adk_rust::prelude::*;
use async_trait::async_trait;
use rmcp::{RoleServer, service::RequestContext};
use serde::Deserialize;
use serde_json::{Map, Value, from_value, json};
use std::sync::Arc;

pub const ADK_TOOL_ARGS: &'static str = "__adk_tool_args";
pub const ADK_TOOL_PERFIX: &'static str = "_adk";

#[derive(Debug, Default, Deserialize)]
pub struct AdkInjectContext {
    pub app_name: Option<String>,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub agent_name: Option<String>,
    pub invocation_id: Option<String>,
    pub branch: Option<String>,
}

impl AdkInjectContext {
    pub fn extract_adk_context(ctx: &RequestContext<RoleServer>) -> Option<Self> {
        // If no tool_args → return empty struct
        let args = match ctx.meta.get(ADK_TOOL_ARGS) {
            Some(Value::Object(map)) => map,
            _ => return None,
        };
        // If no "_adk" field → return empty struct
        let adk_val = match args.get(ADK_TOOL_PERFIX) {
            Some(val) => val.clone(),
            None => return None,
        };

        // Try deserialize → if fails return empty struct
        match from_value::<Self>(adk_val) {
            Ok(v) => Some(v),
            Err(_) => None,
        }
    }
}

pub struct AdkInjectSessionTool {
    pub inner: Arc<dyn Tool>,
}

impl AdkInjectSessionTool {
    pub fn new(inner: Arc<dyn Tool>) -> Self {
        Self { inner }
    }
}

fn ensure_object(args: Value) -> Map<String, Value> {
    match args {
        Value::Object(m) => m,
        other => {
            let mut m = Map::new();
            m.insert("input".to_string(), other);
            m
        }
    }
}

#[async_trait]
impl Tool for AdkInjectSessionTool {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn description(&self) -> &str {
        self.inner.description()
    }

    async fn execute(&self, ctx: Arc<dyn ToolContext>, args: Value) -> Result<Value> {
        let mut obj = ensure_object(args);
        // Pull identity directly from ToolContext
        obj.insert(
            ADK_TOOL_PERFIX.to_string(),
            json!({
                "app_name": ctx.app_name(),
                "user_id": ctx.user_id(),
                "session_id": ctx.session_id(),
                "agent_name": ctx.agent_name(),
                "invocation_id": ctx.invocation_id(),
                "branch": ctx.branch(),
            }),
        );
        self.inner.execute(ctx, Value::Object(obj)).await
    }
}
