<!-- Generated: 2026-04-20 | Files scanned: ~25 | Token estimate: ~1200 -->

# アーキテクチャ (C4 モデル)

Delphai は Godot 4 を描画/入力、Rust を純粋シム、という二層構造。gdext FFI が境界。LLM は唯一のネット依存で、再構築時にも保存する部分。

## 1. System Context (C4 Level 1)

```mermaid
C4Context
  title System Context — Delphai
  Person(player, "プレイヤー", "観察者として村を見る")
  System(delphai, "Delphai Simulation", "村シミュレーション + 神託インターフェース")
  System_Ext(player2, "Player2 LLM API", "localhost HTTP (OpenAI互換)")
  System_Ext(godot, "Godot Editor/Runtime 4.x", "描画・シーン・入力")
  Rel(player, delphai, "観察 + 神託入力")
  Rel(delphai, godot, "Node/Scene API")
  Rel(delphai, player2, "POST /v1/chat/completions", "HTTP")
```

## 2. Container (C4 Level 2)

```mermaid
C4Container
  title Container — Delphai Runtime
  Person(player, "プレイヤー")
  System_Boundary(b, "Delphai") {
    Container(gd, "Godot Scene (GDScript)", "GDScript 4", "world.tscn / world.gd / *_factory.gd / terrain_builder.gd")
    Container(gdext, "delphai-gdext", "Rust cdylib", "WorldNode FFI: #[func] 群")
    Container(core, "delphai-core", "Rust library", "純粋シム: world, pathfinding, agent, animal, resource, tech, llm")
    Container(bench, "delphai-bench", "Rust bin + criterion", "ベンチ計測")
    Container(terrain3d, "Terrain3D addon", "GDExtension (外部)", "ハイトマップ・テクスチャ描画")
  }
  System_Ext(player2, "Player2 LLM", "HTTP")
  Rel(player, gd, "入力")
  Rel(gd, gdext, "_process → tick() + getter群", "FFI")
  Rel(gdext, core, "Rust 呼び出し")
  Rel(gd, terrain3d, "Terrain3D / Terrain3DAssets")
  Rel(core, player2, "reqwest HTTP", "async")
  Rel(bench, core, "criterion bench")
```

## 3. Component (C4 Level 3) — delphai-core 内部

```mermaid
C4Component
  title Component — delphai-core
  Container_Boundary(core, "delphai-core") {
    Component(world, "world", "Rust mod", "World 集約。tick() オーケストレーション")
    Component(pathfinding, "pathfinding", "Rust mod", "TilePos / MoveState / WalkGrid / 1-step 局所探索")
    Component(agent, "agent", "Rust mod", "behavior FSM / citizen / conversation")
    Component(animal, "animal", "Rust mod", "鹿の徘徊・逃走・捕食")
    Component(resource, "resource", "Rust mod", "ベリー/水 再生ロジック")
    Component(tech, "tech", "Rust mod", "技術ツリー進行")
    Component(llm, "llm", "Rust mod", "provider / queue / prompt / parser / memory")
  }
  Rel(world, pathfinding, "MoveState 更新")
  Rel(world, agent, "behavior::tick()")
  Rel(world, animal, "animal tick")
  Rel(world, resource, "再生 tick")
  Rel(world, tech, "進行 tick")
  Rel(world, llm, "推論要求/応答適用")
```

## 4. 毎フレーム / 毎 tick のデータフロー

```mermaid
sequenceDiagram
  participant GD as world.gd (_process)
  participant FFI as WorldNode (gdext)
  participant W as World (core)
  participant P as phases
  participant LLM as Player2Provider
  loop 60 FPS
    GD->>FFI: tick_accum += delta
    alt tick 時刻に到達 (4Hz)
      GD->>FFI: tick()
      FFI->>W: tick()
      W->>P: decay → resource regen → behavior → stationary_interactions → movement → hunting → animals → birth?
      W-->>FFI: 状態更新
    end
    GD->>FFI: get_citizen_world_pos(i, alpha)
    FFI->>W: get_citizen_world_pos(i, alpha)
    W-->>GD: Vector2 (prev→curr 線形補間)
    GD->>GD: Node3D.position 反映
  end
  opt 会話開始
    GD->>FFI: build_conversation_prompt_str
    FFI->>LLM: POST /v1/chat/completions
    LLM-->>FFI: YAML応答
    FFI->>W: apply_citizen_response (parse 後)
  end
```

## 5. 設計原則

- **境界は FFI 一本**。gdext 以外に GDScript↔Rust の経路は作らない
- **コアは `godot` クレート非依存**。`delphai-core` は単体で `cargo test` 可能
- **tick は 4Hz (DAY_TICKS=600)**、描画は 60FPS。補間は Rust 側で実行 (`MoveState.prev_tile_pos`)
- **LLM は副作用の境界**。`InferenceQueue` で優先度制御、応答は YAML パース後に `apply_citizen_response`
- **再構築時に残すのは `llm/` サブモジュール一式のみ**。world/pathfinding/agent/animal/resource/tech/gdext は作り直し対象

## 6. 主要パラメータ (定数)

| 定数 | 値 | 定義元 |
|---|---|---|
| TICK_RATE | 4 Hz | `world.gd` |
| MAP_SIZE | 24×14 | `world.gd` / `World::new` |
| TILE_SIZE | 2.0 m | `world.gd` / core |
| DAY_TICKS | 600 | `world.rs` |
| FED_DECAY | 0.004/tick | `world.rs` |
| HYDRATION_DECAY | 0.007/tick | `world.rs` |
| MAX_CITIZENS | 8 | `world.rs` |
| BIRTH_THRESHOLD | 200 ticks | `world.rs` |
| STEP_COOLDOWN / ARRIVE_COOLDOWN | 0 / 1 | `pathfinding.rs` |
