class_name TerrainBuilder
extends RefCounted

## Pure builder for the terrain grid and its walkable bitmap.
## All methods are static — no state lives on this class. `world.gd` calls
## `build_collision_plane`, `build_visual_backdrop`, `build_features`,
## and `make_walkable_map` during setup.
##
## `get_height_at(x, z)` is the single Y-source-of-truth for placing objects
## on the ground. Currently returns 0.0 (flat world); Sprint 13.3 replaces
## the body with Terrain3D Raycast sampling.

# Terrain codes
const T_GROUND   := 0
const T_FOREST   := 1
const T_SHALLOW  := 2
const T_DEEP     := 3
const T_MOUNTAIN := 4

# Layout zones (named constants replace inline magic numbers).
const RIVER_DEEP_COL       := 18
const RIVER_SHALLOW_COLS   := [17, 19]
const MOUNTAIN_CORNER_MAX  := 4   # col,row ≤ this
const MOUNTAIN_DIAG_MAX    := 6   # col+row ≤ this
const FOREST_A_COLS        := [3, 10]  # inclusive range — NW
const FOREST_A_ROWS        := [1, 6]
const FOREST_B_COLS        := [4, 10]   # SW — moved from [12,16] so the SE quadrant
const FOREST_B_ROWS        := [8, 12]   # remains open flat grass for the village.

## Classify a tile. `map_w`/`map_h` are inclusive of border walls.
static func get_terrain(col: int, row: int, map_w: int, map_h: int) -> int:
	if col == 0 or col == map_w - 1 or row == 0 or row == map_h - 1:
		return T_MOUNTAIN
	if col <= MOUNTAIN_CORNER_MAX and row <= MOUNTAIN_CORNER_MAX and col + row <= MOUNTAIN_DIAG_MAX:
		return T_MOUNTAIN
	if row >= 1 and row <= map_h - 2:
		if col == RIVER_DEEP_COL:
			return T_DEEP
		if col in RIVER_SHALLOW_COLS:
			return T_SHALLOW
	if col >= FOREST_A_COLS[0] and col <= FOREST_A_COLS[1] and row >= FOREST_A_ROWS[0] and row <= FOREST_A_ROWS[1]:
		if (col * 17 + row * 31) % 4 < 3:
			return T_FOREST
	if col >= FOREST_B_COLS[0] and col <= FOREST_B_COLS[1] and row >= FOREST_B_ROWS[0] and row <= FOREST_B_ROWS[1]:
		if (col * 13 + row * 23) % 4 < 3:
			return T_FOREST
	return T_GROUND

## Single Y-source-of-truth for placing objects on the ground.
## Currently returns 0.0 (flat world); Sprint 13.3 replaces the body with
## Terrain3D Raycast sampling so citizens/resources/animals snap to the heightmap.
static func get_height_at(_x: float, _z: float) -> float:
	return 0.0

# Deprecated: Sprint 13.1 で Terrain3D に置換予定。
# terrian.glb は装飾背景としてのみ使用中で、物理・ロジックとは分離している。
const GROUND_GLB         := "res://assets/geography/terrian.glb"
# Deprecated: Sprint 13.1 で Terrain3D に置換予定。
const GROUND_GLB_SCALE   := 0.15
# Deprecated: Sprint 13.1 で Terrain3D に置換予定。
const GROUND_GLB_Y       := -0.05

## Build the invisible collision plane under `parent`. Only physics — no visual.
## The StaticBody+CollisionShape provide character grounding; the green PlaneMesh
## has been removed (terrian.glb / Terrain3D owns the visual).
static func build_collision_plane(parent: Node3D, map_w: int, map_h: int, tile_size: float) -> void:
	var body := StaticBody3D.new()
	body.name = "Terrain"
	parent.add_child(body)

	var center := Vector3((map_w - 1) * 0.5 * tile_size, get_height_at((map_w - 1) * 0.5 * tile_size, (map_h - 1) * 0.5 * tile_size), (map_h - 1) * 0.5 * tile_size)

	var col_shape := CollisionShape3D.new()
	var box := BoxShape3D.new()
	box.size = Vector3(map_w * tile_size, 0.1, map_h * tile_size)
	col_shape.shape = box
	col_shape.position = center
	body.add_child(col_shape)

## Load `terrian.glb` as a decorative backdrop centered at `center`.
## Sprint 13.1 で Terrain3D ノード生成に置換予定。
static func build_visual_backdrop(parent: Node3D, center: Vector3) -> void:
	var packed := load(GROUND_GLB) as PackedScene
	if packed == null:
		return
	var scene := packed.instantiate() as Node3D
	if scene == null:
		return
	scene.name = "TerrainBackdrop"
	scene.scale = Vector3.ONE * GROUND_GLB_SCALE
	scene.position = center + Vector3(0, GROUND_GLB_Y, 0)
	parent.add_child(scene)

## Build trees per-tile under a new "TerrainFeatures" node.
## Mountains, shallow/deep water, and the green ground plane are now fully
## expressed by `terrian.glb` — we no longer add primitive polygons for them.
## The walkable bitmap still classifies T_MOUNTAIN and T_DEEP as blocked.
static func build_features(parent: Node3D, map_w: int, map_h: int, tile_size: float) -> void:
	var container := Node3D.new()
	container.name = "TerrainFeatures"
	parent.add_child(container)
	for row in range(map_h):
		for col in range(map_w):
			var t := get_terrain(col, row, map_w, map_h)
			if t == T_FOREST:
				var x := col * tile_size
				var z := row * tile_size
				var wpos := Vector3(x, get_height_at(x, z), z)
				_add_tree(container, wpos)

## Produce the walkable bitmap sent to the Rust side.
## `1` = walkable, `0` = blocked (deep water, mountains).
static func make_walkable_map(map_w: int, map_h: int) -> PackedByteArray:
	var data := PackedByteArray()
	data.resize(map_w * map_h)
	for row in range(map_h):
		for col in range(map_w):
			var t := get_terrain(col, row, map_w, map_h)
			data[row * map_w + col] = 0 if (t == T_DEEP or t == T_MOUNTAIN) else 1
	return data

# ── Private feature builders ──────────────────────────────────────────────────

const TREE_GLB       := "res://assets/nature/simple_nature_pack_glb.glb"
const TREE_NODE_NAME := "Oak_Tree_01"
const TREE_SCALE     := 0.35

static func _add_tree(parent: Node3D, pos: Vector3) -> void:
	var glb_node := GlbLoader.load_subscene(TREE_GLB, TREE_NODE_NAME)
	if glb_node == null:
		return
	var root := Node3D.new()
	root.position = pos
	glb_node.scale = Vector3.ONE * TREE_SCALE
	root.add_child(glb_node)
	parent.add_child(root)
