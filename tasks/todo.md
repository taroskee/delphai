# MVP TODO

**MVP:** 住民5–10人が協力して狩猟・採集・採水し、3段階の技術進化を遂げる俯瞰3Dシミュ。
LLM は Phase 2 で再活性化（Phase 1 中は外してある）。視覚参考: 0 A.D. 風。

**Phase 1 基準**: 起動直後に 5分以上眺めていられる（没入感・視覚的リッチさ）。

**再現ドキュメント**: 旧実装の構造は `@docs/CODEMAPS/`（architecture / backend / frontend / data / dependencies）に C4 図付きで保存済。必要なら参照するが、**コピペ再実装は禁止**（継ぎ足しバグの再発防止）。

---

## 検証順（コミット前）

1. `cargo test -p delphai-core`（削除後はまず LLM 系のみ pass）
2. `cargo clippy --workspace`（警告ゼロを維持）
3. IDE 診断: `mcp__ide__getDiagnostics` で GDScript parser error なし確認
4. `make build`（`.so`、gdext cdylib）
5. Godot シーン起動確認（UI/3D を変えた場合）
6. 該当 Sprint の smoke test（`make smoke-*`）

詳細は @tasks/lessons.md の「初日に踏みやすい地雷」を参照。

---

## 現在のスプリント (Phase 1 再構築)

> **再構築の背景（2026-04-20）**: Sprint 13.R5 / R6 / Phase C.1 で 3 度目の「テクスチャ未反映」「マス目移動」修正に失敗。継ぎ足し修正の累積が限界。LLM 以外を全削除し、`docs/CODEMAPS/` を再現資料としてゼロから作り直す。
>
> **Phase 通過判定**: 起動直後に 5 分以上眺めていられる（没入感・視覚的リッチさ）。

### 再構築の原則（@tasks/lessons.md 準拠）

1. **TDD 厳守** — 各機能は `失敗するテスト → 最小実装 → GREEN` の順。計画を先に全 `[x]` にしない
2. **3 点揃うまで未完了** — `cargo test` / `make build` / **人間目視** の 3 つ全部揃うまで Sprint の `[x]` を打たない
3. **GDScript 挙動は Rust FFI に寄せる** — `get_citizen_world_pos(idx, alpha)` のようにランタイム挙動を Rust に引き込み cargo test でガード
4. **計画=実装の混同禁止** — 計画記述中は `[ ]`、Edit/Write 直後のターンでのみ `[x]`
5. **ハック修正禁止** — 2 連続で同じ症状が再発したら作業停止、人間に相談
6. **LLM サブシステムは保存** — `crates/delphai-core/src/llm/` と依存する `agent/citizen.rs` は触らない

### 保存範囲（削除外）

- `crates/delphai-core/src/llm/*`（7 ファイル）— Player2Provider / YamlResponseParser / InferenceQueue / Memory
- `crates/delphai-core/src/agent/citizen.rs` — LLM が依存（`Citizen` / `Emotion` / `Relationship`）
- `crates/delphai-bench/*` — ベンチ（LLM 向け）
- `game/assets/`、`game/addons/terrain_3d/`、`game/demo/`、`game/project.godot`、`game/delphai.gdextension`、`game/icon.png`
- `docs/CODEMAPS/*` — 再現ドキュメント

### 削除範囲（Sprint N0 で実施）

- `crates/delphai-core/src/{world,pathfinding,animal,resource,tech}.rs`
- `crates/delphai-core/src/agent/{behavior,conversation}.rs`
- `crates/delphai-gdext/src/lib.rs`（godot init だけの空スタブに置換）
- `game/scenes/world.{gd,tscn}`
- `game/scripts/*.gd`（すべて。`tests/smoke_terrain_assets.gd` 含む — 旧地形前提のため）

---

### ✅ Sprint N0–N4.3 完了サマリ（2026-04-20 〜 04-21）

完了済み。詳細は git log 参照。以下は「新人が初日に間違えやすい」地雷だけ残す。

**N0 削除+スタブ化**（`bc0a2c9` 以前を一掃）
- 保存: `src/llm/*` / `src/agent/citizen.rs` / `delphai-bench` / `game/assets/` / `game/addons/terrain_3d/` / `docs/CODEMAPS/`
- 削除: `src/{world,pathfinding,animal,resource,tech}.rs` / `src/agent/{behavior,conversation}.rs` / `game/scripts/**` / `game/scenes/world.*`

**N1 最小 World** — `World { tick_count, citizens: Vec<Citizen>, citizen_moves: Vec<MoveState> }` + `TilePos { x:i16, y:i16 }`（`pathfinding.rs`）

**N2 移動 + 補間** — `MoveState` と `World::get_citizen_world_pos(idx, alpha)`（prev→curr lerp）。履歴配列・cooldown は**入れない**（R5.4 の止まりバグを最初から避ける）。

**N3 gdext FFI** — `WorldNode::{initialize, tick, get_citizen_count, get_citizen_name, get_citizen_world_pos(i, alpha) -> Vector2}`

