class_name VillageBuilder
extends RefCounted

## Pure builder for the village landmark (campfire + stone ring).
## Placed at the village center tile so the player immediately sees
## "this is where things are happening" on game start.

const STONE_COUNT    := 8
const STONE_RADIUS   := 0.7
const STONE_SIZE     := 0.22
const EMBER_RADIUS   := 0.22
const FIRE_LIGHT_RANGE := 6.0

## Add a campfire (stone ring + glowing ember + point light) centered at `pos`.
static func add_campfire(parent: Node3D, pos: Vector3) -> void:
	var root := Node3D.new()
	root.name = "Campfire"
	root.position = pos
	parent.add_child(root)

	_add_stone_ring(root)
	_add_ember(root)
	_add_firelight(root)

static func _add_stone_ring(root: Node3D) -> void:
	var stone_mat := StandardMaterial3D.new()
	stone_mat.albedo_color = Color(0.45, 0.42, 0.40)
	for i in range(STONE_COUNT):
		var angle := TAU * float(i) / float(STONE_COUNT)
		var stone_mi := MeshInstance3D.new()
		var box := BoxMesh.new()
		box.size = Vector3(STONE_SIZE, STONE_SIZE * 0.7, STONE_SIZE)
		stone_mi.mesh = box
		stone_mi.material_override = stone_mat
		stone_mi.position = Vector3(cos(angle) * STONE_RADIUS, STONE_SIZE * 0.35, sin(angle) * STONE_RADIUS)
		root.add_child(stone_mi)

static func _add_ember(root: Node3D) -> void:
	var ember_mi := MeshInstance3D.new()
	var sphere := SphereMesh.new()
	sphere.radius = EMBER_RADIUS
	sphere.height = EMBER_RADIUS * 2.0
	ember_mi.mesh = sphere
	var ember_mat := StandardMaterial3D.new()
	ember_mat.albedo_color = Color(1.0, 0.55, 0.18)
	ember_mat.emission_enabled = true
	ember_mat.emission = Color(1.0, 0.45, 0.10)
	ember_mat.emission_energy_multiplier = 2.5
	ember_mi.material_override = ember_mat
	ember_mi.position.y = EMBER_RADIUS
	root.add_child(ember_mi)

static func _add_firelight(root: Node3D) -> void:
	var light := OmniLight3D.new()
	light.light_color = Color(1.0, 0.65, 0.30)
	light.light_energy = 1.6
	light.omni_range = FIRE_LIGHT_RANGE
	light.position.y = 0.6
	root.add_child(light)
