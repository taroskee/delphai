use crate::agent::{
    behavior::{tick as behavior_tick, BehaviorAction, BehaviorState, Needs},
    citizen::{Citizen, Emotion},
};
use crate::animal::Animal;
use crate::llm::provider::CitizenResponse;
use crate::pathfinding::{step_citizen, MoveState, TilePos, WalkGrid};
use crate::resource::{Resource, ResourceKind, BERRY_BUSH_RESPAWN_TICKS};
use crate::tech::TechTree;

/// Maximum number of entries kept in a citizen's memory_summary.
const MEMORY_MAX_ENTRIES: usize = 8;

/// Per-citizen vitals. Bigger-is-better: 1.0 = fully sated, 0.0 = critical.
#[derive(Debug, Clone)]
pub struct CitizenVitals {
    pub fed: f32,
    pub hydration: f32,
}

impl CitizenVitals {
    pub fn new(fed: f32, hydration: f32) -> Self {
        Self { fed, hydration }
    }
}

impl Default for CitizenVitals {
    fn default() -> Self {
        Self { fed: 1.0, hydration: 1.0 }
    }
}

const FED_DECAY_PER_TICK: f32 = 0.004;
const HYDRATION_DECAY_PER_TICK: f32 = 0.007;
const GATHER_RATE: f32 = 0.05;
const DRINK_RATE: f32 = 0.08;
const MAX_CITIZENS: usize = 8;
/// Food gained by each hunter when a kill succeeds.
const HUNT_FED_GAIN: f32 = 0.5;
/// All citizens must have fed > this AND hydration > this to accumulate prosperity.
const PROSPERITY_VITALS_THRESHOLD: f32 = 0.8;
/// Prosperity ticks needed before a new citizen is born.
const BIRTH_THRESHOLD: u32 = 200;

/// Spawn position for new citizens (roughly map center on a 24×14 grid).
const BIRTH_TILE: TilePos = TilePos { x: 12, y: 7 };

/// Names assigned to born citizens (cycles through the pool).
const CITIZEN_NAMES: &[&str] = &[
    "Rael", "Mira", "Thorn", "Sola", "Garan", "Lysa", "Brek", "Asha",
];

/// Holds all mutable world state. `tick()` advances one game turn.
pub struct World {
    pub citizens: Vec<Citizen>,
    pub behavior_states: Vec<BehaviorState>,
    pub vitals: Vec<CitizenVitals>,
    pub move_states: Vec<MoveState>,
    pub walk_grid: Option<WalkGrid>,
    pub resources: Vec<Resource>,
    pub animals: Vec<Animal>,
    /// Preserved for Phase 2 reintegration — not wired into Phase 1 gameplay.
    pub tech_tree: TechTree,
    pub tick_count: u64,
    /// Accumulated ticks where all citizens are well-fed and hydrated.
    pub prosperity_ticks: u32,
    /// Number of citizens born since last polled by GDScript.
    pub pending_births: u32,
}

impl World {
    pub fn new(citizens: Vec<Citizen>) -> Self {
        let n = citizens.len();
        Self {
            behavior_states: vec![BehaviorState::default(); n],
            vitals: vec![CitizenVitals::default(); n],
            move_states: (0..n)
                .map(|_| MoveState::new(TilePos::default(), TilePos::default(), 4))
                .collect(),
            walk_grid: None,
            resources: Vec::new(),
            animals: Vec::new(),
            tech_tree: TechTree::new(),
            tick_count: 0,
            prosperity_ticks: 0,
            pending_births: 0,
            citizens,
        }
    }

    pub fn add_animal(&mut self, a: Animal) {
        self.animals.push(a);
    }

    /// Return the tile position of the nearest alive animal.
    pub fn nearest_animal_pos(&self, from: TilePos) -> Option<TilePos> {
        self.animals
            .iter()
            .filter(|a| a.alive)
            .min_by_key(|a| a.pos.manhattan_dist(from))
            .map(|a| a.pos)
    }


