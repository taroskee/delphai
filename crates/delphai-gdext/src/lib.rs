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
    resource::Resource,
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
/// $WorldSim.tick(randf())
/// ```
#[derive(GodotClass)]
#[class(base=Node)]
pub struct WorldNode {
    world: Option<World>,
    base: Base<Node>,
}

#[godot_api]
impl INode for WorldNode {
    fn init(base: Base<Node>) -> Self {
        Self { world: None, base }
    }
}

#[godot_api]
impl WorldNode {
    /// Create the world with citizens and seed resources.
    #[func]
    fn initialize(&mut self) {
        let mut kael = make_citizen("Kael", &["curious", "optimistic"]);
        kael.divine_awareness = 1.0;

        let mut elder = make_citizen("Elder", &["wise", "cautious"]);
        elder.divine_awareness = 0.65;

        let hara = make_citizen("Hara", &["brave", "impulsive"]);

        let mut world = World::new(vec![kael, elder, hara]);

        // Place citizens at starting tiles on a 24×14 map.
        let starts: &[(i16, i16, u32)] = &[(7, 8, 4), (12, 8, 3), (17, 8, 4)];
        for (i, &(x, y, r)) in starts.iter().enumerate() {
            if let Some(state) = world.move_states.get_mut(i) {
                let pos = TilePos::new(x, y);
                *state = MoveState::new(pos, pos, r);
            }
        }

        // Seed some resources — positions adjusted when real map is available.
        world.add_resource(Resource::berry_bush(TilePos::new(5, 5)));
        world.add_resource(Resource::berry_bush(TilePos::new(15, 10)));
        world.add_resource(Resource::water_source(TilePos::new(20, 3)));

        self.world = Some(world);
    }

    /// Advance one game turn.
    #[func]
    fn tick(&mut self, roll: f64) {
        if let Some(world) = &mut self.world {
            world.tick(roll as f32);
        }
    }

    #[func]
    fn get_citizen_count(&self) -> i64 {
        self.world.as_ref().map_or(0, |w| w.citizens.len() as i64)
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
    fn get_citizen_fed(&self, idx: i64) -> f64 {
        self.world
            .as_ref()
            .and_then(|w| w.vitals.get(idx as usize))
            .map(|v| v.fed as f64)
            .unwrap_or(0.0)
    }

    #[func]
    fn get_citizen_hydration(&self, idx: i64) -> f64 {
        self.world
            .as_ref()
            .and_then(|w| w.vitals.get(idx as usize))
            .map(|v| v.hydration as f64)
            .unwrap_or(0.0)
    }

    #[func]
    fn get_citizen_behavior(&self, idx: i64) -> GString {
        self.world
            .as_ref()
            .and_then(|w| w.behavior_states.get(idx as usize))
            .map(|s| GString::from(s.as_str()))
            .unwrap_or_default()
    }

    #[func]
    fn get_tick_count(&self) -> i64 {
        self.world.as_ref().map_or(0, |w| w.tick_count as i64)
    }

    /// Upload the walkability grid from GDScript (1=walkable, 0=blocked).
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

    /// Resource count exposed to GDScript for placing resource nodes in the scene.
    #[func]
    fn get_resource_count(&self) -> i64 {
        self.world.as_ref().map_or(0, |w| w.resources.len() as i64)
    }

    /// Tile position of resource `idx`.
    #[func]
    fn get_resource_pos(&self, idx: i64) -> Vector2i {
        self.world
            .as_ref()
            .and_then(|w| w.resources.get(idx as usize))
            .map(|r| Vector2i::new(r.pos.x as i32, r.pos.y as i32))
            .unwrap_or_default()
    }

    /// Kind string of resource `idx`: "berry_bush" or "water_source".
    #[func]
    fn get_resource_kind(&self, idx: i64) -> GString {
        self.world
            .as_ref()
            .and_then(|w| w.resources.get(idx as usize))
            .map(|r| GString::from(r.kind.as_str()))
            .unwrap_or_default()
    }

    /// Current quantity of resource `idx` (0.0–1.0 for berry_bush; 1.0 for water_source).
    #[func]
    fn get_resource_quantity(&self, idx: i64) -> f64 {
        self.world
            .as_ref()
            .and_then(|w| w.resources.get(idx as usize))
            .map(|r| r.quantity.min(1.0) as f64)
            .unwrap_or(0.0)
    }

    // -------------------------------------------------------------------------
    // Phase 2 LLM hooks — preserved for reintegration, not wired into tick().
    // -------------------------------------------------------------------------

    /// Build an Ollama-ready conversation prompt for two citizens by index.
    #[func]
    fn build_conversation_prompt_str(&self, i_idx: i64, p_idx: i64) -> GString {
        let prompt =
            build_prompt_for_pair(self.world.as_ref(), i_idx as usize, p_idx as usize);
        GString::from(prompt.unwrap_or_default())
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

    /// Record that `listener_idx` heard `speaker_name` say `speech`.
    #[func]
    fn record_heard_speech(&mut self, listener_idx: i64, speaker_name: GString, speech: GString) {
        let Some(world) = &mut self.world else { return };
        let Some(citizen) = world.citizens.get_mut(listener_idx as usize) else { return };
        let entry = format!("{} said: \"{}\"", speaker_name, speech);
        append_memory(&mut citizen.memory_summary, &entry, "\n", 8);
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
}

/// Pure helper: build a conversation prompt given an optional World ref and indices.
fn build_prompt_for_pair(world: Option<&World>, i_idx: usize, p_idx: usize) -> Option<String> {
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
        divine_voice: None,
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
        let prompt = build_prompt_for_pair(Some(&world), 0, 1).unwrap();
        assert!(prompt.contains("Kael"), "missing initiator name");
        assert!(prompt.contains("Elder"), "missing partner name");
    }

    #[test]
    fn prompt_returns_none_for_out_of_range_partner() {
        let world = two_citizen_world();
        assert!(build_prompt_for_pair(Some(&world), 0, 99).is_none());
    }

    #[test]
    fn prompt_returns_none_when_world_is_none() {
        assert!(build_prompt_for_pair(None, 0, 1).is_none());
    }

    #[test]
    fn prompt_contains_yaml_instruction() {
        let world = two_citizen_world();
        let prompt = build_prompt_for_pair(Some(&world), 0, 1).unwrap();
        assert!(prompt.contains("speech"));
        assert!(prompt.contains("YAML"));
        assert!(prompt.contains("Japanese"));
    }
}
