use app_config::AppConfig;
use app_error::AppError;
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct LlamaChatRequest<'a> {
    pub model: &'a str,
    pub messages: Vec<LlamaMessage<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct LlamaMessage<'a> {
    pub role: &'a str, // "system" | "user" | "assistant"
    pub content: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct LlamaChatResponse {
    pub choices: Vec<LlamaChoice>,
}

#[derive(Debug, Deserialize)]
pub struct LlamaChoice {
    pub message: LlamaMessageOut,
}

#[derive(Debug, Deserialize)]
pub struct LlamaMessageOut {
    pub role: String,
    pub content: String,
}

/// Send a chat prompt to llama.cpp server (OpenAI-compatible endpoint)
/// and return the assistant's content.
pub async fn chat(
    config: &AppConfig,
    system_prompt: Option<&str>,
    user_prompt: &str,
) -> Result<String, AppError> {
    // llama.cpp server is typically OpenAI-compatible:
    // POST {base}/chat/completions
    let url = format!(
        "{}/chat/completions",
        config.llm_base_url.trim_end_matches('/')
    );

    // Build messages
    let mut messages: Vec<LlamaMessage<'_>> = Vec::new();
    if let Some(sys) = system_prompt {
        if !sys.is_empty() {
            messages.push(LlamaMessage {
                role: "system",
                content: sys,
            });
        }
    }
    messages.push(LlamaMessage {
        role: "user",
        content: user_prompt,
    });

    let req = LlamaChatRequest {
        model: &config.llm_model,
        messages,
        temperature: Some(0.2),
        max_tokens: Some(512),
        stream: Some(false),
    };

    // Headers
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/json"),
    );

    // If you always have a token, keep this.
    // If token may be empty, you can guard it:
    // if !config.llm_token.trim().is_empty() { ... }
    let token = header::HeaderValue::from_str(&format!("Bearer {}", &config.llm_token))
        .map_err(|e| AppError::internal(format!("{}", e)))?;
    headers.insert(header::AUTHORIZATION, token);

    let client = Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| AppError::internal(format!("{}", e)))?;

    // Send
    let resp = client
        .post(url)
        .json(&req)
        .send()
        .await
        .map_err(|e| AppError::internal(format!("{}", e)))?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(AppError::internal(format!(
            "llama.cpp returned {}: {}",
            status, body
        )));
    }

    // Parse
    let parsed: LlamaChatResponse = serde_json::from_str(&body)?;
    let answer = parsed
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .unwrap_or_default();

    if answer.trim().is_empty() {
        return Err(AppError::internal("Chat answer is empty"));
    }

    Ok(answer)
}
