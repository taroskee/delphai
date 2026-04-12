# MVP TODO

MVPの定義: 住民5-10人がLLMで会話し、プレイヤーの「声」に反応し、技術ツリーが3段階進行するデモ。

---

## 既知の問題・インフラ

- [x] **macOS dylib ビルド**: Godot は macOS 上で動くが、devcontainer 内では `.so`（Linux）しかビルドできない。
  - 解決: `prebuilt/macos/` に CI (GitHub Actions) がビルドしてコミット。
  - ローカル: Mac ターミナル（devcontainer 外）で `make build-mac` を一度だけ実行。
  - 参照: `.github/workflows/build-libs.yml`, `Makefile`, `game/delphai.gdextension`

---

## Phase 0: 技術検証 ✅

- [x] Godot 4.4+ / Rust (gdext) Hello World — `delphai-gdext`(cdylib) / `delphai-core`(lib) 分離済み
- [x] LLMベンチマーク基盤 (`cargo bench --bench llm_throughput`)
- [x] モデル選定
  - ~~Qwen3.5 2B/0.8B~~ — 詩的だが支離滅裂、Gemmaに集中するため除外
  - **Gemma4 E2B** — primary（ルーティン会話）avg 1248ms YAML
  - **Gemma4 E4B** — 重要シーン候補。avg 3715-8133ms → Phase 2 住民増加時に限定運用
- [x] JSON/YAML比較 — YAML: 30%速く(1248ms vs 1792ms)、32%少ないトークン(70 vs 103) → **YAML採用**
- [x] Crisis評価 — E2B: avg 2.3/3適合(YAML切替で安定)。E4B: 品質高いが遅すぎ
- [x] NobodyWho — **スキップ**: Ollama 1.2s安定動作中、埋め込みの複雑度不要
- [x] Player2 GDScript計測 — **Phase 3 延期**: APIキー未取得
- [x] バッチ推論実計測 — **Phase 2 延期**: 住民3人では不要。スタブでカーブのみ確認済み
- [x] **Go/No-Go: GO ✅** — E2B YAML 1248ms < 3秒目標、焚き火デモ実動作確認済み

---

## Phase 1: 焚き火デモ (3-4週間)

> 住民3人が焚き火を囲んで会話する。プレイヤーが声を届けると反応する。
> これが面白くなければプロジェクトを止める。

### Rust ✅

- [x] `Citizen` 構造体 (name, personality_tags, memory_summary, emotion, relationships, divine_awareness)
- [x] ルールベース行動ループ — `tick(state, needs) -> BehaviorAction`
- [x] 会話トリガー判定（近接 + ランダム確率、Idle同士のみ、1人1会話/tick）
- [x] `LlmProvider` トレイト + Player2実装
- [x] 優先度付き推論キュー（プレイヤー注目 > イベント > 定期）
- [x] プロンプトビルダー / バッチプロンプトビルダー
- [x] レスポンスパーサー (JSON/YAML切替可能、partial parse対応)
- [x] 記憶圧縮（64エントリ超で最古48件をLLM要約→1エントリ）
- [x] `World::tick()` / `apply_response()` / `record_heard_speech()`
- [x] 焚き火デモ修正: プレイヤーの声プロンプト、会話の自然化・短縮、memory上限(8エントリ)、会話ペアのローテーション、divine_awareness成長システム

### UI (Godot)

- [x] 2Dマップ + 住民スプライト — chara2_1/3_1/4_1、焚き火ループアニメ、夜空+地面背景
- [x] プロンプト入力フィールド + 「声を届ける」ボタン（ヒント付きplaceholder）
- [x] 会話バブル + ログウィンドウ（スプライト直上バブル + 画面下部ログ）
- [ ] **[次]** 住民クリックで詳細パネル（divine_awareness / emotion / 関係性）
- [ ] **[次]** CampAmbience.mp3 を焚き火シーンに追加（`assets/sfx/Ambience/CampAmbience.mp3`）
- [ ] **[次]** 神の声エフェクト: crystalball ループ（`assets/effect/other/crystalballV001effect-loop/`、16フレーム）をボイス送信時に一瞬表示

### シナリオ

- [ ] 住民3人: 好奇心旺盛な若者 / 慎重な長老 / 勇敢な狩人
- [x] 会話サイクル: ペア選出 → LLM生成 → 感情・関係性反映
- [x] プレイヤーの声: 認知度に応じた反応変化（shake→surprise→nod）
- [ ] **判定: 10分遊んで面白いか**

