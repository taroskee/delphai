# MVP TODO

**MVP:** 住民5–10人が協力して狩猟・採集・採水し、3段階の技術進化を遂げる俯瞰3Dシミュ。
LLM は Phase 2 で再活性化（Phase 1 中は外してある）。視覚参考: 0 A.D. 風。

**Phase 1 基準**: 起動直後に 5分以上眺めていられる（没入感・視覚的リッチさ）。

---

## 検証順（コミット前）

1. `cargo test -p delphai-core`（現在 165 passed）
2. `cargo clippy -p delphai-core -p delphai-gdext`（警告ゼロを維持）
3. IDE 診断: `mcp__ide__getDiagnostics` で GDScript parser error なし確認
4. `make build`（`.so`、gdext cdylib）
5. Godot シーン起動確認（UI/3D を変えた場合）

詳細は @tasks/lessons.md の「初日に踏みやすい地雷」を参照。

---

## 現在のスプリント (Phase 1)

> Phase通過判定: 5分以上眺めていられるか

### Sprint 13: 地形システム根本再設計（Terrain3D 導入）

> **根本原因**: terrian.glb がゲーム物理と分離。住民が GLB 地表に接地しない。
> **ゴール**: 地形が視覚 + 物理 + 論理を統一。5分以上眺めていられる没入感を回復。

#### 13.1 Terrain3D プロシージャル生成 + リファクタ下地 + Parser Error 修正

> **目的**: FastNoiseLite で起動時に高度マップ生成 → Terrain3D に流す。村中心を force-flat で平坦化。Phase 1 判定「5分以上眺めていられるか」に向けた根本的な地形統一。
> **根本原因 (13.01)**: terrian.glb がゲーム物理と分離。住民が GLB 地表に接地しない。
> **方針**: FastNoiseLite で起動時に高度マップ生成 → Terrain3D に流す（post-MVP では seed を乱数化するだけで再利用可能）。

**リファクタ下地（13.01 完了項目）:**
- [x] `TerrainBuilder.get_height_at(x: float, z: float) -> float` 新設（seam 維持）
- [x] `TerrainBuilder.build_ground` を 2 関数に分割: `build_collision_plane` + `build_visual_backdrop`
- [x] `world.gd::tile_to_world` / `_map_center` / `resource_factory.gd::make` / `build_features` の Y を `get_height_at` 経由に（挙動不変）
- [x] コミット: `refactor(terrain): extract height-at seam for Sprint 13 Terrain3D prep`

**Terrain3D 実装（13.1 完了項目）:**
- [x] Godot Asset Library から `Terrain3D` をインストール
- [x] `game/project.godot` に `[editor_plugins]` セクション追加
- [x] `terrain_builder.gd::build_terrain3d(parent, village_center) -> Terrain3D`:
  - `FastNoiseLite` 固定 seed=42、frequency=0.05、TERRAIN_HEIGHT_SCALE=5.0
  - `Image.FORMAT_RF` に高度値を書き込み（512×512px）
  - `terrain.data.import_images([img, null, null], Vector3.ZERO, 0.0, TERRAIN_HEIGHT_SCALE)`
  - `Terrain3DAssets` に単色 grass テクスチャ 1 枚（**MVP 最小。カラバリ 4 種は 13.2 で追加**）
  - **問題: 単色グリーンだけでは高度マップが視認できない** → テクスチャ分類が必須
- [x] `get_height_at(x, z)` を `terrain.data.get_height(Vector3(x, 0, z))` に実装 + NaN→0.0 ガード
- [x] `world.gd::_ready` で `build_terrain3d` を先に実行
- [x] 村 SE (21,10) を force-flat オーバーレイで平坦化（VILLAGE_FLAT_RADIUS=10m、FADE_RADIUS=15m）

**Parser Error 修正（13.1.1 完了項目）:**
- [x] `terrain.collision.mode = Terrain3DCollision.DYNAMIC` → `DYNAMIC_GAME` に変更（ランタイム用途）
- [x] `tasks/lessons.md` に enum 確認方法を追記（addon バイナリの strings で確認）
- [x] コミット: `fix(terrain): use DYNAMIC_GAME — Terrain3DCollision has no DYNAMIC member`

#### 13.2 高度マップ → タイル分類（共通層）+ 視認性回復 試行

> **目的**: 単色 grass だけでは procgen 高度が見えず Phase 1 判定失敗。タイル分類キャッシュ + auto-shader slope-blend で視覚的に高度マップを可視化する。ただし結果として視認性改善不十分。

**実装完了:**
- [x] `TerrainBuilder._classify_layout()` 抽出、`classify_tiles_from_height()` で procgen 高所を T_MOUNTAIN 昇格
- [x] `TerrainBuilder.get_terrain()` を O(1) キャッシュ化（API 互換）
- [x] Terrain3D auto_shader：grass↔dirt 斜面ブレンド（texture 1 に茶色 dirt 追加）
- [x] `world.gd::_ready` で cache 初期化タイミング確保
- [x] `cargo test` 165 passed / `cargo clippy` 警告ゼロ / IDE parser error なし

**フィードバック (2026-04-20)**: 視認性改善されず。高度マップの尾根/谷は部分的に褐色化するも、ポリゴン重なりは解消せず。市民（Tree_Stump_01 placeholder）も草に埋まったまま。