    /// Attempt to spawn a new citizen when the tribe is thriving.
    fn maybe_spawn_citizen(&mut self) {
        let all_thriving = self.vitals.iter().all(|v| {
            v.fed > PROSPERITY_VITALS_THRESHOLD && v.hydration > PROSPERITY_VITALS_THRESHOLD
        });
        if all_thriving && self.citizens.len() < MAX_CITIZENS {
            self.prosperity_ticks += 1;
            if self.prosperity_ticks >= BIRTH_THRESHOLD {
                self.prosperity_ticks = 0;
                self.birth_citizen();
            }
        } else {
            self.prosperity_ticks = 0;
        }
    }

    fn birth_citizen(&mut self) {
        let name = CITIZEN_NAMES[self.citizens.len() % CITIZEN_NAMES.len()].to_string();
        let citizen = Citizen {
            name,
            personality_tags: vec![],
            memory_summary: String::new(),
            emotion: Emotion::Happy,
            relationships: vec![],
            divine_awareness: 0.0,
        };
        self.citizens.push(citizen);
        self.behavior_states.push(BehaviorState::default());
        self.vitals.push(CitizenVitals::default());
        self.move_states.push(MoveState::new(BIRTH_TILE, BIRTH_TILE, 4));
        self.pending_births += 1;
    }

    pub fn add_resource(&mut self, r: Resource) {
        self.resources.push(r);
    }

    /// Return the tile position of the nearest available resource of `kind`.
    pub fn nearest_resource_pos(&self, from: TilePos, kind: ResourceKind) -> Option<TilePos> {
        self.resources
            .iter()
            .filter(|r| r.kind == kind && r.is_available())
            .min_by_key(|r| r.pos.manhattan_dist(from))
            .map(|r| r.pos)
    }

