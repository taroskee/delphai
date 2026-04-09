use crate::agent::{Citizen, Emotion};

/// World context injected into every prompt.
#[derive(Debug, Clone)]
pub struct WorldContext {
    pub era: String,
    pub setting: String,
}

/// A conversation prompt between two citizens.
#[derive(Debug, Clone)]
pub struct ConversationPromptInput<'a> {
    pub world: &'a WorldContext,
    pub initiator: &'a Citizen,
    pub partner: &'a Citizen,
    /// Optional divine voice (player message). Filtered by initiator's divine_awareness.
    pub divine_voice: Option<&'a str>,
}

/// A prompt for a citizen reacting to the divine voice alone.
#[derive(Debug, Clone)]
pub struct DivineVoicePromptInput<'a> {
    pub world: &'a WorldContext,
    pub citizen: &'a Citizen,
    pub message: &'a str,
}

/// Format a single citizen's profile block for inclusion in a prompt.
fn format_citizen_block(citizen: &Citizen) -> String {
    format!(
        "[{}]\nPersonality: {}\nEmotion: {}\nMemory: {}",
        citizen.name,
        citizen.personality_tags.join(", "),
        emotion_label(&citizen.emotion),
        citizen.memory_summary,
    )
}

/// Format the relationship line from `initiator` to `partner`, if one exists.
fn format_relationship_line(initiator: &Citizen, partner_name: &str) -> Option<String> {
    initiator
        .relationships
        .iter()
        .find(|r| r.target_name == partner_name)
        .map(|rel| {
            format!(
                "Relationship with {}: familiarity {:.0}%, trust {:.0}%",
                partner_name,
                rel.familiarity * 100.0,
                rel.trust * 100.0,
            )
        })
}

/// Build a conversation prompt string for two citizens.
pub fn build_conversation_prompt(input: &ConversationPromptInput) -> String {
    let mut parts = Vec::new();

    parts.push(format!(
        "You are simulating a primitive civilization. Era: {}. Setting: {}.\nAll speech must be written in Japanese.",
        input.world.era, input.world.setting
    ));

    parts.push(format!("\n{}", format_citizen_block(input.initiator)));

    if let Some(line) = format_relationship_line(input.initiator, &input.partner.name) {
        parts.push(line);
    }

    parts.push(format!("\n{}", format_citizen_block(input.partner)));

    if let Some(voice) = input.divine_voice {
        if let Some(text) = filter_divine_voice(voice, input.initiator.divine_awareness) {
            parts.push(format!(
                "\n[SUPERNATURAL EVENT — respond to this NOW]\nJust this moment, {} heard a mysterious disembodied voice say: \"{text}\"\nThis was sudden and inexplicable — {} must react with surprise and share what they heard with {}. Do NOT treat this as a topic or theme. It is a real event that just happened.",
                input.initiator.name, input.initiator.name, input.partner.name
            ));
        }
    }

    parts.push(format!(
        "\nGenerate {}'s response as YAML (speech must be in Japanese):\nspeech: ...\ninner_thought: ...\naction: ...\nemotion_change: ...\ntech_hint: ~  # null if none",
        input.initiator.name,
    ));

    parts.join("\n")
}

/// Build a prompt for a single citizen reacting to the divine voice.
pub fn build_divine_voice_prompt(input: &DivineVoicePromptInput) -> String {
    let mut parts = Vec::new();

    parts.push(format!(
        "You are simulating a primitive civilization. Era: {}. Setting: {}.",
        input.world.era, input.world.setting
    ));

    parts.push(format!(
        "\n[{}]\nPersonality: {}\nEmotion: {}\nMemory: {}\nDivine awareness: {:.0}%",
        input.citizen.name,
        input.citizen.personality_tags.join(", "),
        emotion_label(&input.citizen.emotion),
        input.citizen.memory_summary,
        input.citizen.divine_awareness * 100.0,
    ));

    let filtered = filter_divine_voice(input.message, input.citizen.divine_awareness);
    if let Some(text) = filtered {
        parts.push(format!("\n[Divine Voice]: {text}"));
    } else {
        parts.push("\n[Divine Voice]: (nothing reaches this citizen)".into());
    }

    parts.push(format!(
        "\nGenerate {}'s response as YAML:\nspeech: ...\ninner_thought: ...\naction: ...\nemotion_change: ...\ntech_hint: ~  # null if none",
        input.citizen.name,
    ));

    parts.join("\n")
}

