use crate::agent::{
    behavior::{tick as behavior_tick, BehaviorAction, BehaviorState, Needs},
    citizen::{Citizen, Emotion},
    conversation::check_conversations,
};
use crate::llm::{
    provider::CitizenResponse,
    queue::{InferencePriority, InferenceQueue, InferenceRequest},
};
use crate::pathfinding::{step_citizen, MoveState, TilePos, WalkGrid};

/// Maximum manhattan distance (tiles) for two citizens to start a conversation.
const CONVERSATION_PROXIMITY_TILES: u32 = 4;
const CONVERSATION_PROBABILITY: f32 = 0.5;
/// Maximum number of entries kept in a citizen's memory_summary.
const MEMORY_MAX_ENTRIES: usize = 8;

/// Per-citizen resource needs, tracked by the world each tick.
#[derive(Debug, Clone)]
pub struct CitizenNeeds {
    pub hunger: f32,
    pub fatigue: f32,
}

impl CitizenNeeds {
    pub fn new(hunger: f32, fatigue: f32) -> Self {
        Self { hunger, fatigue }
    }
}

impl Default for CitizenNeeds {
    fn default() -> Self {
        Self {
            hunger: 0.0,
            fatigue: 0.0,
        }
    }
}

/// A citizen pair that needs LLM inference this tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingConversation {
    pub initiator_idx: usize,
    pub partner_idx: usize,
}

/// Holds all mutable world state. `tick()` advances one game turn.
pub struct World {
    pub citizens: Vec<Citizen>,
    pub behavior_states: Vec<BehaviorState>,
    pub needs: Vec<CitizenNeeds>,
    pub move_states: Vec<MoveState>,
    pub walk_grid: Option<WalkGrid>,
    pub queue: InferenceQueue,
    pub tick_count: u64,
}

impl World {
    pub fn new(citizens: Vec<Citizen>) -> Self {
        let n = citizens.len();
        Self {
            behavior_states: vec![BehaviorState::default(); n],
            needs: vec![CitizenNeeds::default(); n],
            move_states: (0..n)
                .map(|_| MoveState::new(TilePos::default(), TilePos::default(), 4))
                .collect(),
            walk_grid: None,
            queue: InferenceQueue::new(3),
            tick_count: 0,
            citizens,
        }
    }

    /// Advance one game turn synchronously.
    ///
    /// Returns citizen index pairs that should be sent to the LLM.
    /// The caller drives the async inference and calls `apply_response()` on results.
    pub fn tick(&mut self, random_roll: f32) -> Vec<PendingConversation> {
        self.tick_count += 1;

        // Step 1: update behavior states
        for i in 0..self.citizens.len() {
            let action = behavior_tick(
                self.behavior_states[i],
                &Needs {
                    hunger: self.needs[i].hunger,
                    fatigue: self.needs[i].fatigue,
                },
            );
            if let BehaviorAction::TransitionTo(next) = action {
                self.behavior_states[i] = next;
            }
        }

        // Step 2: drive movement (only when walk_grid is set)
        // Use take/replace to satisfy the borrow checker: &WalkGrid + &mut MoveState simultaneously.
        if let Some(grid) = self.walk_grid.take() {
            for (i, state) in self.move_states.iter_mut().enumerate() {
                if self.behavior_states[i] == BehaviorState::Idle {
                    let seed = self
                        .tick_count
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add(i as u64);
                    step_citizen(state, &grid, seed);
                }
            }
            self.walk_grid = Some(grid);
        }

        // Step 3: find conversation candidates
        let citizen_tuples: Vec<(String, TilePos, BehaviorState)> = self
            .citizens
            .iter()
            .enumerate()
            .map(|(i, c)| (c.name.clone(), self.move_states[i].tile_pos, self.behavior_states[i]))
            .collect();

        let requests = check_conversations(
            &citizen_tuples,
            CONVERSATION_PROXIMITY_TILES,
            CONVERSATION_PROBABILITY,
            random_roll,
        );

        // Step 4: enqueue requests and collect pending conversations
        let mut pending = Vec::new();
        for req in requests {
            let initiator_idx = self
                .citizens
                .iter()
                .position(|c| c.name == req.initiator)
                .expect("initiator must exist in world");
            let partner_idx = self
                .citizens
                .iter()
                .position(|c| c.name == req.partner)
                .expect("partner must exist in world");

            self.queue.push(InferenceRequest {
                priority: InferencePriority::Normal,
                tag: format!("{}-{}", req.initiator, req.partner),
                initiator: self.citizens[initiator_idx].clone(),
                partner: Some(self.citizens[partner_idx].clone()),
            });

            pending.push(PendingConversation {
                initiator_idx,
                partner_idx,
            });
        }

        pending
    }
}

