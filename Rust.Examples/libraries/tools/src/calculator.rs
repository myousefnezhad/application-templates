#![allow(dead_code)]
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

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SumRequest {
    pub a: i32,
    pub b: i32,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SumResult {
    pub result: i32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SubRequest {
    pub a: i32,
    pub b: i32,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SubResult {
    pub result: i32,
}

#[derive(Debug, Clone)]
pub struct CalculatorTools {
    pub tool_router: ToolRouter<Self>,
    pub prompt_router: PromptRouter<Self>,
}

#[tool_router]
impl CalculatorTools {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
            prompt_router: Self::prompt_router(),
        }
    }

    #[tool(description = "Calculate the sum of two numbers (Calc version)")]
    fn sum(&self, Parameters(req): Parameters<SumRequest>) -> Result<Json<SumResult>, McpError> {
        Ok(Json(SumResult {
            result: req.a + req.b,
        }))
    }

    #[tool(description = "Calculate the difference of two numbers")]
    fn sub(&self, Parameters(req): Parameters<SubRequest>) -> Result<Json<SubResult>, McpError> {
        Ok(Json(SubResult {
            result: req.a - req.b,
        }))
    }
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct MathArgs {
    pub expression: String,
}

#[prompt_router]
impl CalculatorTools {
    #[prompt(
        name = "explain_math",
        description = "Explains how to solve a math problem"
    )]
    async fn explain_math(
        &self,
        Parameters(args): Parameters<MathArgs>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        Ok(GetPromptResult {
            description: Some("Math Explanation".into()),
            messages: vec![
                PromptMessage::new_text(
                    PromptMessageRole::Assistant,
                    "You are a helpful math tutor.",
                ),
                PromptMessage::new_text(
                    PromptMessageRole::User,
                    format!(
                        "Please explain the steps to solve this expression: {}",
                        args.expression
                    ),
                ),
            ],
        })
    }
}
