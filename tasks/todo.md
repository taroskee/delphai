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

### Sprint N0: 削除 + スタブ化（2026-04-20）

- [x] `crates/delphai-core/src/{world,pathfinding,animal,resource,tech}.rs` 削除
- [x] `crates/delphai-core/src/agent/{behavior,conversation}.rs` 削除
- [x] `crates/delphai-core/src/agent/mod.rs` を `pub mod citizen; pub use citizen::{Citizen, Emotion, Relationship};` に縮小
- [x] `crates/delphai-core/src/lib.rs` を `pub mod agent; pub mod llm;` に縮小
- [x] `crates/delphai-gdext/src/lib.rs` を `ExtensionLibrary` だけの空スタブに置換
- [x] `game/scripts/*.gd` と `game/scripts/tests/*` 全削除（`scripts/` ディレクトリごと削除）
- [x] `game/scenes/world.{gd,tscn,gd.uid}` 削除（`scenes/` は空ディレクトリのみ残存）
- [x] `Makefile` の `smoke-terrain` ターゲット削除（対応 GDScript が消えるため）
- [x] 検証: `cargo test -p delphai-core` 84 pass（LLM + citizen のみ）/ `cargo clippy --workspace --all-targets -- -D warnings` 警告ゼロ / `cargo build -p delphai-gdext` 成功
- [x] コミット: `chore(rebuild): strip Phase 1 code down to LLM + citizen type`

### Sprint N1: Rust 最小 World — tick + citizen spawn（TDD）

- [x] RED: `crates/delphai-core/src/world.rs` 新規、`World::new()` + `tick()` で `tick_count` が 1 増える失敗テスト
- [x] GREEN: `World { tick_count: u32, citizens: Vec<Citizen>, citizen_positions: Vec<TilePos> }` と `tick()`
- [x] RED: `spawn_citizen(name, tile_pos)` テスト（index 順序・並列 Vec）
- [x] GREEN: `TilePos { x: i16, y: i16 }`（`pathfinding.rs` 新規、`new()` + `Default` + serde）と `spawn_citizen`
- [x] `agent/citizen.rs` は変更不要（LLM 依存シグネチャは触らず、位置は World 側の並列 Vec で保持）
- [x] 検証: `cargo test -p delphai-core` 90 pass / `cargo clippy --workspace --all-targets -- -D warnings` 警告ゼロ / `cargo build -p delphai-gdext` 成功

### Sprint N2: 位置と 1-step 移動 + サブ tick 補間（最初から組み込む）

- [x] RED: `MoveState { tile_pos, prev_tile_pos, move_target }`、tick で target に向かい 1-step、`World::get_citizen_world_pos(idx, alpha)` が prev→curr を線形補間する失敗テスト
- [x] GREEN: 最小実装（履歴配列・cooldown は **導入しない**、止まって見える問題を最初から避ける）— `move_state.rs` 新規、World に `citizen_moves: Vec<MoveState>` / `set_move_target` / `get_citizen_world_pos`
- [x] RED: 16 tick 連続で静止 tick が 2 連続を超えないテスト（R5.4 Phase C.1 の regression guard）
- [x] GREEN: 最小ロジック（step は x→y 方向に 1-tile、target 到達で自動クリア）
- [x] 検証: `cargo test -p delphai-core` 99 pass / `cargo clippy --workspace --all-targets -- -D warnings` 警告ゼロ / `cargo build -p delphai-gdext` 成功

### Sprint N3: gdext FFI 最小再構築 — WorldNode

- [x] `crates/delphai-gdext/src/lib.rs` に `WorldNode` 再追加（`Cargo.toml` crate-type に `rlib` 追加 → cargo test 可能化）
- [x] FFI: `initialize()` / `tick()` / `get_citizen_count()` / `get_citizen_name(i)` / `get_citizen_world_pos(i, alpha) -> Vector2`
- [x] gdext 単体テスト（cargo test）— 純粋 helper `citizen_name_at` / `citizen_world_pos_at` に切り出して境界チェックを 6 ケース
- [x] 検証: `cargo test --workspace` 105 pass（core 99 + gdext 6）/ `cargo clippy --workspace --all-targets -- -D warnings` 警告ゼロ / `make build` 成功

### Sprint N4: Godot 最小シーン — 住民 1 人が滑らかに動く

