# Phase 0 検証手順書

> 目的: LLMがゲームに使えるか判定する（Go/No-Go）
> 合格基準: **応答3秒以内** かつ **日本語品質がゲームに使える**

---

## 前提

- devcontainerにはGPU/Ollama/Godotなし → **本機（ローカルPC）で実施**
- Rust側 `Player2Provider` は実装・テスト済み
- ベンチマーク基盤 (`cargo bench --bench llm_throughput`) セットアップ済み

---

## 推奨順序

```
Step 1 (Ollama比較) → Step 2 (Player2) → Step 3 (NobodyWho) → Step 4 (バッチ) → Step 5 (判定)
```

理由: ローカルLLMの品質がわかれば、Player2と比較する基準ができる。
NobodyWhoはベストモデル決定後に検証した方が効率的。

---

## Step 1: ローカルLLM候補モデル比較

**場所:** 本機（GPU搭載PC推奨）

### 1a. Ollama インストール & 実行

```bash
# macOS
brew install ollama
ollama serve  # 別タブで起動しておく
```

### 1b. 評価スクリプト

```bash
python3 tasks/phase0/eval_models.py
```

モデル未インストールなら自動pull。3回ずつ実行し、JSON成功率・平均応答時間を自動集計。
ログは `tasks/phase0/logs/` に保存される。

### 1c. 評価基準

各モデルを3回ずつ実行し記録:

| モデル | 平均応答(ms) | JSON成功率 (3/3) | 日本語自然さ (1-5) | キャラ維持 (1-5) |
|---|---|---|---|---|
| qwen3:1.7b | | | | |
| gemma2:2b | | | | |
| phi3:mini | | | | |

**重みづけ:** JSON安定性(40%) > 速度(30%) > 日本語品質(20%) > キャラ維持(10%)

---

## Step 2: Player2 API — GDScript応答速度計測

**場所:** 本機Godot

```gdscript
extends Node

var http := HTTPRequest.new()

func _ready():
    add_child(http)
    http.request_completed.connect(_on_response)
    var body := JSON.stringify({
        "model": "player2-default",
        "messages": [{"role": "user", "content": "あなたは石器時代の集落の狩人です。空腹で疲れています。道具は石斧だけです。次にどうしますか？JSON形式で答えてください。"}],
        "response_format": {"type": "json_object"}
    })
    var headers := ["Content-Type: application/json"]
    var start := Time.get_ticks_msec()
    http.request("https://api.player2.ai/v1/chat/completions", headers, HTTPClient.METHOD_POST, body)
    set_meta("start", start)

func _on_response(result, code, headers, body):
    var elapsed := Time.get_ticks_msec() - get_meta("start")
    var json := JSON.parse_string(body.get_string_from_utf8())
    print("=== Player2 応答 (%dms) ===" % elapsed)
    print(json)
```

**記録:**
| 項目 | 値 |
|---|---|
| 応答時間 (ms) | |
| JSON形式で返ったか | |
| 日本語の自然さ (1-5) | |

---

## Step 3: NobodyWho 検証

**場所:** 本機Godot

1. https://github.com/nobodywho-ooo/NobodyWho からプラグイン取得
2. Step 1で選んだベストモデルのGGUFファイルを使って動作確認

**確認項目:**
- [ ] Godotエディタ内でモデルロードできるか
- [ ] GDScriptからプロンプト送信→応答取得できるか
- [ ] 応答速度はOllama直叩きと同等か
- [ ] メモリ使用量（タスクマネージャで確認）

**判定:**
- 動く → Godot内蔵LLMとして採用（Ollama不要でUX良い）
- 不安定/遅い → Ollama経由 + Rust HTTP実装で進める

---

## Step 4: バッチ推論テスト

Step 3の結果に応じてOllamaまたはNobodyWhoで実施。

1プロンプトに複数住民ペアを詰めて生成できるか確認:

```
あなたはゲームマスターです。以下の住民ペアの会話をJSON配列で生成してください。

--- ペア1 ---
タカシ(好奇心旺盛な若者, 空腹) と ユキ(慎重な長老, 疲労)。石器時代の集落、焚き火の前。

--- ペア2 ---
ケンジ(勇敢な狩人, 元気) と タカシ(好奇心旺盛な若者, 空腹)。川辺で石斧を研いでいる。
```

**記録:**
| batch_size | 応答時間 | JSON配列で返ったか | 個別推論との速度比 |
|---|---|---|---|
| 2ペア | | | |
| 3ペア | | | |

**合格:** batch_size=3で個別3回より速い + JSON配列が安定

---

## Step 5: Go/No-Go 判定

### 必須条件（1つでもNoなら No-Go）
- [ ] 応答が3秒以内（ローカルLLM or Player2）
- [ ] JSON形式で安定して返る（成功率 >= 80%）
- [ ] 日本語がゲームに使える（自然さ >= 3/5）

### 推奨条件（No-Goにはしないが改善計画を立てる）
- [ ] バッチ推論が個別より効率的
- [ ] NobodyWhoが安定動作
- [ ] キャラクター性が維持される

### 判定
- **Go:** → Phase 1 UI実装に進む
- **Conditional Go:** → 必須OK・推奨一部NG → 対策を決めてPhase 1へ
- **No-Go:** → モデル変更/アーキテクチャ再検討
