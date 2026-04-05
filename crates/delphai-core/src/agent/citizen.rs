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
    /// 0.0 (stranger) to 1.0 (intimate). How well they know each other.
    pub familiarity: f32,
    /// 0.0 (distrust) to 1.0 (complete trust).
    pub trust: f32,
}

impl Relationship {
    pub fn clamp(&mut self) {
        self.familiarity = self.familiarity.clamp(0.0, 1.0);
        self.trust = self.trust.clamp(0.0, 1.0);
    }
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
                familiarity: 0.6,
                trust: 0.8,
            }],
            divine_awareness: 0.15,
        };
        let json = serde_json::to_string(&citizen).unwrap();
        let restored: Citizen = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name, "Kael");
        assert_eq!(restored.emotion, Emotion::Happy);
        assert!((restored.relationships[0].familiarity - 0.6).abs() < f32::EPSILON);
        assert!((restored.relationships[0].trust - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn relationship_clamp() {
        let mut rel = Relationship {
            target_name: "test".into(),
            familiarity: 1.5,
            trust: -0.3,
        };
        rel.clamp();
        assert_eq!(rel.familiarity, 1.0);
        assert_eq!(rel.trust, 0.0);
    }

    #[test]
    fn relationship_serde_roundtrip() {
        let rel = Relationship {
            target_name: "Elder".into(),
            familiarity: 0.7,
            trust: 0.3,
        };
        let json = serde_json::to_string(&rel).unwrap();
        let restored: Relationship = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.target_name, "Elder");
        assert!((restored.familiarity - 0.7).abs() < f32::EPSILON);
        assert!((restored.trust - 0.3).abs() < f32::EPSILON);
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
