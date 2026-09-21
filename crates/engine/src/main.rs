//! The real connection to WhatsApp, on the `whatsapp-rust` client (Signal sessions, message
//! decryption, history, calls) instead of a hand-made one. It speaks the same line protocol to the ui
//! as `zapzap-core --demo` (see the ui's `handle_line`), so the ui cannot tell them apart.
//!
//! What reaches the ui today: the pairing QR, the connection state, incoming messages and voice-call
//! rings. What the ui can ask for: send a text, decline a call. Answering a call needs the media
//! plane (relay transport, audio devices), which is not wired: the caller is told the call was declined.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;
use whatsapp_rust::prelude::*;
use whatsapp_rust::wacore::proto_helpers::{build_quote_context_with_info, MessageBuilderExt, MessageExt};
use whatsapp_rust::wacore_binary::JidExt;
use whatsapp_rust::wacore::types::call::{CallAction, IncomingCall};
use whatsapp_rust::wacore::types::events::EventKind;
use zapzap_core::audio;
use zapzap_core::call::{Audio, Calls, Event as CallEvent, Lines};
use zapzap_core::contacts::ContactBook;
use zapzap_core::reactions::{Reactions, ME};
use zapzap_core::link::{clock_at, commands, contacts_reply, say, Command};
use zapzap_core::trim;

/// SQLite's page cache, per connection (its default is 512).
const SQLITE_CACHE_KIB: u32 = 64;

/// How many contacts the ui keeps at a time.
const UI_CONTACTS: usize = 40;

/// Messages of a history conversation the ui shows.
const MESSAGES_PER_CHAT: usize = 8;

/// The ui keeps this many conversations.
const MAX_CHATS: usize = 16;

/// What the engine remembers between events.
struct State {
    calls: Calls,
    /// The voice call that is ringing, so the ui's "Recusar" knows what to reject.
    ringing: Option<IncomingCall>,
    /// The ui refers to conversations by number: the position of their JID in this list.
    chats: Vec<(Jid, String)>,
    /// Recent messages the ui may answer: what an answer has to quote.
    recent: Vec<Quotable>,
    /// Every saved contact's name; the ui asks for the ones that match what was typed.
    book: ContactBook,
    /// Who reacted with what, to tell the ui a count per emoji.
    reactions: Reactions,
    /// Chats the ui dropped: a new message brings them back.
    removed: Vec<usize>,
}

/// A message kept so that an answer to it can quote it.
struct Quotable {
    chat: Jid,
    id: String,
    sender: Jid,
    from_me: bool,
    text: String,
}

/// How many messages are kept for quoting (the ui keeps 128).
const QUOTABLE: usize = 128;
/// Longest quoted text kept.
const QUOTED_TEXT: usize = 200;

type Shared = Arc<Mutex<State>>;

impl State {
    fn remember(&mut self, chat: &Jid, id: &str, sender: &Jid, from_me: bool, text: &str) {
        if id.is_empty() {
            return;
        }
        if self.recent.len() == QUOTABLE {
            self.recent.remove(0);
        }
        let text: String = text.chars().take(QUOTED_TEXT).collect();
        self.recent.push(Quotable { chat: chat.clone(), id: id.to_string(), sender: sender.clone(), from_me, text });
    }
}

/// The id of the message a message answers, if it is an answer.
fn replied_id(message: &wa::Message) -> String {
    message
        .extended_text_message
        .as_option()
        .and_then(|extended| extended.context_info.as_option())
        .and_then(|context| context.stanza_id.clone())
        .unwrap_or_default()
}

fn tell_ui(lines: Lines) {
    for line in lines {
        say(&line.iter().map(String::as_str).collect::<Vec<_>>());
    }
}

fn audio_state() -> Audio {
    Audio { microphone: audio::microphone_available(), speaker: audio::speaker_available() }
}

fn data_directory() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))?;
    Some(base.join("zapzap"))
}

