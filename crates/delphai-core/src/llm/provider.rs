use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Provider error: {0}")]
    Provider(String),
}

/// LLM output JSON schema for a single citizen.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

    async fn generate(&self, prompt: &str) -> Result<CitizenResponse, LlmError>;

    /// Batch generation. Default: sequential. Providers may override.
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
}
