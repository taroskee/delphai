use crate::agent::Citizen;
use crate::move_state::MoveState;
use crate::pathfinding::{TilePos, WalkGrid};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::collections::VecDeque;

/// Per-citizen recent-tile buffer. Used by `step_with_grid` to break detour
/// ties — candidates NOT in this buffer beat candidates that are in it.
const HISTORY_LEN: usize = 8;

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
    walk_grid: Option<WalkGrid>,
    /// Per-citizen recent-tile buffer (bounded to `HISTORY_LEN`). Index-parallel
    /// with `citizens` / `citizen_moves`. Updated every tick AFTER stepping.
    citizen_history: Vec<VecDeque<TilePos>>,
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

    /// Install a walkable grid. When present, `tick()` uses `step_with_grid`
    /// (obstacle-aware) and random-walk target picks only walkable tiles.
    /// `cells` is row-major, length `width * height`.
    pub fn set_walkable_map(&mut self, width: i16, height: i16, cells: Vec<bool>) {
        self.walk_grid = Some(WalkGrid::from_row_major(width, height, cells));
    }

    pub fn walk_grid(&self) -> Option<&WalkGrid> {
        self.walk_grid.as_ref()
    }

    pub fn tick(&mut self) {
        self.tick_count += 1;
        if let Some(grid) = self.walk_grid.as_ref() {
            for (i, m) in self.citizen_moves.iter_mut().enumerate() {
                let history = self.citizen_history.get(i).map(|d| d.as_slices());
                // Flatten VecDeque into a temporary Vec so we can hand a single
                // &[TilePos] to step_with_grid (two-slice form is ergonomic
                // only for iteration, not `contains`).
                let hist_vec: Vec<TilePos> = match history {
                    Some((a, b)) => a.iter().chain(b.iter()).copied().collect(),
                    None => Vec::new(),
                };
                m.step_with_grid(grid, &hist_vec);
            }
        } else {
            for m in &mut self.citizen_moves {
                m.step();
            }
        }
        for (i, m) in self.citizen_moves.iter().enumerate() {
            if let Some(hist) = self.citizen_history.get_mut(i) {
                let tp = m.tile_pos();
                if hist.back() != Some(&tp) {
                    if hist.len() == HISTORY_LEN {
                        hist.pop_front();
                    }
                    hist.push_back(tp);
                }
            }
        }
        if let Some(rw) = self.random_walk.as_mut() {
            let grid = self.walk_grid.as_ref();
            for m in &mut self.citizen_moves {
                if m.move_target.is_none() {
                    m.move_target =
                        Some(pick_random_target_on_grid(&mut rw.rng, m.tile_pos(), rw.bounds, grid));
                }
            }
        }
    }

    /// Spawn a new citizen at `tile_pos`. Returns the index in `citizens` /
    /// `citizen_moves` / `citizen_history` (kept in parallel — never reorder
    /// one without the others).
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
        self.citizen_history.push(VecDeque::with_capacity(HISTORY_LEN));
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

