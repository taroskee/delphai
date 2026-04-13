// `godot_api` proc-macro generates Result<_, CallError> where CallError is large.
// This is inherent to gdext and outside our control.
#![allow(clippy::result_large_err)]

use delphai_core::{
    agent::citizen::{Citizen, Emotion},
    llm::{
        prompt::{build_conversation_prompt, ConversationPromptInput, WorldContext},
        provider::CitizenResponse,
    },
    pathfinding::{MoveState, TilePos, WalkGrid},
    world::{append_memory, apply_response, World},
};
use godot::prelude::*;

struct DelphaiExtension;

#[gdextension]
unsafe impl ExtensionLibrary for DelphaiExtension {}

/// Godot node that owns the Rust `World` simulation.
///
/// Usage from GDScript:
/// ```gdscript
/// $WorldSim.initialize()
/// var pending = $WorldSim.tick(randf())
/// ```
#[derive(GodotClass)]
#[class(base=Node)]
pub struct WorldNode {
    world: Option<World>,
    divine_voice: Option<String>,
    base: Base<Node>,
}

#[godot_api]
impl INode for WorldNode {
    fn init(base: Base<Node>) -> Self {
        Self {
            world: None,
            divine_voice: None,
            base,
        }
    }
}

#[godot_api]
impl WorldNode {
    /// Create the world with 3 hardcoded citizens and set their initial tile positions.
    #[func]
    fn initialize(&mut self) {
        let mut kael = make_citizen("Kael", &["curious", "optimistic"]);
        kael.divine_awareness = 1.0;

        let mut elder = make_citizen("Elder", &["wise", "cautious"]);
        elder.divine_awareness = 0.65;

        let hara = make_citizen("Hara", &["brave", "impulsive"]);

        let mut world = World::new(vec![kael, elder, hara]);

        // Place citizens at their starting tiles on the 24×14 map.
        // Kael=(7,8) wander_radius=4, Elder=(12,8) radius=3, Hara=(17,8) radius=4
        let starts: &[(i16, i16, u32)] = &[(7, 8, 4), (12, 8, 3), (17, 8, 4)];
        for (i, &(x, y, r)) in starts.iter().enumerate() {
            if let Some(state) = world.move_states.get_mut(i) {
                let pos = TilePos::new(x, y);
                *state = MoveState::new(pos, pos, r);
            }
        }

        self.world = Some(world);
    }

    /// Advance one game turn. Returns Array of {initiator_idx, partner_idx} Dictionaries.
    #[func]
    fn tick(&mut self, roll: f64) -> Array<Dictionary> {
        let Some(world) = &mut self.world else {
            return Array::new();
        };
        let pending = world.tick(roll as f32);
        let mut result: Array<Dictionary> = Array::new();
        for p in pending {
            let mut dict = Dictionary::new();
            dict.set("initiator_idx", p.initiator_idx as i64);
            dict.set("partner_idx", p.partner_idx as i64);
            result.push(&dict);
        }
        result
    }

    #[func]
    fn get_citizen_count(&self) -> i64 {
        self.world
            .as_ref()
            .map_or(0, |w| w.citizens.len() as i64)
    }

    #[func]
    fn get_citizen_name(&self, idx: i64) -> GString {
        self.world
            .as_ref()
            .and_then(|w| w.citizens.get(idx as usize))
            .map(|c| GString::from(c.name.as_str()))
            .unwrap_or_default()
    }

    #[func]
    fn get_citizen_emotion(&self, idx: i64) -> GString {
        self.world
            .as_ref()
            .and_then(|w| w.citizens.get(idx as usize))
            .map(|c| GString::from(emotion_str(&c.emotion)))
            .unwrap_or_default()
    }

    #[func]
    fn get_tick_count(&self) -> i64 {
        self.world
            .as_ref()
            .map_or(0, |w| w.tick_count as i64)
    }

    /// Store the player's divine voice. Empty string clears it.
    #[func]
    fn set_divine_voice(&mut self, voice: GString) {
        let s = voice.to_string();
        self.divine_voice = if s.is_empty() { None } else { Some(s) };
    }

    /// Clear the divine voice.
    #[func]
    fn clear_divine_voice(&mut self) {
        self.divine_voice = None;
    }

    /// Build an Ollama-ready conversation prompt for two citizens by index.
    #[func]
    fn build_conversation_prompt_str(&self, i_idx: i64, p_idx: i64) -> GString {
        let prompt =
            build_prompt_for_pair(self.world.as_ref(), i_idx as usize, p_idx as usize, self.divine_voice.as_deref());
        GString::from(prompt.unwrap_or_default())
    }

    /// Get a citizen's current divine awareness (0.0–1.0).
    #[func]
    fn get_divine_awareness(&self, citizen_idx: i64) -> f64 {
        self.world
            .as_ref()
            .and_then(|w| w.citizens.get(citizen_idx as usize))
            .map(|c| c.divine_awareness as f64)
            .unwrap_or(0.0)
    }

    /// Increase a citizen's divine awareness by `delta` (clamped to [0, 1]).
    #[func]
    fn grow_divine_awareness(&mut self, citizen_idx: i64, delta: f64) {
        let Some(world) = &mut self.world else { return };
        let Some(citizen) = world.citizens.get_mut(citizen_idx as usize) else { return };
        citizen.divine_awareness = (citizen.divine_awareness + delta as f32).clamp(0.0, 1.0);
    }

