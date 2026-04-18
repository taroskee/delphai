extends Node3D

## Thin orchestrator: drives the Rust simulation and wires together the helper
## builders (TerrainBuilder, CitizenFactory, AnimalFactory, ResourceFactory,
## DebugHud) each tick. Heavy construction logic lives in `game/scripts/*.gd`.

const TILE_SIZE    := 2.0
const MAP_WIDTH    := 24
const MAP_HEIGHT   := 14
const TICK_RATE    := 4.0    # ticks per second
const DAY_TICKS    := 600    # one full day cycle in ticks (~2.5 min at 4 Hz)
const CAM_ZOOM_MIN := 10.0
const CAM_ZOOM_MAX := 200.0
const CAM_ZOOM_DEFAULT := 22.0
const CAM_PAN_SPEED := 0.05
const CAM_KEY_PAN_SPEED := 24.0    # world units per second
const CAM_KEY_ZOOM_SPEED := 20.0   # world units per second
const CAM_WHEEL_ZOOM_STEP := 2.5
const CAM_PINCH_ZOOM_GAIN := 18.0
const CAM_TRACKPAD_PAN_GAIN := 0.05
# Velocity-based drag acceleration: slow drag = 1x, fast drag (>= REF_PX px/s) = MAX x.
const CAM_DRAG_ACCEL_MAX    := 4.0
const CAM_DRAG_ACCEL_REF_PX := 1500.0
# Keyboard pan acceleration: hold-time ramps multiplier up to MAX at RAMP per second.
const CAM_KEY_ACCEL_MAX  := 3.0
const CAM_KEY_ACCEL_RAMP := 1.5

# Village center tile — matches the citizen cluster spawn in
# `crates/delphai-gdext/src/lib.rs::initialize`. Camera and landmarks focus here.
const VILLAGE_CENTER_COL := 10
const VILLAGE_CENTER_ROW := 8

const CITIZEN_LERP_SPEED := 8.0
const ANIMAL_LERP_SPEED  := 6.0

# Citizen chat bubble lines (Japanese) keyed by behavior state
const CHAT_LINES := {
	"idle":          ["のんびり...", "いい天気だ", "休憩しよう", "静かだな", "平和だな"],
	"seeking_food":  ["お腹すいた！", "何か食べ物を...", "腹ペコだ...", "食料が必要だ"],
	"gathering":     ["いただき！", "美味しそう", "収穫だ！", "食料を集める"],
	"seeking_water": ["喉が渇いた！", "水が欲しい...", "水源を探せ", "水..."],
	"drinking":      ["うまい！", "生き返る...", "水は命だ", "ふぅ..."],
	"hunting":       ["みんなで狩ろう！", "獲物を追え！", "協力だ！", "行くぞ！"],
}

var _world_sim: WorldNode
var _citizen_nodes: Array     = []
var _citizen_behaviors: Array = []
var _resource_meshes: Array   = []
var _fed_bars: Array          = []
var _hyd_bars: Array          = []
var _tick_acc: float          = 0.0
var _gather_time: float       = 0.0

var _chat_gap_timers: Array   = []
var _chat_show_timers: Array  = []

var _animal_nodes: Array      = []

var _citizen_target_pos: Array = []
var _animal_target_pos: Array  = []

var _notify_lbl: Label        = null
var _notify_timer: float      = 0.0

var _debug_panel: VBoxContainer = null

var _cam: Camera3D            = null
var _cam_dragging: bool       = false
var _key_pan_hold: float      = 0.0

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
	TerrainBuilder.build_ground(self, MAP_WIDTH, MAP_HEIGHT, TILE_SIZE)
	TerrainBuilder.build_features(self, MAP_WIDTH, MAP_HEIGHT, TILE_SIZE)
	_world_sim.set_walkable_map(TerrainBuilder.make_walkable_map(MAP_WIDTH, MAP_HEIGHT), MAP_WIDTH, MAP_HEIGHT)
	VillageBuilder.add_campfire(self, tile_to_world(VILLAGE_CENTER_COL, VILLAGE_CENTER_ROW))
	_build_resources()
	_build_citizens()
	_build_animals()
	_build_debug_ui()
	_build_notify_label()
	_build_bgm()

