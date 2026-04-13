# MVP TODO

MVPの定義: 住民5-10人がLLMで会話し、プレイヤーの「声」に反応し、技術ツリーが3段階進行するデモ。

---

## 既知の問題・インフラ

- [x] **macOS dylib ビルド**: Godot は macOS 上で動くが、devcontainer 内では `.so`（Linux）しかビルドできない。
  - 解決: `prebuilt/macos/` に CI (GitHub Actions) がビルドしてコミット。
  - ローカル: Mac ターミナル（devcontainer 外）で `make build-mac` を一度だけ実行。
  - 参照: `.github/workflows/build-libs.yml`, `Makefile`, `game/delphai.gdextension`

---

## Phase 0: 技術検証 ✅

- [x] Godot 4.4+ / Rust (gdext) Hello World — `delphai-gdext`(cdylib) / `delphai-core`(lib) 分離済み
- [x] LLMベンチマーク基盤 (`cargo bench --bench llm_throughput`)
- [x] モデル選定
  - ~~Qwen3.5 2B/0.8B~~ — 詩的だが支離滅裂、Gemmaに集中するため除外
  - **Gemma4 E2B** — primary（ルーティン会話）avg 1248ms YAML
  - **Gemma4 E4B** — 重要シーン候補。avg 3715-8133ms → Phase 2 住民増加時に限定運用
- [x] JSON/YAML比較 — YAML: 30%速く(1248ms vs 1792ms)、32%少ないトークン(70 vs 103) → **YAML採用**
- [x] Crisis評価 — E2B: avg 2.3/3適合(YAML切替で安定)。E4B: 品質高いが遅すぎ
- [x] NobodyWho — **スキップ**: Ollama 1.2s安定動作中、埋め込みの複雑度不要
- [x] Player2 GDScript計測 — **Phase 3 延期**: APIキー未取得
- [x] バッチ推論実計測 — **Phase 2 延期**: 住民3人では不要。スタブでカーブのみ確認済み
- [x] **Go/No-Go: GO ✅** — E2B YAML 1248ms < 3秒目標、焚き火デモ実動作確認済み

---

## Phase 1: ワールドデモ（全面改訂）

> **判定基準**: 眺めているだけで5分飽きないか。声を届けたとき「伝わった感」があるか。
> ※旧「焚き火デモ」のRust基盤はそのまま流用。UI・AIをゲームとして再設計。

### 完了済み基盤 ✅

- [x] `Citizen` 構造体 (name, personality_tags, memory_summary, emotion, relationships, divine_awareness)
- [x] `LlmProvider` トレイト + Ollama実装、YAML非同期キュー
- [x] プロンプトビルダー / レスポンスパーサー / 記憶圧縮
- [x] `World::tick()` / `apply_response()` / `record_heard_speech()`
- [x] 2D TileMapLayer（terrain.png + water.png）— 24×14タイル、3倍スケール
- [x] 住民スプライト・焚き火アニメ・会話バブル・ログウィンドウ
- [x] プレイヤーの声入力UI + divine_awareness 成長システム

---

### Sprint A: ワールド基盤 + パスファインダー（RCT方式）✅

- [x] `crates/delphai-core/src/pathfinding.rs` 新規
  - `WalkGrid { width, height, cells: Vec<bool> }`
  - `WalkGrid::neighbors(pos) -> [Option<TilePos>; 4]`
  - `WalkGrid::step_toward(from, to, history) -> TilePos`（ローカル1ステップ決定）
  - 履歴リングバッファ（直近16タイル）によるループ防止
  - stuck_counter ≥ 3 で緊急脱出（ランダム隣接タイル）
- [x] `crates/delphai-core/src/pathfinding.rs` に `MoveState` 追加
  - `tile_pos`, `move_target`, `move_history[16]`, `move_cooldown`, `stuck_counter`, `wander_center`, `wander_radius`, `facing`
