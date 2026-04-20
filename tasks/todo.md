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

### Sprint 13: 地形システム最適化（procgen パラメータチューニング）

13.1/13.2 の失敗分析を踏まえ、procgen parameters (TERRAIN_NOISE_FREQ, TERRAIN_HEIGHT_SCALE, auto_slope) を調整中。視認性 improvement を Godot で検証予定。

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

**Sprint 13: 地形システム根本再設計**

13.1-13.2 の失敗分析と改善: terrian.glb（装飾 GLB、物理なし）を Terrain3D ProcGen（FastNoiseLite + Godot auto-shader）に置換。force-flat village overlay で村中心を平坦化し、tile classification キャッシュで movement/collision logic の O(1) 判定を実現。

**13.1 実装**: Terrain3D 統合、FastNoiseLite noise (seed=42, freq=0.05, height_scale=5.0)、village force-flat overlay (radius=10m, fade=15m)、tile category cache (classification: GRASS/SHALLOW/DEEP/UNKNOWN)。

**13.2 失敗分析**: auto-shader blend（slope-based grass↔dirt）が inactive のまま。理由: procgen slope が auto_slope=10 threshold 以下で、ほぼ全タイルが GRASS に留まる。freq=0.05 の 20m 波長は地形を緩すぎるコースに→視認性の改善なし。

**13.3 改善案 + 市民 UI**: procgen パラメータ調整（freq: 0.05→0.08, height_scale: 5.0→8.0, auto_slope: 10→5）でより急峻な ridge を生成し、auto-shader activation margin を広げる。併せて Tree_Stump_01 GLB（scenery に紛れて読みにくい）を Godot chess pawn primitive に戻し（cylinder 3-part: base+body+head）、市民の視覚的区別性を復帰。
