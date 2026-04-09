use super::provider::{CitizenResponse, LlmError};
use serde_json::Value;
use serde_yaml;

/// Format-agnostic response parser trait.
/// Implement this for JSON, YAML, or any other serialization format.
pub trait ResponseParser: Send + Sync {
    fn parse_response(&self, text: &str) -> Result<CitizenResponse, LlmError>;
    fn parse_batch_response(
        &self,
        text: &str,
        expected_count: usize,
    ) -> Result<Vec<CitizenResponse>, LlmError>;
}

/// JSON implementation of ResponseParser.
pub struct JsonResponseParser;

impl ResponseParser for JsonResponseParser {
    fn parse_response(&self, text: &str) -> Result<CitizenResponse, LlmError> {
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
            .map_err(|e| LlmError::Provider(format!("invalid citizen response: {e}")))
    }

    fn parse_batch_response(
        &self,
        text: &str,
        expected_count: usize,
    ) -> Result<Vec<CitizenResponse>, LlmError> {
        let cleaned = strip_markdown_fence(text.trim());
        let value: Value = serde_json::from_str(&cleaned)?;

        let arr = match value {
            Value::Array(arr) => arr,
            Value::Object(_) => vec![value],
            _ => return Err(LlmError::Provider("expected array or object".into())),
        };

        if arr.is_empty() {
            return Err(LlmError::Provider("empty response array".into()));
        }

        let responses: Vec<CitizenResponse> = arr
            .into_iter()
            .enumerate()
            .map(|(i, v)| {
                serde_json::from_value(v).unwrap_or_else(|e| {
                    eprintln!("[warn] batch entry {i} malformed, using default: {e}");
                    CitizenResponse::default()
                })
            })
            .collect();

        if responses.len() != expected_count {
            return Err(LlmError::Provider(format!(
                "expected {} responses, got {}",
                expected_count,
                responses.len()
            )));
        }

        Ok(responses)
    }
}

/// Backward-compatible free function: delegates to JsonResponseParser.
pub fn parse_response(text: &str) -> Result<CitizenResponse, LlmError> {
    JsonResponseParser.parse_response(text)
}

/// Backward-compatible free function: delegates to JsonResponseParser.
pub fn parse_batch_response(
    text: &str,
    expected_count: usize,
) -> Result<Vec<CitizenResponse>, LlmError> {
    JsonResponseParser.parse_batch_response(text, expected_count)
}

/// YAML implementation of ResponseParser.
pub struct YamlResponseParser;

impl ResponseParser for YamlResponseParser {
    fn parse_response(&self, text: &str) -> Result<CitizenResponse, LlmError> {
        let cleaned = strip_markdown_fence(text.trim());
        serde_yaml::from_str(&cleaned)
            .map_err(|e| LlmError::Provider(format!("YAML parse error: {e}")))
    }

    fn parse_batch_response(
        &self,
        text: &str,
        expected_count: usize,
    ) -> Result<Vec<CitizenResponse>, LlmError> {
        let cleaned = strip_markdown_fence(text.trim());

        // Try YAML sequence first (list of mappings)
        let docs: serde_yaml::Value = serde_yaml::from_str(&cleaned)
            .map_err(|e| LlmError::Provider(format!("YAML parse error: {e}")))?;

        let responses: Vec<CitizenResponse> = match docs {
            serde_yaml::Value::Sequence(seq) => seq
                .into_iter()
                .enumerate()
                .map(|(i, v)| {
                    serde_yaml::from_value(v).unwrap_or_else(|e| {
                        eprintln!("[warn] yaml batch entry {i} malformed, using default: {e}");
                        CitizenResponse::default()
                    })
                })
                .collect(),
            serde_yaml::Value::Mapping(_) => {
                let r: CitizenResponse = serde_yaml::from_value(docs)
                    .map_err(|e| LlmError::Provider(format!("invalid citizen response: {e}")))?;
                vec![r]
            }
            _ => return Err(LlmError::Provider("expected YAML sequence or mapping".into())),
        };

        if responses.is_empty() {
            return Err(LlmError::Provider("empty YAML response".into()));
        }

        if responses.len() != expected_count {
            return Err(LlmError::Provider(format!(
                "expected {} responses, got {}",
                expected_count,
                responses.len()
            )));
        }

        Ok(responses)
    }
}

