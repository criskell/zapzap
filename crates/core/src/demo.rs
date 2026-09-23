//! Stand-in conversations for developing the ui without a paired account, and a contact who
//! answers: `zapzap-core --demo`.

use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, Instant};

use crate::audio;
use crate::call::{Audio, Calls, Event, Outcome};
use crate::contacts::ContactBook;
use crate::link::{clock, commands, contacts_reply, say, stamp_at, Command};
use crate::trim;

const DELIVERED_AFTER: Duration = Duration::from_millis(500);
const READ_AFTER: Duration = Duration::from_millis(1200);
const REPLY_AFTER: Duration = Duration::from_millis(2600);
/// How many contacts the ui keeps at a time.
const UI_CONTACTS: usize = 40;
const RING_FOR: Duration = Duration::from_millis(2500);
const NO_ANSWER_AFTER: Duration = Duration::from_secs(8);
const MISSED_AFTER: Duration = Duration::from_secs(25);
const INCOMING_AFTER: Duration = Duration::from_secs(20);
const IDLE: Duration = Duration::from_secs(3600);

/// (name, status, unread)
const CHATS: [(&str, &str, u8); 8] = [
    ("Ana", "online", 0),
    ("Família", "Mãe, Pai, Julia, você", 3),
    ("Trabalho", "8 participantes", 12),
    ("Bruno", "visto por último hoje às 09:48", 0),
    ("Grupo da faculdade", "42 participantes", 0),
    ("Carlos", "online", 0),
    ("Mercado", "conta comercial", 1),
    ("Julia", "visto por último sexta-feira", 0),
];

/// (message, the message it answers): history entries that quote an earlier one.
const QUOTED: [(usize, usize); 2] = [(12, 11), (14, 13)];

/// (history entry, kind, width, height, label): entries that carry a photo, video, voice or document.
/// Kinds are the ones of the `MEDIA` line: 1 photo, 2 video, 3 voice, 4 document.
const MEDIA: [(usize, u8, u32, u32, &str); 4] = [(30, 1, 1600, 1200, ""), (31, 2, 1280, 720, "0:24"), (32, 3, 0, 0, "0:12"), (33, 4, 0, 0, "Cardápio da semana.pdf")];

/// (conversation, sent by me, text, time)
const HISTORY: [(usize, bool, &str, &str); 34] = [
    (0, false, "Oi! Vamos almoçar hoje?", "12:31"),
    (0, true, "Bora! Onde você quer ir?", "12:33"),
    (0, false, "Aquele restaurante perto do escritório, o de comida por quilo que abriu semana passada.", "12:35"),
    (0, true, "Fechado. Chego lá em 20 minutos.", "12:36"),
    (0, false, "Perfeito! Vou pedir uma mesa perto da janela.", "12:37"),
    (0, false, "Ah, chama o Bruno também?", "12:37"),
    (0, true, "Chamei, mas ele disse que já tinha marcado reunião com o cliente às 13h.", "12:39"),
    (0, false, "Que pena! Fica pra próxima então.", "12:40"),
    (0, true, "Combinado. Já estou saindo do prédio, me espera na entrada?", "12:41"),
    (0, false, "Tô te esperando aqui na frente.", "12:42"),
    (0, false, "Aliás, o cardápio de hoje tem feijoada e uma opção vegetariana que parece ótima.", "12:42"),
    (0, true, "Feijoada com certeza. Preciso de energia pra tarde inteira de planilha.", "12:44"),
    (0, false, "Kkkkk verdade, a sua semana está pesada 😂😂", "12:45"),
    (0, true, "Cheguei! Estou de camisa azul 🙂", "12:52"),
    (0, false, "Te vi! Pode vir. 😄", "12:53"),
    (1, false, "Filho, já saímos da estrada.", "10:40"),
    (1, false, "Trânsito tranquilo, deve dar umas duas horas ainda.", "10:41"),
    (1, true, "Boa viagem, avisa quando chegar!", "10:50"),
    (1, false, "Chegamos bem ❤️", "11:02"),
    (2, false, "Pessoal, o deploy de hoje vai sair depois do almoço.", "09:30"),
    (2, true, "Beleza. Os testes de regressão passaram todos.", "09:41"),
    (2, false, "Deploy saiu 🚀", "10:15"),
    (3, true, "Consegue me mandar o relatório do trimestre?", "09:40"),
    (3, false, "Mandei o arquivo. Vai Brasil 🇧🇷!", "09:48"),
    (4, false, "Pessoal, a prova de sexta foi adiada.", "Ontem"),
    (4, false, "Prova adiada 🎉", "Ontem"),
    (5, false, "Valeu! 👍🏽", "Ontem"),
    (6, false, "Seu pedido saiu para entrega", "Sáb"),
    (7, false, "Kkkkk 🤣", "Sex"),
    (7, true, "Verdade!", "Sex"),
    (1, false, "Olha como ficou a mesa", "11:01"),
    (1, false, "", "11:01"),
    (1, true, "", "11:02"),
    (1, false, "", "11:03"),
];

