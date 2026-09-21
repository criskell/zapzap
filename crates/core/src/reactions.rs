//! Who reacted with what to which message, so the ui gets one count per emoji (a group message can
//! carry many reactions). Bounded: the oldest message is forgotten first.

/// Messages whose reactions are kept.
const MESSAGES: usize = 64;

/// Kinds of emoji the ui shows for a message.
const SHOWN: usize = 3;

/// The person a reaction of ours is filed under.
pub const ME: &str = "me";

struct Entry {
    chat: usize,
    id: String,
    /// One emoji per person: (person, emoji).
    by: Vec<(String, String)>,
}

#[derive(Default)]
pub struct Reactions {
    entries: Vec<Entry>,
}

impl Reactions {
    /// `who` reacted with `emoji` to message `id` of `chat`; an empty emoji takes their reaction back.
    pub fn set(&mut self, chat: usize, id: &str, who: &str, emoji: &str) {
        let at = match self.entries.iter().position(|entry| entry.chat == chat && entry.id == id) {
            Some(at) => at,
            None => {
                if self.entries.len() == MESSAGES {
                    self.entries.remove(0);
                }
                self.entries.push(Entry { chat, id: id.to_string(), by: Vec::new() });
                self.entries.len() - 1
            }
        };
        let by = &mut self.entries[at].by;
        by.retain(|(person, _)| person != who);
        if !emoji.is_empty() {
            by.push((who.to_string(), emoji.to_string()));
        }
    }

    /// The `REACTIONS` line for a message: our emoji and `emoji:count` pairs, most used first.
    pub fn line(&self, chat: usize, id: &str) -> Vec<String> {
        let by = self.entries.iter().find(|entry| entry.chat == chat && entry.id == id).map(|entry| entry.by.as_slice()).unwrap_or(&[]);
        let mine = by.iter().find(|(person, _)| person == ME).map_or("", |(_, emoji)| emoji.as_str());
        let mut counts: Vec<(&str, usize)> = Vec::new();
        for (_, emoji) in by {
            match counts.iter_mut().find(|(known, _)| known == emoji) {
                Some((_, n)) => *n += 1,
                None => counts.push((emoji, 1)),
            }
        }
        counts.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
        let pairs: Vec<String> = counts.iter().take(SHOWN).map(|(emoji, n)| format!("{emoji}:{}", (*n).min(99))).collect();
        ["REACTIONS".to_string(), chat.to_string(), id.to_string(), mine.to_string(), pairs.join(" ")].to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_by_emoji_most_used_first() {
        let mut reactions = Reactions::default();
        reactions.set(0, "a", "ana", "😂");
        reactions.set(0, "a", "bia", "👍");
        reactions.set(0, "a", "caio", "👍");
        reactions.set(0, "a", ME, "👍");
        assert_eq!(reactions.line(0, "a"), ["REACTIONS", "0", "a", "👍", "👍:3 😂:1"]);
    }

    #[test]
    fn changing_or_taking_back_a_reaction() {
        let mut reactions = Reactions::default();
        reactions.set(1, "b", "ana", "❤");
        reactions.set(1, "b", "ana", "🙏");
        assert_eq!(reactions.line(1, "b"), ["REACTIONS", "1", "b", "", "🙏:1"]);
        reactions.set(1, "b", "ana", "");
        assert_eq!(reactions.line(1, "b"), ["REACTIONS", "1", "b", "", ""]);
    }

    #[test]
    fn forgets_the_oldest_message() {
        let mut reactions = Reactions::default();
        for n in 0..=MESSAGES {
            reactions.set(0, &n.to_string(), "ana", "👍");
        }
        assert_eq!(reactions.line(0, "0")[4], "");
        assert_eq!(reactions.line(0, "1")[4], "👍:1");
    }
}
