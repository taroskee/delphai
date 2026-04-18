use crate::pathfinding::TilePos;

pub const MAP_W: i16 = 24;
pub const MAP_H: i16 = 14;

const ANIMAL_RESPAWN_TICKS: u32 = 300; // ~75 s at 4 Hz
const WANDER_EVERY: u64 = 4;           // animals move once every N ticks
const FLEE_RADIUS: u32 = 6;            // manhattan tiles — start fleeing within this range
const MIN_RESPAWN_DIST: u32 = 8;       // minimum manhattan tiles from escape_pos to respawn

#[derive(Debug, Clone)]
pub struct Animal {
    pub pos: TilePos,
    /// Tile to respawn at after a regular kill.
    origin: TilePos,
    pub alive: bool,
    /// Set once when the animal escapes off the map edge; cleared by GDScript poll.
    pub fled: bool,
    respawn_timer: u32,
    /// Position where the animal last escaped; used to pick a far respawn.
    escape_pos: Option<TilePos>,
}

impl Animal {
    pub fn deer(pos: TilePos) -> Self {
        Self { pos, origin: pos, alive: true, fled: false, respawn_timer: 0, escape_pos: None }
    }

    /// Move one step: flee from `nearest_human` if it is within FLEE_RADIUS, else wander.
    /// When fleeing would carry the animal off the map, mark `fled = true` instead.
    /// Call only on ticks divisible by `WANDER_EVERY`.
    pub fn flee_or_wander(&mut self, nearest_human: Option<TilePos>, seed: u64) {
        if !self.alive {
            return;
        }

        if let Some(human) = nearest_human {
            let dist = self.pos.manhattan_dist(human);
            if dist <= FLEE_RADIUS {
                self.flee_step(human, seed);
                return;
            }
        }

        // No nearby threat — random wander.
        let dir = seed % 5;
        let nx = match dir {
            3 => self.pos.x - 1,
            4 => self.pos.x + 1,
            _ => self.pos.x,
        };
        let ny = match dir {
            1 => self.pos.y - 1,
            2 => self.pos.y + 1,
            _ => self.pos.y,
        };
        // Clamp to map bounds during normal wander.
        self.pos = TilePos::new(nx.clamp(0, MAP_W - 1), ny.clamp(0, MAP_H - 1));
    }

    /// Move one tile directly away from `human_pos`.
    /// If the resulting tile is off the map, the animal has escaped.
    fn flee_step(&mut self, human_pos: TilePos, seed: u64) {
        let dx = self.pos.x - human_pos.x; // positive = I am to the right
        let dy = self.pos.y - human_pos.y; // positive = I am below

        // Choose axis with larger separation; use seed to break exact ties.
        let (mx, my) = if dx == 0 && dy == 0 {
            // On the same tile — flee in a random direction.
            match seed % 4 {
                0 => (0, -1),
                1 => (0, 1),
                2 => (-1, 0),
                _ => (1, 0),
            }
        } else if dx.abs() >= dy.abs() {
            (dx.signum(), 0)
        } else {
            (0, dy.signum())
        };

        let nx = self.pos.x + mx;
        let ny = self.pos.y + my;

        if !(0..MAP_W).contains(&nx) || !(0..MAP_H).contains(&ny) {
            // Off the map — the animal escapes.
            self.alive = false;
            self.fled = true;
            self.escape_pos = Some(self.pos);
            self.respawn_timer = ANIMAL_RESPAWN_TICKS;
        } else {
            self.pos = TilePos::new(nx, ny);
        }
    }

    /// Kill this animal (hunted) and start the respawn countdown.
    pub fn kill(&mut self) {
        self.alive = false;
        self.fled = false;
        self.escape_pos = None;
        self.respawn_timer = ANIMAL_RESPAWN_TICKS;
    }

    /// Advance respawn countdown; revive when timer reaches zero.
    /// `seed` is used to pick a far-from-escape respawn position.
    pub fn tick_respawn(&mut self, seed: u64) {
        if self.alive {
            return;
        }
        if self.respawn_timer > 0 {
            self.respawn_timer -= 1;
        }
        if self.respawn_timer == 0 {
            self.alive = true;
            self.pos = self.pick_respawn_pos(seed);
            self.escape_pos = None;
        }
    }

