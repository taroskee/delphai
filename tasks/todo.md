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

- [x] カメラ操作: マウスドラッグでパン、スクロールでズーム（0 A.D. 風）
- [x] 住民モデル仮置き改善: カプセル → 簡易人型（Godot primitives組み合わせ）
- [x] 昼夜サイクル: `DirectionalLight3D` の角度をtick連動で変化
- [x] 環境光: `WorldEnvironment` + Sky設定（夕暮れ感）
- [x] `sfx/Ambience/CampAmbience.mp3` BGM再生

### Sprint 5: 技術ツリー第1段階（石器時代）

> 判定: 採集回数が閾値を超えると「道具製作」が解禁される

- [x] `crates/delphai-core/src/tech.rs` 新規
  - `TechNode { id, name, required_points: u32, unlocked: bool }`
  - `TechTree { nodes: Vec<TechNode>, research_points: u32 }`
  - `TechTree::try_unlock() -> Option<TechNodeId>`
- [x] 採集イベントで `research_points += 1`
- [x] 最初の技術: 「石器」解禁 → `GATHER_RATE *= 1.5`、農業 → ベリー再生3倍速、青銅器 → 採集2倍
- [x] UI: 右上に研究ポイントと次の技術までの進捗バー

### Sprint 6: 文明の成長（人口増加）

> 判定: 住民が prosperity を維持すると子供が生まれ、文明が育っていく感覚がある

- [x] `world.rs`: 全住民 fed > 0.8 && hydration > 0.8 が200tick継続 → 新市民誕生（最大8人）
- [x] `world.rs`: `pending_births: u32` フィールド、`maybe_spawn_citizen()` / `birth_citizen()`
- [x] `world.rs`: `effective_gather_rate()` — bronze_tools=2x, stone_tools=1.5x, なし=1x
- [x] `world.rs`: `effective_respawn_ticks()` — agriculture解禁でベリー再生1/3に短縮
- [x] `lib.rs`: `pop_citizen_birth() -> bool` GDExt API
- [x] `lib.rs`: ベリーブッシュ3本に増設（人口増加に備えた資源充実）
- [x] `world.gd`: tick毎に `pop_citizen_birth()` → 市民ノードを動的追加
- [x] `world.gd`: 技術解禁・市民誕生時に通知バナー表示（3.5秒フェードアウト）
- [x] 156テスト全通過、`.so` ビルド成功

### Sprint 7: 生命感 + 動物 (Life & Presence)

> 判定: 住民が会話し、重ならず、動物と協力して狩り、植物が枯れたら非表示になる

- [x] **[Rust] tech tree を Phase 1 から除去**
  - `world.rs`: `effective_gather_rate()` / `effective_respawn_ticks()` 削除、定数を直接使用
  - `world.rs`: `self.tech_tree.add_points(1)` 削除（`tech_tree` フィールド自体は Phase 2 用に残す）
  - `world.gd`: `_build_tech_ui()` / `_update_tech_ui()` 呼び出し削除、Tech UI 変数削除
- [x] **[GDScript] 資源メッシュ非表示バグ修正**
  - `_update_resources()`: `qty <= 0.0` → `mesh.visible = false`、`qty > 0.0` → `mesh.visible = true` + スケール適用
- [x] **[GDScript] 市民の重なり解消**
  - `_sync_citizen_pos()`: 黄金角オフセット（`idx * 2.399963 rad`）で市民を 0.3 タイル半径の円状に配置
- [x] **[GDScript] 市民の会話バブル**
  - `_make_citizen()`: `chat_lbl: Label3D` を y=1.85 に追加
  - `_chat_timers: Array` でタイマー管理（ランダム 3–8 秒）
  - `_process()`: タイムアウトで behavior に対応した台詞をランダム表示（2.5 秒後消える）
  - 台詞: idle / seeking_food / gathering / seeking_water / drinking / hunting 各 4 行（日本語）
- [x] **[Rust+GDScript] 動物 + 協力狩猟**
  - `animal.rs` 新規: `Animal { pos, origin, alive, respawn_timer }` + `wander(seed)` + `kill()`
  - `behavior.rs`: `BehaviorState::Hunting` 追加 + `tick()` に Hunting アーム（FED_SATED で Idle へ）
  - `world.rs`: `animals: Vec<Animal>` 追加、`nearest_animal_pos()` 追加
    - Idle で fed < 0.5 かつ alive な動物あり → Hunting へ
    - 2 人以上が同タイルに到達した動物を kill → 各ハンターに fed += 0.5
    - 動物は 4 tick に 1 回 wander、kill 後 300 tick で同地点リスポーン
  - `lib.rs (core)`: `pub mod animal` 追加
  - `lib.rs (gdext)`: `get_animal_count()`, `get_animal_pos()`, `get_animal_alive()` + 鹿 2 頭を initialize に追加
  - `world.gd`: 動物ノード（茶色 BoxMesh）+ `_build_animals()` + `_update_animals()` + hunting 行動色

### Sprint 8: 鹿の逃走 + フェードアウト (Deer Flee)

> 判定: 人間が近づくと鹿が逃げ、マップ端まで逃げたらフェードアウトして消え、離れた場所へリスポーン

