use crate::agent::Citizen;
use crate::move_state::MoveState;
use crate::pathfinding::TilePos;

#[derive(Debug, Default)]
pub struct World {
    pub tick_count: u32,
    pub citizens: Vec<Citizen>,
    pub citizen_moves: Vec<MoveState>,
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tick(&mut self) {
        self.tick_count += 1;
        for m in &mut self.citizen_moves {
            m.step();
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
}
