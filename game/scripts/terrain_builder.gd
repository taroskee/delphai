class_name TerrainBuilder
extends RefCounted

## Pure builder for the terrain grid and its walkable bitmap.
## Sprint 13.R redesign: heightmap is a composite shape
## (gentle_noise + two Gaussian mountains + sine-curve river trench +
##  village flat overlay), not pure noise. Tile classification is
## height-driven (no more hardcoded river columns).
##
## `world.gd` calls `build_collision_plane`, `build_terrain3d`,
## `build_river_water`, `classify_tiles_from_height`, `build_features`,
## and `make_walkable_map` during setup.
##
## `get_height_at(x, z)` is the single Y-source-of-truth for placing
## objects on the ground. Before `build_terrain3d` has run it returns
## 0.0; afterwards it samples Terrain3DData.

# Terrain codes
const T_GROUND   := 0
const T_FOREST   := 1
const T_SHALLOW  := 2
const T_DEEP     := 3
const T_MOUNTAIN := 4

# Forest zones — layout rules only. `classify_tiles_from_height` promotes
# any FOREST tile whose Gaussian-mountain height exceeds the threshold to
# T_MOUNTAIN, so trees only spawn on the gentle-slope remainder.
const FOREST_A_COLS := [3, 10]   # NW zone (upslope of Mountain 1)
const FOREST_A_ROWS := [1, 6]
const FOREST_B_COLS := [4, 10]   # SW zone (left bank of the river)
const FOREST_B_ROWS := [8, 12]

# ── Terrain3D composite-shape parameters (Sprint 13.R1) ───────────────────────
# Fixed seed keeps the MVP reproducible; post-MVP will randomize.
const TERRAIN_SEED         := 42
const TERRAIN_REGION_SIZE  := 512    # meters per Terrain3DRegion
const TERRAIN_IMAGE_SIZE   := 512    # heightmap pixels (1 px = 1 m)
# Raw heightmap value ∈ [-1, 1] is multiplied by HEIGHT_SCALE to give world
# Y. 20 gives room for 15m mountains and 2m river trenches simultaneously.
const TERRAIN_HEIGHT_SCALE := 20.0

# Base meadow noise — very gentle so mountains and the river read clearly.
const BASE_NOISE_FREQ  := 0.06
const BASE_NOISE_AMP_M := 0.6       # peak-to-peak meters

# Two Gaussian mountains in the N part of the map. Positions are world-space
# meters matching heightmap pixel coordinates (1 px = 1 m).
const MOUNTAIN_1_POS      := Vector2(6.0, 4.0)
const MOUNTAIN_1_HEIGHT_M := 15.0
const MOUNTAIN_1_SIGMA_M  := 7.0
const MOUNTAIN_2_POS      := Vector2(28.0, 4.0)
const MOUNTAIN_2_HEIGHT_M := 12.0
const MOUNTAIN_2_SIGMA_M  := 9.0

# Winding river: centerline x as a function of z runs roughly N→S across the
# map, weaving west of the village (which sits at col 21 ≈ x=42).
# river_x(z) = BASE + AMP * sin(z * FREQ) + SLOPE * z
const RIVER_X_BASE         := 20.0
const RIVER_X_AMP          := 4.0
const RIVER_X_FREQ         := 0.2
const RIVER_X_SLOPE        := 0.15
const RIVER_TRENCH_WIDTH_M := 3.0    # lateral half-width of the trench
const RIVER_TRENCH_DEPTH_M := 2.0    # center depth below surrounding meadow

# Village flat overlay — within FLAT_RADIUS the heightmap is forced to 0 so
# citizens spawn on level ground. FADE_RADIUS linearly blends back to the
# composite shape.
const VILLAGE_FLAT_RADIUS := 10.0
const VILLAGE_FADE_RADIUS := 15.0

# Height-driven tile classification thresholds (Sprint 13.R3)
const MOUNTAIN_HEIGHT_THRESHOLD := 3.0
const DEEP_HEIGHT_THRESHOLD     := -1.2
const SHALLOW_HEIGHT_THRESHOLD  := -0.3

# River water mesh (Sprint 13.R2) — a flat strip threading through the trench.
# Width equals the trench width at mid-depth so water touches both banks.
const RIVER_WATER_WIDTH_M := 3.0
const RIVER_WATER_Y       := -1.0   # midway between trench lip (0) and bottom (-2)

# Held so `get_height_at(x, z)` can sample the live Terrain3D.data.
static var _terrain: Terrain3D = null

# Per-tile T_* cache populated by `classify_tiles_from_height`. Until it
# runs, `get_terrain` falls back to layout-only classification.
static var _tile_cache: PackedByteArray = PackedByteArray()
static var _tile_cache_w: int = 0

## Classify a tile. After `classify_tiles_from_height` runs, returns the
## cached value in O(1); before then, falls through to layout-only rules.
static func get_terrain(col: int, row: int, map_w: int, map_h: int) -> int:
	if _tile_cache_w == map_w and _tile_cache.size() == map_w * map_h:
		return _tile_cache[row * map_w + col]
	return _classify_layout(col, row, map_w, map_h)

