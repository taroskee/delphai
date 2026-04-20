# 教訓記録 (Lessons Learned)

> ミスが発生するたびに本ファイルを更新する
> 基準: 「新しいチームメンバーが初日に間違えること」だけ記録
> フォーマット: 各エントリは **タイトル(WHAT) / 状況(WHEN) / 原因(WHY) / 新メンバーへの指示(HOW)** の 4 フィールド。超えるなら分割する

---

## [2026-04] [macOS] unsigned dylib で Godot が Code Signature Invalid クラッシュ

- **状況**: `make build-mac` 後に Godot 起動で `EXC_BAD_ACCESS (SIGKILL - Code Signature Invalid)`
- **原因**: macOS 26.4 の dyld が unsigned dylib のロードを拒否する
- **新メンバーへ**: Mac では devcontainer 外で `make build-mac` を使う（`codesign --force --sign -` 済み）。CI も署名する

## [2026-04] [Godot3D] TILE_SIZE=1.0 だと世界が小さく没入感ゼロ

- **状況**: 24×14 タイルが画面中央に小さく表示され、5 分眺めていられない
- **原因**: TILE_SIZE がズーム距離に比べて小さすぎ
- **新メンバーへ**: TILE_SIZE を変えたら `world.gd` の CAM_* / camera height も同倍率でスケール（ゲームロジックは不変）

## [2026-04] [LLM] JSON 末尾カンマで parse 失敗 → YAML で解消

- **状況**: LLM が JSON 出力で末尾カンマ・コメントを混入、parse 失敗率 66%
- **原因**: LLM の JSON 生成は本質的に不正 JSON を吐きやすい
- **新メンバーへ**: プロンプト出力は YAML 固定。JSON は使うな（30% 速い、parse エラーなし）

## [2026-04] [Godot3D] Terrain3D の enum / メソッド名を推測して Parser Error

- **状況**: `Terrain3DCollision.DYNAMIC` と書いて Godot Parser Error。readthedocs は 403
- **原因**: 実際の enum は `DYNAMIC_GAME / DYNAMIC_EDITOR / FULL_GAME / FULL_EDITOR`。推測が外れた
- **新メンバーへ**: Terrain3D の API は `strings game/addons/terrain_3d/bin/libterrain.*.so` か `game/demo/CodeGeneratedDemo.tscn` で確認してから書く

## [2026-04] [Godot3D] 特定座標を平坦化したい時は seed 選定より force-flat

- **状況**: ProcGen 導入で村 SE (21,10) が斜面に落ち、住民/焚き火が浮く or 沈むリスク
- **原因**: seed 探索は決定性に欠け、MVP 再現性を壊す
- **新メンバーへ**: 高度マップに force-flat 円盤を重ねる（`VILLAGE_FLAT_RADIUS=10m` 内は 0m 固定、`FADE_RADIUS=15m` まで線形補間）。`_generate_heightmap` を参照

## [2026-04] [AI運用] 計画の checkbox を実装前に [x] にして完了扱い

- **状況**: TODO を新規作成したターン内で、同じ応答内に全 checkbox を `[x]` に変えた。実 diff は 0 バイト
- **原因**: 計画記述と実装を同一ターンで処理する癖。AI は文章生成で完了感を得るが、ユーザーは diff でしか検証しない
- **新メンバーへ**: 計画フェーズの checkbox は必ず `[ ]`。該当ファイルの Edit/Write 直後のターンでしか `[x]` に変えない。GDScript/Godot 変更は「Godot 起動目視」の `[ ]` が残る限り前段の `[x]` も信用するな

## [2026-04] [AI運用] GDScript 挙動を cargo test の GREEN だけで完了宣言

- **状況**: Sprint 13.R5 で `tick-fraction 補間` / `_load_texture_asset` を書き、`cargo test` 165 passed で完了扱い。ユーザー目視で「反映されてない」が発覚
- **原因**: GDScript ランタイム挙動は Rust テストでは捕捉不能。テクスチャも `Terrain3DMaterial.set_shader_param` が silent-fail するため GREEN でも動かない
- **新メンバーへ**: GDScript/Terrain3D 変更には `godot --headless --script` の smoke test を必ず先に RED で作る。ランタイム挙動は Rust 側 FFI（例: `World::world_pos_at(idx, alpha)`）に寄せて cargo test でガードする

## [2026-04] [Rust] memory_summary の無制限 append でプロンプト肥大化

- **状況**: 会話ログが 887 → 1884 → 3357 文字と肥大化、LLM 品質が低下
- **原因**: `apply_response` と `record_heard_speech` が制限なく append し続けた
- **新メンバーへ**: 新しい記憶追加箇所には必ず上限を設ける。`append_memory()` ヘルパーで最新 N エントリに制限（初期値 8）

---

## 初日に踏みやすい地雷（新メンバー向け）

### コード構造

- **`game/scenes/world.gd` は薄いオーケストレータ** — 構築ロジックは `game/scripts/*.gd` に分離。新責務は新スクリプトか既存ヘルパーへ
- **`MAP_W`/`MAP_H` は Rust と GDScript で重複定義** — FFI 生成は未導入。片方変えたら**両方同期**
- **地形の通行可否は `TerrainBuilder.make_walkable_map` → `World.set_walkable_map`** — T_DEEP と T_MOUNTAIN を `0` で送る
- **`World::tick()` は phase-split 済み** — `tick_decay → tick_resources → ... → maybe_spawn_citizen`。順序変更前に各 `tick_*` 本文を読む

### ファイル管理

- **`crates/delphai-core/src/llm/` は Phase 2 用に温存** — 未使用に見えても削除しない

### 検証の順番（コミット前に必ず）

1. `cargo test -p delphai-core`（165 passed 維持）
2. `cargo clippy -p delphai-core -p delphai-gdext`（警告ゼロ）
3. `make build`（.so リビルド）
4. Godot 起動確認（UI/3D 変更時）

---

## 削除済みのアイテム

- `delphai-bench` の lib.rs — bench クレートは `src/lib.rs` を置かない（benches/ 直下で十分）
- チェスポーン市民・プリミティブ水源・緑 PlaneMesh — Sprint 12.5 で GLB 化完了
