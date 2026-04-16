extends Node3D

## Main 3D scene: drives Rust simulation, builds world geometry, syncs citizens each tick.

const TILE_SIZE  := 1.0
const MAP_WIDTH  := 24
const MAP_HEIGHT := 14
const TICK_RATE  := 4.0   # ticks per second

var _world_sim: WorldNode
var _citizen_nodes: Array = []
var _tick_acc: float = 0.0

# ── Lifecycle ─────────────────────────────────────────────────────────────────

func _ready() -> void:
	_world_sim = WorldNode.new()
	_world_sim.name = "WorldSim"
	add_child(_world_sim)
	_world_sim.initialize()

	_build_environment()
	_build_lighting()
	_build_camera()
	_build_terrain()
	_send_walkable_map()
	_build_resources()
	_build_citizens()

func _process(delta: float) -> void:
	_tick_acc += delta
	if _tick_acc >= 1.0 / TICK_RATE:
		_tick_acc -= 1.0 / TICK_RATE
		_world_sim.tick(randf())
		_update_citizens()

# ── Coordinate helpers ────────────────────────────────────────────────────────

func tile_to_world(col: int, row: int) -> Vector3:
	return Vector3(col * TILE_SIZE, 0.0, row * TILE_SIZE)

func _map_center() -> Vector3:
	return Vector3((MAP_WIDTH - 1) * 0.5 * TILE_SIZE, 0.0, (MAP_HEIGHT - 1) * 0.5 * TILE_SIZE)

# ── Scene builders ────────────────────────────────────────────────────────────

func _build_environment() -> void:
	var env_node := WorldEnvironment.new()
	env_node.name = "WorldEnvironment"
	var env := Environment.new()
	env.background_mode = Environment.BG_SKY
	var sky := Sky.new()
	sky.sky_material = ProceduralSkyMaterial.new()
	env.sky = sky
	env.ambient_light_source = Environment.AMBIENT_SOURCE_SKY
	env.ambient_light_energy = 0.5
	env_node.environment = env
	add_child(env_node)

func _build_lighting() -> void:
	var sun := DirectionalLight3D.new()
	sun.name = "Sun"
	sun.rotation_degrees = Vector3(-50.0, 35.0, 0.0)
	sun.light_energy = 1.2
	sun.shadow_enabled = true
	add_child(sun)

func _build_camera() -> void:
	var cam := Camera3D.new()
	cam.name = "Camera3D"
	var c := _map_center()
	cam.position = Vector3(c.x, 18.0, c.z + 10.0)
	cam.rotation_degrees = Vector3(-55.0, 0.0, 0.0)
	add_child(cam)

func _build_terrain() -> void:
	var body := StaticBody3D.new()
	body.name = "Terrain"
	add_child(body)

	var mesh_inst := MeshInstance3D.new()
	var plane := PlaneMesh.new()
	plane.size = Vector2(MAP_WIDTH * TILE_SIZE, MAP_HEIGHT * TILE_SIZE)
	mesh_inst.mesh = plane
	var mat := StandardMaterial3D.new()
	mat.albedo_color = Color(0.33, 0.52, 0.22)  # grass
	mesh_inst.material_override = mat
	mesh_inst.position = _map_center()
	body.add_child(mesh_inst)

	var col_shape := CollisionShape3D.new()
	var box := BoxShape3D.new()
	box.size = Vector3(MAP_WIDTH * TILE_SIZE, 0.1, MAP_HEIGHT * TILE_SIZE)
	col_shape.shape = box
	col_shape.position = _map_center()
	body.add_child(col_shape)

func _send_walkable_map() -> void:
	var data := PackedByteArray()
	data.resize(MAP_WIDTH * MAP_HEIGHT)
	data.fill(1)  # all tiles walkable for now
	_world_sim.set_walkable_map(data, MAP_WIDTH, MAP_HEIGHT)

func _build_resources() -> void:
	var container := Node3D.new()
	container.name = "Resources"
	add_child(container)
	var count: int = _world_sim.get_resource_count()
	for i in range(count):
		var tile: Vector2i = _world_sim.get_resource_pos(i)
		var kind: String   = _world_sim.get_resource_kind(i)
		container.add_child(_make_resource(kind, tile.x, tile.y))