## Layout-only classification — borders + forest zones.
## Mountain/river classes now come exclusively from sampled heights in
## `classify_tiles_from_height` (no hardcoded columns).
static func _classify_layout(col: int, row: int, map_w: int, map_h: int) -> int:
	if col == 0 or col == map_w - 1 or row == 0 or row == map_h - 1:
		return T_MOUNTAIN
	if col >= FOREST_A_COLS[0] and col <= FOREST_A_COLS[1] and row >= FOREST_A_ROWS[0] and row <= FOREST_A_ROWS[1]:
		if (col * 17 + row * 31) % 4 < 3:
			return T_FOREST
	if col >= FOREST_B_COLS[0] and col <= FOREST_B_COLS[1] and row >= FOREST_B_ROWS[0] and row <= FOREST_B_ROWS[1]:
		if (col * 13 + row * 23) % 4 < 3:
			return T_FOREST
	return T_GROUND

## Build the per-tile T_* cache from layout + sampled composite heights.
## Must be called after `build_terrain3d` so `get_height_at` is populated.
## GROUND/FOREST tiles whose height falls in a mountain/river band are
## promoted so movement and water behave consistently with the visible
## terrain.
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
				elif h < DEEP_HEIGHT_THRESHOLD:
					t = T_DEEP
				elif h < SHALLOW_HEIGHT_THRESHOLD:
					t = T_SHALLOW
			_tile_cache[row * map_w + col] = t

## Single Y-source-of-truth for placing objects on the ground.
static func get_height_at(x: float, z: float) -> float:
	if _terrain == null:
		return 0.0
	var h: float = _terrain.data.get_height(Vector3(x, 0.0, z))
	return 0.0 if is_nan(h) else h

## Invisible collision plane — flat fallback if Terrain3DCollision fails.
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
## Heightmap = gentle_noise + mountain_1 + mountain_2 + river_trench
## (village_flat_overlay on top).
static func build_terrain3d(parent: Node3D, village_center: Vector3) -> Terrain3D:
	var terrain := Terrain3D.new()
	terrain.name = "Terrain3D"
	terrain.region_size = TERRAIN_REGION_SIZE

	var assets := Terrain3DAssets.new()
	assets.set_texture(0, _create_grass_asset())
	assets.set_texture(1, _create_dirt_asset())
	terrain.assets = assets

	parent.add_child(terrain)

	# Auto-shader blends grass (tex 0) → dirt (tex 1) by slope so Gaussian
	# mountain flanks read as brown ridges against the green meadow.
	# auto_slope=15 matches the ~52° slope produced at Gaussian σ with the
	# current height/sigma (Sprint 13.R4 calibration).
	terrain.material.auto_shader = true
	terrain.material.set_shader_param("auto_slope", 15.0)
	terrain.material.set_shader_param("blend_sharpness", 0.85)

	var img := _generate_heightmap(village_center)
	terrain.data.import_images([img, null, null], Vector3.ZERO, 0.0, TERRAIN_HEIGHT_SCALE)

	# Terrain3D v1.0.1: enum is DISABLED / DYNAMIC_GAME / DYNAMIC_EDITOR
	# / FULL_GAME / FULL_EDITOR. Use DYNAMIC_GAME for runtime collision.
	terrain.collision.mode = Terrain3DCollision.DYNAMIC_GAME

	_terrain = terrain
	return terrain

## Build a blue water strip following the river trench centerline.
## Procedural triangle strip so the mesh hugs the sine curve without
## seams or rotated-plane gaps. `map_length_z` is the playable z-extent
## in world meters.
static func build_river_water(parent: Node3D, map_length_z: float) -> void:
	var container := Node3D.new()
	container.name = "RiverWater"
	parent.add_child(container)

	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)

	var half_w := RIVER_WATER_WIDTH_M * 0.5
	var step := 1.0
	var z := 0.0
	var have_prev := false
	var prev_left := Vector3.ZERO
	var prev_right := Vector3.ZERO

	while z <= map_length_z:
		var rx := _river_x_at(z)
		# Tangent from finite difference, then perpendicular in XZ plane.
		var tangent := Vector2(_river_x_at(z + 0.1) - rx, 0.1).normalized()
		var normal := Vector2(-tangent.y, tangent.x)

		var left := Vector3(rx + normal.x * half_w, RIVER_WATER_Y, z + normal.y * half_w)
		var right := Vector3(rx - normal.x * half_w, RIVER_WATER_Y, z - normal.y * half_w)

		if have_prev:
			st.add_vertex(prev_left)
			st.add_vertex(prev_right)
			st.add_vertex(right)
			st.add_vertex(prev_left)
			st.add_vertex(right)
			st.add_vertex(left)

		prev_left = left
		prev_right = right
		have_prev = true
		z += step

	st.generate_normals()
	var mesh := st.commit()

	var mi := MeshInstance3D.new()
	mi.mesh = mesh
	mi.material_override = _create_water_material()
	container.add_child(mi)

