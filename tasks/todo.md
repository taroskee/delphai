# MVP TODO

MVPの定義: 住民5-10人が協力して狩猟・採集・採水し、3段階の技術進化を遂げる3Dシミュレーションゲーム。  
（LLMは Phase 2 で再活性化。参考ビジョン: 0 A.D. スタイルの没入感ある俯瞰3D）

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

## Phase 1: 3D文明シミュレーション（全面改訂）

> **方針転換 (2026-04-16)**: LLMを一旦外し、needs-driven行動AIで住民が自律的に生活するシミュレーションを先に作る。
> キャラが生き生きしてから、Phase 2 でLLMを再統合する。
> ビジュアル参考: 0 A.D. スタイルの俯瞰3D（Godot 4 + CharacterBody3D）

### 流用できるRust基盤 ✅

- [x] `Citizen` 構造体 (name, personality_tags, memory_summary, emotion, relationships, divine_awareness) — LLM再統合時に使用
- [x] `LlmProvider` トレイト + Ollama実装、YAML非同期キュー — `crates/delphai-core/src/llm/` ごと保存
- [x] プロンプトビルダー / レスポンスパーサー / 記憶圧縮 — Phase 2 で再活性化
- [x] `pathfinding.rs` — `WalkGrid` / `MoveState` / `step_toward()` — タイル座標はそのまま3Dにも使える
- [x] RCTパスファインダー（履歴リングバッファ、stuck脱出）✅

### 削除対象（2D専用・LLM依存GDExt API）

- [x] `game/scenes/main.gd` — 2D TileMap前提、3Dで作り直す
- [x] `game/scenes/citizen_sprite.gd` — AnimatedSprite2D前提、CharacterBody3Dで作り直す
- [x] `game/scenes/map.gd` — TileMapLayer前提、3D地形で作り直す

---

### Sprint 1: Rustコア改修（needs-driven行動）

> 判定: `cargo test` 全通過 + `.so` ビルド成功

- [x] `crates/delphai-core/src/resource.rs` 新規
  - `ResourceKind`: `BerryBush` / `WaterSource`
  - `Resource { kind, pos: TilePos, quantity: f32, respawn_timer: u32 }`
  - `Resource::berry_bush(pos)` / `Resource::water_source(pos)`
  - `Resource::is_available() -> bool`（quantity > 0）
- [x] `crates/delphai-core/src/lib.rs` に `pub mod resource;` 追加
- [x] `crates/delphai-core/src/agent/behavior.rs` 全面改修
  - `Needs { fed: f32, hydration: f32 }` — bigger-is-better（1.0=満腹/満水、0.0=枯渇）
  - `BehaviorState { Idle, SeekingFood, Gathering, SeekingWater, Drinking }`
  - 定数: `FED_SEEK_THRESHOLD=0.3`, `FED_SATED=0.9`, `HYDRATION_SEEK_THRESHOLD=0.3`, `HYDRATION_SATED=0.9`
  - 優先順位: hydration < fed → 水を先に求める
- [x] `crates/delphai-core/src/world.rs` 大幅改修
  - `CitizenVitals { fed: f32, hydration: f32 }` に改名（旧 `CitizenNeeds { hunger, fatigue }` 廃止）
  - `World` から `InferenceQueue` 除去（フィールド削除、import削除）
  - `World::tick()` 戻り値を `()` に変更（旧 `Vec<PendingConversation>`）
  - `resources: Vec<Resource>` フィールド追加
  - `World::add_resource(r: Resource)` 追加
  - `World::nearest_resource_pos(from: TilePos, kind: ResourceKind) -> Option<TilePos>` 追加
  - needs decay: `FED_DECAY_PER_TICK=0.004`, `HYDRATION_DECAY_PER_TICK=0.007`
  - gather/drink rate: `GATHER_RATE=0.05`, `DRINK_RATE=0.08`
  - `apply_response()` / `append_memory()` は残す（Phase 2 LLM再統合用）
- [x] `crates/delphai-core/src/agent/conversation.rs` テスト修正
  - `BehaviorState::Sleeping` → `BehaviorState::Idle` に置換
  - `BehaviorState::Eating` → `BehaviorState::Gathering` に置換
- [x] `crates/delphai-gdext/src/lib.rs` API整理
  - 削除: `set_divine_voice`, `clear_divine_voice` 等の不要な LLM直結API
  - 追加: `get_resource_count() -> i64`, `get_resource_pos(idx: i64) -> Vector2i`, `get_resource_kind(idx: i64) -> GString`
  - 追加: `get_citizen_fed(idx: i64) -> f64`, `get_citizen_hydration(idx: i64) -> f64`, `get_citizen_behavior(idx: i64) -> GString`
  - LLM フック（`apply_citizen_response`, `record_heard_speech`, etc.）は Phase 2 用に温存
- [x] `cargo test` 全通過確認（149テスト、コミット `f9891b3`）
- [x] `make build` (.so) 成功確認

### Sprint 2: Godot 3D シーン構築

> 判定: Godotで起動し、住民が3D空間を歩き回る

