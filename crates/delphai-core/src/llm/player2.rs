use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use super::provider::{CitizenResponse, LlmError, LlmProvider};
use super::response_parser::{ResponseParser, YamlResponseParser};

const DEFAULT_BASE_URL: &str = "https://api.player2.ai/v1";
const DEFAULT_MODEL: &str = "player2-default";

pub struct Player2Provider {
    client: reqwest::Client,
    base_url: String,
    model: String,
    parser: Box<dyn ResponseParser>,
}

impl Player2Provider {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: DEFAULT_BASE_URL.to_string(),
            model: DEFAULT_MODEL.to_string(),
            parser: Box::new(YamlResponseParser),
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

    pub fn with_parser(mut self, parser: Box<dyn ResponseParser>) -> Self {
        self.parser = parser;
        self
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
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
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

    async fn generate_text(&self, prompt: &str) -> Result<String, LlmError> {
        let url = format!("{}/chat/completions", self.base_url);
        let body = ChatRequest {
            model: &self.model,
            messages: vec![ChatMessage {
                role: "user",
                content: prompt,
            }],
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
        Ok(text)
    }

    /// Override default to use injected parser instead of hardcoded JSON.
    async fn generate(&self, prompt: &str) -> Result<CitizenResponse, LlmError> {
        let text = self.generate_text(prompt).await?;
        self.parser.parse_response(&text)
    }

    /// Override default to use injected parser instead of hardcoded JSON.
    async fn generate_packed(
        &self,
        prompt: &str,
        expected_count: usize,
    ) -> Result<Vec<CitizenResponse>, LlmError> {
        let text = self.generate_text(prompt).await?;
        self.parser.parse_batch_response(&text, expected_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_uses_injected_parser() {
        use super::super::response_parser::JsonResponseParser;
        let p = Player2Provider::new().with_parser(Box::new(JsonResponseParser));
        let r = p
            .parser
            .parse_response(
                r#"{"speech":"おはよう","inner_thought":"眠い","action":"wake_up","emotion_change":"neutral","tech_hint":null}"#,
            )
            .unwrap();
        assert_eq!(r.speech, "おはよう");
    }
}
