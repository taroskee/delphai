use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BehaviorState {
    #[default]
    Idle,
    SeekingFood,
    Gathering,
    SeekingWater,
    Drinking,
    /// Pursuing an animal for cooperative hunting. Transitions to Idle when fed.
    Hunting,
}

impl BehaviorState {
    pub fn as_str(self) -> &'static str {
        match self {
            BehaviorState::Idle => "idle",
            BehaviorState::SeekingFood => "seeking_food",
            BehaviorState::Gathering => "gathering",
            BehaviorState::SeekingWater => "seeking_water",
            BehaviorState::Drinking => "drinking",
            BehaviorState::Hunting => "hunting",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BehaviorAction {
    Stay,
    TransitionTo(BehaviorState),
}

/// Needs are bigger-is-better: 1.0 = fully sated, 0.0 = critical.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Needs {
    pub fed: f32,
    pub hydration: f32,
}

const HYDRATION_SEEK_THRESHOLD: f32 = 0.3;
const HYDRATION_SATED: f32 = 0.9;
const FED_SEEK_THRESHOLD: f32 = 0.3;
const FED_SATED: f32 = 0.9;

/// Pure function: given current state and needs, decide the next behavior action.
///
/// Hydration takes priority over food. The world is responsible for driving movement
/// and transitioning SeekingFood→Gathering or SeekingWater→Drinking when the citizen
/// arrives at a resource.
pub fn tick(state: BehaviorState, needs: &Needs) -> BehaviorAction {
    // Hydration is critical — override everything except already handling water.
    if needs.hydration < HYDRATION_SEEK_THRESHOLD {
        match state {
            BehaviorState::SeekingWater | BehaviorState::Drinking => {}
            _ => return BehaviorAction::TransitionTo(BehaviorState::SeekingWater),
        }
    }

    match state {
        BehaviorState::Drinking => {
            if needs.hydration >= HYDRATION_SATED {
                BehaviorAction::TransitionTo(BehaviorState::Idle)
            } else {
                BehaviorAction::Stay
            }
        }
        BehaviorState::SeekingWater => BehaviorAction::Stay,

        BehaviorState::Gathering => {
            if needs.fed >= FED_SATED {
                BehaviorAction::TransitionTo(BehaviorState::Idle)
            } else {
                BehaviorAction::Stay
            }
        }
        BehaviorState::SeekingFood => BehaviorAction::Stay,

        BehaviorState::Idle => {
            if needs.fed < FED_SEEK_THRESHOLD {
                BehaviorAction::TransitionTo(BehaviorState::SeekingFood)
            } else {
                BehaviorAction::Stay
            }
        }

        BehaviorState::Hunting => {
            if needs.fed >= FED_SATED {
                BehaviorAction::TransitionTo(BehaviorState::Idle)
            } else {
                BehaviorAction::Stay
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full() -> Needs {
        Needs { fed: 1.0, hydration: 1.0 }
    }

    fn hungry() -> Needs {
        Needs { fed: 0.1, hydration: 1.0 }
    }

    fn thirsty() -> Needs {
        Needs { fed: 1.0, hydration: 0.1 }
    }

    fn both_critical() -> Needs {
        Needs { fed: 0.1, hydration: 0.1 }
    }

    #[test]
    fn default_state_is_idle() {
        assert_eq!(BehaviorState::default(), BehaviorState::Idle);
    }

    #[test]
    fn idle_stays_idle_when_needs_full() {
        assert_eq!(tick(BehaviorState::Idle, &full()), BehaviorAction::Stay);
    }

    #[test]
    fn idle_seeks_food_when_hungry() {
        assert_eq!(
            tick(BehaviorState::Idle, &hungry()),
            BehaviorAction::TransitionTo(BehaviorState::SeekingFood)
        );
    }

    #[test]
    fn idle_seeks_water_when_thirsty() {
        assert_eq!(
            tick(BehaviorState::Idle, &thirsty()),
            BehaviorAction::TransitionTo(BehaviorState::SeekingWater)
        );
    }

    #[test]
    fn hydration_overrides_food_seek() {
        // Both critical → water first
        assert_eq!(
            tick(BehaviorState::Idle, &both_critical()),
            BehaviorAction::TransitionTo(BehaviorState::SeekingWater)
        );
    }

    #[test]
    fn gathering_interrupted_by_thirst() {
        // Even mid-gather, switch to water if critically thirsty
        assert_eq!(
            tick(BehaviorState::Gathering, &thirsty()),
            BehaviorAction::TransitionTo(BehaviorState::SeekingWater)
        );
    }

    #[test]
    fn gathering_stays_while_hungry() {
        let needs = Needs { fed: 0.5, hydration: 1.0 };
        assert_eq!(tick(BehaviorState::Gathering, &needs), BehaviorAction::Stay);
    }

    #[test]
    fn gathering_returns_to_idle_when_sated() {
        let needs = Needs { fed: 0.95, hydration: 1.0 };
        assert_eq!(
            tick(BehaviorState::Gathering, &needs),
            BehaviorAction::TransitionTo(BehaviorState::Idle)
        );
    }

    #[test]
    fn drinking_stays_while_thirsty() {
        let needs = Needs { fed: 1.0, hydration: 0.5 };
        assert_eq!(tick(BehaviorState::Drinking, &needs), BehaviorAction::Stay);
    }

    #[test]
    fn drinking_returns_to_idle_when_hydrated() {
        let needs = Needs { fed: 1.0, hydration: 0.95 };
        assert_eq!(
            tick(BehaviorState::Drinking, &needs),
            BehaviorAction::TransitionTo(BehaviorState::Idle)
        );
    }

    #[test]
    fn seeking_food_stays_seeking() {
        assert_eq!(tick(BehaviorState::SeekingFood, &hungry()), BehaviorAction::Stay);
    }

    #[test]
    fn seeking_water_stays_seeking() {
        assert_eq!(tick(BehaviorState::SeekingWater, &thirsty()), BehaviorAction::Stay);
    }

    #[test]
    fn seeking_food_interrupted_by_thirst() {
        assert_eq!(
            tick(BehaviorState::SeekingFood, &thirsty()),
            BehaviorAction::TransitionTo(BehaviorState::SeekingWater)
        );
    }
}
