extends Node3D

# Phase 1 Sprint N4 / N4.1: 住民 1 人を 4Hz tick + フレーム補間で滑らかに動かす最小シーン。
# 住民は連続ランダムウォーク（Rust 側 World::enable_random_walk）。
# カメラ pan/zoom は Sprint 13.R0 の実装から移植（bc0a2c9）。

const TILE_SIZE: float = 2.0
const TICK_HZ: float = 4.0
const TICK_INTERVAL: float = 1.0 / TICK_HZ
const CITIZEN_Y: float = 0.7

const MAP_WIDTH: int = 24
const MAP_HEIGHT: int = 14
const RANDOM_WALK_SEED: int = 0xC0FFEE

# Camera tunables — copied verbatim from Sprint 13.R0 world.gd.
const CAM_ZOOM_MIN := 10.0
const CAM_ZOOM_MAX := 200.0
const CAM_ZOOM_DEFAULT := 22.0
const CAM_PAN_SPEED := 0.05
const CAM_KEY_PAN_SPEED := 24.0
const CAM_KEY_ZOOM_SPEED := 20.0
const CAM_WHEEL_ZOOM_STEP := 2.5
const CAM_PINCH_ZOOM_GAIN := 18.0
const CAM_TRACKPAD_PAN_GAIN := 0.05
const CAM_DRAG_ACCEL_MAX := 4.0
const CAM_DRAG_ACCEL_REF_PX := 1500.0
const CAM_KEY_ACCEL_MAX := 3.0
const CAM_KEY_ACCEL_RAMP := 1.5

@onready var _world_node: Node = $WorldNode
@onready var _citizens_parent: Node3D = $Citizens

var _tick_accum: float = 0.0
var _citizen_nodes: Array[Node3D] = []

var _cam: Camera3D = null
var _cam_dragging: bool = false
var _key_pan_hold: float = 0.0

func _ready() -> void:
	_world_node.initialize(MAP_WIDTH, MAP_HEIGHT, RANDOM_WALK_SEED)
	_spawn_citizen_visuals()
	_build_camera()

func _spawn_citizen_visuals() -> void:
	var count: int = _world_node.get_citizen_count()
	for i in count:
		var mesh_inst := MeshInstance3D.new()
		mesh_inst.name = "Citizen%d" % i
		var capsule := CapsuleMesh.new()
		capsule.radius = 0.3
		capsule.height = 1.4
		mesh_inst.mesh = capsule
		_citizens_parent.add_child(mesh_inst)
		_citizen_nodes.append(mesh_inst)

func _build_camera() -> void:
	_cam = Camera3D.new()
	_cam.name = "Camera3D"
	var center_x := MAP_WIDTH * 0.5 * TILE_SIZE
	var center_z := MAP_HEIGHT * 0.5 * TILE_SIZE
	_cam.position = Vector3(center_x, CAM_ZOOM_DEFAULT, center_z + 14.0)
	_cam.rotation_degrees = Vector3(-55.0, 0.0, 0.0)
	add_child(_cam)

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
	_tick_accum += delta
	while _tick_accum >= TICK_INTERVAL:
		_world_node.tick()
		_tick_accum -= TICK_INTERVAL
	var alpha: float = clampf(_tick_accum / TICK_INTERVAL, 0.0, 1.0)
	for i in _citizen_nodes.size():
		var pos2: Vector2 = _world_node.get_citizen_world_pos(i, alpha)
		_citizen_nodes[i].position = Vector3(pos2.x * TILE_SIZE, CITIZEN_Y, pos2.y * TILE_SIZE)