const FAVORITES: [&str; 3] = ["Mãe", "Bruno", "Grupo da faculdade"];

/// (kind: 0 received, 1 missed, 2 outgoing, count, time, name)
const CALLS: [(u8, u8, &str, &str); 8] = [
    (0, 1, "10:12", "Mãe"),
    (0, 2, "Ontem", "Mãe"),
    (1, 1, "sexta-feira", "Bruno"),
    (0, 1, "quinta-feira", "Ana"),
    (2, 1, "quinta-feira", "Carlos"),
    (1, 3, "quarta-feira", "Trabalho"),
    (0, 1, "terça-feira", "Julia"),
    (2, 1, "segunda-feira", "Mãe"),
];

/// (segments, unseen, time, name)
const STORIES: [(u8, u8, &str, &str); 3] = [(4, 1, "Hoje às 10:40", "Tia Darc"), (3, 0, "Ontem às 21:53", "Tio Raul"), (2, 0, "Ontem às 12:19", "Alexandre")];

/// (unread, time, name, preview)
const CHANNELS: [(u16, &str, &str, &str); 3] = [
    (208, "10:53", "Memes Melted Videos", "Foco no STF pt2 👀"),
    (71, "10:13", "Pastor Antônio Júnior", "Receba esta oração e tenha um dia abençoado."),
    (61, "sexta-feira", "Netflix Brasil", "E AGORA? Você só pode assistir uma série a vida inteira."),
];

/// (verified, followers, name)
const SUGGESTIONS: [(u8, &str, &str); 5] = [
    (1, "14 M seguidores", "Pack de Figurinhas"),
    (1, "120 mil seguidores", "Roblox Brasil"),
    (0, "77 mil seguidores", "MINECRAFT ADDONS"),
    (0, "7,9 mil seguidores", "Momento De Oração"),
    (1, "1 M seguidores", "Rafa & Luiz"),
];

/// Contacts of the new-conversation panel, in alphabetical order with the numbers first.
const CONTACTS: [&str; 14] = [
    "996985554", "Acor", "Airton", "Airton Vieira", "Ana", "Bruno", "Carlos", "Família", "Grupo da faculdade", "Julia", "Mãe", "Mercado", "Osmar", "Trabalho",
];

