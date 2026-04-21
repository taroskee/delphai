use crate::pathfinding::TilePos;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MoveState {
    pub tile_pos: TilePos,
    pub prev_tile_pos: TilePos,
    pub move_target: Option<TilePos>,
}

impl MoveState {
    pub fn new(tile_pos: TilePos) -> Self {
        Self {
            tile_pos,
            prev_tile_pos: tile_pos,
            move_target: None,
        }
    }

    /// Advance one tile toward `move_target`. No history, no cooldown — any
    /// such state machines belong in higher layers (Sprint N5+). `prev_tile_pos`
    /// is always set to the tile occupied before this step so that renderers
    /// can interpolate smoothly. When the target is reached we clear it and
    /// leave `prev` equal to `tile_pos` (zero-length segment is a valid input
    /// to `world_pos`).
    pub fn step(&mut self) {
        self.prev_tile_pos = self.tile_pos;
        let Some(target) = self.move_target else { return };
        if target == self.tile_pos {
            self.move_target = None;
            return;
        }
        let dx = (target.x - self.tile_pos.x).signum();
        let dy = (target.y - self.tile_pos.y).signum();
        self.tile_pos.x += dx;
        self.tile_pos.y += dy;
        if self.tile_pos == target {
            self.move_target = None;
        }
    }

    /// Linear interpolation between `prev_tile_pos` and `tile_pos`.
    /// `alpha` is clamped to `[0.0, 1.0]`.
    pub fn world_pos(&self, alpha: f32) -> (f32, f32) {
        let a = alpha.clamp(0.0, 1.0);
        let px = f32::from(self.prev_tile_pos.x);
        let py = f32::from(self.prev_tile_pos.y);
        let cx = f32::from(self.tile_pos.x);
        let cy = f32::from(self.tile_pos.y);
        (px + (cx - px) * a, py + (cy - py) * a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_initializes_prev_equal_to_current_and_no_target() {
        let m = MoveState::new(TilePos::new(2, 3));
        assert_eq!(m.tile_pos, TilePos::new(2, 3));
        assert_eq!(m.prev_tile_pos, TilePos::new(2, 3));
        assert_eq!(m.move_target, None);
    }

    #[test]
    fn step_with_no_target_is_idle_and_resets_prev() {
        let mut m = MoveState::new(TilePos::new(5, 5));
        m.step();
        assert_eq!(m.tile_pos, TilePos::new(5, 5));
        assert_eq!(m.prev_tile_pos, TilePos::new(5, 5));
    }

    #[test]
    fn step_moves_diagonally_when_both_axes_need_to_change() {
        let mut m = MoveState::new(TilePos::new(0, 0));
        m.move_target = Some(TilePos::new(3, 3));
        m.step();
        assert_eq!(m.tile_pos, TilePos::new(1, 1));
        m.step();
        assert_eq!(m.tile_pos, TilePos::new(2, 2));
        m.step();
        assert_eq!(m.tile_pos, TilePos::new(3, 3));
        assert_eq!(m.move_target, None, "target cleared on arrival");
    }

    #[test]
    fn step_moves_diagonally_toward_negative_target() {
        let mut m = MoveState::new(TilePos::new(0, 0));
        m.move_target = Some(TilePos::new(-2, -2));
        m.step();
        assert_eq!(m.tile_pos, TilePos::new(-1, -1));
        m.step();
        assert_eq!(m.tile_pos, TilePos::new(-2, -2));
        assert_eq!(m.move_target, None);
    }

    #[test]
    fn step_falls_back_to_axis_step_when_only_one_axis_differs() {
        let mut m = MoveState::new(TilePos::new(5, 5));
        m.move_target = Some(TilePos::new(5, 2));
        m.step();
        assert_eq!(m.tile_pos, TilePos::new(5, 4));
        m.step();
        assert_eq!(m.tile_pos, TilePos::new(5, 3));
        m.step();
        assert_eq!(m.tile_pos, TilePos::new(5, 2));

        let mut n = MoveState::new(TilePos::new(0, 0));
        n.move_target = Some(TilePos::new(3, 0));
        n.step();
        assert_eq!(n.tile_pos, TilePos::new(1, 0));
        n.step();
        assert_eq!(n.tile_pos, TilePos::new(2, 0));
        n.step();
        assert_eq!(n.tile_pos, TilePos::new(3, 0));
    }

    #[test]
    fn step_diagonal_then_axis_when_target_is_not_equidistant() {
        let mut m = MoveState::new(TilePos::new(0, 0));
        m.move_target = Some(TilePos::new(3, 1));
        m.step();
        assert_eq!(m.tile_pos, TilePos::new(1, 1));
        m.step();
        assert_eq!(m.tile_pos, TilePos::new(2, 1));
        m.step();
        assert_eq!(m.tile_pos, TilePos::new(3, 1));
    }

    #[test]
    fn world_pos_interpolates_linearly() {
        let m = MoveState {
            tile_pos: TilePos::new(2, 0),
            prev_tile_pos: TilePos::new(1, 0),
            move_target: None,
        };
        assert_eq!(m.world_pos(0.0), (1.0, 0.0));
        assert_eq!(m.world_pos(1.0), (2.0, 0.0));
        assert_eq!(m.world_pos(0.25), (1.25, 0.0));
    }

    #[test]
    fn world_pos_clamps_alpha() {
        let m = MoveState {
            tile_pos: TilePos::new(1, 0),
            prev_tile_pos: TilePos::new(0, 0),
            move_target: None,
        };
        assert_eq!(m.world_pos(-1.0), (0.0, 0.0));
        assert_eq!(m.world_pos(2.0), (1.0, 0.0));
    }
}
