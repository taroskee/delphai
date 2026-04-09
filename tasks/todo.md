# MVP TODO

MVPの定義: 住民5-10人がLLMで会話し、プレイヤーの「声」に反応し、技術ツリーが3段階進行するデモ。

---

## リファクタリング（Phase 1着手前に完了）

> ビルド破損・設計不整合・コード重複の修正。既存テストを通しながら順番に実施。

- [x] **[🔴] `delphai-bench/src/lib.rs` 削除**
  - `delphai-core` が `[dev-dependencies]` のため `src/lib.rs` がコンパイル不可
  - `lib.rs` の全コードは `benches/llm_throughput.rs` に重複済みで不要
  - 修正: `src/lib.rs` を削除、`cargo build --workspace` が通ることを確認
- [x] **[🔴] `InferenceRequest` をバッチ推論対応に再設計**
  - `prompt: String`（事前ビルド済み）を保持する現設計はバッチ推論と非互換
  - `build_batch_conversation_prompt` は `(&Citizen, &Citizen)` ペアが必要だが文字列から復元不可
  - 修正: `prompt: String` → `initiator: Citizen, partner: Option<Citizen>` に変更し、テストも更新
- [x] **[🟡] プロンプトビルダーの重複排除**
  - `build_conversation_prompt` と `build_batch_conversation_prompt` の市民プロファイルブロックが70%重複
  - 修正: `format_citizen_block(citizen, partner_name)` ヘルパーを抽出
- [x] **[🟠] `parse_batch_response` のpartial parse化**
  - 現在: 1件でも壊れると全件失敗
  - 修正: 壊れたエントリはデフォルト値+警告ログでスキップ、`CitizenResponse` に `Default` derive追加
- [x] **[🟠] `apply_compression` の戻り値を `bool` に変更**
  - 現在: エントリ不足時にサイレントno-op（呼び出し側が検出できない）

---

## Phase 0: 技術検証 (1-2週間)

- [x] Godot 4.4+ / Rust (gdext) の Hello World
  - `delphai-gdext` クレート (cdylib) を `delphai-core` (lib) から分離。devcontainerでgodot-core OOM回避
- [x] LLMベンチマーク基盤 (`cargo bench --bench llm_throughput`) のセットアップ
- [x] Ollamaモデル比較 第1ラウンド（旧モデル）
  - phi3:mini 脱落（遅い・日本語壊滅）
  - qwen3:1.7b — think:false で 5/5適合、平均1034ms
  - gemma2:2b — action:null問題、プロンプトでは解決不可
  - 旧候補は全て世代遅れと判明
- [x] Ollamaモデル比較 第2ラウンド（最新モデル）
  - ~~Qwen3.5 2B/0.8B~~ — 詩的だが支離滅裂、Gemma に集中するため除外
  - [x] Gemma 4 E2B — 内容×速度バランス最良 → **ルーティン会話の主力**
  - [x] Gemma 4 E4B — 品質高め → **重要シーン候補**（デュアルモデル構成）
  - [x] Ollama structured output (JSON Schema) で string型強制
  - [x] **最終モデル選定**: Gemma4 E2B（primary） / E4B（重要シーン候補）
    - 切替は `LlmProvider::with_parser()` + コンフィグで対応予定（Phase 1）
- [x] **重要シーン評価** (`--scenario crisis`: 敵接近 × 3性格 × 2モデル)
  - [x] eval_models.py に `--scenario crisis` 実装済み
  - [ ] 計測実行（Ollama起動必要）→ E2B vs E4B の性格分岐を目視確認
    - E2B は重要シーンで論理矛盾が出ることがある（実観測）→ デュアルモデル採用の根拠
- [x] **JSON/YAML 形式比較**（トークン削減調査）
  - [x] eval_models.py に `--format yaml` モード追加 + pyyaml でパース
  - [x] Rust `YamlResponseParser` 実装（TDD）: `serde_yaml = "0.9"` を追加
  - [x] **YAMLが大幅に高速と判明** → Rust デフォルトを YAML に切替済み（`player2.rs` + `prompt.rs`）
  - [ ] 定量計測: eval_count を JSON と比較（Ollama起動必要・参考値として）
- [ ] Player2 API を GDScript の HTTPRequest で叩く → 応答速度の計測
  - Rust側: `Player2Provider` (HTTP/JSON) 実装済み・テスト済み。GDScript側は本機Godotで実施
- [ ] NobodyWho (Godot用Rust LLMプラグイン) の検証
- [ ] バッチ推論テスト: 1プロンプトに住民ペア2-3組を詰めて同時生成できるか確認
- [ ] **Go/No-Go:** 応答が3秒以内で品質がゲームに使えるか

