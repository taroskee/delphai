# 教訓記録 (Lessons Learned)

> ミスが発生するたびに本ファイルを更新する
> 基準: 「新しいチームメンバーが初日に間違えること」だけ記録

---

## [2026-04] [macOS] unsigned dylib で Godot が Code Signature Invalid クラッシュ

- 状況: `make build-mac` 後に Godot を起動すると即座にクラッシュ。`EXC_BAD_ACCESS (SIGKILL - Code Signature Invalid)`
- 原因: macOS 26.4 から dyld が unsigned dylib のロードを拒否するようになった
- 対策: `Makefile` の `build-mac` / `build-mac-release` に `codesign --force --sign -` を追加
- **新メンバーへの指示**: Mac では `make build-mac` を実行（devcontainer 外）。CI が署名する

## [2026-04] [Godot3D] TILE_SIZE=1.0 だと「世界が小さく見える」で没入感 0

- 状況: 24×14 タイルが画面中央に小さく表示されて、5分眺めていられない
- 原因: TILE_SIZE=1.0 はズーム距離に比べて世界スケールが小さすぎた
- 対策: TILE_SIZE を 2.0 に倍にする。カメラ高さ・ZOOM・スクロール・パンをすべて比例スケール
- **新メンバーへの指示**: TILE_SIZE を変えたら `world.gd` の CAM_* / camera height も同倍率で調整（ゲームロジックは変更不要）

## [2026-04] [LLM] JSON 末尾カンマで parse 失敗 → YAML で解消

- 状況: LLM が JSON を出力するときに末尾カンマ・コメントを混入、parse 失敗率 66%
- 原因: LLM の JSON 生成は本質的に不正 JSON を出力しやすい
- 対策: プロンプト出力フォーマットはデフォルト YAML。JSON は使わない
- **新メンバーへの指示**: LLM の出力は YAML で指定する（30% 速い、parse エラーなし）

## [2026-04] [Godot3D] Terrain3D enum 名は推測せず addon バイナリで確認

- 状況: Sprint 13.1 で `Terrain3DCollision.DYNAMIC` と書いて Godot Parser Error。readthedocs は 403
- 原因: Terrain3D v1.0.1 の enum は `DISABLED / DYNAMIC_GAME / DYNAMIC_EDITOR / FULL_GAME / FULL_EDITOR`。`DYNAMIC` 単体は存在しない
- 対策: addon バイナリ (`game/addons/terrain_3d/bin/libterrain.*.so`) の strings で enum 値を直接確認 → ランタイムは `DYNAMIC_GAME`
- **新メンバーへの指示**: Terrain3D の enum / メソッド名は推測せず、`strings <so>` で確認するか `game/demo/CodeGeneratedDemo.tscn` を参照する

## [2026-04] [Godot3D] Terrain3D 固定 seed + force-flat オーバーレイで村を平坦化

- 状況: Sprint 13.1 で FastNoiseLite ベースの ProcGen を導入した際、村 SE (21,10) が高低差の中腹に落ち、住民/焚き火が斜面に浮く or 沈む危険があった
- 候補案: (a) 平坦な seed を探索する (b) 高度マップに force-flat 円盤を重ねる
- 採用: (b)。理由は決定的・MVP 再現性・コード量が少ない。`VILLAGE_FLAT_RADIUS=10m` 内は 0m に強制、`FADE_RADIUS=15m` まで noise と線形補間。seed=42 固定は「どの seed でも村は必ず平坦」を保証する
- **新メンバーへの指示**: ProcGen に「特定座標を平坦にしたい」要件が出たら seed 選定より force-flat オーバーレイを先に検討する（Sprint 13.1 の `_generate_heightmap` を参考）

## [2026-04] [Rust] memory_summary の無制限 append でプロンプトが肥大化

- 状況: 会話ログで 887 → 1884 → 3357 文字と肥大化、LLM 品質低下
- 原因: `apply_response` と `record_heard_speech` が制限なく append し続けた
- 対策: `append_memory()` ヘルパーで最新 N エントリに制限。初期値は 8
- **新メンバーへの指示**: 新しい記憶追加箇所には必ず上限を設ける

---

## 初日に踏みやすい地雷（新メンバー向け）

### コード構造

- **`game/scenes/world.gd` は薄いオーケストレータ** — 構築ロジックは `game/scripts/*.gd` に分離。新しい責務を足すときは `world.gd` に戻さず、新スクリプトか既存ヘルパーに追加する
- **`MAP_W`/`MAP_H` は Rust と GDScript で重複定義** — FFI 生成は未導入。片方変えたら**必ず両方同期**
- **地形の通行可否は `TerrainBuilder.make_walkable_map` → `World.set_walkable_map`** — T_DEEP と T_MOUNTAIN を `0` で送る。新地形追加時は両方を更新
- **`World::tick()` は phase-split 済み** — `tick_decay → tick_resources → ... → maybe_spawn_citizen`。順序変更前に各 `tick_*` 本文を読むこと

### ファイル管理

- **`crates/delphai-core/src/llm/` は Phase 2 用に温存** — 未使用に見えても削除しない（LLM 再統合のため）

### 検証の順番（コミット前に3つ必ず）

1. `cargo test -p delphai-core`（165 passed 維持）
2. `cargo clippy -p delphai-core -p delphai-gdext`（警告ゼロ）
3. `make build`（.so リビルド）
4. Godot 起動確認（UI/3D 変更時）

---

## 削除済みのアイテム

- `delphai-bench` の lib.rs — bench クレートは `src/lib.rs` を置かない（benches/ 直下のコードで十分）
- チェスポーン市民・プリミティブ水源・緑PlaneMesh — Sprint 12.5 で GLB 化完了
