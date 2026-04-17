# 教訓記録 (Lessons Learned)

> ミスが発生するたびに本ファイルを更新する
> 効果のないルールは消す

## 記録フォーマット

各エントリは以下の形式で追記する:

```
### [日付] [カテゴリ] 簡潔なタイトル
- 状況: 何が起きたか
- 原因: なぜ起きたか
- 対策: 今後どう防ぐか
- CLAUDE.md反映: 済/未
```

---

### [2026-04] [Rust] delphai-bench が `[dev-dependencies]` 依存で lib.rs をコンパイルできない

- 状況: `delphai-bench/src/lib.rs` がビルドエラー。`delphai-core` が dev-dep 扱いで本番ビルドに含まれない
- 原因: bench クレートの `src/lib.rs` は benches/ のコードと完全重複しており、ライブラリとして切り出す意味がなかった
- 対策: bench クレートには `src/lib.rs` を置かない。bench 共通コードは `benches/` 直下に置く
- CLAUDE.md反映: 未

### [2026-04] [LLM] JSON 末尾カンマで parse 失敗 → YAML で解消

- 状況: crisis シナリオで E2B が JSON 末尾カンマを出力し 1/3 しか適合しない
- 原因: LLM の JSON 生成は末尾カンマ・コメント混入などの不正 JSON を出力しやすい
- 対策: LLM の出力フォーマットはデフォルト YAML。JSON は使わない。YAML は 30% 速く 32% トークンが少なく parse エラーも出ない
- CLAUDE.md反映: 未

### [2026-04] [Godot/プロンプト] `[Divine Voice]` タグだけでは「テーマ」として解釈される

- 状況: プレイヤーの声を届けると「炎のゆらめきは...」という詩的雑談になる
- 原因: `[Divine Voice]: text` の形式では LLM がそれを会話テーマや情景描写のインスピレーションとして解釈する
- 対策: `[SUPERNATURAL EVENT]` ブロックで「今この瞬間 {name} が不思議な声を聞いた体験」として明示する
- CLAUDE.md反映: 未

### [2026-04] [Rust] memory_summary の無制限 append でプロンプトが肥大化

- 状況: 会話ログで 887文字 → 1884文字 → 3357文字 と急増し、LLM の品質が低下
- 原因: `apply_response` と `record_heard_speech` が制限なく memory_summary に append し続けた
- 対策: `append_memory()` ヘルパーで最新 N エントリに制限する。初期値は 8。新しい記憶追加箇所を実装するたびに必ず上限を設ける
- CLAUDE.md反映: 未

### [2026-04] [Godot3D] TILE_SIZE=1.0 だとカメラ高さ18でマップが画面に対して小さく見える

- 状況: 24×14 タイルのマップが画面中央に小さく表示され、世界感が出ない
- 原因: TILE_SIZE=1.0（1ユニット/タイル）に対してカメラ高さ18はズームアウトしすぎ。逆にズームインするとパン操作が重くなる
- 対策: TILE_SIZE を倍（2.0）にする。カメラ高さ・ZOOM範囲・スクロールステップ・パンスケール係数をすべて2倍に合わせる。ゲームロジック（Rust側タイル座標）は変更不要
- CLAUDE.md反映: 未

### [2026-04] [macOS] unsigned dylib で Godot が Code Signature Invalid クラッシュ

- 状況: `make build-mac` 後に Godot を起動すると即座にクラッシュ。`EXC_BAD_ACCESS (SIGKILL - Code Signature Invalid)` / `Termination: CODESIGNING Code 2`
- 原因: macOS 26.4 (Tahoe) から dyld が unsigned な dylib のロードを拒否するようになった。`cargo build` は署名なし dylib を生成する
- 対策: `Makefile` の `build-mac` / `build-mac-release` ターゲットに `codesign --force --sign -` を追加してアドホック署名する。Apple Developer cert 不要でローカル開発では十分
- CLAUDE.md反映: 未