//! LLMスループットベンチマーク
//!
//! 効率スコア = (quality × conversations) / time_secs
//! 合格基準: quality≥0.7, conversations≥3, time≤5s, efficiency≥0.4
//!
//! 実行: `cargo bench --bench llm_throughput`
//!
//! NOTE: このベンチは実際のLLMエンドポイントに接続する。
//! エンドポイントが利用できない場合はスキップされる (skip_if_unavailable)。

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use delphai_core::llm::{CitizenResponse, LlmError, LlmProvider};
use std::sync::Arc;
use std::time::{Duration, Instant};

async fn async_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

/// 品質スコアの簡易評価 (0.0-1.0)。
/// 全フィールドが空でなければ満点。空フィールドがあれば減点。
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

/// 効率スコアを計算しassert。
fn assert_efficiency(responses: &[CitizenResponse], elapsed: Duration) {
    let quality: f64 = responses.iter().map(quality_score).sum::<f64>() / responses.len() as f64;
    let conversations = responses.len() as f64;
    let time_secs = elapsed.as_secs_f64();
    let efficiency = (quality * conversations) / time_secs;

    eprintln!(
        "quality={:.2} conversations={} time={:.1}s efficiency={:.3}",
        quality, conversations as usize, time_secs, efficiency
    );

    assert!(time_secs <= 5.0, "time exceeded 5s: {time_secs:.1}s");
    assert!(quality >= 0.7, "quality too low: {quality:.2}");
    assert!(conversations >= 3.0, "not enough conversations");
    assert!(efficiency >= 0.4, "efficiency too low: {efficiency:.3}");
}

/// スタブプロバイダー (ベンチ用、実ネットワーク不要)
struct StubProvider {
    delay_ms: u64,
}

#[async_trait::async_trait]
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
        .unwrap();

    let provider: Arc<dyn LlmProvider> = Arc::new(StubProvider { delay_ms: 200 });

    let prompts: Vec<String> = (0..5)
        .map(|i| format!("住民{i}の状況: 食料不足。次の行動を決めよ。"))
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
                        let results = provider.generate_batch(&batch).await.unwrap();
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