/// Filter divine voice based on citizen's awareness level.
/// Returns None if awareness is too low for anything to get through.
///
/// Tiers (from todo.md):
///   0%:      nothing
///   1-30%:   noise (garbled fragments)
///   31-60%:  fragments (partial words)
///   61-90%:  oracle (mostly clear)
///   91-100%: verbatim
pub fn filter_divine_voice(message: &str, awareness: f32) -> Option<String> {
    if awareness <= 0.0 {
        return None;
    }
    if awareness >= 0.91 {
        return Some(message.to_string());
    }
    if awareness >= 0.61 {
        return Some(format!("(oracle) {message}"));
    }
    if awareness >= 0.31 {
        // Show fragments: keep only some characters
        let fragment: String = message
            .chars()
            .enumerate()
            .map(|(i, c)| if i % 3 == 0 || c == ' ' { c } else { '…' })
            .collect();
        return Some(format!("(fragments) {fragment}"));
    }
    // 1-30%: noise
    Some(format!("(noise) {}", garble(message)))
}

fn garble(message: &str) -> String {
    message
        .chars()
        .enumerate()
        .map(|(i, c)| {
            if c == ' ' {
                ' '
            } else if i % 5 == 0 {
                c
            } else {
                '…'
            }
        })
        .collect()
}

/// Input for a batch prompt that packs multiple conversation pairs into one LLM call.
#[derive(Debug, Clone)]
pub struct BatchConversationInput<'a> {
    pub world: &'a WorldContext,
    pub pairs: Vec<(&'a Citizen, &'a Citizen)>,
    pub divine_voice: Option<&'a str>,
}

/// Build a single prompt containing multiple conversation pairs.
/// The LLM is instructed to return a JSON array with one entry per pair.
pub fn build_batch_conversation_prompt(input: &BatchConversationInput) -> String {
    let mut parts = Vec::new();

    parts.push(format!(
        "You are simulating a primitive civilization. Era: {}. Setting: {}.",
        input.world.era, input.world.setting
    ));

    for (idx, (initiator, partner)) in input.pairs.iter().enumerate() {
        parts.push(format!("\n--- Pair {} ---", idx + 1));

        parts.push(format_citizen_block(initiator));

        if let Some(line) = format_relationship_line(initiator, &partner.name) {
            parts.push(line);
        }

        parts.push(format_citizen_block(partner));

        if let Some(voice) = input.divine_voice {
            if let Some(text) = filter_divine_voice(voice, initiator.divine_awareness) {
                parts.push(format!("[Divine Voice for {}]: {text}", initiator.name));
            }
        }
    }

    parts.push(format!(
        "\nRespond with a YAML sequence of {} mappings, one per pair:\n- speech: ...\n  inner_thought: ...\n  action: ...\n  emotion_change: ...\n  tech_hint: ~",
        input.pairs.len(),
    ));

    parts.join("\n")
}

