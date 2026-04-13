extends Node2D

## Renders one citizen as an animated pixel-art sprite with speech bubble.
## Emotion animations (shake / surprise / nod) reflect divine_awareness level.

const BASE_FRAMES_PATH := "res://assets/characters/frames/"
const SPEECH_CLEAR_DELAY := 5.0

# divine_awareness → name label color
const AWARENESS_COLORS: Array = [
	Color(0.5,  0.5,  0.5,  1.0),  # 0: gray    — unaware (< 0.25)
	Color(0.95, 0.75, 0.25, 1.0),  # 1: amber   — noticing (0.25–0.55)
	Color(1.0,  1.0,  0.4,  1.0),  # 2: yellow  — trusting (0.55–0.80)
	Color(1.0,  1.0,  1.0,  1.0),  # 3: white   — devoted  (≥ 0.80)
]

var _sprite: AnimatedSprite2D
var _name_label: Label
var _emotion_label: Label
var _speech_label: Label
var _speech_bg: ColorRect
var _speech_timer: Timer
var _char_path: String

# ---------------------------------------------------------------------------
# Setup
# ---------------------------------------------------------------------------

func setup(char_name: String, char_path: String) -> void:
	_char_path = char_path
	_build_sprite()
	_build_name_labels(char_name)
	_build_speech_bubble()
	_build_speech_timer()

func _build_sprite() -> void:
	_sprite = AnimatedSprite2D.new()
	var frames := SpriteFrames.new()
	frames.remove_animation("default")

	# Base animations — every character type has down_stand + walk frames
	_add_anim(frames, "idle",    ["down_stand"],                                       1.0,  true)
	_add_anim(frames, "breathe", ["down_stand", "down_walk1", "down_stand", "down_walk2"], 3.0, true)

	# Reaction animations — chara variants have these
	_add_anim(frames, "shake",   ["shake1",   "shake2",   "shake3",   "shake2",   "shake1"],   8.0, false)
	_add_anim(frames, "nod",     ["nod1",     "nod2",     "nod3",     "nod2",     "nod1"],     6.0, false)
	_add_anim(frames, "surprise",["surprise", "down_stand", "surprise"],                        4.0, false)
	_add_anim(frames, "laugh",   ["laugh1",   "laugh2",   "laugh3",   "laugh2",   "laugh1"],   6.0, false)

	_sprite.sprite_frames = frames
	_sprite.scale = Vector2(3.0, 3.0)

	if frames.has_animation("breathe") and frames.get_frame_count("breathe") > 0:
		_sprite.play("breathe")
	elif frames.has_animation("idle"):
		_sprite.play("idle")

	_sprite.animation_finished.connect(_on_animation_finished)
	add_child(_sprite)

## Try to load each named frame from the character's directory.
## Silently skips missing frames; skips the whole animation if none load.
func _add_anim(
	frames: SpriteFrames,
	anim_name: String,
	frame_names: Array,
	fps: float,
	loop: bool
) -> void:
	var loaded: Array[Texture2D] = []
	for fname: String in frame_names:
		var path := BASE_FRAMES_PATH + _char_path + "/" + fname + ".png"
		if ResourceLoader.exists(path):
			var tex := load(path) as Texture2D
			if tex:
				loaded.append(tex)

	if loaded.is_empty():
		return

	frames.add_animation(anim_name)
	frames.set_animation_speed(anim_name, fps)
	frames.set_animation_loop(anim_name, loop)
	for tex in loaded:
		frames.add_frame(anim_name, tex)

func _build_name_labels(char_name: String) -> void:
	_name_label = Label.new()
	_name_label.text = char_name
	_name_label.position = Vector2(-24, 28)
	_name_label.add_theme_color_override("font_color", AWARENESS_COLORS[0])
	_name_label.add_theme_font_size_override("font_size", 13)
	add_child(_name_label)

	_emotion_label = Label.new()
	_emotion_label.text = "neutral"
	_emotion_label.position = Vector2(-24, 44)
	_emotion_label.add_theme_color_override("font_color", Color(0.65, 0.65, 0.65, 1.0))
	_emotion_label.add_theme_font_size_override("font_size", 9)
	add_child(_emotion_label)

