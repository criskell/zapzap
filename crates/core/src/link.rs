//! The line protocol between this process and the ui. Fields are separated by tabs and the last
//! one is free text. The ui documents what it accepts in `handle_line`; what it sends is `SEND`.

use std::io::{BufRead, Write};
use std::sync::mpsc::{self, Receiver};
use std::time::{SystemTime, UNIX_EPOCH};

/// Brazil is one time zone for practical purposes: Brasilia time, UTC-3, no daylight saving.
const BRASILIA_OFFSET_SECONDS: i64 = -3 * 3600;

pub enum Command {
    Send { chat: usize, text: String },
    /// Delete for everyone the message of ours with this id.
    Delete { chat: usize, id: String },
    /// An emoji reaction on the message with this id; an empty emoji takes the reaction back.
    React { chat: usize, id: String, emoji: String },
    Star { chat: usize, id: String, starred: bool },
    MarkRead { chat: usize },
    Older { chat: usize, id: String },
    Thumb { chat: usize, id: String },
    Edit { chat: usize, id: String, text: String },
    Mute { chat: usize, muted: bool },
    ClearChat { chat: usize },
    DeleteChat { chat: usize },
    /// A text that answers the message with this id.
    Reply { chat: usize, id: String, text: String },
    /// The ui placed a voice call to `name`.
    Dial { name: String },
    Answer,
    Hangup,
    /// The ui shows the new-conversation panel and wants the contacts that match `query`.
    Contacts { query: String },
    /// The ui picked a contact that has no conversation yet.
    OpenChat { name: String },
}

/// One line for the ui. When the ui has closed the pipe there is nobody to serve: exit.
pub fn say(fields: &[&str]) {
    let mut out = std::io::stdout().lock();
    if writeln!(out, "{}", fields.join("\t")).and_then(|()| out.flush()).is_err() {
        std::process::exit(0);
    }
}

/// Commands typed in the ui, read on a thread of their own so the connection never waits for them.
pub fn commands() -> Receiver<Command> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines().map_while(|line| line.ok()) {
            if let Some(command) = parse_command(&line) {
                if sender.send(command).is_err() {
                    return;
                }
            }
        }
    });
    receiver
}

fn parse_command(line: &str) -> Option<Command> {
    if let Some(rest) = line.strip_prefix("DELETE\t") {
        let mut fields = rest.splitn(2, '\t');
        return Some(Command::Delete { chat: fields.next()?.parse().ok()?, id: fields.next()?.to_string() });
    }
    if let Some(rest) = line.strip_prefix("REACT\t") {
        let mut fields = rest.splitn(3, '\t');
        return Some(Command::React { chat: fields.next()?.parse().ok()?, id: fields.next()?.to_string(), emoji: fields.next()?.to_string() });
    }
    if let Some(rest) = line.strip_prefix("THUMB\t") {
        let mut fields = rest.splitn(2, '\t');
        return Some(Command::Thumb { chat: fields.next()?.parse().ok()?, id: fields.next()?.to_string() });
    }
    if let Some(rest) = line.strip_prefix("OLDER\t") {
        let mut fields = rest.splitn(2, '\t');
        return Some(Command::Older { chat: fields.next()?.parse().ok()?, id: fields.next()?.to_string() });
    }
    if let Some(rest) = line.strip_prefix("EDIT\t") {
        let mut fields = rest.splitn(3, '\t');
        return Some(Command::Edit { chat: fields.next()?.parse().ok()?, id: fields.next()?.to_string(), text: fields.next()?.to_string() });
    }
    if let Some(rest) = line.strip_prefix("MUTE\t") {
        let mut fields = rest.splitn(2, '\t');
        return Some(Command::Mute { chat: fields.next()?.parse().ok()?, muted: fields.next()? == "1" });
    }
    if let Some(rest) = line.strip_prefix("DELETECHAT\t") {
        return Some(Command::DeleteChat { chat: rest.parse().ok()? });
    }
    if let Some(rest) = line.strip_prefix("CLEARCHAT\t") {
        return Some(Command::ClearChat { chat: rest.parse().ok()? });
    }
    if let Some(rest) = line.strip_prefix("MARKREAD\t") {
        return Some(Command::MarkRead { chat: rest.parse().ok()? });
    }
    if let Some(rest) = line.strip_prefix("STAR\t") {
        let mut fields = rest.splitn(3, '\t');
        return Some(Command::Star { chat: fields.next()?.parse().ok()?, id: fields.next()?.to_string(), starred: fields.next()? == "1" });
    }
    if let Some(rest) = line.strip_prefix("REPLY\t") {
        let mut fields = rest.splitn(3, '\t');
        return Some(Command::Reply { chat: fields.next()?.parse().ok()?, id: fields.next()?.to_string(), text: fields.next()?.to_string() });
    }
    let mut fields = line.splitn(3, '\t');
    match (fields.next()?, fields.next(), fields.next()) {
        ("SEND", Some(chat), Some(text)) => Some(Command::Send { chat: chat.parse().ok()?, text: text.to_string() }),
        ("DIAL", Some(name), None) => Some(Command::Dial { name: name.to_string() }),
        ("ANSWER", None, None) => Some(Command::Answer),
        ("HANGUP", None, None) => Some(Command::Hangup),
        ("OPENCHAT", Some(name), None) => Some(Command::OpenChat { name: name.to_string() }),
        ("CONTACTS", Some(query), None) => Some(Command::Contacts { query: query.to_string() }),
        _ => None,
    }
}