func _input(event: InputEvent) -> void:
	if _cam == null:
		return
	if event is InputEventMouseButton:
		_handle_mouse_button(event as InputEventMouseButton)
	elif event is InputEventMouseMotion and _cam_dragging:
		_handle_mouse_drag(event as InputEventMouseMotion)
	elif event is InputEventMagnifyGesture:
		_zoom_by((1.0 - (event as InputEventMagnifyGesture).factor) * CAM_PINCH_ZOOM_GAIN)
	elif event is InputEventPanGesture:
		_pan_by_screen_delta((event as InputEventPanGesture).delta * CAM_TRACKPAD_PAN_GAIN / CAM_PAN_SPEED)

func _handle_mouse_button(btn: InputEventMouseButton) -> void:
	match btn.button_index:
		MOUSE_BUTTON_MIDDLE, MOUSE_BUTTON_RIGHT:
			_cam_dragging = btn.pressed
		MOUSE_BUTTON_WHEEL_UP:
			_zoom_by(-CAM_WHEEL_ZOOM_STEP)
		MOUSE_BUTTON_WHEEL_DOWN:
			_zoom_by(CAM_WHEEL_ZOOM_STEP)

## Incremental drag: each event contributes `motion.relative` pixels, scaled by
## zoom and by a velocity-based acceleration so fast flicks move further than
## the raw pixel count would suggest.
func _handle_mouse_drag(motion: InputEventMouseMotion) -> void:
	var zoom_scale := _cam.position.y / 36.0
	var speed_px := motion.velocity.length()
	var accel := 1.0 + clampf(speed_px / CAM_DRAG_ACCEL_REF_PX, 0.0, CAM_DRAG_ACCEL_MAX - 1.0)
	_cam.position.x -= motion.relative.x * CAM_PAN_SPEED * zoom_scale * accel
	_cam.position.z -= motion.relative.y * CAM_PAN_SPEED * zoom_scale * accel

func _pan_by_screen_delta(delta_px: Vector2) -> void:
	var zoom_scale := _cam.position.y / 36.0
	_cam.position.x -= delta_px.x * CAM_PAN_SPEED * zoom_scale
	_cam.position.z -= delta_px.y * CAM_PAN_SPEED * zoom_scale

func _zoom_by(amount: float) -> void:
	_cam.position.y = clampf(_cam.position.y + amount, CAM_ZOOM_MIN, CAM_ZOOM_MAX)

func _process_keyboard_camera(delta: float) -> void:
	if _cam == null:
		return
	var pan_x := 0.0
	var pan_z := 0.0
	if Input.is_key_pressed(KEY_W) or Input.is_key_pressed(KEY_UP):
		pan_z -= 1.0
	if Input.is_key_pressed(KEY_S) or Input.is_key_pressed(KEY_DOWN):
		pan_z += 1.0
	if Input.is_key_pressed(KEY_A) or Input.is_key_pressed(KEY_LEFT):
		pan_x -= 1.0
	if Input.is_key_pressed(KEY_D) or Input.is_key_pressed(KEY_RIGHT):
		pan_x += 1.0
	if pan_x != 0.0 or pan_z != 0.0:
		_key_pan_hold += delta
		var accel := minf(1.0 + _key_pan_hold * CAM_KEY_ACCEL_RAMP, CAM_KEY_ACCEL_MAX)
		var zoom_scale := _cam.position.y / 36.0
		_cam.position.x += pan_x * CAM_KEY_PAN_SPEED * zoom_scale * delta * accel
		_cam.position.z += pan_z * CAM_KEY_PAN_SPEED * zoom_scale * delta * accel
	else:
		_key_pan_hold = 0.0
	if Input.is_key_pressed(KEY_EQUAL) or Input.is_key_pressed(KEY_PLUS):
		_zoom_by(-CAM_KEY_ZOOM_SPEED * delta)
	if Input.is_key_pressed(KEY_MINUS):
		_zoom_by(CAM_KEY_ZOOM_SPEED * delta)

