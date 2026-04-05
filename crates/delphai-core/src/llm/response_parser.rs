use super::provider::{CitizenResponse, LlmError};
use serde_json::Value;

/// Parse a single CitizenResponse from raw LLM output text.
///
/// Handles:
/// - Plain JSON object
/// - JSON array (takes first element)
/// - Markdown-fenced JSON (```json ... ```)
/// - Trailing/leading whitespace
pub fn parse_response(text: &str) -> Result<CitizenResponse, LlmError> {
    let cleaned = strip_markdown_fence(text.trim());
    let value: Value = serde_json::from_str(&cleaned)?;

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

/// Parse a batch response: expects a JSON array of CitizenResponse objects.
///
/// Handles:
/// - JSON array
/// - Markdown-fenced JSON
/// - If a single object is returned instead of array, wraps it in a Vec
pub fn parse_batch_response(text: &str, expected_count: usize) -> Result<Vec<CitizenResponse>, LlmError> {
    let cleaned = strip_markdown_fence(text.trim());
    let value: Value = serde_json::from_str(&cleaned)?;

    let arr = match value {
        Value::Array(arr) => arr,
        Value::Object(_) => vec![value],
        _ => return Err(LlmError::Provider("expected JSON array or object".into())),
    };

    if arr.is_empty() {
        return Err(LlmError::Provider("empty response array".into()));
    }

    let results: Result<Vec<CitizenResponse>, _> = arr
        .into_iter()
        .map(|v| {
            serde_json::from_value(v)
                .map_err(|e| LlmError::Provider(format!("invalid citizen JSON: {e}")))
        })
        .collect();

    let responses = results?;

    if responses.len() != expected_count {
        return Err(LlmError::Provider(format!(
            "expected {} responses, got {}",
            expected_count,
            responses.len()
        )));
    }

    Ok(responses)
}

/// Strip markdown code fences if present (```json ... ``` or ``` ... ```).
fn strip_markdown_fence(text: &str) -> String {
    let trimmed = text.trim();
    if let Some(rest) = trimmed.strip_prefix("```json") {
        if let Some(inner) = rest.strip_suffix("```") {
            return inner.trim().to_string();
        }
    }
    if let Some(rest) = trimmed.strip_prefix("```") {
        if let Some(inner) = rest.strip_suffix("```") {
            return inner.trim().to_string();
        }
    }
    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_json() -> &'static str {
        r#"{"speech":"hello","inner_thought":"thinking","action":"idle","emotion_change":"neutral","tech_hint":null}"#
    }

    // --- parse_response ---

    #[test]
    fn parse_plain_object() {
        let r = parse_response(valid_json()).unwrap();
        assert_eq!(r.speech, "hello");
    }

    #[test]
    fn parse_array_takes_first() {
        let json = format!("[{},{}]", valid_json(), valid_json().replace("hello", "bye"));
        let r = parse_response(&json).unwrap();
        assert_eq!(r.speech, "hello");
    }

    #[test]
    fn parse_empty_array_errors() {
        let r = parse_response("[]");
        assert!(r.is_err());
    }

    #[test]
    fn parse_invalid_json_errors() {
        assert!(parse_response("not json").is_err());
    }

    #[test]
    fn parse_wrong_schema_errors() {
        assert!(parse_response(r#"{"foo":"bar"}"#).is_err());
    }

    #[test]
    fn parse_markdown_fenced_json() {
        let fenced = format!("```json\n{}\n```", valid_json());
        let r = parse_response(&fenced).unwrap();
        assert_eq!(r.speech, "hello");
    }

    #[test]
    fn parse_markdown_fenced_no_lang() {
        let fenced = format!("```\n{}\n```", valid_json());
        let r = parse_response(&fenced).unwrap();
        assert_eq!(r.speech, "hello");
    }

    #[test]
    fn parse_with_leading_trailing_whitespace() {
        let padded = format!("  \n {} \n  ", valid_json());
        let r = parse_response(&padded).unwrap();
        assert_eq!(r.speech, "hello");
    }

    // --- parse_batch_response ---

    #[test]
    fn batch_parse_array_of_two() {
        let json = format!(
            "[{},{}]",
            valid_json(),
            valid_json().replace("hello", "bye")
        );
        let r = parse_batch_response(&json, 2).unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].speech, "hello");
        assert_eq!(r[1].speech, "bye");
    }

    #[test]
    fn batch_parse_single_object_wraps_in_vec() {
        let r = parse_batch_response(valid_json(), 1).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].speech, "hello");
    }

    #[test]
    fn batch_parse_count_mismatch_errors() {
        let json = format!("[{}]", valid_json());
        let r = parse_batch_response(&json, 3);
        assert!(r.is_err());
        assert!(r.unwrap_err().to_string().contains("expected 3"));
    }

    #[test]
    fn batch_parse_empty_array_errors() {
        assert!(parse_batch_response("[]", 1).is_err());
    }

    #[test]
    fn batch_parse_markdown_fenced() {
        let json = format!("[{}]", valid_json());
        let fenced = format!("```json\n{}\n```", json);
        let r = parse_batch_response(&fenced, 1).unwrap();
        assert_eq!(r[0].speech, "hello");
    }

    // --- strip_markdown_fence ---

    #[test]
    fn strip_fence_no_fence() {
        assert_eq!(strip_markdown_fence("hello"), "hello");
    }

    #[test]
    fn strip_fence_with_json_lang() {
        assert_eq!(strip_markdown_fence("```json\n{}\n```"), "{}");
    }

    #[test]
    fn strip_fence_without_lang() {
        assert_eq!(strip_markdown_fence("```\n{}\n```"), "{}");
    }
}