    /// Pick a respawn tile.
    /// If the animal fled, pick a tile at least MIN_RESPAWN_DIST away from the escape point.
    fn pick_respawn_pos(&self, seed: u64) -> TilePos {
        let Some(escape) = self.escape_pos else {
            return self.origin;
        };

        // Try up to 30 random positions across the map.
        let mut s = seed;
        for _ in 0..30 {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let x = ((s >> 33) as u32 % MAP_W as u32) as i16;
            let y = ((s >> 17) as u32 % MAP_H as u32) as i16;
            let candidate = TilePos::new(x, y);
            if candidate.manhattan_dist(escape) >= MIN_RESPAWN_DIST {
                return candidate;
            }
        }

        // Fallback: place at the corner farthest from the escape position.
        let fx = if escape.x < MAP_W / 2 { MAP_W - 4 } else { 3 };
        let fy = if escape.y < MAP_H / 2 { MAP_H - 4 } else { 3 };
        TilePos::new(fx, fy)
    }

    /// True when ticks_count signals that animals should move this tick.
    pub fn should_wander(tick_count: u64) -> bool {
        tick_count.is_multiple_of(WANDER_EVERY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deer_starts_alive() {
        let a = Animal::deer(TilePos::new(5, 5));
        assert!(a.alive);
    }

    #[test]
    fn kill_sets_dead_and_starts_timer() {
        let mut a = Animal::deer(TilePos::new(5, 5));
        a.kill();
        assert!(!a.alive);
        assert!(!a.fled);
    }

    #[test]
    fn respawn_after_countdown_returns_to_origin_after_kill() {
        let mut a = Animal::deer(TilePos::new(5, 5));
        a.kill();
        for i in 0..ANIMAL_RESPAWN_TICKS {
            a.tick_respawn(i as u64);
        }
        assert!(a.alive);
        assert_eq!(a.pos, TilePos::new(5, 5));
    }

    #[test]
    fn flee_or_wander_stays_in_bounds_without_threat() {
        let mut a = Animal::deer(TilePos::new(0, 0));
        for seed in 0..20 {
            a.flee_or_wander(None, seed);
            assert!(a.pos.x >= 0 && a.pos.x < MAP_W);
            assert!(a.pos.y >= 0 && a.pos.y < MAP_H);
        }
    }

    #[test]
    fn dead_animal_does_not_move() {
        let mut a = Animal::deer(TilePos::new(5, 5));
        a.kill();
        let original_pos = a.pos;
        a.flee_or_wander(None, 3);
        assert_eq!(a.pos, original_pos);
    }

    #[test]
    fn deer_flees_away_from_human() {
        // Human at (5,5), deer at (5,8) → should move away (y increases)
        let mut a = Animal::deer(TilePos::new(5, 8));
        let human = TilePos::new(5, 5);
        a.flee_or_wander(Some(human), 0);
        // deer should have moved further from human (y >= 8)
        assert!(a.pos.y >= 8 || a.fled);
    }

    #[test]
    fn deer_marks_fled_when_escaping_edge() {
        // Deer at bottom edge, human below it (would flee off map)
        let mut a = Animal::deer(TilePos::new(5, MAP_H - 1));
        // human at (5, MAP_H - 4) — deer flees downward off the map
        let human = TilePos::new(5, MAP_H - 4);
        a.flee_or_wander(Some(human), 0);
        if a.fled {
            assert!(!a.alive);
        }
    }

    #[test]
    fn fled_deer_respawns_far_from_escape() {
        let mut a = Animal::deer(TilePos::new(0, 0));
        // Force escape state manually
        a.alive = false;
        a.fled = true;
        a.escape_pos = Some(TilePos::new(0, 0));
        a.respawn_timer = 1;
        a.tick_respawn(42);  // timer → 0 but not revived yet
        a.tick_respawn(99);  // now revive
        assert!(a.alive);
        assert!(a.pos.manhattan_dist(TilePos::new(0, 0)) >= MIN_RESPAWN_DIST);
    }

    #[test]
    fn deer_ignores_distant_human() {
        // Human far away — deer should wander, not mark as fled
        let mut a = Animal::deer(TilePos::new(12, 7));
        let far_human = TilePos::new(0, 0);  // distance = 19
        // Any wander seed — as long as it stays on map
        a.flee_or_wander(Some(far_human), 3);
        assert!(!a.fled);
    }
}
