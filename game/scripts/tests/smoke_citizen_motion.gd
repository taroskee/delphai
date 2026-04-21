extends SceneTree
# Phase 1 Sprint N4 smoke test — headless で world.tscn を 2 秒回し、
# citizen[0] の position が "連続して" 変化していることを確認する。
# マス目移動（tick 間に止まる）へ戻った場合、MAX_STATIC_FRAMES を超えて fail する。
#
# 実行: GODOT=godot make smoke-citizen
#   - Rust 側補間 (get_citizen_world_pos) がフレーム毎に微小に動くことを要求
#   - 60fps / TICK_HZ=4 なら 1 tick = 15 frames、同じ tile に留まらない限り毎フレーム動く

const DURATION_MS: int = 2000
const MAX_STATIC_FRAMES: int = 10
const MIN_MOTION_DELTA: float = 0.0005

var _world_scene: Node = null
var _start_ms: int = 0
var _prev_pos: Vector3 = Vector3(1.0e9, 1.0e9, 1.0e9)
var _consecutive_static: int = 0
var _max_consecutive_static: int = 0
var _frames_sampled: int = 0
var _finished: bool = false

func _initialize() -> void:
	var pack: PackedScene = load("res://scenes/world.tscn") as PackedScene
	if pack == null:
		push_error("[smoke] failed to load world.tscn")
		quit(1)
		return
	_world_scene = pack.instantiate()
	root.add_child(_world_scene)
	_start_ms = Time.get_ticks_msec()
	process_frame.connect(_on_frame)

func _on_frame() -> void:
	if _finished or _world_scene == null:
		return
	var citizens_parent: Node = _world_scene.get_node_or_null("Citizens")
	if citizens_parent == null or citizens_parent.get_child_count() == 0:
		if Time.get_ticks_msec() - _start_ms > 500:
			push_error("[smoke] no citizens spawned within 500ms")
			_finished = true
			quit(1)
		return
	var citizen: Node3D = citizens_parent.get_child(0) as Node3D
	var cur: Vector3 = citizen.position
	if cur.distance_to(_prev_pos) < MIN_MOTION_DELTA:
		_consecutive_static += 1
		if _consecutive_static > _max_consecutive_static:
			_max_consecutive_static = _consecutive_static
	else:
		_consecutive_static = 0
	_prev_pos = cur
	_frames_sampled += 1
	if Time.get_ticks_msec() - _start_ms >= DURATION_MS:
		_finished = true
		print("[smoke] frames=%d max_consecutive_static=%d" % [_frames_sampled, _max_consecutive_static])
		if _max_consecutive_static > MAX_STATIC_FRAMES:
			push_error("[smoke] FAIL: max_consecutive_static=%d > %d (grid-move regression?)" % [_max_consecutive_static, MAX_STATIC_FRAMES])
			quit(1)
		else:
			print("[smoke] PASS")
			quit(0)
