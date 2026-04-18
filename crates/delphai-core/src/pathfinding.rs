//! RCT/openRCT2-inspired tile-based pathfinding.
//!
//! Uses local one-step decisions (not full A*): each tick, pick the walkable
//! neighbour closest to the goal, penalising tiles in the recent history buffer
//! to prevent loops. Stuck detection triggers a random-direction escape.

pub const HISTORY_LEN: usize = 16;
/// Ticks a citizen waits between steps (controls walking speed).
pub const STEP_COOLDOWN: u32 = 1;
/// Ticks a citizen rests after reaching a wander destination.
const ARRIVE_COOLDOWN: u32 = 8;
/// Steps without progress before triggering escape.
const STUCK_THRESHOLD: u32 = 3;
/// History penalty per repeated tile (makes backtracking unattractive).
const HISTORY_PENALTY: i32 = 10;

// ---------------------------------------------------------------------------
// TilePos
// ---------------------------------------------------------------------------

/// Integer grid coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TilePos {
    pub x: i16,
    pub y: i16,
}

impl TilePos {
    pub fn new(x: i16, y: i16) -> Self {
        Self { x, y }
    }

    pub fn manhattan_dist(self, other: Self) -> u32 {
        self.x.abs_diff(other.x) as u32 + self.y.abs_diff(other.y) as u32
    }
}

// ---------------------------------------------------------------------------
// MoveState
// ---------------------------------------------------------------------------

/// Per-citizen movement state — tile position, target, history, and cooldowns.
#[derive(Debug, Clone)]
pub struct MoveState {
    pub tile_pos: TilePos,
    pub move_target: Option<TilePos>,
    /// Ring buffer: the last HISTORY_LEN tiles visited.
    pub move_history: [TilePos; HISTORY_LEN],
    pub history_head: usize,
    pub move_cooldown: u32,
    pub stuck_counter: u32,
    pub wander_center: TilePos,
    pub wander_radius: u32,
    pub sight_radius: u32,
    /// 0=down  1=left  2=right  3=up
    pub facing: u8,
}

impl MoveState {
    pub fn new(tile_pos: TilePos, wander_center: TilePos, wander_radius: u32) -> Self {
        Self {
            tile_pos,
            move_target: None,
            move_history: [TilePos::default(); HISTORY_LEN],
            history_head: 0,
            move_cooldown: 0,
            stuck_counter: 0,
            wander_center,
            wander_radius,
            sight_radius: 5,
            facing: 0,
        }
    }

    fn push_history(&mut self, pos: TilePos) {
        self.move_history[self.history_head] = pos;
        self.history_head = (self.history_head + 1) % HISTORY_LEN;
    }
}

// ---------------------------------------------------------------------------
// WalkGrid
// ---------------------------------------------------------------------------

/// Boolean walkability grid stored in row-major order.
pub struct WalkGrid {
    pub width: usize,
    pub height: usize,
    cells: Vec<bool>,
}

impl WalkGrid {
    /// `cells` must have exactly `width * height` entries.
    pub fn new(width: usize, height: usize, cells: Vec<bool>) -> Self {
        assert_eq!(
            cells.len(),
            width * height,
            "cells length must equal width * height"
        );
        Self { width, height, cells }
    }

    pub fn is_walkable(&self, pos: TilePos) -> bool {
        if pos.x < 0 || pos.y < 0 {
            return false;
        }
        let x = pos.x as usize;
        let y = pos.y as usize;
        x < self.width && y < self.height && self.cells[y * self.width + x]
    }

    /// 4-connected walkable neighbours of `pos`.
    pub fn neighbors(&self, pos: TilePos) -> Vec<TilePos> {
        [
            TilePos::new(pos.x, pos.y - 1), // up
            TilePos::new(pos.x, pos.y + 1), // down
            TilePos::new(pos.x - 1, pos.y), // left
            TilePos::new(pos.x + 1, pos.y), // right
        ]
        .into_iter()
        .filter(|p| self.is_walkable(*p))
        .collect()
    }

    /// Pick the walkable neighbour of `from` that minimises distance to `to`,
    /// with a penalty for tiles that appear in `history` (backtrack avoidance).
    /// Returns `from` unchanged when no walkable neighbours exist.
    pub fn step_toward(&self, from: TilePos, to: TilePos, history: &[TilePos]) -> TilePos {
        let neighbours = self.neighbors(from);
        if neighbours.is_empty() {
            return from;
        }
        neighbours
            .into_iter()
            .min_by_key(|&n| {
                let dist = n.manhattan_dist(to) as i32;
                let penalty = history.iter().filter(|&&h| h == n).count() as i32 * HISTORY_PENALTY;
                dist + penalty
            })
            .unwrap_or(from)
    }

