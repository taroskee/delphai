extends Node2D

const TILE_PX    := 16
const TILE_SCALE := 3
const MAP_COLS   := 24
const MAP_ROWS   := 14

# Tile type constants
const G := 0  # Grass
const W := 1  # Water
const D := 2  # Dirt road
const B := 3  # Bridge (road over water)

# Verified atlas coordinates — terrain.png (16px grid)
const GRASS_ATLAS  := Vector2i(3, 2)  # pixel (56, 40)  = green  ✓
const DIRT_ATLAS   := Vector2i(6, 4)  # pixel (104, 72) = brown  ✓
const BRIDGE_ATLAS := Vector2i(2, 4)  # pixel (40, 72)  = sandy  ✓

# Verified atlas coordinate — water.png (16px grid)
const WATER_ATLAS  := Vector2i(2, 3)  # pixel (40, 56)  = blue   ✓

# Tree sprite regions in outside.png (native pixel Rect2, displayed at 1.5x scale)
const TREE_REGIONS: Array[Rect2] = [
	Rect2(544,  32, 80, 48),   # round canopy  (cols 34-38, rows 2-4)
	Rect2(640, 160, 64, 64),   # square clump  (cols 40-43, rows 10-13)
	Rect2(288, 128, 48, 64),   # small tree    (cols 18-20, rows 8-11)
]

# Tile positions that must stay tree-free (campfire + citizens ± 2 tile margin)
const AVOID_TILES: Array[Vector2i] = [
	Vector2i(12, 8), Vector2i(12, 9),   # campfire / Elder
	Vector2i( 7, 8), Vector2i( 7, 9),   # Kael
	Vector2i(17, 8), Vector2i(17, 9),   # Hara
	Vector2i(11, 8), Vector2i(13, 8),   # campfire neighbors
	Vector2i( 6, 8), Vector2i( 8, 8),   # Kael neighbors
	Vector2i(16, 8), Vector2i(18, 8),   # Hara neighbors
]

# River: cols 2-4, Road: rows 5-6, Bridge: intersection
const MAP_LAYOUT: Array = [
	[G,G,W,W,W,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G],  # row  0
	[G,G,W,W,W,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G],  # row  1
	[G,G,W,W,W,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G],  # row  2
	[G,G,W,W,W,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G],  # row  3
	[G,G,W,W,W,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G],  # row  4
	[D,D,B,B,B,D,D,D,D,D,D,D,D,D,D,D,D,D,D,D,D,D,D,D],  # row  5 road
	[D,D,B,B,B,D,D,D,D,D,D,D,D,D,D,D,D,D,D,D,D,D,D,D],  # row  6 road
	[G,G,W,W,W,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G],  # row  7
	[G,G,W,W,W,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G],  # row  8 campfire
	[G,G,W,W,W,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G],  # row  9
	[G,G,W,W,W,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G],  # row 10
	[G,G,W,W,W,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G],  # row 11
	[G,G,W,W,W,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G],  # row 12
	[G,G,W,W,W,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G,G],  # row 13
]

func _ready() -> void:
	_build_tilemap()
	_scatter_trees()

## Return a flat PackedByteArray (row-major) where 1=walkable, 0=blocked.
## Water tiles (W) are blocked; grass, dirt, and bridge are walkable.
func get_walkable_data() -> PackedByteArray:
	var data := PackedByteArray()
	data.resize(MAP_COLS * MAP_ROWS)
	for row in range(MAP_ROWS):
		for col in range(MAP_COLS):
			var cell_type: int = MAP_LAYOUT[row][col]
			data[row * MAP_COLS + col] = 0 if cell_type == W else 1
	return data

# ---------------------------------------------------------------------------
# TileMap ground layer
# ---------------------------------------------------------------------------

func _build_tilemap() -> void:
	var terrain_tex := load("res://assets/tile/TILESETS/terrain.png") as Texture2D
	var water_tex   := load("res://assets/tile/TILESETS/water.png")   as Texture2D
	if not terrain_tex or not water_tex:
		push_error("[map.gd] Failed to load tileset textures")
		return

	var tile_size := Vector2i(TILE_PX, TILE_PX)

	# --- terrain source (grass / dirt / bridge) ---
	var terrain_src := TileSetAtlasSource.new()
	terrain_src.texture = terrain_tex
	terrain_src.texture_region_size = tile_size
	terrain_src.create_tile(GRASS_ATLAS)
	terrain_src.create_tile(DIRT_ATLAS)
	terrain_src.create_tile(BRIDGE_ATLAS)

	# --- water source ---
	var water_src := TileSetAtlasSource.new()
	water_src.texture = water_tex
	water_src.texture_region_size = tile_size
	water_src.create_tile(WATER_ATLAS)

	var tile_set := TileSet.new()
	tile_set.tile_size = tile_size
	var terrain_id: int = tile_set.add_source(terrain_src)
	var water_id: int   = tile_set.add_source(water_src)

	var layer := TileMapLayer.new()
	layer.tile_set = tile_set
	layer.scale = Vector2(TILE_SCALE, TILE_SCALE)
	layer.position = Vector2.ZERO
	add_child(layer)

	# Fill tiles according to MAP_LAYOUT
	for row in range(MAP_ROWS):
		for col in range(MAP_COLS):
			var cell_type: int = MAP_LAYOUT[row][col]
			var coords := Vector2i(col, row)
			match cell_type:
				G: layer.set_cell(coords, terrain_id, GRASS_ATLAS)
				W: layer.set_cell(coords, water_id,   WATER_ATLAS)
				D: layer.set_cell(coords, terrain_id, DIRT_ATLAS)
				B: layer.set_cell(coords, terrain_id, BRIDGE_ATLAS)

# ---------------------------------------------------------------------------
# Tree scattering (Sprite2D regions from outside.png)
# ---------------------------------------------------------------------------

func _scatter_trees() -> void:
	var outside_tex := load("res://assets/tile/TILESETS/outside.png") as Texture2D
	if not outside_tex:
		push_error("[map.gd] Failed to load outside.png for trees")
		return

	var rng := RandomNumberGenerator.new()
	rng.seed = 42  # deterministic layout

	# Candidate grass tiles, excluding road rows, water cols, and avoid tiles
	var candidates: Array[Vector2i] = []
	for row in range(MAP_ROWS):
		if row == 5 or row == 6:
			continue  # skip road rows
		for col in range(MAP_COLS):
			if col >= 2 and col <= 4:
				continue  # skip river cols
			var tile_pos := Vector2i(col, row)
			if tile_pos in AVOID_TILES:
				continue
			candidates.append(tile_pos)

	# Place ~35 trees, pick random candidates without repeats
	candidates.shuffle()  # uses engine RNG but determinism via seed is best-effort in GDScript
	var tree_count: int = mini(35, candidates.size())

	for i in range(tree_count):
		var tc := candidates[i]
		var region: Rect2 = TREE_REGIONS[rng.randi() % TREE_REGIONS.size()]

		var sprite := Sprite2D.new()
		sprite.texture = outside_tex
		sprite.region_enabled = true
		sprite.region_rect = region
		sprite.scale = Vector2(1.5, 1.5)

		# World position: tile center in TileMapLayer space × TILE_SCALE
		# TileMapLayer is at (0,0) with scale (3,3), so world_x = col * 16 * 3
		var world_x := (tc.x * TILE_PX + TILE_PX / 2) * TILE_SCALE
		var world_y := (tc.y * TILE_PX + TILE_PX / 2) * TILE_SCALE - region.size.y * 1.5 * 0.4
		sprite.position = Vector2(world_x, world_y)

		add_child(sprite)
