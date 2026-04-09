//! LLM throughput benchmark — sequential vs packed inference.
//!
//! Two modes per batch size:
//!   - sequential: N independent `generate()` calls (one prompt → one response, N times)
//!   - packed:     1 `generate_packed()` call (one packed prompt → N responses)
//!
//! Realistic stub model:
//!   total_latency = base_ms + per_response_ms * num_responses
//!
//! This reflects an LLM where each call has fixed network/loading overhead (`base_ms`)
//! and token generation cost scales with output size.
//!
//! Run: `cargo bench --bench llm_throughput`

use async_trait::async_trait;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use delphai_core::llm::{CitizenResponse, LlmError, LlmProvider};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn quality_score(r: &CitizenResponse) -> f64 {
    let mut score = 0.0_f64;
    if !r.speech.is_empty() {
        score += 0.4;
    }
    if !r.inner_thought.is_empty() {
        score += 0.3;
    }
    if !r.action.is_empty() {
        score += 0.2;
    }
    if !r.emotion_change.is_empty() {
        score += 0.1;
    }
    score
}

fn report_efficiency(label: &str, responses: &[CitizenResponse], elapsed: Duration) {
    let quality: f64 = responses.iter().map(quality_score).sum::<f64>() / responses.len() as f64;
    let conversations = responses.len() as f64;
    let time_secs = elapsed.as_secs_f64();
    let efficiency = (quality * conversations) / time_secs;
    eprintln!(
        "[{label}] quality={quality:.2} convs={} time={time_secs:.3}s efficiency={efficiency:.3}",
        conversations as usize,
    );
}

/// Stub provider that simulates LLM latency: base + per_response × N.
/// Detects packed prompts by looking for the "YAML sequence of N mappings" instruction
/// emitted by `build_batch_conversation_prompt` to know how many responses to return.
struct StubProvider {
    base_ms: u64,
    per_response_ms: u64,
}

fn one_object() -> &'static str {
    "speech: 食料を集めよう\ninner_thought: 腹が減った\naction: gather_food\nemotion_change: motivated"
}

fn detect_packed_count(prompt: &str) -> Option<usize> {
    // Match: "YAML sequence of N mappings"
    let needle = "YAML sequence of ";
    let start = prompt.find(needle)? + needle.len();
    let rest = &prompt[start..];
    let end = rest.find(' ')?;
    rest[..end].parse().ok()
}

#[async_trait]
impl LlmProvider for StubProvider {
    fn name(&self) -> &str {
        "stub"
    }

    async fn generate_text(&self, prompt: &str) -> Result<String, LlmError> {
        let count = detect_packed_count(prompt).unwrap_or(1);
        let total_ms = self.base_ms + self.per_response_ms * count as u64;
        tokio::time::sleep(Duration::from_millis(total_ms)).await;

        if count == 1 {
            Ok(one_object().to_string())
        } else {
            // YAML sequence: each entry prefixed with "- " and fields indented
            let entry = one_object().replace('\n', "\n  ");
            let arr = (0..count)
                .map(|_| format!("- {entry}"))
                .collect::<Vec<_>>()
                .join("\n");
            Ok(arr)
        }
    }
}

fn build_packed_prompt(n: usize) -> String {
    // Mimics the trailing instruction emitted by build_batch_conversation_prompt
    // so the stub can detect the count.
    format!("simulate {n} citizens.\nRespond with a YAML sequence of {n} mappings, one per pair.")
}

fn bench_llm_batch(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime");

    // Realistic-ish stub: 50ms HTTP overhead + 100ms per response generation.
    let provider: Arc<dyn LlmProvider> = Arc::new(StubProvider {
        base_ms: 50,
        per_response_ms: 100,
    });

    let single_prompts: Vec<String> = (0..5)
        .map(|i| format!("citizen {i}: food shortage. decide next action."))
        .collect();

    let mut group = c.benchmark_group("llm_throughput");
    group.measurement_time(Duration::from_secs(20));

    for &n in &[1usize, 3, 5] {
        // --- sequential mode: N × generate() ---
        group.bench_with_input(BenchmarkId::new("sequential", n), &n, |b, &size| {
            b.to_async(&rt).iter(|| {
                let provider = Arc::clone(&provider);
                let batch = single_prompts[..size].to_vec();
                async move {
                    let start = Instant::now();
                    let results = provider
                        .generate_batch(&batch)
                        .await
                        .expect("stub should not fail");
                    let elapsed = start.elapsed();
                    report_efficiency("seq", &results, elapsed);
                    results
                }
            });
        });

        // --- packed mode: 1 × generate_packed() ---
        group.bench_with_input(BenchmarkId::new("packed", n), &n, |b, &size| {
            let packed = build_packed_prompt(size);
            b.to_async(&rt).iter(|| {
                let provider = Arc::clone(&provider);
                let prompt = packed.clone();
                async move {
                    let start = Instant::now();
                    let results = provider
                        .generate_packed(&prompt, size)
                        .await
                        .expect("stub should not fail");
                    let elapsed = start.elapsed();
                    report_efficiency("packed", &results, elapsed);
                    results
                }
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_llm_batch);
criterion_main!(benches);

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;
    #[allow(unused_imports)]
    use std::time::Instant;

    #[test]
    fn detect_packed_count_finds_n() {
        assert_eq!(
            detect_packed_count("Respond with a YAML sequence of 3 mappings, one per pair."),
            Some(3)
        );
    }

    #[test]
    fn detect_packed_count_returns_none_for_single_prompt() {
        assert_eq!(detect_packed_count("just a single citizen prompt"), None);
    }

    #[tokio::test]
    async fn stub_returns_array_for_packed_prompt() {
        let p = StubProvider {
            base_ms: 0,
            per_response_ms: 0,
        };
        let result = p.generate_packed(&build_packed_prompt(3), 3).await.unwrap();
        assert_eq!(result.len(), 3);
    }

    #[tokio::test]
    async fn stub_returns_single_for_simple_prompt() {
        let p = StubProvider {
            base_ms: 0,
            per_response_ms: 0,
        };
        let result = p.generate("simple prompt").await.unwrap();
        assert_eq!(result.speech, "食料を集めよう");
    }

    #[tokio::test]
    async fn packed_is_faster_than_sequential_for_n_3() {
        let p = StubProvider {
            base_ms: 50,
            per_response_ms: 10,
        };
        let prompts: Vec<String> = (0..3).map(|i| format!("p{i}")).collect();

        let t0 = Instant::now();
        let _ = p.generate_batch(&prompts).await.unwrap();
        let seq_elapsed = t0.elapsed();

        let t0 = Instant::now();
        let _ = p.generate_packed(&build_packed_prompt(3), 3).await.unwrap();
        let packed_elapsed = t0.elapsed();

        // sequential: 3 × (50 + 10) = 180ms; packed: 1 × (50 + 30) = 80ms
        assert!(
            packed_elapsed < seq_elapsed,
            "packed {:?} should be faster than sequential {:?}",
            packed_elapsed,
            seq_elapsed
        );
    }
}