/// The number the ui knows the chat by, adding it (and telling the ui) the first time it is seen.
fn chat_number(state: &mut State, jid: &Jid, name: &str) -> Option<usize> {
    if let Some(index) = state.chats.iter().position(|(known, _)| known == jid) {
        if let Some(at) = state.removed.iter().position(|&gone| gone == index) {
            state.removed.remove(at);
            let shown = state.chats[index].1.clone();
            say(&["CHAT", &index.to_string(), "0", "", &shown, ""]);
        }
        return Some(index);
    }
    if state.chats.len() >= MAX_CHATS {
        return None;
    }
    let shown = if name.is_empty() { jid.user.to_string() } else { name.to_string() };
    state.chats.push((jid.clone(), shown.clone()));
    let index = state.chats.len() - 1;
    say(&["CHAT", &index.to_string(), "0", "", &shown, ""]);
    Some(index)
}

async fn on_message(shared: Shared, context: MessageContext) {
    // An emoji reaction is a message of its own: it names the message it is on and carries the emoji.
    if let Some(reaction) = context.message.reaction_message.as_option() {
        let (Some(target), emoji) = (reaction.key.as_option().and_then(|key| key.id.clone()), reaction.text.clone().unwrap_or_default()) else { return };
        let mut state = shared.lock().expect("state");
        let Some(chat) = state.chats.iter().position(|(jid, _)| *jid == context.info.source.chat) else { return };
        let who = if context.info.source.is_from_me { ME.to_string() } else { context.info.source.sender.to_string() };
        state.reactions.set(chat, &target, &who, &emoji);
        say(&state.reactions.line(chat, &target).iter().map(String::as_str).collect::<Vec<_>>());
        return;
    }
    // A deletion for everyone is a protocol message of type REVOKE naming the message.
    if let Some(protocol) = context.message.protocol_message.as_option() {
        // An edit carries the message it changes and the new content.
        if protocol.r#type == Some(wa::message::protocol_message::Type::MESSAGE_EDIT.into()) {
            let target = protocol.key.as_option().and_then(|key| key.id.clone());
            let text = protocol.edited_message.as_option().and_then(|edited| edited.text_content().map(str::to_string));
            if let (Some(target), Some(text)) = (target, text) {
                let state = shared.lock().expect("state");
                if let Some(chat) = state.chats.iter().position(|(jid, _)| *jid == context.info.source.chat) {
                    say(&["EDITED", &chat.to_string(), &target, &text]);
                }
            }
            return;
        }
        if protocol.r#type == Some(wa::message::protocol_message::Type::REVOKE.into()) {
            if let Some(target) = protocol.key.as_option().and_then(|key| key.id.clone()) {
                let state = shared.lock().expect("state");
                if let Some(chat) = state.chats.iter().position(|(jid, _)| *jid == context.info.source.chat) {
                    say(&["REVOKED", &chat.to_string(), &target, if context.info.source.is_from_me { "1" } else { "0" }]);
                }
            }
        }
        return;
    }
    let Some(text) = context.message.text_content() else { return };
    let info = &context.info;
    let mut state = shared.lock().expect("state");
    // Only the other side's push name names a chat; ours would name it after ourselves.
    let name = if info.source.is_from_me || info.source.is_group { "" } else { info.push_name.as_str() };
    let Some(chat) = chat_number(&mut state, &info.source.chat, name) else { return };
    let id = info.id.to_string();
    state.remember(&info.source.chat, &id, &info.source.sender, info.source.is_from_me, text);
    let time = clock_at(info.timestamp.timestamp());
    let outgoing = if info.source.is_from_me { "1" } else { "0" };
    say(&["MSG", &chat.to_string(), outgoing, "0", &time, &id, &replied_id(&context.message), text]);
}

/// What the ui needs of a history conversation: it keeps few, and each with only its last messages.
struct Recent {
    timestamp: u64,
    jid: Jid,
    name: String,
    unread: u32,
    messages: Vec<HistoryMessage>,
}

/// A message of a history conversation, oldest first.
struct HistoryMessage {
    from_me: bool,
    at: u64,
    id: String,
    reply: String,
    text: String,
}

