use adk_core::Content;
use adk_rust::session::{CreateRequest, GetRequest};
use app_agent::runner::stream_response_parser;
use app_error::AppError;
use app_middleware::get_email;
use app_state::AppState;
use askama::Template;
use axum::response::Html;
use axum::{
    extract::{Json, State},
    http::HeaderMap,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Template)]
#[template(path = "agent.html")]
struct AgentPage;

pub async fn get_agent() -> Result<Html<String>, AppError> {
    let page = AgentPage;
    Ok(Html(page.render()?))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatPostInput {
    pub session_id: Option<String>,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatPostOutput {
    pub session_id: String,
    pub content: String,
}

pub async fn post_agent(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Json(args): Json<ChatPostInput>,
) -> Result<Json<ChatPostOutput>, AppError> {
    let config = state.config.clone();
    let user_id = match get_email(&headers) {
        None => {
            return Err(AppError::internal("Cannot identify user"));
        }
        Some(u) => u,
    };
    let agent_runner = match &state.agent_runner {
        None => {
            return Err(AppError::internal("Cannot find agent runner"));
        }
        Some(runner) => runner.clone(),
    };
    let agent_session = match &state.agent_session {
        None => {
            return Err(AppError::internal("Cannot find agent session"));
        }
        Some(session) => session.clone(),
    };
    let new_session_id = Uuid::new_v4().to_string();
    let agent_current_session = match &args.session_id {
        // Make a new session
        None => match agent_session
            .create(CreateRequest {
                app_name: config.agent_app_name.clone(),
                user_id: user_id.clone(),
                session_id: Some(new_session_id.clone()),
                state: HashMap::new(),
            })
            .await
        {
            Err(e) => {
                return Err(AppError::internal(&format!("{}", &e)));
            }
            Ok(_) => new_session_id,
        },
        // Find Exising Session
        Some(session_id) => match agent_session
            .get(GetRequest {
                app_name: config.agent_app_name.clone(),
                user_id: user_id.clone(),
                session_id: session_id.clone(),
                num_recent_events: None,
                after: None,
            })
            .await
        {
            Ok(_) => session_id.clone(),
            // Make a new Session if session not found
            Err(_) => match agent_session
                .create(CreateRequest {
                    app_name: config.agent_app_name.clone(),
                    user_id: user_id.clone(),
                    session_id: Some(new_session_id.clone()),
                    state: HashMap::new(),
                })
                .await
            {
                Err(e) => {
                    return Err(AppError::internal(&format!("{}", &e)));
                }
                Ok(_) => session_id.clone(),
            },
        },
    };
    // Making Agent Runner
    let user_input = Content::new("user").with_text(args.content);
    let mut stream = match agent_runner
        .run(user_id.clone(), agent_current_session.clone(), user_input)
        .await
    {
        Ok(s) => s,
        Err(e) => return Err(AppError::internal(&format!("{}", &e))),
    };
    // Generating Answer
    let content = stream_response_parser(&mut stream, None).await?;

    Ok(Json(ChatPostOutput {
        session_id: agent_current_session.clone(),
        content,
    }))
}
