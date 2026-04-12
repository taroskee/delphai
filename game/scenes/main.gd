extends Node2D

const OLLAMA_URL := "http://localhost:11434/api/generate"
const OLLAMA_MODEL := "gemma4:e2b"

const CitizenSpriteScript = preload("res://scenes/citizen_sprite.gd")

# Character sprite paths (must have emotion frames: shake/nod/surprise/laugh)
const CITIZEN_CHAR_PATHS: Array = ["chara/chara2_1", "chara/chara3_1", "chara/chara4_1"]
# Positions around the campfire (1152 x 648 layout)
const CITIZEN_POSITIONS: Array = [
	Vector2(340, 410),   # Kael  — left
	Vector2(576, 440),   # Elder — center, slightly forward
	Vector2(820, 410),   # Hara  — right
]

var _world_sim
var _citizen_sprites: Array = []
var _conv_log: RichTextLabel
var _log_scroll: ScrollContainer
var _voice_input: LineEdit
var _http: HTTPRequest

# LLM call queue: Array of {initiator_idx, partner_idx, is_reply?}
var _llm_queue: Array = []
var _llm_busy: bool = false
var _current_pair: Dictionary = {}
var _log_file: FileAccess = null

func _ready() -> void:
	_open_log_file()
	_build_world()
	_build_scene()
	_build_ui()
	_log_message("=== 焚き火デモ 起動 ===")
	_update_citizen_panels()

# ---------------------------------------------------------------------------
# World simulation setup
# ---------------------------------------------------------------------------

func _build_world() -> void:
	if not ClassDB.class_exists("WorldNode"):
		push_error("WorldNode class not found — rebuild libdelphai_gdext and restart Godot")
		return

	_world_sim = ClassDB.instantiate("WorldNode")
	add_child(_world_sim)
	_world_sim.initialize()

	var timer := Timer.new()
	timer.wait_time = 2.0
	timer.autostart = true
	add_child(timer)
	timer.timeout.connect(_on_timer_timeout)

	_http = HTTPRequest.new()
	add_child(_http)
	_http.request_completed.connect(_on_http_completed)

# ---------------------------------------------------------------------------
# 2D scene: background + fire + citizen sprites
# ---------------------------------------------------------------------------

func _build_scene() -> void:
	_build_background()
	_build_fire()
	_build_citizen_sprites()

func _build_background() -> void:
	var bg_layer := CanvasLayer.new()
	bg_layer.layer = -10
	add_child(bg_layer)

	# Night sky
	var sky := ColorRect.new()
	sky.color = Color(0.03, 0.04, 0.12, 1.0)
	sky.size = Vector2(1152, 648)
	sky.position = Vector2.ZERO
	bg_layer.add_child(sky)

	# Ground — darker earth tone
	var ground := ColorRect.new()
	ground.color = Color(0.08, 0.11, 0.04, 1.0)
	ground.size = Vector2(1152, 230)
	ground.position = Vector2(0, 418)
	bg_layer.add_child(ground)

	# Campfire glow on ground — warm orange circle under fire
	var glow := ColorRect.new()
	glow.color = Color(0.55, 0.22, 0.02, 0.35)
	glow.size = Vector2(220, 60)
	glow.position = Vector2(576 - 110, 418)
	bg_layer.add_child(glow)

func _build_fire() -> void:
	var fire_sprite := AnimatedSprite2D.new()
	var fire_frames := SpriteFrames.new()
	fire_frames.remove_animation("default")
	fire_frames.add_animation("burn")
	fire_frames.set_animation_speed("burn", 12.0)
	fire_frames.set_animation_loop("burn", true)

	# fireV005: 51×51px frames — at scale 2.5 renders as ~128×128px campfire
	var fire_base := "res://assets/effect/fires/loop/fireV005effect-loop/"
	for i in range(18):
		var path := fire_base + "fireV005effect-loop%03d.png" % i
		if ResourceLoader.exists(path):
			var tex := load(path) as Texture2D
			if tex:
				fire_frames.add_frame("burn", tex)

	fire_sprite.sprite_frames = fire_frames
	fire_sprite.scale = Vector2(2.5, 2.5)
	fire_sprite.position = Vector2(576, 400)
	if fire_frames.has_animation("burn") and fire_frames.get_frame_count("burn") > 0:
		fire_sprite.play("burn")
	add_child(fire_sprite)

func _build_citizen_sprites() -> void:
	if not _world_sim:
		return
	var count: int = _world_sim.get_citizen_count()
	for i in range(count):
		var cname: String = _world_sim.get_citizen_name(i)
		var char_path: String = CITIZEN_CHAR_PATHS[i] if i < CITIZEN_CHAR_PATHS.size() else "chara/chara2_1"

		var sprite_node := CitizenSpriteScript.new()
		add_child(sprite_node)
		sprite_node.position = CITIZEN_POSITIONS[i] if i < CITIZEN_POSITIONS.size() else Vector2(576, 420)
		sprite_node.setup(cname, char_path)
		_citizen_sprites.append(sprite_node)

