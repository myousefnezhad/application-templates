use app_config::AppConfig;
use app_error::AppError;
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct EmbeddingsRequest<'a> {
    pub input: &'a str,
    pub model: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct EmbeddingsResponse {
    pub data: Vec<EmbeddingData>,
}

#[derive(Debug, Deserialize)]
pub struct EmbeddingData {
    pub embedding: Vec<f32>,
}

pub async fn embedding(config: &AppConfig, input: &str) -> Result<Vec<f32>, AppError> {
    let url = format!("{}/embeddings", &config.rag_base_url.trim_end_matches('/'));
    let req = EmbeddingsRequest {
        model: &config.rag_model,
        input,
    };
    let mut headers = header::HeaderMap::new();
    let token = match header::HeaderValue::from_str(&format!("Bearer {}", &config.rag_token)) {
        Ok(t) => t,
        Err(e) => return Err(AppError::internal(format!("{}", &e))),
    };
    headers.insert(header::AUTHORIZATION, token);
    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/json"),
    );
    let client = match Client::builder().default_headers(headers).build() {
        Ok(c) => c,
        Err(e) => return Err(AppError::internal(format!("{}", &e))),
    };
    let resp = match client.post(url).json(&req).send().await {
        Ok(r) => r,
        Err(e) => return Err(AppError::internal(format!("{}", &e))),
    };
    let status = &resp.status();
    let body = &resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(AppError::internal(format!(
            "server returned {}: {}",
            &status, body
        )));
    }
    let parsed: EmbeddingsResponse = serde_json::from_str(&body)?;
    let embedding = parsed
        .data
        .into_iter()
        .next()
        .map(|d| d.embedding)
        .unwrap_or_default();

    if embedding.len() == 0 {
        return Err(AppError::internal("Embedding is empty"));
    }
    Ok(embedding)
}
