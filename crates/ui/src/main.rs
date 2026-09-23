#![no_std]
#![no_main]

mod canvas;
mod chat;
mod font;
mod icon_ids;
mod icons;
mod pack;
mod platform;
mod view;

use canvas::Canvas;
use chat::{sidebar_width, CallKind, Chat, Focus, Tab};
use platform::{sys, Cursor, Event, Window};
use view::{FindHit, Hit};
use zapzap_qr::Qr;

const BAND_ROWS: usize = 4;
const MAX_WIDTH: usize = 3840;
static mut BAND: [u32; MAX_WIDTH * BAND_ROWS] = [0; MAX_WIDTH * BAND_ROWS];
/// Kept out of the stack: it is zero-initialised, so its pages cost nothing until they are used.
static mut CHAT: Chat = Chat::new();

const STDIN: i32 = 0;
/// Half a blink of the caret.
const CARET_BLINK_MS: i32 = 530;
/// Longest gap between clicks that still count as a double click.
const MULTI_CLICK_MS: i64 = 450;
const LINE_CAPACITY: usize = 320;
const NOTICE_CAPACITY: usize = 128;

/// The text box a drag that started in it is selecting in.
#[derive(Clone, Copy)]
enum Drag {
    Message,
    Search,
    Find,
}

enum Screen {
    Notice,
    Login(Qr),
    Chat,
}

struct Notice {
    bytes: [u8; NOTICE_CAPACITY],
    len: usize,
}

impl Notice {
    fn set(&mut self, text: &[u8]) {
        let len = text.len().min(NOTICE_CAPACITY);
        self.bytes[..len].copy_from_slice(&text[..len]);
        self.len = len;
    }

    fn text(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }
}

core::arch::global_asm!(
    ".globl _start",
    "_start:",
    "xor ebp, ebp",
    "mov rdi, rsp",
    "and rsp, -16",
    "call {entry}",
    "ud2",
    entry = sym entry,
);

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    sys::exit(101)
}

unsafe extern "C" fn entry(stack: *const usize) -> ! {
    let argc = *stack;
    run(stack.add(argc + 2).cast())
}

fn number(bytes: &[u8]) -> Option<usize> {
    bytes.iter().try_fold(0usize, |total, &digit| digit.is_ascii_digit().then(|| total * 10 + (digit - b'0') as usize))
}

fn text(bytes: &[u8]) -> Option<&str> {
    core::str::from_utf8(bytes).ok()
}