/// Reads a history-sync chunk one conversation at a time (never the whole inflated blob at once, which
/// can be tens of MB) and keeps only the conversations and messages the ui can show.
fn recent_from_history(sync: &whatsapp_rust::wacore::types::events::LazyHistorySync) -> Vec<Recent> {
    let mut stream = sync.stream();
    let mut kept: Vec<Recent> = Vec::new();
    while let Ok(Some(conversation)) = stream.next_conversation() {
        let Ok(jid) = conversation.id.parse::<Jid>() else { continue };
        if jid.is_status_broadcast() {
            continue;
        }
        let timestamp = conversation.conversation_timestamp.or(conversation.last_msg_timestamp).unwrap_or(0);
        // Not among the freshest already kept: skip the work of reading its messages.
        if kept.len() >= MAX_CHATS && kept.iter().all(|other| other.timestamp >= timestamp) {
            continue;
        }
        let mut messages: Vec<HistoryMessage> = conversation
            .messages
            .iter()
            .filter_map(|entry| {
                let info = entry.message.as_option()?;
                let message = info.message.as_option()?;
                let key = info.key.as_option();
                Some(HistoryMessage {
                    from_me: key.is_some_and(|key| key.from_me.unwrap_or(false)),
                    at: info.message_timestamp.unwrap_or(0),
                    id: key.and_then(|key| key.id.clone()).unwrap_or_default(),
                    reply: replied_id(message),
                    text: message.text_content()?.to_string(),
                })
            })
            .collect();
        messages.sort_by_key(|message| message.at);
        let skip = messages.len().saturating_sub(MESSAGES_PER_CHAT);
        messages.drain(..skip);
        kept.push(Recent { timestamp, jid, name: conversation.name.clone().unwrap_or_default(), unread: conversation.unread_count.unwrap_or(0), messages });
        kept.sort_by_key(|recent| std::cmp::Reverse(recent.timestamp));
        kept.truncate(MAX_CHATS);
    }
    kept
}

/// Shows the conversations of a history chunk that the ui does not know yet.
fn show_history(state: &mut State, recents: Vec<Recent>, own: Option<Jid>) {
    for recent in recents {
        if state.chats.len() >= MAX_CHATS || state.chats.iter().any(|(jid, _)| *jid == recent.jid) {
            continue;
        }
        let shown = if recent.name.is_empty() { recent.jid.user.to_string() } else { recent.name.clone() };
        if !recent.name.is_empty() {
            state.book.insert(&recent.name);
        }
        state.chats.push((recent.jid.clone(), shown.clone()));
        let chat = (state.chats.len() - 1).to_string();
        let last = recent.messages.last().map_or(recent.timestamp, |message| message.at);
        say(&["CHAT", &chat, &recent.unread.min(255).to_string(), &clock_at(last as i64), &shown, ""]);
        for message in &recent.messages {
            // Who wrote a history message is not known here beyond "me or the chat": enough to quote it.
            let sender = if message.from_me { own.clone().unwrap_or_else(|| recent.jid.clone()) } else { recent.jid.clone() };
            state.remember(&recent.jid, &message.id, &sender, message.from_me, &message.text);
            say(&["MSG", &chat, if message.from_me { "1" } else { "0" }, "0", &clock_at(message.at as i64), &message.id, &message.reply, &message.text]);
        }
    }
}

/// "visto por último hoje às 09:48", from Brasilia calendar days.
fn last_seen_text(now: i64, seen: i64) -> String {
    const OFFSET: i64 = -3 * 3600;
    let days_ago = (now + OFFSET).div_euclid(86_400) - (seen + OFFSET).div_euclid(86_400);
    let time = clock_at(seen);
    match days_ago {
        i64::MIN..=0 => format!("visto por último hoje às {time}"),
        1 => format!("visto por último ontem às {time}"),
        _ => {
            let (_, month, day) = civil_date(seen + OFFSET);
            format!("visto por último em {day:02}/{month:02} às {time}")
        }
    }
}

/// (year, month, day) of a day count since 1970-01-01 (Howard Hinnant's civil-from-days).
fn civil_date(unix_seconds: i64) -> (i64, i64, i64) {
    let z = unix_seconds.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(month <= 2), month, day)
}

