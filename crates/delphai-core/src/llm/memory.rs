use serde::{Deserialize, Serialize};

const COMPRESS_THRESHOLD: usize = 64;
const COMPRESS_OLDEST_COUNT: usize = 48;

/// A single memory entry for a citizen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub tick: u64,
    pub content: String,
}

/// Citizen memory buffer with compression support.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryBuffer {
    entries: Vec<MemoryEntry>,
}

/// Result of checking whether compression is needed.
#[derive(Debug, Clone, PartialEq)]
pub enum CompressionCheck {
    /// No compression needed.
    NotNeeded,
    /// These entries should be summarized by the LLM, then passed to `apply_compression`.
    NeedsCompression {
        entries_to_compress: Vec<MemoryEntry>,
    },
}

impl MemoryBuffer {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn push(&mut self, tick: u64, content: String) {
        self.entries.push(MemoryEntry { tick, content });
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entries(&self) -> &[MemoryEntry] {
        &self.entries
    }

    /// Build a text block from all entries, for inclusion in prompts.
    pub fn to_prompt_text(&self) -> String {
        self.entries
            .iter()
            .map(|e| format!("[tick {}] {}", e.tick, e.content))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Check if compression is needed. If so, returns the oldest entries to compress.
    /// The caller should send these to the LLM for summarization, then call `apply_compression`.
    pub fn check_compression(&self) -> CompressionCheck {
        if self.entries.len() <= COMPRESS_THRESHOLD {
            return CompressionCheck::NotNeeded;
        }

        let entries_to_compress = self.entries[..COMPRESS_OLDEST_COUNT].to_vec();
        CompressionCheck::NeedsCompression {
            entries_to_compress,
        }
    }

    /// Apply compression: replace the oldest N entries with a single summary entry.
    /// `summary_tick` should be the tick of the newest compressed entry.
    pub fn apply_compression(&mut self, summary: String) {
        if self.entries.len() <= COMPRESS_OLDEST_COUNT {
            return;
        }

        let summary_tick = self.entries[COMPRESS_OLDEST_COUNT - 1].tick;
        let remaining = self.entries.split_off(COMPRESS_OLDEST_COUNT);
        self.entries.clear();
        self.entries.push(MemoryEntry {
            tick: summary_tick,
            content: format!("[summary] {summary}"),
        });
        self.entries.extend(remaining);
    }

    /// Build a prompt for the LLM to summarize the given entries.
    pub fn build_compression_prompt(entries: &[MemoryEntry]) -> String {
        let mut parts = Vec::new();
        parts.push(
            "Summarize the following memory entries into a single concise paragraph. \
             Preserve key facts, relationships, and emotional events. Respond with just the summary text."
                .to_string(),
        );
        parts.push(String::new());
        for e in entries {
            parts.push(format!("[tick {}] {}", e.tick, e.content));
        }
        parts.join("\n")
    }
}

impl Default for MemoryBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fill_buffer(n: usize) -> MemoryBuffer {
        let mut buf = MemoryBuffer::new();
        for i in 0..n {
            buf.push(i as u64, format!("event {i}"));
        }
        buf
    }

    #[test]
    fn new_buffer_is_empty() {
        let buf = MemoryBuffer::new();
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn push_increases_len() {
        let mut buf = MemoryBuffer::new();
        buf.push(1, "hello".into());
        assert_eq!(buf.len(), 1);
    }

    #[test]
    fn entries_are_accessible() {
        let mut buf = MemoryBuffer::new();
        buf.push(5, "found fire".into());
        assert_eq!(buf.entries()[0].tick, 5);
        assert_eq!(buf.entries()[0].content, "found fire");
    }

    #[test]
    fn to_prompt_text_formats_correctly() {
        let mut buf = MemoryBuffer::new();
        buf.push(1, "saw a deer".into());
        buf.push(3, "talked to Elder".into());
        let text = buf.to_prompt_text();
        assert!(text.contains("[tick 1] saw a deer"));
        assert!(text.contains("[tick 3] talked to Elder"));
    }

    #[test]
    fn no_compression_below_threshold() {
        let buf = fill_buffer(64);
        assert_eq!(buf.check_compression(), CompressionCheck::NotNeeded);
    }

    #[test]
    fn compression_needed_above_threshold() {
        let buf = fill_buffer(65);
        match buf.check_compression() {
            CompressionCheck::NeedsCompression {
                entries_to_compress,
            } => {
                assert_eq!(entries_to_compress.len(), COMPRESS_OLDEST_COUNT);
                assert_eq!(entries_to_compress[0].content, "event 0");
                assert_eq!(
                    entries_to_compress.last().unwrap().content,
                    format!("event {}", COMPRESS_OLDEST_COUNT - 1)
                );
            }
            CompressionCheck::NotNeeded => panic!("expected compression needed"),
        }
    }

    #[test]
    fn apply_compression_replaces_oldest_with_summary() {
        let mut buf = fill_buffer(70);
        buf.apply_compression("a lot happened".into());

        // 1 summary + (70 - 48) remaining = 23
        assert_eq!(buf.len(), 23);
        assert!(buf.entries()[0].content.contains("[summary]"));
        assert!(buf.entries()[0].content.contains("a lot happened"));
        // summary tick = tick of the 48th entry (index 47)
        assert_eq!(buf.entries()[0].tick, 47);
        // remaining entries start from event 48
        assert_eq!(buf.entries()[1].content, "event 48");
    }

    #[test]
    fn apply_compression_noop_when_too_few() {
        let mut buf = fill_buffer(10);
        buf.apply_compression("should not change".into());
        assert_eq!(buf.len(), 10);
    }

    #[test]
    fn build_compression_prompt_includes_entries() {
        let entries = vec![
            MemoryEntry {
                tick: 1,
                content: "found berries".into(),
            },
            MemoryEntry {
                tick: 5,
                content: "argued with Hunter".into(),
            },
        ];
        let prompt = MemoryBuffer::build_compression_prompt(&entries);
        assert!(prompt.contains("Summarize"));
        assert!(prompt.contains("found berries"));
        assert!(prompt.contains("argued with Hunter"));
    }

    #[test]
    fn double_compression() {
        // After first compression, push more until threshold again
        let mut buf = fill_buffer(70);
        buf.apply_compression("first summary".into());
        // Now 23 entries. Push 42 more to hit 65.
        for i in 70..(70 + 42) {
            buf.push(i as u64, format!("event {i}"));
        }
        assert_eq!(buf.len(), 65);

        match buf.check_compression() {
            CompressionCheck::NeedsCompression { entries_to_compress } => {
                assert_eq!(entries_to_compress.len(), COMPRESS_OLDEST_COUNT);
            }
            CompressionCheck::NotNeeded => panic!("expected compression needed"),
        }
    }

    #[test]
    fn serde_roundtrip() {
        let mut buf = fill_buffer(5);
        buf.push(10, "special event".into());
        let json = serde_json::to_string(&buf).unwrap();
        let restored: MemoryBuffer = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.len(), 6);
        assert_eq!(restored.entries()[5].content, "special event");
    }
}