func _build_speech_bubble() -> void:
	_speech_bg = ColorRect.new()
	_speech_bg.color = Color(0.0, 0.0, 0.0, 0.72)
	_speech_bg.position = Vector2(-78, -130)
	_speech_bg.size = Vector2(156, 66)
	_speech_bg.modulate.a = 0.0
	add_child(_speech_bg)

	_speech_label = Label.new()
	_speech_label.position = Vector2(-73, -126)
	_speech_label.custom_minimum_size = Vector2(146, 0)
	_speech_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_speech_label.add_theme_color_override("font_color", Color.WHITE)
	_speech_label.add_theme_font_size_override("font_size", 10)
	_speech_label.modulate.a = 0.0
	add_child(_speech_label)

func _build_speech_timer() -> void:
	_speech_timer = Timer.new()
	_speech_timer.one_shot = true
	_speech_timer.timeout.connect(_clear_speech)
	add_child(_speech_timer)

# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------

## Show a speech bubble above the sprite, auto-clear after SPEECH_CLEAR_DELAY.
func show_speech(text: String) -> void:
	var display := text.left(72) + ("…" if text.length() > 72 else "")
	_speech_label.text = display
	_speech_label.modulate.a = 1.0
	_speech_bg.modulate.a = 1.0
	_speech_timer.start(SPEECH_CLEAR_DELAY)

## Play a reaction animation that reflects the citizen's current divine_awareness.
## Low awareness → skeptical shake. Mid → surprise. High → trusting nod.
func play_reaction(awareness: float) -> void:
	_update_name_color(awareness)
	if awareness < 0.25:
		_play_once("shake")
	elif awareness < 0.55:
		_play_once("surprise")
	else:
		_play_once("nod")

## Update the small emotion text below the name.
func set_emotion_label(emotion: String) -> void:
	if _emotion_label:
		_emotion_label.text = emotion

## Tween-interpolate this node to screen_pos over one step duration.
## Plays "breathe" while moving; returns to "breathe" (looping) on arrival.
func walk_to(screen_pos: Vector2) -> void:
	var tween := create_tween()
	tween.tween_property(self, "position", screen_pos, 0.12)

## Update sprite facing based on Rust pathfinding direction.
## dir: 0=down  1=left  2=right  3=up
func set_facing(dir: int) -> void:
	if not _sprite:
		return
	match dir:
		1:  # left — flip sprite horizontally
			_sprite.flip_h = true
		2:  # right — reset flip
			_sprite.flip_h = false
		_:  # up/down — keep current flip, only h-flip matters with this spritesheet
			pass

# ---------------------------------------------------------------------------
# Private
# ---------------------------------------------------------------------------

func _play_once(anim: String) -> void:
	if _sprite and _sprite.sprite_frames and _sprite.sprite_frames.has_animation(anim):
		_sprite.play(anim)

func _on_animation_finished() -> void:
	if not _sprite or not _sprite.sprite_frames:
		return
	if _sprite.sprite_frames.has_animation("breathe"):
		_sprite.play("breathe")
	elif _sprite.sprite_frames.has_animation("idle"):
		_sprite.play("idle")

func _update_name_color(awareness: float) -> void:
	var tier := 0
	if   awareness >= 0.80: tier = 3
	elif awareness >= 0.55: tier = 2
	elif awareness >= 0.25: tier = 1
	if _name_label:
		_name_label.add_theme_color_override("font_color", AWARENESS_COLORS[tier])

func _clear_speech() -> void:
	if _speech_label: _speech_label.modulate.a = 0.0
	if _speech_bg:    _speech_bg.modulate.a    = 0.0
