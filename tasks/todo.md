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

#### 検証

- `cargo test -p delphai-core`（165 passed 維持）
- `cargo clippy -p delphai-core -p delphai-gdext`（警告ゼロ）
- `make build`
- Godot 起動して目視: 村中心が画面中央／住民・鹿・資源が見える／マウスホイール・右ドラッグ・トラックパッドピンチ・WASD すべて機能

### Sprint 12: 森林山 + 自由軸感移動（Sprint 11 完了後）

> 判定: 山の一部が森化して見え、住民の移動が格子に縛られて見えない。

- [ ] 山に木を生やす — `TerrainBuilder.build_features` の T_MOUNTAIN タイルに `_add_tree` をハッシュ密度で追加（山+森の複合表現）
- [ ] 自由軸感 — まず方針A（視覚 lerp 維持 + タイル内ランダム微オフセット）。方針B（`MoveState`/`World.tick` を連続座標化、pathfinding を steering へ）は必要になってから。

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
