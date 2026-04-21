use crate::agent::Citizen;
use crate::pathfinding::TilePos;

#[derive(Debug, Default)]
pub struct World {
    pub tick_count: u32,
    pub citizens: Vec<Citizen>,
    pub citizen_positions: Vec<TilePos>,
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tick(&mut self) {
        self.tick_count += 1;
    }

    /// Spawn a new citizen at `tile_pos`. Returns the index in `citizens` /
    /// `citizen_positions` (kept in parallel — never reorder one without the other).
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
        self.citizen_positions.push(tile_pos);
        idx
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
        assert_eq!(w.citizen_positions[0], TilePos { x: 3, y: 7 });
    }

    #[test]
    fn spawn_citizen_assigns_sequential_indices() {
        let mut w = World::new();
        let a = w.spawn_citizen("A", TilePos { x: 0, y: 0 });
        let b = w.spawn_citizen("B", TilePos { x: 1, y: 2 });
        assert_eq!(a, 0);
        assert_eq!(b, 1);
        assert_eq!(w.citizen_positions[1], TilePos { x: 1, y: 2 });
    }
}