    /// Advance one game turn synchronously.
    pub fn tick(&mut self, _random_roll: f32) {
        self.tick_count += 1;

        // Step 1: decay vitals
        for v in &mut self.vitals {
            v.fed = (v.fed - FED_DECAY_PER_TICK).max(0.0);
            v.hydration = (v.hydration - HYDRATION_DECAY_PER_TICK).max(0.0);
        }

        // Step 2: tick resources (respawn timers)
        for r in &mut self.resources {
            r.tick();
        }

        // Step 3: update behavior states from needs.
        // After behavior_tick, override Idle→Hunting when hungry and animals exist.
        let any_alive_animal = self.animals.iter().any(|a| a.alive);
        for i in 0..self.citizens.len() {
            let action = behavior_tick(
                self.behavior_states[i],
                &Needs {
                    fed: self.vitals[i].fed,
                    hydration: self.vitals[i].hydration,
                },
            );
            if let BehaviorAction::TransitionTo(next) = action {
                self.behavior_states[i] = next;
            }
            // Prefer hunting over berry-gathering when fed < threshold and animals available.
            if self.behavior_states[i] == BehaviorState::SeekingFood
                && any_alive_animal
                && self.vitals[i].fed > 0.15 // still strong enough to hunt
            {
                self.behavior_states[i] = BehaviorState::Hunting;
            }
        }

        // Step 4a: stationary resource interactions (no grid required).
        for i in 0..self.citizens.len() {
            match self.behavior_states[i] {
                BehaviorState::Gathering => {
                    let pos = self.move_states[i].tile_pos;
                    let mut gathered = 0.0_f32;
                    for r in &mut self.resources {
                        if r.kind == ResourceKind::BerryBush && r.pos == pos && r.is_available() {
                            let take = GATHER_RATE.min(r.quantity);
                            r.deplete_with_respawn(take, BERRY_BUSH_RESPAWN_TICKS);
                            gathered = take;
                            break;
                        }
                    }
                    self.vitals[i].fed = (self.vitals[i].fed + gathered).min(1.0);

                    // If the bush ran out, go back to seeking
                    let still_available = self.resources.iter().any(|r| {
                        r.kind == ResourceKind::BerryBush && r.pos == pos && r.is_available()
                    });
                    if !still_available {
                        self.behavior_states[i] = BehaviorState::SeekingFood;
                    }
                }
                BehaviorState::Drinking => {
                    self.vitals[i].hydration = (self.vitals[i].hydration + DRINK_RATE).min(1.0);
                }
                _ => {}
            }
        }

        // Step 4b: movement (requires grid).
        // Use take/replace to satisfy the borrow checker.
        if let Some(grid) = self.walk_grid.take() {
            for i in 0..self.citizens.len() {
                let seed = self
                    .tick_count
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(i as u64 * 2654435761);

                match self.behavior_states[i] {
                    BehaviorState::Idle => {
                        step_citizen(&mut self.move_states[i], &grid, seed);
                    }
                    BehaviorState::SeekingFood => {
                        let from = self.move_states[i].tile_pos;
                        if let Some(target) = self.nearest_resource_pos(from, ResourceKind::BerryBush) {
                            if target == from {
                                let available = self.resources.iter().any(|r| {
                                    r.kind == ResourceKind::BerryBush && r.pos == target && r.is_available()
                                });
                                if available {
                                    self.behavior_states[i] = BehaviorState::Gathering;
                                }
                            } else {
                                self.move_states[i].move_target = Some(target);
                                step_citizen(&mut self.move_states[i], &grid, seed);
                            }
                        } else {
                            step_citizen(&mut self.move_states[i], &grid, seed);
                        }
                    }
                    BehaviorState::SeekingWater => {
                        let from = self.move_states[i].tile_pos;
                        if let Some(target) = self.nearest_resource_pos(from, ResourceKind::WaterSource) {
                            if target == from {
                                self.behavior_states[i] = BehaviorState::Drinking;
                            } else {
                                self.move_states[i].move_target = Some(target);
                                step_citizen(&mut self.move_states[i], &grid, seed);
                            }
                        } else {
                            step_citizen(&mut self.move_states[i], &grid, seed);
                        }
                    }
                    BehaviorState::Hunting => {
                        let from = self.move_states[i].tile_pos;
                        if let Some(target) = self.nearest_animal_pos(from) {
                            if target != from {
                                self.move_states[i].move_target = Some(target);
                                step_citizen(&mut self.move_states[i], &grid, seed);
                            }
                            // At target tile: wait for other hunters (resolved in step 4c).
                        } else {
                            // No animals alive — fall back to berry gathering.
                            self.behavior_states[i] = BehaviorState::SeekingFood;
                        }
                    }
                    // Gathering and Drinking are handled in step 4a — no movement.
                    BehaviorState::Gathering | BehaviorState::Drinking => {}
                }
            }
            self.walk_grid = Some(grid);
        }

        // Step 4c: cooperative hunting — kill animals with 2+ hunters on same tile.
        for ai in 0..self.animals.len() {
            if !self.animals[ai].alive {
                continue;
            }
            let animal_pos = self.animals[ai].pos;
            let hunters_here: Vec<usize> = (0..self.citizens.len())
                .filter(|&ci| {
                    self.behavior_states[ci] == BehaviorState::Hunting
                        && self.move_states[ci].tile_pos == animal_pos
                })
                .collect();
            if hunters_here.len() >= 2 {
                self.animals[ai].kill();
                for ci in hunters_here {
                    self.vitals[ci].fed = (self.vitals[ci].fed + HUNT_FED_GAIN).min(1.0);
                }
            }
        }

        // Step 4d: animal respawn + wander.
        let do_wander = Animal::should_wander(self.tick_count);
        for (ai, animal) in self.animals.iter_mut().enumerate() {
            animal.tick_respawn();
            if do_wander {
                let seed = self
                    .tick_count
                    .wrapping_mul(1442695040888963407)
                    .wrapping_add(ai as u64 * 6364136223846793005);
                animal.wander(seed);
            }
        }

        // Step 5: population growth when the tribe is thriving.
        self.maybe_spawn_citizen();
    }
}