func _process(delta: float) -> void:
	_process_keyboard_camera(delta)
	_tick_acc += delta
	if _tick_acc >= 1.0 / TICK_RATE:
		_tick_acc -= 1.0 / TICK_RATE
		_world_sim.tick(randf())
		_update_citizens()
		_check_births()

	var c_lerp := clampf(CITIZEN_LERP_SPEED * delta, 0.0, 1.0)
	for i in range(_citizen_nodes.size()):
		_citizen_nodes[i].position = _citizen_nodes[i].position.lerp(_citizen_target_pos[i], c_lerp)
	var a_lerp := clampf(ANIMAL_LERP_SPEED * delta, 0.0, 1.0)
	for i in range(_animal_nodes.size()):
		if _animal_nodes[i].visible:
			_animal_nodes[i].position = _animal_nodes[i].position.lerp(_animal_target_pos[i], a_lerp)

	_gather_time += delta
	_animate_gathering()
	_update_chat_bubbles(delta)

	if _notify_timer > 0.0:
		_notify_timer -= delta
		var alpha := clampf(_notify_timer / 0.8, 0.0, 1.0)
		_notify_lbl.modulate.a = alpha
		if _notify_timer <= 0.0:
			_notify_lbl.visible = false

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
	env.ambient_light_energy = 0.7
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
	var village := tile_to_world(VILLAGE_CENTER_COL, VILLAGE_CENTER_ROW)
	_cam.position = Vector3(village.x, CAM_ZOOM_DEFAULT, village.z + 14.0)
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

func _build_resources() -> void:
	var container := Node3D.new()
	container.name = "Resources"
	add_child(container)
	var count: int = _world_sim.get_resource_count()
	for i in range(count):
		var tile: Vector2i = _world_sim.get_resource_pos(i)
		var kind: String   = _world_sim.get_resource_kind(i)
		var rnode := ResourceFactory.make(kind, tile.x, tile.y, TILE_SIZE)
		container.add_child(rnode)
		_resource_meshes.append(rnode.get_meta("mesh_inst"))

func _build_citizens() -> void:
	var container := Node3D.new()
	container.name = "Citizens"
	add_child(container)
	var count: int = _world_sim.get_citizen_count()
	for i in range(count):
		var cname: String = _world_sim.get_citizen_name(i)
		var node := CitizenFactory.make(cname, i)
		container.add_child(node)
		_citizen_nodes.append(node)
		_citizen_behaviors.append("idle")
		_chat_gap_timers.append(randf_range(1.0, 4.0))
		_chat_show_timers.append(0.0)
		_citizen_target_pos.append(Vector3.ZERO)
		_sync_citizen_pos(i)
		node.position = _citizen_target_pos[i]

func _build_animals() -> void:
	var container := Node3D.new()
	container.name = "Animals"
	add_child(container)
	var count: int = _world_sim.get_animal_count()
	for i in range(count):
		var anode := AnimalFactory.make_deer()
		container.add_child(anode)
		_animal_nodes.append(anode)
		_animal_target_pos.append(Vector3.ZERO)
	_update_animals()
	for i in range(_animal_nodes.size()):
		_animal_nodes[i].position = _animal_target_pos[i]

func _build_debug_ui() -> void:
	var layer := CanvasLayer.new()
	layer.name = "DebugUI"
	add_child(layer)

	_debug_panel = VBoxContainer.new()
	_debug_panel.name = "CitizenPanel"
	_debug_panel.position = Vector2(10.0, 10.0)
	layer.add_child(_debug_panel)

	var count := _world_sim.get_citizen_count()
	for i in range(count):
		_append_debug_row(_world_sim.get_citizen_name(i))

func _append_debug_row(cname: String) -> void:
	var bars := DebugHud.add_row(_debug_panel, cname)
	_fed_bars.append(bars["fed_bar"])
	_hyd_bars.append(bars["hyd_bar"])

func _build_notify_label() -> void:
	var layer := CanvasLayer.new()
	layer.name = "NotifyLayer"
	add_child(layer)
	_notify_lbl = DebugHud.make_notify_label()
	layer.add_child(_notify_lbl)

func _show_notify(msg: String) -> void:
	_notify_lbl.text = msg
	_notify_lbl.modulate.a = 1.0
	_notify_lbl.visible = true
	_notify_timer = 3.5

# ── Per-tick updates ──────────────────────────────────────────────────────────

