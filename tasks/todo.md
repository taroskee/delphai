# MVP TODO

**MVP:** 住民5–10人が協力して狩猟・採集・採水し、3段階の技術進化を遂げる俯瞰3Dシミュ。
LLM は Phase 2 で再活性化（Phase 1 中は外してある）。視覚参考: 0 A.D. 風。

**Phase 1 基準**: 起動直後に 5分以上眺めていられる（没入感・視覚的リッチさ）。

---

## 検証順（コミット前）

1. `cargo test -p delphai-core`（現在 165 passed）
2. `cargo clippy -p delphai-core -p delphai-gdext`（警告ゼロを維持）
3. `make build`（`.so`、gdext cdylib）
4. Godot シーン起動確認（UI/3D を変えた場合）

詳細は @tasks/lessons.md の「初日に踏みやすい地雷」を参照。

---

## 現在のスプリント (Phase 1)

> Phase通過判定: 5分以上眺めていられるか

### Sprint 10: GLB アセット適用

> 判定: 動物・資源・森が GLB に差し替わり、視覚的にリッチになる。

- [x] 動物: `Animals_FREE.glb` → `deer`
- [x] 資源・茂み: `simple_nature_pack_glb.glb` → `Plant_01`
- [x] 森: `Oak_Tree_01`
- [x] 水源: `Rock_01` を 3 つ円周配置（プリミティブ CylinderMesh 廃止）
- [x] 住民: `Tree_Stump_01` をプレースホルダ（チェスポーン廃止）
- [x] 地形: `terrian.glb` を装飾背景として配置

### Sprint 11: 視認性向上 + 入力拡張

> ゴール: 起動直後に「何が起きているか」が分かる画面。マウス/トラックパッド/キーボードで自由に見渡せる。

- [x] 村中心を SE 平坦部 (21, 10) に移動、カメラフォーカス
- [x] 焚き火ランドマーク（石の輪 + 発光燠）配置
- [x] 入力拡張: ピンチズーム、2 指パン、WASD/矢印パン、キーボードズーム、右ドラッグ
- [x] 環境アンビエント 0.5→0.7、カメラズーム 36→22 に調整

### Sprint 12: 地形 GLB 完全化 + 南東スポーン

> ゴール: 独自ポリゴンは焚き火以外全削除。terrian.glb が唯一の地形視覚。
> 背景: ユーザが Blender で `terrian.glb` の南東を平坦化。
> **問題**: 現在も「terrian.glb が装飾レイヤのままで、ゲームロジックと無関係」のため、GLB 地表と住民が一致しない

- [x] 緑 PlaneMesh、茶色山ポリゴン削除（GLB に任せる）
- [x] 市民・水源を GLB アセット化
- [x] 村 SE に配置、カメラ・スポーン座標更新
- [ ] **[Sprint 13 へ延期]** `GROUND_GLB_SCALE` / `GROUND_GLB_Y` 微調整

### Sprint 13: 地形システム根本再設計（Terrain3D 導入）

> **根本原因**: terrian.glb がゲーム物理と分離。住民が GLB 地表に接地しない。
> **ゴール**: 地形が視覚 + 物理 + 論理を統一。5分以上眺めていられる没入感を回復。

#### 13.1 Terrain3D プラグイン導入

- [ ] Godot Asset Library から `Terrain3D` をインストール（Godot 4.3+）
- [ ] `Terrain3D` プロジェクト設定: 4096×4096 ハイトマップ、TILE_SIZE=2.0 のグリッドに整列
- [ ] 既存 `terrian.glb` のハイトマップを `Terrain3D` にベイク（Blender で高さデータ抽出）
  - **参考**: `terrian_original.glb` (78MB) の高さフィールドをサンプリング → 16bit灰度テクスチャに変換
  - Godot の `Image` API で生成、またはフリーツール（Meshlab / Blender heightmap export）を使用
- [ ] テクスチャペイント: 草地（緑）・浅瀬（薄青）・深水（濃青）・山（灰）を割り当て
- [ ] 自動 LOD + 衝突形状（`Terrain3D` が生成）

#### 13.2 タイル分類の Terrain3D ハイトマップ導出

- [ ] `TerrainBuilder.sample_terrain_from_terrain3d(terrain3d_node)` 新設:
  - 各タイル(col,row)中心から Raycast で地表Yを取得
  - 法線勾配から傾斜判定（>45° → T_MOUNTAIN）
  - Y 高さから水判定（<-1.0 → T_DEEP、<0 → T_SHALLOW）
  - 起動時1回サンプリング → 配列キャッシュ
- [ ] 既存 `get_terrain()` を キャッシュ配列ベースに変更（O(1) ルックアップ）
- [ ] `make_walkable_map()` は結果をそのまま使用（内容のみ変更、API 不変）

#### 13.3 住民・資源・動物の地表吸着

- [ ] `_snap_to_terrain(pos: Vector3, terrain3d_node) -> Vector3` ヘルパー実装
  - 水平位置からハイトマップ高さを Raycast で取得
  - Node3D の `.position.y` を吸着
- [ ] `_build_citizens()`, `_build_resources()`, `_build_animals()` で吸着処理を追加
- [ ] 結果: 住民が GLB 地表に自然に立つ、川に落ちない、山に埋まらない

#### 13.4 ゲームロジック互換性確認

- [ ] Rust の `tick_movement` が新タイル分類を尊重するか確認（walking bitmap 変更 → 検証）
- [ ] 住民が shallow タイル (col=17,19) で移動速度が半減するか（Sprint 12.2 deferred）
- [ ] 鹿が T_DEEP を避けるか

#### 13.5 検証 + コミット

- [ ] 実機確認: 起動して SE 村を眺める → **5分以上眺めていられるか** （**これが Phase 1 基準の通過判定**）
- [ ] `cargo test -p delphai-core`（165 passed 維持）
- [ ] `cargo clippy` / `make build` ゼロエラー
- [ ] コミット: `refactor(terrain): integrate Terrain3D, unify visual+physics+logic`
- [ ] `game/assets/inventry.md` 更新: terrian.glb → Terrain3D への移行を記載
- [ ] `CLAUDE.md` に Terrain3D セットアップ手順を追記（新メンバー向け）

#### リスク & フォールバック

| リスク | 兆候 | フォールバック |
|---|---|---|
| Terrain3D が delphai の scale に合わない | 導入 30 分で明らか | terrian.glb Raycast 手動実装（Option D） |
| ハイトマップベイク失敗 | Blender 変換で高さ損失 | Sketchfab モデルを Godot Mesh に直接変換、`ConcavePolygonShape3D` で衝突化 |
| パフォーマンス低下（起動遅延） | 3 秒以上 | Terrain3D LOD 調整 、またはハイトマップ解像度削減（2048×2048） |

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

## Phase 4: リリース

- [ ] Steam + itch.io、トレーラー、ストリーマー配布

---

## やらないこと

マルチプレイ、TTS、青銅器以降、DLC、コンソール、住民 100 人超。

## リファクタ保留（必要になったら着手）

- `crates/delphai-gdext/src/lib.rs` の `#[func]` 定型コード削減
- `MAP_W`/`MAP_H` の Rust/GDScript 共有化（FFI or codegen）
- `memory_summary` 初期値の 8 をコンフィグ化
