class_name CitizenFactory
extends RefCounted

## Builds a chess-pawn citizen Node3D with name/behavior/chat labels.
## Meta keys set on the returned root:
##   "mat"      → StandardMaterial3D used for all body parts (tinted by vitals)
##   "beh_lbl"  → behavior Label3D
##   "chat_lbl" → chat bubble Label3D (hidden by default)

static func make(cname: String, idx: int) -> Node3D:
	var root := Node3D.new()
	root.name = "Citizen_%d" % idx

	var mat := StandardMaterial3D.new()
	mat.albedo_color = Color(0.85, 0.65, 0.35)
	root.set_meta("mat", mat)

	# Chess pawn — base disc
	var base_mi := MeshInstance3D.new()
	var base_cyl := CylinderMesh.new()
	base_cyl.top_radius    = 0.22
	base_cyl.bottom_radius = 0.26
	base_cyl.height        = 0.10
	base_mi.mesh = base_cyl
	base_mi.material_override = mat
	base_mi.position.y = 0.05
	root.add_child(base_mi)

	# Chess pawn — body pillar
	var body_mi := MeshInstance3D.new()
	var body_cyl := CylinderMesh.new()
	body_cyl.top_radius    = 0.13
	body_cyl.bottom_radius = 0.18
	body_cyl.height        = 0.55
	body_mi.mesh = body_cyl
	body_mi.material_override = mat
	body_mi.position.y = 0.38
	root.add_child(body_mi)

	# Chess pawn — head sphere
	var head_mi := MeshInstance3D.new()
	var head_sphere := SphereMesh.new()
	head_sphere.radius = 0.18
	head_sphere.height = 0.36
	head_mi.mesh = head_sphere
	head_mi.material_override = mat
	head_mi.position.y = 0.82
	root.add_child(head_mi)

	# Name label (billboard)
	var name_lbl := Label3D.new()
	name_lbl.text = cname
	name_lbl.font_size = 28
	name_lbl.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	name_lbl.position.y = 1.15
	root.add_child(name_lbl)

	# Behavior label
	var beh_lbl := Label3D.new()
	beh_lbl.text = "idle"
	beh_lbl.font_size = 22
	beh_lbl.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	beh_lbl.position.y = 1.4
	beh_lbl.modulate = Color(0.9, 0.9, 0.9)
	root.add_child(beh_lbl)
	root.set_meta("beh_lbl", beh_lbl)

	# Chat bubble
	var chat_lbl := Label3D.new()
	chat_lbl.text = ""
	chat_lbl.font_size = 24
	chat_lbl.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	chat_lbl.position.y = 1.65
	chat_lbl.modulate = Color(1.0, 1.0, 0.75)
	chat_lbl.visible = false
	root.add_child(chat_lbl)
	root.set_meta("chat_lbl", chat_lbl)

	return root
