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

**新課題: 視認性 低 / ポリゴン重なり（Phase 1 判定未達成）:**
- [ ] スクリーンショット 2026-04-20: ゲーム画面の地形・市民が区別しづらい
  - 単色グリーン テクスチャだけでは高度マップが視認できない（谷・尾根が見えない）
  - 市民（Tree_Stump_01）が茶色い球で埋もれている（GLB 人型モデル未取得のため）
  - **対策**: 13.2 のテクスチャ分類（高度 → grass/shallow/deep/mountain カラバリ）で高度マップを可視化
- [x] IDE 診断: `mcp__ide__getDiagnostics` で GDScript parser error なし確認
- [ ] Godot 実機起動で村・木・鹿・住民・水源・焚き火が既存位置に一致（ユーザ手動確認）
- [x] `cargo test -p delphai-core`（165 passed 維持）
- [x] `cargo clippy -p delphai-core -p delphai-gdext`（警告ゼロ）

#### 13.2 高度マップ → タイル分類（共通層）

> **key**: この分類ロジック＝水源・速度・鉱石配置の基盤。MVP/post-MVP で同じ。

- [ ] `TerrainBuilder.classify_tiles_from_height(heightfield, map_w, map_h, tile_size) -> Array[int]`:
  - `y < -1.0` → `T_DEEP`
  - `-1.0 <= y < 0.3` → `T_SHALLOW`
  - `y > 5.0` or `slope > 45°` → `T_MOUNTAIN`
  - `FastNoiseLite` forest layer > 0.6 → `T_FOREST`
  - 他 → `T_GROUND`
  - 起動時 1 回計算 → `_tile_cache` 配列に保存
- [ ] `world.gd::get_terrain(col, row)` を キャッシュ配列ルックアップに変更（O(1)、内容のみ変、API 同じ）
- [ ] `make_walkable_map()` は結果をそのまま利用（内容変更、API 不変）

#### 13.3 住民・資源・動物の地表吸着

- [ ] `world.gd::tile_to_world(col, row)` で `get_height_at` 経由 Y 取得（既存 seam、動作確認）
- [ ] `_build_citizens()`, `_build_resources()`, `_build_animals()` が get_height_at を呼ぶか確認（13.01 完了済み）

#### 13.4 ゲームロジック互換性

- [ ] Rust `tick_movement` が SHALLOW で速度半減、DEEP を避けるか検証
- [ ] 鹿が T_DEEP を回避するか確認

#### 13.5 検証 + コミット

- [ ] Godot 実機起動 → SE 村を 5 分以上眺めていられるか（**Phase 1 判定**）
- [x] `cargo test -p delphai-core` 165 passed，`cargo check -p delphai-gdext` 成功
- [ ] コミット: `feat(terrain): procedural Terrain3D generation, shared MVP+post-MVP layer`
- [ ] `game/assets/inventry.md` 更新: terrian.glb → Terrain3D ProcGen 方針
- [ ] `tasks/lessons.md` に Terrain3D seed 固定の理由を追記

#### post-MVP への橋渡し（未着手）

seed を乱数化 + 分類閾値をワールド設定化するだけ。共通層（水源・速度・鉱石）は追加作業なし。

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