func _make_resource(kind: String, col: int, row: int) -> Node3D:
	var root := Node3D.new()
	root.name = "Resource_" + kind
	var mesh_inst := MeshInstance3D.new()
	var mat := StandardMaterial3D.new()

	if kind == "berry_bush":
		var sphere := SphereMesh.new()
		sphere.radius = 0.4
		sphere.height = 0.8
		mesh_inst.mesh = sphere
		mat.albedo_color = Color(0.08, 0.55, 0.08)
		root.position = tile_to_world(col, row) + Vector3(0, 0.4, 0)
	else:  # water_source
		var cyl := CylinderMesh.new()
		cyl.top_radius    = 0.5
		cyl.bottom_radius = 0.5
		cyl.height        = 0.12
		mesh_inst.mesh = cyl
		mat.albedo_color = Color(0.1, 0.45, 0.95)
		root.position = tile_to_world(col, row) + Vector3(0, 0.06, 0)

	mesh_inst.material_override = mat
	root.add_child(mesh_inst)
	return root

func _build_citizens() -> void:
	var container := Node3D.new()
	container.name = "Citizens"
	add_child(container)
	var count: int = _world_sim.get_citizen_count()
	for i in range(count):
		var cname: String = _world_sim.get_citizen_name(i)
		var node := _make_citizen(cname, i)
		container.add_child(node)
		_citizen_nodes.append(node)
		_sync_citizen_pos(i)

func _make_citizen(cname: String, idx: int) -> CharacterBody3D:
	var body := CharacterBody3D.new()
	body.name = "Citizen_%d" % idx

	# Capsule visual
	var mesh_inst := MeshInstance3D.new()
	var cap := CapsuleMesh.new()
	cap.radius = 0.2
	cap.height = 0.8
	mesh_inst.mesh = cap
	mesh_inst.position.y = 0.5
	var mat := StandardMaterial3D.new()
	mat.albedo_color = Color(0.85, 0.65, 0.35)
	mesh_inst.material_override = mat
	body.add_child(mesh_inst)
	body.set_meta("mat", mat)

	# Collision shape
	var col := CollisionShape3D.new()
	var cap_shape := CapsuleShape3D.new()
	cap_shape.radius = 0.2
	cap_shape.height = 0.4
	col.shape = cap_shape
	col.position.y = 0.5
	body.add_child(col)

	# Name label (billboard)
	var name_lbl := Label3D.new()
	name_lbl.text = cname
	name_lbl.font_size = 28
	name_lbl.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	name_lbl.position.y = 1.15
	body.add_child(name_lbl)

	# Behavior label (billboard)
	var beh_lbl := Label3D.new()
	beh_lbl.text = "idle"
	beh_lbl.font_size = 22
	beh_lbl.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	beh_lbl.position.y = 1.4
	beh_lbl.modulate = Color(0.9, 0.9, 0.9)
	body.add_child(beh_lbl)
	body.set_meta("beh_lbl", beh_lbl)

	return body

# ── Per-tick updates ──────────────────────────────────────────────────────────

func _sync_citizen_pos(idx: int) -> void:
	var tile: Vector2i = _world_sim.get_citizen_tile_pos(idx)
	_citizen_nodes[idx].position = tile_to_world(tile.x, tile.y)

func _update_citizens() -> void:
	for i in range(_citizen_nodes.size()):
		_sync_citizen_pos(i)
		var beh: String  = _world_sim.get_citizen_behavior(i)
		var fed: float   = _world_sim.get_citizen_fed(i)
		var hyd: float   = _world_sim.get_citizen_hydration(i)

		var beh_lbl: Label3D = _citizen_nodes[i].get_meta("beh_lbl")
		beh_lbl.text     = beh
		beh_lbl.modulate = _behavior_color(beh)

		var mat: StandardMaterial3D = _citizen_nodes[i].get_meta("mat")
		if hyd < 0.3:
			mat.albedo_color = Color(0.3, 0.5, 1.0)    # blue  = thirsty
		elif fed < 0.3:
			mat.albedo_color = Color(1.0, 0.5, 0.15)   # orange = hungry
		else:
			mat.albedo_color = Color(0.85, 0.65, 0.35) # normal

func _behavior_color(b: String) -> Color:
	match b:
		"idle":          return Color(1.0, 1.0, 1.0)
		"seeking_food":  return Color(1.0, 0.6, 0.0)
		"gathering":     return Color(0.2, 0.9, 0.2)
		"seeking_water": return Color(0.3, 0.6, 1.0)
		"drinking":      return Color(0.5, 0.85, 1.0)
	return Color(0.7, 0.7, 0.7)
