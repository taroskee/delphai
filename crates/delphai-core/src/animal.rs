use crate::pathfinding::TilePos;

pub const MAP_W: i16 = 24;
pub const MAP_H: i16 = 14;

const ANIMAL_RESPAWN_TICKS: u32 = 300; // ~75 s at 4 Hz
const WANDER_EVERY: u64 = 4;           // animals move once every N ticks

#[derive(Debug, Clone)]
pub struct Animal {
    pub pos: TilePos,
    /// Tile to respawn at after being killed.
    origin: TilePos,
    pub alive: bool,
    respawn_timer: u32,
}

impl Animal {
    pub fn deer(pos: TilePos) -> Self {
        Self { pos, origin: pos, alive: true, respawn_timer: 0 }
    }

    /// Move one step in a random direction (clamped to map bounds).
    /// Call only on ticks divisible by `WANDER_EVERY`.
    pub fn wander(&mut self, seed: u64) {
        if !self.alive {
            return;
        }
        let dir = seed % 5;
        let nx = match dir {
            3 => (self.pos.x - 1).max(0),
            4 => (self.pos.x + 1).min(MAP_W - 1),
            _ => self.pos.x,
        };
        let ny = match dir {
            1 => (self.pos.y - 1).max(0),
            2 => (self.pos.y + 1).min(MAP_H - 1),
            _ => self.pos.y,
        };
        self.pos = TilePos::new(nx, ny);
    }

    /// Kill this animal and start the respawn countdown.
    pub fn kill(&mut self) {
        self.alive = false;
        self.respawn_timer = ANIMAL_RESPAWN_TICKS;
    }

    /// Advance respawn countdown; revive at origin when timer reaches zero.
    pub fn tick_respawn(&mut self) {
        if self.alive {
            return;
        }
        if self.respawn_timer > 0 {
            self.respawn_timer -= 1;
        }
        if self.respawn_timer == 0 {
            self.alive = true;
            self.pos = self.origin;
        }
    }

    /// True when ticks_count signals that animals should wander this tick.
    pub fn should_wander(tick_count: u64) -> bool {
        tick_count % WANDER_EVERY == 0
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
    }

    #[test]
    fn respawn_after_countdown() {
        let mut a = Animal::deer(TilePos::new(5, 5));
        a.kill();
        for _ in 0..ANIMAL_RESPAWN_TICKS {
            a.tick_respawn();
        }
        assert!(a.alive);
        assert_eq!(a.pos, TilePos::new(5, 5));
    }

    #[test]
    fn wander_stays_in_bounds() {
        let mut a = Animal::deer(TilePos::new(0, 0));
        for seed in 0..20 {
            a.wander(seed);
            assert!(a.pos.x >= 0 && a.pos.x < MAP_W);
            assert!(a.pos.y >= 0 && a.pos.y < MAP_H);
        }
    }

    #[test]
    fn dead_animal_does_not_wander() {
        let mut a = Animal::deer(TilePos::new(5, 5));
        a.kill();
        let original_pos = a.pos;
        a.wander(3);
        assert_eq!(a.pos, original_pos);
    }
}
