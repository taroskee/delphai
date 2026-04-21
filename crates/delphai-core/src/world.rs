use crate::agent::Citizen;
use crate::move_state::MoveState;
use crate::pathfinding::TilePos;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

/// Bounds for random-walk target selection: target tiles are clamped to
/// `0..width` × `0..height`.
#[derive(Debug, Clone, Copy)]
pub struct MapBounds {
    pub width: i16,
    pub height: i16,
}

/// Configure World to re-issue a random walk target whenever a citizen becomes
/// idle. Kept as a struct so the RNG is reproducible under test.
#[derive(Debug)]
struct RandomWalk {
    rng: SmallRng,
    bounds: MapBounds,
}

#[derive(Debug, Default)]
pub struct World {
    pub tick_count: u32,
    pub citizens: Vec<Citizen>,
    pub citizen_moves: Vec<MoveState>,
    random_walk: Option<RandomWalk>,
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    /// Opt into continuous random-walk behavior: whenever a citizen's
    /// `move_target` is cleared, `tick()` will pick a new random tile within
    /// `bounds` so the citizen keeps moving. `seed` makes the walk
    /// reproducible for tests.
    pub fn enable_random_walk(&mut self, seed: u64, bounds: MapBounds) {
        self.random_walk = Some(RandomWalk {
            rng: SmallRng::seed_from_u64(seed),
            bounds,
        });
    }

    pub fn tick(&mut self) {
        self.tick_count += 1;
        for m in &mut self.citizen_moves {
            m.step();
        }
        if let Some(rw) = self.random_walk.as_mut() {
            for m in &mut self.citizen_moves {
                if m.move_target.is_none() {
                    m.move_target = Some(pick_random_target(&mut rw.rng, m.tile_pos, rw.bounds));
                }
            }
        }
    }

    /// Spawn a new citizen at `tile_pos`. Returns the index in `citizens` /
    /// `citizen_moves` (kept in parallel — never reorder one without the other).
    pub fn spawn_citizen(&mut self, name: impl Into<String>, tile_pos: TilePos) -> usize {
        let idx = self.citizens.len();
        self.citizens.push(Citizen {
            name: name.into(),
            personality_tags: Vec::new(),
            memory_summary: String::new(),
            emotion: Default::default(),
            relationships: Vec::new(),
            divine_awareness: 0.0,
        });
        self.citizen_moves.push(MoveState::new(tile_pos));
        idx
    }

    pub fn set_move_target(&mut self, idx: usize, target: TilePos) {
        self.citizen_moves[idx].move_target = Some(target);
    }

    /// Linear interpolation between `prev_tile_pos` and `tile_pos` at
    /// `alpha ∈ [0.0, 1.0]`. 0.0 = previous tile, 1.0 = current tile.
    /// Returned as `(x, y)` in tile-space (caller scales to world units).
    pub fn get_citizen_world_pos(&self, idx: usize, alpha: f32) -> (f32, f32) {
        self.citizen_moves[idx].world_pos(alpha)
    }
}

