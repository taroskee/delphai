// `godot_api` proc-macro generates Result<_, CallError> where CallError is large.
// This is inherent to gdext and outside our control.
#![allow(clippy::result_large_err)]

use delphai_core::{
    agent::citizen::{Citizen, Emotion},
    llm::{
        prompt::{build_conversation_prompt, ConversationPromptInput, WorldContext},
        provider::CitizenResponse,
    },
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
    /// Create the world with 3 hardcoded citizens for the campfire demo.
    #[func]
    fn initialize(&mut self) {
        let mut kael = make_citizen("Kael", &["curious", "optimistic"]);
        kael.divine_awareness = 1.0; // Kael hears the divine voice clearly

        let mut elder = make_citizen("Elder", &["wise", "cautious"]);
        elder.divine_awareness = 0.65; // Elder hears it as "oracle" fragments

        let hara = make_citizen("Hara", &["brave", "impulsive"]);
        // Hara has 0.0 awareness — the voice doesn't reach her

        self.world = Some(World::new(vec![kael, elder, hara]));
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
    /// Returns empty string if world is uninitialised or indices are out of range.
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
    /// Call this each time the player's voice reaches the world.
    #[func]
    fn grow_divine_awareness(&mut self, citizen_idx: i64, delta: f64) {
        let Some(world) = &mut self.world else { return };
        let Some(citizen) = world.citizens.get_mut(citizen_idx as usize) else { return };
        citizen.divine_awareness = (citizen.divine_awareness + delta as f32).clamp(0.0, 1.0);
    }

    /// Record that `listener_idx` heard `speaker_name` say `speech`.
    /// Appended to memory_summary so the next prompt includes it as context.
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
        let Some(world) = &mut self.world else {
            return;
        };
        let Some(citizen) = world.citizens.get_mut(idx as usize) else {
            return;
        };
        let response = CitizenResponse {
            speech: speech.to_string(),
            inner_thought: String::new(),
            action: String::new(),
            emotion_change: emotion.to_string(),
            tech_hint: None,
        };
        apply_response(citizen, &response);
    }
}

/// Pure helper: build a conversation prompt given an optional World ref and indices.
/// Extracted from WorldNode so it can be unit-tested without a Godot runtime.
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
        assert!(prompt.contains("gather wood"), "divine voice should appear for aware citizen");
    }

    #[test]
    fn prompt_excludes_voice_content_for_unaware_initiator() {
        // awareness=0: sensed placeholder injected but raw text must not appear
        let world = two_citizen_world(); // divine_awareness = 0.0
        let prompt = build_prompt_for_pair(Some(&world), 0, 1, Some("gather wood")).unwrap();
        assert!(!prompt.contains("gather wood"), "raw text must not reach unaware citizen");
        assert!(prompt.contains("sensed"), "sensed placeholder should appear");
    }

    #[test]
    fn prompt_contains_yaml_instruction() {
        let world = two_citizen_world();
        let prompt = build_prompt_for_pair(Some(&world), 0, 1, None).unwrap();
        assert!(prompt.contains("speech"), "missing speech field");
        assert!(prompt.contains("YAML"), "missing YAML format instruction");
        assert!(prompt.contains("Japanese"), "missing Japanese language instruction");
    }
}