**分析**: auto-shader の auto_slope=10 では斜面判定が小さすぎて、procgen ノイズスケール（freq=0.05 → 波長 20m）では勾配が緩く、大部分が grass のままで texture 1 に到達しない。視覚的な高度差を出すには:
- (a) より急峻な noise (freq↑ または height_scale↑)
- (b) auto-shader の auto_slope を下げる（より緩い勾配で blend）
- (c) コントロールマップベースの per-texture 制御（手写し必須、Sprint 13.2 で回避）

**次:**

#### 13.3 視認性改善案：terrain noise 急峻化

> **仮説**: auto_shader blend が active にならないのは procgen が緩すぎる。freq=0.05 で 20m 波長のなだらかな noise では、斜面角度が auto_slope=10 以下に留まり大部分 grass のまま。視認性を出すには noise をより急峻にする。

- [ ] TERRAIN_NOISE_FREQ: 0.05 → 0.08 or 0.10 に上げて短波長化（より急峻な ridge）、または TERRAIN_HEIGHT_SCALE: 5.0 → 8.0 に上げて高さ倍増
- [ ] auto_slope パラメータ: 現在 10.0 → 5.0 に下げて blend threshold を緩くする（より緩い勾配で grass→dirt）
- [ ] Godot で村が見分けやすくなるか検証
- [ ] 村 force-flat（VILLAGE_FLAT_RADIUS=10m）が narrow terrain と干渉しないか確認

#### post-MVP への橋渡し（未着手）

seed 乱数化 + 分類閾値・noise パラメータのワールド設定化。共通層（water source / movement speed / ore placement）は追加作業なし。

---

## Phase 2: LLM 再統合 + 文明進行

- [ ] `World::tick()` に LLM キュー再接続（needs 状態をプロンプトに追加）
- [ ] `divine_awareness` 復活
- [ ] Bonsai 8B vs Gemma4 比較（採用基準: レイテンシ≤1000ms ∧ YAML 適合率≥90% ∧ 品質≥2.0、未達なら Gemma4 E2B 継続）
- [ ] 技術ツリー拡張: 石器→農業→青銅器、会話キーワードで研究ポイント加算
- [ ] 住民 5→10→20、部族分裂・戦争トリガー
- [ ] HDL 検討は住民20人以上で YAML がボトルネックになった時点で

## Phase 3: ポリッシュ

- [ ] セーブ/ロード、技術ツリー表示 UI、低スペック確認（GTX1060 / CPU only / M1）、外部プレイテスト 5 人

---

## 踏みやすい地雷 & 教訓

このセクションは新しいチームメンバーが初日に間違えることだけ記録。詳細は `@tasks/lessons.md` 参照。

### Terrain3D

- **enum 値を推測するな** → `strings game/addons/terrain_3d/bin/libterrain.*.so` で確認。`DYNAMIC` 単体は存在しない；`DYNAMIC_GAME`（ランタイム）を使用。
- **auto_shader の slope threshold は仕様次第** → `auto_slope=10` が「デフォルト急峻」と仮定するな。procgen freq や height_scale で斜面勾配が変わる。テスト必須。
- **force-flat オーバーレイで村を平坦化** → seed 探索より reliable。`VILLAGE_FLAT_RADIUS=10m` 内は 0m 強制、`FADE_RADIUS=15m` まで liner fade。

### ゲームロジック

- **`MAP_W`/`MAP_H` は Rust と GDScript で重複定義** → FFI 共有なし。片方変えたら必ず両方同期。
- **地形の通行可否：`T_DEEP` と `T_MOUNTAIN` を `0` で送る** → `make_walkable_map()`。新地形追加時は両方を更新。
- **`World::tick()` は phase-split 済み** → `tick_decay → tick_resources → ... → maybe_spawn_citizen`。順序変更前に各本文を読むこと。

### 検証順序（コミット前 4 つ）

1. `cargo test -p delphai-core`（165 passed 維持）
2. `cargo clippy -p delphai-core -p delphai-gdext`（警告ゼロ）
3. IDE 診断：`mcp__ide__getDiagnostics` で GDScript parser error なし確認
4. Godot 起動確認（UI/3D 変更時）

---

## やらないこと

マルチプレイ、TTS、青銅器以降、DLC、コンソール、住民 100 人超。

## リファクタ保留（必要になったら着手）

- `crates/delphai-gdext/src/lib.rs` の `#[func]` 定型コード削減
- `MAP_W`/`MAP_H` の Rust/GDScript 共有化（FFI or codegen）
- `memory_summary` 初期値の 8 をコンフィグ化

---

## 完了 Phase

### Phase 0: MVP 基盤構築（Sprint 1-4）

Godot 4 + Rust FFI で 3D 世界構築。村中心カメラ、昼夜サイクル、リソース・市民システム基盤、UI フレームワーク、BGM、パンズーム入力対応。

### Phase 1 完了（Sprint 5-12）

**Sprint 5-6**: 技術ツリー（石器）、文明成長ロジック、研究ポイント UI、unlock 通知。
**Sprint 7-8**: 狩猟・採集・採水、チャットバブル、鹿 AI（逃げ・遠隔リスポーン）、リソース隠蔽。
**Sprint 9**: 地形・移動・市民システム完成、TILE_SIZE 倍増で世界スケール向上。
**Sprint 10-12**: GLB アセット全置換（動物・木・資源・市民）、視認性向上（村 SE 配置、カメラ調整、入力拡張）。
