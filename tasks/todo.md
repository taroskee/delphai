# MVP TODO

**MVP:** 住民5–10人が協力して狩猟・採集・採水し、3段階の技術進化を遂げる俯瞰3Dシミュ。
LLM は Phase 2 で再活性化（Phase 1 中は外してある）。視覚参考: 0 A.D. 風。

## 初日に踏みやすい地雷 (新メンバー向け)

- **macOS dylib は devcontainer 内でビルドできない** — `.so` は Linux 用。Mac では `make build-mac`（Mac ターミナル、devcontainer 外）。CI が `prebuilt/macos/` にコミット済み。
- **LLM サブシステム (`crates/delphai-core/src/llm/`) は残す** — Phase 2 用に温存されている。未使用に見えても削除しない。
- **`crates/delphai-core/src/world.rs` の `tick()` は phase-split 済み** — `tick_decay → tick_resources → tick_behaviors → tick_stationary_interactions → tick_movement → tick_hunting → tick_animals → maybe_spawn_citizen`。順序変更前に各 `tick_*` 本文を読むこと。
- **`game/scenes/world.gd` は薄いオーケストレータ (< 400 行)** — 構築ロジックは `game/scripts/{terrain_builder,citizen_factory,animal_factory,resource_factory,debug_hud}.gd` に分離。新しい責務を足すときは `world.gd` に戻さず、新スクリプトか既存ヘルパーに追加する。
- **`MAP_W`/`MAP_H` は Rust と GDScript 両方で重複定義されている** — FFI 生成は未導入。片方変えたら必ず両方同期。
- **地形の通行可否は `TerrainBuilder.make_walkable_map` → `World.set_walkable_map`** — 深い川 (T_DEEP) と山 (T_MOUNTAIN) を `0` で送る。新しい通行不可地形を足すときは両方を更新。

## 検証順（コミット前）

1. `cargo test -p delphai-core`（現在 165 passed）
2. `cargo clippy -p delphai-core -p delphai-gdext`（警告ゼロを維持）
3. `make build`（`.so`、gdext cdylib）
4. Godot シーン起動確認（UI/3D を変えた場合）

---

## 現在のスプリント (Phase 1)

### Sprint 10: GLB アセット適用

> 判定: 動物・資源・森が GLB に差し替わり、視覚的にリッチになる。

- [x] 動物: `Animals_FREE.glb` → `deer` を `AnimalFactory.make_deer()` で採用（GLB に animation 無し → `AnimationPlayer` はスコープ外）
- [x] 資源・茂み: `simple_nature_pack_glb.glb` → `Plant_01` を `ResourceFactory` で採用（外側 Node3D ラッパで quantity scale と分離）
- [x] 森: `simple_nature_pack_glb.glb` → `Oak_Tree_01` を `TerrainBuilder._add_tree` で採用
- [x] `game/assets/inventry.md` を解析ノード一覧・適用チェック付きで更新
- [x] 地形: `terrian.glb` をモノリス装飾背景として平面に重ねる（Sketchfab 単一メッシュ・タイル分割なし → 格子判定は既存 bitmap を維持）
- [ ] 水源: nature pack に水メッシュなし → primitive Cylinder 継続
- [ ] 住民: 所有 GLB は Unreal 向けのみ、Godot 変換未了 → チェスポーン継続

### Sprint 11: 視認性向上 + 入力拡張（最優先）

> ゴール: 起動直後に「何が起きているか」が分かる画面。マウス/トラックパッド/キーボードで自由に見渡せる。
> 背景: ユーザ報告「茶色い背景をすごいズームで見てて何やってるか分からず面白くなかった」。

#### 11.1 配置とカメラ再設計

- [x] 住民3人を村中心 (10, 8) 付近にクラスタ配置（`crates/delphai-gdext/src/lib.rs::initialize`）
- [x] 資源・鹿を両森林 + 川沿いに再配置（ベリー4・水源3・鹿3）
- [x] カメラを村中心にフレーミング — focus を `tile_to_world(VILLAGE_CENTER_COL, VILLAGE_CENTER_ROW)` に
- [x] デフォルトズームを近く（`CAM_ZOOM_DEFAULT = 22.0`、従来 36.0）に寄せる
- [x] 村中心に焚き火ランドマーク — 新 `game/scripts/village_builder.gd`（石の輪 + 発光する燠 + `OmniLight3D`）

#### 11.2 入力拡張

- [x] トラックパッド ピンチズーム — `InputEventMagnifyGesture` を `_input` で処理
- [x] トラックパッド 2 本指パン — `InputEventPanGesture`
- [x] キーボード パン — WASD / 矢印キー（`_process_keyboard_camera` でフレーム毎サンプリング）
- [x] キーボード ズーム — `+` / `=` / `-`
- [x] 右クリックドラッグパン（ミドルボタンと同じハンドラ）
- [x] ズーム/パンを関数分割（`_handle_mouse_button`, `_zoom_by`, `_pan_by_screen_delta`）
- [x] パン加速 — ドラッグ速度ベース倍率（1x〜4x）+ キーボード押下継続時間ランプ（1x〜3x、1.5/秒）

