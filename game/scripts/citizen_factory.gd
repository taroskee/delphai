class_name CitizenFactory
extends RefCounted

## Builds a chess-pawn citizen Node3D with name/behavior/chat labels.
##
## The body is a 3-part primitive (base disc + body pillar + head sphere).
## Used as a placeholder until a real human GLB is imported. Reverted from
## Tree_Stump_01 because the stump silhouette read as scenery, not as people.
##
## Meta keys set on the returned root:
##   "mat"      → StandardMaterial3D used for all body parts (tinted by vitals)
##   "beh_lbl"  → behavior Label3D
##   "chat_lbl" → chat bubble Label3D (hidden by default)

const BODY_COLOR    := Color(0.85, 0.65, 0.35)
const LABEL_Y_NAME  := 1.15
const LABEL_Y_BEH   := 1.40
const LABEL_Y_CHAT  := 1.65

static func make(cname: String, idx: int) -> Node3D:
	var root := Node3D.new()
	root.name = "Citizen_%d" % idx

	var mat := StandardMaterial3D.new()
	mat.albedo_color = BODY_COLOR
	root.set_meta("mat", mat)

	_add_pawn_body(root, mat)
	_add_labels(root, cname)
	return root

static func _add_pawn_body(root: Node3D, mat: StandardMaterial3D) -> void:
	var base_mi := MeshInstance3D.new()
	var base_cyl := CylinderMesh.new()
	base_cyl.top_radius    = 0.22
	base_cyl.bottom_radius = 0.26
	base_cyl.height        = 0.10
	base_mi.mesh = base_cyl
	base_mi.material_override = mat
	base_mi.position.y = 0.05
	root.add_child(base_mi)

	var body_mi := MeshInstance3D.new()
	var body_cyl := CylinderMesh.new()
	body_cyl.top_radius    = 0.13
	body_cyl.bottom_radius = 0.18
	body_cyl.height        = 0.55
	body_mi.mesh = body_cyl
	body_mi.material_override = mat
	body_mi.position.y = 0.38
	root.add_child(body_mi)

	var head_mi := MeshInstance3D.new()
	var head_sphere := SphereMesh.new()
	head_sphere.radius = 0.18
	head_sphere.height = 0.36
	head_mi.mesh = head_sphere
	head_mi.material_override = mat
	head_mi.position.y = 0.82
	root.add_child(head_mi)

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