async fn on_event(shared: Shared, event: Arc<whatsapp_rust::wacore::types::events::Event>, own_jid: Option<Jid>) {
    use whatsapp_rust::wacore::types::events::Event;
    use whatsapp_rust::wacore::types::presence::ReceiptType;
    let mut state = shared.lock().expect("state");
    match &*event {
        Event::HistorySync(sync) => {
            let recents = recent_from_history(sync);
            show_history(&mut state, recents, own_jid);
        }
        Event::Receipt(receipt) if !receipt.source.is_from_me => {
            let line = match receipt.r#type {
                ReceiptType::Read | ReceiptType::ReadSelf => "READ",
                ReceiptType::Delivered => "DELIVERED",
                _ => return,
            };
            if let Some(chat) = state.chats.iter().position(|(jid, _)| *jid == receipt.source.chat) {
                say(&[line, &chat.to_string()]);
            }
        }
        Event::ChatPresence(update) if !update.source.is_group => {
            if let Some(chat) = state.chats.iter().position(|(jid, _)| *jid == update.source.chat) {
                let text = if update.state == whatsapp_rust::wacore::types::presence::ChatPresence::Composing { "digitando…" } else { "" };
                say(&["PRESENCE", &chat.to_string(), text]);
            }
        }
        Event::Presence(update) => {
            if let Some(chat) = state.chats.iter().position(|(jid, _)| *jid == update.from) {
                let text = match (update.unavailable, update.last_seen) {
                    (false, _) => "online".to_string(),
                    (true, Some(seen)) => last_seen_text(whatsapp_rust::wacore::time::now_utc().timestamp(), seen.timestamp()),
                    (true, None) => String::new(),
                };
                say(&["PRESENCE", &chat.to_string(), &text]);
            }
        }
        Event::StarUpdate(update) => {
            if let Some(chat) = state.chats.iter().position(|(jid, _)| *jid == update.chat_jid) {
                say(&["STARRED", &chat.to_string(), &update.message_id, if update.action.starred.unwrap_or(false) { "1" } else { "0" }]);
            }
        }
        Event::ContactUpdate(update) => {
            let name = update.action.full_name.as_deref().filter(|name| !name.is_empty()).or(update.action.first_name.as_deref());
            if let Some(name) = name {
                state.book.insert_with_id(name, Some(&update.jid.to_string()));
            }
        }
        Event::SelfPushNameUpdated(update) => say(&["PROFILE", &update.new_name]),
        Event::IncomingCall(call) => {
            let is_voice = matches!(call.action, CallAction::Offer { is_video: false, group_jid: None, .. });
            if !is_voice || state.ringing.is_some() {
                return;
            }
            let caller = call.notify.clone().filter(|name| !name.is_empty()).unwrap_or_else(|| call.from.user.to_string());
            let out = state.calls.handle(CallEvent::Incoming(caller), &audio_state());
            state.ringing = Some(call.clone());
            tell_ui(out.lines);
        }
        Event::MissedCall(missed) => {
            if state.ringing.as_ref().is_some_and(|call| call.action.call_id() == missed.call_id) {
                state.ringing = None;
                tell_ui(state.calls.handle(CallEvent::PeerEnded(String::new()), &audio_state()).lines);
            }
        }
        Event::CallEndedElsewhere(ended) => {
            if state.ringing.as_ref().is_some_and(|call| call.action.call_id() == ended.call_id) {
                state.ringing = None;
                tell_ui(state.calls.handle(CallEvent::AnsweredElsewhere, &audio_state()).lines);
            }
        }
        Event::Disconnected(_) => say(&["NOTICE", "Sem conexão com o WhatsApp. Tentando de novo..."]),
        _ => {}
    }
}

/// Sends `text` to a chat, quoting the message `quoted` if it is given and still remembered, and tells the
/// ui the id the new message got.
async fn send_text(shared: &Shared, client: &Arc<Client>, chat: usize, text: String, quoted: Option<String>) {
    let (jid, context) = {
        let state = shared.lock().expect("state");
        let Some((jid, _)) = state.chats.get(chat) else { return };
        let context = quoted.and_then(|id| {
            let found = state.recent.iter().find(|known| known.chat == *jid && known.id == id)?;
            Some(build_quote_context_with_info(found.id.clone(), &found.sender, jid, jid, &wa::Message::text(found.text.clone())))
        });
        (jid.clone(), context)
    };
    let message = match context {
        Some(context) => wa::Message::text_with_context(text.clone(), context),
        None => wa::Message::text(text.clone()),
    };
    match client.send_message(&jid, message).await {
        Ok(sent) => {
            say(&["SENT", &chat.to_string(), &sent.message_id]);
            if let Some(own) = client.pn() {
                shared.lock().expect("state").remember(&jid, &sent.message_id, &own, true, &text);
            }
        }
        Err(error) => {
            eprintln!("send failed: {error}");
            say(&["STATUS", "Não foi possível enviar a mensagem."]);
        }
    }
}

