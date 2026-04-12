# DelphAI

原始時代から現代まで、LLMで駆動する住民たちの文明進化を見守るシミュレーションゲーム。

プレイヤーは「神」として声を届けるが、住民が従うかは彼ら次第。
戦争は愚かであるというメッセージを届けたい。

DLC: 住民を細胞、世界を人体にした版。

---

## 開発環境セットアップ

### Godot MCP（Claude CodeからGodotを操作する）

ローカルのGodot GUIをdevcontainer内のClaude Codeから操作するための設定。

**host側（Macターミナル）:**
```bash
# supergatewayでgodot-mcpをSSEサーバとして公開
npx supergateway --stdio "npx @coding-solo/godot-mcp" --port 3001
```

**devcontainer側（初回のみ）:**
```bash
# stdioからSSEに切り替え（setup.shで自動設定済みの場合は不要）
claude mcp remove godot
claude mcp add --transport sse godot http://host.docker.internal:3001/sse
```

その後Claude Codeを再起動すると `mcp__godot__*` ツールが使えるようになる。

Godotプロジェクトのパスはhost側のパスで指定する（例: `/Users/machina/Documents/delphai/game`）。

---

## よく間違えること

### プレイヤーにできることは3つだけ

1. プロンプトを入力する（声を届ける）
2. 住民の声を聴く
3. 世界の様子を確認する

それ以外の操作を足さない。住民を直接動かせない。建物を建てられない。プレイヤーは「声」だけの存在。

### 住民は最初プレイヤーの声が聞こえない

認知度、信頼度0%からスタートする。声を届けてもノイズとして無視される。
これをスキップするチュートリアルを作らない。この「伝わらなさ」がゲーム体験の核。

### LLMは住民全員に毎ターン走らせない

RCTの最大の教訓：**ゲームデザインで計算量を回避する。** RCTはゲストの経路探索を「目的地を選んでから歩く」ではなく「分岐点でランダムに方向を選ぶ」設計に変えて、パスファインディングの爆発を消した。

同じ原則をLLMに適用する：

- 各ターンでLLM推論するのは住民の5-10%だけ。残りはルールベースAI
- 住民は普段は黙々と働いている。会話が発生した時だけLLM
- 「住民100人の大会議」はやらない。「代表者5人の会議 + 群衆はルールベース」

### 1回のLLMコールで複数住民をシミュレートする

住民1人に1回のAPIコール、ではない。入力トークン数の限界まで詰め込んで、1コールで複数住民の会話を同時に生成する。これがこのゲームのスループットを決定的に左右する。

注意点：
- 詰め込みすぎると出力品質が落ちる。ベンチマークで最適なバッチサイズを見つける
- モデルのコンテキストウィンドウ (例: 4096トークン) から出力分を引いた残りが入力の上限
- バッチ内の住民ペアは独立していること。ペアAの結果がペアBに影響する場合はバッチにできない

### LLMスループットをベンチマークで管理する

「なんとなく速くなった気がする」で最適化しない。指標を定義して計測する。

```
LLM効率スコア = (会話品質スコア × 会話数) / 処理時間(秒)
```

| 指標 | 計測方法 | 目標 |
|------|---------|------|
| 会話品質スコア (0-1) | JSON パース成功率 × キャラ一貫性 × 世界観逸脱なし | ≥ 0.7 |
| 会話数 / コール | 1回のLLMコールで生成された会話ペア数 | ≥ 3 |
| 処理時間 | LLMコール発行から全レスポンス受信まで | ≤ 5秒 |
| LLM効率スコア | 上記の複合指標 | ≥ 0.4 |

pytest-benchmark 的にCIで回す：

```bash
# ベンチマーク実行例
cargo bench --bench llm_throughput

# 出力例:
# batch_size=1  quality=0.85  convos=1   time=2.1s  score=0.40
# batch_size=3  quality=0.80  convos=3   time=3.4s  score=0.71  ← 最適
# batch_size=5  quality=0.65  convos=5   time=4.8s  score=0.68
# batch_size=8  quality=0.45  convos=8   time=6.2s  score=0.58  ← 品質劣化
```