/// Strip markdown code fences if present (```json/```yaml/``` ... ```).
fn strip_markdown_fence(text: &str) -> String {
    let trimmed = text.trim();
    for prefix in &["```json", "```yaml", "```"] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            if let Some(inner) = rest.strip_suffix("```") {
                return inner.trim().to_string();
            }
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

    // --- trait-based tests ---

    #[test]
    fn trait_parse_plain_object() {
        let parser = JsonResponseParser;
        let r = parser.parse_response(valid_json()).unwrap();
        assert_eq!(r.speech, "hello");
    }

    #[test]
    fn trait_parse_batch() {
        let parser = JsonResponseParser;
        let json = format!(
            "[{},{}]",
            valid_json(),
            valid_json().replace("hello", "bye")
        );
        let r = parser.parse_batch_response(&json, 2).unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].speech, "hello");
        assert_eq!(r[1].speech, "bye");
    }

    #[test]
    fn trait_object_is_dyn_compatible() {
        let parser: Box<dyn ResponseParser> = Box::new(JsonResponseParser);
        let r = parser.parse_response(valid_json()).unwrap();
        assert_eq!(r.speech, "hello");
    }

    // --- free function backward compat ---

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

    // --- batch ---

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
    fn batch_parse_malformed_entry_uses_default_not_fail() {
        // One valid + one malformed object → should return 2 results, not fail.
        let malformed = r#"{"speech":"ok","inner_thought":"","action":"","emotion_change":"","tech_hint":null}"#;
        let bad = r#"{"not_a_valid_field": 123}"#;
        let json = format!("[{malformed},{bad}]");
        let r = parse_batch_response(&json, 2).unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].speech, "ok");
        // malformed entry gets default values
        assert_eq!(r[1].speech, "");
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

    #[test]
    fn strip_fence_with_yaml_lang() {
        assert_eq!(
            strip_markdown_fence("```yaml\nkey: value\n```"),
            "key: value"
        );
    }

    // --- YamlResponseParser ---

    fn valid_yaml() -> &'static str {
        "speech: hello\ninner_thought: thinking\naction: idle\nemotion_change: neutral"
    }

    #[test]
    fn yaml_parse_single_response() {
        let parser = YamlResponseParser;
        let r = parser.parse_response(valid_yaml()).unwrap();
        assert_eq!(r.speech, "hello");
        assert_eq!(r.inner_thought, "thinking");
        assert_eq!(r.action, "idle");
        assert_eq!(r.emotion_change, "neutral");
    }

    #[test]
    fn yaml_parse_batch_as_sequence() {
        let parser = YamlResponseParser;
        let yaml = "- speech: hello\n  inner_thought: t1\n  action: a1\n  emotion_change: e1\n- speech: bye\n  inner_thought: t2\n  action: a2\n  emotion_change: e2";
        let r = parser.parse_batch_response(yaml, 2).unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].speech, "hello");
        assert_eq!(r[1].speech, "bye");
    }

    #[test]
    fn yaml_parse_markdown_fenced() {
        let parser = YamlResponseParser;
        let fenced = format!("```yaml\n{}\n```", valid_yaml());
        let r = parser.parse_response(&fenced).unwrap();
        assert_eq!(r.speech, "hello");
    }

    #[test]
    fn yaml_parser_is_dyn_compatible() {
        let parser: Box<dyn ResponseParser> = Box::new(YamlResponseParser);
        let r = parser.parse_response(valid_yaml()).unwrap();
        assert_eq!(r.speech, "hello");
    }

    #[test]
    fn yaml_parse_invalid_errors() {
        let parser = YamlResponseParser;
        assert!(parser.parse_response("not: valid: yaml: :").is_err());
    }

    #[test]
    fn yaml_parse_wrong_schema_errors() {
        let parser = YamlResponseParser;
        assert!(parser.parse_response("foo: bar").is_err());
    }

    #[test]
    fn yaml_parse_batch_single_mapping() {
        let parser = YamlResponseParser;
        let r = parser.parse_batch_response(valid_yaml(), 1).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].speech, "hello");
    }

    #[test]
    fn yaml_parse_batch_count_mismatch_errors() {
        let parser = YamlResponseParser;
        let yaml = format!("- {}", valid_yaml().replace('\n', "\n  "));
        let r = parser.parse_batch_response(&yaml, 3);
        assert!(r.is_err());
        assert!(r.unwrap_err().to_string().contains("expected 3"));
    }
}