/// Reacts with `emoji` (empty takes the reaction back) to the remembered message `id` of a chat.
async fn send_reaction(shared: &Shared, client: &Arc<Client>, chat: usize, id: &str, emoji: String) {
    let target = {
        let state = shared.lock().expect("state");
        let Some((jid, _)) = state.chats.get(chat) else { return };
        let Some(found) = state.recent.iter().find(|known| known.chat == *jid && known.id == id) else { return };
        (jid.clone(), found.from_me, found.sender.clone())
    };
    let (jid, from_me, sender) = target;
    let key = wa::MessageKey {
        remote_jid: Some(jid.to_string()),
        from_me: Some(from_me),
        id: Some(id.to_string()),
        participant: (jid.is_group() && !from_me).then(|| sender.to_string()),
    };
    let reaction = wa::message::ReactionMessage {
        key: key.into(),
        text: Some(emoji.clone()),
        sender_timestamp_ms: Some(whatsapp_rust::wacore::time::now_utc().timestamp_millis()),
        ..Default::default()
    };
    let message = wa::Message { reaction_message: reaction.into(), ..Default::default() };
    if let Err(error) = client.send_message(&jid, message).await {
        eprintln!("reaction failed: {error}");
        say(&["STATUS", "Não foi possível enviar a reação."]);
        return;
    }
    let mut state = shared.lock().expect("state");
    state.reactions.set(chat, id, ME, &emoji);
    say(&state.reactions.line(chat, id).iter().map(String::as_str).collect::<Vec<_>>());
}

/// What the ui typed, one command at a time.
async fn serve_commands(shared: Shared, client: Arc<Client>, mut commands: mpsc::UnboundedReceiver<Command>) {
    while let Some(command) = commands.recv().await {
        match command {
            Command::Send { chat, text } => send_text(&shared, &client, chat, text, None).await,
            Command::Reply { chat, id, text } => send_text(&shared, &client, chat, text, Some(id)).await,
            Command::React { chat, id, emoji } => send_reaction(&shared, &client, chat, &id, emoji).await,
            Command::Mute { chat, muted } => {
                let jid = shared.lock().expect("state").chats.get(chat).map(|(jid, _)| jid.clone());
                let Some(jid) = jid else { continue };
                let actions = client.chat_actions();
                let done = if muted { actions.mute_chat(&jid).await } else { actions.unmute_chat(&jid).await };
                if let Err(error) = done {
                    eprintln!("mute failed: {error}");
                    say(&["STATUS", "Não foi possível mudar as notificações da conversa."]);
                }
            }
            Command::DeleteChat { chat } => {
                let jid = {
                    let mut state = shared.lock().expect("state");
                    if !state.removed.contains(&chat) {
                        state.removed.push(chat);
                    }
                    state.chats.get(chat).map(|(jid, _)| jid.clone())
                };
                let Some(jid) = jid else { continue };
                if let Err(error) = client.chat_actions().delete_chat(&jid, false, None).await {
                    eprintln!("delete chat failed: {error}");
                    say(&["STATUS", "Não foi possível apagar a conversa."]);
                }
            }
            Command::ClearChat { chat } => {
                let jid = shared.lock().expect("state").chats.get(chat).map(|(jid, _)| jid.clone());
                let Some(jid) = jid else { continue };
                if let Err(error) = client.chat_actions().clear_chat(&jid, false, false, None).await {
                    eprintln!("clear failed: {error}");
                    say(&["STATUS", "Não foi possível limpar a conversa."]);
                }
            }
            Command::Edit { chat, id, text } => {
                let jid = shared.lock().expect("state").chats.get(chat).map(|(jid, _)| jid.clone());
                let Some(jid) = jid else { continue };
                let content = wa::Message { conversation: Some(text), ..Default::default() };
                if let Err(error) = client.edit_message(jid, id, content).await {
                    eprintln!("edit failed: {error}");
                    say(&["STATUS", "Não foi possível editar a mensagem."]);
                }
            }
            Command::MarkRead { chat } => {
                let jid = shared.lock().expect("state").chats.get(chat).map(|(jid, _)| jid.clone());
                let Some(jid) = jid else { continue };
                if let Err(error) = client.chat_actions().mark_chat_as_read(&jid, true, None).await {
                    eprintln!("mark read failed: {error}");
                }
            }
            Command::Star { chat, id, starred } => {
                let target = {
                    let state = shared.lock().expect("state");
                    let Some((jid, _)) = state.chats.get(chat) else { continue };
                    let Some(found) = state.recent.iter().find(|known| known.chat == *jid && known.id == id) else { continue };
                    (jid.clone(), found.from_me, found.sender.clone())
                };
                let (jid, from_me, sender) = target;
                let participant = (jid.is_group() && !from_me).then_some(&sender);
                let actions = client.chat_actions();
                let done = if starred { actions.star_message(&jid, participant, &id, from_me).await } else { actions.unstar_message(&jid, participant, &id, from_me).await };
                if let Err(error) = done {
                    eprintln!("star failed: {error}");
                    say(&["STATUS", "Não foi possível favoritar a mensagem."]);
                }
            }
            Command::Delete { chat, id } => {
                let jid = shared.lock().expect("state").chats.get(chat).map(|(jid, _)| jid.clone());
                let Some(jid) = jid else { continue };
                if let Err(error) = client.revoke_message(jid, id, whatsapp_rust::send::RevokeType::Sender).await {
                    eprintln!("delete failed: {error}");
                    say(&["STATUS", "Não foi possível apagar a mensagem."]);
                }
            }
            Command::Contacts { query } => {
                let state = shared.lock().expect("state");
                let found = state.book.search(&query, UI_CONTACTS);
                tell_ui(contacts_reply(&found));
            }
            Command::OpenChat { name } => {
                let mut state = shared.lock().expect("state");
                let jid = state.book.id_of(&name).and_then(|id| id.parse::<Jid>().ok());
                if let Some(chat) = jid.and_then(|jid| chat_number(&mut state, &jid, &name)) {
                    say(&["SHOWCHAT", &chat.to_string()]);
                }
            }
            Command::Dial { name } => {
                eprintln!("the ui asked to call {name}; outgoing calls are not implemented");
                say(&["CALLSTATE", "failed", "Ligações indisponíveis", "As ligações de voz ainda não estão implementadas no ZapZap. Por enquanto só a interface existe."]);
            }
            Command::Answer | Command::Hangup => {
                let answering = matches!(command, Command::Answer);
                let call = {
                    let mut state = shared.lock().expect("state");
                    state.calls.handle(CallEvent::Hangup, &audio_state());
                    state.ringing.take()
                };
                if let Some(call) = call {
                    if let Err(error) = client.voip().reject(&call).await {
                        eprintln!("reject failed: {error}");
                    }
                    if answering {
                        say(&["CALLSTATE", "failed", "Não foi possível atender", "O ZapZap ainda não consegue transmitir áudio: a ligação foi recusada."]);
                    }
                }
            }
        }
    }
}