#### 11.3 背景整合

- [x] 環境アンビエント 0.5→0.7（近接ズームで黒く見える緩和）
- [ ] `terrian.glb` バックドロップがプレーンから突き出さないか起動時に目視確認（必要なら scale/Y 調整）

### Sprint 12: 地形 GLB 完全化 + 川ロジック整備 + 南東スポーン（最優先）

> ゴール: `terrian.glb` (南東平坦化版) が唯一の地形視覚。独自ポリゴンは焚き火以外全削除。
> 背景: ユーザが Blender で `terrian.glb` の南東を平坦化 (74MB, 元は `terrian_original.glb` 78MB)。
> 「茶色や緑の独自3Dポリゴンは没入感なくなる、焚き火以外は全てglbにしましょ」

#### 12.1 地形独自ポリゴン全削除（焚き火のみ残す）

- [x] `TerrainBuilder.build_ground`: 緑 PlaneMesh の MeshInstance3D を削除（StaticBody+CollisionShape は接地用途で残す）
- [x] `TerrainBuilder.build_features` → `_add_mountain` 呼び出し削除（T_MOUNTAIN walkable=0 は維持、視覚は GLB 任せ）
- [x] `TerrainBuilder.build_features` → `_add_water_plane` 呼び出し削除（川は GLB 側に表現あり）
- [x] `TerrainBuilder._add_tree_fallback` 削除（Oak GLB load 失敗なら無描画、Sprint 10 で安定動作確認済）
- [x] `VillageBuilder` 焚き火（石の輪 + 燠 + OmniLight3D）は **残す** — ランドマーク用途、ユーザ明示指示

#### 12.2 川ロジック整備（鉛直軸ベース）

- [x] T_SHALLOW (cols 17, 19): walkable=1 維持、**水源 Resource** を南東近傍の shallow タイル (col=19 の row=9,10,11) に配置
- [x] T_DEEP (col=18): walkable=0 維持（移動不可）
- [ ] **[deferred → Sprint 13]** T_SHALLOW タイル通過時の移動速度半減（`World::set_terrain_type_map` + `tick_movement` 速度係数）

#### 12.3 南東平坦部へスポーン/カメラ移動

- [x] `VILLAGE_CENTER_COL/ROW` を (21, 10) に更新（川の東側、SE 象限、grass）
- [x] `crates/delphai-gdext/src/lib.rs::initialize` の住民3人 (20,10)/(21,10)/(22,10)、ベリー4 (Forest A+B)、水源3 (col=19)、鹿3 (各森 + NE grass) を再配置
- [x] `FOREST_B_COLS/ROWS` を [4,10]×[8,12] (SW) に移動して SE 村域を空ける
- [ ] `terrian.glb` を Godot でロードし南東平坦部が村中心と一致するか目視確認（scale/Y 微調整が必要なら 12.4）

#### 12.4 接地・視覚調整

- [ ] `GROUND_GLB_Y` / `GROUND_GLB_SCALE` を微調整 — 南東平坦部が Y≈0 でキャラ接地（目視確認後）
- [ ] カメラデフォルトズーム / Y 位置を再確認（必要なら再調整）

#### 検証

- `cargo test -p delphai-core`（165 passed 維持）
- `cargo clippy -p delphai-core -p delphai-gdext`（警告ゼロ）
- `make build`
- Godot 起動して目視:
  - 緑プレーン・茶色山・青水面・樹 fallback **すべて不可視**
  - GLB 地形 + 焚き火 + Oak 樹 + Plant 茂み + 鹿 + 住民のみ視覚
  - 南東平坦部中心に村・住民3人
  - 住民が川の shallow（col 17/19）を渡れ、deep（col=18）で止まる
  - キャラが平坦部に自然接地

### Sprint 12.5: 残存プリミティブ GLB 化（最優先・緊急バグ修正）

> ゴール: スクリーンショット (2026-04-19 13:39:36) で視認される「緑」「茶色」のプリミティブポリゴンを排除。
> 背景: Sprint 12.1 後も残る **チェスポーン市民 (茶)** と **水源シリンダー (青)** は `焚き火以外すべて GLB` ルール違反。
> 原則: **焚き火 (VillageBuilder) のみホワイトリスト**。それ以外のプリミティブメッシュは GLB 置換 or 削除。

#### 12.5.1 緑/茶プリミティブの発生源特定（完了済調査）

- [x] grep で `PlaneMesh` / `BoxMesh` / `CylinderMesh` / `SphereMesh` + `albedo_color` を全走査
- [x] 発見: (a) `CitizenFactory` 3 部位 × 3 人 = 9 プリミティブ (茶 0.85,0.65,0.35)、(b) `ResourceFactory._make_water_source` × 3 (青 0.1,0.45,0.95)、(c) `VillageBuilder` 石 8 + 燠 1 (焚き火ホワイトリスト)、(d) fallback (ベリー緑球・鹿茶球) — GLB ロード成功時は発火しない
- [x] スクショ解析: SE の茶色マス = 市民3人 + 石リング重畳、NW の緑 = Oak/Plant GLB の樹冠（GLB 由来で違反ではない）