## Build trees per-tile under a new "TerrainFeatures" node.
## Trees spawn only where the composite height stayed in the walkable
## band (mountains/river have already claimed steep/deep tiles).
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

# ── Private shape helpers ─────────────────────────────────────────────────────

## River centerline x at world-space z. Shared between heightmap generation
## and water-mesh tessellation so both follow exactly the same curve.
static func _river_x_at(z: float) -> float:
	return RIVER_X_BASE + RIVER_X_AMP * sin(z * RIVER_X_FREQ) + RIVER_X_SLOPE * z

## Composite heightmap. FORMAT_RF stores raw noise in [-1, 1]; Terrain3D
## multiplies by TERRAIN_HEIGHT_SCALE on import.
static func _generate_heightmap(village_center: Vector3) -> Image:
	var noise := FastNoiseLite.new()
	noise.seed = TERRAIN_SEED
	noise.frequency = BASE_NOISE_FREQ

	var size := TERRAIN_IMAGE_SIZE
	var img := Image.create_empty(size, size, false, Image.FORMAT_RF)

	var vx := village_center.x
	var vz := village_center.z
	var flat_r2 := VILLAGE_FLAT_RADIUS * VILLAGE_FLAT_RADIUS
	var fade_r2 := VILLAGE_FADE_RADIUS * VILLAGE_FADE_RADIUS

	var base_noise_amp_raw := BASE_NOISE_AMP_M / TERRAIN_HEIGHT_SCALE
	var m1_amp_raw := MOUNTAIN_1_HEIGHT_M / TERRAIN_HEIGHT_SCALE
	var m1_two_sigma2 := 2.0 * MOUNTAIN_1_SIGMA_M * MOUNTAIN_1_SIGMA_M
	var m2_amp_raw := MOUNTAIN_2_HEIGHT_M / TERRAIN_HEIGHT_SCALE
	var m2_two_sigma2 := 2.0 * MOUNTAIN_2_SIGMA_M * MOUNTAIN_2_SIGMA_M
	var trench_amp_raw := RIVER_TRENCH_DEPTH_M / TERRAIN_HEIGHT_SCALE

	for y in range(size):
		var wz := float(y)
		var rx := _river_x_at(wz)
		for x in range(size):
			var wx := float(x)

			# Gentle meadow base.
			var h := noise.get_noise_2d(wx, wz) * base_noise_amp_raw

			# Mountain 1 (Gaussian bump)
			var dx1 := wx - MOUNTAIN_1_POS.x
			var dz1 := wz - MOUNTAIN_1_POS.y
			h += m1_amp_raw * exp(-(dx1 * dx1 + dz1 * dz1) / m1_two_sigma2)

			# Mountain 2
			var dx2 := wx - MOUNTAIN_2_POS.x
			var dz2 := wz - MOUNTAIN_2_POS.y
			h += m2_amp_raw * exp(-(dx2 * dx2 + dz2 * dz2) / m2_two_sigma2)

			# River trench — triangular profile, deepest at centerline.
			var river_dist := absf(wx - rx)
			if river_dist < RIVER_TRENCH_WIDTH_M:
				h -= (1.0 - river_dist / RIVER_TRENCH_WIDTH_M) * trench_amp_raw

			# Village flat overlay (applied last so it wins over shape+noise).
			var vdx := wx - vx
			var vdz := wz - vz
			var d2 := vdx * vdx + vdz * vdz
			if d2 <= flat_r2:
				h = 0.0
			elif d2 < fade_r2:
				var t := (sqrt(d2) - VILLAGE_FLAT_RADIUS) / (VILLAGE_FADE_RADIUS - VILLAGE_FLAT_RADIUS)
				h *= t

			img.set_pixel(x, y, Color(clampf(h, -1.0, 1.0), 0.0, 0.0, 1.0))
	return img

# ── Private material / asset factories ───────────────────────────────────────

static func _create_grass_asset() -> Terrain3DTextureAsset:
	var img := Image.create_empty(64, 64, false, Image.FORMAT_RGBA8)
	img.fill(Color(0.26, 0.52, 0.26, 1.0))
	var tex := ImageTexture.create_from_image(img)
	var ta := Terrain3DTextureAsset.new()
	ta.name = "Grass"
	ta.albedo_texture = tex
	ta.uv_scale = 0.5
	return ta

static func _create_dirt_asset() -> Terrain3DTextureAsset:
	var img := Image.create_empty(64, 64, false, Image.FORMAT_RGBA8)
	img.fill(Color(0.42, 0.30, 0.18, 1.0))
	var tex := ImageTexture.create_from_image(img)
	var ta := Terrain3DTextureAsset.new()
	ta.name = "Dirt"
	ta.albedo_texture = tex
	ta.uv_scale = 0.5
	return ta

static func _create_water_material() -> StandardMaterial3D:
	var mat := StandardMaterial3D.new()
	mat.albedo_color = Color(0.25, 0.45, 0.75, 0.85)
	mat.metallic = 0.2
	mat.roughness = 0.35
	mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	return mat

# ── Private feature builders ─────────────────────────────────────────────────

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