async fn run() {
    let Some(directory) = data_directory() else {
        say(&["NOTICE", "Não foi possível achar onde guardar os dados (HOME não definido)."]);
        return;
    };
    if let Err(error) = std::fs::create_dir_all(&directory) {
        eprintln!("cannot create {}: {error}", directory.display());
        say(&["NOTICE", "Não foi possível criar a pasta de dados do ZapZap."]);
        return;
    }
    say(&["NOTICE", "Conectando ao WhatsApp..."]);
    let database = directory.join("whatsapp.db");
    // A small page cache: the store holds a few keys and sessions, not a big database.
    let config = whatsapp_rust::store::SqliteStoreConfig { cache_size_kib: SQLITE_CACHE_KIB, ..Default::default() };
    let store = match SqliteStore::with_config(&database.to_string_lossy(), config).await {
        Ok(store) => store,
        Err(error) => {
            eprintln!("cannot open {}: {error}", database.display());
            say(&["NOTICE", "Não foi possível abrir o banco de dados do ZapZap."]);
            return;
        }
    };

    let shared: Shared = Arc::new(Mutex::new(State { calls: Calls::new(), ringing: None, chats: Vec::new(), recent: Vec::new(), book: ContactBook::new(), reactions: Reactions::default(), removed: Vec::new() }));
    let (message_state, event_state) = (shared.clone(), shared.clone());
    let bot = Bot::builder()
        .with_backend(store)
        .on_qr_code(|code, _timeout| async move { say(&["QR", &code]) })
        .on_connected(|_client| async {
            say(&["RESET"]);
            say(&["OPEN"]);
            say(&["STATUS", "Conectado. Carregando as conversas recentes..."]);
        })
        .on_logged_out(|_info| async { say(&["NOTICE", "Este aparelho foi desconectado da conta. Reinicie para parear de novo."]) })
        .on_message(move |context| on_message(message_state.clone(), context))
        .on_event_for(
            &[
                EventKind::IncomingCall,
                EventKind::MissedCall,
                EventKind::CallEndedElsewhere,
                EventKind::Disconnected,
                EventKind::HistorySync,
                EventKind::Receipt,
                EventKind::SelfPushNameUpdated,
                EventKind::ChatPresence,
                EventKind::ContactUpdate,
                EventKind::StarUpdate,
                EventKind::Presence,
            ],
            move |event, client| on_event(event_state.clone(), event, client.pn()),
        )
        .build()
        .await;
    let bot = match bot {
        Ok(bot) => bot,
        Err(error) => {
            eprintln!("cannot start the client: {error}");
            say(&["NOTICE", "Não foi possível iniciar o cliente do WhatsApp."]);
            return;
        }
    };

    // The ui's lines arrive on a thread of their own (stdin blocks); hand them to the async side.
    let (sender, receiver) = mpsc::unbounded_channel();
    let from_ui = commands();
    std::thread::spawn(move || {
        while let Ok(command) = from_ui.recv() {
            if sender.send(command).is_err() {
                return;
            }
        }
    });
    tokio::spawn(serve_commands(shared, bot.client(), receiver));
    // Start-up code is done with once the client runs: let go of the code pages that are idle, now
    // and every half minute, so the resident set follows what the process actually executes.
    tokio::spawn(async {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(15)).await;
            trim::drop_idle_code();
        }
    });
    bot.run().await;
}