# ---------------------------------------------------------------------------
# UI: log panel + voice input (CanvasLayer overlay)
# ---------------------------------------------------------------------------

func _build_ui() -> void:
	var canvas := CanvasLayer.new()
	canvas.layer = 10
	add_child(canvas)

	var root_vbox := VBoxContainer.new()
	canvas.add_child(root_vbox)
	root_vbox.set_anchors_preset(Control.PRESET_FULL_RECT)
	root_vbox.offset_left = 12.0
	root_vbox.offset_top = 12.0
	root_vbox.offset_right = -12.0
	root_vbox.offset_bottom = -12.0

	# Spacer pushes log to bottom
	var spacer := Control.new()
	spacer.size_flags_vertical = Control.SIZE_EXPAND_FILL
	root_vbox.add_child(spacer)

	# Log area
	_log_scroll = ScrollContainer.new()
	_log_scroll.custom_minimum_size = Vector2(0, 160)
	root_vbox.add_child(_log_scroll)

	_conv_log = RichTextLabel.new()
	_conv_log.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_conv_log.fit_content = true
	_conv_log.scroll_active = false
	_conv_log.add_theme_color_override("default_color", Color(0.9, 0.9, 0.85, 1.0))
	_log_scroll.add_child(_conv_log)

	# Voice input row
	var voice_row := HBoxContainer.new()
	root_vbox.add_child(voice_row)

	_voice_input = LineEdit.new()
	_voice_input.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_voice_input.placeholder_text = "声を届ける... 例: 「北の川に水源がある」「東の森に鹿の群れがいる」"
	voice_row.add_child(_voice_input)

	var voice_button := Button.new()
	voice_button.text = "声を届ける"
	voice_button.pressed.connect(_on_voice_pressed)
	voice_row.add_child(voice_button)

# ---------------------------------------------------------------------------
# Timer / tick
# ---------------------------------------------------------------------------

func _on_timer_timeout() -> void:
	if not _world_sim:
		return
	var pending: Array = _world_sim.tick(randf())
	for p in pending:
		var i_name: String = _world_sim.get_citizen_name(p["initiator_idx"])
		var p_name: String = _world_sim.get_citizen_name(p["partner_idx"])
		_log_message("[tick %d] %s と %s が会話中..." % [
			_world_sim.get_tick_count(), i_name, p_name
		])
		_llm_queue.append(p)
	_process_llm_queue()
	_update_citizen_panels()

# ---------------------------------------------------------------------------
# LLM pipeline
# ---------------------------------------------------------------------------

func _process_llm_queue() -> void:
	if _llm_busy or _llm_queue.is_empty():
		return
	_current_pair = _llm_queue.pop_front()
	_call_ollama_for_pair(_current_pair["initiator_idx"], _current_pair["partner_idx"])

func _call_ollama_for_pair(i_idx: int, p_idx: int) -> void:
	if not _world_sim:
		return
	var prompt: String = _world_sim.build_conversation_prompt_str(i_idx, p_idx)
	if prompt.is_empty():
		var i_name: String = _world_sim.get_citizen_name(i_idx)
		_log_message("[デバッグ] %s のプロンプト構築失敗（メソッド未登録の可能性）" % i_name)
		return
	# Clear divine voice after baking it into this prompt
	_world_sim.clear_divine_voice()
	var i_name: String = _world_sim.get_citizen_name(i_idx)
	var p_name: String = _world_sim.get_citizen_name(p_idx)
	_log_message("[LLM送信] %s → %s (%d文字)" % [i_name, p_name, prompt.length()])
	_llm_busy = true
	_send_ollama_request(prompt)

func _send_ollama_request(prompt: String) -> void:
	var body := JSON.stringify({
		"model": OLLAMA_MODEL,
		"prompt": prompt,
		"stream": false
	})
	var headers := PackedStringArray(["Content-Type: application/json"])
	var err := _http.request(OLLAMA_URL, headers, HTTPClient.METHOD_POST, body)
	if err != OK:
		push_error("HTTPRequest failed to send: %d" % err)
		_llm_busy = false
		_process_llm_queue()