    /// Choose a random walkable tile within `radius` tiles of `center`.
    /// Falls back to `center` when no walkable candidate is found in 20 tries.
    /// `seed` should mix the tick count and citizen index for variety.
    pub fn pick_wander_target(&self, center: TilePos, radius: u32, seed: u64) -> TilePos {
        let r = radius as i16;
        let range = (2 * r + 1) as u16;
        for attempt in 0..20u64 {
            // LCG-style hash to spread across the search space
            let s = seed
                .wrapping_add(attempt.wrapping_mul(6364136223846793005))
                .wrapping_add(1442695040888963407);
            let dx = ((s >> 33) as u16 % range) as i16 - r;
            let dy = ((s >> 17) as u16 % range) as i16 - r;
            let candidate = TilePos::new(center.x + dx, center.y + dy);
            if self.is_walkable(candidate) {
                return candidate;
            }
        }
        center
    }
}

// ---------------------------------------------------------------------------
// step_citizen — drives one tick of movement for a single citizen
// ---------------------------------------------------------------------------

/// Advance one citizen's movement by one game tick.
///
/// Returns the citizen's tile position after the tick (may be unchanged if on
/// cooldown). Mutates `state` in place.
pub fn step_citizen(state: &mut MoveState, grid: &WalkGrid, seed: u64) -> TilePos {
    if state.move_cooldown > 0 {
        state.move_cooldown -= 1;
        return state.tile_pos;
    }

    // Choose a wander target when none is active
    if state.move_target.is_none() {
        state.move_target = Some(grid.pick_wander_target(
            state.wander_center,
            state.wander_radius,
            seed,
        ));
    }

    let target = state.move_target.unwrap();

    if state.tile_pos == target {
        state.move_target = None;
        state.move_cooldown = ARRIVE_COOLDOWN;
        state.stuck_counter = 0;
        return state.tile_pos;
    }

    let next = grid.step_toward(state.tile_pos, target, &state.move_history);

    if next == state.tile_pos {
        // No progress — increment stuck counter
        state.stuck_counter += 1;
        if state.stuck_counter >= STUCK_THRESHOLD {
            // Emergency escape: pick a random walkable neighbour
            let neighbours = grid.neighbors(state.tile_pos);
            if !neighbours.is_empty() {
                let escape = neighbours[seed as usize % neighbours.len()];
                set_facing(state, escape);
                state.push_history(escape);
                state.tile_pos = escape;
            }
            state.stuck_counter = 0;
            state.move_target = None;
        }
    } else {
        set_facing(state, next);
        state.push_history(next);
        state.tile_pos = next;
        state.stuck_counter = 0;
        state.move_cooldown = STEP_COOLDOWN;
    }

    state.tile_pos
}