fn emotion_label(emotion: &Emotion) -> &'static str {
    match emotion {
        Emotion::Neutral => "neutral",
        Emotion::Happy => "happy",
        Emotion::Anxious => "anxious",
        Emotion::Angry => "angry",
        Emotion::Sad => "sad",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{Citizen, Emotion, Relationship};

    fn test_world() -> WorldContext {
        WorldContext {
            era: "Stone Age".into(),
            setting: "A small tribe near a river".into(),
        }
    }

    fn make_citizen(name: &str, personality: &[&str], emotion: Emotion, awareness: f32) -> Citizen {
        Citizen {
            name: name.into(),
            personality_tags: personality.iter().map(|s| s.to_string()).collect(),
            memory_summary: "nothing notable".into(),
            emotion,
            relationships: vec![],
            divine_awareness: awareness,
        }
    }

    // --- filter_divine_voice ---

    #[test]
    fn divine_voice_zero_awareness_returns_none() {
        assert!(filter_divine_voice("hello", 0.0).is_none());
    }

    #[test]
    fn divine_voice_full_awareness_returns_verbatim() {
        let result = filter_divine_voice("go north", 1.0).unwrap();
        assert_eq!(result, "go north");
    }

    #[test]
    fn divine_voice_high_awareness_is_oracle() {
        let result = filter_divine_voice("go north", 0.75).unwrap();
        assert!(result.contains("oracle"), "expected oracle tag: {result}");
        assert!(result.contains("go north"));
    }

    #[test]
    fn divine_voice_medium_awareness_is_fragments() {
        let result = filter_divine_voice("go north", 0.45).unwrap();
        assert!(result.contains("fragments"), "expected fragments tag: {result}");
        assert!(result.contains("…"), "expected garbled chars: {result}");
    }

    #[test]
    fn divine_voice_low_awareness_is_noise() {
        let result = filter_divine_voice("go north", 0.15).unwrap();
        assert!(result.contains("noise"), "expected noise tag: {result}");
    }

    // --- build_conversation_prompt ---

    #[test]
    fn conversation_prompt_contains_both_names() {
        let world = test_world();
        let a = make_citizen("Kael", &["curious", "young"], Emotion::Happy, 0.0);
        let b = make_citizen("Elder", &["wise", "cautious"], Emotion::Neutral, 0.0);
        let input = ConversationPromptInput {
            world: &world,
            initiator: &a,
            partner: &b,
            divine_voice: None,
        };
        let prompt = build_conversation_prompt(&input);
        assert!(prompt.contains("Kael"), "missing initiator name");
        assert!(prompt.contains("Elder"), "missing partner name");
    }

    #[test]
    fn conversation_prompt_contains_era_and_setting() {
        let world = test_world();
        let a = make_citizen("A", &[], Emotion::Neutral, 0.0);
        let b = make_citizen("B", &[], Emotion::Neutral, 0.0);
        let input = ConversationPromptInput {
            world: &world,
            initiator: &a,
            partner: &b,
            divine_voice: None,
        };
        let prompt = build_conversation_prompt(&input);
        assert!(prompt.contains("Stone Age"));
        assert!(prompt.contains("river"));
    }

    #[test]
    fn conversation_prompt_contains_personality_tags() {
        let world = test_world();
        let a = make_citizen("A", &["brave", "loyal"], Emotion::Neutral, 0.0);
        let b = make_citizen("B", &["shy"], Emotion::Neutral, 0.0);
        let input = ConversationPromptInput {
            world: &world,
            initiator: &a,
            partner: &b,
            divine_voice: None,
        };
        let prompt = build_conversation_prompt(&input);
        assert!(prompt.contains("brave, loyal"));
        assert!(prompt.contains("shy"));
    }

    #[test]
    fn conversation_prompt_includes_relationship() {
        let world = test_world();
        let mut a = make_citizen("Kael", &[], Emotion::Neutral, 0.0);
        a.relationships.push(Relationship {
            target_name: "Elder".into(),
            familiarity: 0.7,
            trust: 0.8,
        });
        let b = make_citizen("Elder", &[], Emotion::Neutral, 0.0);
        let input = ConversationPromptInput {
            world: &world,
            initiator: &a,
            partner: &b,
            divine_voice: None,
        };
        let prompt = build_conversation_prompt(&input);
        assert!(prompt.contains("familiarity 70%"), "missing familiarity: {prompt}");
        assert!(prompt.contains("trust 80%"), "missing trust: {prompt}");
    }

    #[test]
    fn conversation_prompt_includes_divine_voice_when_aware() {
        let world = test_world();
        let a = make_citizen("A", &[], Emotion::Neutral, 0.95);
        let b = make_citizen("B", &[], Emotion::Neutral, 0.0);
        let input = ConversationPromptInput {
            world: &world,
            initiator: &a,
            partner: &b,
            divine_voice: Some("build a fire"),
        };
        let prompt = build_conversation_prompt(&input);
        assert!(prompt.contains("build a fire"));
    }

    #[test]
    fn conversation_prompt_excludes_divine_voice_when_unaware() {
        let world = test_world();
        let a = make_citizen("A", &[], Emotion::Neutral, 0.0);
        let b = make_citizen("B", &[], Emotion::Neutral, 0.0);
        let input = ConversationPromptInput {
            world: &world,
            initiator: &a,
            partner: &b,
            divine_voice: Some("build a fire"),
        };
        let prompt = build_conversation_prompt(&input);
        assert!(!prompt.contains("build a fire"));
        assert!(!prompt.contains("Divine Voice"));
    }

    #[test]
    fn conversation_prompt_contains_json_instruction() {
        let world = test_world();
        let a = make_citizen("A", &[], Emotion::Neutral, 0.0);
        let b = make_citizen("B", &[], Emotion::Neutral, 0.0);
        let input = ConversationPromptInput {
            world: &world,
            initiator: &a,
            partner: &b,
            divine_voice: None,
        };
        let prompt = build_conversation_prompt(&input);
        assert!(prompt.contains("speech"));
        assert!(prompt.contains("inner_thought"));
        assert!(prompt.contains("YAML"));
    }

    // --- build_divine_voice_prompt ---

    #[test]
    fn divine_prompt_contains_awareness_percentage() {
        let world = test_world();
        let c = make_citizen("Kael", &["curious"], Emotion::Happy, 0.65);
        let input = DivineVoicePromptInput {
            world: &world,
            citizen: &c,
            message: "hello",
        };
        let prompt = build_divine_voice_prompt(&input);
        assert!(prompt.contains("65%"), "missing awareness %: {prompt}");
    }

    // --- build_batch_conversation_prompt ---

    #[test]
    fn batch_prompt_contains_all_pair_names() {
        let world = test_world();
        let a = make_citizen("Kael", &["curious"], Emotion::Happy, 0.0);
        let b = make_citizen("Elder", &["wise"], Emotion::Neutral, 0.0);
        let c = make_citizen("Hunter", &["brave"], Emotion::Angry, 0.0);
        let d = make_citizen("Child", &["shy"], Emotion::Sad, 0.0);
        let input = BatchConversationInput {
            world: &world,
            pairs: vec![(&a, &b), (&c, &d)],
            divine_voice: None,
        };
        let prompt = build_batch_conversation_prompt(&input);
        assert!(prompt.contains("Kael"));
        assert!(prompt.contains("Elder"));
        assert!(prompt.contains("Hunter"));
        assert!(prompt.contains("Child"));
    }

    #[test]
    fn batch_prompt_has_pair_separators() {
        let world = test_world();
        let a = make_citizen("A", &[], Emotion::Neutral, 0.0);
        let b = make_citizen("B", &[], Emotion::Neutral, 0.0);
        let c = make_citizen("C", &[], Emotion::Neutral, 0.0);
        let d = make_citizen("D", &[], Emotion::Neutral, 0.0);
        let input = BatchConversationInput {
            world: &world,
            pairs: vec![(&a, &b), (&c, &d)],
            divine_voice: None,
        };
        let prompt = build_batch_conversation_prompt(&input);
        assert!(prompt.contains("Pair 1"));
        assert!(prompt.contains("Pair 2"));
    }

    #[test]
    fn batch_prompt_requests_json_array() {
        let world = test_world();
        let a = make_citizen("A", &[], Emotion::Neutral, 0.0);
        let b = make_citizen("B", &[], Emotion::Neutral, 0.0);
        let input = BatchConversationInput {
            world: &world,
            pairs: vec![(&a, &b)],
            divine_voice: None,
        };
        let prompt = build_batch_conversation_prompt(&input);
        assert!(prompt.contains("YAML sequence of 1"));
    }

    #[test]
    fn batch_prompt_divine_voice_per_initiator_awareness() {
        let world = test_world();
        let aware = make_citizen("Aware", &[], Emotion::Neutral, 0.95);
        let unaware = make_citizen("Unaware", &[], Emotion::Neutral, 0.0);
        let partner1 = make_citizen("P1", &[], Emotion::Neutral, 0.0);
        let partner2 = make_citizen("P2", &[], Emotion::Neutral, 0.0);
        let input = BatchConversationInput {
            world: &world,
            pairs: vec![(&aware, &partner1), (&unaware, &partner2)],
            divine_voice: Some("build a fire"),
        };
        let prompt = build_batch_conversation_prompt(&input);
        assert!(prompt.contains("Divine Voice for Aware"));
        assert!(!prompt.contains("Divine Voice for Unaware"));
    }

    #[test]
    fn divine_prompt_unaware_citizen_sees_nothing() {
        let world = test_world();
        let c = make_citizen("A", &[], Emotion::Neutral, 0.0);
        let input = DivineVoicePromptInput {
            world: &world,
            citizen: &c,
            message: "secret message",
        };
        let prompt = build_divine_voice_prompt(&input);
        assert!(!prompt.contains("secret message"));
        assert!(prompt.contains("nothing reaches"));
    }
}