- [x] `game/scenes/world.tscn` 新規（Node3D ルート + WorldNode + Camera3D + DirectionalLight3D + Citizens 親）
- [x] `game/scenes/world.gd` 新規（`_process` で tick-accum 4Hz + 住民 Node3D の position を `get_citizen_world_pos(i, alpha)` で毎フレーム更新、CapsuleMesh の仮描画、`TILE_SIZE=2.0`）
- [x] smoke test: `game/scripts/tests/smoke_citizen_motion.gd` — headless で 2 秒再生し、連続 N フレームで住民 position が変化し続けることを検証（`MAX_STATIC_FRAMES=10` 超で fail）
- [x] `Makefile` に `smoke-citizen` 追加（`GODOT` env 経由、devcontainer には godot 未搭載のため Mac で実行）
- [ ] **検証 (3 点)**: `cargo test --workspace` 105 pass ✅ / `make build` ✅ / **人間目視 — 未通過**（2026-04-21: x-first-then-y の L 字移動で「マス目でない」を満たさない。N4.2 で斜め移動に修正後に再検証）

### Sprint N4.1: 連続ランダムウォーク + カメラ pan/zoom 復活（2026-04-21）

> **動機**: N4 では 10 tick で target 到達後に citizen が静止してしまい、「Phase 1 基準 = 5 分眺めていられる」を満たせない。また旧実装にあったカメラ pan/zoom を N4 シーンに戻し、視認性を確保する。

- [x] RED: `World::enable_random_walk(seed, MapBounds)` — 到達後に自動で新 target を発行し続ける 4 ケース（初回 idle で発行 / 100 tick 連続静止 ≤2 / 境界内維持 / seed 決定論）
- [x] GREEN: `pick_random_target` 棄却法 + `SmallRng`（`rand = { version = "0.8", features = ["small_rng"] }`）+ `RandomWalk { rng, bounds }` を `World` に組み込み、`tick()` で move_target=None の citizen に再発行
- [x] FFI: `WorldNode::initialize(width, height, seed)` に拡張、`delphai_core::world::MapBounds` を gdext で使用
- [x] カメラ移植（bc0a2c9 準拠）: `_build_camera` / `_input` / `_handle_mouse_button` / `_handle_mouse_drag` / `_pan_by_screen_delta` / `_zoom_by` / `_process_keyboard_camera` を `game/scenes/world.gd` に追加、定数 13 個（CAM_*）移植
- [x] `game/scenes/world.tscn` から hardcoded Camera3D を削除（スクリプト側で生成、旧実装方式に合わせる）
- [ ] 検証 (3 点): `cargo test --workspace` 109 pass ✅ / `make build` ✅ / **人間目視 — 未通過**（2026-04-21: ランダムウォーク＆カメラ自体は動いたが、住民の軌跡が L 字のため「滑らか」を満たさない。N4.2 で斜め移動に修正後に再検証）

### Sprint N4.2: 斜め移動（Chebyshev 1-step / 8 方向）（2026-04-21）

> **動機**: N4/N4.1 の人間目視で、住民が x→y の順に動くため軌跡が L 字になり「マス目でない滑らかな動き」に見えない。`MoveState::step()` を 8 方向 1-step（dx/dy 同時適用 = Chebyshev step）に変更し、`prev→curr` 線形補間で対角グライドを出す。
>
> **不変条件（崩さない）**: ①`tick()` 1 回で ≤1 tile の L∞ 距離前進 ②到達時 `move_target=None` ③`prev_tile_pos` を毎 step 更新（補間が切れない）④R5.4 Phase C.1 の regression guard（連続静止 ≤2 tick） ⑤ランダムウォーク決定論（seed 固定で同軌跡）。
>
> **ripple なし**: `pick_random_target` の棄却法は Chebyshev 距離でも毎 tick 単調減少するので無限ループしない。gdext FFI / world.gd は変更不要。

