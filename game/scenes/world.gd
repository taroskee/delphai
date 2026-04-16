extends Node3D

## Main 3D scene: drives Rust simulation, builds world geometry, syncs citizens each tick.

const TILE_SIZE    := 1.0
const MAP_WIDTH    := 24
const MAP_HEIGHT   := 14
const TICK_RATE    := 4.0    # ticks per second
const DAY_TICKS    := 600    # one full day cycle in ticks (~2.5 min at 4 Hz)
const CAM_ZOOM_MIN := 6.0
const CAM_ZOOM_MAX := 30.0
const CAM_PAN_SPEED := 0.025

var _world_sim: WorldNode
var _citizen_nodes: Array     = []
var _citizen_behaviors: Array = []  # cached behavior string per citizen
var _resource_meshes: Array   = []  # MeshInstance3D per resource (for scale updates)
var _fed_bars: Array          = []
var _hyd_bars: Array          = []
var _tick_acc: float          = 0.0
var _gather_time: float       = 0.0

# Tech UI
var _tech_bar: ProgressBar    = null
var _tech_lbl: Label          = null   # "Stone Tools  12 / 50"

# Camera rig
var _cam: Camera3D            = null
var _cam_dragging: bool       = false
var _cam_drag_start: Vector2  = Vector2.ZERO
var _cam_pos_start: Vector3   = Vector3.ZERO

# Day/night
var _sun: DirectionalLight3D  = null

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
	_build_debug_ui()
	_build_tech_ui()
	_build_bgm()

func _input(event: InputEvent) -> void:
	if _cam == null:
		return
	if event is InputEventMouseButton:
		var btn := event as InputEventMouseButton
		if btn.button_index == MOUSE_BUTTON_MIDDLE:
			_cam_dragging = btn.pressed
			if btn.pressed:
				_cam_drag_start = btn.position
				_cam_pos_start  = _cam.position
		elif btn.button_index == MOUSE_BUTTON_WHEEL_UP:
			_cam.position.y = clampf(_cam.position.y - 1.5, CAM_ZOOM_MIN, CAM_ZOOM_MAX)
		elif btn.button_index == MOUSE_BUTTON_WHEEL_DOWN:
			_cam.position.y = clampf(_cam.position.y + 1.5, CAM_ZOOM_MIN, CAM_ZOOM_MAX)
	elif event is InputEventMouseMotion and _cam_dragging:
		var motion := event as InputEventMouseMotion
		var delta_px := motion.position - _cam_drag_start
		# Scale pan by current zoom height so speed feels consistent
		var zoom_scale := _cam.position.y / 18.0
		_cam.position.x = _cam_pos_start.x - delta_px.x * CAM_PAN_SPEED * zoom_scale
		_cam.position.z = _cam_pos_start.z - delta_px.y * CAM_PAN_SPEED * zoom_scale

func _process(delta: float) -> void:
	_tick_acc += delta
	if _tick_acc >= 1.0 / TICK_RATE:
		_tick_acc -= 1.0 / TICK_RATE
		_world_sim.tick(randf())
		_update_citizens()

	_gather_time += delta
	_animate_gathering()

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
	_sun = DirectionalLight3D.new()
	_sun.name = "Sun"
	_sun.rotation_degrees = Vector3(-50.0, 35.0, 0.0)
	_sun.light_energy = 1.2
	_sun.shadow_enabled = true
	add_child(_sun)

func _build_camera() -> void:
	_cam = Camera3D.new()
	_cam.name = "Camera3D"
	var c := _map_center()
	_cam.position = Vector3(c.x, 18.0, c.z + 10.0)
	_cam.rotation_degrees = Vector3(-55.0, 0.0, 0.0)
	add_child(_cam)

func _build_bgm() -> void:
	var player := AudioStreamPlayer.new()
	player.name = "BGM"
	var stream := load("res://assets/sfx/Ambience/CampAmbience.mp3")
	if stream:
		player.stream = stream
		player.volume_db = -8.0
		player.autoplay = true
	add_child(player)

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
		var rnode := _make_resource(kind, tile.x, tile.y)
		container.add_child(rnode)
		_resource_meshes.append(rnode.get_meta("mesh_inst"))

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
	root.set_meta("mesh_inst", mesh_inst)
	root.set_meta("kind", kind)
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
		_citizen_behaviors.append("idle")
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

	# Head sphere (simple humanoid silhouette)
	var head_inst := MeshInstance3D.new()
	var head_sphere := SphereMesh.new()
	head_sphere.radius = 0.14
	head_sphere.height = 0.28
	head_inst.mesh = head_sphere
	head_inst.position.y = 1.0
	head_inst.material_override = mat
	body.add_child(head_inst)

	# Collision shape
	var col := CollisionShape3D.new()
	var cap_shape := CapsuleShape3D.new()
	cap_shape.radius = 0.2
	cap_shape.height = 0.4
	col.shape = cap_shape
	col.position.y = 0.5
	body.add_child(col)

	# Name label (billboard) — above head sphere (head top at ~y=1.14)
	var name_lbl := Label3D.new()
	name_lbl.text = cname
	name_lbl.font_size = 28
	name_lbl.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	name_lbl.position.y = 1.35
	body.add_child(name_lbl)

	# Behavior label (billboard)
	var beh_lbl := Label3D.new()
	beh_lbl.text = "idle"
	beh_lbl.font_size = 22
	beh_lbl.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	beh_lbl.position.y = 1.6
	beh_lbl.modulate = Color(0.9, 0.9, 0.9)
	body.add_child(beh_lbl)
	body.set_meta("beh_lbl", beh_lbl)

	return body

