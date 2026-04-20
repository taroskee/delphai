<!-- Generated: 2026-04-20 | Files scanned: ~10 | Token estimate: ~950 -->

# Frontend — Godot 4 (`game/`)

描画・入力・シーンのみ。ロジックは持たず、Rust 側 `WorldNode` (FFI) に委譲。

## プロジェクト構成

```
game/
├── project.godot
├── delphai.gdextension           ← gdext → libdelphai_gdext.{so,dylib,dll}
├── addons/
│   └── terrain_3d/                ← 外部 addon (Cesium/TokisanGames)
├── scenes/
│   ├── world.tscn                 ← ルートシーン
│   └── world.gd         (489)     ← _process ループ + FFI 呼び出し
└── scripts/
    ├── terrain_builder.gd (389)   ← Terrain3D ハイトマップ生成
    ├── citizen_factory.gd (87)    ← 住民 Node3D 生成
    ├── animal_factory.gd  (59)    ← 鹿 Node3D 生成
    ├── resource_factory.gd(80)    ← ベリー/水源 Node3D 生成
    ├── village_builder.gd (59)    ← 家/小屋 配置
    ├── glb_loader.gd      (73)    ← GLB ロード/フォールバック
    ├── debug_hud.gd       (48)    ← FPS/tick 表示
    └── tests/                     ← GDScript テスト (追加された)
```

## シーンツリー (world.tscn)

```
World (Node3D, script: world.gd)
├── WorldNode                     (Rust FFI — delphai-gdext)
├── Terrain3D                     (addon)
├── Camera3D
├── DirectionalLight3D
├── Citizens (Node3D)             ← citizen_factory.gd が子を追加
├── Animals (Node3D)
├── Resources (Node3D)
├── Village (Node3D)
└── DebugHUD (CanvasLayer)
```

## 毎フレームループ (`world.gd`)

```gdscript
# 概略 (実装そのものではなく構造)
func _process(delta):
    _tick_accum += delta
    if _tick_accum >= TICK_INTERVAL:   # 4Hz (0.25s)
        world_node.tick()
        _tick_accum -= TICK_INTERVAL
        _sync_births_and_deaths()       # pop_citizen_birth / pop_animal_fled
    var alpha = _tick_accum / TICK_INTERVAL
    for i in citizen_count:
        var pos = world_node.get_citizen_world_pos(i, alpha)
        _citizen_nodes[i].position = Vector3(pos.x, y, pos.y)
```

**重要**: 補間 (`alpha`) は Rust 側 `MoveState.prev_tile_pos` → `tile_pos` の線形補間。GDScript では補間計算を持たない。

## terrain_builder.gd

`build_terrain3d(terrain_node, map_w, map_h)`:
1. `Terrain3DAssets` にテクスチャ 2 枚を `set_texture(0|1, ...)` — Grass / Dirt (インライン化済)
2. **複合ハイトマップ生成**: `gentle_noise + mountain1_gaussian + mountain2_gaussian + river_trench + village_flat_overlay`
3. `classify_tiles_from_height(h)` → 5 コード (T_GROUND=0 / T_FOREST=1 / T_SHALLOW=2 / T_DEEP=3 / T_MOUNTAIN=4)
4. 水面は別 `MeshInstance3D` (半透明平面)、村は (21, 10) 周辺を平坦化

定数: `GRASS_ALBEDO_PATH`, `DIRT_ALBEDO_PATH`, `*_NORMAL_PATH`, `*_FALLBACK_COLOR`。

## factory 系

| factory | 生成物 | GLB フォールバック |
|---|---|---|
| citizen_factory | `CharacterBody3D` + `MeshInstance3D` (CapsuleMesh) | GLB 指定、無ければプリミティブ |
| animal_factory | 鹿 Node3D | 同上 |
| resource_factory | ベリー茂み / 水源 | 同上 |
| village_builder | 家・小屋 (プリミティブ) | なし |

`glb_loader.gd` が `ResourceLoader.load` のラッパ + 失敗時プリミティブ生成。

## FFI 呼び出し点 (world.gd → WorldNode)

- 初期化: `initialize()`, `set_walkable_map(grid)`
- 毎 tick: `tick()`, `pop_citizen_birth()`, `pop_animal_fled()`
- 毎フレーム: `get_citizen_world_pos(i, alpha)`, `get_citizen_facing(i)`, `get_citizen_emotion(i)`
- 会話: `build_conversation_prompt_str(i)`, `apply_citizen_response(i, yaml)`, `record_heard_speech(i, text)`
- 神託: `get_divine_awareness(i)`, `grow_divine_awareness(i, delta)`

## デバッグ

- `debug_hud.gd` が FPS / tick / citizen needs を可視化
- Godot MCP 経由で `mcp__godot__run_project` → `get_debug_output` で確認

## 再構築時の方針

`scripts/*.gd` と `world.gd` / `world.tscn` は **全面書き直し対象**。Rust 側 API 形状が変わるので、FFI シグネチャから作り直す。Terrain3D addon の使い方と GLB フォールバック規約だけは移植元として参照価値あり。