- [x] `World::tick()` に移動処理追加（move_cooldown制御、wander目標選択、take/replace borrow pattern）
- [x] `WorldNode::set_walkable_map(data: PackedByteArray)` GDExt公開
- [x] `WorldNode::get_citizen_tile_pos(idx) -> Vector2i` GDExt公開
- [x] `WorldNode::get_citizen_facing(idx) -> int` GDExt公開
- [x] `map.gd` に `get_walkable_data() -> PackedByteArray` 追加
- [x] `main.gd` 起動時に `set_walkable_map()` 呼び出し
- [x] `citizen_sprite.gd` に `walk_to(screen_pos)` / `set_facing(dir)` 追加（Tween補間）
- [x] `main.gd` でtick毎に tile_pos 変化を検知 → `sprite.walk_to()` 呼び出し
- [x] `conversation.rs` を `TilePos` + マンハッタン距離に完全移行（float Position 廃止）
- [x] 全テスト 143件通過、`.so` リビルド完了

### Sprint B: 視界 + 近接会話

- [ ] `Citizen` に `sight_radius: u32 = 5` 追加
- [ ] `World::get_visible_from(idx) -> Vec<usize>` 実装
- [ ] 会話トリガーをタイマー駆動 → **移動イベント駆動** に変更
  - 条件: 互いの視界内 かつ 距離≤3 かつ 両者Idle かつ クールダウン切れ
  - `World::tick()` が `WorldEvent::ConversationReady { i, j }` を返す
- [ ] `crates/delphai-core/src/event.rs` 新規（`WorldEvent` enum）
- [ ] 会話中: `move_target = None`（その場停止）、互いの方向に `facing` セット
- [ ] 会話終了後: `wander_center` からランダム目的地を再選択
- [ ] `F1` キーで視界サークルオーバーレイ表示/非表示（デバッグ用、デフォルトOFF）

### Sprint C: 動物エコシステム

- [ ] `crates/delphai-core/src/animal.rs` 新規
  - `Animal { id, animal_type: AnimalType, move_state: MoveState }`
  - `AnimalType`: Deer（wander_radius=8, cooldown=3）/ Rabbit（radius=5, cooldown=1）
  - 逃げ条件: 住民が2タイル以内 → 反対方向へ move_target セット
- [ ] GDExt公開: `get_animal_count()`, `get_animal_tile_pos(id)`, `get_animal_type(id)`
- [ ] `game/scenes/animal_sprite.gd` 新規（`animal/individual_frames/` のフレーム使用）
- [ ] `main.gd` にアニマルスプライト管理追加

### Sprint D: プレイヤー介入の手ごたえ

- [ ] LLM YAML スキーマ拡張: `action_taken: bool`, `action_direction: str`
  - `action_taken: true` → Rust で `move_target` を方向にセット
  - `action_taken: false` → ログ記録のみ（アニメなし）
  - `divine_awareness` 高いほど `action_taken: true` を出しやすいプロンプト設計
- [ ] 声が届いた住民の頭上に `!` スプライト（1秒表示）

---

### 判定チェックポイント

- [ ] **Sprint A完了後**: キャラが歩き回る — 眺めて飽きないか？
- [ ] **Sprint B完了後**: 近くで会話する — 自然に見えるか？
- [ ] **Sprint D完了後**: 声を届けたらキャラが動く — 「伝わった感」があるか？
- [ ] **最終判定**: 眺めているだけで5分飽きない / 声を届けたとき手ごたえがある

---

## アセット在庫（Phase 2以降で使う）

> `game/assets/` に含まれる未使用アセット。必要になった時に参照。

### キャラクター
| カテゴリ | バリアント数 | 感情フレーム | 用途候補 |
|---|---|---|---|
| `chara` (chara2〜5 × 8) | 32 | あり (shake/nod/surprise/laugh) | 主要住民・会話キャラ |
| `npc` (npc1〜4 × 8) | 32 | なし (walk/stand のみ) | 背景モブ、Phase 2 増加住民 |
| `military` (military1〜3 × 8) | 24 | なし | 戦士・紛争キャラ、Phase 2 戦争 |
| `bonus1` (× 8) | 8 | 要確認 | 特殊キャラ枠 |
| `animals` (cat1〜4, dog1〜4) | 8 | なし | ペット・環境演出 |

### エフェクト
| アセット | フレーム | 用途候補 |
|---|---|---|
| `effect/fires/loop/fireV001〜006` | 各10f | 焚き火バリエーション、松明、家の炉 |
| `effect/water/waterV001〜003` | 各数f | 水源発見エフェクト（デモシナリオ直結） |

