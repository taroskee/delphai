use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Instant;

use super::provider::{CitizenResponse, LlmError, LlmProvider};

const DEFAULT_BASE_URL: &str = "https://api.player2.ai/v1";
const DEFAULT_MODEL: &str = "player2-default";

pub struct Player2Provider {
    client: reqwest::Client,
    base_url: String,
    model: String,
}

impl Player2Provider {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: DEFAULT_BASE_URL.to_string(),
            model: DEFAULT_MODEL.to_string(),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    pub fn parse_citizen_response(text: &str) -> Result<CitizenResponse, LlmError> {
        let value: Value = serde_json::from_str(text)?;
        let obj = match &value {
            Value::Array(arr) => arr
                .first()
                .ok_or_else(|| LlmError::Provider("empty response array".into()))?
                .clone(),
            _ => value,
        };
        serde_json::from_value(obj)
            .map_err(|e| LlmError::Provider(format!("invalid citizen JSON: {e}")))
    }
}

impl Default for Player2Provider {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    response_format: ResponseFormat,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    kind: &'static str,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: MessageContent,
}

#[derive(Deserialize)]
struct MessageContent {
    content: String,
}

#[async_trait]
impl LlmProvider for Player2Provider {
    fn name(&self) -> &str {
        "player2"
    }

    async fn generate(&self, prompt: &str) -> Result<CitizenResponse, LlmError> {
        let url = format!("{}/chat/completions", self.base_url);
        let body = ChatRequest {
            model: &self.model,
            messages: vec![ChatMessage {
                role: "user",
                content: prompt,
            }],
            response_format: ResponseFormat {
                kind: "json_object",
            },
        };

        let start = Instant::now();
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;
        let elapsed = start.elapsed();

        let chat: ChatResponse = resp.json().await?;
        let text = chat
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| LlmError::Provider("empty choices".into()))?;

        eprintln!("[player2] latency={:.0}ms", elapsed.as_millis());

        Self::parse_citizen_response(&text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_json_object() {
        let json = r#"{
            "speech": "おはよう",
            "inner_thought": "眠い",
            "action": "wake_up",
            "emotion_change": "neutral",
            "tech_hint": null
        }"#;
        let r = Player2Provider::parse_citizen_response(json).unwrap();
        assert_eq!(r.speech, "おはよう");
        assert_eq!(r.action, "wake_up");
        assert!(r.tech_hint.is_none());
    }

    #[test]
    fn parse_array_takes_first_element() {
        let json = r#"[
            {"speech":"A","inner_thought":"a","action":"idle","emotion_change":"happy","tech_hint":null},
            {"speech":"B","inner_thought":"b","action":"talk","emotion_change":"sad","tech_hint":null}
        ]"#;
        let r = Player2Provider::parse_citizen_response(json).unwrap();
        assert_eq!(r.speech, "A");
    }

    #[test]
    fn parse_empty_array_is_error() {
        let result = Player2Provider::parse_citizen_response("[]");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("empty"), "expected 'empty' in error: {err}");
    }

    #[test]
    fn parse_invalid_json_is_error() {
        let result = Player2Provider::parse_citizen_response("not json");
        assert!(result.is_err());
    }

    #[test]
    fn parse_valid_json_but_wrong_schema_is_error() {
        let result = Player2Provider::parse_citizen_response(r#"{"foo": "bar"}"#);
        assert!(result.is_err());
    }
}