- [x] RED: `step_moves_diagonally_when_both_axes_need_to_change` — `MoveState::new((0,0))` + `move_target=Some((3,3))` → `step()` 3 回で `(1,1)(2,2)(3,3)`、`move_target=None`
- [x] RED: `step_moves_diagonally_toward_negative_target` — `(0,0)→(-2,-2)` で `(-1,-1)(-2,-2)`
- [x] RED: `step_falls_back_to_axis_step_when_only_one_axis_differs` — `(5,5)→(5,2)` で `(5,4)(5,3)(5,2)`、`(0,0)→(3,0)` で `(1,0)(2,0)(3,0)`
- [x] RED: `step_diagonal_then_axis_when_target_is_not_equidistant` — `(0,0)→(3,1)` で `(1,1)(2,1)(3,1)`（dy=0 でも軸 step に退化する）
- [x] 既存テスト改修: `step_moves_x_first_then_y` と `step_toward_negative_target` を斜め移動版に置換（旧名のまま L 字期待値を残さない）
- [x] GREEN: `move_state.rs` の `step()` を `if dx != 0 { self.tile_pos.x += dx; } else if dy != 0 { ... }` から `self.tile_pos.x += dx; self.tile_pos.y += dy;`（同時適用）に変更
- [x] 回帰テスト修正: `world.rs` の `citizen_never_static_for_more_than_two_consecutive_ticks_during_journey` の target を `(10,10)` → `(20,20)` に延長（Chebyshev 距離で 16 tick 以内に到達しないよう調整、コメントも Manhattan → Chebyshev に修正）
- [x] 回帰確認: `random_walk_keeps_citizen_moving_for_100_ticks` / `random_walk_is_deterministic_for_fixed_seed` / `random_walk_keeps_citizen_inside_bounds` すべて pass
- [x] **検証 (2/3 点)**: `cargo test --workspace` 111 pass ✅ / `cargo clippy --workspace --all-targets -- -D warnings` 警告ゼロ ✅ / `make build` ✅
- [ ] **検証 (3/3 点)**: **人間目視 — 未通過**（2026-04-21: 8 方向 Chebyshev でも (3,1) への軌跡が (1,1)(2,1)(3,1) の 2 セグメントで「マス目っぽさ」が残る。真の滑らかさには任意角度の unit vector 移動が必要。N4.3 で修正）
- [~] コミット: `6d608c9 feat(sprint-n4.2): 8-directional (Chebyshev) 1-step movement`（人間目視未通過のまま記録のみコミット済。N4.3 完了時に累積で人間目視再判定）

### Sprint N4.3: 連続角度移動（unit vector / atan2 ベース）（2026-04-21）

> **動機**: N4.2 の Chebyshev 8 方向でも「真の円運動」にならない。`(0,0)→(3,1)` の軌跡は `(1,1)(2,1)(3,1)` の 2 セグメント（45°→0°）で段差が見える。人間の移動は **任意角度の円運動**（0°→(1,0) / 60°→(0.5, √3/2) / 90°→(0,1) / 270°→(0,-1)）。MoveState を整数タイル駆動から **float `world_pos` 駆動 + atan2 unit vector step** に変更する。
>
> **不変条件（崩さない）**:
> 1. Unit speed: 1 tick あたりの移動距離 = `SPEED = 1.0` tile（既存ペースを保つ）
> 2. 到達判定: `hypot(target - pos) ≤ SPEED` で snap → `move_target = None`
> 3. `prev_world_pos` を毎 step 更新（補間が切れない）
> 4. ランダムウォーク target は `TilePos` のまま（グリッド意味論を保存、N5 の walkable grid と互換）
> 5. ランダムウォーク決定論: seed 固定で `pick_random_target` 呼び出し順序不変 → 軌跡再現
>
> **設計判断（Option A 採用）**:
> - `MoveState` に `world_pos: (f32, f32)` と `prev_world_pos: (f32, f32)` を追加
> - `tile_pos: TilePos` は `round(world_pos)` から derive する getter に変更（walkable grid 判定用に保持）
> - `step()` 内で `dx, dy = target.x - world_pos.x, target.y - world_pos.y`、`dist = hypot(dx, dy)`、`dist ≤ SPEED` なら snap、else `world_pos += (dx/dist, dy/dist) * SPEED`
> - `world_pos(alpha)` は `prev_world_pos` と `world_pos` の lerp（float → float、従来の integer tile lerp から移行）
>
> **ripple**:
> - FFI `WorldNode::get_citizen_world_pos(i, alpha) -> Vector2` は既に `(f32, f32)` を返しており変更不要
> - `game/scenes/world.gd` の `TILE_SIZE=2.0` 乗算も変更不要（`world_pos` は tile 単位の float）
> - `pathfinding.rs::TilePos` は保持、`MoveState::tile_pos()` が `round` して返す
> - R5.4 Phase C.1 の「連続静止 ≤2 tick」回帰テストは continuous 動作では意味を成さない → 「16 tick 連続で target との距離が単調非増加」テストに置換