/// Map a `CitizenResponse` back onto a `Citizen` (preserved for Phase 2 LLM reintegration).
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
    use crate::llm::provider::CitizenResponse;

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

    fn make_world(names: &[&str]) -> World {
        World::new(names.iter().map(|n| make_citizen(n)).collect())
    }

    // --- vitals decay ---

    #[test]
    fn tick_decays_fed_each_turn() {
        let mut world = make_world(&["A"]);
        world.tick(0.0);
        assert!(world.vitals[0].fed < 1.0);
        assert!((world.vitals[0].fed - (1.0 - FED_DECAY_PER_TICK)).abs() < 1e-5);
    }

    #[test]
    fn tick_decays_hydration_each_turn() {
        let mut world = make_world(&["A"]);
        world.tick(0.0);
        assert!(world.vitals[0].hydration < 1.0);
        assert!((world.vitals[0].hydration - (1.0 - HYDRATION_DECAY_PER_TICK)).abs() < 1e-5);
    }

    #[test]
    fn vitals_never_go_below_zero() {
        let mut world = make_world(&["A"]);
        world.vitals[0].fed = 0.0;
        world.vitals[0].hydration = 0.0;
        world.tick(0.0);
        assert_eq!(world.vitals[0].fed, 0.0);
        assert_eq!(world.vitals[0].hydration, 0.0);
    }

    // --- tick: behavior transitions from needs ---

    #[test]
    fn tick_transitions_to_seeking_water_when_thirsty() {
        let mut world = make_world(&["A"]);
        world.vitals[0].hydration = 0.1;
        world.tick(0.0);
        assert_eq!(world.behavior_states[0], BehaviorState::SeekingWater);
    }

    #[test]
    fn tick_transitions_to_seeking_food_when_hungry() {
        let mut world = make_world(&["A"]);
        world.vitals[0].fed = 0.1;
        // hydration must be high so water doesn't take priority
        world.vitals[0].hydration = 1.0;
        world.tick(0.0);
        assert_eq!(world.behavior_states[0], BehaviorState::SeekingFood);
    }

    #[test]
    fn tick_increments_tick_count() {
        let mut world = make_world(&["A"]);
        assert_eq!(world.tick_count, 0);
        world.tick(0.0);
        assert_eq!(world.tick_count, 1);
        world.tick(0.0);
        assert_eq!(world.tick_count, 2);
    }

    // --- resource interaction ---

    #[test]
    fn drinking_increases_hydration() {
        let mut world = make_world(&["A"]);
        world.vitals[0].hydration = 0.5;
        world.behavior_states[0] = BehaviorState::Drinking;
        world.tick(0.0);
        assert!(world.vitals[0].hydration > 0.5);
    }

    #[test]
    fn nearest_resource_pos_returns_closest() {
        let mut world = make_world(&["A"]);
        world.add_resource(Resource::berry_bush(TilePos::new(10, 0)));
        world.add_resource(Resource::berry_bush(TilePos::new(3, 0)));
        let from = TilePos::new(0, 0);
        let nearest = world.nearest_resource_pos(from, ResourceKind::BerryBush);
        assert_eq!(nearest, Some(TilePos::new(3, 0)));
    }

    #[test]
    fn nearest_resource_pos_skips_depleted() {
        let mut world = make_world(&["A"]);
        let mut depleted = Resource::berry_bush(TilePos::new(1, 0));
        depleted.deplete(1.0);
        world.add_resource(depleted);
        world.add_resource(Resource::berry_bush(TilePos::new(5, 0)));
        let from = TilePos::new(0, 0);
        let nearest = world.nearest_resource_pos(from, ResourceKind::BerryBush);
        assert_eq!(nearest, Some(TilePos::new(5, 0)));
    }

    // --- apply_response (preserved for Phase 2 LLM reintegration) ---

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
    fn apply_response_appends_speech_to_memory() {
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
}
