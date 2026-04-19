class_name CitizenFactory
extends RefCounted

## Builds a citizen Node3D (GLB-based placeholder) with name/behavior/chat labels.
##
## The body visual is `Tree_Stump_01` from `simple_nature_pack_glb.glb` — a
## human-sized stump used as a placeholder until a real human GLB is imported.
## If the GLB fails to load, the citizen is a "ghost" (labels only, no body),
## per the "everything GLB except the campfire" rule.
##
## Meta keys set on the returned root:
##   "body"     → body root Node3D (used by world.gd to tint via `modulate`
##                for hungry/thirsty states; null if the GLB was missing)
##   "beh_lbl"  → behavior Label3D
##   "chat_lbl" → chat bubble Label3D (hidden by default)

const NATURE_GLB    := "res://assets/nature/simple_nature_pack_glb.glb"
const BODY_NODE     := "Tree_Stump_01"
const BODY_SCALE    := 0.6   # Tree_Stump_01 is ~1m in source; shrink to citizen-size
const LABEL_Y_NAME  := 1.15
const LABEL_Y_BEH   := 1.40
const LABEL_Y_CHAT  := 1.65

static func make(cname: String, idx: int) -> Node3D:
	var root := Node3D.new()
	root.name = "Citizen_%d" % idx

	var body := _make_body()
	if body != null:
		root.add_child(body)
	root.set_meta("body", body)

	_add_labels(root, cname)
	return root

static func _make_body() -> Node3D:
	var glb_node := GlbLoader.load_subscene(NATURE_GLB, BODY_NODE)
	if glb_node == null:
		return null
	var wrapper := Node3D.new()
	wrapper.name = "CitizenBody"
	glb_node.scale = Vector3.ONE * BODY_SCALE
	wrapper.add_child(glb_node)
	return wrapper

static func _add_labels(root: Node3D, cname: String) -> void:
	var name_lbl := Label3D.new()
	name_lbl.text = cname
	name_lbl.font_size = 28
	name_lbl.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	name_lbl.position.y = LABEL_Y_NAME
	root.add_child(name_lbl)

	var beh_lbl := Label3D.new()
	beh_lbl.text = "idle"
	beh_lbl.font_size = 22
	beh_lbl.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	beh_lbl.position.y = LABEL_Y_BEH
	beh_lbl.modulate = Color(0.9, 0.9, 0.9)
	root.add_child(beh_lbl)
	root.set_meta("beh_lbl", beh_lbl)

	var chat_lbl := Label3D.new()
	chat_lbl.text = ""
	chat_lbl.font_size = 24
	chat_lbl.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	chat_lbl.position.y = LABEL_Y_CHAT
	chat_lbl.modulate = Color(1.0, 1.0, 0.75)
	chat_lbl.visible = false
	root.add_child(chat_lbl)
	root.set_meta("chat_lbl", chat_lbl)
