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

### Sprint 13: 地形システム再設計（合成形状生成）

**Phase 13 (13.1/13.2/13.3) 全失敗の根本原因**: "noise-only + slope blend" では、意図したランドマーク（山・川）が作れない。FastNoiseLite の出力はほぼ均一な micro-ridges で、auto_slope threshold を下げても視覚的に「山」「川」に見えない。パラメータ tuning では解決不可能。

**新設計の方針**: 明示的な形状（Gaussian 山 + パラメトリック川）をベース noise に合成する。リファレンス画像 `pngtree-a-serene-mountain-landscape-*.png` が目標（2つの雪山 + 曲がりくねった川 + 緑の草原）。

#### Sprint 13.R0: 人間のみ表示への単純化（視覚ベースライン確立）

**背景**: `2026-04-20.png` のスクショで、浮遊する木 (緑の葉+茶色の幹) と灰色の岩が地形上に残っており、R1-R3 の合成地形の検証を阻害している。一度すべての装飾を落とし、地形＋人間だけの最小構成に戻す。

- [x] `world.gd _ready()` で非人間ビルダーを無効化:
  - `TerrainBuilder.build_features`（樹木 GLB）
  - `VillageBuilder.add_campfire`（焚火）
  - `_build_resources`（resource メッシュ）
  - `_build_animals`（鹿）
- [x] 地形系（`build_terrain3d` / `build_river_water` / `classify_tiles_from_height` / `build_collision_plane` / `make_walkable_map`）は維持（人間の歩行判定に必要）
- [x] `_update_resources` / `_update_animals` のループ上限を `_world_sim` カウントからローカル配列サイズに変更（ビルダー無効化時の index out-of-bounds 対策）
- [ ] 検証: Godot 起動で「人間以外のポリゴンが無い」ことを目視確認 (cargo test 165 passed / clippy 0 warnings / IDE diagnostics clean / make build OK 済)

#### Sprint 13.R1: 合成 heightmap 生成器（手続き的に意図した形を描く）

- [x] `_generate_heightmap` を全面書き直し：`base_noise + mountain_1 + mountain_2 + river_trench + village_flat`
- [x] 新定数: `MOUNTAIN_1_POS/HEIGHT/SIGMA`, `MOUNTAIN_2_POS/HEIGHT/SIGMA`, `RIVER_*`（sine-curve path + trench width/depth）
- [x] `TERRAIN_HEIGHT_SCALE` を 8→20 に引き上げ（山 15m の余裕）
- [x] ベース noise 振幅を ±0.6m に抑制（草原を穏やかに）
- [x] 村（VILLAGE_CENTER 周辺）の force-flat overlay は維持

#### Sprint 13.R2: 川水面メッシュ（Terrain3D trench に追従する青い帯）

- [x] `TerrainBuilder.build_river_water(parent)` 追加：SurfaceTool triangle strip で sine-curve に追従
- [x] 水面 Y = -1.0m（trench 底 -2m と lip 0m の中間）
- [x] 幅は RIVER_WATER_WIDTH_M=3.0m（trench 幅と一致）
- [x] StandardMaterial3D albedo (0.25, 0.45, 0.75) + metallic 0.2 + alpha 0.85
- [x] `world.gd _ready` から呼び出し配線済み

#### Sprint 13.R3: タイル分類の再ベース化（形状駆動）

- [x] `_classify_layout` から hardcoded `RIVER_DEEP_COL / RIVER_SHALLOW_COLS` 削除
- [x] `classify_tiles_from_height` でのみ分類：`h > 3m → MOUNTAIN`, `h < -1.2m → DEEP`, `h < -0.3m → SHALLOW`, else `GROUND/FOREST`
- [x] `MOUNTAIN_CORNER_MAX / MOUNTAIN_DIAG_MAX` も削除（高さ駆動で十分）
- [x] 森は NW/SW 領域の layout ルール残置（procgen 無関係）

#### Sprint 13.R4: auto-shader 調整 + 検証

- [ ] `auto_slope` を山が dirt になる値に再調整（実測で決定、初期値 15.0）
- [ ] 検証: `cargo test -p delphai-core` 165 passed / `cargo clippy` 警告ゼロ / `mcp__ide__getDiagnostics` parser error なし / Godot 起動して 2 山 + 川 + 草原がリファレンスに近いか目視
- [ ] 村 SE の平坦性を再確認（住民が斜面に浮かない）

#### post-MVP への橋渡し（未着手）

seed / 山座標 / 川カーブ係数のワールド設定化。複数河川、湖、森の procgen。

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

### Phase 1 完了（Sprint 5-13）

**Sprint 5-6**: 技術ツリー（石器）、文明成長ロジック、研究ポイント UI、unlock 通知。
**Sprint 7-8**: 狩猟・採集・採水、チャットバブル、鹿 AI（逃げ・遠隔リスポーン）、リソース隠蔽。
**Sprint 9**: 移動・市民システム完成、TILE_SIZE 倍増で世界スケール向上。
**Sprint 10-12**: GLB アセット全置換（動物・木・資源・市民）、視認性向上（村 SE 配置、カメラ調整、入力拡張）。

**Sprint 13: 地形システム再設計（13.1-13.3 失敗 → 13.R 合成形状へ全面再設計）**

terrian.glb（装飾 GLB、物理なし）を Terrain3D に置換する初期統合は成功。しかし noise-only アプローチは 13.1-13.3 で失敗し、13.R で合成形状生成（Gaussian 山 + パラメトリック川 + ベース noise）へ全面再設計した。force-flat village overlay と tile classification キャッシュは引き続き活用。

**失敗した試行（13.1-13.3）**:
- 13.1: Terrain3D 統合 + FastNoiseLite (seed=42, freq=0.05, height_scale=5.0) + force-flat overlay。地形がほぼ flat
- 13.2: auto-shader slope blend inactive。slope が threshold 以下でほぼ全面 GRASS、視認性ゼロ
- 13.3: freq 0.05→0.08 / height_scale 5→8 / auto_slope 10→5 tuning。micro-ridges は増えたが意図したランドマーク（山・川）は依然不在

**教訓**: noise パラメータ tuning で「偶然」リファレンスに近い景観を得るのは不可能。MVP 目標（2山+川+草原）は意図して描かないと出ない。

**13.R 再設計（採用）**: heightmap = `gentle_noise(±0.6m) + gaussian_mountain_1(15m, σ=7m) + gaussian_mountain_2(12m, σ=8m) + sine_river_trench(-2m, width=3m) + village_flat`。`TERRAIN_HEIGHT_SCALE` 8→20。川水面は Terrain3D trench 上に青い Plane strip。タイル分類は hardcoded 列座標を廃止し heightmap 駆動に（`h>3m→MOUNTAIN`, `h<-0.3m→SHALLOW/DEEP`）。

**市民視覚**: Tree_Stump_01 GLB → チェスポーン primitive (cylinder 3-part: base+body+head) に戻し、可読性を復帰。