- [x] **[Rust] `animal.rs` 全面改修**
  - `flee_or_wander(nearest_human: Option<TilePos>, seed: u64)` — 旧 `wander(seed)` を置換
  - `FLEE_RADIUS=6` タイル以内に人間 → 距離の大きい軸で逃走
  - マップ外 → `fled=true`, `alive=false`, `escape_pos=Some(...)`, タイマー開始
  - `tick_respawn(seed: u64)` — `escape_pos` から `MIN_RESPAWN_DIST=8` 以上離れた位置へリスポーン（30回試行 + 対角コーナーfallback）
- [x] **[Rust] `world.rs` 更新**
  - 人間位置スナップショット（`Vec<TilePos>`）をループ前に取得してボロウ問題回避
  - 各動物の最近傍人間を算出 → `flee_or_wander` に渡す
  - `tick_respawn(seed)` に seed を渡すよう変更
- [x] **[GDExt] `lib.rs` 更新**
  - `pop_animal_fled(idx: i64) -> bool` 追加 — `fled` フラグを読み取りつつクリア（one-shot）
- [x] **[GDScript] `world.gd` 更新**
  - `pop_animal_fled(i)` が true → `TRANSPARENCY_ALPHA` + alpha 1→0 tween → 非表示
  - リスポーン後の初回フレーム → alpha 0→1 fade-in tween
- [x] 165テスト全通過、`.so` ビルド成功

### Sprint 9: 地形システム + スムーズ移動 + モデル改善 ✅

> 判定: 山/森/川が視覚的に区別でき、住民が滑らかに移動する

- [x] **[GDScript] 地形システム** — `_get_terrain(col, row) -> int` + 5 タイプ定義
  - 境界・NW コーナー = 山（通行不可）
  - 列 18 = 深い川（通行不可）、列 17/19 = 浅い川（採水地）
  - 森ゾーン A（左上）/ B（右中）: ハッシュで木の疎密を生成
- [x] **[GDScript] 地形フィーチャー描画** — 山（灰色コーン）/ 木（幹+葉球）/ 水面（半透明プレーン）
- [x] **[GDScript] `_send_walkable_map()` 更新** — T_DEEP/T_MOUNTAIN を 0 として Rust に送信
- [x] **[Rust] `lib.rs` 座標修正** — WaterSource を浅い川の (17,7) へ、鹿2頭目を深い川を避け (21,11) へ
- [x] **[GDScript] スムーズ移動** — `_citizen_target_pos` / `_animal_target_pos` 配列 + `_process()` lerp
- [x] **[GDScript] 住民モデル改善** — チェスポーン型 (ベース円盤 + ボディ柱 + 頭球, Node3D)
- [x] 165テスト全通過、`.so` ビルド成功

### Sprint 10: 3Dアセット適用 (GLB Asset Integration)

> 判定: 住民・動物・地形・資源が GLB モデルに差し替わり、世界が視覚的にリッチになる

- [ ] **[GDScript] 動物モデル差し替え** (`game/assets/animal/Animals_FREE.glb`)
  - `_make_deer()`: `SphereMesh` → GLB の鹿 Mesh に差し替え
  - アニメーション対応: `AnimationPlayer` で idle/run 切り替え
- [ ] **[GDScript] 地形モデル差し替え** (`game/assets/geography/terrian.glb`)
  - `PlaneMesh` の地面 → `terrian.glb` の `MeshInstance3D` に置換
- [ ] **[GDScript] 自然小物 (資源ノード)** (`game/assets/nature/simple_nature_pack_glb.glb`)
  - BerryBush: `SphereMesh` → GLB の茂みメッシュ
  - WaterSource: `CylinderMesh` → GLB の水源メッシュ
- [ ] **[GDScript] 住民モデル** — チェスポーン → GLB 人型（`game/assets/` に候補があれば）
- [ ] `game/assets/inventry.md` を更新（適用済みアセットにチェック）

---

### 判定チェックポイント

- [x] **Sprint 1完了後**: `cargo test` 全通過、needs-driven行動がテストで確認できる
- [x] **Sprint 2完了後**: 3D空間で住民が歩き回る — 眺めて飽きないか？
- [x] **Sprint 3完了後**: 住民が資源を求めて行動する — 目的を持って動いているか？
- [x] **Sprint 5完了後**: 技術が進む — プレイヤーが成長を感じられるか？
- [x] **Sprint 6完了後**: 文明が育つ — 住民が増え、技術が解禁され、5分以上眺められる
- [x] **最終判定 (Sprint 7)**: 住民が会話し、重ならず、動物と協力して狩り、植物が枯れたら非表示
- [x] **Sprint 8**: 鹿が人間から逃げ、マップ端でフェードアウト、遠い位置へリスポーン
- [x] **Sprint 9**: 地形システム（山/森/川）+ スムーズ移動 + チェスポーン住民モデル
- [ ] **Sprint 10**: GLBアセットを適用し、視覚的にリッチな世界になる

---

## アセット在庫

→ [`game/assets/inventry.md`](../game/assets/inventry.md) に移管（単一責務）

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