fn main() {
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("async runtime");
    runtime.block_on(run());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use whatsapp_rust::wacore::types::events::LazyHistorySync;
    use whatsapp_rust::waproto::whatsapp as proto;

    fn text(from_me: bool, at: u64, body: &str) -> proto::HistorySyncMsg {
        proto::HistorySyncMsg {
            message: proto::WebMessageInfo {
                key: proto::MessageKey { from_me: Some(from_me), ..Default::default() }.into(),
                message: proto::Message { conversation: Some(body.to_string()), ..Default::default() }.into(),
                message_timestamp: Some(at),
                ..Default::default()
            }
            .into(),
            ..Default::default()
        }
    }

    fn conversation(id: &str, name: Option<&str>, timestamp: u64, unread: u32, messages: Vec<proto::HistorySyncMsg>) -> proto::Conversation {
        proto::Conversation {
            id: id.to_string(),
            name: name.map(str::to_string),
            conversation_timestamp: Some(timestamp),
            unread_count: Some(unread),
            messages,
            ..Default::default()
        }
    }

    fn lazy(conversations: Vec<proto::Conversation>) -> LazyHistorySync {
        let history = proto::HistorySync { sync_type: proto::history_sync::HistorySyncType::INITIAL_BOOTSTRAP, conversations, ..Default::default() };
        let raw = whatsapp_rust::waproto::codec::history_sync_to_vec(&history);
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&raw).unwrap();
        LazyHistorySync::new(encoder.finish().unwrap().into(), raw.len(), 0, None, None)
    }

    #[test]
    fn the_freshest_conversations_come_first_with_their_last_messages_in_order() {
        let sync = lazy(vec![
            conversation("5511000000001@s.whatsapp.net", Some("Ana"), 100, 0, vec![text(false, 90, "antiga"), text(true, 100, "recente")]),
            conversation("5511000000002@s.whatsapp.net", None, 300, 3, vec![text(false, 300, "oi")]),
        ]);
        let recents = recent_from_history(&sync);
        assert_eq!(recents.iter().map(|recent| recent.jid.user.to_string()).collect::<Vec<_>>(), ["5511000000002", "5511000000001"]);
        assert_eq!(recents[0].unread, 3);
        assert_eq!(recents[1].name, "Ana");
        assert_eq!(recents[1].messages.iter().map(|message| message.text.as_str()).collect::<Vec<_>>(), ["antiga", "recente"]);
        assert!(recents[1].messages[1].from_me, "the second message is ours");
    }

    #[test]
    fn only_the_last_messages_are_kept() {
        let messages = (0..20u64).map(|n| text(false, n, &format!("m{n}"))).collect();
        let recents = recent_from_history(&lazy(vec![conversation("5511000000001@s.whatsapp.net", None, 1, 0, messages)]));
        assert_eq!(recents[0].messages.len(), MESSAGES_PER_CHAT);
        assert_eq!(recents[0].messages.last().map(|message| message.text.as_str()), Some("m19"));
    }

    #[test]
    fn no_more_conversations_than_the_ui_holds_and_the_stalest_are_dropped() {
        let many = (0..(MAX_CHATS as u64 + 5)).map(|n| conversation(&format!("55110000000{n:02}@s.whatsapp.net"), None, n + 1, 0, vec![text(false, n, "x")])).collect();
        let recents = recent_from_history(&lazy(many));
        assert_eq!(recents.len(), MAX_CHATS);
        assert!(recents.iter().all(|recent| recent.timestamp > 5));
    }

    #[test]
    fn the_status_broadcast_is_not_a_conversation() {
        let recents = recent_from_history(&lazy(vec![conversation("status@broadcast", None, 9, 0, vec![text(false, 9, "status")])]));
        assert!(recents.is_empty());
    }

    #[test]
    fn a_message_names_the_one_it_answers() {
        let answer = proto::Message {
            extended_text_message: proto::message::ExtendedTextMessage {
                context_info: proto::ContextInfo { stanza_id: Some("Q1".to_string()), ..Default::default() }.into(),
                ..Default::default()
            }
            .into(),
            ..Default::default()
        };
        assert_eq!(replied_id(&answer), "Q1");
        assert_eq!(replied_id(&proto::Message::default()), "");
    }

    #[test]
    fn only_the_latest_messages_are_kept_for_quoting_and_ids_are_needed() {
        let jid: Jid = "5511000000001@s.whatsapp.net".parse().unwrap();
        let mut state = State { calls: Calls::new(), ringing: None, chats: Vec::new(), recent: Vec::new(), book: ContactBook::new(), reactions: Reactions::default(), removed: Vec::new() };
        for n in 0..(QUOTABLE + 10) {
            state.remember(&jid, &format!("id{n}"), &jid, false, "texto");
        }
        state.remember(&jid, "", &jid, false, "sem id");
        assert_eq!(state.recent.len(), QUOTABLE);
        assert_eq!(state.recent[0].id, "id10");
    }

    #[test]
    fn long_quoted_texts_are_cut() {
        let jid: Jid = "5511000000001@s.whatsapp.net".parse().unwrap();
        let mut state = State { calls: Calls::new(), ringing: None, chats: Vec::new(), recent: Vec::new(), book: ContactBook::new(), reactions: Reactions::default(), removed: Vec::new() };
        state.remember(&jid, "x", &jid, false, &"a".repeat(1000));
        assert_eq!(state.recent[0].text.chars().count(), QUOTED_TEXT);
    }

    #[test]
    fn last_seen_says_today_yesterday_or_the_date() {
        // 2026-09-20 12:00 Brasilia = 15:00 UTC.
        let now = 1_789_916_400;
        assert_eq!(last_seen_text(now, now - 3 * 3600), "visto por último hoje às 09:00");
        assert_eq!(last_seen_text(now, now - 26 * 3600), "visto por último ontem às 10:00");
        assert_eq!(last_seen_text(now, now - 10 * 86_400), "visto por último em 10/09 às 12:00");
    }

    #[test]
    fn civil_dates_are_right_across_leap_days() {
        assert_eq!(civil_date(0), (1970, 1, 1));
        assert_eq!(civil_date(951_782_400), (2000, 2, 29));
        assert_eq!(civil_date(1_789_916_400), (2026, 9, 20));
    }

    #[test]
    fn the_ui_learns_each_conversation_once() {
        let mut state = State { calls: Calls::new(), ringing: None, chats: Vec::new(), recent: Vec::new(), book: ContactBook::new(), reactions: Reactions::default(), removed: Vec::new() };
        let jid: Jid = "5511000000001@s.whatsapp.net".parse().unwrap();
        let recent = || Recent { timestamp: 1, jid: jid.clone(), name: "Ana".into(), unread: 1, messages: vec![HistoryMessage { from_me: false, at: 1, id: "A1".into(), reply: String::new(), text: "oi".into() }] };
        show_history(&mut state, vec![recent()], None);
        show_history(&mut state, vec![recent()], None);
        assert_eq!(state.chats.len(), 1);
    }
}
