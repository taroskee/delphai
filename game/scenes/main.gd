extends Node2D

const OLLAMA_URL := "http://localhost:11434/api/generate"
const OLLAMA_MODEL := "gemma4:e2b"

var _world_sim
var _citizen_name_labels: Array[Label] = []
var _citizen_emotion_labels: Array[Label] = []
var _citizen_speech_labels: Array[Label] = []
var _conv_log: RichTextLabel
var _log_scroll: ScrollContainer
var _voice_input: LineEdit
var _http: HTTPRequest

# LLM call queue: Array of {initiator_idx, partner_idx}
var _llm_queue: Array = []
var _llm_busy: bool = false
var _current_pair: Dictionary = {}

func _ready() -> void:
	_build_world()
	_build_ui()
	_log_message("=== 焚き火デモ 起動 ===")
	_update_citizen_panels()

# ---------------------------------------------------------------------------
# World setup
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
# UI setup
# ---------------------------------------------------------------------------

func _build_ui() -> void:
	var canvas := CanvasLayer.new()
	add_child(canvas)

	var root_panel := VBoxContainer.new()
	root_panel.set_anchors_preset(Control.PRESET_FULL_RECT)
	root_panel.offset_left = 20.0
	root_panel.offset_top = 20.0
	root_panel.offset_right = -20.0
	root_panel.offset_bottom = -20.0
	canvas.add_child(root_panel)

	# --- Citizens header ---
	var citizens_label := Label.new()
	citizens_label.text = "▼ 住民"
	citizens_label.add_theme_font_size_override("font_size", 18)
	root_panel.add_child(citizens_label)

	# --- Citizen panels ---
	var citizen_row := HBoxContainer.new()
	root_panel.add_child(citizen_row)

	for cname in ["Kael", "Elder", "Hara"]:
		var vbox := VBoxContainer.new()
		vbox.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		citizen_row.add_child(vbox)

		var name_label := Label.new()
		name_label.text = cname
		name_label.add_theme_font_size_override("font_size", 16)
		vbox.add_child(name_label)
		_citizen_name_labels.append(name_label)

		var emotion_label := Label.new()
		emotion_label.text = "neutral"
		vbox.add_child(emotion_label)
		_citizen_emotion_labels.append(emotion_label)

		var speech_label := Label.new()
		speech_label.text = ""
		speech_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		speech_label.custom_minimum_size = Vector2(0, 40)
		vbox.add_child(speech_label)
		_citizen_speech_labels.append(speech_label)

	# --- Separator ---
	root_panel.add_child(HSeparator.new())

	# --- Log area ---
	var log_label := Label.new()
	log_label.text = "▼ 会話ログ"
	log_label.add_theme_font_size_override("font_size", 18)
	root_panel.add_child(log_label)

	_log_scroll = ScrollContainer.new()
	_log_scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_log_scroll.custom_minimum_size = Vector2(0, 200)
	root_panel.add_child(_log_scroll)

	_conv_log = RichTextLabel.new()
	_conv_log.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_conv_log.fit_content = true
	_conv_log.scroll_active = false
	_log_scroll.add_child(_conv_log)

	# --- Voice input row ---
	var voice_row := HBoxContainer.new()
	root_panel.add_child(voice_row)

	_voice_input = LineEdit.new()
	_voice_input.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_voice_input.placeholder_text = "神の声を入力..."
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
	# Clear divine voice after it is baked into this prompt so it doesn't
	# silently influence every subsequent conversation.
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
	# Strip markdown code fences (LLMs often wrap YAML in ```yaml ... ```)
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
		if i_idx < _citizen_speech_labels.size():
			_citizen_speech_labels[i_idx].text = '"%s"' % speech
		_update_citizen_panels()

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
	if _world_sim:
		_world_sim.set_divine_voice(text)
		# Queue a divine voice reaction from citizen[0] (Kael, the most aware)
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
		if i >= _citizen_name_labels.size():
			break
		_citizen_name_labels[i].text = _world_sim.get_citizen_name(i)
		_citizen_emotion_labels[i].text = _world_sim.get_citizen_emotion(i)

## Extract a single-line YAML field value.
## Handles `field: foo bar`, `field: "foo bar"`, and `field: 'foo bar'`.
## Does NOT support YAML block scalars (| / >) — skips them as empty.
func _extract_yaml_field(text: String, field: String) -> String:
	for line in text.split("\n"):
		var stripped := line.strip_edges()
		var prefix := field + ":"
		if stripped.begins_with(prefix):
			var value := stripped.substr(prefix.length()).strip_edges()
			# Block scalar indicators are multi-line — skip, return empty
			if value == "|" or value == ">" or value == "|-" or value == ">-":
				return ""
			# Strip surrounding quotes
			if (value.begins_with('"') and value.ends_with('"')) or \
			   (value.begins_with("'") and value.ends_with("'")):
				value = value.substr(1, value.length() - 2)
			if value != "~" and value != "null" and not value.is_empty():
				return value
	return ""

## Strip markdown code fences (```yaml ... ``` or ``` ... ```) from LLM output.
func _strip_code_fences(text: String) -> String:
	var lines := text.split("\n")
	var result: PackedStringArray = []
	for line in lines:
		var stripped := line.strip_edges()
		if stripped.begins_with("```"):
			continue  # drop fence lines
		result.append(line)
	return "\n".join(result).strip_edges()

func _log_message(msg: String) -> void:
	if not _conv_log:
		return
	_conv_log.append_text(msg + "\n")
	await get_tree().process_frame
	if _log_scroll:
		_log_scroll.scroll_vertical = int(_log_scroll.get_v_scroll_bar().max_value)
