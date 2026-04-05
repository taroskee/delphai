//! LLM throughput benchmark.
//!
//! Efficiency = (quality * conversations) / time_secs
//! Pass criteria: quality >= 0.7, conversations >= 3, time <= 5s, efficiency >= 0.4
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

fn assert_efficiency(responses: &[CitizenResponse], elapsed: Duration) {
    let quality: f64 = responses.iter().map(quality_score).sum::<f64>() / responses.len() as f64;
    let conversations = responses.len() as f64;
    let time_secs = elapsed.as_secs_f64();
    let efficiency = (quality * conversations) / time_secs;

    eprintln!(
        "quality={quality:.2} conversations={} time={time_secs:.1}s efficiency={efficiency:.3}",
        conversations as usize,
    );

    assert!(time_secs <= 5.0, "time exceeded 5s: {time_secs:.1}s");
    assert!(quality >= 0.7, "quality too low: {quality:.2}");
    assert!(conversations >= 3.0, "not enough conversations");
    assert!(efficiency >= 0.4, "efficiency too low: {efficiency:.3}");
}

struct StubProvider {
    delay_ms: u64,
}

#[async_trait]
impl LlmProvider for StubProvider {
    fn name(&self) -> &str {
        "stub"
    }

    async fn generate(&self, _prompt: &str) -> Result<CitizenResponse, LlmError> {
        tokio::time::sleep(Duration::from_millis(self.delay_ms)).await;
        Ok(CitizenResponse {
            speech: "食料を集めよう".into(),
            inner_thought: "腹が減った".into(),
            action: "gather_food".into(),
            emotion_change: "motivated".into(),
            tech_hint: None,
        })
    }
}

fn bench_llm_batch(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime");

    let provider: Arc<dyn LlmProvider> = Arc::new(StubProvider { delay_ms: 200 });

    let prompts: Vec<String> = (0..5)
        .map(|i| format!("citizen {i}: food shortage. decide next action."))
        .collect();

    let mut group = c.benchmark_group("llm_throughput");
    group.measurement_time(Duration::from_secs(30));

    for &batch_size in &[1usize, 3, 5] {
        group.bench_with_input(
            BenchmarkId::new("batch_size", batch_size),
            &batch_size,
            |b, &size| {
                b.to_async(&rt).iter(|| {
                    let provider = Arc::clone(&provider);
                    let batch = prompts[..size].to_vec();
                    async move {
                        let start = Instant::now();
                        let results = provider
                            .generate_batch(&batch)
                            .await
                            .expect("stub provider should not fail");
                        let elapsed = start.elapsed();
                        assert_efficiency(&results, elapsed);
                        results
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_llm_batch);
criterion_main!(benches);
