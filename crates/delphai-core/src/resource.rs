use crate::pathfinding::TilePos;

/// Maximum berry amount on a fully-regenerated bush. Chosen so a citizen can
/// gather ~5 times from a full bush at `GATHER_PER_TICK=1.0` before it empties.
pub const BERRY_AMOUNT_MAX: f32 = 5.0;

/// Amount a citizen pulls from a resource in a single `gather` call (one tick
/// of `Gathering` behavior). Caller clamps at zero.
pub const GATHER_PER_TICK: f32 = 1.0;

/// Per-tick regeneration rate for a berry bush. 0.01/tick ≈ one full bush
/// every ~500 ticks (~2 min at 4Hz) — slow enough that depletion matters,
/// fast enough that abandoned bushes recover within a session.
pub const BERRY_REGEN_PER_TICK: f32 = 0.01;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    Berry,
}

/// A harvestable tile-anchored resource. `amount` is a float so partial
/// regeneration accumulates smoothly; callers `gather` to deplete and
/// `regenerate` each tick to refill toward `BERRY_AMOUNT_MAX`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Resource {
    pub kind: ResourceKind,
    pub amount: f32,
    pub tile_pos: TilePos,
}

impl Resource {
    pub fn new_berry(tile_pos: TilePos) -> Self {
        Self {
            kind: ResourceKind::Berry,
            amount: BERRY_AMOUNT_MAX,
            tile_pos,
        }
    }

    /// Returns the amount actually taken (0.0 when already empty). Caller
    /// inspects the return value to decide whether the citizen was fed.
    pub fn gather(&mut self) -> f32 {
        let taken = self.amount.min(GATHER_PER_TICK);
        self.amount -= taken;
        if self.amount < 0.0 {
            self.amount = 0.0;
        }
        taken
    }

    /// Apply one tick of regeneration. Clamped at `BERRY_AMOUNT_MAX` so a
    /// bush never exceeds its cap.
    pub fn regenerate(&mut self) {
        let rate = match self.kind {
            ResourceKind::Berry => BERRY_REGEN_PER_TICK,
        };
        self.amount = (self.amount + rate).min(BERRY_AMOUNT_MAX);
    }

    pub fn is_depleted(&self) -> bool {
        self.amount <= 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-6;

    #[test]
    fn new_berry_starts_at_max_amount() {
        let r = Resource::new_berry(TilePos::new(3, 4));
        assert_eq!(r.kind, ResourceKind::Berry);
        assert_eq!(r.tile_pos, TilePos::new(3, 4));
        assert!((r.amount - BERRY_AMOUNT_MAX).abs() < EPS);
    }

    #[test]
    fn gather_decreases_amount_by_gather_per_tick() {
        let mut r = Resource::new_berry(TilePos::new(0, 0));
        let taken = r.gather();
        assert!((taken - GATHER_PER_TICK).abs() < EPS);
        assert!((r.amount - (BERRY_AMOUNT_MAX - GATHER_PER_TICK)).abs() < EPS);
    }

    #[test]
    fn gather_returns_zero_and_leaves_amount_at_zero_when_empty() {
        let mut r = Resource::new_berry(TilePos::new(0, 0));
        r.amount = 0.0;
        let taken = r.gather();
        assert!(taken.abs() < EPS);
        assert!(r.amount.abs() < EPS);
    }

    #[test]
    fn gather_on_nearly_empty_takes_only_remaining() {
        let mut r = Resource::new_berry(TilePos::new(0, 0));
        r.amount = 0.3;
        let taken = r.gather();
        assert!((taken - 0.3).abs() < EPS);
        assert!(r.amount.abs() < EPS);
    }

    #[test]
    fn regenerate_increments_amount_by_rate() {
        let mut r = Resource::new_berry(TilePos::new(0, 0));
        r.amount = 2.0;
        r.regenerate();
        assert!((r.amount - (2.0 + BERRY_REGEN_PER_TICK)).abs() < EPS);
    }

    #[test]
    fn regenerate_clamps_at_max() {
        let mut r = Resource::new_berry(TilePos::new(0, 0));
        r.amount = BERRY_AMOUNT_MAX;
        r.regenerate();
        assert!((r.amount - BERRY_AMOUNT_MAX).abs() < EPS);
    }

    #[test]
    fn regenerate_from_empty_eventually_refills() {
        let mut r = Resource::new_berry(TilePos::new(0, 0));
        r.amount = 0.0;
        // BERRY_AMOUNT_MAX=5, rate=0.01 → 500 ticks to full.
        for _ in 0..600 {
            r.regenerate();
        }
        assert!((r.amount - BERRY_AMOUNT_MAX).abs() < EPS);
    }

    #[test]
    fn is_depleted_reflects_zero_amount() {
        let mut r = Resource::new_berry(TilePos::new(0, 0));
        assert!(!r.is_depleted());
        r.amount = 0.0;
        assert!(r.is_depleted());
    }
}
