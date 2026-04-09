use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::response_parser::{JsonResponseParser, ResponseParser};

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("Provider error: {0}")]
    Provider(String),
}

impl From<serde_json::Error> for LlmError {
    fn from(e: serde_json::Error) -> Self {
        LlmError::Parse(e.to_string())
    }
}

/// LLM output JSON schema for a single citizen.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct CitizenResponse {
    pub speech: String,
    pub inner_thought: String,
    pub action: String,
    pub emotion_change: String,
    pub tech_hint: Option<String>,
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn name(&self) -> &str;

    /// Low-level: send a prompt to the LLM and return the raw response text.
    /// This is the only method providers MUST implement.
    async fn generate_text(&self, prompt: &str) -> Result<String, LlmError>;

    /// High-level: send a prompt and parse it into a single CitizenResponse.
    /// Default impl uses `JsonResponseParser`. Override to use a different parser.
    async fn generate(&self, prompt: &str) -> Result<CitizenResponse, LlmError> {
        let text = self.generate_text(prompt).await?;
        JsonResponseParser.parse_response(&text)
    }

    /// High-level: send a packed prompt (containing multiple citizen pairs) and
    /// parse `expected_count` `CitizenResponse`s from a single LLM call.
    /// Default impl uses `JsonResponseParser`. Override to use a different parser.
    async fn generate_packed(
        &self,
        prompt: &str,
        expected_count: usize,
    ) -> Result<Vec<CitizenResponse>, LlmError> {
        let text = self.generate_text(prompt).await?;
        JsonResponseParser.parse_batch_response(&text, expected_count)
    }

    /// Sequential N-prompt batch (each prompt → one response). Default: serial.
    /// Providers may override to do parallel HTTP.
    async fn generate_batch(&self, prompts: &[String]) -> Result<Vec<CitizenResponse>, LlmError> {
        let mut results = Vec::with_capacity(prompts.len());
        for prompt in prompts {
            results.push(self.generate(prompt).await?);
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn citizen_response_roundtrip_serde() {
        let original = CitizenResponse {
            speech: "hello".into(),
            inner_thought: "thinking".into(),
            action: "idle".into(),
            emotion_change: "neutral".into(),
            tech_hint: None,
        };
        let json = serde_json::to_string(&original).unwrap();
        let restored: CitizenResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn citizen_response_with_tech_hint() {
        let json = r#"{
            "speech": "fire!",
            "inner_thought": "hot",
            "action": "discover",
            "emotion_change": "excited",
            "tech_hint": "fire_making"
        }"#;
        let r: CitizenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(r.tech_hint, Some("fire_making".into()));
    }

    #[test]
    fn citizen_response_missing_field_is_error() {
        let json = r#"{"speech": "hello"}"#;
        let result = serde_json::from_str::<CitizenResponse>(json);
        assert!(result.is_err());
    }

    // --- default trait impls (generate / generate_packed) ---

    struct StubProvider {
        text: String,
    }

    #[async_trait]
    impl LlmProvider for StubProvider {
        fn name(&self) -> &str {
            "stub"
        }
        async fn generate_text(&self, _prompt: &str) -> Result<String, LlmError> {
            Ok(self.text.clone())
        }
    }

    fn one_json() -> &'static str {
        r#"{"speech":"a","inner_thought":"b","action":"c","emotion_change":"d","tech_hint":null}"#
    }

    #[tokio::test]
    async fn default_generate_parses_json_object() {
        let p = StubProvider { text: one_json().into() };
        let r = p.generate("anything").await.unwrap();
        assert_eq!(r.speech, "a");
    }

    #[tokio::test]
    async fn default_generate_packed_parses_array() {
        let json = format!("[{},{}]", one_json(), one_json().replace("\"a\"", "\"x\""));
        let p = StubProvider { text: json };
        let r = p.generate_packed("anything", 2).await.unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].speech, "a");
        assert_eq!(r[1].speech, "x");
    }

    #[tokio::test]
    async fn default_generate_packed_count_mismatch_errors() {
        let json = format!("[{}]", one_json());
        let p = StubProvider { text: json };
        let result = p.generate_packed("anything", 3).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("expected 3"));
    }

    #[tokio::test]
    async fn default_generate_batch_is_sequential() {
        let p = StubProvider { text: one_json().into() };
        let prompts = vec!["p1".into(), "p2".into(), "p3".into()];
        let r = p.generate_batch(&prompts).await.unwrap();
        assert_eq!(r.len(), 3);
    }
}