### SFX
| アセット | 用途候補 |
|---|---|
| `sfx/Ambience/CampAmbience.mp3` | **即使用可** — 焚き火BGM |
| `sfx/Food & Herbs/` | 食料取得・fed パラメータ回復音 |
| `sfx/Coins/` | 資源取引、Phase 2 経済 |
| `sfx/Weapons/` | 戦争・戦闘、Phase 2 |

### タイルセット（Phase 2 マップ構築用）
`outside.png` — 野外フィールド、`terrain.png` — 地形、`world.png` — ワールドマップ、`castle.png` — 拠点

### 動物スプライト（シートのみ）
`animal/individual_frames/animals1〜5, birds1〜2, horse` — 鹿群れシナリオに `animals` を使用候補（要フレーム分割）

---

## Phase 2: 文明進行 (4-6週間)

### 🔬 Bonsai 8B vs Gemma4 比較検証（Phase 2 最初のスプリント）

> **約束**: Phase 1 完了後に必ず実施。Gemma4 E2B と Bonsai 8B を同一条件で比較する。

**背景**: Bonsai 8B は NLP 特化・軽量だが独自量子化のため Ollama 非対応。住民増加時のボトルネック解消候補。

**Research（実装前）**
- [ ] Bonsai 8B の推論要件確認（GGUF 変換可否 / HuggingFace Hub の有無 / 公式ランタイム）
- [ ] ランタイム選定: llama-server → vLLM → HF Inference API の優先順で検討
- [ ] 日本語性能・YAML 出力安定性をモデルカードで事前確認

**実装**
- [ ] `crates/delphai-core/src/providers/bonsai.rs` 新規（`LlmProvider` トレイト実装）
  - llama-server 経由なら `OllamaProvider` のほぼコピー（URL + モデル名差し替えのみ）
  - OpenAI互換なら `/v1/completions` エンドポイント
- [ ] `WorldNode::set_llm_provider(name: String)` GDExt 公開（実行時に切り替え可能）
- [ ] `Makefile` に `run-bonsai-server` ターゲット追加

**ベンチマーク（`crates/delphai-bench/` 拡張）**
- [ ] Gemma4 E2B vs Bonsai 8B を同一プロンプトセットで `criterion` 計測
- [ ] 計測指標: レイテンシ avg/p95、YAML 適合率（`speech`/`emotion_change`/`action_taken` parse 成功率）、日本語品質スコア（5シナリオ × 1-3点手動）

**採用判定基準**
- 採用: レイテンシ ≤ 1000ms かつ YAML 適合率 ≥ 90% かつ 品質スコア ≥ 2.0
- 部分採用: ルーティン会話のみ Bonsai、重要シーンは Gemma E4B
- 不採用: 上記未達 → Gemma4 E2B 継続

---

### 技術ツリー

- [ ] `tree_protocol.toml` パーサー実装
- [ ] 最小ツリー15-20ノード（石器時代→農業時代→青銅器時代）
- [ ] 技術進歩トリガー: 会話キーワード → 研究ポイント加算 → 閾値で解放

### 世界

- [ ] 住民の増加（5→10→20人）
- [ ] 資源（食料/素材）の基本パラメータ
- [ ] ティア制更新: 注目住民=毎ターン / 重要人物=3ターンごと / その他=ルールベース
- [ ] **HDL検討**: 住民20人以上でYAMLがボトルネックになった時点で独自フォーマット調査

### 戦争

- [ ] 部族分裂 → 戦争トリガー（資源争い/侮辱/復讐）→ 人口減少・技術喪失
- [ ] プレイヤーの介入は認知度に依存。完全には止められない

### 認知度

- [ ] 声を届けるたびに微量上昇。声の内容が現実に起こると急上昇。放置で低下
- [ ] 0%: プロンプト除外 / 1-30%: ノイズ / 31-60%: 断片 / 61-90%: お告げ / 91-100%: ほぼそのまま

---

## Phase 3: ポリッシュ (3-4週間)

- [ ] Player2 API 計測 + LLMセットアップウィザード（Player2 → ローカル → クラウド）
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
