class_name GlbLoader
extends RefCounted

## Helpers for reusing imported GLB scenes.
##
## `load_subscene` instantiates a GLB, finds a specific sub-node by name,
## detaches it, and discards the rest. `prepare_fade_materials` walks a
## node tree and converts imported surface materials to per-instance copies
## with TRANSPARENCY_ALPHA so callers can tween albedo alpha.

const _FADE_META := "_delphai_fade_mats"

## Load `glb_path`, find a descendant named `node_name`, detach and return it.
## Returns null if the file cannot be loaded or the node is missing.
static func load_subscene(glb_path: String, node_name: String) -> Node3D:
	var packed := load(glb_path) as PackedScene
	if packed == null:
		push_warning("GlbLoader: failed to load " + glb_path)
		return null
	var full := packed.instantiate() as Node3D
	if full == null:
		push_warning("GlbLoader: root is not Node3D for " + glb_path)
		return null
	var sub := full.find_child(node_name, true, false) as Node3D
	if sub == null:
		push_warning("GlbLoader: node '" + node_name + "' missing in " + glb_path)
		full.queue_free()
		return null
	sub.get_parent().remove_child(sub)
	sub.position = Vector3.ZERO
	sub.rotation = Vector3.ZERO
	full.queue_free()
	return sub

## Walk the subtree and ensure every MeshInstance3D surface has a StandardMaterial3D
## override with TRANSPARENCY_ALPHA. Returns the flat list of those materials.
## Idempotent: caches the prepared material list on each mesh instance.
static func prepare_fade_materials(node: Node) -> Array:
	var out: Array = []
	_collect_fade_materials(node, out)
	return out

static func _collect_fade_materials(node: Node, out: Array) -> void:
	if node is MeshInstance3D:
		var mi := node as MeshInstance3D
		var cached: Array
		if mi.has_meta(_FADE_META):
			cached = mi.get_meta(_FADE_META)
		else:
			cached = _prepare_mesh_fade(mi)
			mi.set_meta(_FADE_META, cached)
		for mat in cached:
			out.append(mat)
	for child in node.get_children():
		_collect_fade_materials(child, out)

static func _prepare_mesh_fade(mi: MeshInstance3D) -> Array:
	var mats: Array = []
	if mi.mesh == null:
		return mats
	for s in range(mi.mesh.get_surface_count()):
		var src := mi.get_surface_override_material(s)
		if src == null:
			src = mi.mesh.surface_get_material(s)
		var mat: StandardMaterial3D
		if src is StandardMaterial3D:
			mat = (src as StandardMaterial3D).duplicate() as StandardMaterial3D
		else:
			mat = StandardMaterial3D.new()
		mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
		mi.set_surface_override_material(s, mat)
		mats.append(mat)
	return mats