**N4 Godot 最小シーン** — `_process` で 4Hz tick-accum + 毎フレーム `get_citizen_world_pos(i, alpha)` で position 更新、`TILE_SIZE=2.0`、CapsuleMesh 仮描画

**N4.1 ランダムウォーク + カメラ** — `World::enable_random_walk(seed, MapBounds)` + `SmallRng` 棄却法。カメラ pan/zoom は `bc0a2c9` から移植。

**N4.2 Chebyshev 8 方向（破棄）** — `(3,1)` 軌跡が `(1,1)(2,1)(3,1)` で段差。人間目視 NG → N4.3 で置換。

**N4.3 unit vector / atan2 連続移動** — `MoveState.pos: (f32, f32)` + `SPEED=1.0`。`step()` は `dist ≤ SPEED` で snap、else `pos += (dx/dist, dy/dist) * SPEED`。

#### N0–N4.3 初日の地雷（必読）

1. **`pos` は float**。`tile_pos()` は `.round() as i16` で derive。walkable grid は `tile_pos()` 経由、`pos` 直接参照 NG。
2. **`step()` は target None でも `prev_pos = pos` を更新**せよ（補間が切れて 1 フレーム跳ねる）。
3. **ランダムウォークは初 tick では動かない**（`step()` が target 割当前に走る）。テストは 1 tick warmup 後に計測。
4. **タイル基準の「連続静止 ≤2」回帰テストは false-fire する**（連続移動では浅角で `round()` が同じタイルに留まる）。代わりに `distance_to_target_monotonically_decreases_during_journey`（毎 tick ユークリッド距離 strictly 減）。
5. **`MoveState` は `Eq` 不可**（f32 のため）。`PartialEq` のみ。
6. **`rand = "0.8"` は `features = ["small_rng"]` 必須**（未指定だと `SmallRng` ビルドエラー）。
7. **`delphai-gdext/Cargo.toml` の `crate-type` に `rlib` を含める**（cdylib 単体だと cargo test が走らない）。
8. **計画記述中は `[ ]`**。Edit/Write 直後のターンでしか `[x]` にしない（R5.4 の失敗の源）。
9. **コミットは 3 点検証（`cargo test` / `make build` / 人間目視）揃うまで `[x]` にしない**。人間目視は Mac で実施。
10. **`game/scenes/world.tscn` から Camera3D は除外**（スクリプト側 `_build_camera` で生成）。

**累計検証**: `cargo test --workspace` 111+6 pass / clippy 0 warn / `make build` ✅ / 人間目視は N4/N4.1/N4.2/N4.3 累積で Mac 再検証 ✅

### Sprint N5: 地形（最小）+ walkable grid

- [x] PlaneMesh ベースの最小地形（24×14 を可視化、単色 OK、Terrain3D は N9 まで使わない）
- [x] `WalkGrid`（行優先 `Vec<bool>`）と `set_walkable_map` FFI（`PackedByteArray`、0=blocked / 非0=walkable）
- [x] `move_state::step_with_grid` に局所 1-step + 履歴 8 件での詰まり回避（`World::tick` が履歴を注入）
- [x] RED → GREEN: 障害物タイルを置くと迂回する (`tick_routes_around_obstacle_when_grid_present`)
- [x] **検証 (3 点)**: `cargo test` 131 pass / clippy 0 warn / `make build` ✅（**人間目視は Mac で**）

#### N5 初日の地雷

1. **`WalkGrid` の OOB = 非 walkable**（唯一の bounds 判定）。呼び出し側でパディング不要、勝手に walkable 判定作り直すな。
2. **`step_with_grid` は target が非 walkable のときは target をクリア**して return（ループ防止）。停止判断は呼び出し側の random_walk が再サンプリングする前提。
3. **履歴は MoveState に入れない** — `MoveState: Copy` を維持したい（`Eq` 不可の制約と合わせると VecDeque を内包できない）。World に `Vec<VecDeque<TilePos>>` で外出し。
4. **履歴は「tick 後」に push**（tick 中の candidate 判定に当 tick 開始タイルを入れない）。
5. **`step_with_grid` の detour 候補は `[0°, ±45°, ±90°, ±135°]` の7つ**、非履歴優先の two-pass。`180°`（引き返し）は入れない（往復振動を避ける）。
6. **Godot 側の PackedByteArray は 0=blocked / 非0=walkable**（`bool` ではなく `u8`。FFI 境界で明示変換）。
7. **GDScript の `OBSTACLE_TILES` は `Vector2i` 配列、tscn に `Terrain` Node3D を追加**していないと `@onready` が null。
8. **`pick_random_target_on_grid` は max_attempts で必ず抜ける**（全ブロックマップで tick を無限ループさせない）。

### Sprint N6: Resource + Behavior（食料系）

- [ ] `resource.rs` 新規: `Resource { kind: Berry, amount, tile_pos }` と再生ロジック
- [ ] `agent/behavior.rs` 新規: `BehaviorState { Idle, SeekingFood, Gathering }`、純粋関数 `tick(state, needs) -> BehaviorAction`
- [ ] `world.rs` に `Vitals { fed }` と `FED_DECAY=0.004/tick`、`FED_LOW=0.4` 閾値
- [ ] RED → GREEN（TDD）
- [ ] **検証 (3 点)**: `cargo test` / `make build` / **人間目視 — 住民が berry 茂みに向かい採集する**

