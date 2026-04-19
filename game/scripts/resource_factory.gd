class_name ResourceFactory
extends RefCounted

## Builds visual Node3D for a resource.
##
## - berry_bush: instance of `simple_nature_pack_glb.glb` → `Plant_01`
##   (falls back to a green sphere if GLB is missing)
## - water_source: cluster of 3× `Rock_01` from `simple_nature_pack_glb.glb`
##   arranged in a ring — marks a water access point. No blue polygon
##   (the pack has no water mesh; we use the GLB-only rule instead).
##
## The scalable root is stored as "mesh_inst" meta so world.gd can
## shrink it when the resource quantity depletes.

const NATURE_GLB        := "res://assets/nature/simple_nature_pack_glb.glb"
const BERRY_NODE_NAME   := "Plant_01"
const BERRY_SCALE       := 0.25  # Plant_01 is ~2m tall in source; shrink to bush-size
const BERRY_FB_COLOR    := Color(0.08, 0.55, 0.08)
const BERRY_FB_RADIUS   := 0.4

const WATER_ROCK_NODE   := "Rock_01"
const WATER_ROCK_COUNT  := 3
const WATER_ROCK_RADIUS := 0.35
const WATER_ROCK_SCALE  := 0.28

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
	# Water access marker: ring of Rock_01 GLB instances. No custom polygon.
	# If the GLB fails to load, the node is an invisible anchor (no fallback mesh)
	# so the scene still runs without injecting a primitive shape.
	var root := Node3D.new()
	for i in range(WATER_ROCK_COUNT):
		var rock := GlbLoader.load_subscene(NATURE_GLB, WATER_ROCK_NODE)
		if rock == null:
			continue
		var angle := TAU * float(i) / float(WATER_ROCK_COUNT)
		rock.position = Vector3(cos(angle) * WATER_ROCK_RADIUS, 0.0, sin(angle) * WATER_ROCK_RADIUS)
		rock.scale = Vector3.ONE * WATER_ROCK_SCALE
		root.add_child(rock)
	return root
