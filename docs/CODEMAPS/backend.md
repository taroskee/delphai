<!-- Generated: 2026-04-20 | Files scanned: ~18 | Token estimate: ~1000 -->

# Backend — Rust Core (`crates/delphai-core`)

純粋シム。`godot` クレート非依存。`cargo test -p delphai-core` で 168 テスト。

## モジュール構成

```
delphai-core/src/
├── lib.rs              ← pub mod 列挙のみ
├── world.rs            (651) World 集約 + tick オーケストレーション
├── pathfinding.rs      (493) TilePos / MoveState / WalkGrid / 局所探索
├── animal.rs           (244) 鹿 state machine (Roaming/Fleeing/Dead)
├── resource.rs         (122) ベリー/水 再生ロジック
├── tech.rs             (140) 技術ツリー進行
├── agent/
│   ├── mod.rs          (7)
│   ├── behavior.rs     (208) BehaviorState FSM + pure tick()
│   ├── citizen.rs      (120) Citizen 構造体 + Needs
│   └── conversation.rs (180) 会話記録・参照
└── llm/  (保存対象)
    ├── mod.rs          (17)
    ├── provider.rs     (162) LlmProvider trait, LlmError, CitizenResponse
    ├── player2.rs      (148) Player2Provider — reqwest HTTP
    ├── queue.rs        (239) InferenceQueue 優先度付き
    ├── prompt.rs       (515) YAML プロンプトビルダー
    ├── response_parser.rs (399) YAML + JSON パーサ
    └── memory.rs       (259) 会話・観察の記憶
```

## `World` 集約 (`world.rs`)

フィールド:
- `citizens: Vec<Citizen>`, `behavior_states: Vec<BehaviorState>`, `vitals: Vec<Vitals>`, `move_states: Vec<MoveState>`
- `walk_grid: WalkGrid`, `resources: Vec<Resource>`, `animals: Vec<Animal>`, `tech_tree: TechTree`
- `tick_count: u32`, `prosperity_ticks: u32`, `pending_births: Vec<Citizen>`

`tick()` フェーズ順 (順序固定):
```
1. 時間進行 (tick_count++)
2. needs decay (fed / hydration)
3. resources regenerate
4. behavior::tick() → BehaviorAction
5. stationary_interactions (採集/飲水/会話)
6. movement (MoveState を介した 1-step 移動)
7. hunting (citizen → animal)
8. animals tick (徘徊/逃走)
9. maybe_spawn_citizen (繁栄閾値)
```

公開ゲッター (gdext から呼ばれる):
- `get_citizen_count/name/emotion/fed/hydration/behavior/tile_pos/facing`
- `get_citizen_world_pos(idx, alpha)` — **サブ tick 補間の要**
- `get_resource_count/type/amount/tile_pos`
- `get_animal_count/kind/tile_pos/state`
- `get_tech_progress/unlocked`

## pathfinding (`pathfinding.rs`)

- `TilePos { x: i16, y: i16 }` 不変値
- `MoveState { tile_pos, prev_tile_pos, move_target, move_history[16], step_cooldown, arrive_cooldown }`
- `WalkGrid` — 行優先 `Vec<bool>`
- `step()` は **局所 1-step**: 歩行可能な隣接 8 近傍から goal に最も近いマスを選択、`move_history` にあるマスはペナルティ
- A* なし。シンプル優先。詰まりは history 回避で解消

## behavior (`agent/behavior.rs`)

```
enum BehaviorState { Idle, SeekingFood, Gathering, SeekingWater, Drinking, Hunting, /*...*/ }
enum BehaviorAction { Idle, MoveTo(TilePos), Gather(ResourceId), Drink(ResourceId), /*...*/ }
fn tick(state: &mut BehaviorState, needs: &Needs) -> BehaviorAction
```

優先度: **hydration < 0.3 → SeekingWater 最優先**、`fed < 0.4 → SeekingFood`、それ以外 Idle。純粋関数。

## animal (`animal.rs`)

```
enum AnimalKind { Deer }
enum AnimalState { Roaming, Fleeing, Dead }
```

citizen が近づくと Fleeing、一定距離で Roaming 復帰。死亡で `pop_animal_fled` に載り GDScript 側が消す。

## LLM サブシステム (保存対象)

- `LlmProvider` trait: `async fn infer(&self, prompt: String) -> Result<String, LlmError>`
- `Player2Provider` — `http://localhost:4315/v1/chat/completions` へ POST
- `InferenceQueue` — 優先度 (Critical / High / Normal) で順次実行
- `YamlResponseParser` — LLM 応答 YAML → `CitizenResponse { text, emotion, action }`
- `Memory` — 発話・観察を citizen ごとに最新 N 件で保持

## テスト

- `#[cfg(test)]` モジュール内インテグレーション的ユニットテスト
- 168 passing (Rust core) + 4 passing (gdext)
- 代表: `world::tests::tick_advances_time`、`pathfinding::tests::avoids_recent_history`、`behavior::tests::hydration_priority_overrides_food`
