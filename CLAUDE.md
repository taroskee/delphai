# CLAUDE.md

> 書くのは「新しいチームメンバーが初日に間違えること」だけ
> AIが既に知っていることは書かない

## 言語

入出力、ドキュメントは日本語
Thinking, coding and git-commiting in English

## 判断の優先順位 (迷ったらここに戻る)

1. **検証なき完了なし** - 動くことを証明してコミット
2. **不具合はテストで再現してから直す** - テストなき修正は技術的負債
3. **計画に合意するまで実装しない** - 進展ないなら手を止めて再計画
4. **副作用を排除** - 可変型より不変型、継承より移譲
5. **不確実*選択の自由=価値** - SOLID、抽象クラスよりプロトコル、ただしKISS/YAGNI

## AIが間違えやすいこと

- CI落ちやバグ報告に対して人間に聞くな - 「DEBUG-FATALレベルでログを設定し、ログ/エラー/テストを見てAIで解決しろ」
- ハック的な修正で済ませる - 「今知っていることを全て踏まえ、エレガントに実装し直せ」

## 検証コマンド

以下、検証の順番3つ

1. 型検査
2. テスト
3. lint

検証完了後は、Gitコミット、プッシュ(AuthorにClaude Codeは記述しない)

## TODO

Plan Modeの計画は`@tasks/todo.md`に簡潔に記述
完了したTODOは必ずチェックをつける

## 自己改善

AIがミスしたら `tasks/lessons.md` に記録し、本ファイルの更新を提案する

## 参照 (必要な時だけ読め)

CI/CD含:`@docs/testing.md`|`@docs/architecture.md`|セキュリティ`@docs/secure-by-design.md`|戦略と意思決定:`@docs/wardley-map.md`|CLAUDE.md運用:`@docs/context-engineering.md`|教訓:`@tasks/lessons.md`|現タスク:`@tasks/todo.md`
