//! OpenAI-compatible provider adapter.
//!
//! The adapter only accepts an operator-configured base URL. Callers cannot supply
//! an arbitrary URL, which keeps request routing out of untrusted workload input.

use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use thiserror::Error;

#[derive(Clone)]
pub struct ProviderClient {
    client: Client,
    endpoint: Url,
    api_key: Option<String>,
    default_model: String,
}

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("invalid provider base URL")]
    InvalidBaseUrl,
    #[error("provider request failed")]
    Transport(#[from] reqwest::Error),
    #[error("provider returned HTTP status {0}")]
    UpstreamStatus(u16),
    #[error("requested model is not the operator-configured model")]
    ModelNotAllowed,
    #[error("provider response did not contain choices[0].message.content")]
    InvalidResponse,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct TokenUsage {
    #[serde(default)]
    pub prompt_tokens: Option<u64>,
    #[serde(default)]
    pub completion_tokens: Option<u64>,
    #[serde(default)]
    pub total_tokens: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct ProviderCompletion {
    pub provider_response: Value,
    pub content: String,
    pub model: String,
    pub usage: Option<TokenUsage>,
}

impl ProviderClient {
    pub fn new(
        base_url: &str,
        api_key: Option<String>,
        default_model: impl Into<String>,
        timeout: Duration,
    ) -> Result<Self, ProviderError> {
        let mut endpoint = Url::parse(base_url).map_err(|_| ProviderError::InvalidBaseUrl)?;
        if !matches!(endpoint.scheme(), "http" | "https")
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
        {
            return Err(ProviderError::InvalidBaseUrl);
        }

        let path = format!("{}/chat/completions", endpoint.path().trim_end_matches('/'));
        endpoint.set_path(&path);

        let client = Client::builder()
            .timeout(timeout)
            .build()
            .map_err(ProviderError::Transport)?;

        Ok(Self {
            client,
            endpoint,
            api_key,
            default_model: default_model.into(),
        })
    }

    pub fn default_model(&self) -> &str {
        &self.default_model
    }

    /// Execute a non-streaming chat completion against the configured provider.
    pub async fn complete(
        &self,
        messages: &[ChatMessage],
        requested_model: Option<&str>,
    ) -> Result<ProviderCompletion, ProviderError> {
        let requested_model = requested_model.filter(|candidate| !candidate.trim().is_empty());
        if requested_model.is_some_and(|candidate| candidate != self.default_model) {
            return Err(ProviderError::ModelNotAllowed);
        }
        let model = &self.default_model;

        let mut request = self
            .client
            .post(self.endpoint.clone())
            .json(&serde_json::json!({
                "model": model,
                "messages": messages,
                "stream": false
            }));

        if let Some(api_key) = &self.api_key {
            request = request.bearer_auth(api_key);
        }

        let response = request.send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(ProviderError::UpstreamStatus(status.as_u16()));
        }

        let provider_response: Value = response.json().await?;
        let content = provider_response
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str)
            .ok_or(ProviderError::InvalidResponse)?
            .to_owned();

        let model = provider_response
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or(model)
            .to_owned();

        let usage = provider_response
            .get("usage")
            .and_then(|value| serde_json::from_value::<TokenUsage>(value.clone()).ok());

        Ok(ProviderCompletion {
            provider_response,
            content,
            model,
            usage,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_endpoint_appends_chat_completions() {
        let client = ProviderClient::new(
            "http://127.0.0.1:11434/v1",
            None,
            "llama3.2",
            Duration::from_secs(5),
        )
        .unwrap();

        assert_eq!(
            client.endpoint.as_str(),
            "http://127.0.0.1:11434/v1/chat/completions"
        );
    }

    #[test]
    fn rejects_non_http_schemes_and_embedded_credentials() {
        assert!(
            ProviderClient::new("file:///etc/passwd", None, "model", Duration::from_secs(5))
                .is_err()
        );
        assert!(
            ProviderClient::new(
                "https://user:password@example.com/v1",
                None,
                "model",
                Duration::from_secs(5)
            )
            .is_err()
        );
    }
}