pub fn clock_at(unix_seconds: i64) -> String {
    let day = (unix_seconds + BRASILIA_OFFSET_SECONDS).rem_euclid(86_400);
    format!("{:02}:{:02}", day / 3600, day % 3600 / 60)
}

/// The `time` field of `MSG` and `PAST`: the time of day and, after a space, the calendar day (days
/// since 1970-01-01, Brasilia time), which the ui turns into the chip above the day's messages.
pub fn stamp_at(unix_seconds: i64) -> String {
    format!("{} {}", clock_at(unix_seconds), (unix_seconds + BRASILIA_OFFSET_SECONDS).div_euclid(86_400))
}

pub fn clock() -> String {
    clock_at(SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |elapsed| elapsed.as_secs() as i64))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_send_line_carries_the_chat_and_the_whole_text() {
        let Some(Command::Send { chat, text }) = parse_command("SEND\t3\toi\tcom tab") else { panic!("not a command") };
        assert_eq!((chat, text.as_str()), (3, "oi\tcom tab"));
    }

    #[test]
    fn lines_that_are_not_commands_are_ignored() {
        assert!(parse_command("SEND\tx\toi").is_none());
        assert!(parse_command("SEND\t1").is_none());
        assert!(parse_command("HELLO\t1\toi").is_none());
        assert!(parse_command("DIAL").is_none());
        assert!(parse_command("").is_none());
    }

    #[test]
    fn call_lines_are_commands() {
        assert!(matches!(parse_command("DIAL\tMãe"), Some(Command::Dial { name }) if name == "Mãe"));
        assert!(matches!(parse_command("ANSWER"), Some(Command::Answer)));
        assert!(matches!(parse_command("HANGUP"), Some(Command::Hangup)));
    }

    #[test]
    fn a_reply_line_carries_the_chat_the_id_and_the_whole_text() {
        let Some(Command::Reply { chat, id, text }) = parse_command("REPLY\t2\tABC123\tsim\tcom tab") else { panic!("not a reply") };
        assert_eq!((chat, id.as_str(), text.as_str()), (2, "ABC123", "sim\tcom tab"));
        assert!(parse_command("REPLY\t2\tABC123").is_none());
    }

    #[test]
    fn a_delete_line_carries_the_chat_and_the_id() {
        assert!(matches!(parse_command("DELETE\t3\tXYZ"), Some(Command::Delete { chat: 3, id }) if id == "XYZ"));
        assert!(parse_command("DELETE\t3").is_none());
    }

    #[test]
    fn a_react_line_may_carry_an_empty_emoji_to_take_the_reaction_back() {
        assert!(matches!(parse_command("REACT\t01\tABC\t👍"), Some(Command::React { chat: 1, id, emoji }) if id == "ABC" && emoji == "👍"));
        assert!(matches!(parse_command("REACT\t2\tABC\t"), Some(Command::React { emoji, .. }) if emoji.is_empty()));
        assert!(matches!(parse_command("THUMB\t02\tABC"), Some(Command::Thumb { chat: 2, id }) if id == "ABC"));
        assert!(matches!(parse_command("OLDER\t03\tABC"), Some(Command::Older { chat: 3, id }) if id == "ABC"));
        assert!(matches!(parse_command("EDIT\t01\tABC\tnovo texto"), Some(Command::Edit { chat: 1, id, text }) if id == "ABC" && text == "novo texto"));
        assert!(matches!(parse_command("MUTE\t02\t1"), Some(Command::Mute { chat: 2, muted: true })));
        assert!(matches!(parse_command("DELETECHAT\t05"), Some(Command::DeleteChat { chat: 5 })));
        assert!(matches!(parse_command("CLEARCHAT\t04"), Some(Command::ClearChat { chat: 4 })));
        assert!(matches!(parse_command("MARKREAD\t03"), Some(Command::MarkRead { chat: 3 })));
        assert!(matches!(parse_command("STAR\t01\tABC\t1"), Some(Command::Star { chat: 1, id, starred: true }) if id == "ABC"));
        assert!(matches!(parse_command("STAR\t01\tABC\t0"), Some(Command::Star { starred: false, .. })));
        assert!(parse_command("REACT\t2\tABC").is_none());
    }

    #[test]
    fn a_contacts_line_carries_the_query_even_when_empty() {
        assert!(matches!(parse_command("CONTACTS\t"), Some(Command::Contacts { query }) if query.is_empty()));
        assert!(matches!(parse_command("CONTACTS\tsil"), Some(Command::Contacts { query }) if query == "sil"));
    }

    #[test]
    fn an_openchat_line_carries_the_contact_name() {
        assert!(matches!(parse_command("OPENCHAT\tMãe"), Some(Command::OpenChat { name }) if name == "Mãe"));
    }

    #[test]
    fn the_clock_shows_brasilia_time() {
        assert_eq!(clock_at(0), "21:00");
        assert_eq!(stamp_at(0), "21:00 -1");
        assert_eq!(stamp_at(3 * 3600), "00:00 0");
        assert_eq!(clock_at(15 * 3600 + 30 * 60), "12:30");
        assert_eq!(clock_at(3 * 3600), "00:00");
    }
}

/// What the ui is told when it asks for contacts: it forgets its list, then gets the matches.
pub fn contacts_reply(names: &[&str]) -> Vec<Vec<String>> {
    let mut lines = vec![vec!["CONTACTRESET".to_string()]];
    lines.extend(names.iter().map(|name| vec!["CONTACT".to_string(), name.to_string()]));
    lines
}