func _on_http_completed(
	result: int,
	response_code: int,
	_headers: PackedStringArray,
	body: PackedByteArray
) -> void:
	_llm_busy = false

	if result != HTTPRequest.RESULT_SUCCESS or response_code != 200:
		var msg := "[Ollama エラー] result=%d code=%d" % [result, response_code]
		push_error(msg)
		_log_message(msg)
		_process_llm_queue()
		return

	var json_text := body.get_string_from_utf8()
	var parsed = JSON.parse_string(json_text)
	if parsed == null or not parsed.has("response"):
		var msg := "[Ollama エラー] 予期しないレスポンス: " + json_text.left(120)
		push_error(msg)
		_log_message(msg)
		_process_llm_queue()
		return

	var raw_response: String = parsed["response"]
	var yaml_text := _strip_code_fences(raw_response)
	_log_message("[生データ] " + yaml_text.left(160).replace("\n", " ↵ "))

	var speech := _extract_yaml_field(yaml_text, "speech")
	var emotion := _extract_yaml_field(yaml_text, "emotion_change")

	if speech.is_empty():
		_log_message("[デバッグ] speech フィールドが空 — YAML: " + yaml_text.left(120))
	elif _world_sim:
		var i_idx: int = _current_pair.get("initiator_idx", 0)
		var i_name: String = _world_sim.get_citizen_name(i_idx)
		_world_sim.apply_citizen_response(i_idx, speech, emotion)
		_log_message("[%s] %s" % [i_name, speech])

		# Update sprite: speech bubble + reaction animation
		if i_idx < _citizen_sprites.size():
			var awareness: float = _world_sim.get_divine_awareness(i_idx)
			_citizen_sprites[i_idx].show_speech(speech)
			_citizen_sprites[i_idx].play_reaction(awareness)

		_update_citizen_panels()

		# Queue partner's reply (once only)
		if not _current_pair.get("is_reply", false):
			var p_idx: int = _current_pair.get("partner_idx", 1)
			if _world_sim:
				_world_sim.record_heard_speech(p_idx, i_name, speech)
			_llm_queue.append({
				"initiator_idx": p_idx,
				"partner_idx": i_idx,
				"is_reply": true,
			})

	_process_llm_queue()

# ---------------------------------------------------------------------------
# Divine voice
# ---------------------------------------------------------------------------

func _on_voice_pressed() -> void:
	var text: String = _voice_input.text.strip_edges()
	if text.is_empty():
		return
	_log_message("[神の声] " + text)
	_voice_input.clear()
	if not _world_sim:
		return

	_world_sim.set_divine_voice(text)

	# Grow awareness for all citizens, play skeptical reaction on first encounter
	var count: int = _world_sim.get_citizen_count()
	for i in range(count):
		_world_sim.grow_divine_awareness(i, 0.07)
		var awareness: float = _world_sim.get_divine_awareness(i)
		var cname: String = _world_sim.get_citizen_name(i)
		var awareness_pct: int = roundi(awareness * 100)
		_log_message("[awareness] %s: %d%%" % [cname, awareness_pct])
		if i < _citizen_sprites.size():
			_citizen_sprites[i].play_reaction(awareness)

	# Queue a divine voice reaction from Kael (citizen 0)
	_llm_queue.append({"initiator_idx": 0, "partner_idx": 1})
	_process_llm_queue()

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

func _update_citizen_panels() -> void:
	if not _world_sim:
		return
	var count: int = _world_sim.get_citizen_count()
	for i in range(count):
		if i >= _citizen_sprites.size():
			break
		var emotion: String = _world_sim.get_citizen_emotion(i)
		_citizen_sprites[i].set_emotion_label(emotion)

## Extract a single-line YAML field value.
## Handles `field: foo bar`, `field: "foo bar"`, and `field: 'foo bar'`.
## Does NOT support YAML block scalars (| / >) — returns empty for those.
func _extract_yaml_field(text: String, field: String) -> String:
	for line in text.split("\n"):
		var stripped := line.strip_edges()
		var prefix := field + ":"
		if stripped.begins_with(prefix):
			var value := stripped.substr(prefix.length()).strip_edges()
			if value == "|" or value == ">" or value == "|-" or value == ">-":
				return ""
			if (value.begins_with('"') and value.ends_with('"')) or \
			   (value.begins_with("'") and value.ends_with("'")):
				value = value.substr(1, value.length() - 2)
			if value != "~" and value != "null" and not value.is_empty():
				return value
	return ""

## Strip markdown code fences from LLM output.
func _strip_code_fences(text: String) -> String:
	var lines := text.split("\n")
	var result: PackedStringArray = []
	for line in lines:
		if line.strip_edges().begins_with("```"):
			continue
		result.append(line)
	return "\n".join(result).strip_edges()

func _open_log_file() -> void:
	DirAccess.make_dir_recursive_absolute("user://logs")
	var t := Time.get_datetime_dict_from_system()
	var fname := "user://logs/conv_%04d%02d%02d_%02d%02d%02d.log" % [
		t["year"], t["month"], t["day"], t["hour"], t["minute"], t["second"]
	]
	_log_file = FileAccess.open(fname, FileAccess.WRITE)
	if _log_file:
		var real_path := ProjectSettings.globalize_path(fname)
		print("[LOG] 会話ログ: ", real_path)
		_log_file.store_line("# ログパス: " + real_path)
		_log_file.flush()
	else:
		push_warning("会話ログファイルを開けませんでした: " + fname)

func _log_message(msg: String) -> void:
	if not _conv_log:
		return
	_conv_log.append_text(msg + "\n")
	if _log_file:
		_log_file.store_line(msg)
		_log_file.flush()
	await get_tree().process_frame
	if _log_scroll:
		_log_scroll.scroll_vertical = int(_log_scroll.get_v_scroll_bar().max_value)