/// Pick a fresh target tile distinct from `current` within `bounds`. Kept free
/// so tests can drive it directly without constructing a World.
fn pick_random_target(rng: &mut SmallRng, current: TilePos, bounds: MapBounds) -> TilePos {
    // With width/height ≥ 2 the rejection loop is bounded in expectation; clamp
    // to 1 so we never divide by zero, but in that degenerate case the only
    // available tile is `current` so we just return it.
    let w = bounds.width.max(1);
    let h = bounds.height.max(1);
    if w == 1 && h == 1 {
        return current;
    }
    loop {
        let x = rng.gen_range(0..w);
        let y = rng.gen_range(0..h);
        let cand = TilePos::new(x, y);
        if cand != current {
            return cand;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_world_has_zero_tick_count() {
        let w = World::new();
        assert_eq!(w.tick_count, 0);
        assert!(w.citizens.is_empty());
    }

    #[test]
    fn tick_increments_tick_count() {
        let mut w = World::new();
        w.tick();
        assert_eq!(w.tick_count, 1);
        w.tick();
        assert_eq!(w.tick_count, 2);
    }

    #[test]
    fn spawn_citizen_stores_name_and_position() {
        let mut w = World::new();
        let idx = w.spawn_citizen("Kael", TilePos { x: 3, y: 7 });
        assert_eq!(idx, 0);
        assert_eq!(w.citizens.len(), 1);
        assert_eq!(w.citizens[0].name, "Kael");
        assert_eq!(w.citizen_moves[0].tile_pos, TilePos { x: 3, y: 7 });
    }

    #[test]
    fn spawn_citizen_assigns_sequential_indices() {
        let mut w = World::new();
        let a = w.spawn_citizen("A", TilePos { x: 0, y: 0 });
        let b = w.spawn_citizen("B", TilePos { x: 1, y: 2 });
        assert_eq!(a, 0);
        assert_eq!(b, 1);
        assert_eq!(w.citizen_moves[1].tile_pos, TilePos { x: 1, y: 2 });
    }

    #[test]
    fn tick_moves_citizen_one_step_toward_target() {
        let mut w = World::new();
        let idx = w.spawn_citizen("Mover", TilePos::new(0, 0));
        w.set_move_target(idx, TilePos::new(3, 0));

        w.tick();
        assert_eq!(w.citizen_moves[idx].tile_pos, TilePos::new(1, 0));
        assert_eq!(w.citizen_moves[idx].prev_tile_pos, TilePos::new(0, 0));

        w.tick();
        assert_eq!(w.citizen_moves[idx].tile_pos, TilePos::new(2, 0));
        assert_eq!(w.citizen_moves[idx].prev_tile_pos, TilePos::new(1, 0));
    }

    #[test]
    fn get_citizen_world_pos_interpolates_prev_to_current() {
        let mut w = World::new();
        let idx = w.spawn_citizen("Mover", TilePos::new(0, 0));
        w.set_move_target(idx, TilePos::new(1, 0));
        w.tick(); // prev=(0,0), curr=(1,0)

        let (x0, y0) = w.get_citizen_world_pos(idx, 0.0);
        let (x1, y1) = w.get_citizen_world_pos(idx, 1.0);
        let (xh, yh) = w.get_citizen_world_pos(idx, 0.5);

        assert!((x0 - 0.0).abs() < 1e-6 && (y0 - 0.0).abs() < 1e-6);
        assert!((x1 - 1.0).abs() < 1e-6 && (y1 - 0.0).abs() < 1e-6);
        assert!((xh - 0.5).abs() < 1e-6 && (yh - 0.0).abs() < 1e-6);
    }

    /// Regression guard for R5.4 Phase C.1 — cooldown/history logic once caused
    /// citizens to appear stuck for many consecutive ticks mid-journey. With the
    /// minimal step model (no history, no cooldown), a citizen with a far target
    /// must move every tick. We allow up to 2 consecutive static ticks just to
    /// leave room for future tile-based contention logic; 3+ would indicate a
    /// real regression.
    #[test]
    fn citizen_never_static_for_more_than_two_consecutive_ticks_during_journey() {
        let mut w = World::new();
        let idx = w.spawn_citizen("Mover", TilePos::new(0, 0));
        // Manhattan distance 20 — journey exceeds 16 ticks so target is not reached.
        w.set_move_target(idx, TilePos::new(10, 10));

        let mut consecutive_static: u32 = 0;
        let mut last_pos = w.citizen_moves[idx].tile_pos;
        for tick_i in 0..16 {
            w.tick();
            let new_pos = w.citizen_moves[idx].tile_pos;
            if new_pos == last_pos {
                consecutive_static += 1;
            } else {
                consecutive_static = 0;
            }
            assert!(
                consecutive_static <= 2,
                "citizen static for >2 consecutive ticks at tick {} (R5.4 Phase C.1 regression)",
                tick_i
            );
            last_pos = new_pos;
        }
    }

    #[test]
    fn random_walk_reissues_target_after_arrival() {
        let mut w = World::new();
        let idx = w.spawn_citizen("Wanderer", TilePos::new(2, 2));
        w.enable_random_walk(42, MapBounds { width: 10, height: 10 });
        // First tick sees no target → picks one; after stepping, target is set.
        w.tick();
        assert!(
            w.citizen_moves[idx].move_target.is_some(),
            "random walk must issue a target on first idle tick"
        );
    }

    #[test]
    fn random_walk_keeps_citizen_moving_for_100_ticks() {
        let mut w = World::new();
        let idx = w.spawn_citizen("Wanderer", TilePos::new(5, 5));
        w.enable_random_walk(0xC0FFEE, MapBounds { width: 12, height: 8 });

        let mut consecutive_static: u32 = 0;
        let mut last_pos = w.citizen_moves[idx].tile_pos;
        for tick_i in 0..100 {
            w.tick();
            let new_pos = w.citizen_moves[idx].tile_pos;
            if new_pos == last_pos {
                consecutive_static += 1;
            } else {
                consecutive_static = 0;
            }
            assert!(
                consecutive_static <= 2,
                "random walk static for >2 consecutive ticks at tick {}",
                tick_i
            );
            last_pos = new_pos;
        }
    }

    #[test]
    fn random_walk_keeps_citizen_inside_bounds() {
        let mut w = World::new();
        let idx = w.spawn_citizen("Wanderer", TilePos::new(0, 0));
        w.enable_random_walk(7, MapBounds { width: 6, height: 4 });
        for _ in 0..200 {
            w.tick();
            let p = w.citizen_moves[idx].tile_pos;
            assert!(p.x >= 0 && p.x < 6, "x out of bounds: {}", p.x);
            assert!(p.y >= 0 && p.y < 4, "y out of bounds: {}", p.y);
        }
    }

    #[test]
    fn random_walk_is_deterministic_for_fixed_seed() {
        let run = |seed: u64| {
            let mut w = World::new();
            let _ = w.spawn_citizen("W", TilePos::new(3, 3));
            w.enable_random_walk(seed, MapBounds { width: 10, height: 10 });
            let mut positions = Vec::new();
            for _ in 0..20 {
                w.tick();
                positions.push(w.citizen_moves[0].tile_pos);
            }
            positions
        };
        assert_eq!(run(123), run(123));
        assert_ne!(run(123), run(456));
    }
}
