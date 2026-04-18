class_name TerrainBuilder
extends RefCounted

## Pure builder for the terrain grid and its walkable bitmap.
## All methods are static — no state lives on this class. `world.gd` calls
## `build_ground`, `build_features`, and `make_walkable_map` during setup.

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
const FOREST_A_COLS        := [3, 10]  # inclusive range
const FOREST_A_ROWS        := [1, 6]
const FOREST_B_COLS        := [12, 16]
const FOREST_B_ROWS        := [7, 12]

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

const GROUND_GLB         := "res://assets/geography/terrian.glb"
# Sketchfab monolithic terrain — not per-tile. Placed as a decorative backdrop
# beneath the flat plane. Scale/Y-offset are empirical; tune in the editor if needed.
const GROUND_GLB_SCALE   := 0.15
const GROUND_GLB_Y       := -0.05

## Build the flat ground plane + collision under `parent`.
## Also loads the decorative `terrian.glb` backdrop beneath when available
## (falls back to the plane-only look if the GLB is missing).
static func build_ground(parent: Node3D, map_w: int, map_h: int, tile_size: float) -> void:
	var body := StaticBody3D.new()
	body.name = "Terrain"
	parent.add_child(body)

	var center := Vector3((map_w - 1) * 0.5 * tile_size, 0.0, (map_h - 1) * 0.5 * tile_size)

	var mesh_inst := MeshInstance3D.new()
	var plane := PlaneMesh.new()
	plane.size = Vector2(map_w * tile_size, map_h * tile_size)
	mesh_inst.mesh = plane
	var mat := StandardMaterial3D.new()
	mat.albedo_color = Color(0.33, 0.52, 0.22)  # grass
	mesh_inst.material_override = mat
	mesh_inst.position = center
	body.add_child(mesh_inst)

	var col_shape := CollisionShape3D.new()
	var box := BoxShape3D.new()
	box.size = Vector3(map_w * tile_size, 0.1, map_h * tile_size)
	col_shape.shape = box
	col_shape.position = center
	body.add_child(col_shape)

	_add_ground_glb_backdrop(parent, center)

static func _add_ground_glb_backdrop(parent: Node3D, center: Vector3) -> void:
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

## Build mountains / trees / water planes per-tile under a new "TerrainFeatures" node.
static func build_features(parent: Node3D, map_w: int, map_h: int, tile_size: float) -> void:
	var container := Node3D.new()
	container.name = "TerrainFeatures"
	parent.add_child(container)
	for row in range(map_h):
		for col in range(map_w):
			var t := get_terrain(col, row, map_w, map_h)
			var wpos := Vector3(col * tile_size, 0.0, row * tile_size)
			match t:
				T_MOUNTAIN:
					_add_mountain(container, wpos, tile_size)
				T_FOREST:
					_add_tree(container, wpos)
				T_SHALLOW:
					_add_water_plane(container, wpos, tile_size, Color(0.3, 0.65, 0.95, 0.72))
				T_DEEP:
					_add_water_plane(container, wpos, tile_size, Color(0.05, 0.18, 0.72, 0.88))

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

static func _add_mountain(parent: Node3D, pos: Vector3, tile_size: float) -> void:
	var mi := MeshInstance3D.new()
	var cyl := CylinderMesh.new()
	cyl.top_radius    = 0.0
	cyl.bottom_radius = tile_size * 0.48
	cyl.height        = tile_size * 0.85
	mi.mesh = cyl
	var mat := StandardMaterial3D.new()
	mat.albedo_color = Color(0.54, 0.51, 0.49)
	mi.material_override = mat
	mi.position = pos + Vector3(0, tile_size * 0.425, 0)
	parent.add_child(mi)

const TREE_GLB       := "res://assets/nature/simple_nature_pack_glb.glb"
const TREE_NODE_NAME := "Oak_Tree_01"
const TREE_SCALE     := 0.35

static func _add_tree(parent: Node3D, pos: Vector3) -> void:
	var root := Node3D.new()
	root.position = pos
	var glb_node := GlbLoader.load_subscene(TREE_GLB, TREE_NODE_NAME)
	if glb_node != null:
		glb_node.scale = Vector3.ONE * TREE_SCALE
		root.add_child(glb_node)
	else:
		_add_tree_fallback(root)
	parent.add_child(root)

static func _add_tree_fallback(root: Node3D) -> void:
	var trunk_mi := MeshInstance3D.new()
	var trunk := CylinderMesh.new()
	trunk.top_radius    = 0.08
	trunk.bottom_radius = 0.12
	trunk.height        = 0.55
	trunk_mi.mesh = trunk
	var trunk_mat := StandardMaterial3D.new()
	trunk_mat.albedo_color = Color(0.40, 0.26, 0.12)
	trunk_mi.material_override = trunk_mat
	trunk_mi.position.y = 0.28
	root.add_child(trunk_mi)

	var canopy_mi := MeshInstance3D.new()
	var canopy := SphereMesh.new()
	canopy.radius = 0.48
	canopy.height = 0.96
	canopy_mi.mesh = canopy
	var canopy_mat := StandardMaterial3D.new()
	canopy_mat.albedo_color = Color(0.13, 0.46, 0.11)
	canopy_mi.material_override = canopy_mat
	canopy_mi.position.y = 0.9
	root.add_child(canopy_mi)

static func _add_water_plane(parent: Node3D, pos: Vector3, tile_size: float, color: Color) -> void:
	var mi := MeshInstance3D.new()
	var plane := PlaneMesh.new()
	plane.size = Vector2(tile_size * 0.98, tile_size * 0.98)
	mi.mesh = plane
	var mat := StandardMaterial3D.new()
	mat.albedo_color = color
	mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	mi.material_override = mat
	mi.position = pos + Vector3(0, 0.01, 0)
	parent.add_child(mi)