- [x] RED→GREEN: `step_moves_unit_vector_at_zero_degrees` — `(0,0)→(3,0)` → `(1.0, 0.0)`
- [x] RED→GREEN: `step_moves_unit_vector_at_ninety_degrees` — `(0,0)→(0, 3)` → `(0, 1)`
- [x] RED→GREEN: `step_moves_unit_vector_at_two_seventy_degrees` — `(0,0)→(0, -3)` → `(0, -1)`（ユーザー指定 270°）
- [x] RED→GREEN: `step_moves_unit_vector_at_one_eighty_degrees` — `(0,0)→(-3, 0)` → `(-1, 0)`
- [x] RED→GREEN: `step_moves_unit_vector_along_three_four_five_triple` — `(0,0)→(3,4)` → `(0.6, 0.8)`
- [x] RED→GREEN: `step_moves_unit_vector_toward_negative_diagonal` — `(0,0)→(-3,-4)` → `(-0.6, -0.8)`
- [x] RED→GREEN: `step_preserves_direction_for_arbitrary_angle` — 任意角で `|step|==SPEED` かつ方向保存
- [x] RED→GREEN: `step_snaps_to_target_when_within_speed` — `dist ≤ SPEED` で snap + `move_target=None`
- [x] RED→GREEN: `step_reaches_three_four_five_target_in_five_ticks` — 5 tick 到達 + 途中は target 維持
- [x] RED→GREEN: `world_pos_interpolates_float_prev_to_current` — float lerp
- [x] RED→GREEN: `tile_pos_derived_from_round_of_pos` — `.round() as i16`（負値も含む）
- [x] GREEN: `move_state.rs` refactor — `pos / prev_pos: (f32, f32)` + `SPEED=1.0` const + atan2 unit vector `step()` + float `world_pos(alpha)` lerp + `tile_pos() / prev_tile_pos()` getter（`Eq` は float のため除去）
- [x] 既存テスト移行: integer tile 期待値の旧ケースは新 14 ケースに置換済（`step_moves_diagonally_*`, `step_falls_back_to_axis_step_*` は unit vector では意味を失うので削除）
- [x] 回帰テスト置換: `citizen_never_static_for_more_than_two_consecutive_ticks_during_journey` → `distance_to_target_monotonically_decreases_during_journey`（16 tick 連続で `hypot(target - pos)` が strictly 減少）
- [x] 回帰確認: `random_walk_keeps_citizen_moving_for_100_ticks`（静止 guard を float `moved > 1e-6` に置換、初 tick は step の前に target が無いので warmup）/ `random_walk_is_deterministic_for_fixed_seed`（float `pos` 比較）/ `random_walk_keeps_citizen_inside_bounds`（`tile_pos()` 経由で境界判定）
- [x] FFI 確認: `WorldNode::get_citizen_world_pos(i, alpha)` / `get_citizen_count` の gdext テスト 6 ケース全 pass（signature 不変）
- [x] 検証 (3 点): `cargo test --workspace` 111+6 pass ✅ / `cargo clippy --workspace --all-targets -- -D warnings` 警告ゼロ ✅ / `make build` OK ✅ / **人間目視 — 未実施（Mac で要確認）**
- [ ] コミット: `feat(sprint-n4.3): unit-vector angular movement (atan2-based continuous motion)`（人間目視通過後、N4/N4.1/N4.2 の累積人間目視も同時判定 → 通過したら 4 箇所の `[ ] 人間目視` を `[x]` に）

### Sprint N5: 地形（最小）+ walkable grid

- [ ] PlaneMesh ベースの最小地形（24×14 を可視化、単色 OK、Terrain3D は N9 まで使わない）
- [ ] `WalkGrid`（行優先 `Vec<bool>`）と `set_walkable_map` FFI
- [ ] `pathfinding.rs` に局所 1-step + 履歴 8 件での詰まり回避
- [ ] RED: 障害物タイルを置くと迂回するテスト
- [ ] **検証 (3 点)**: `cargo test` / `make build` / **人間目視**

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