## Phase 1: 焚き火デモ (3-4週間)

> 住民3人が焚き火を囲んで会話する。プレイヤーが声を届けると反応する。
> これが面白くなければプロジェクトを止める。

### エージェント (Rust)

- [x] `Citizen` 構造体: name, personality_tags, memory_summary, emotion, relationships, divine_awareness
- [x] ルールベース行動ループ（移動/食事/睡眠のステートマシン）
  - `tick(state, needs) -> BehaviorAction` 純粋関数。fatigue≥0.8→Sleep, hunger≥0.7→Eat
- [x] 会話トリガー判定（近接 + ランダム確率）
  - `check_conversations()` ランダム外部注入、Idle同士のみ、1人1会話/tick

### LLM統合 (Rust)

- [ ] `LlmProvider` トレイト → Player2実装 / llama.cpp実装 (NobodyWho検証結果に応じて切替)
  - Player2実装済み。llama.cpp実装はPhase 0検証待ち
- [x] 優先度付き推論キュー（プレイヤー注目 > イベント > 定期）
- [x] 1ターン最大N回の推論制限
- [x] プロンプトビルダー（世界設定 + 個性 + 記憶 + 状況 → プロンプト文字列）
- [x] バッチプロンプトビルダー（複数住民ペアを1プロンプトに詰め込む）
- [x] レスポンスパーサー + フォールバック（配列レスポンス対応）
- [x] レスポンスパーサー疎結合化（ResponseParserトレイト導入、JSON/YAML切替可能に）
- [x] 記憶圧縮（64エントリ超で最古48件をLLM要約→1エントリに）
- [ ] LLMベンチマーク: batch_size=1,3,5 で効率スコア計測 → 最適バッチサイズ決定

### UI (Godot)

- [ ] 2Dマップ + 住民スプライト（仮アセット）
- [ ] プロンプト入力フィールド + 「声を届ける」ボタン
- [ ] 会話バブル or ログウィンドウ
- [ ] 住民クリックで詳細パネル

### シナリオ

- [ ] 住民3人: 好奇心旺盛な若者 / 慎重な長老 / 勇敢な狩人
- [ ] 会話サイクル: ペア選出 → LLM生成 → 感情・関係性反映
- [ ] 神の声: 認知度に応じた反応変化
- [ ] **判定: 10分遊んで面白いか**

## Phase 2: 文明進行 (4-6週間)

### 技術ツリー

- [ ] `tree_protocol.toml` パーサー実装
- [ ] 最小ツリー15-20ノード（石器時代→農業時代→青銅器時代）
- [ ] 技術進歩トリガー: 会話にキーワード含まれる → 研究ポイント加算 → 閾値到達で解放

### 世界

- [ ] 住民の増加（5→10→20人）
- [ ] 資源（食料/素材）の基本パラメータ
- [ ] ティア制更新: 注目住民=毎ターン / 重要人物=3ターンごと / その他=ルールベース
- [ ] **HDL検討**: 住民数増加時のバッチ処理向けに高密度シリアライゼーション（HDL）を調査
  - YAMLより更にトークン削減できる独自フォーマット + Rustパーサーの可能性
  - 住民20人以上・バッチサイズ5以上でボトルネックが出た時点で着手

### 戦争

- [ ] 部族分裂（人口増加→意見対立→分裂）
- [ ] 戦争トリガー（資源争い/侮辱/復讐、感情蓄積が閾値超え）
- [ ] 戦争結果: 人口減少、技術喪失、文明後退
- [ ] プレイヤーの介入は認知度に依存。完全には止められない

### 認知度

- [ ] 声を届けるたびに微量上昇。声の内容が現実に起こると急上昇。放置で低下
- [ ] 0%: プロンプトに含めない / 1-30%: ノイズ / 31-60%: 断片 / 61-90%: お告げ / 91-100%: ほぼそのまま届く

## Phase 3: ポリッシュ (3-4週間)

- [ ] LLMセットアップウィザード（Player2 → ローカル → クラウドの順に案内）
- [ ] セーブ/ロード
- [ ] 技術ツリー表示UI
- [ ] 時代に応じた背景変化
- [ ] 最低スペックテスト（GTX 1060 / CPU only / Apple Silicon M1）
- [ ] 外部プレイテスト5人以上

## Phase 4: リリース (2-3週間)

- [ ] Steamストアページ + itch.io
- [ ] トレーラー
- [ ] ストリーマーへのキー配布

---

## やらないこと (MVPスコープ外)

3D、マルチプレイヤー、TTS、青銅器時代より先、DLC、コンソール、住民100人以上。