func _sync_citizen_pos(idx: int) -> void:
	var tile: Vector2i = _world_sim.get_citizen_tile_pos(idx)
	var base: Vector3 = tile_to_world(tile.x, tile.y)
	# Golden angle offset prevents overlap when multiple citizens share a tile
	const GOLDEN_ANGLE := 2.399963  # radians ≈ 137.5°
	const OFFSET_RADIUS := 0.28
	var angle := idx * GOLDEN_ANGLE
	var offset := Vector3(cos(angle) * OFFSET_RADIUS, 0.0, sin(angle) * OFFSET_RADIUS)
	_citizen_target_pos[idx] = base + offset

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
	_update_animals()
	_update_day_night()

func _update_day_night() -> void:
	var tick: int = _world_sim.get_tick_count()
	var progress := float(tick % DAY_TICKS) / float(DAY_TICKS)
	var angle_x := -10.0 - 340.0 * progress
	_sun.rotation_degrees.x = angle_x
	var noon := 1.0 - absf(progress - 0.22) * 5.0
	_sun.light_energy = lerpf(0.05, 1.4, clampf(noon, 0.0, 1.0))

func _check_births() -> void:
	while _world_sim.pop_citizen_birth():
		var idx := _citizen_nodes.size()
		var cname: String = _world_sim.get_citizen_name(idx)
		var citizens_container := get_node_or_null("Citizens")
		if citizens_container == null:
			break
		var node := CitizenFactory.make(cname, idx)
		citizens_container.add_child(node)
		_citizen_nodes.append(node)
		_citizen_behaviors.append("idle")
		_chat_gap_timers.append(randf_range(1.0, 4.0))
		_chat_show_timers.append(0.0)
		_citizen_target_pos.append(Vector3.ZERO)
		_sync_citizen_pos(idx)
		node.position = _citizen_target_pos[idx]
		_append_debug_row(cname)
		_show_notify("New citizen born: " + cname + "!")

func _update_resources() -> void:
	var count: int = _world_sim.get_resource_count()
	for i in range(count):
		var qty: float = _world_sim.get_resource_quantity(i)
		if qty <= 0.0:
			_resource_meshes[i].visible = false
		else:
			_resource_meshes[i].visible = true
			_resource_meshes[i].scale = Vector3(qty, qty, qty)

func _update_animals() -> void:
	var count: int = _world_sim.get_animal_count()
	for i in range(count):
		var fled: bool = _world_sim.pop_animal_fled(i)
		var alive: bool = _world_sim.get_animal_alive(i)
		var anode: Node3D = _animal_nodes[i]

		if fled:
			var tween := AnimalFactory.fade_alpha(anode, 0.0, 0.6)
			tween.tween_callback(func(): anode.visible = false)
		elif alive:
			var tile: Vector2i = _world_sim.get_animal_pos(i)
			_animal_target_pos[i] = tile_to_world(tile.x, tile.y)
			if not anode.visible:
				anode.position = _animal_target_pos[i]
				AnimalFactory.set_alpha(anode, 0.0)
				anode.visible = true
				AnimalFactory.fade_alpha(anode, 1.0, 0.6)

func _update_chat_bubbles(delta: float) -> void:
	for i in range(_citizen_nodes.size()):
		if _chat_show_timers[i] > 0.0:
			_chat_show_timers[i] -= delta
			if _chat_show_timers[i] <= 0.0:
				var chat_lbl: Label3D = _citizen_nodes[i].get_meta("chat_lbl")
				chat_lbl.visible = false

		if _chat_gap_timers[i] > 0.0:
			_chat_gap_timers[i] -= delta
			if _chat_gap_timers[i] <= 0.0:
				var beh: String = _citizen_behaviors[i]
				var lines: Array = CHAT_LINES.get(beh, ["..."])
				var chat_lbl: Label3D = _citizen_nodes[i].get_meta("chat_lbl")
				chat_lbl.text = lines[randi() % lines.size()]
				chat_lbl.visible = true
				_chat_show_timers[i] = 2.5
				_chat_gap_timers[i] = randf_range(4.0, 9.0)

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
		"hunting":       return Color(0.9, 0.2, 0.2)
	return Color(0.7, 0.7, 0.7)
