use crate::pathfinding::TilePos;

/// Fullness threshold below which a citizen decides to seek food. Above this
/// level the citizen may idle (random walk). Tuned with `FED_DECAY=0.004/tick`
/// so a full citizen drops into `SeekingFood` after ~150 ticks (~37s at 4Hz).
pub const FED_LOW: f32 = 0.4;

/// Chebyshev distance at or below which a citizen is "adjacent enough" to a
/// resource to start gathering. `1` means one tile away in any of 8
/// directions — also matches the tile the citizen is standing on.
pub const GATHER_RANGE: i16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BehaviorState {
    #[default]
    Idle,
    SeekingFood,
    Gathering,
}

/// Snapshot of needs driving behavior selection. Kept as a plain struct so
/// `decide` stays pure and easily unit-testable.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vitals {
    pub fed: f32,
}

impl Default for Vitals {
    fn default() -> Self {
        Self { fed: 1.0 }
    }
}

/// What the world layer should do for this citizen on the current tick.
/// `decide` never mutates — callers map actions onto `World` state
/// (clearing/setting `move_target`, calling `Resource::gather`, updating
/// `fed`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BehaviorAction {
    /// Stay idle. Random-walk layer keeps roaming if enabled.
    Idle,
    /// Set `move_target` to the given resource tile (citizen is hungry but
    /// not yet adjacent).
    SeekFood { target: TilePos },
    /// Pull from the resource at this index. Citizen is already adjacent.
    Gather { resource_idx: usize },
}

/// Pure decision function. Inputs: current behavior state, vitals, the
/// citizen's tile position, and the nearest-known-non-depleted berry
/// (index + tile) if any. Output: the action to take this tick, plus the
/// new state to persist (the caller stores it alongside the citizen).
///
/// State machine:
/// - `Idle` + `fed ≥ FED_LOW` → stay Idle.
/// - `Idle` + `fed < FED_LOW` + berry found → `SeekingFood` + SeekFood.
///   (no berry found → stay Idle; nothing to do.)
/// - `SeekingFood` + adjacent to berry → `Gathering` + Gather.
/// - `SeekingFood` + not adjacent + berry still exists → SeekFood (keep going).
/// - `SeekingFood` + berry gone → back to Idle.
/// - `Gathering` + still adjacent + fed < 1.0 → keep Gathering + Gather.
/// - `Gathering` + fed full OR resource depleted OR wandered away → Idle.
pub fn decide(
    state: BehaviorState,
    vitals: Vitals,
    citizen_tile: TilePos,
    nearest_berry: Option<(usize, TilePos)>,
) -> (BehaviorState, BehaviorAction) {
    match state {
        BehaviorState::Idle => {
            if vitals.fed >= FED_LOW {
                return (BehaviorState::Idle, BehaviorAction::Idle);
            }
            match nearest_berry {
                Some((_idx, tile)) => (
                    BehaviorState::SeekingFood,
                    BehaviorAction::SeekFood { target: tile },
                ),
                None => (BehaviorState::Idle, BehaviorAction::Idle),
            }
        }
        BehaviorState::SeekingFood => match nearest_berry {
            Some((idx, tile)) => {
                if chebyshev(citizen_tile, tile) <= GATHER_RANGE {
                    (
                        BehaviorState::Gathering,
                        BehaviorAction::Gather { resource_idx: idx },
                    )
                } else {
                    (
                        BehaviorState::SeekingFood,
                        BehaviorAction::SeekFood { target: tile },
                    )
                }
            }
            None => (BehaviorState::Idle, BehaviorAction::Idle),
        },
        BehaviorState::Gathering => match nearest_berry {
            Some((idx, tile))
                if chebyshev(citizen_tile, tile) <= GATHER_RANGE && vitals.fed < 1.0 =>
            {
                (
                    BehaviorState::Gathering,
                    BehaviorAction::Gather { resource_idx: idx },
                )
            }
            _ => (BehaviorState::Idle, BehaviorAction::Idle),
        },
    }
}