/// One line from the core process. Fields are separated by tabs and the last one is free text:
///   `QR` payload                      a pairing code
///   `NOTICE` text                     a message on an otherwise empty screen
///   `RESET`                           forget every conversation and message
///   `CHAT` id unread time name status define or update a conversation
///   `PRESENCE` chat text              only the status line under the chat's name (online, typing...)
///   `MSG` chat out read time id reply text  append a message (out and read are 0 or 1; id names it, reply is the id it answers, both may be empty)
///   `PAST` chat out read time id reply text  an older message, put before the conversation's first (same fields as `MSG`)
///   `MEDIA` chat id kind w h label   message `id` carries a photo (1), video (2), voice message (3), document (4) or sticker (5); label is a file name or a duration
///   `THUMB` chat id w h, `THUMBROW` chat id y hex   the preview picture of a message, row by row (RGB565 little endian, hex); `THUMBFAIL` chat id when there is none
///   `PASTEND` chat more               that is all for the last `OLDER`; more is 1 when older ones may still exist
///   `REVOKED` chat id by_me           the message was deleted for everyone (by_me is 0 or 1)
///   `EDITED` chat id text             a message now reads `text` (edited by its sender)
///   `STARRED` chat id 1|0             a message was marked (1) or unmarked (0) as a favourite
///   `REACTIONS` chat id mine counts    the reactions of a message: our emoji (or empty) and `emoji:count` pairs, most used first
///   `SENT` chat id                    the connection gave this id to what we sent last in the chat
///   `CALL` kind count time name      a recent call (kind 0 received, 1 missed, 2 outgoing)
///   `STORY` segments unseen time name  a contact's status (its ring has `segments` parts, `unseen` of them new)
///   `MYSTORY` time                    when my own status was posted
///   `CHANNEL` unread time name preview  a followed channel
///   `SUGGEST` verified followers name   a channel to follow (verified is 0 or 1)
///   `SHOWCHAT` chat                   open that conversation (the answer to picking a contact without one)
///   `CONTACTRESET`                    the contacts sent before are outdated (the core answers each `CONTACTS query` with this and the matches)
///   `CONTACT` name                    a contact for the new-conversation panel, in alphabetical order
///   `INCOMING` name                   someone calls us
///   `CALLSTATE` active                the call we placed was answered
///   `CALLSTATE` ended [reason]        the call is over; a reason (no answer, busy...) stays on screen for a moment
///   `CALLSTATE` failed title body     the call could not be made: a message box explains
///   `NEWCALL` kind count time name    a call that just ended, at the top of the recents
///   `PROFILE` name                    the account name of the settings screen
///   `FAVORITE` name                   a favourite contact of the calls screen
///   `DELIVERED` chat                  the other side's phone got what we sent
///   `READ` chat                       the other side read what we sent
///   `OPEN`                            show the chat screen
///   `STATUS` text                     a line of bad news, shown until the next key or click
fn handle_line(line: &[u8], screen: &mut Screen, notice: &mut Notice, chat: &mut Chat) {
    let mut fields = line.splitn(2, |&b| b == b'\t');
    let (Some(command), rest) = (fields.next(), fields.next().unwrap_or(&[])) else { return };
    match command {
        b"QR" => {
            *screen = match zapzap_qr::encode(rest) {
                Some(qr) => Screen::Login(qr),
                None => Screen::Notice,
            };
        }
        b"NOTICE" => {
            notice.set(rest);
            *screen = Screen::Notice;
        }
        b"RESET" => chat.reset(),
        b"OPEN" => {
            chat.go_live();
            *screen = Screen::Chat;
        }
        b"DELIVERED" => {
            if let Some(id) = number(rest) {
                chat.mark_delivered(id);
            }
        }
        b"READ" => {
            if let Some(id) = number(rest) {
                chat.mark_read(id);
            }
        }
        b"STATUS" => chat.set_banner(text(rest).unwrap_or("")),
        b"CHAT" => {
            let mut parts = rest.splitn(5, |&b| b == b'\t');
            if let (Some(id), Some(unread), Some(time), Some(name), Some(status)) = (parts.next(), parts.next(), parts.next(), parts.next(), parts.next()) {
                if let (Some(id), Some(unread), Some(time), Some(name), Some(status)) = (number(id), number(unread), text(time), text(name), text(status)) {
                    chat.set_conversation(id, unread.min(255) as u8, time, name, status);
                }
            }
        }
        b"CALL" => {
            let mut parts = rest.splitn(4, |&b| b == b'\t');
            if let (Some(kind), Some(count), Some(time), Some(name)) = (parts.next(), parts.next(), parts.next(), parts.next()) {
                if let (Some(kind), Some(count), Some(time), Some(name)) = (number(kind), number(count), text(time), text(name)) {
                    let kind = match kind {
                        1 => CallKind::Missed,
                        2 => CallKind::Outgoing,
                        _ => CallKind::Received,
                    };
                    chat.add_call(kind, count.min(255) as u8, time, name);
                }
            }
        }
        b"STORY" => {
            let mut parts = rest.splitn(4, |&b| b == b'\t');
            if let (Some(segments), Some(unseen), Some(time), Some(name)) = (parts.next(), parts.next(), parts.next(), parts.next()) {
                if let (Some(segments), Some(unseen), Some(time), Some(name)) = (number(segments), number(unseen), text(time), text(name)) {
                    chat.add_story(segments.min(60) as u16, unseen.min(60) as u16, time, name);
                }
            }
        }
        b"MYSTORY" => chat.set_my_status(text(rest).unwrap_or("")),
        b"CHANNEL" => {
            let mut parts = rest.splitn(4, |&b| b == b'\t');
            if let (Some(unread), Some(time), Some(name), Some(preview)) = (parts.next(), parts.next(), parts.next(), parts.next()) {
                if let (Some(unread), Some(time), Some(name), Some(preview)) = (number(unread), text(time), text(name), text(preview)) {
                    chat.add_channel(unread.min(999) as u16, time, name, preview);
                }
            }
        }
        b"SUGGEST" => {
            let mut parts = rest.splitn(3, |&b| b == b'\t');
            if let (Some(verified), Some(followers), Some(name)) = (parts.next(), parts.next(), parts.next()) {
                if let (Some(followers), Some(name)) = (text(followers), text(name)) {
                    chat.add_suggestion(verified == b"1", followers, name);
                }
            }
        }
        b"SHOWCHAT" => {
            if let Some(id) = number(rest) {
                chat.show_chat(id);
            }
        }
        b"CONTACTRESET" => chat.clear_contacts(),
        b"CONTACT" => chat.add_contact(text(rest).unwrap_or("")),
        b"INCOMING" => chat.incoming_call(text(rest).unwrap_or("")),
        b"CALLSTATE" => {
            let mut parts = rest.splitn(3, |&b| b == b'\t');
            match (parts.next(), parts.next(), parts.next()) {
                (Some(b"active"), _, _) => chat.call_connected(),
                (Some(b"failed"), Some(title), Some(body)) => chat.call_failed(text(title).unwrap_or(""), text(body).unwrap_or("")),
                (Some(b"ended"), reason, _) => chat.call_ended(text(reason.unwrap_or(&[])).unwrap_or("")),
                _ => {}
            }
        }
        b"NEWCALL" => {
            let mut parts = rest.splitn(4, |&b| b == b'\t');
            if let (Some(kind), Some(count), Some(time), Some(name)) = (parts.next(), parts.next(), parts.next(), parts.next()) {
                if let (Some(kind), Some(count), Some(time), Some(name)) = (number(kind), number(count), text(time), text(name)) {
                    let kind = match kind {
                        1 => CallKind::Missed,
                        2 => CallKind::Outgoing,
                        _ => CallKind::Received,
                    };
                    chat.add_call_first(kind, count.min(255) as u8, time, name);
                }
            }
        }
        b"PROFILE" => chat.set_profile(text(rest).unwrap_or("")),
        b"FAVORITE" => chat.add_favorite(text(rest).unwrap_or("")),
        b"PRESENCE" => {
            let mut parts = rest.splitn(2, |&b| b == b'\t');
            if let (Some(id), Some(status)) = (parts.next().and_then(number), parts.next().and_then(text)) {
                chat.set_status(id, status);
            }
        }
        b"MSG" => {
            let mut parts = rest.splitn(7, |&b| b == b'\t');
            if let (Some(chat_id), Some(outgoing), Some(read), Some(time), Some(id), Some(reply), Some(body)) =
                (parts.next(), parts.next(), parts.next(), parts.next(), parts.next(), parts.next(), parts.next())
            {
                if let (Some(chat_id), Some(time), Some(id), Some(reply), Some(body)) = (number(chat_id), text(time), text(id), text(reply), text(body)) {
                    chat.receive(chat_id, outgoing == b"1", read == b"1", time, id, reply, body);
                }
            }
        }
        b"PAST" => {
            let mut parts = rest.splitn(7, |&b| b == b'\t');
            if let (Some(chat_id), Some(outgoing), Some(read), Some(time), Some(id), Some(reply), Some(body)) =
                (parts.next(), parts.next(), parts.next(), parts.next(), parts.next(), parts.next(), parts.next())
            {
                if let (Some(chat_id), Some(time), Some(id), Some(reply), Some(body)) = (number(chat_id), text(time), text(id), text(reply), text(body)) {
                    chat.receive_older(chat_id, outgoing == b"1", read == b"1", time, id, reply, body);
                }
            }
        }
        b"MEDIA" => {
            let mut parts = rest.splitn(6, |&b| b == b'\t');
            if let (Some(chat_id), Some(id), Some(kind), Some(width), Some(height), Some(label)) =
                (parts.next().and_then(number), parts.next().and_then(text), parts.next().and_then(number), parts.next().and_then(number), parts.next().and_then(number), parts.next().and_then(text))
            {
                chat.set_media(chat_id, id, kind as u8, width as u32, height as u32, label);
            }
        }
        b"THUMB" => {
            let mut parts = rest.splitn(4, |&b| b == b'\t');
            if let (Some(chat_id), Some(id), Some(width), Some(height)) = (parts.next().and_then(number), parts.next().and_then(text), parts.next().and_then(number), parts.next().and_then(number)) {
                chat.thumb_start(chat_id, id, width, height);
            }
        }
        b"THUMBROW" => {
            let mut parts = rest.splitn(4, |&b| b == b'\t');
            if let (Some(chat_id), Some(id), Some(row), Some(hex)) = (parts.next().and_then(number), parts.next().and_then(text), parts.next().and_then(number), parts.next()) {
                chat.thumb_row(chat_id, id, row, hex);
            }
        }
        b"THUMBFAIL" => {
            let mut parts = rest.splitn(2, |&b| b == b'\t');
            if let (Some(chat_id), Some(id)) = (parts.next().and_then(number), parts.next().and_then(text)) {
                chat.thumb_failed(chat_id, id);
            }
        }
        b"PASTEND" => {
            let mut parts = rest.splitn(2, |&b| b == b'\t');
            if let (Some(chat_id), Some(more)) = (parts.next().and_then(number), parts.next()) {
                chat.older_done(chat_id, more == b"1");
            }
        }
        b"REVOKED" => {
            let mut parts = rest.splitn(3, |&b| b == b'\t');
            if let (Some(chat_id), Some(id), Some(by_me)) = (parts.next().and_then(number), parts.next().and_then(text), parts.next()) {
                chat.mark_deleted(chat_id, id, by_me == b"1");
            }
        }
        b"EDITED" => {
            let mut parts = rest.splitn(3, |&b| b == b'\t');
            if let (Some(chat_id), Some(id), Some(new_text)) = (parts.next().and_then(number), parts.next().and_then(text), parts.next().and_then(text)) {
                chat.set_text(chat_id, id, new_text);
            }
        }
        b"STARRED" => {
            let mut parts = rest.splitn(3, |&b| b == b'\t');
            if let (Some(chat_id), Some(id), Some(flag)) = (parts.next().and_then(number), parts.next().and_then(text), parts.next()) {
                chat.set_starred(chat_id, id, flag == b"1");
            }
        }
        b"REACTIONS" => {
            let mut parts = rest.splitn(4, |&b| b == b'\t');
            if let (Some(chat_id), Some(id), Some(mine), Some(counts)) = (parts.next().and_then(number), parts.next().and_then(text), parts.next().and_then(text), parts.next().and_then(text)) {
                chat.set_reactions(chat_id, id, mine, counts);
            }
        }
        b"SENT" => {
            let mut parts = rest.splitn(2, |&b| b == b'\t');
            if let (Some(chat_id), Some(id)) = (parts.next().and_then(number), parts.next().and_then(text)) {
                chat.set_sent_id(chat_id, id);
            }
        }
        _ => {}
    }
}