fn set_facing(state: &mut MoveState, next: TilePos) {
    let dx = next.x - state.tile_pos.x;
    let dy = next.y - state.tile_pos.y;
    state.facing = if dy > 0 {
        0 // down
    } else if dy < 0 {
        3 // up
    } else if dx < 0 {
        1 // left
    } else {
        2 // right
    };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn all_walkable(w: usize, h: usize) -> WalkGrid {
        WalkGrid::new(w, h, vec![true; w * h])
    }

    fn make_state(x: i16, y: i16) -> MoveState {
        MoveState::new(TilePos::new(x, y), TilePos::new(x, y), 4)
    }

    // --- TilePos ---

    #[test]
    fn manhattan_dist_same_tile() {
        let p = TilePos::new(3, 5);
        assert_eq!(p.manhattan_dist(p), 0);
    }

    #[test]
    fn manhattan_dist_adjacent() {
        let a = TilePos::new(0, 0);
        let b = TilePos::new(1, 0);
        assert_eq!(a.manhattan_dist(b), 1);
    }

    #[test]
    fn manhattan_dist_diagonal() {
        let a = TilePos::new(0, 0);
        let b = TilePos::new(3, 4);
        assert_eq!(a.manhattan_dist(b), 7);
    }

    // --- WalkGrid::is_walkable ---

    #[test]
    fn is_walkable_in_bounds() {
        let grid = all_walkable(5, 5);
        assert!(grid.is_walkable(TilePos::new(2, 2)));
    }

    #[test]
    fn is_walkable_false_for_blocked_cell() {
        let cells: Vec<bool> = vec![true, false, true, true];
        let grid = WalkGrid::new(2, 2, cells);
        assert!(!grid.is_walkable(TilePos::new(1, 0)));
    }

    #[test]
    fn is_walkable_false_out_of_bounds() {
        let grid = all_walkable(3, 3);
        assert!(!grid.is_walkable(TilePos::new(3, 0)));
        assert!(!grid.is_walkable(TilePos::new(-1, 0)));
        assert!(!grid.is_walkable(TilePos::new(0, -1)));
    }

    // --- WalkGrid::neighbors ---

    #[test]
    fn center_tile_has_four_neighbours() {
        let grid = all_walkable(5, 5);
        assert_eq!(grid.neighbors(TilePos::new(2, 2)).len(), 4);
    }

    #[test]
    fn corner_tile_has_two_neighbours() {
        let grid = all_walkable(5, 5);
        assert_eq!(grid.neighbors(TilePos::new(0, 0)).len(), 2);
    }

    #[test]
    fn edge_tile_has_three_neighbours() {
        let grid = all_walkable(5, 5);
        assert_eq!(grid.neighbors(TilePos::new(2, 0)).len(), 3);
    }

    // --- WalkGrid::step_toward ---

    #[test]
    fn step_toward_moves_closer() {
        let grid = all_walkable(10, 10);
        let from = TilePos::new(0, 0);
        let to = TilePos::new(5, 0);
        let next = grid.step_toward(from, to, &[]);
        assert_eq!(next, TilePos::new(1, 0));
    }

    #[test]
    fn step_toward_returns_from_when_no_neighbours() {
        // 1×1 grid — no neighbours possible
        let grid = all_walkable(1, 1);
        let pos = TilePos::new(0, 0);
        assert_eq!(grid.step_toward(pos, pos, &[]), pos);
    }

    #[test]
    fn step_toward_penalises_history() {
        let grid = all_walkable(5, 5);
        let from = TilePos::new(2, 2);
        let to = TilePos::new(3, 2); // prefer right

        // Without history, picks right (distance 1)
        let right = grid.step_toward(from, to, &[]);
        assert_eq!(right, TilePos::new(3, 2));

        // With right in history repeatedly, penalty should redirect
        let history = vec![TilePos::new(3, 2); 3];
        let redirected = grid.step_toward(from, to, &history);
        // Penalty = 3 * 10 = 30, so any distance-1 alternative scores lower
        assert_ne!(redirected, TilePos::new(3, 2));
    }

    // --- WalkGrid::pick_wander_target ---

    #[test]
    fn pick_wander_target_is_walkable() {
        let grid = all_walkable(20, 20);
        let center = TilePos::new(10, 10);
        let target = grid.pick_wander_target(center, 4, 42);
        assert!(grid.is_walkable(target));
    }

    #[test]
    fn pick_wander_target_within_radius() {
        let grid = all_walkable(20, 20);
        let center = TilePos::new(10, 10);
        for seed in 0..20u64 {
            let target = grid.pick_wander_target(center, 4, seed);
            assert!(
                center.x.abs_diff(target.x) <= 4,
                "x out of radius: {target:?}"
            );
            assert!(
                center.y.abs_diff(target.y) <= 4,
                "y out of radius: {target:?}"
            );
        }
    }

    #[test]
    fn pick_wander_target_falls_back_to_center_when_blocked() {
        // 1×1 all-blocked grid
        let grid = WalkGrid::new(3, 3, vec![false; 9]);
        let center = TilePos::new(1, 1);
        let target = grid.pick_wander_target(center, 1, 99);
        assert_eq!(target, center);
    }

    // --- step_citizen ---

    #[test]
    fn step_citizen_moves_toward_target() {
        let grid = all_walkable(10, 10);
        let mut state = make_state(0, 0);
        state.move_target = Some(TilePos::new(5, 0));
        let pos = step_citizen(&mut state, &grid, 0);
        assert_eq!(pos, TilePos::new(1, 0));
    }

    #[test]
    fn step_citizen_respects_cooldown() {
        let grid = all_walkable(10, 10);
        let mut state = make_state(0, 0);
        state.move_target = Some(TilePos::new(5, 0));
        state.move_cooldown = 3;
        let pos = step_citizen(&mut state, &grid, 0);
        assert_eq!(pos, TilePos::new(0, 0)); // did not move
        assert_eq!(state.move_cooldown, 2);
    }

    #[test]
    fn step_citizen_clears_target_on_arrival() {
        let grid = all_walkable(10, 10);
        let mut state = make_state(3, 3);
        state.move_target = Some(TilePos::new(3, 3)); // already at target
        step_citizen(&mut state, &grid, 0);
        assert!(state.move_target.is_none());
        assert!(state.move_cooldown > 0);
    }

    #[test]
    fn step_citizen_updates_facing_on_move_right() {
        let grid = all_walkable(10, 10);
        let mut state = make_state(0, 5);
        state.move_target = Some(TilePos::new(5, 5));
        step_citizen(&mut state, &grid, 0);
        assert_eq!(state.facing, 2); // right
    }

    #[test]
    fn step_citizen_updates_facing_on_move_down() {
        let grid = all_walkable(10, 10);
        let mut state = make_state(5, 0);
        state.move_target = Some(TilePos::new(5, 5));
        step_citizen(&mut state, &grid, 0);
        assert_eq!(state.facing, 0); // down
    }

    #[test]
    fn step_citizen_pushes_history() {
        let grid = all_walkable(10, 10);
        let mut state = make_state(0, 0);
        state.move_target = Some(TilePos::new(3, 0));
        let initial_head = state.history_head;
        step_citizen(&mut state, &grid, 0);
        assert_ne!(state.history_head, initial_head);
        assert_eq!(state.move_history[initial_head], TilePos::new(1, 0));
    }
}
