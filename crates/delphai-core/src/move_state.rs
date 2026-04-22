use crate::pathfinding::{TilePos, WalkGrid};

/// Movement speed in tile units per tick. Each step advances the citizen by a
/// unit vector (toward `move_target`) scaled by `SPEED`, so citizens trace a
/// smooth circular path at any angle rather than snapping to 8 compass
/// directions. Keeping this as `1.0` preserves the prior pace (≈4 tiles/sec at
/// 4Hz tick) so existing visual tuning still applies.
const SPEED: f32 = 1.0;

/// Floating-point world position + integer tile target. `pos` is the source of
/// truth; `tile_pos()` derives the grid cell via `round()` for walkable-grid
/// lookups (Sprint N5+). `prev_pos` is refreshed every tick so frame-rate
/// interpolation (`world_pos`) never produces a jump, even when the citizen is
/// idle.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MoveState {
    pub pos: (f32, f32),
    pub prev_pos: (f32, f32),
    pub move_target: Option<TilePos>,
}

impl MoveState {
    pub fn new(tile_pos: TilePos) -> Self {
        let p = (f32::from(tile_pos.x), f32::from(tile_pos.y));
        Self {
            pos: p,
            prev_pos: p,
            move_target: None,
        }
    }

    /// Grid cell currently occupied by this citizen (nearest integer tile).
    pub fn tile_pos(&self) -> TilePos {
        TilePos::new(self.pos.0.round() as i16, self.pos.1.round() as i16)
    }

    pub fn prev_tile_pos(&self) -> TilePos {
        TilePos::new(self.prev_pos.0.round() as i16, self.prev_pos.1.round() as i16)
    }

    /// Advance one tick at `SPEED` toward `move_target` using unit-vector
    /// (atan2-equivalent) movement. When remaining distance ≤ SPEED, snap to
    /// the target exactly and clear it. `prev_pos` is always updated first so
    /// the renderer can interpolate smoothly.
    pub fn step(&mut self) {
        self.prev_pos = self.pos;
        let Some(target) = self.move_target else { return };
        let tx = f32::from(target.x);
        let ty = f32::from(target.y);
        let dx = tx - self.pos.0;
        let dy = ty - self.pos.1;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist <= SPEED {
            self.pos = (tx, ty);
            self.move_target = None;
            return;
        }
        let inv = SPEED / dist;
        self.pos.0 += dx * inv;
        self.pos.1 += dy * inv;
    }

    /// Obstacle-aware step. Tries the desired unit vector first; if the
    /// landing tile is non-walkable, tries deviations in ±45° / ±90° / ±135°
    /// order. `history` gets second priority — if a candidate lands on a
    /// recently-visited tile, other walkable candidates win, but a historied
    /// tile is still preferable to getting stuck.
    ///
    /// If the target itself is non-walkable, clears the target so the caller
    /// (e.g. random walk) picks a fresh one. If every direction is blocked,
    /// `prev_pos` is still updated so interpolation does not snap visually.
    pub fn step_with_grid(&mut self, grid: &WalkGrid, history: &[TilePos]) {
        self.prev_pos = self.pos;
        let Some(target) = self.move_target else { return };
        let tx = f32::from(target.x);
        let ty = f32::from(target.y);
        let dx = tx - self.pos.0;
        let dy = ty - self.pos.1;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist <= SPEED {
            if grid.is_walkable(target) {
                self.pos = (tx, ty);
            }
            // Either way, consume the target. Non-walkable target means the
            // caller picked an invalid tile; clearing lets them re-pick.
            self.move_target = None;
            return;
        }

        let base_angle = dy.atan2(dx);
        const DEVIATIONS: [f32; 7] = [
            0.0,
            -std::f32::consts::FRAC_PI_4,
            std::f32::consts::FRAC_PI_4,
            -std::f32::consts::FRAC_PI_2,
            std::f32::consts::FRAC_PI_2,
            -3.0 * std::f32::consts::FRAC_PI_4,
            3.0 * std::f32::consts::FRAC_PI_4,
        ];

        let mut fallback: Option<(f32, f32)> = None;
        for d in DEVIATIONS {
            let angle = base_angle + d;
            let ux = angle.cos();
            let uy = angle.sin();
            let cand = (self.pos.0 + ux * SPEED, self.pos.1 + uy * SPEED);
            let cand_tile = TilePos::new(cand.0.round() as i16, cand.1.round() as i16);
            if !grid.is_walkable(cand_tile) {
                continue;
            }
            if !history.contains(&cand_tile) {
                self.pos = cand;
                return;
            }
            if fallback.is_none() {
                fallback = Some(cand);
            }
        }
        if let Some(cand) = fallback {
            self.pos = cand;
        }
    }

