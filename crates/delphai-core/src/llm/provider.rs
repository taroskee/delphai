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

/// LLM出力のJSONスキーマ。住民1人分。
#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// 単一プロンプトを送信し、住民レスポンスを返す。
    async fn generate(&self, prompt: &str) -> Result<CitizenResponse, LlmError>;

    /// バッチプロンプトを送信し、複数住民のレスポンスを返す。
    /// デフォルト実装は逐次実行。プロバイダーが対応する場合はオーバーライドする。
    async fn generate_batch(&self, prompts: &[String]) -> Result<Vec<CitizenResponse>, LlmError> {
        let mut results = Vec::with_capacity(prompts.len());
        for prompt in prompts {
            results.push(self.generate(prompt).await?);
        }
        Ok(results)
    }
}
