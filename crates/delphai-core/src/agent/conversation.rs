use serde::{Deserialize, Serialize};

use super::behavior::BehaviorState;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConversationRequest {
    pub initiator: String,
    pub partner: String,
}

/// Check which citizen pairs should start a conversation this tick.
///
/// `random_roll`: injected randomness in 0.0..1.0 for testability.
/// Only Idle citizens are eligible. Each citizen appears in at most one conversation.
pub fn check_conversations(
    citizens: &[(String, Position, BehaviorState)],
    proximity_threshold: f32,
    probability: f32,
    random_roll: f32,
) -> Vec<ConversationRequest> {
    if random_roll >= probability {
        return Vec::new();
    }

    let mut idle: Vec<_> = citizens
        .iter()
        .filter(|(_, _, state)| *state == BehaviorState::Idle)
        .collect();

    // Rotate the idle list so every citizen gets a fair chance to initiate.
    // Uses random_roll to pick a starting offset, breaking the Kael-Elder lock.
    if idle.len() > 1 {
        let offset = (random_roll * idle.len() as f32) as usize % idle.len();
        idle.rotate_left(offset);
    }

    let threshold_sq = proximity_threshold * proximity_threshold;
    let mut used = vec![false; idle.len()];
    let mut requests = Vec::new();

    for i in 0..idle.len() {
        if used[i] {
            continue;
        }
        for j in (i + 1)..idle.len() {
            if used[j] {
                continue;
            }
            let dx = idle[i].1.x - idle[j].1.x;
            let dy = idle[i].1.y - idle[j].1.y;
            if dx * dx + dy * dy < threshold_sq {
                requests.push(ConversationRequest {
                    initiator: idle[i].0.clone(),
                    partner: idle[j].0.clone(),
                });
                used[i] = true;
                used[j] = true;
                break;
            }
        }
    }

    requests
}

#[cfg(test)]
mod tests {
    use super::*;

    fn idle(name: &str, x: f32, y: f32) -> (String, Position, BehaviorState) {
        (name.into(), Position { x, y }, BehaviorState::Idle)
    }

    fn with_state(name: &str, x: f32, y: f32, state: BehaviorState) -> (String, Position, BehaviorState) {
        (name.into(), Position { x, y }, state)
    }

    #[test]
    fn empty_citizen_list_returns_empty() {
        let r = check_conversations(&[], 50.0, 0.5, 0.0);
        assert!(r.is_empty());
    }

    #[test]
    fn single_citizen_returns_empty() {
        let citizens = vec![idle("A", 0.0, 0.0)];
        let r = check_conversations(&citizens, 50.0, 0.5, 0.0);
        assert!(r.is_empty());
    }

    #[test]
    fn no_conversation_when_too_far() {
        let citizens = vec![idle("A", 0.0, 0.0), idle("B", 200.0, 0.0)];
        let r = check_conversations(&citizens, 50.0, 0.5, 0.0);
        assert!(r.is_empty());
    }

    #[test]
    fn no_conversation_when_roll_exceeds_probability() {
        let citizens = vec![idle("A", 0.0, 0.0), idle("B", 10.0, 0.0)];
        let r = check_conversations(&citizens, 50.0, 0.3, 0.5);
        assert!(r.is_empty());
    }

    #[test]
    fn conversation_triggers_when_close_and_lucky() {
        let citizens = vec![idle("A", 0.0, 0.0), idle("B", 10.0, 0.0)];
        let r = check_conversations(&citizens, 50.0, 0.5, 0.3);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].initiator, "A");
        assert_eq!(r[0].partner, "B");
    }

    #[test]
    fn sleeping_citizen_excluded() {
        let citizens = vec![
            idle("A", 0.0, 0.0),
            with_state("B", 10.0, 0.0, BehaviorState::Sleeping),
        ];
        let r = check_conversations(&citizens, 50.0, 0.5, 0.0);
        assert!(r.is_empty());
    }

    #[test]
    fn eating_citizen_excluded() {
        let citizens = vec![
            idle("A", 0.0, 0.0),
            with_state("B", 10.0, 0.0, BehaviorState::Eating),
        ];
        let r = check_conversations(&citizens, 50.0, 0.5, 0.0);
        assert!(r.is_empty());
    }

    #[test]
    fn three_citizens_at_most_one_conversation() {
        let citizens = vec![
            idle("A", 0.0, 0.0),
            idle("B", 5.0, 0.0),
            idle("C", 10.0, 0.0),
        ];
        let r = check_conversations(&citizens, 50.0, 1.0, 0.0);
        assert_eq!(r.len(), 1, "each citizen in at most one conversation per tick");
    }

    #[test]
    fn conversation_request_has_correct_names() {
        let citizens = vec![idle("Kael", 0.0, 0.0), idle("Elder", 5.0, 0.0)];
        let r = check_conversations(&citizens, 50.0, 1.0, 0.0);
        assert_eq!(r[0].initiator, "Kael");
        assert_eq!(r[0].partner, "Elder");
    }

    #[test]
    fn rotation_gives_third_citizen_a_chance() {
        // With 3 close citizens and roll=0.67, offset=2 → [C, A, B] → C-A converses
        let citizens = vec![
            idle("Kael", 0.0, 0.0),
            idle("Elder", 5.0, 0.0),
            idle("Hara", 10.0, 0.0),
        ];
        // roll=0.67 → offset = (0.67 * 3.0) as usize = 2 → rotate by 2 → [Hara, Kael, Elder]
        let r = check_conversations(&citizens, 50.0, 1.0, 0.67);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].initiator, "Hara");
    }
}