    /// Record that `listener_idx` heard `speaker_name` say `speech`.
    #[func]
    fn record_heard_speech(&mut self, listener_idx: i64, speaker_name: GString, speech: GString) {
        let Some(world) = &mut self.world else { return };
        let Some(citizen) = world.citizens.get_mut(listener_idx as usize) else { return };
        let entry = format!("{} said: \"{}\"", speaker_name, speech);
        append_memory(&mut citizen.memory_summary, &entry, "\n", 8);
    }

    /// Apply an LLM response to a citizen (speech + emotion string).
    #[func]
    fn apply_citizen_response(&mut self, idx: i64, speech: GString, emotion: GString) {
        let Some(world) = &mut self.world else { return };
        let Some(citizen) = world.citizens.get_mut(idx as usize) else { return };
        let response = CitizenResponse {
            speech: speech.to_string(),
            inner_thought: String::new(),
            action: String::new(),
            emotion_change: emotion.to_string(),
            tech_hint: None,
        };
        apply_response(citizen, &response);
    }

    /// Upload the walkability grid from GDScript (1=walkable, 0=blocked).
    /// Call this once during _ready after the TileMap is built.
    #[func]
    fn set_walkable_map(&mut self, data: PackedByteArray, width: i64, height: i64) {
        let cells: Vec<bool> = data.to_vec().iter().map(|&b| b != 0).collect();
        let grid = WalkGrid::new(width as usize, height as usize, cells);
        if let Some(world) = &mut self.world {
            world.walk_grid = Some(grid);
        }
    }

    /// Return the tile position of citizen `idx` as Vector2i(col, row).
    #[func]
    fn get_citizen_tile_pos(&self, idx: i64) -> Vector2i {
        self.world
            .as_ref()
            .and_then(|w| w.move_states.get(idx as usize))
            .map(|s| Vector2i::new(s.tile_pos.x as i32, s.tile_pos.y as i32))
            .unwrap_or_default()
    }

    /// Return the facing direction of citizen `idx`: 0=down 1=left 2=right 3=up.
    #[func]
    fn get_citizen_facing(&self, idx: i64) -> i64 {
        self.world
            .as_ref()
            .and_then(|w| w.move_states.get(idx as usize))
            .map(|s| s.facing as i64)
            .unwrap_or(0)
    }
}

/// Pure helper: build a conversation prompt given an optional World ref and indices.
fn build_prompt_for_pair(
    world: Option<&World>,
    i_idx: usize,
    p_idx: usize,
    divine_voice: Option<&str>,
) -> Option<String> {
    let world = world?;
    let initiator = world.citizens.get(i_idx)?;
    let partner = world.citizens.get(p_idx)?;
    let ctx = WorldContext {
        era: "Stone Age".into(),
        setting: "A small campfire settlement".into(),
    };
    let input = ConversationPromptInput {
        world: &ctx,
        initiator,
        partner,
        divine_voice,
    };
    Some(build_conversation_prompt(&input))
}

fn make_citizen(name: &str, tags: &[&str]) -> Citizen {
    Citizen {
        name: name.into(),
        personality_tags: tags.iter().map(|s| s.to_string()).collect(),
        memory_summary: String::new(),
        emotion: Emotion::Neutral,
        relationships: vec![],
        divine_awareness: 0.0,
    }
}

fn emotion_str(e: &Emotion) -> &'static str {
    match e {
        Emotion::Neutral => "neutral",
        Emotion::Happy => "happy",
        Emotion::Anxious => "anxious",
        Emotion::Angry => "angry",
        Emotion::Sad => "sad",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_citizen_world() -> World {
        let citizens = vec![
            make_citizen("Kael", &["curious", "optimistic"]),
            make_citizen("Elder", &["wise", "cautious"]),
        ];
        World::new(citizens)
    }

    #[test]
    fn prompt_contains_both_citizen_names() {
        let world = two_citizen_world();
        let prompt = build_prompt_for_pair(Some(&world), 0, 1, None).unwrap();
        assert!(prompt.contains("Kael"), "missing initiator name");
        assert!(prompt.contains("Elder"), "missing partner name");
    }

    #[test]
    fn prompt_returns_none_for_out_of_range_partner() {
        let world = two_citizen_world();
        assert!(build_prompt_for_pair(Some(&world), 0, 99, None).is_none());
    }

    #[test]
    fn prompt_returns_none_when_world_is_none() {
        assert!(build_prompt_for_pair(None, 0, 1, None).is_none());
    }

    #[test]
    fn prompt_includes_divine_voice_for_aware_initiator() {
        let mut world = two_citizen_world();
        world.citizens[0].divine_awareness = 1.0;
        let prompt = build_prompt_for_pair(Some(&world), 0, 1, Some("gather wood")).unwrap();
        assert!(prompt.contains("gather wood"));
    }

    #[test]
    fn prompt_excludes_voice_content_for_unaware_initiator() {
        let world = two_citizen_world();
        let prompt = build_prompt_for_pair(Some(&world), 0, 1, Some("gather wood")).unwrap();
        assert!(!prompt.contains("gather wood"), "raw text must not reach unaware citizen");
        assert!(prompt.contains("sensed"), "sensed placeholder should appear");
    }

    #[test]
    fn prompt_contains_yaml_instruction() {
        let world = two_citizen_world();
        let prompt = build_prompt_for_pair(Some(&world), 0, 1, None).unwrap();
        assert!(prompt.contains("speech"));
        assert!(prompt.contains("YAML"));
        assert!(prompt.contains("Japanese"));
    }
}