/// Map a `CitizenResponse` back onto a `Citizen`.
///
/// Updates `emotion` and appends `speech` to `memory_summary`.
pub fn apply_response(citizen: &mut Citizen, response: &CitizenResponse) {
    citizen.emotion = parse_emotion(&response.emotion_change);

    if !response.speech.is_empty() {
        append_memory(&mut citizen.memory_summary, &response.speech, " | ", MEMORY_MAX_ENTRIES);
    }

    if let Some(hint) = &response.tech_hint {
        eprintln!("[tech_hint] {}: {hint}", citizen.name);
    }
}

/// Append `entry` to `summary` using `sep` as separator.
/// Keeps at most `max_entries` entries by dropping the oldest when the limit is exceeded.
pub fn append_memory(summary: &mut String, entry: &str, sep: &str, max_entries: usize) {
    if summary.is_empty() {
        *summary = entry.to_string();
        return;
    }
    summary.push_str(sep);
    summary.push_str(entry);
    let entries: Vec<&str> = summary.split(sep).collect();
    if entries.len() > max_entries {
        *summary = entries[entries.len() - max_entries..].join(sep);
    }
}

fn parse_emotion(s: &str) -> Emotion {
    match s.to_lowercase().as_str() {
        "happy" | "joy" | "joyful" => Emotion::Happy,
        "angry" | "anger" | "rage" => Emotion::Angry,
        "sad" | "grief" | "sorrow" => Emotion::Sad,
        "anxious" | "fear" | "scared" | "worried" => Emotion::Anxious,
        _ => Emotion::Neutral,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::citizen::Emotion;

    fn make_citizen(name: &str) -> Citizen {
        Citizen {
            name: name.into(),
            personality_tags: vec![],
            memory_summary: String::new(),
            emotion: Emotion::Neutral,
            relationships: vec![],
            divine_awareness: 0.0,
        }
    }

    /// Build a world with citizens placed at the given tile positions.
    fn make_world_with_tiles(names: &[&str], tiles: Vec<TilePos>) -> World {
        let citizens: Vec<Citizen> = names.iter().map(|n| make_citizen(n)).collect();
        let mut world = World::new(citizens);
        for (i, pos) in tiles.into_iter().enumerate() {
            if let Some(s) = world.move_states.get_mut(i) {
                s.tile_pos = pos;
                s.wander_center = pos;
            }
        }
        world
    }

    /// Citizens within CONVERSATION_PROXIMITY_TILES of each other.
    fn close_tiles(n: usize) -> Vec<TilePos> {
        (0..n).map(|i| TilePos::new(i as i16 * 2, 0)).collect()
    }

    /// Citizens spread far apart (> CONVERSATION_PROXIMITY_TILES).
    fn spread_tiles(n: usize) -> Vec<TilePos> {
        (0..n).map(|i| TilePos::new(i as i16 * 20, 0)).collect()
    }

    // --- tick: behavior state ---

    #[test]
    fn tick_advances_behavior_states_to_sleep_when_fatigued() {
        let mut world = make_world_with_tiles(&["A"], vec![TilePos::new(0, 0)]);
        world.needs[0].fatigue = 0.9;

        world.tick(0.0);

        assert_eq!(world.behavior_states[0], BehaviorState::Sleeping);
    }

    #[test]
    fn tick_increments_tick_count() {
        let mut world = make_world_with_tiles(&["A"], vec![TilePos::new(0, 0)]);
        assert_eq!(world.tick_count, 0);
        world.tick(1.0);
        assert_eq!(world.tick_count, 1);
        world.tick(1.0);
        assert_eq!(world.tick_count, 2);
    }

    // --- tick: conversation pending ---

    #[test]
    fn tick_returns_pending_for_idle_close_pair() {
        let mut world = make_world_with_tiles(&["Kael", "Elder"], close_tiles(2));
        let pending = world.tick(0.0);
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].initiator_idx, 0);
        assert_eq!(pending[0].partner_idx, 1);
    }

    #[test]
    fn tick_no_pending_when_all_sleeping() {
        let mut world = make_world_with_tiles(&["A", "B"], close_tiles(2));
        world.behavior_states[0] = BehaviorState::Sleeping;
        world.behavior_states[1] = BehaviorState::Sleeping;
        world.needs[0].fatigue = 0.5;
        world.needs[1].fatigue = 0.5;

        let pending = world.tick(0.0);
        assert!(pending.is_empty());
    }

    #[test]
    fn tick_no_pending_when_citizens_far_apart() {
        let mut world = make_world_with_tiles(&["A", "B"], spread_tiles(2));
        let pending = world.tick(0.0);
        assert!(pending.is_empty());
    }

    #[test]
    fn tick_no_pending_when_roll_exceeds_probability() {
        let mut world = make_world_with_tiles(&["A", "B"], close_tiles(2));
        let pending = world.tick(1.0);
        assert!(pending.is_empty());
    }

    #[test]
    fn tick_enqueues_request_to_queue() {
        let mut world = make_world_with_tiles(&["A", "B"], close_tiles(2));
        assert!(world.queue.is_empty());
        world.tick(0.0);
        assert!(!world.queue.is_empty());
    }

    // --- apply_response ---

    #[test]
    fn apply_response_updates_emotion() {
        let mut citizen = make_citizen("Kael");
        let response = CitizenResponse {
            speech: String::new(),
            inner_thought: String::new(),
            action: String::new(),
            emotion_change: "happy".into(),
            tech_hint: None,
        };
        apply_response(&mut citizen, &response);
        assert_eq!(citizen.emotion, Emotion::Happy);
    }

    #[test]
    fn apply_response_maps_angry() {
        let mut citizen = make_citizen("Kael");
        let response = CitizenResponse {
            speech: String::new(),
            inner_thought: String::new(),
            action: String::new(),
            emotion_change: "angry".into(),
            tech_hint: None,
        };
        apply_response(&mut citizen, &response);
        assert_eq!(citizen.emotion, Emotion::Angry);
    }

    #[test]
    fn apply_response_unknown_emotion_falls_back_to_neutral() {
        let mut citizen = make_citizen("Kael");
        citizen.emotion = Emotion::Happy;
        let response = CitizenResponse {
            speech: String::new(),
            inner_thought: String::new(),
            action: String::new(),
            emotion_change: "perplexed".into(),
            tech_hint: None,
        };
        apply_response(&mut citizen, &response);
        assert_eq!(citizen.emotion, Emotion::Neutral);
    }

    #[test]
    fn apply_response_appends_speech_to_empty_memory() {
        let mut citizen = make_citizen("Kael");
        let response = CitizenResponse {
            speech: "Let us gather food".into(),
            inner_thought: String::new(),
            action: String::new(),
            emotion_change: "neutral".into(),
            tech_hint: None,
        };
        apply_response(&mut citizen, &response);
        assert_eq!(citizen.memory_summary, "Let us gather food");
    }

    #[test]
    fn apply_response_appends_speech_to_existing_memory() {
        let mut citizen = make_citizen("Kael");
        citizen.memory_summary = "found berries".into();
        let response = CitizenResponse {
            speech: "Let us gather food".into(),
            inner_thought: String::new(),
            action: String::new(),
            emotion_change: "neutral".into(),
            tech_hint: None,
        };
        apply_response(&mut citizen, &response);
        assert_eq!(citizen.memory_summary, "found berries | Let us gather food");
    }

    #[test]
    fn append_memory_caps_at_max_entries() {
        let mut summary = String::new();
        for i in 0..10 {
            append_memory(&mut summary, &format!("entry{i}"), " | ", 8);
        }
        let entries: Vec<&str> = summary.split(" | ").collect();
        assert_eq!(entries.len(), 8);
        assert_eq!(entries[0], "entry2");
        assert_eq!(entries[7], "entry9");
    }

    #[test]
    fn append_memory_within_limit_keeps_all() {
        let mut summary = String::new();
        for i in 0..5 {
            append_memory(&mut summary, &format!("e{i}"), " | ", 8);
        }
        let entries: Vec<&str> = summary.split(" | ").collect();
        assert_eq!(entries.len(), 5);
    }

    #[test]
    fn apply_response_empty_speech_leaves_memory_unchanged() {
        let mut citizen = make_citizen("Kael");
        citizen.memory_summary = "prior memory".into();
        let response = CitizenResponse {
            speech: String::new(),
            inner_thought: String::new(),
            action: String::new(),
            emotion_change: "neutral".into(),
            tech_hint: None,
        };
        apply_response(&mut citizen, &response);
        assert_eq!(citizen.memory_summary, "prior memory");
    }
}
