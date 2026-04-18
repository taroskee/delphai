class_name DebugHud
extends RefCounted

## Static helpers for the debug UI (citizen fed/hydration rows + notify banner).

## Build a stat ProgressBar with consistent sizing.
static func make_stat_bar(color: Color) -> ProgressBar:
	var bar := ProgressBar.new()
	bar.min_value = 0.0
	bar.max_value = 1.0
	bar.value     = 1.0
	bar.custom_minimum_size = Vector2(80, 18)
	bar.modulate  = color
	bar.show_percentage = false
	return bar

## Append a citizen row to `panel`. Returns {fed_bar, hyd_bar}.
static func add_row(panel: VBoxContainer, cname: String) -> Dictionary:
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 6)
	panel.add_child(row)

	var lbl := Label.new()
	lbl.text = cname
	lbl.custom_minimum_size = Vector2(64, 0)
	row.add_child(lbl)

	var fed_bar := make_stat_bar(Color(1.0, 0.55, 0.1))
	row.add_child(fed_bar)

	var hyd_bar := make_stat_bar(Color(0.3, 0.6, 1.0))
	row.add_child(hyd_bar)

	return {"fed_bar": fed_bar, "hyd_bar": hyd_bar}

## Build the floating notify banner used for population events.
static func make_notify_label() -> Label:
	var lbl := Label.new()
	lbl.name = "NotifyLabel"
	lbl.text = ""
	lbl.add_theme_font_size_override("font_size", 28)
	lbl.add_theme_color_override("font_color", Color(1.0, 0.9, 0.3))
	lbl.set_anchor_and_offset(SIDE_LEFT,  0.5, -200.0)
	lbl.set_anchor_and_offset(SIDE_RIGHT, 0.5,  200.0)
	lbl.set_anchor_and_offset(SIDE_TOP,   0.0,   60.0)
	lbl.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	lbl.visible = false
	return lbl