/// The cursor for what a point of the window would do when clicked: the I-beam over text boxes, the hand
/// over buttons and rows, the arrow elsewhere.
fn cursor_over(hit: &Hit) -> Cursor {
    match hit {
        Hit::SearchBox(_) | Hit::MessageBox(_) | Hit::Find(FindHit::Box(_)) => Cursor::Text,
        Hit::Nothing | Hit::Conversation | Hit::InfoPanel | Hit::EmojiPanel | Hit::ForwardOutside => Cursor::Arrow,
        _ => Cursor::Pointer,
    }
}

fn draw(c: &mut Canvas, screen: &Screen, notice: &Notice, chat: &Chat) {
    match screen {
        Screen::Notice => view::notice(c, notice.text()),
        Screen::Login(qr) => view::login(c, qr),
        Screen::Chat => view::chat_screen(c, chat),
    }
}

unsafe fn run(envp: *const *const u8) -> ! {
    let (mut width, mut height) = (1100u32, 700u32);
    let mut window = Window::new(envp, b"ZapZap", width, height);
    let chat = &mut *core::ptr::addr_of_mut!(CHAT);
    chat.start(width as i32, height as i32);
    let mut screen = Screen::Notice;
    let mut notice = Notice { bytes: [0; NOTICE_CAPACITY], len: 0 };
    notice.set("Conectando ao WhatsApp...".as_bytes());
    let mut line = [0u8; LINE_CAPACITY];
    let mut line_len = 0;
    let mut stdin_open = true;
    let mut dirty = false;
    // When and where the last click was, and how many in a row it made.
    let mut last_click = (0i64, 0i32, 0i32, 0u8);
    let mut drag_in: Option<Drag> = None;
    loop {
        // A running call redraws once a second for its clock.
        let ticking = chat.call_ticking();
        let blinking = matches!(screen, Screen::Chat) && chat.editing() && chat.window_focused;
        let timeout = if blinking { CARET_BLINK_MS } else if ticking { 1000 } else { -1 };
        let (window_ready, stdin_ready) = sys::wait_readable(window.fd(), if stdin_open { STDIN } else { -1 }, timeout);
        if timeout >= 0 && !window_ready && !stdin_ready {
            if ticking {
                chat.tick();
            }
            if blinking {
                chat.caret_on = !chat.caret_on;
            }
            dirty = true;
        }
        if stdin_ready {
            let mut chunk = [0u8; 256];
            let count = sys::read_some(STDIN, &mut chunk);
            stdin_open = count > 0;
            for &byte in &chunk[..count] {
                if byte != b'\n' {
                    if line_len < LINE_CAPACITY {
                        line[line_len] = byte;
                        line_len += 1;
                    }
                    continue;
                }
                if line_len < LINE_CAPACITY {
                    handle_line(&line[..line_len], &mut screen, &mut notice, chat);
                    dirty = true;
                }
                line_len = 0;
            }
        }
        while window_ready {
            let on_chat = matches!(screen, Screen::Chat);
            match window.next_event() {
                Event::Close => sys::exit(0),
                Event::Redraw => dirty = true,
                Event::Resize(w, h) if (w, h) != (width, height) && w > 0 && h > 0 => {
                    (width, height) = (w, h);
                    chat.resize(w as i32, h as i32);
                    dirty = true;
                }
                Event::Resize(..) => {}
                Event::Key(key) if on_chat => {
                    chat.key(key);
                    if let Some(text) = chat.pending_copy() {
                        window.copy(text);
                    }
                    if chat.take_paste_request() {
                        window.request_paste();
                    }
                    dirty = true;
                }
                Event::Motion { x, y, pressed } => {
                    // Holding the button down after a click in a text box selects as the pointer moves.
                    match (pressed, drag_in) {
                        (true, Some(target)) if on_chat => {
                            match target {
                                Drag::Message => chat.drag_field(true, view::message_caret_at(chat, x, width as i32)),
                                Drag::Search => chat.drag_field(false, view::search_caret_at(chat, x, width as i32, !chat.dialpad_open)),
                                Drag::Find => chat.drag_find(view::find_caret_at(chat, x, width as i32)),
                            }
                            dirty = true;
                        }
                        (false, Some(_)) => drag_in = None,
                        _ => {}
                    }
                    // The reaction button follows the pointer over the messages.
                    let hover = if on_chat && chat.open && chat.tab == Tab::Chats && x >= sidebar_width(width as i32) && !chat.forward_open && !chat.dialog_open && !view::over_info(chat, x, width as i32) {
                        view::hover_at(chat, x, y, width as i32)
                    } else {
                        None
                    };
                    if hover != chat.hover {
                        chat.hover = hover;
                        dirty = true;
                    }
                    // The text cursor over the text boxes, the arrow anywhere else.
                    let cursor = if !on_chat {
                        Cursor::Arrow
                    } else if chat.dialog_open {
                        if view::dialog_hit(x, y, width as i32, height as i32, chat).is_some() { Cursor::Pointer } else { Cursor::Arrow }
                    } else {
                        cursor_over(&view::hit_test(x, y, width as i32, height as i32, chat))
                    };
                    window.show_cursor(cursor);
                }
                Event::RightClick { x, y } if on_chat => {
                    // The pop-up over the message under the pointer (Reply, Copy); a right click elsewhere closes it.
                    chat.message_menu = if chat.open && chat.tab == Tab::Chats && x >= sidebar_width(width as i32) && !view::over_info(chat, x, width as i32) {
                        view::message_at(chat, x, y, width as i32).map(|index| (index, x, y))
                    } else {
                        None
                    };
                    dirty = true;
                }
                Event::Focus(focused) => {
                    chat.window_focused = focused;
                    chat.caret_on = true;
                    dirty = true;
                }
                Event::Paste if on_chat => {
                    chat.paste(window.pasted());
                    dirty = true;
                }
                Event::Click { x, y } if on_chat && chat.dialog_open => {
                    match view::dialog_hit(x, y, width as i32, height as i32, chat) {
                        Some(true) if chat.confirming() => chat.dialog_answer(true),
                        Some(true) => chat.dialog_open = false,
                        Some(false) => chat.dialog_answer(false),
                        None => {}
                    }
                    dirty = true;
                }
                Event::Click { x, y } if on_chat && chat.chat_menu_open => {
                    match view::conversation_menu_entry_at(x, y, width as i32) {
                        Some(0) => chat.info_open = true,
                        Some(1) => chat.toggle_silence(),
                        Some(2) => chat.close_conversation(),
                        Some(3) => chat.ask_clear(),
                        Some(4) => chat.ask_remove(),
                        _ => {}
                    }
                    chat.chat_menu_open = false;
                    dirty = true;
                }
                Event::Click { x, y } if on_chat && chat.menu_open => {
                    if chat.tab == Tab::Chats {
                        match view::chats_menu_entry_at(x, y, chat::sidebar_width(width as i32)) {
                            Some(2) => chat.open_starred(),
                            Some(4) => chat.mark_all_read(),
                            _ => {}
                        }
                    }
                    chat.menu_open = false;
                    dirty = true;
                }
                Event::Click { x, y } if on_chat => {
                    // Clicks close together in time and place count as a double or triple click.
                    let now = sys::monotonic_ms();
                    let (at, last_x, last_y, before) = last_click;
                    let clicks = if now - at <= MULTI_CLICK_MS && (x - last_x).abs() <= 4 && (y - last_y).abs() <= 4 { (before + 1).min(3) } else { 1 };
                    last_click = (now, x, y, clicks);
                    drag_in = None;
                    chat.flash = None;
                    let mut keep_menu = false;
                    match view::hit_test(x, y, width as i32, height as i32, chat) {
                        Hit::Emoji(index) if chat.picker_reaction.is_some() => chat.react_with_picker(index),
                        Hit::Emoji(index) => chat.insert_emoji(index),
                        Hit::EmojiPanel => {}
                        Hit::EmojiButton => chat.toggle_picker(),
                        other => {
                            chat.close_picker();
                            match other {
                                Hit::Tab(tab) => chat.set_tab(tab),
                                Hit::NewChat => chat.open_new_chat(),
                                Hit::Menu => chat.menu_open = true,
                                Hit::ChatMenu => chat.chat_menu_open = true,
                                Hit::HeaderInfo => chat.info_open = true,
                                Hit::InfoClose => chat.info_open = false,
                                Hit::InfoRow(0) => chat.open_starred(),
                                Hit::InfoRow(1) => chat.toggle_silence(),
                                Hit::InfoRow(2) => chat.ask_clear(),
                                Hit::InfoRow(_) => chat.ask_remove(),
                                Hit::InfoPanel => {}
                                Hit::OpenDialpad => chat.open_dialpad(),
                                Hit::Key(key) => chat.dial_key(key),
                                Hit::Answer => chat.answer_call(),
                                Hit::Decline | Hit::HangUp => chat.hang_up(),
                                Hit::Mute => chat.muted = !chat.muted,
                                Hit::Call(index) => chat.call_favorite(index),
                                Hit::Back if chat.dialpad_open => chat.close_dialpad(),
                                Hit::Back if chat.starred_open => chat.close_starred(),
                                Hit::Starred(nth) => chat.open_starred_message(nth),
                                Hit::Back => chat.close_new_chat(),
                                Hit::Contact(index) => chat.pick_contact(index),
                                Hit::SelfChat => chat.pick_self(),
                                Hit::NumberChat => chat.pick_number(),
                                Hit::Row(index) => chat.select(index),
                                Hit::Chip(index) => chat.set_unread_only(index == 1),
                                Hit::SearchBox(x) => {
                                    chat.focus = Focus::Search;
                                    let icon = !chat.dialpad_open;
                                    chat.click_field(false, view::search_caret_at(chat, x, width as i32, icon), clicks);
                                    drag_in = Some(Drag::Search);
                                }
                                Hit::MessageBox(x) => {
                                    chat.focus = Focus::Message;
                                    chat.click_field(true, view::message_caret_at(chat, x, width as i32), clicks);
                                    drag_in = Some(Drag::Message);
                                }
                                Hit::FindOpen => chat.open_find(),
                                Hit::ScrollBottom => chat.scroll_to_bottom(),
                                Hit::ReplyClose => chat.cancel_reply(),
                                Hit::Quote(index) => chat.jump_to_message(index),
                                Hit::Reaction(slot) => {
                                    if let Some((index, _, _)) = chat.message_menu.take() {
                                        chat.react(index, chat::QUICK_REACTIONS[slot]);
                                    }
                                }
                                Hit::MenuReply => {
                                    if let Some((index, _, _)) = chat.message_menu.take() {
                                        chat.start_reply(index);
                                    }
                                }
                                Hit::ReactionMore => {
                                    if let Some((index, _, _)) = chat.message_menu.take() {
                                        chat.open_reaction_picker(index);
                                    }
                                }
                                Hit::HoverButton(index) => {
                                    let (mx, my) = view::hover_button_menu_at(chat, index, width as i32);
                                    chat.message_menu = Some((index, mx, my));
                                    keep_menu = true;
                                }
                                Hit::MenuForward => {
                                    if let Some((index, _, _)) = chat.message_menu.take() {
                                        chat.open_forward(index);
                                    }
                                }
                                Hit::ForwardTo(target) => chat.forward_to(target),
                                Hit::ForwardOutside => chat.forward_open = false,
                                Hit::MenuStar => {
                                    if let Some((index, _, _)) = chat.message_menu.take() {
                                        chat.toggle_star(index);
                                    }
                                }
                                Hit::MenuEdit => {
                                    if let Some((index, _, _)) = chat.message_menu.take() {
                                        chat.start_edit(index);
                                    }
                                }
                                Hit::MenuDelete => {
                                    if let Some((index, _, _)) = chat.message_menu.take() {
                                        chat.ask_delete(index);
                                    }
                                }
                                Hit::MenuCopy => {
                                    if let Some((index, _, _)) = chat.message_menu.take() {
                                        chat.copy_message(index);
                                    }
                                }
                                Hit::Find(FindHit::Box(x)) => {
                                    chat.click_find(view::find_caret_at(chat, x, width as i32), clicks);
                                    drag_in = Some(Drag::Find);
                                }
                                Hit::Find(FindHit::Up) => chat.find_step(false),
                                Hit::Find(FindHit::Down) => chat.find_step(true),
                                Hit::Find(FindHit::Close) => chat.close_find(),
                                Hit::Conversation => chat.focus = Focus::Message,
                                Hit::Send => chat.send(),
                                _ => {}
                            }
                        }
                    }
                    // Any click closes the pop-up over a message, after what it chose has been done.
                    if !keep_menu {
                        chat.message_menu = None;
                    }
                    if let Some(text) = chat.pending_copy() {
                        window.copy(text);
                    }
                    dirty = true;
                }
                Event::Scroll { x, y, down } if on_chat => {
                    let notches = if down { 1 } else { -1 };
                    if chat.forward_open {
                        chat.scroll_forward(notches);
                    } else if chat.picker_open && view::in_picker(x, y, width as i32, height as i32) {
                        chat.scroll_picker(notches);
                    } else if x < sidebar_width(width as i32) {
                        match chat.tab {
                            Tab::Chats if !chat.new_chat && !chat.starred_open => chat.scroll_list(notches),
                            _ => chat.scroll_panel(notches),
                        }
                    } else {
                        chat.scroll_conversation(notches);
                    }
                    dirty = true;
                }
                Event::Key(_) | Event::Click { .. } | Event::RightClick { .. } | Event::Scroll { .. } | Event::Paste => {}
            }
            if !window.has_pending() {
                break;
            }
        }
        chat.request_older_if_top();
        chat.request_thumbs();
        if dirty {
            let drawn_width = (width as usize).min(MAX_WIDTH);
            let max_rows = window.max_rows(drawn_width as u32).min(BAND_ROWS);
            let band = &mut *core::ptr::addr_of_mut!(BAND);
            for y0 in (0..height as usize).step_by(max_rows) {
                let rows = max_rows.min(height as usize - y0);
                let pixels = &mut band[..drawn_width * rows];
                let mut canvas = Canvas { px: pixels, w: drawn_width as i32, h: height as i32, y0: y0 as i32, rows: rows as i32 };
                draw(&mut canvas, &screen, &notice, chat);
                window.present(&band[..drawn_width * rows], drawn_width as u32, y0 as u32, rows as u32);
            }
            dirty = false;
            sys::drop_idle_file_pages();
        }
    }
}
