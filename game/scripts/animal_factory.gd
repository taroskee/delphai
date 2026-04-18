class_name AnimalFactory
extends RefCounted

## Builds deer Node3D visuals from `Animals_FREE.glb` (node "deer").
## Falls back to a primitive sphere if the GLB is missing so the game still runs.
## `fade_alpha` and `set_alpha` walk all MeshInstance3D descendants so they work
## regardless of the GLB's internal hierarchy.

const DEER_GLB          := "res://assets/animal/Animals_FREE.glb"
const DEER_NODE_NAME    := "deer"
const DEER_SCALE        := 0.45
const FADE_DURATION     := 0.6

# Fallback primitive (used when the GLB is unavailable).
const FB_COLOR          := Color(0.72, 0.50, 0.28)  # tan/brown
const FB_RADIUS         := 0.25

static func make_deer() -> Node3D:
	var root := Node3D.new()
	root.name = "Deer"

	var visual := GlbLoader.load_subscene(DEER_GLB, DEER_NODE_NAME)
	if visual != null:
		visual.scale = Vector3.ONE * DEER_SCALE
		root.add_child(visual)
	else:
		root.add_child(_make_fallback_sphere())
	return root

## Fade all descendant mesh materials to `target_a` over `duration` seconds.
## Returns a tween whose subsequent `tween_callback` fires after the fade group.
static func fade_alpha(anode: Node3D, target_a: float, duration: float) -> Tween:
	var mats := GlbLoader.prepare_fade_materials(anode)
	var tween := anode.create_tween()
	if mats.is_empty():
		return tween
	tween.set_parallel(true)
	for mat in mats:
		tween.tween_property(mat, "albedo_color:a", target_a, duration)
	tween.chain()  # following tween_callback waits for the fade group
	return tween

## Set alpha on every descendant mesh material instantly (no tween).
static func set_alpha(anode: Node3D, a: float) -> void:
	var mats := GlbLoader.prepare_fade_materials(anode)
	for mat in mats:
		mat.albedo_color.a = a

static func _make_fallback_sphere() -> MeshInstance3D:
	var mesh_inst := MeshInstance3D.new()
	var sphere := SphereMesh.new()
	sphere.radius = FB_RADIUS
	sphere.height = FB_RADIUS * 2.0
	mesh_inst.mesh = sphere
	var mat := StandardMaterial3D.new()
	mat.albedo_color = FB_COLOR
	mesh_inst.material_override = mat
	mesh_inst.position.y = FB_RADIUS
	return mesh_inst