    /// Linear interpolation between `prev_pos` and `pos`. `alpha` is clamped
    /// to `[0.0, 1.0]`.
    pub fn world_pos(&self, alpha: f32) -> (f32, f32) {
        let a = alpha.clamp(0.0, 1.0);
        let (px, py) = self.prev_pos;
        let (cx, cy) = self.pos;
        (px + (cx - px) * a, py + (cy - py) * a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-6;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < EPS
    }

    fn close2(a: (f32, f32), b: (f32, f32)) -> bool {
        close(a.0, b.0) && close(a.1, b.1)
    }

    #[test]
    fn new_initializes_prev_equal_to_current_and_no_target() {
        let m = MoveState::new(TilePos::new(2, 3));
        assert!(close2(m.pos, (2.0, 3.0)));
        assert!(close2(m.prev_pos, (2.0, 3.0)));
        assert_eq!(m.move_target, None);
        assert_eq!(m.tile_pos(), TilePos::new(2, 3));
        assert_eq!(m.prev_tile_pos(), TilePos::new(2, 3));
    }

    #[test]
    fn step_with_no_target_is_idle_and_resets_prev() {
        let mut m = MoveState::new(TilePos::new(5, 5));
        m.step();
        assert!(close2(m.pos, (5.0, 5.0)));
        assert!(close2(m.prev_pos, (5.0, 5.0)));
    }

    #[test]
    fn step_moves_unit_vector_at_zero_degrees() {
        // 0°: (cos, sin) = (1, 0). Target east → step to (1, 0).
        let mut m = MoveState::new(TilePos::new(0, 0));
        m.move_target = Some(TilePos::new(3, 0));
        m.step();
        assert!(close2(m.pos, (1.0, 0.0)), "pos={:?}", m.pos);
    }

    #[test]
    fn step_moves_unit_vector_at_ninety_degrees() {
        // 90°: (cos, sin) = (0, 1). Target north → step to (0, 1).
        let mut m = MoveState::new(TilePos::new(0, 0));
        m.move_target = Some(TilePos::new(0, 3));
        m.step();
        assert!(close2(m.pos, (0.0, 1.0)), "pos={:?}", m.pos);
    }

    #[test]
    fn step_moves_unit_vector_at_two_seventy_degrees() {
        // 270° per user spec: (cos 270°, sin 270°) = (0, -1). Target (0, -3)
        // lies exactly along 270° from origin.
        let mut m = MoveState::new(TilePos::new(0, 0));
        m.move_target = Some(TilePos::new(0, -3));
        m.step();
        assert!(close2(m.pos, (0.0, -1.0)), "pos={:?}", m.pos);
    }

    #[test]
    fn step_moves_unit_vector_at_one_eighty_degrees() {
        let mut m = MoveState::new(TilePos::new(0, 0));
        m.move_target = Some(TilePos::new(-3, 0));
        m.step();
        assert!(close2(m.pos, (-1.0, 0.0)), "pos={:?}", m.pos);
    }

    #[test]
    fn step_moves_unit_vector_along_three_four_five_triple() {
        // (3, 4, 5) pythagorean → unit step = (0.6, 0.8) exactly. Validates
        // that non-axis-aligned motion is truly angular (not Chebyshev).
        let mut m = MoveState::new(TilePos::new(0, 0));
        m.move_target = Some(TilePos::new(3, 4));
        m.step();
        assert!(close2(m.pos, (0.6, 0.8)), "pos={:?}", m.pos);
    }

    #[test]
    fn step_moves_unit_vector_toward_negative_diagonal() {
        let mut m = MoveState::new(TilePos::new(0, 0));
        m.move_target = Some(TilePos::new(-3, -4));
        m.step();
        assert!(close2(m.pos, (-0.6, -0.8)), "pos={:?}", m.pos);
    }

    #[test]
    fn step_preserves_direction_for_arbitrary_angle() {
        // User spec: 60° → (1/2, √3/2). TilePos is i16 so we can't place a
        // target exactly at 60°, but the invariant is: step direction == unit
        // vector toward target, and |step| == SPEED. Verify with target (1, 2)
        // (≈63.4°): |step| == 1.0 and step / |step| == target / |target|.
        let mut m = MoveState::new(TilePos::new(0, 0));
        m.move_target = Some(TilePos::new(1, 2));
        m.step();
        let mag = (m.pos.0 * m.pos.0 + m.pos.1 * m.pos.1).sqrt();
        assert!(close(mag, SPEED), "|step|={} expected {}", mag, SPEED);
        let tmag = (1.0_f32 * 1.0 + 2.0 * 2.0).sqrt();
        let unit = (1.0 / tmag, 2.0 / tmag);
        assert!(close2(m.pos, unit), "pos={:?} expected unit={:?}", m.pos, unit);
    }

    #[test]
    fn step_snaps_to_target_when_within_speed() {
        // Target is exactly SPEED=1.0 away → first step snaps and clears.
        let mut m = MoveState::new(TilePos::new(0, 0));
        m.move_target = Some(TilePos::new(1, 0));
        m.step();
        assert!(close2(m.pos, (1.0, 0.0)));
        assert_eq!(m.move_target, None, "target cleared on arrival");
    }

    #[test]
    fn step_reaches_three_four_five_target_in_five_ticks() {
        let mut m = MoveState::new(TilePos::new(0, 0));
        m.move_target = Some(TilePos::new(3, 4));
        for _ in 0..4 {
            m.step();
            assert!(m.move_target.is_some(), "target should still be set");
        }
        m.step();
        assert!(close2(m.pos, (3.0, 4.0)));
        assert_eq!(m.move_target, None);
    }

    #[test]
    fn world_pos_interpolates_float_prev_to_current() {
        let m = MoveState {
            pos: (2.0, 0.8),
            prev_pos: (1.2, 0.5),
            move_target: None,
        };
        assert!(close2(m.world_pos(0.0), (1.2, 0.5)));
        assert!(close2(m.world_pos(1.0), (2.0, 0.8)));
        assert!(close2(m.world_pos(0.5), (1.6, 0.65)));
    }

    #[test]
    fn world_pos_clamps_alpha() {
        let m = MoveState {
            pos: (1.0, 0.0),
            prev_pos: (0.0, 0.0),
            move_target: None,
        };
        assert!(close2(m.world_pos(-1.0), (0.0, 0.0)));
        assert!(close2(m.world_pos(2.0), (1.0, 0.0)));
    }

    #[test]
    fn step_with_grid_all_walkable_matches_plain_step() {
        let mut a = MoveState::new(TilePos::new(0, 0));
        a.move_target = Some(TilePos::new(3, 4));
        let mut b = a;
        let grid = WalkGrid::new_all_walkable(10, 10);
        for _ in 0..4 {
            a.step();
            b.step_with_grid(&grid, &[]);
            assert!(close2(a.pos, b.pos), "a={:?} b={:?}", a.pos, b.pos);
            assert_eq!(a.move_target, b.move_target);
        }
    }

    #[test]
    fn step_with_grid_detours_around_direct_obstacle() {
        // (0,0) → (2,0) with (1,0) blocked. The citizen must never occupy
        // (1, 0) but should still reach (2, 0) eventually.
        let mut m = MoveState::new(TilePos::new(0, 0));
        m.move_target = Some(TilePos::new(2, 0));
        let mut grid = WalkGrid::new_all_walkable(3, 3);
        grid.set(TilePos::new(1, 0), false);
        for tick_i in 0..20 {
            m.step_with_grid(&grid, &[]);
            assert_ne!(
                m.tile_pos(),
                TilePos::new(1, 0),
                "stepped on blocked tile at tick {}",
                tick_i
            );
            if m.move_target.is_none() {
                break;
            }
        }
        assert_eq!(m.tile_pos(), TilePos::new(2, 0), "did not reach target");
    }

    #[test]
    fn step_with_grid_clears_target_when_target_is_non_walkable() {
        // If the target itself is blocked, step can never snap. Clearing the
        // target on arrival-proximity signals "try a new one" to the caller.
        let mut m = MoveState::new(TilePos::new(0, 0));
        m.move_target = Some(TilePos::new(1, 0));
        let mut grid = WalkGrid::new_all_walkable(3, 3);
        grid.set(TilePos::new(1, 0), false);
        m.step_with_grid(&grid, &[]);
        assert_eq!(m.move_target, None);
        // Did not enter the blocked tile.
        assert_ne!(m.tile_pos(), TilePos::new(1, 0));
    }

    #[test]
    fn step_with_grid_stays_put_when_all_directions_blocked() {
        // 1x1 pocket at (1,1), target (5,5) beyond walls.
        let mut m = MoveState::new(TilePos::new(1, 1));
        m.move_target = Some(TilePos::new(5, 5));
        let mut grid = WalkGrid::from_row_major(3, 3, vec![false; 9]);
        grid.set(TilePos::new(1, 1), true); // only the pocket is walkable
        let before = m.pos;
        m.step_with_grid(&grid, &[]);
        // No candidate walkable → pos unchanged, prev_pos updated to before.
        assert!(close2(m.pos, before));
        assert!(close2(m.prev_pos, before));
    }

    #[test]
    fn step_with_grid_history_prefers_unvisited_tile() {
        // With two walkable detour candidates, history should push the
        // algorithm toward the non-visited one.
        // Setup: at (1,1) targeting (1,3). (1,2) blocked. Both (0,2) and (2,2)
        // are valid detours; history contains (2,2), so (0,2) should win.
        let mut m = MoveState::new(TilePos::new(1, 1));
        m.move_target = Some(TilePos::new(1, 3));
        let mut grid = WalkGrid::new_all_walkable(3, 4);
        grid.set(TilePos::new(1, 2), false);
        let history = [TilePos::new(2, 2)];
        m.step_with_grid(&grid, &history);
        // Picked (0,2) or equivalent non-history walkable, NOT (2,2).
        assert_ne!(m.tile_pos(), TilePos::new(2, 2));
        assert_ne!(m.tile_pos(), TilePos::new(1, 2));
    }

    #[test]
    fn tile_pos_derived_from_round_of_pos() {
        let m = MoveState {
            pos: (2.4, 1.6),
            prev_pos: (0.0, 0.0),
            move_target: None,
        };
        assert_eq!(m.tile_pos(), TilePos::new(2, 2));

        let m2 = MoveState {
            pos: (-0.3, 0.7),
            prev_pos: (0.0, 0.0),
            move_target: None,
        };
        assert_eq!(m2.tile_pos(), TilePos::new(0, 1));
    }
}
