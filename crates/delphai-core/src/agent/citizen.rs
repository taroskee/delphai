use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Emotion {
    #[default]
    Neutral,
    Happy,
    Anxious,
    Angry,
    Sad,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Relationship {
    pub target_name: String,
    /// -100 (hostile) to +100 (devoted).
    pub affinity: i8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citizen {
    pub name: String,
    pub personality_tags: Vec<String>,
    pub memory_summary: String,
    pub emotion: Emotion,
    pub relationships: Vec<Relationship>,
    /// 0.0 (unaware) to 1.0 (fully aware of the god/player).
    pub divine_awareness: f32,
}

impl Citizen {
    pub fn clamp_awareness(&mut self) {
        self.divine_awareness = self.divine_awareness.clamp(0.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn citizen_default_emotion_is_neutral() {
        assert_eq!(Emotion::default(), Emotion::Neutral);
    }

    #[test]
    fn citizen_roundtrip_serde() {
        let citizen = Citizen {
            name: "Kael".into(),
            personality_tags: vec!["curious".into(), "young".into()],
            memory_summary: "found berries near the river".into(),
            emotion: Emotion::Happy,
            relationships: vec![Relationship {
                target_name: "Elder".into(),
                affinity: 30,
            }],
            divine_awareness: 0.15,
        };
        let json = serde_json::to_string(&citizen).unwrap();
        let restored: Citizen = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name, "Kael");
        assert_eq!(restored.emotion, Emotion::Happy);
        assert_eq!(restored.relationships[0].affinity, 30);
    }

    #[test]
    fn relationship_affinity_boundary() {
        let lo = Relationship { target_name: "enemy".into(), affinity: -100 };
        let hi = Relationship { target_name: "friend".into(), affinity: 100 };
        let lo_json = serde_json::to_string(&lo).unwrap();
        let hi_json = serde_json::to_string(&hi).unwrap();
        let lo_back: Relationship = serde_json::from_str(&lo_json).unwrap();
        let hi_back: Relationship = serde_json::from_str(&hi_json).unwrap();
        assert_eq!(lo_back.affinity, -100);
        assert_eq!(hi_back.affinity, 100);
    }

    #[test]
    fn divine_awareness_clamp() {
        let mut citizen = Citizen {
            name: "test".into(),
            personality_tags: vec![],
            memory_summary: String::new(),
            emotion: Emotion::default(),
            relationships: vec![],
            divine_awareness: 1.5,
        };
        citizen.clamp_awareness();
        assert_eq!(citizen.divine_awareness, 1.0);

        citizen.divine_awareness = -0.5;
        citizen.clamp_awareness();
        assert_eq!(citizen.divine_awareness, 0.0);
    }
}
