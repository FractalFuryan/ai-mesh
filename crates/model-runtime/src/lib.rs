use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("empty response from model runtime")]
    EmptyResponse,
}

#[derive(Debug, Clone)]
pub struct LlamaRuntime {
    client: Client,
    pub base_url: String,
    pub model_name: String,
}

impl LlamaRuntime {
    pub fn new(base_url: impl Into<String>, model_name: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into(),
            model_name: model_name.into(),
        }
    }

    pub async fn chat(&self, prompt: &str) -> Result<String, RuntimeError> {
        let req = ChatCompletionRequest {
            model: self.model_name.clone(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            temperature: Some(0.0), // deterministic for testing
            max_tokens: Some(512),
        };

        let url = format!(
            "{}/v1/chat/completions",
            self.base_url.trim_end_matches('/')
        );

        let resp: ChatCompletionResponse = self
            .client
            .post(url)
            .json(&req)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        resp.choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .filter(|s| !s.trim().is_empty())
            .ok_or(RuntimeError::EmptyResponse)
    }
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: AssistantMessage,
}

#[derive(Debug, Deserialize)]
struct AssistantMessage {
    content: String,
}