---

## アセット在庫（Phase 2以降で使う）

> `game/assets/` に含まれる未使用アセット。必要になった時に参照。

### キャラクター
| カテゴリ | バリアント数 | 感情フレーム | 用途候補 |
|---|---|---|---|
| `chara` (chara2〜5 × 8) | 32 | あり (shake/nod/surprise/laugh) | 主要住民・会話キャラ |
| `npc` (npc1〜4 × 8) | 32 | なし (walk/stand のみ) | 背景モブ、Phase 2 増加住民 |
| `military` (military1〜3 × 8) | 24 | なし | 戦士・紛争キャラ、Phase 2 戦争 |
| `bonus1` (× 8) | 8 | 要確認 | 特殊キャラ枠 |
| `animals` (cat1〜4, dog1〜4) | 8 | なし | ペット・環境演出 |

### エフェクト
| アセット | フレーム | 用途候補 |
|---|---|---|
| `effect/fires/loop/fireV001〜006` | 各10f | 焚き火バリエーション、松明、家の炉 |
| `effect/other/crystalballV001-002` | 各16f | 神の声ビジュアル、神託演出 |
| `effect/water/waterV001〜003` | 各数f | 水源発見エフェクト（デモシナリオ直結） |
| `effect/boosts-shields-energy/` | 各数f | 技術進歩・ブースト演出 |
| `effect/explosions/` | 8種 | 戦争・災害 |
| `tile/TILESETS/animated/torch` | tilesheet | 松明タイル、Phase 2 マップ |

### SFX
| アセット | 用途候補 |
|---|---|
| `sfx/Ambience/CampAmbience.mp3` | **即使用可** — 焚き火BGM |
| `sfx/Food & Herbs/` | 食料取得・fed パラメータ回復音 |
| `sfx/Coins/` | 資源取引、Phase 2 経済 |
| `sfx/Weapons/` | 戦争・戦闘、Phase 2 |

### タイルセット（Phase 2 マップ構築用）
`outside.png` — 野外フィールド、`terrain.png` — 地形、`world.png` — ワールドマップ、`castle.png` — 拠点

### 動物スプライト（シートのみ）
`animal/individual_frames/animals1〜5, birds1〜2, horse` — 鹿群れシナリオに `animals` を使用候補（要フレーム分割）

---

## Phase 2: 文明進行 (4-6週間)

### 技術ツリー

- [ ] `tree_protocol.toml` パーサー実装
- [ ] 最小ツリー15-20ノード（石器時代→農業時代→青銅器時代）
- [ ] 技術進歩トリガー: 会話キーワード → 研究ポイント加算 → 閾値で解放

### 世界

- [ ] 住民の増加（5→10→20人）
- [ ] 資源（食料/素材）の基本パラメータ
- [ ] ティア制更新: 注目住民=毎ターン / 重要人物=3ターンごと / その他=ルールベース
- [ ] **HDL検討**: 住民20人以上でYAMLがボトルネックになった時点で独自フォーマット調査

### 戦争

- [ ] 部族分裂 → 戦争トリガー（資源争い/侮辱/復讐）→ 人口減少・技術喪失
- [ ] プレイヤーの介入は認知度に依存。完全には止められない

### 認知度

- [ ] 声を届けるたびに微量上昇。声の内容が現実に起こると急上昇。放置で低下
- [ ] 0%: プロンプト除外 / 1-30%: ノイズ / 31-60%: 断片 / 61-90%: お告げ / 91-100%: ほぼそのまま

---

## Phase 3: ポリッシュ (3-4週間)

- [ ] Player2 API 計測 + LLMセットアップウィザード（Player2 → ローカル → クラウド）
- [ ] セーブ/ロード
- [ ] 技術ツリー表示UI
- [ ] 時代に応じた背景変化
- [ ] 最低スペックテスト（GTX 1060 / CPU only / Apple Silicon M1）
- [ ] 外部プレイテスト5人以上

## Phase 4: リリース (2-3週間)

- [ ] Steamストアページ + itch.io
- [ ] トレーラー
- [ ] ストリーマーへのキー配布

---

## やらないこと (MVPスコープ外)

3D、マルチプレイヤー、TTS、青銅器時代より先、DLC、コンソール、住民100人以上。
