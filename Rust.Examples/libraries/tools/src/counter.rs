#![allow(dead_code)]
use app_adk_utils::session::tools::AdkInjectContext;
use app_error::AppError;
use app_schema::tools::counter::{CounterPgResult, CounterRow};
use app_state::AppState;
use rmcp::{
    ErrorData as McpError, RoleServer,
    handler::server::{
        router::{prompt::PromptRouter, tool::ToolRouter},
        wrapper::{Json, Parameters},
    },
    model::*,
    prompt, prompt_router, schemars,
    service::RequestContext,
    tool, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StructRequest {
    pub a: i32,
    pub b: i32,
}

#[derive(Clone)]
pub struct CounterTools {
    state: Arc<AppState>,
    pub tool_router: ToolRouter<Self>,
    pub prompt_router: PromptRouter<Self>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CouterIncrementRequest {
    pub value: i64,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct CounterResult {
    value: i64,
}

#[tool_router]
impl CounterTools {
    pub fn new(state: Arc<AppState>) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
            prompt_router: Self::prompt_router(),
        }
    }

    // Helper for resources (internal logic)
    pub fn _create_resource_text(&self, uri: &str, name: &str) -> Resource {
        RawResource::new(uri, name.to_string()).no_annotation()
    }

    #[tool(description = "Increment the counter by provided value")]
    async fn increment_counter(
        &self,
        Parameters(args): Parameters<CouterIncrementRequest>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<Json<CounterResult>, McpError> {
        let state = &self.state.clone();
        let pg = state.pg.clone();
        let adk = AdkInjectContext::extract_adk_context(&ctx);
        let (app, user, session): (String, String, String) = match &adk {
            None => (
                "default".to_owned(),
                "default".to_owned(),
                "default".to_owned(),
            ),
            Some(i) => (
                i.app_name.clone().unwrap_or_else(|| "default".to_owned()),
                i.user_id.clone().unwrap_or_else(|| "default".to_owned()),
                i.session_id.clone().unwrap_or_else(|| "default".to_owned()),
            ),
        };
        let counter = sqlx::query_as::<_, CounterPgResult>(CounterRow::insert())
            .bind(&app)
            .bind(&user)
            .bind(&session)
            .bind(&args.value)
            .fetch_one(&pg)
            .await
            .map_err(AppError::from)?;
        Ok(Json(CounterResult {
            value: counter.counter,
        }))
    }

    #[tool(description = "Get the current counter value")]
    async fn get_counter_value(
        &self,
        ctx: RequestContext<RoleServer>,
    ) -> Result<Json<CounterResult>, McpError> {
        let state = &self.state.clone();
        let pg = state.pg.clone();
        let adk = AdkInjectContext::extract_adk_context(&ctx);
        let (app, user, session): (String, String, String) = match &adk {
            None => (
                "default".to_owned(),
                "default".to_owned(),
                "default".to_owned(),
            ),
            Some(i) => (
                i.app_name.clone().unwrap_or_else(|| "default".to_owned()),
                i.user_id.clone().unwrap_or_else(|| "default".to_owned()),
                i.session_id.clone().unwrap_or_else(|| "default".to_owned()),
            ),
        };
        let counter = sqlx::query_as::<_, CounterPgResult>(CounterRow::select())
            .bind(&app)
            .bind(&user)
            .bind(&session)
            .fetch_one(&pg)
            .await
            .map_err(AppError::from)?;
        Ok(Json(CounterResult {
            value: counter.counter,
        }))
    }

    // #[tool(description = "This only makes an error")]
    // async fn make_error(&self) -> Result<Json<CounterResult>, McpError> {
    //     Err(AppError::internal("Just a simple error").into())
    // }

    // Explicitly defining manual resource handlers here for main.rs to call
    pub async fn list_my_resources(&self) -> Vec<Resource> {
        vec![
            self._create_resource_text("str:////home/tony/temp/", "cwd"),
            self._create_resource_text("memo://insights", "memo-name"),
        ]
    }

    pub async fn read_my_resource(&self, uri: &str) -> Result<String, McpError> {
        match uri {
            "str:////home/tony/temp/" => {
                Ok(include_str!("../../../mcp_contents/test_content.txt").to_string())
            }
            "memo://insights" => Ok("Business Intelligence Memo...".to_string()),
            _ => Err(McpError::resource_not_found(
                "not found",
                Some(json!({ "uri": uri })),
            )),
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct CounterAnalysisArgs {
    pub goal: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SummaryArgs {
    #[schemars(description = "Style of summary: 'brief' or 'verbose'")]
    pub style: Option<String>,
}

#[prompt_router]
impl CounterTools {
    #[prompt(name = "counter_analysis")]
    async fn counter_analysis(
        &self,
        Parameters(args): Parameters<CounterAnalysisArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        let state = &self.state.clone();
        let pg = state.pg.clone();
        let adk = AdkInjectContext::extract_adk_context(&ctx);
        let (app, user, session): (String, String, String) = match &adk {
            None => (
                "default".to_owned(),
                "default".to_owned(),
                "default".to_owned(),
            ),
            Some(i) => (
                i.app_name.clone().unwrap_or_else(|| "default".to_owned()),
                i.user_id.clone().unwrap_or_else(|| "default".to_owned()),
                i.session_id.clone().unwrap_or_else(|| "default".to_owned()),
            ),
        };
        let counter = sqlx::query_as::<_, CounterPgResult>(CounterRow::select())
            .bind(&app)
            .bind(&user)
            .bind(&session)
            .fetch_one(&pg)
            .await
            .map_err(AppError::from)?;
        Ok(GetPromptResult {
            description: Some("Analysis".into()),
            messages: vec![PromptMessage::new_text(
                PromptMessageRole::User,
                format!("Current: {}, Goal: {}", counter.counter, args.goal),
            )],
        })
    }

    #[prompt(
        name = "summarize_counter",
        description = "Returns a textual summary of the current counter state"
    )]
    async fn summarize_counter(
        &self,
        Parameters(args): Parameters<SummaryArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        let state = &self.state.clone();
        let pg = state.pg.clone();
        let adk = AdkInjectContext::extract_adk_context(&ctx);
        let (app, user, session): (String, String, String) = match &adk {
            None => (
                "default".to_owned(),
                "default".to_owned(),
                "default".to_owned(),
            ),
            Some(i) => (
                i.app_name.clone().unwrap_or_else(|| "default".to_owned()),
                i.user_id.clone().unwrap_or_else(|| "default".to_owned()),
                i.session_id.clone().unwrap_or_else(|| "default".to_owned()),
            ),
        };
        let counter = sqlx::query_as::<_, CounterPgResult>(CounterRow::select())
            .bind(&app)
            .bind(&user)
            .bind(&session)
            .fetch_one(&pg)
            .await
            .map_err(AppError::from)?;
        // Logic: Handle the optional argument
        let style = args.style.as_deref().unwrap_or("brief");

        let summary_text = match style {
            "verbose" => format!(
                "SYSTEM STATUS REPORT:\nThe core counter variable is currently holding the integer value of {}.\nSystem is operational.",
                counter.counter
            ),
            _ => format!("The counter value is {}.", counter.counter), // brief default
        };

        Ok(GetPromptResult {
            description: Some("Counter State Summary".into()),
            messages: vec![PromptMessage::new_text(
                PromptMessageRole::User,
                summary_text,
            )],
        })
    }

    #[prompt(
        name = "summarize_content",
        description = "Returns a textual summary of the content"
    )]
    async fn summarize_content(
        &self,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        Ok(GetPromptResult {
            description: Some("Counter State Summary".into()),
            messages: vec![PromptMessage::new_text(
                PromptMessageRole::User,
                format!("Summarize the following content:"),
            )],
        })
    }
}
