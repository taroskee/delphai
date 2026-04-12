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

### [2026-04] [LLM] gemma2:2b は `action: null` を出力しプロンプトで修正不可

- 状況: gemma2:2b が structured output で `action` フィールドを常に null で返す
- 原因: モデルの世代が古く、JSON Schema 強制でも指示無視が発生する
- 対策: LLMモデル選定時は必ず structured output の null 返し率を計測してから採用判断する。世代が古いモデルは早期除外
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

### [2026-04] [Rust] 会話ペア選出がリスト先頭固定になり特定住民が会話独占

- 状況: Kael と Elder が常にペアになり Hara が会話に参加しない
- 原因: idle 市民リストを順番に走査するため、最初の近接ペア（インデックス 0-1）が毎回当選する
- 対策: `random_roll` でリストをローテーションしてから走査する。リスト順に依存するペア選出を書く場合は必ずランダム性を注入する
- CLAUDE.md反映: 未