- [x] `game/scenes/world.tscn` 新規 — メインシーン（Node3D ルート）
- [x] `game/scenes/world.gd` 新規 — `WorldNode` 呼び出し、tick駆動
- [x] 市民: `CharacterBody3D` + `CapsuleMesh` + `Label3D`（名前・行動状態）をworld.gd内で生成
- [x] 行動状態に応じてカプセル色変化（idle=茶, seeking_food=橙, seeking_water=青）
- [x] カメラ設定: `Camera3D` を俯瞰固定（55°、高さ18、マップ中心）
- [x] 地形: `StaticBody3D` + `PlaneMesh` + `StandardMaterial3D`（草色）
- [x] タイル座標 → 3D世界座標変換 `tile_to_world(col, row) -> Vector3`
- [x] tick毎に `get_citizen_tile_pos()` → 位置同期
- [x] 資源: `SphereMesh`（BerryBush 緑）/ `CylinderMesh`（WaterSource 青）
- [x] `WorldEnvironment` + `ProceduralSkyMaterial` + `DirectionalLight3D`（影付き）
- [x] 2D ファイル削除: `main.gd`, `main.tscn`, `citizen_sprite.gd`, `map.gd`

### Sprint 3: 資源配置 + 採取ビジュアル

> 判定: 住民が berry bush に近づき、食べると量が減る

- [x] `game/scenes/resource_node.tscn` — `StaticBody3D` + `MeshInstance3D`（色で種別区別）
- [x] `world.gd` 起動時に `get_resource_count()` → 資源ノード生成
- [x] tick毎に `get_citizen_behavior()` で状態取得 → Gathering中はアニメ（仮: scale pulse）
- [x] 資源量に応じてMeshスケール変化（berry bushが減ると小さくなる）
- [x] `get_citizen_fed()` / `get_citizen_hydration()` → デバッグUI（ProgressBar × n人）

### Sprint 4: カメラ操作 + 見栄え

> 判定: 眺めているだけで5分飽きない

- [ ] カメラ操作: マウスドラッグでパン、スクロールでズーム（0 A.D. 風）
- [ ] 住民モデル仮置き改善: カプセル → 簡易人型（Godot primitives組み合わせ）
- [ ] 昼夜サイクル: `DirectionalLight3D` の角度をtick連動で変化
- [ ] 環境光: `WorldEnvironment` + Sky設定（夕暮れ感）
- [ ] `sfx/Ambience/CampAmbience.mp3` BGM再生

### Sprint 5: 技術ツリー第1段階（石器時代）

> 判定: 採集回数が閾値を超えると「道具製作」が解禁される

- [ ] `crates/delphai-core/src/tech.rs` 新規
  - `TechNode { id, name, required_points: u32, unlocked: bool }`
  - `TechTree { nodes: Vec<TechNode>, research_points: u32 }`
  - `TechTree::try_unlock() -> Option<TechNodeId>`
- [ ] 採集イベントで `research_points += 1`
- [ ] 最初の技術: 「石器」解禁 → `GATHER_RATE *= 1.5`
- [ ] UI: 右上に研究ポイントと次の技術までの進捗バー

---

### 判定チェックポイント

- [x] **Sprint 1完了後**: `cargo test` 全通過、needs-driven行動がテストで確認できる
- [x] **Sprint 2完了後**: 3D空間で住民が歩き回る — 眺めて飽きないか？
- [ ] **Sprint 3完了後**: 住民が資源を求めて行動する — 目的を持って動いているか？
- [ ] **Sprint 5完了後**: 技術が進む — プレイヤーが成長を感じられるか？
- [ ] **最終判定**: 眺めているだけで5分飽きない / 文明が育っていく感覚がある

---

## アセット在庫（必要になった時に参照）

> `game/assets/` に含まれる未使用アセット

### SFX（即使用可）
| アセット | 用途 |
|---|---|
| `sfx/Ambience/CampAmbience.mp3` | Sprint 4 BGM |
| `sfx/Food & Herbs/` | 採集時 SE |

### 3Dモデル（未所持 → Phase 2で検討）
- 住民: 現状はGodot primitives（カプセル）で代替
- 動物: Sprint 3以降で追加検討（`animals1〜5` スプライトは2D専用）

### 2D資産（封印）
キャラクタースプライト・タイルセット一式は 2D 版に戻す場合のみ参照。3D移行後は使わない。

---

## Phase 2: LLM再統合 + 文明進行 (4-6週間)

> Phase 1 完了後に開始。`crates/delphai-core/src/llm/` はそのまま使える。

### LLM再活性化

- [ ] `World::tick()` に LLM キューを再接続
- [ ] 住民の needs状態をプロンプトに追加（「空腹で川へ向かっている」など文脈付与）
- [ ] `divine_awareness` システム復活

### 🔬 Bonsai 8B vs Gemma4 比較検証

> **約束**: Phase 1 完了後に必ず実施。

**採用判定基準**
- 採用: レイテンシ ≤ 1000ms かつ YAML 適合率 ≥ 90% かつ 品質スコア ≥ 2.0
- 不採用: 上記未達 → Gemma4 E2B 継続

### 技術ツリー拡張

- [ ] 農業時代（石器時代 → 農業時代）
- [ ] 青銅器時代
- [ ] 技術進歩トリガー: 会話キーワード → 研究ポイント加算

### 世界拡張

- [ ] 住民の増加（5→10→20人）
- [ ] 部族分裂・戦争トリガー
- [ ] **HDL検討**: 住民20人以上でYAMLがボトルネックになった時点で独自フォーマット調査

---

## Phase 3: ポリッシュ (3-4週間)

- [ ] セーブ/ロード
- [ ] 技術ツリー表示UI
- [ ] 最低スペックテスト（GTX 1060 / CPU only / Apple Silicon M1）
- [ ] 外部プレイテスト5人以上

## Phase 4: リリース (2-3週間)

- [ ] Steamストアページ + itch.io
- [ ] トレーラー
- [ ] ストリーマーへのキー配布

---

## やらないこと (MVPスコープ外)

マルチプレイヤー、TTS、青銅器時代より先、DLC、コンソール、住民100人以上。
