use crate::pathfinding::TilePos;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    BerryBush,
    WaterSource,
}

impl ResourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ResourceKind::BerryBush => "berry_bush",
            ResourceKind::WaterSource => "water_source",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Resource {
    pub kind: ResourceKind,
    pub pos: TilePos,
    pub quantity: f32,
    pub respawn_timer: u32,
}

const BERRY_BUSH_MAX_QUANTITY: f32 = 1.0;
pub const BERRY_BUSH_RESPAWN_TICKS: u32 = 200;

impl Resource {
    pub fn berry_bush(pos: TilePos) -> Self {
        Self {
            kind: ResourceKind::BerryBush,
            pos,
            quantity: BERRY_BUSH_MAX_QUANTITY,
            respawn_timer: 0,
        }
    }

    pub fn water_source(pos: TilePos) -> Self {
        Self {
            kind: ResourceKind::WaterSource,
            pos,
            quantity: f32::INFINITY,
            respawn_timer: 0,
        }
    }

    /// Returns true if there is something to gather from this resource.
    pub fn is_available(&self) -> bool {
        self.quantity > 0.0
    }

    /// Advance one tick: count down respawn timer and refill when ready.
    pub fn tick(&mut self) {
        if self.kind == ResourceKind::WaterSource {
            return; // water never depletes
        }
        if self.quantity <= 0.0 {
            if self.respawn_timer == 0 {
                self.quantity = BERRY_BUSH_MAX_QUANTITY;
            } else {
                self.respawn_timer -= 1;
            }
        }
    }

    /// Deplete `amount` from this resource. Starts respawn timer when exhausted.
    pub fn deplete(&mut self, amount: f32) {
        self.deplete_with_respawn(amount, BERRY_BUSH_RESPAWN_TICKS);
    }

    /// Deplete `amount` with a custom respawn timer (allows tech effects to speed up regrowth).
    pub fn deplete_with_respawn(&mut self, amount: f32, respawn_ticks: u32) {
        if self.kind == ResourceKind::WaterSource {
            return;
        }
        self.quantity = (self.quantity - amount).max(0.0);
        if self.quantity <= 0.0 {
            self.respawn_timer = respawn_ticks;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn berry_bush_starts_available() {
        let b = Resource::berry_bush(TilePos::new(0, 0));
        assert!(b.is_available());
        assert_eq!(b.kind, ResourceKind::BerryBush);
    }

    #[test]
    fn water_source_always_available() {
        let mut w = Resource::water_source(TilePos::new(0, 0));
        w.deplete(100.0);
        assert!(w.is_available());
    }

    #[test]
    fn berry_bush_depletes() {
        let mut b = Resource::berry_bush(TilePos::new(0, 0));
        b.deplete(1.0);
        assert!(!b.is_available());
    }

    #[test]
    fn berry_bush_respawns_after_timer() {
        let mut b = Resource::berry_bush(TilePos::new(0, 0));
        b.deplete(1.0);
        assert!(!b.is_available());
        // Drain timer to 0
        for _ in 0..BERRY_BUSH_RESPAWN_TICKS {
            b.tick();
        }
        // Next tick refills
        b.tick();
        assert!(b.is_available());
    }
}