func _build_debug_ui() -> void:
	var layer := CanvasLayer.new()
	layer.name = "DebugUI"
	add_child(layer)

	var panel := VBoxContainer.new()
	panel.name = "CitizenPanel"
	panel.position = Vector2(10.0, 10.0)
	layer.add_child(panel)

	var count := _world_sim.get_citizen_count()
	for i in range(count):
		var cname: String = _world_sim.get_citizen_name(i)
		var row := HBoxContainer.new()
		row.add_theme_constant_override("separation", 6)
		panel.add_child(row)

		var lbl := Label.new()
		lbl.text = cname
		lbl.custom_minimum_size = Vector2(64, 0)
		row.add_child(lbl)

		# Food bar (orange)
		var fed_bar := ProgressBar.new()
		fed_bar.min_value = 0.0
		fed_bar.max_value = 1.0
		fed_bar.value     = 1.0
		fed_bar.custom_minimum_size = Vector2(80, 18)
		fed_bar.modulate  = Color(1.0, 0.55, 0.1)
		fed_bar.show_percentage = false
		row.add_child(fed_bar)
		_fed_bars.append(fed_bar)

		# Water bar (blue)
		var hyd_bar := ProgressBar.new()
		hyd_bar.min_value = 0.0
		hyd_bar.max_value = 1.0
		hyd_bar.value     = 1.0
		hyd_bar.custom_minimum_size = Vector2(80, 18)
		hyd_bar.modulate  = Color(0.3, 0.6, 1.0)
		hyd_bar.show_percentage = false
		row.add_child(hyd_bar)
		_hyd_bars.append(hyd_bar)

func _build_tech_ui() -> void:
	var layer := CanvasLayer.new()
	layer.name = "TechUI"
	add_child(layer)

	var panel := VBoxContainer.new()
	panel.name = "TechPanel"
	panel.set_anchor_and_offset(SIDE_RIGHT, 1.0, -210.0)
	panel.set_anchor_and_offset(SIDE_TOP, 0.0, 10.0)
	layer.add_child(panel)

	var title := Label.new()
	title.text = "Research"
	title.add_theme_color_override("font_color", Color(1.0, 0.9, 0.5))
	panel.add_child(title)

	_tech_lbl = Label.new()
	_tech_lbl.text = "—"
	panel.add_child(_tech_lbl)

	_tech_bar = ProgressBar.new()
	_tech_bar.min_value = 0.0
	_tech_bar.max_value = 1.0
	_tech_bar.value = 0.0
	_tech_bar.custom_minimum_size = Vector2(200, 20)
	_tech_bar.modulate = Color(1.0, 0.85, 0.2)
	_tech_bar.show_percentage = false
	panel.add_child(_tech_bar)

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

		_citizen_behaviors[i] = beh

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

		_fed_bars[i].value = fed
		_hyd_bars[i].value = hyd

	_update_resources()
	_update_day_night()
	_update_tech_ui()

func _update_day_night() -> void:
	var tick: int = _world_sim.get_tick_count()
	var progress := float(tick % DAY_TICKS) / float(DAY_TICKS)  # 0.0 → 1.0 per day
	# Sun arcs from east (dawn) overhead (noon) to west (dusk): X goes -10° → -80° → -170°
	var angle_x := -10.0 - 340.0 * progress
	_sun.rotation_degrees.x = angle_x
	# Brightness peaks at noon (progress ≈ 0.22), dim at night
	var noon := 1.0 - absf(progress - 0.22) * 5.0
	_sun.light_energy = lerpf(0.05, 1.4, clampf(noon, 0.0, 1.0))

func _update_tech_ui() -> void:
	if _tech_lbl == null or _tech_bar == null:
		return
	var pts: int    = _world_sim.get_research_points()
	var name: String = _world_sim.get_next_tech_name()
	var req: int    = _world_sim.get_next_tech_required()
	if name == "":
		_tech_lbl.text  = "All techs unlocked!"
		_tech_bar.value = 1.0
	else:
		_tech_lbl.text  = "%s  %d / %d" % [name.replace("_", " ").capitalize(), pts, req]
		_tech_bar.max_value = float(req) if req > 0 else 1.0
		_tech_bar.value     = float(pts)

func _update_resources() -> void:
	var count: int = _world_sim.get_resource_count()
	for i in range(count):
		var qty: float = _world_sim.get_resource_quantity(i)
		# berry_bush shrinks as it depletes; water_source stays full
		var s := maxf(0.15, qty)
		_resource_meshes[i].scale = Vector3(s, s, s)

func _animate_gathering() -> void:
	for i in range(_citizen_nodes.size()):
		if _citizen_behaviors[i] == "gathering":
			var pulse := 1.0 + 0.12 * sin(_gather_time * 8.0)
			_citizen_nodes[i].scale = Vector3(pulse, pulse, pulse)
		else:
			_citizen_nodes[i].scale = Vector3.ONE

func _behavior_color(b: String) -> Color:
	match b:
		"idle":          return Color(1.0, 1.0, 1.0)
		"seeking_food":  return Color(1.0, 0.6, 0.0)
		"gathering":     return Color(0.2, 0.9, 0.2)
		"seeking_water": return Color(0.3, 0.6, 1.0)
		"drinking":      return Color(0.5, 0.85, 1.0)
	return Color(0.7, 0.7, 0.7)