fn opening_lines() -> Vec<Vec<String>> {
    let mut lines = vec![vec!["RESET".to_string()]];
    for (id, (name, status, unread)) in CHATS.iter().enumerate() {
        let time = HISTORY.iter().rev().find(|message| message.0 == id).map_or("", |message| message.3);
        lines.push(["CHAT", &id.to_string(), &unread.to_string(), time, name, status].map(String::from).to_vec());
    }
    for (number, (chat, outgoing, text, time)) in HISTORY.into_iter().enumerate() {
        let (outgoing, read) = if outgoing { ("1", "1") } else { ("0", "0") };
        let reply = QUOTED.iter().find(|(message, _)| *message == number).map_or(String::new(), |(_, quoted)| format!("d{quoted}"));
        lines.push(["MSG", &chat.to_string(), outgoing, read, time, &format!("d{number}"), &reply, text].map(String::from).to_vec());
        if let Some(&(_, kind, width, height, label)) = MEDIA.iter().find(|entry| entry.0 == number) {
            lines.push(["MEDIA", &chat.to_string(), &format!("d{number}"), &kind.to_string(), &width.to_string(), &height.to_string(), label].map(String::from).to_vec());
        }
    }
    // Reactions to show: one person's on something I said, mine plus another's on something they said.
    lines.push(["REACTIONS", "0", "d1", "", "👍:1"].map(String::from).to_vec());
    lines.push(["REACTIONS", "0", "d0", "❤", "❤:2 😂:1"].map(String::from).to_vec());
    lines.push(["STARRED", "0", "d2", "1"].map(String::from).to_vec());
    for name in FAVORITES {
        lines.push(["FAVORITE", name].map(String::from).to_vec());
    }
    for (kind, count, time, name) in CALLS {
        lines.push(["CALL", &kind.to_string(), &count.to_string(), time, name].map(String::from).to_vec());
    }
    lines.push(["PROFILE", "zapzap"].map(String::from).to_vec());
    lines.push(["MYSTORY", "Ontem às 13:11"].map(String::from).to_vec());
    for (segments, unseen, time, name) in STORIES {
        lines.push(["STORY", &segments.to_string(), &unseen.to_string(), time, name].map(String::from).to_vec());
    }
    for (unread, time, name, preview) in CHANNELS {
        lines.push(["CHANNEL", &unread.to_string(), time, name, preview].map(String::from).to_vec());
    }
    for (verified, followers, name) in SUGGESTIONS {
        lines.push(["SUGGEST", &verified.to_string(), followers, name].map(String::from).to_vec());
    }
    lines.push(vec!["OPEN".to_string()]);
    lines
}

fn send(line: &[String]) {
    say(&line.iter().map(String::as_str).collect::<Vec<_>>());
}

/// Something the demo does later: a line for the ui, or something that happens to the call.
enum Due {
    Line(Vec<String>),
    Call(Event),
}

/// How the pretend contacts react to a call from the ui.
fn peer_reaction(name: &str) -> (Duration, Event) {
    match name {
        "Mãe" => (RING_FOR, Event::PeerAnswered),
        "Bruno" => (RING_FOR, Event::PeerEnded("Ocupado".to_string())),
        _ => (NO_ANSWER_AFTER, Event::RingTimeout),
    }
}

fn play(out: Outcome) {
    for line in out.lines {
        send(&line);
    }
}

