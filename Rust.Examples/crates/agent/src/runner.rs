use adk_core::{AdkError, EventStream};
use adk_rust::prelude::{Event, Part};
use futures::StreamExt;

pub struct AgentResponse {
    pub response: String,
    pub thinkings: String,
    pub functions: String,
    pub tools: String,
}

pub async fn stream_response_parser(
    stream: &mut EventStream,
    mut history: Option<&mut Vec<Event>>,
) -> Result<AgentResponse, AdkError> {
    // Print only the final response content
    let mut response = String::new();
    let mut thinkings = String::new();
    let mut functions = String::new();
    let mut tools = String::new();

    while let Some(ev) = stream.next().await {
        let ev = ev?;
        if let Some(h) = history.as_deref_mut() {
            h.push(ev.clone());
        }
        match &ev.content() {
            Some(ctx) => {
                for part in ctx.parts.iter() {
                    match &part {
                        Part::Thinking { thinking, .. } => thinkings.push_str(&thinking),
                        _ => (),
                    }
                    match &part {
                        Part::Text { text } => response.push_str(&text),
                        _ => (),
                    }
                    match &part {
                        Part::FunctionResponse {
                            function_response, ..
                        } => functions.push_str(&format!("{:?}", &function_response)),
                        _ => (),
                    }
                    match &part {
                        Part::ServerToolResponse {
                            server_tool_response,
                        } => tools.push_str(&format!("{:?}", &server_tool_response)),
                        _ => (),
                    }
                }
            }
            None => (),
        }
    }

    Ok(AgentResponse {
        response,
        thinkings,
        tools,
        functions,
    })
}