/// Pick a fresh target tile distinct from `current` within `bounds`, optionally
/// constrained to walkable tiles. Kept free so tests can drive it directly
/// without constructing a World.
///
/// If `grid` is provided and the current tile is inside bounds but no walkable
/// tile ≠ current exists, this falls back to `current` after a bounded number
/// of attempts — better than looping forever on pathological maps.
fn pick_random_target_on_grid(
    rng: &mut SmallRng,
    current: TilePos,
    bounds: MapBounds,
    grid: Option<&WalkGrid>,
) -> TilePos {
    let w = bounds.width.max(1);
    let h = bounds.height.max(1);
    if w == 1 && h == 1 {
        return current;
    }
    // Rejection sampling. On dense maps this converges in O(1) attempts; on
    // sparse maps we cap at a generous budget relative to map area so we never
    // hang a tick loop on an unreachable fully-blocked grid.
    let max_attempts = (w as u32 * h as u32 * 4).max(64);
    for _ in 0..max_attempts {
        let x = rng.gen_range(0..w);
        let y = rng.gen_range(0..h);
        let cand = TilePos::new(x, y);
        if cand == current {
            continue;
        }
        match grid {
            Some(g) if !g.is_walkable(cand) => continue,
            _ => return cand,
        }
    }
    current
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
        assert_eq!(w.citizen_moves[0].tile_pos(), TilePos { x: 3, y: 7 });
    }

    #[test]
    fn spawn_citizen_assigns_sequential_indices() {
        let mut w = World::new();
        let a = w.spawn_citizen("A", TilePos { x: 0, y: 0 });
        let b = w.spawn_citizen("B", TilePos { x: 1, y: 2 });
        assert_eq!(a, 0);
        assert_eq!(b, 1);
        assert_eq!(w.citizen_moves[1].tile_pos(), TilePos { x: 1, y: 2 });
    }

    #[test]
    fn tick_moves_citizen_one_step_toward_target() {
        let mut w = World::new();
        let idx = w.spawn_citizen("Mover", TilePos::new(0, 0));
        w.set_move_target(idx, TilePos::new(3, 0));

        w.tick();
        assert_eq!(w.citizen_moves[idx].tile_pos(), TilePos::new(1, 0));
        assert_eq!(w.citizen_moves[idx].prev_tile_pos(), TilePos::new(0, 0));

        w.tick();
        assert_eq!(w.citizen_moves[idx].tile_pos(), TilePos::new(2, 0));
        assert_eq!(w.citizen_moves[idx].prev_tile_pos(), TilePos::new(1, 0));
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

    /// Regression guard for Sprint N4.3 — with continuous unit-vector motion,
    /// `tile_pos()` can legitimately stay constant for several consecutive ticks
    /// on shallow angles (e.g. heading (0.3, 0.95)), so the old tile-based
    /// static-tick guard false-fires. The real invariant is that Euclidean
    /// distance to the target strictly decreases every tick until arrival.
    #[test]
    fn distance_to_target_monotonically_decreases_during_journey() {
        let mut w = World::new();
        let idx = w.spawn_citizen("Mover", TilePos::new(0, 0));
        // Target far enough (|d|≈28.28) that we don't snap within 16 ticks.
        let target = TilePos::new(20, 20);
        w.set_move_target(idx, target);

        let dist_to_target = |m: &MoveState| -> f32 {
            let dx = f32::from(target.x) - m.pos.0;
            let dy = f32::from(target.y) - m.pos.1;
            (dx * dx + dy * dy).sqrt()
        };

        let mut prev_dist = dist_to_target(&w.citizen_moves[idx]);
        for tick_i in 0..16 {
            w.tick();
            let new_dist = dist_to_target(&w.citizen_moves[idx]);
            assert!(
                new_dist < prev_dist - 1e-4,
                "distance did not strictly decrease at tick {}: {} -> {}",
                tick_i,
                prev_dist,
                new_dist
            );
            prev_dist = new_dist;
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

        // Continuous motion: every tick the float `pos` must change by ~SPEED.
        // Note: the first tick runs step() before random_walk assigns a target,
        // so we warm up one tick, then assert motion every subsequent tick.
        w.tick();
        let mut last_pos = w.citizen_moves[idx].pos;
        for tick_i in 0..100 {
            w.tick();
            let new_pos = w.citizen_moves[idx].pos;
            let dx = new_pos.0 - last_pos.0;
            let dy = new_pos.1 - last_pos.1;
            let moved = (dx * dx + dy * dy).sqrt();
            assert!(
                moved > 1e-6,
                "random walk did not move at tick {} (pos={:?})",
                tick_i,
                new_pos
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
            let p = w.citizen_moves[idx].tile_pos();
            assert!(p.x >= 0 && p.x < 6, "x out of bounds: {}", p.x);
            assert!(p.y >= 0 && p.y < 4, "y out of bounds: {}", p.y);
        }
    }

    #[test]
    fn set_walkable_map_stores_grid() {
        let mut w = World::new();
        // 3x2 with (1,0) blocked.
        let cells = vec![true, false, true, true, true, true];
        w.set_walkable_map(3, 2, cells);
        let g = w.walk_grid().expect("grid installed");
        assert_eq!(g.width(), 3);
        assert_eq!(g.height(), 2);
        assert!(g.is_walkable(TilePos::new(0, 0)));
        assert!(!g.is_walkable(TilePos::new(1, 0)));
        assert!(g.is_walkable(TilePos::new(2, 0)));
    }

    #[test]
    fn tick_routes_around_obstacle_when_grid_present() {
        // (0,0) → (2,0) with (1,0) blocked. With grid present, the citizen must
        // detour (probably via y=1) and reach (2,0) without ever stepping on
        // (1,0).
        let mut w = World::new();
        let idx = w.spawn_citizen("Pathfinder", TilePos::new(0, 0));
        let mut cells = vec![true; 9]; // 3x3
        cells[1] = false; // (1, 0) row-major index = 0*3 + 1
        w.set_walkable_map(3, 3, cells);
        w.set_move_target(idx, TilePos::new(2, 0));

        for tick_i in 0..30 {
            w.tick();
            let p = w.citizen_moves[idx].tile_pos();
            assert_ne!(p, TilePos::new(1, 0), "stepped on blocked tile at tick {}", tick_i);
            if w.citizen_moves[idx].move_target.is_none()
                && p == TilePos::new(2, 0)
            {
                return;
            }
        }
        panic!(
            "did not reach (2,0) within 30 ticks; final pos={:?}",
            w.citizen_moves[idx].pos
        );
    }

    #[test]
    fn tick_records_citizen_history_bounded_to_eight() {
        // Move far enough that the history buffer definitely fills and
        // overflows. Grid-enabled path is required for history semantics.
        let mut w = World::new();
        let idx = w.spawn_citizen("Hist", TilePos::new(0, 0));
        w.set_walkable_map(20, 20, vec![true; 400]);
        w.set_move_target(idx, TilePos::new(15, 0));
        for _ in 0..20 {
            w.tick();
        }
        let hist = &w.citizen_history[idx];
        assert!(hist.len() <= HISTORY_LEN, "history grew past cap: {}", hist.len());
        assert_eq!(hist.len(), HISTORY_LEN, "history should be saturated after a long walk");
    }

    #[test]
    fn random_walk_only_picks_walkable_targets() {
        // Narrow walkable corridor: column x=0 walkable, everything else blocked.
        // Random targets must never land on blocked tiles.
        let mut w = World::new();
        let idx = w.spawn_citizen("Corridor", TilePos::new(0, 0));
        let width: i16 = 4;
        let height: i16 = 6;
        let mut cells = vec![false; (width as usize) * (height as usize)];
        for y in 0..height {
            cells[(y as usize) * (width as usize)] = true; // x=0 walkable
        }
        w.set_walkable_map(width, height, cells);
        w.enable_random_walk(99, MapBounds { width, height });

        for tick_i in 0..50 {
            w.tick();
            if let Some(t) = w.citizen_moves[idx].move_target {
                assert_eq!(t.x, 0, "random walk picked non-walkable x={} at tick {}", t.x, tick_i);
            }
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
                positions.push(w.citizen_moves[0].pos);
            }
            positions
        };
        assert_eq!(run(123), run(123));
        assert_ne!(run(123), run(456));
    }
}