fn chebyshev(a: TilePos, b: TilePos) -> i16 {
    (a.x - b.x).abs().max((a.y - b.y).abs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_with_high_fed_stays_idle() {
        let (state, action) = decide(
            BehaviorState::Idle,
            Vitals { fed: 0.9 },
            TilePos::new(0, 0),
            Some((0, TilePos::new(5, 5))),
        );
        assert_eq!(state, BehaviorState::Idle);
        assert_eq!(action, BehaviorAction::Idle);
    }

    #[test]
    fn idle_with_low_fed_and_berry_seeks_food() {
        let (state, action) = decide(
            BehaviorState::Idle,
            Vitals { fed: 0.3 },
            TilePos::new(0, 0),
            Some((2, TilePos::new(5, 5))),
        );
        assert_eq!(state, BehaviorState::SeekingFood);
        assert_eq!(
            action,
            BehaviorAction::SeekFood {
                target: TilePos::new(5, 5)
            }
        );
    }

    #[test]
    fn idle_with_low_fed_but_no_berry_stays_idle() {
        let (state, action) = decide(
            BehaviorState::Idle,
            Vitals { fed: 0.2 },
            TilePos::new(0, 0),
            None,
        );
        assert_eq!(state, BehaviorState::Idle);
        assert_eq!(action, BehaviorAction::Idle);
    }

    #[test]
    fn seeking_food_adjacent_transitions_to_gathering() {
        // Adjacent: chebyshev <= 1. (1, 0) → (0, 0): distance 1.
        let (state, action) = decide(
            BehaviorState::SeekingFood,
            Vitals { fed: 0.3 },
            TilePos::new(1, 0),
            Some((7, TilePos::new(0, 0))),
        );
        assert_eq!(state, BehaviorState::Gathering);
        assert_eq!(action, BehaviorAction::Gather { resource_idx: 7 });
    }

    #[test]
    fn seeking_food_on_same_tile_counts_as_adjacent() {
        let (state, action) = decide(
            BehaviorState::SeekingFood,
            Vitals { fed: 0.3 },
            TilePos::new(4, 4),
            Some((3, TilePos::new(4, 4))),
        );
        assert_eq!(state, BehaviorState::Gathering);
        assert_eq!(action, BehaviorAction::Gather { resource_idx: 3 });
    }

    #[test]
    fn seeking_food_far_keeps_seeking() {
        let (state, action) = decide(
            BehaviorState::SeekingFood,
            Vitals { fed: 0.3 },
            TilePos::new(0, 0),
            Some((1, TilePos::new(5, 5))),
        );
        assert_eq!(state, BehaviorState::SeekingFood);
        assert_eq!(
            action,
            BehaviorAction::SeekFood {
                target: TilePos::new(5, 5)
            }
        );
    }

    #[test]
    fn seeking_food_with_berry_gone_returns_to_idle() {
        let (state, action) = decide(
            BehaviorState::SeekingFood,
            Vitals { fed: 0.3 },
            TilePos::new(0, 0),
            None,
        );
        assert_eq!(state, BehaviorState::Idle);
        assert_eq!(action, BehaviorAction::Idle);
    }

    #[test]
    fn gathering_stays_gathering_when_adjacent_and_hungry() {
        let (state, action) = decide(
            BehaviorState::Gathering,
            Vitals { fed: 0.5 },
            TilePos::new(4, 4),
            Some((3, TilePos::new(4, 4))),
        );
        assert_eq!(state, BehaviorState::Gathering);
        assert_eq!(action, BehaviorAction::Gather { resource_idx: 3 });
    }

    #[test]
    fn gathering_returns_to_idle_when_full() {
        let (state, action) = decide(
            BehaviorState::Gathering,
            Vitals { fed: 1.0 },
            TilePos::new(4, 4),
            Some((3, TilePos::new(4, 4))),
        );
        assert_eq!(state, BehaviorState::Idle);
        assert_eq!(action, BehaviorAction::Idle);
    }

    #[test]
    fn gathering_returns_to_idle_when_resource_depleted() {
        // Resource gone (depleted removal represented as None).
        let (state, action) = decide(
            BehaviorState::Gathering,
            Vitals { fed: 0.5 },
            TilePos::new(4, 4),
            None,
        );
        assert_eq!(state, BehaviorState::Idle);
        assert_eq!(action, BehaviorAction::Idle);
    }

    #[test]
    fn gathering_returns_to_idle_if_wandered_out_of_range() {
        // 5 tiles away is out of chebyshev range 1.
        let (state, action) = decide(
            BehaviorState::Gathering,
            Vitals { fed: 0.5 },
            TilePos::new(0, 0),
            Some((3, TilePos::new(5, 5))),
        );
        assert_eq!(state, BehaviorState::Idle);
        assert_eq!(action, BehaviorAction::Idle);
    }

    #[test]
    fn vitals_default_is_fully_fed() {
        let v = Vitals::default();
        assert!((v.fed - 1.0).abs() < 1e-6);
    }
}
