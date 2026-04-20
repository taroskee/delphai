class_name TerrainBuilder
extends RefCounted

## Pure builder for the terrain grid and its walkable bitmap.
## All methods are static — no state lives on this class except `_terrain`,
## which holds the live Terrain3D node so `get_height_at` can query it.
##
## `world.gd` calls `build_collision_plane`, `build_terrain3d`,
## `build_features`, and `make_walkable_map` during setup.
##
## `get_height_at(x, z)` is the single Y-source-of-truth for placing objects
## on the ground. Before `build_terrain3d` has run it returns 0.0; afterwards
## it samples Terrain3DData.

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

# Terrain3D procgen parameters (Sprint 13.1, adjusted 13.3).
# Fixed seed keeps MVP reproducible; post-MVP randomizes this only.
# freq 0.05→0.08 (shorter wavelength, steeper ridges) and height_scale 5→8
# (taller noise) to make auto-shader slope blend active across the terrain.
const TERRAIN_SEED         := 42
const TERRAIN_NOISE_FREQ   := 0.08
const TERRAIN_REGION_SIZE  := 512    # meters per Terrain3DRegion
const TERRAIN_IMAGE_SIZE   := 512    # heightmap pixels (1 px = 1 m)
const TERRAIN_HEIGHT_SCALE := 8.0    # noise [-1, 1] → ±8 m world
# Village flat-zone: within FLAT_RADIUS height is forced to 0 so citizens
# spawn on a level plain. FADE_RADIUS smooths the transition back to noise.
const VILLAGE_FLAT_RADIUS  := 10.0
const VILLAGE_FADE_RADIUS  := 15.0

# Held so `get_height_at(x, z)` can sample the live Terrain3D.data.
# Remains null until `build_terrain3d` runs, in which case `get_height_at`
# returns 0.0 (flat world) — preserving the Sprint 13.01 seam behavior.
static var _terrain: Terrain3D = null

# Per-tile T_* cache populated by `classify_tiles_from_height`.
# Until populated, `get_terrain` falls back to layout-based classification
# (preserves boot-time behavior before world.gd has called classify).
static var _tile_cache: PackedByteArray = PackedByteArray()
static var _tile_cache_w: int = 0

## Classify a tile. `map_w`/`map_h` are inclusive of border walls.
## After `classify_tiles_from_height` runs, returns cached value (O(1)).
## Before then, falls through to layout-only classification.
static func get_terrain(col: int, row: int, map_w: int, map_h: int) -> int:
	if _tile_cache_w == map_w and _tile_cache.size() == map_w * map_h:
		return _tile_cache[row * map_w + col]
	return _classify_layout(col, row, map_w, map_h)

## Layout-only classification (no height sampling). Source of truth for
## river/forest/mountain-corner placement. The cache layer in
## `classify_tiles_from_height` calls this and additionally promotes
## tall procgen tiles to T_MOUNTAIN.
static func _classify_layout(col: int, row: int, map_w: int, map_h: int) -> int:
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

## Build the per-tile T_* cache from layout + sampled procgen heights.
## Must be called after `build_terrain3d` so `get_height_at` is populated.
## Heights above MOUNTAIN_HEIGHT_THRESHOLD are promoted to T_MOUNTAIN so
## procgen ridges block movement in addition to the layout border walls.
const MOUNTAIN_HEIGHT_THRESHOLD := 3.0
static func classify_tiles_from_height(map_w: int, map_h: int, tile_size: float) -> void:
	_tile_cache.resize(map_w * map_h)
	_tile_cache_w = map_w
	for row in range(map_h):
		for col in range(map_w):
			var t := _classify_layout(col, row, map_w, map_h)
			if t == T_GROUND or t == T_FOREST:
				var h := get_height_at(col * tile_size, row * tile_size)
				if h > MOUNTAIN_HEIGHT_THRESHOLD:
					t = T_MOUNTAIN
			_tile_cache[row * map_w + col] = t

## Single Y-source-of-truth for placing objects on the ground.
## Returns 0.0 until `build_terrain3d` has populated `_terrain`.
## After procgen, samples Terrain3DData; NaN results (sampled outside a
## region) fall back to 0.0 so callers never get a poisoned Y.
static func get_height_at(x: float, z: float) -> float:
	if _terrain == null:
		return 0.0
	var h: float = _terrain.data.get_height(Vector3(x, 0.0, z))
	return 0.0 if is_nan(h) else h

## Build the invisible collision plane under `parent`. Only physics — no visual.
## Terrain3D owns the primary collision; this BoxShape3D is a flat fallback
## ensuring characters cannot fall through in case Terrain3DCollision fails
## to initialize or heightmap generation produces NaN outside the region.
static func build_collision_plane(parent: Node3D, map_w: int, map_h: int, tile_size: float) -> void:
	var body := StaticBody3D.new()
	body.name = "Terrain"
	parent.add_child(body)

	var cx := (map_w - 1) * 0.5 * tile_size
	var cz := (map_h - 1) * 0.5 * tile_size
	var center := Vector3(cx, get_height_at(cx, cz), cz)

	var col_shape := CollisionShape3D.new()
	var box := BoxShape3D.new()
	box.size = Vector3(map_w * tile_size, 0.1, map_h * tile_size)
	col_shape.shape = box
	col_shape.position = center
	body.add_child(col_shape)