#### 12.5.2 市民をチェスポーンから GLB プレースホルダへ

- [x] `CitizenFactory.make()` の base disc + body pillar + head sphere (3 × CylinderMesh/SphereMesh + 茶 StandardMaterial3D) を削除
- [x] 代替: `simple_nature_pack_glb.glb` の `Tree_Stump_01` を `GlbLoader.load_subscene` でロードし `BODY_SCALE = 0.6` で配置（人間の GLB が入手できるまでの暫定プレースホルダ）
- [x] Label3D（name / behavior / chat）は維持、高さを Tree_Stump のトップに合わせて `LABEL_Y_NAME/BEH/CHAT = 1.15/1.40/1.65` に調整
- [x] `_update_citizens()` の色替えは `_tint_citizen_body(idx, fed, hyd)` ヘルパで GLB 子の `MeshInstance3D` 全 surface の `albedo_color` を更新（`Node3D.modulate` は Godot 4 に無い → `GlbLoader.prepare_fade_materials` 経由でマテリアルを取得しキャッシュ）
- [x] 失敗時: GLB ロード不能なら body=null（Label3D のみ表示 = ゴースト市民）。fallback プリミティブは作らない
- [x] `game/assets/inventry.md` 更新: `Tree_Stump_01` の Phase 1 用途（市民プレースホルダ）を記載

#### 12.5.3 水源を GLB で表現（primitive Cylinder 廃止）

- [x] 選択肢: `Rock_01` を 3 つ円周配置（小さな泉の縁石っぽく）を採用
- [x] 実装: `ResourceFactory._make_water_source()` の `CylinderMesh` + 青 StandardMaterial3D を削除、`GlbLoader.load_subscene(NATURE_GLB, "Rock_01")` を 3 回ロードして `WATER_ROCK_RADIUS = 0.35` / `WATER_ROCK_SCALE = 0.28` で円形配置
- [x] `qty` スケール用の「枯渇で縮小」挙動は外側ラッパ `Node3D` のスケールで継続（既存契約維持）
- [x] GLB ロード失敗時は**非表示アンカー**（fallback なし、continue で Rock インスタンスのみスキップ）
- [x] `game/assets/inventry.md` 更新: `Rock_01` の Phase 1 用途（水源プレースホルダ）を記載

#### 12.5.4 検証

- [ ] `grep -rn "albedo_color" game/scripts/` で残存 StandardMaterial3D 色定数を確認、期待値は:
  - `village_builder.gd` の石色 + 燠色（焚き火ホワイトリスト）
  - `world.gd` の `beh_lbl.modulate` / `_behavior_color` (Label3D、ポリゴンではない)
  - `debug_hud.gd` の UI バー色（HUD、3D シーンポリゴンではない）
  - `glb_loader.gd` の `tween.tween_property` `albedo_color:a`（GLB 由来マテリアルのアルファ、色替えではない）
- [ ] `cargo test -p delphai-core`（165 passed 維持 — Rust 側は変更なし）
- [ ] `cargo clippy -p delphai-core -p delphai-gdext`（警告ゼロ維持）
- [ ] `make build`（.so 再ビルド不要だが確認）
- [ ] Godot 起動し目視:
  - SE 村中心: 石リング + 燠（焚き火）+ 市民3人が Tree_Stump で表現、チェスポーンの茶色塊が消滅
  - 水源3: 石配置の小さなマーカー、青シリンダー消滅
  - 緑ポリゴンは Oak/Plant GLB のみ（NW 森林）

### Sprint 13: 自由軸感移動 + [optional] 川速度低下（Sprint 12 + 12.5 完了後）

- [ ] 自由軸感 — 方針A（視覚 lerp 維持 + タイル内ランダム微オフセット）
- [ ] [stretch from 12.2] T_SHALLOW 移動速度半減

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

- LLM 再統合 — Phase 2 タスクに含まれる
- `crates/delphai-gdext/src/lib.rs` の `#[func]` 定型コード削減 — マクロで圧縮可能だが `#[func]` 制約で大幅整理は難しい
- `MAP_W`/`MAP_H` の Rust/GDScript 共有化 — FFI or codegen 導入まで手動同期

---

## 完了履歴

git ログ参照。主要マイルストーン:

- Phase 0 技術検証（Gemma4 E2B YAML 採用、Go/No-Go GO）
- Sprint 1–6: Rust コア + Godot 3D + 資源/採取 + カメラ + 技術ツリー + 人口増加
- Sprint 7: 会話バブル、市民散開、協力狩猟、枯渇非表示
- Sprint 8: 鹿の逃走 + フェードアウト + 遠方リスポーン
- Sprint 9: 地形システム、スムーズ移動、チェスポーン市民
- 大規模リファクタ (Opus 4.7): `world.rs::tick` phase-split、`world.gd` を `game/scripts/*.gd` へ分割、重複ヘルパー抽出、地形マジックナンバー定数化、clippy 警告ゼロ化
