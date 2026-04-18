class_name ResourceFactory
extends RefCounted

## Builds visual Node3D for a resource.
##
## - berry_bush: instance of `simple_nature_pack_glb.glb` → `Plant_01`
##   (falls back to a green sphere if GLB is missing)
## - water_source: primitive CylinderMesh (no water mesh in the pack)
##
## The scalable root is stored as "mesh_inst" meta so world.gd can
## shrink it when the resource quantity depletes.

const NATURE_GLB        := "res://assets/nature/simple_nature_pack_glb.glb"
const BERRY_NODE_NAME   := "Plant_01"
const BERRY_SCALE       := 0.25  # Plant_01 is ~2m tall in source; shrink to bush-size
const BERRY_FB_COLOR    := Color(0.08, 0.55, 0.08)
const BERRY_FB_RADIUS   := 0.4

const WATER_COLOR       := Color(0.1, 0.45, 0.95)
const WATER_RADIUS      := 0.5
const WATER_HEIGHT      := 0.12

static func make(kind: String, col: int, row: int, tile_size: float) -> Node3D:
	var root := Node3D.new()
	root.name = "Resource_" + kind
	var wpos := Vector3(col * tile_size, 0.0, row * tile_size)

	var scalable: Node3D
	if kind == "berry_bush":
		scalable = _make_berry_bush()
		root.position = wpos + Vector3(0, 0.0, 0)
	else:  # water_source
		scalable = _make_water_source()
		root.position = wpos + Vector3(0, 0.06, 0)

	root.add_child(scalable)
	root.set_meta("mesh_inst", scalable)
	root.set_meta("kind", kind)
	return root

static func _make_berry_bush() -> Node3D:
	# Wrap the visual in a scalable root so world.gd can overwrite `.scale`
	# without clobbering the GLB's intrinsic size.
	var root := Node3D.new()
	var glb_node := GlbLoader.load_subscene(NATURE_GLB, BERRY_NODE_NAME)
	if glb_node != null:
		glb_node.scale = Vector3.ONE * BERRY_SCALE
		root.add_child(glb_node)
	else:
		var mi := MeshInstance3D.new()
		var sphere := SphereMesh.new()
		sphere.radius = BERRY_FB_RADIUS
		sphere.height = BERRY_FB_RADIUS * 2.0
		mi.mesh = sphere
		var mat := StandardMaterial3D.new()
		mat.albedo_color = BERRY_FB_COLOR
		mi.material_override = mat
		mi.position.y = BERRY_FB_RADIUS
		root.add_child(mi)
	return root

static func _make_water_source() -> Node3D:
	var root := Node3D.new()
	var mi := MeshInstance3D.new()
	var cyl := CylinderMesh.new()
	cyl.top_radius    = WATER_RADIUS
	cyl.bottom_radius = WATER_RADIUS
	cyl.height        = WATER_HEIGHT
	mi.mesh = cyl
	var mat := StandardMaterial3D.new()
	mat.albedo_color = WATER_COLOR
	mi.material_override = mat
	root.add_child(mi)
	return root