### Sprint N7: Water + Hydration（hydration 優先ロジック含む）

- [ ] `Resource::Water` 追加
- [ ] `BehaviorState::{SeekingWater, Drinking}` 追加
- [ ] `HYDRATION_DECAY=0.007/tick`、hydration<0.3 が fed<0.4 より優先されるテスト
- [ ] **検証 (3 点)**

### Sprint N8: Animal + Hunting

- [ ] `animal.rs` 新規: `Animal { kind: Deer, state: Roaming/Fleeing/Dead }`、逃走ロジック（citizen 接近で Fleeing、距離回復で Roaming）
- [ ] `BehaviorState::Hunting` と捕食
- [ ] `get_animal_world_pos(i, alpha)` FFI（citizen と同じ補間）
- [ ] **検証 (3 点)**

### Sprint N9: 地形リッチ化（Terrain3D 再導入 or ArrayMesh 自前）

> R1-R5 の教訓: noise-only の手 tuning は必ず破綻する。再導入するなら合成形状のみ、かつ shader uniform の hint_range を超えない値を使う。

- [ ] 技術選定: Terrain3D 再採用 vs ArrayMesh 自前（`game/demo/src/CodeGenerated.gd` の API 例を読んだ上で）
- [ ] 最小合成 heightmap（まず Gaussian 山 1 つだけ、動作確認後に増やす）
- [ ] テクスチャ適用（`uv_scale=0.1` 以下、`mipmaps/generate=true`、`auto_slope≤10`）
- [ ] smoke test: `smoke_terrain_assets.gd` を新規作成（shader param を RED で検証）
- [ ] **検証 (3 点)**

### Sprint N10: Phase 1 判定 — 5 分眺めていられるか

- [ ] 住民 5 人、berry 4、water 3、deer 3、合成地形の全部入り
- [ ] 30 秒録画 + 自己レビュー（0 A.D. 風の雰囲気が出ているか）
- [ ] 外部の人 1 人に見てもらう
- [ ] Phase 1 完了判定

---

## Phase 2: LLM 再統合 + 文明進行

- [ ] `World::tick()` に LLM キュー再接続（needs 状態をプロンプトに追加）
- [ ] `divine_awareness` 復活
- [ ] Bonsai 8B vs Gemma4 比較（採用基準: レイテンシ≤1000ms ∧ YAML 適合率≥90% ∧ 品質≥2.0、未達なら Gemma4 E2B 継続）
- [ ] 技術ツリー拡張: 石器→農業→青銅器、会話キーワードで研究ポイント加算（`tech.rs` 新規 or 再作成）
- [ ] 住民 5→10→20、部族分裂・戦争トリガー
- [ ] HDL 検討は住民20人以上で YAML がボトルネックになった時点で

## Phase 3: ポリッシュ

- [ ] セーブ/ロード、技術ツリー表示 UI、低スペック確認（GTX1060 / CPU only / M1）、外部プレイテスト 5 人

---

## やらないこと

マルチプレイ、TTS、青銅器以降、DLC、コンソール、住民 100 人超。

## リファクタ保留（必要になったら着手）

- `crates/delphai-gdext/src/lib.rs` の `#[func]` 定型コード削減
- `MAP_W`/`MAP_H` の Rust/GDScript 共有化（FFI or codegen）
- `memory_summary` 初期値の 8 をコンフィグ化

---

## 破棄した旧実装（2026-04-20）

Sprint 0〜13.R6 の実装は継ぎ足し修正の累積で破綻し、Phase 1 基準（5 分眺めていられる）を満たせなかった。以下の形で痕跡を残す:

- **コード**: git history（`bc0a2c9` 時点まで）に残る。`git log -- crates/delphai-core/src/world.rs` で復元可能
- **構造**: `docs/CODEMAPS/{architecture,backend,frontend,data,dependencies}.md` に C4 図付きで保存
- **教訓**: `@tasks/lessons.md`（特に「計画=実装の混同」「GDScript ランタイム挙動を cargo test で確認したと誤認」）

**引き継ぐべきだったこと**:

- LLM は YAML 出力（JSON より堅牢）
- TILE_SIZE=2.0 + camera 調整セット
- 村を (21, 10) SE 寄せ
- hydration 優先（fed より先に枯れる）
- サブ tick 補間は Rust 側に寄せる（GDScript では lerp しない）
- Terrain3D shader uniform の `auto_slope` は `hint_range(0, 10)`、`uv_scale` は 0.03〜0.1 の範囲

**引き継がないこと**（失敗の源）:

- noise-only heightmap
- `STEP_COOLDOWN=1, ARRIVE_COOLDOWN=8` のバースト移動
- `set_shader_param` の silent-fail を知らずに 15.0 を渡す
- 「計画記述したターン中に全 checkbox を `[x]`」のフロー
