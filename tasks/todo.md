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
  - [x] 計測実行完了 → E2B vs E4B の性格分岐を確認
    - E2B × ケン: 1/3適合(JSON末尾カンマ), avg 3468ms / E4B × ケン: 3/3, avg 8133ms(遅すぎ)
    - E2B × ミツ: 3/3, avg 2566ms / E4B × ミツ: 3/3, avg 4420ms
    - E2B × タカシ: 3/3, avg 2245ms / E4B × タカシ: 3/3, avg 3715ms
    - **結論**: 重要シーンも E2B で対応可。E4B は住民数が増えた時の重要NPCに限定
- [x] **JSON/YAML 形式比較**（トークン削減調査）
  - [x] eval_models.py に `--format yaml` モード追加 + pyyaml でパース
  - [x] Rust `YamlResponseParser` 実装（TDD）: `serde_yaml = "0.9"` を追加
  - [x] **YAMLが大幅に高速と判明** → Rust デフォルトを YAML に切替済み（`player2.rs` + `prompt.rs`）
  - [x] 定量計測完了: YAML avg 1248ms / 70tokens vs JSON avg 1792ms / 103tokens (E2B, campfire)
    - YAML が 30%速く 32%少ないトークン → YAML採用確定
- [ ] Player2 API を GDScript の HTTPRequest で叩く → 応答速度の計測
  - Rust側: `Player2Provider` (HTTP/JSON) 実装済み・テスト済み
  - **→ Phase 3 に延期**: API キー未取得・外部サービス依存。ローカルOllamaで Phase 1-2 進める
- [x] NobodyWho (Godot用Rust LLMプラグイン) の検証
  - **→ スキップ決定**: Ollama が十分高速(1.2s)で安定動作中。NobodyWho はGodotプロセス内埋め込みで複雑度が高い。配布時の問題が出たら再検討
- [x] バッチ推論テスト: 1プロンプトに住民ペア2-3組を詰めて同時生成できるか確認
  - `cargo bench --bench llm_throughput` のスタブで batch_size=1,3,5 の効率カーブは確認済み
  - **→ Phase 2 に延期**: Phase 1は住民3人のみ。バッチ本計測は住民10人以上になった時点で実施
- [x] **Go/No-Go: GO ✅**
  - E2B YAML: avg **1248ms** (目標3秒以内 ✓)
  - JSON parse品質: campfire 5/5、crisis 2.3/3平均 (E2B)。YAML切替で安定
  - 焚き火デモ実動作確認済み（3住民が会話し神の声に反応）
  - **Phase 1 着手条件 クリア**

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

- [x] `LlmProvider` トレイト → Player2実装 / llama.cpp実装 (NobodyWho検証結果に応じて切替)
  - Player2実装済み。llama.cpp実装はPhase 0検証待ち
- [x] 優先度付き推論キュー（プレイヤー注目 > イベント > 定期）
- [x] 1ターン最大N回の推論制限
- [x] プロンプトビルダー（世界設定 + 個性 + 記憶 + 状況 → プロンプト文字列）
- [x] バッチプロンプトビルダー（複数住民ペアを1プロンプトに詰め込む）
- [x] レスポンスパーサー + フォールバック（配列レスポンス対応）
- [x] レスポンスパーサー疎結合化（ResponseParserトレイト導入、JSON/YAML切替可能に）
- [x] 記憶圧縮（64エントリ超で最古48件をLLM要約→1エントリに）
- [x] `World` 構造体 + `tick()` + `apply_response()` 実装
  - `tick(random_roll)`: 行動状態更新 → 会話判定 → キュー積み込み → `PendingConversation` 返却
  - `apply_response()`: `CitizenResponse` → 感情/記憶に反映
- [x] LLMベンチマーク: batch_size=1,3,5 で効率スコア計測 → 最適バッチサイズ決定
  - ベンチ基盤実装済み（`cargo bench --bench llm_throughput`）。実LLM計測はPhase 0 Go/No-Go時に実施

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

### 焚き火デモ修正 (2026-04-09)

- [x] **神の声プロンプト改善**: 現在「おはよう」→「炎のゆらめきは...」という詩的雑談になってる
  - 原因: `[Divine Voice]: text` だけでは「聞こえた超自然的な声」ではなく「テーマ/インスピレーション」として解釈される
  - 修正: `[SUPERNATURAL EVENT]` ブロックで「今この瞬間 {name} が不思議な声を聞いた体験」として明示
- [x] **Elder→Kaelへの返答コンテキスト**: Kaelの発言がElderの記憶に入っていないため返答できない
  - 原因: `apply_response(kael)` はKaelの記憶のみ更新、Elderは何を返答すべきか知らない
  - 修正: `record_heard_speech(listener_idx, speaker_name, speech)` Rust `#[func]` を追加。返答キュー積み込み前にGDScriptから呼ぶ
- [x] **会話ログファイル保存**: `user://logs/conv_YYYYMMDD_HHMMSS.log` に全会話を書き出す

### 焚き火デモ修正 (2026-04-11)

- [x] **memory_summaryの肥大化防止**: `record_heard_speech` + `apply_response` が無制限にappend
  - 原因: ログで887文字→1884文字→3357文字と急増。プロンプトが肥大化してLLMの品質が下がる
  - 修正: `append_memory()` ヘルパーで最新8エントリに制限。`apply_response` + `record_heard_speech` 両方に適用
- [x] **Haraが会話に参加しない**: `check_conversations` が常にKael-Elderペアを選出
  - 原因: idle市民リストをそのまま順番に走査するため、最初の近接ペア(0-1)が毎回当選
  - 修正: `random_roll` でリストをローテーション → Haraも均等に選ばれるようになった
- [x] **会話トピックのループ**: KaelとElderが「川へ行こう」「危険だ注意しろ」を10回以上繰り返す
  - 原因: memory_summaryにループした発言が積み重なりLLMが同じ文脈を繰り返す。memory_summary肥大化と同根
  - 修正: memory_summary上限(8エントリ)で古い文脈が消えループが解消される見込み

### 焚き火デモ修正 (2026-04-10)

- [x] **会話の自然化・短縮**: 発言が長すぎる・詩的すぎる問題
  - 目標スタイル例:
    - Kael「今日は何してた？」
    - Elder「後継者に指導してた。将来が楽しみだ。お前は？」
    - Kael「鹿を狩ろうとしたが逃げられた。追ったら果物の森を見つけた」
    - Elder「それはよかった、塞翁が馬じゃな」
    - Kael「どういう意味ですか？」
  - 修正: プロンプトに「1-2文・日常語・詩的禁止・具体的な行動や出来事を話す」制約 + 良い例/悪い例を追加
- [x] **divine_awareness成長システム**: 神の声を聞くたびに住民のawarenessが上昇し、会話が変化する
  - awareness=0.0: 何かを感じたが内容不明（sensedプレースホルダー）
  - 神の声を届けるたびに全住民のawarenessが+0.07上昇
  - awareness=0.3→断片、0.6→お告げ、0.91→ほぼそのまま（既存ティアを利用）
  - 修正: `filter_divine_voice(0.0)` をNone→Some("(sensed...)") に変更、`grow_divine_awareness` / `get_divine_awareness` Rust func追加、GDScript側で声送信ごとに呼ぶ
- [x] **ログファイルパスをUIに表示**: 起動時に `user://` の実パスをコンソールと最初のログ行に出力

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
