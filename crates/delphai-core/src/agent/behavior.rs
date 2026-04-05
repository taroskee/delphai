use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BehaviorState {
    #[default]
    Idle,
    Moving,
    Eating,
    Sleeping,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BehaviorAction {
    Stay,
    TransitionTo(BehaviorState),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Needs {
    pub hunger: f32,
    pub fatigue: f32,
}

const FATIGUE_SLEEP_THRESHOLD: f32 = 0.8;
const FATIGUE_WAKE_THRESHOLD: f32 = 0.1;
const HUNGER_EAT_THRESHOLD: f32 = 0.7;
const HUNGER_SATED_THRESHOLD: f32 = 0.1;

/// Pure function: given current state and needs, decide the next action.
pub fn tick(state: BehaviorState, needs: &Needs) -> BehaviorAction {
    // Sleep overrides everything except already sleeping
    if needs.fatigue >= FATIGUE_SLEEP_THRESHOLD && state != BehaviorState::Sleeping {
        return BehaviorAction::TransitionTo(BehaviorState::Sleeping);
    }

    match state {
        BehaviorState::Sleeping => {
            if needs.fatigue <= FATIGUE_WAKE_THRESHOLD {
                BehaviorAction::TransitionTo(BehaviorState::Idle)
            } else {
                BehaviorAction::Stay
            }
        }
        BehaviorState::Eating => {
            if needs.hunger <= HUNGER_SATED_THRESHOLD {
                BehaviorAction::TransitionTo(BehaviorState::Idle)
            } else {
                BehaviorAction::Stay
            }
        }
        BehaviorState::Idle | BehaviorState::Moving => {
            if needs.hunger >= HUNGER_EAT_THRESHOLD {
                BehaviorAction::TransitionTo(BehaviorState::Eating)
            } else {
                BehaviorAction::Stay
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_idle() {
        assert_eq!(BehaviorState::default(), BehaviorState::Idle);
    }

    #[test]
    fn idle_stays_idle_when_needs_low() {
        let r = tick(BehaviorState::Idle, &Needs { hunger: 0.1, fatigue: 0.1 });
        assert_eq!(r, BehaviorAction::Stay);
    }

    #[test]
    fn idle_to_eating_when_hungry() {
        let r = tick(BehaviorState::Idle, &Needs { hunger: 0.8, fatigue: 0.1 });
        assert_eq!(r, BehaviorAction::TransitionTo(BehaviorState::Eating));
    }

    #[test]
    fn idle_to_sleeping_when_fatigued() {
        let r = tick(BehaviorState::Idle, &Needs { hunger: 0.1, fatigue: 0.9 });
        assert_eq!(r, BehaviorAction::TransitionTo(BehaviorState::Sleeping));
    }

    #[test]
    fn fatigue_overrides_hunger() {
        let r = tick(BehaviorState::Idle, &Needs { hunger: 0.9, fatigue: 0.9 });
        assert_eq!(r, BehaviorAction::TransitionTo(BehaviorState::Sleeping));
    }

    #[test]
    fn eating_stays_eating_while_hungry() {
        let r = tick(BehaviorState::Eating, &Needs { hunger: 0.5, fatigue: 0.1 });
        assert_eq!(r, BehaviorAction::Stay);
    }

    #[test]
    fn eating_to_idle_when_sated() {
        let r = tick(BehaviorState::Eating, &Needs { hunger: 0.05, fatigue: 0.1 });
        assert_eq!(r, BehaviorAction::TransitionTo(BehaviorState::Idle));
    }

    #[test]
    fn eating_to_sleeping_when_exhausted() {
        let r = tick(BehaviorState::Eating, &Needs { hunger: 0.5, fatigue: 0.9 });
        assert_eq!(r, BehaviorAction::TransitionTo(BehaviorState::Sleeping));
    }

    #[test]
    fn sleeping_stays_sleeping_while_tired() {
        let r = tick(BehaviorState::Sleeping, &Needs { hunger: 0.5, fatigue: 0.5 });
        assert_eq!(r, BehaviorAction::Stay);
    }

    #[test]
    fn sleeping_to_idle_when_rested() {
        let r = tick(BehaviorState::Sleeping, &Needs { hunger: 0.5, fatigue: 0.05 });
        assert_eq!(r, BehaviorAction::TransitionTo(BehaviorState::Idle));
    }

    #[test]
    fn sleeping_ignores_hunger() {
        let r = tick(BehaviorState::Sleeping, &Needs { hunger: 0.99, fatigue: 0.5 });
        assert_eq!(r, BehaviorAction::Stay);
    }
}