## Build a procedurally-generated Terrain3D under `parent`.
## Replaces the deprecated terrian.glb backdrop. Heightmap is seeded
## FastNoiseLite forced flat within VILLAGE_FLAT_RADIUS of `village_center`,
## so citizens spawned at the village tile stand on level ground.
static func build_terrain3d(parent: Node3D, village_center: Vector3) -> Terrain3D:
	var terrain := Terrain3D.new()
	terrain.name = "Terrain3D"
	terrain.region_size = TERRAIN_REGION_SIZE

	var assets := Terrain3DAssets.new()
	assets.set_texture(0, _create_grass_asset())
	assets.set_texture(1, _create_dirt_asset())
	terrain.assets = assets

	parent.add_child(terrain)

	# Auto-shader blends grass (texture 0) → dirt (texture 1) by slope, so
	# procgen ridges are immediately visible without a hand-painted control map.
	# Shader uniform names verified via `strings game/addons/terrain_3d/bin/libterrain.*.so`.
	# auto_slope=5 (down from 10) lowers blend threshold so more of the terrain
	# matches the shader's slope condition (freq↑ & height_scale↑ make ridges steeper).
	terrain.material.auto_shader = true
	terrain.material.set_shader_param("auto_slope", 5.0)
	terrain.material.set_shader_param("blend_sharpness", 0.975)

	var img := _generate_heightmap(village_center)
	terrain.data.import_images([img, null, null], Vector3.ZERO, 0.0, TERRAIN_HEIGHT_SCALE)

	# Terrain3D v1.0.1: enum members are DISABLED / DYNAMIC_GAME / DYNAMIC_EDITOR
	# / FULL_GAME / FULL_EDITOR. There is no plain `DYNAMIC`. Use DYNAMIC_GAME
	# for runtime collision (DYNAMIC_EDITOR builds collision only inside the editor).
	terrain.collision.mode = Terrain3DCollision.DYNAMIC_GAME

	_terrain = terrain
	return terrain

## Build trees per-tile under a new "TerrainFeatures" node.
## Mountains, shallow/deep water, and the green ground plane are now fully
## expressed by the Terrain3D heightmap — we no longer add primitive polygons
## for them. The walkable bitmap still classifies T_MOUNTAIN and T_DEEP as
## blocked.
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

# ── Private terrain builders ──────────────────────────────────────────────────

## Seeded FastNoiseLite heightmap with a flattened disc around the village.
## Image is FORMAT_RF; each pixel stores raw noise ∈ [-1, 1] in the R channel.
## `import_images` multiplies by TERRAIN_HEIGHT_SCALE later.
static func _generate_heightmap(village_center: Vector3) -> Image:
	var noise := FastNoiseLite.new()
	noise.seed = TERRAIN_SEED
	noise.frequency = TERRAIN_NOISE_FREQ

	var size := TERRAIN_IMAGE_SIZE
	var img := Image.create_empty(size, size, false, Image.FORMAT_RF)
	var vx := village_center.x
	var vz := village_center.z
	var flat_r2 := VILLAGE_FLAT_RADIUS * VILLAGE_FLAT_RADIUS
	var fade_r2 := VILLAGE_FADE_RADIUS * VILLAGE_FADE_RADIUS
	for y in range(size):
		for x in range(size):
			var dx := float(x) - vx
			var dz := float(y) - vz
			var d2 := dx * dx + dz * dz
			var h := noise.get_noise_2d(float(x), float(y))
			if d2 <= flat_r2:
				h = 0.0
			elif d2 < fade_r2:
				var t := (sqrt(d2) - VILLAGE_FLAT_RADIUS) / (VILLAGE_FADE_RADIUS - VILLAGE_FLAT_RADIUS)
				h *= t
			img.set_pixel(x, y, Color(h, 0.0, 0.0, 1.0))
	return img

## Grass base texture (texture 0). Solid green; auto-shader picks this on
## near-flat tiles. Slope-driven blend toward dirt (texture 1) reveals ridges.
static func _create_grass_asset() -> Terrain3DTextureAsset:
	var img := Image.create_empty(64, 64, false, Image.FORMAT_RGBA8)
	img.fill(Color(0.26, 0.52, 0.26, 1.0))
	var tex := ImageTexture.create_from_image(img)
	var ta := Terrain3DTextureAsset.new()
	ta.name = "Grass"
	ta.albedo_texture = tex
	ta.uv_scale = 0.5
	return ta

## Dirt overlay texture (texture 1). Slope-driven auto-shader fades to this on
## steep faces so procgen ridges read as brown ridges instead of flat green.
static func _create_dirt_asset() -> Terrain3DTextureAsset:
	var img := Image.create_empty(64, 64, false, Image.FORMAT_RGBA8)
	img.fill(Color(0.42, 0.30, 0.18, 1.0))
	var tex := ImageTexture.create_from_image(img)
	var ta := Terrain3DTextureAsset.new()
	ta.name = "Dirt"
	ta.albedo_texture = tex
	ta.uv_scale = 0.5
	return ta

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