このベンチマークで：
- モデル変更時に品質が落ちていないか検知する
- プロンプト変更時にバッチサイズの最適値がズレていないか確認する
- ハードウェアごとの推奨バッチサイズを決定する

### LLMプロバイダーは1つに縛らない

RimWorld LLM mod（RimTalk/EchoColony）の教訓。全て複数プロバイダー対応で成功している。

| 優先 | プロバイダー | 理由 |
|------|------------|------|
| 1st | Player2 | APIキー不要。プレイヤーはアプリを起動するだけ。セットアップの壁がない |
| 2nd | ローカルLLM (llama.cpp) | オフライン動作。買い切りの長期保証。Player2が死んでもゲームは動く |
| 3rd | クラウドAPI (OpenAI等) | ユーザーが自分でAPIキーを取得。上級者向け |

Player2をデフォルトにする。「Ollamaをインストールして、モデルをダウンロードして...」で20人中19人が脱落する（RimTalk開発者の実測値）。

### 技術ツリーはプロトコルで定義する

最初は技術ツリーだけ実装する。ただしハードコードしない。

```toml
# tree_protocol.toml の例

[meta]
tree_type = "tech"  # "tech" | "social" | "culture" | ...
version = 1

[[nodes]]
id = "stone_tools"
name = "石器"
era = "primitive"
prerequisites = []
keywords = ["石", "割る", "尖らせる", "道具"]

[[nodes]]
id = "fire"
name = "火の利用"
era = "primitive"
prerequisites = ["stone_tools"]
keywords = ["火", "燃える", "暖かい", "焼く"]

[[nodes]]
id = "agriculture"
name = "農業"
era = "agricultural"
prerequisites = ["stone_tools"]
keywords = ["種", "植える", "育てる", "畑", "収穫"]
```

このプロトコルに従えば社会ツリーも文化ツリーも同じコードで動く。ただしMVPでは技術ツリーだけ。「社会ツリーも欲しい」と思っても、まず技術ツリーを完成させてから。

### 住民の記憶は全会話履歴ではない

コンテキストウィンドウは有限。全部入れると破綻する。

- 住民の個性: タグ5-10個（`勇敢`, `懐疑的`, `農民`）
- 記憶: 圧縮サマリー100-200トークン。古い記憶はLLMで要約して圧縮
- 関係性: `{相手の名前: 関係性}` の短いリスト

LLMへの入力はできる限り短くする。

### LLMの出力はJSON

自由形式のテキストを返させない。パースできなくなる。

```json
{
  "speech": "あの山の向こうに何があるか、見に行こう",
  "inner_thought": "長老は反対するだろうな...",
  "action": "propose_exploration",
  "emotion_change": "excited",
  "tech_hint": null
}
```

パース失敗時のフォールバックも必ず書く。LLMは必ず壊れたJSONを返す日が来る。

---

## アーキテクチャ

```
Godot 4.x ──GDExtension──▶ Rust (シミュレーション) ──HTTP──▶ LLM推論
(表示/UI)                   (エージェント/ツリー/キュー)        (Player2 / llama.cpp)
```

Godot: MIT、2Dに強い、軽い。Unityはライセンスリスク。

Rust (gdext): エージェントの並行処理とLLM推論キューの非同期管理にGDScriptでは足りない。

LLM: 1B-4Bの量子化モデル (Q4_K_M)。7Bは重すぎる。

---

## 参考

| 作品 | 何を学ぶか |
|------|-----------|
| Civilization | 技術ツリーの設計 |
| RimWorld + RimTalk | LLM NPC会話の実装パターン、Player2統合 |
| RollerCoaster Tycoon | デザインレベルでの最適化。「重い機能は設計で回避する」哲学 |
| 0 A.D. | 古代戦争シミュレーションのAI設計（Petra AI）、市民兵の経済/軍事の二面性 |
| openage | AoE2エンジンのリバースエンジニアリングから得られた最適化知見、nyanデータ記述言語 |