/// Plays the demo until the ui goes away: shows the conversations, and for every message typed
/// in the ui marks it read a moment later and answers. Calls go through the same state machine as
/// the real connection will use; the contacts answer, are busy or ignore the call by name.
pub fn run() {
    for line in opening_lines() {
        send(&line);
    }
    let commands = commands();
    let mut calls = Calls::new();
    let mut extra_chats: Vec<String> = Vec::new();
    // How many pages of older messages each conversation has given (the demo has two to give).
    let mut older_pages = [0u8; 64];
    let mut sent = 0;
    let mut book = ContactBook::new();
    for name in CONTACTS {
        book.insert(name);
    }
    let audio = || Audio { microphone: audio::microphone_available(), speaker: audio::speaker_available() };
    let mut due: Vec<(Instant, Due)> = Vec::new();
    due.push((Instant::now() + INCOMING_AFTER, Due::Call(Event::Incoming("Mãe".to_string()))));
    loop {
        trim::drop_idle_code();
        let wait = due.iter().map(|(at, _)| at.saturating_duration_since(Instant::now())).min().unwrap_or(IDLE);
        match commands.recv_timeout(wait) {
            Ok(command @ (Command::Send { .. } | Command::Reply { .. })) => {
                let (chat, text, quoted) = match command {
                    Command::Send { chat, text } => (chat, text, String::new()),
                    Command::Reply { chat, id, text } => (chat, text, id),
                    _ => unreachable!(),
                };
                eprintln!("[demo] sent to chat {chat}: {text}");
                sent += 1;
                send(&["SENT".to_string(), chat.to_string(), format!("s{sent}")]);
                let now = Instant::now();
                due.push((now + DELIVERED_AFTER, Due::Line(vec!["DELIVERED".to_string(), chat.to_string()])));
                due.push((now + READ_AFTER, Due::Line(vec!["READ".to_string(), chat.to_string()])));
                due.push((now + REPLY_AFTER - Duration::from_millis(1500), Due::Line(["PRESENCE", &chat.to_string(), "digitando…"].map(String::from).to_vec())));
                due.push((now + REPLY_AFTER + Duration::from_millis(300), Due::Line(["PRESENCE", &chat.to_string(), "online"].map(String::from).to_vec())));
                let reply = "Recebido! (resposta automática do modo de demonstração)";
                // An answer to an answer quotes it back.
                due.push((now + REPLY_AFTER, Due::Line(["MSG", &chat.to_string(), "0", "0", &clock(), &format!("r{sent}"), &quoted, reply].map(String::from).to_vec())));
            }
            Ok(Command::Dial { name }) => {
                eprintln!("[demo] calling {name}");
                let was_idle = calls.is_idle();
                let out = calls.handle(Event::Dial(name.clone()), &audio());
                if was_idle && calls.peer().is_some() {
                    let (after, event) = peer_reaction(&name);
                    due.push((Instant::now() + after, Due::Call(event)));
                }
                play(out);
            }
            Ok(Command::Contacts { query }) => {
                let found = book.search(&query, UI_CONTACTS);
                for line in contacts_reply(&found) {
                    send(&line);
                }
            }
            Ok(Command::Delete { chat, id }) => eprintln!("[demo] deleted {id} in chat {chat}"),
            Ok(Command::Thumb { chat, id }) => {
                // A soft diagonal gradient, warm in one corner: a stand-in for the preview picture.
                let (width, height) = (48usize, 36usize);
                send(&["THUMB".to_string(), chat.to_string(), id.clone(), width.to_string(), height.to_string()]);
                for row in 0..height {
                    let hex: String = (0..width)
                        .map(|column| {
                            let (r, g, b) = (90 + column * 3, 70 + row * 3 + column, 160 - row * 2);
                            let packed = ((r.min(255) >> 3) << 11 | (g.min(255) >> 2) << 5 | (b.min(255) >> 3)) as u16;
                            format!("{:02x}{:02x}", packed & 0xff, packed >> 8)
                        })
                        .collect();
                    send(&["THUMBROW".to_string(), chat.to_string(), id.clone(), row.to_string(), hex]);
                }
            }
            Ok(Command::Older { chat, .. }) => {
                let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |elapsed| elapsed.as_secs() as i64);
                let page = older_pages.get_mut(chat).map_or(2, |pages| std::mem::replace(pages, pages.saturating_add(1)));
                if page < 2 {
                    // Newest first: each goes before the conversation's first message.
                    for n in (0..6).rev() {
                        let number = page as usize * 6 + n;
                        let outgoing = if number % 3 == 0 { "1" } else { "0" };
                        send(&["PAST".to_string(), chat.to_string(), outgoing.to_string(), "1".to_string(), stamp_at(now - (page as i64 * 7 + 2) * 86_400), format!("old{chat}_{number}"), String::new(), format!("Mensagem antiga {}", number + 1)]);
                    }
                }
                send(&["PASTEND".to_string(), chat.to_string(), if page < 1 { "1" } else { "0" }.to_string()]);
            }
            Ok(Command::Edit { chat, id, text }) => eprintln!("[demo] {id} in chat {chat} edited to {text:?}"),
            Ok(Command::Mute { chat, muted }) => eprintln!("[demo] chat {chat} muted: {muted}"),
            Ok(Command::DeleteChat { chat }) => eprintln!("[demo] chat {chat} deleted"),
            Ok(Command::ClearChat { chat }) => eprintln!("[demo] chat {chat} cleared"),
            Ok(Command::MarkRead { chat }) => eprintln!("[demo] chat {chat} read"),
            Ok(Command::Star { chat, id, starred }) => eprintln!("[demo] {} {id} in chat {chat}", if starred { "starred" } else { "unstarred" }),
            Ok(Command::React { chat, id, emoji }) => eprintln!("[demo] reaction {emoji:?} on {id} in chat {chat}"),
            Ok(Command::OpenChat { name }) => {
                let name = if name.is_empty() { "Você".to_string() } else { name };
                // A contact without a conversation gets an empty one, after the eight that exist.
                let id = CHATS.len() + extra_chats.iter().position(|known| *known == name).unwrap_or_else(|| {
                    extra_chats.push(name.clone());
                    extra_chats.len() - 1
                });
                send(&["CHAT", &id.to_string(), "0", "", &name, ""].map(String::from));
                send(&["SHOWCHAT", &id.to_string()].map(String::from));
            }
            Ok(command @ (Command::Answer | Command::Hangup)) => {
                let event = if matches!(command, Command::Answer) { Event::Answer } else { Event::Hangup };
                // Whatever the other side had scheduled for this call is moot once we act on it.
                due.retain(|(_, item)| !matches!(item, Due::Call(Event::PeerAnswered | Event::PeerEnded(_) | Event::RingTimeout)));
                play(calls.handle(event, &audio()));
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        let now = Instant::now();
        let (ready, waiting): (Vec<_>, Vec<_>) = due.into_iter().partition(|(at, _)| *at <= now);
        due = waiting;
        for (_, item) in ready {
            match item {
                Due::Line(line) => send(&line),
                Due::Call(event) => {
                    let ringing_in = matches!(event, Event::Incoming(_));
                    let out = calls.handle(event, &audio());
                    if ringing_in && out.reject_busy {
                        eprintln!("[demo] a call came in while another was going: turned away as busy");
                    } else if ringing_in {
                        // Nobody answers: the caller gives up.
                        due.push((now + MISSED_AFTER, Due::Call(Event::PeerEnded(String::new()))));
                    }
                    play(out);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ui stores these in fixed arrays; anything larger would be cut.
    const UI_MAX_CHATS: usize = 48;
    const UI_MAX_MESSAGES: usize = 384;
    const UI_NAME: usize = 40;
    const UI_STATUS: usize = 48;
    const UI_MESSAGE: usize = 240;

    #[test]
    fn the_demo_fits_the_limits_of_the_ui() {
        assert!(CHATS.len() <= UI_MAX_CHATS && HISTORY.len() <= UI_MAX_MESSAGES);
        assert!(CHATS.iter().all(|(name, status, _)| name.len() <= UI_NAME && status.len() <= UI_STATUS));
        assert!(HISTORY.iter().all(|(chat, _, text, _)| *chat < CHATS.len() && text.len() <= UI_MESSAGE));
    }

    #[test]
    fn every_conversation_has_a_message_and_the_screen_opens_last() {
        assert!((0..CHATS.len()).all(|id| HISTORY.iter().any(|message| message.0 == id)));
        let lines = opening_lines();
        assert_eq!(lines.first().unwrap()[0], "RESET");
        assert_eq!(lines.last().unwrap()[0], "OPEN");
        assert_eq!(lines.iter().filter(|line| line[0] == "CHAT").count(), CHATS.len());
        assert!(lines.iter().all(|line| line[0] != "CONTACT"), "contacts are sent on request");
        assert_eq!(lines.iter().filter(|line| line[0] == "MSG").count(), HISTORY.len());
    }

    #[test]
    fn no_field_can_break_the_line_format() {
        let forbidden = |text: &str| text.contains('\t') || text.contains('\n');
        assert!(opening_lines().iter().flatten().all(|field| !forbidden(field)));
    }
}
