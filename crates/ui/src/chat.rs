//! What the chat screen knows: the conversations, their messages, the two text fields and the
//! scroll offsets. All in fixed-size arrays; nothing here allocates.

use crate::font;
use crate::platform::{sys, EditKey, Key};

pub const MAX_CHATS: usize = 48;
const MAX_MESSAGES: usize = 384;
const FIND_CAPACITY: usize = 48;
/// WhatsApp's message ids are shorter than this; a longer one is cut.
/// Sends `head`, the two-digit number of `chat` and `tail` to the core.
fn write_command(head: &[u8], chat: usize, tail: &[u8]) {
    let mut line = [0u8; 32];
    let mut at = 0;
    for part in [head, &[b'0' + (chat / 10) as u8, b'0' + (chat % 10) as u8], tail] {
        line[at..at + part.len()].copy_from_slice(part);
        at += part.len();
    }
    sys::write_all(1, &line[..at]);
}

/// Kinds of emoji a message shows in its reaction pill.
pub const TALLY: usize = 3;
/// Kinds of what a message carries, as the `MEDIA` line numbers them.
pub const MEDIA_IMAGE: u8 = 1;
pub const MEDIA_VIDEO: u8 = 2;
pub const MEDIA_AUDIO: u8 = 3;
pub const MEDIA_DOCUMENT: u8 = 4;
pub const MEDIA_STICKER: u8 = 5;
const LABEL_CAPACITY: usize = 40;
/// Width of a photo's box, and the bounds of its height; voice and document boxes have fixed sizes.
const MEDIA_W: i32 = 240;
const MEDIA_MIN_H: i32 = 90;
const MEDIA_MAX_H: i32 = 300;
const AUDIO_H: i32 = 44;
const DOCUMENT_H: i32 = 60;
/// Space between a message's box and its text.
pub const BLOCK_GAP: i32 = 4;
/// Preview pictures kept at a time, each at most this many pixels on a side.
pub const THUMB_SLOTS: usize = 6;
pub const THUMB_SIDE: usize = 48;

/// Height of a row of the favourite messages panel.
pub const STARRED_ROW_H: i32 = 76;
pub const ID_CAPACITY: usize = 24;
/// Height a quoted message adds to a bubble.
pub const QUOTE_H: i32 = 44;
/// Height of the bar over the footer while a reply is being written.
pub const REPLY_BAR_H: i32 = 60;
/// Room a bubble needs under it for its reactions pill.
pub const REACTION_H: i32 = 14;
/// The list of conversations to forward to: row height and the visible height of the list.
pub const FORWARD_ROW_H: i32 = 56;
pub const FORWARD_LIST_H: i32 = 336;
/// The emoji offered to react with.
pub const QUICK_REACTIONS: [char; 6] = ['👍', '❤', '😂', '😮', '😢', '🙏'];
const HISTORY_RANK: u32 = 1_000_000;
const LIVE_RANK: u32 = 2_000_000;
const MESSAGE_CAPACITY: usize = 240;
const SEARCH_CAPACITY: usize = 48;
const NAME_CAPACITY: usize = 40;
const STATUS_CAPACITY: usize = 48;
const TIME_CAPACITY: usize = 20;
const BANNER_CAPACITY: usize = 96;
pub const MAX_CALLS: usize = 24;
pub const MAX_FAVORITES: usize = 6;
pub const MAX_STORIES: usize = 8;
pub const MAX_CHANNELS: usize = 6;
pub const MAX_SUGGESTIONS: usize = 6;
pub const MAX_CONTACTS: usize = 40;
const TEXT_CAPACITY: usize = 96;

// Geometry shared by the layout here and the drawing in view.rs.
pub const RAIL_W: i32 = 64;
pub const HEADER_H: i32 = 64;
pub const ROW_H: i32 = 72;
pub const INPUT_H: i32 = 64;
/// Below the header, the search box and the filter chips.
pub const LIST_TOP: i32 = 162;
const LIST_SCROLL_STEP: i32 = 36;
const CONVERSATION_SCROLL_STEP: i32 = 60;

// The emoji picker: a grid of cells over the conversation, scrolled a row at a time.
pub const PICKER_COLS: usize = 8;
pub const PICKER_ROWS: usize = 6;
pub const PICKER_CELL: i32 = 34;
pub const PICKER_PAD: i32 = 8;

/// Message text, as WhatsApp Web sets it (14.2 px in the page, from memory, rounded here).
pub const BODY: u16 = 14;
pub const META: u16 = 12;
pub const LINE_HEIGHT: i32 = 19;
pub const BUBBLE_PAD_X: i32 = 12;
pub const BUBBLE_PAD_Y: i32 = 8;
const META_ROW: i32 = 15;
const META_GAP: i32 = 8;
pub const TICKS_WIDTH: i32 = 18;
/// Room above the first bubble for the date chip.
pub const CONVERSATION_TOP: i32 = 52;
const CONVERSATION_BOTTOM: i32 = 16;
/// The row a day chip takes above the first message of a day (chip 26 px high, 14 above and 12 below).
pub const DAY_CHIP_ROW: i32 = 38;
const SAME_SENDER_GAP: i32 = 3;
const OTHER_SENDER_GAP: i32 = 12;
/// Brazil is one time zone for practical purposes: Brasilia time, UTC-3, no daylight saving.
const BRASILIA_OFFSET_SECONDS: i64 = -3 * 3600;

// The calls list: a header, the favourites, a header, the recent calls; all scroll together.
/// Where the content of the calls list starts when it is not scrolled.
pub const CALLS_TOP: i32 = 119;
pub const CALLS_ROW_H: i32 = 76;
pub const FAVORITES_HEADER_H: i32 = 56;
pub const RECENTS_HEADER_H: i32 = 61;
/// Below the header and the search box; nothing of the list shows above this.
pub const CALLS_VIEW_TOP: i32 = 107;

/// The screens of the icon rail.
/// Where a voice call stands.
#[derive(Clone, Copy, PartialEq)]
pub enum CallPhase {
    Idle,
    /// Someone is calling us.
    Incoming,
    /// We are calling and the other side has not picked up.
    Ringing,
    Active,
    /// Over: the screen stays a moment with the reason (no answer, busy, declined) before it closes.
    Ended,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Tab {
    Chats,
    Calls,
    Status,
    Channels,
    Settings,
}

// The status screen: no search box, the list starts under the header.
pub const STATUS_LABEL_H: i32 = 72;
pub const STATUS_ROW_H: i32 = 72;
// The channels screen.
pub const CHANNEL_ROW_H: i32 = 76;
pub const SUGGESTIONS_HEADER_H: i32 = 43;
pub const SUGGESTION_ROW_H: i32 = 72;
pub const DISCOVER_BAR_H: i32 = 70;
// The settings screen: the profile photo, then one row per entry.
pub const SETTINGS_PHOTO_TOP: i32 = 180;
pub const SETTINGS_ROWS_TOP: i32 = 363;
pub const SETTINGS_ROW_H: i32 = 68;
pub const SETTINGS_LOGOUT_H: i32 = 60;
pub const SETTINGS_ENTRIES: [(&str, &str); 7] = [
    ("Perfil", "Nome, foto do perfil, nome de usuário"),
    ("Conta", "Notificações de segurança, dados da conta"),
    ("Privacidade", "Contatos bloqueados, mensagens temporárias"),
    ("Conversas", "Tema, papel de parede, configurações de conversas"),
    ("Notificações", "Mensagens, grupos, sons"),
    ("Atalhos do teclado", "Ações rápidas"),
    ("Ajuda e feedback", "Central de Ajuda, fale conosco, Política de Privacidade"),
];

/// A line of one of the list screens: a name, a second line of text, a time, and two numbers whose
/// meaning belongs to the screen (segments and unseen for a status, unread for a channel).
pub struct Item {
    pub first: u16,
    pub second: u16,
    pub flag: bool,
    name: [u8; NAME_CAPACITY],
    name_len: u8,
    text: [u8; TEXT_CAPACITY],
    text_len: u8,
    time: [u8; TIME_CAPACITY],
    time_len: u8,
}

impl Item {
    const EMPTY: Item = Item {
        first: 0,
        second: 0,
        flag: false,
        name: [0; NAME_CAPACITY],
        name_len: 0,
        text: [0; TEXT_CAPACITY],
        text_len: 0,
        time: [0; TIME_CAPACITY],
        time_len: 0,
    };

    pub fn name(&self) -> &str {
        core::str::from_utf8(&self.name[..self.name_len as usize]).unwrap_or("")
    }

    pub fn text(&self) -> &str {
        core::str::from_utf8(&self.text[..self.text_len as usize]).unwrap_or("")
    }

    pub fn time(&self) -> &str {
        core::str::from_utf8(&self.time[..self.time_len as usize]).unwrap_or("")
    }

    fn set(&mut self, first: u16, second: u16, flag: bool, time: &str, name: &str, text: &str) {
        (self.first, self.second, self.flag) = (first, second, flag);
        store(&mut self.time, &mut self.time_len, time);
        store(&mut self.name, &mut self.name_len, name);
        store(&mut self.text, &mut self.text_len, text);
    }
}

/// The rows of the status screen, in drawing order.
#[derive(Clone, Copy)]
pub enum StatusItem {
    Mine,
    RecentLabel,
    SeenLabel,
    Story(usize),
}

// The new-conversation panel: three actions, the account itself, then the contacts by initial.
pub const NEW_CHAT_TOP: i32 = 122;
pub const NEW_CHAT_ACTION_H: i32 = 64;
pub const NEW_CHAT_ROW_H: i32 = 72;
pub const NEW_CHAT_SELF_TOP: i32 = 323;
/// The name of the conversation with ourselves.
pub const SELF_CHAT: &str = "Você";
pub const NEW_CHAT_ACTIONS: [&str; 3] = ["Novo grupo", "Novo contato", "Nova comunidade"];

/// The rows of the new-conversation panel, in drawing order.
#[derive(Clone, Copy)]
pub enum ContactsItem {
    Action(usize),
    Myself,
    Letter(char),
    Contact(usize),
    /// The number typed in the search box, as a conversation to start.
    Number,
}

/// The most digits a phone number can have (E.164).
pub const PHONE_DIGITS: usize = 15;

/// The digits of `text` when it reads as a phone number (digits, an optional plus, spaces, dashes and
/// parentheses, at least eight digits); how many were written into `digits`.
pub fn phone_digits(text: &str, digits: &mut [u8; PHONE_DIGITS]) -> Option<usize> {
    let mut count = 0;
    for (index, c) in text.chars().enumerate() {
        match c {
            '0'..='9' if count < PHONE_DIGITS => {
                digits[count] = c as u8;
                count += 1;
            }
            '+' if index == 0 => {}
            ' ' | '-' | '(' | ')' => {}
            _ => return None,
        }
    }
    (count >= 8).then_some(count)
}

/// The letter a contact is filed under: its initial without accent, or `#`.
pub fn initial_of(name: &str) -> char {
    match name.chars().next().map(fold) {
        Some(c) if c.is_ascii_lowercase() => c.to_ascii_uppercase(),
        _ => '#',
    }
}

/// The rows of the channels screen, in drawing order.
#[derive(Clone, Copy)]
pub enum ChannelsItem {
    Followed(usize),
    SuggestionsHeader,
    Suggestion(usize),
    DiscoverBar,
}

#[derive(Clone, Copy, PartialEq)]
pub enum CallKind {
    Received,
    Missed,
    Outgoing,
}

/// One row of the calls screen, in the order it is drawn.
#[derive(Clone, Copy)]
pub enum CallsItem {
    FavoritesHeader,
    Favorite(usize),
    RecentsHeader,
    Recent(usize),
}

/// Text copied into a fixed array, cut at a character boundary if it does not fit.
fn store<const N: usize>(destination: &mut [u8; N], length: &mut u8, text: &str) {
    let mut end = text.len().min(N);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    destination[..end].copy_from_slice(&text.as_bytes()[..end]);
    *length = end as u8;
}

/// One entry of the conversation list, as the core described it.
pub struct Conversation {
    used: bool,
    /// Where it sits in the list: higher is nearer the top. Conversations that come with the history
    /// keep the order they arrived in; any activity after that puts one on top.
    rank: u32,
    pub unread: u8,
    name: [u8; NAME_CAPACITY],
    name_len: u8,
    status: [u8; STATUS_CAPACITY],
    status_len: u8,
    time: [u8; TIME_CAPACITY],
    time_len: u8,
    /// What was being typed when the user left this conversation; the open one keeps it in `Chat::draft`.
    draft: [u8; MESSAGE_CAPACITY],
    draft_len: u8,
    /// Notifications are off for it.
    pub silenced: bool,
}

impl Conversation {
    const EMPTY: Conversation =
        Conversation { used: false, rank: 0, unread: 0, name: [0; NAME_CAPACITY], name_len: 0, status: [0; STATUS_CAPACITY], status_len: 0, time: [0; TIME_CAPACITY], time_len: 0, draft: [0; MESSAGE_CAPACITY], draft_len: 0, silenced: false };

    pub fn name(&self) -> &str {
        core::str::from_utf8(&self.name[..self.name_len as usize]).unwrap_or("")
    }

    pub fn status(&self) -> &str {
        core::str::from_utf8(&self.status[..self.status_len as usize]).unwrap_or("")
    }

    pub fn time(&self) -> &str {
        core::str::from_utf8(&self.time[..self.time_len as usize]).unwrap_or("")
    }
}

/// A recent call, or (with `kind` unused) a favourite contact.
pub struct Call {
    pub kind: CallKind,
    pub count: u8,
    name: [u8; NAME_CAPACITY],
    name_len: u8,
    time: [u8; TIME_CAPACITY],
    time_len: u8,
}

impl Call {
    const EMPTY: Call = Call { kind: CallKind::Received, count: 1, name: [0; NAME_CAPACITY], name_len: 0, time: [0; TIME_CAPACITY], time_len: 0 };

    pub fn name(&self) -> &str {
        core::str::from_utf8(&self.name[..self.name_len as usize]).unwrap_or("")
    }

    pub fn time(&self) -> &str {
        core::str::from_utf8(&self.time[..self.time_len as usize]).unwrap_or("")
    }
}

/// A text box with a fixed capacity; typing past it is ignored. It has a caret, and a selection when
/// the anchor is somewhere else than the caret. Positions are byte offsets on character boundaries.
pub struct Field<const N: usize> {
    bytes: [u8; N],
    len: usize,
    cursor: usize,
    anchor: usize,
}

/// A character that belongs to a word for the purposes of moving and deleting by word.
fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

impl<const N: usize> Field<N> {
    const fn new() -> Self {
        Self { bytes: [0; N], len: 0, cursor: 0, anchor: 0 }
    }

    pub fn text(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// The selected range (start, end); empty when nothing is selected.
    pub fn selection(&self) -> (usize, usize) {
        (self.cursor.min(self.anchor), self.cursor.max(self.anchor))
    }

    pub fn selected_text(&self) -> &str {
        let (start, end) = self.selection();
        &self.text()[start..end]
    }

    /// Replaces the whole content with `text` (cut to fit), the caret at its end.
    fn set(&mut self, text: &[u8]) {
        let mut len = text.len().min(N);
        while len > 0 && len < text.len() && text[len] & 0xc0 == 0x80 {
            len -= 1;
        }
        self.bytes[..len].copy_from_slice(&text[..len]);
        self.len = len;
        self.cursor = len;
        self.anchor = len;
    }

    fn previous_boundary(&self, mut at: usize) -> usize {
        while at > 0 {
            at -= 1;
            if self.bytes[at] & 0xc0 != 0x80 {
                break;
            }
        }
        at
    }

    fn next_boundary(&self, mut at: usize) -> usize {
        while at < self.len {
            at += 1;
            if at == self.len || self.bytes[at] & 0xc0 != 0x80 {
                break;
            }
        }
        at
    }

    /// Start of the word before `at`: skips spaces and punctuation, then the word itself.
    fn word_before(&self, at: usize) -> usize {
        let text = self.text();
        let mut start = at;
        let mut in_word = false;
        for (index, c) in text[..at].char_indices().rev() {
            if is_word(c) {
                in_word = true;
                start = index;
            } else if in_word {
                break;
            } else {
                start = index;
            }
        }
        start
    }

    /// End of the word after `at`.
    fn word_after(&self, at: usize) -> usize {
        let text = self.text();
        let mut end = at;
        let mut in_word = false;
        for (index, c) in text[at..].char_indices() {
            if is_word(c) {
                in_word = true;
                end = at + index + c.len_utf8();
            } else if in_word {
                break;
            } else {
                end = at + index + c.len_utf8();
            }
        }
        end
    }

    /// Removes `start..end`; the caret lands at `start`.
    fn remove(&mut self, start: usize, end: usize) {
        self.bytes.copy_within(end..self.len, start);
        self.len -= end - start;
        self.cursor = start;
        self.anchor = start;
    }

    fn remove_selection(&mut self) -> bool {
        let (start, end) = self.selection();
        if start == end {
            return false;
        }
        self.remove(start, end);
        true
    }

    /// Types `c` at the caret, over the selection if there is one.
    fn push(&mut self, c: char) {
        let mut utf8 = [0u8; 4];
        self.insert_bytes(c.encode_utf8(&mut utf8).as_bytes());
    }

    fn insert_bytes(&mut self, encoded: &[u8]) {
        self.remove_selection();
        if self.len + encoded.len() <= N {
            self.bytes.copy_within(self.cursor..self.len, self.cursor + encoded.len());
            self.bytes[self.cursor..self.cursor + encoded.len()].copy_from_slice(encoded);
            self.len += encoded.len();
            self.cursor += encoded.len();
            self.anchor = self.cursor;
        }
    }

    /// Backspace: the selection, else the character before the caret.
    fn pop(&mut self) {
        if !self.remove_selection() && self.cursor > 0 {
            self.remove(self.previous_boundary(self.cursor), self.cursor);
        }
    }

    fn delete_forward(&mut self) {
        if !self.remove_selection() && self.cursor < self.len {
            self.remove(self.cursor, self.next_boundary(self.cursor));
        }
    }

    fn delete_word_back(&mut self) {
        if !self.remove_selection() {
            self.remove(self.word_before(self.cursor), self.cursor);
        }
    }

    fn delete_word_forward(&mut self) {
        if !self.remove_selection() {
            self.remove(self.cursor, self.word_after(self.cursor));
        }
    }

    fn delete_to_start(&mut self) {
        if !self.remove_selection() {
            self.remove(0, self.cursor);
        }
    }

    fn delete_to_end(&mut self) {
        if !self.remove_selection() {
            self.remove(self.cursor, self.len);
        }
    }

    /// Moves the caret; with `extend` the anchor stays and the selection grows, else it collapses.
    pub fn move_to(&mut self, at: usize, extend: bool) {
        self.cursor = at.min(self.len);
        if !extend {
            self.anchor = self.cursor;
        }
    }

    /// Selects the run of like characters around `at`: a word, a stretch of spaces, or of punctuation.
    fn select_word(&mut self, at: usize) {
        let text = self.text();
        let class = |c: char| if is_word(c) { 0 } else if c.is_whitespace() { 1 } else { 2 };
        let at = at.min(self.len);
        let Some(reference) = text[at..].chars().next().or_else(|| text[..at].chars().next_back()) else { return };
        let kind = class(reference);
        let start = text[..at].char_indices().rev().take_while(|&(_, c)| class(c) == kind).last().map_or(at, |(index, _)| index);
        let end = text[at..].char_indices().find(|&(_, c)| class(c) != kind).map_or(self.len, |(index, _)| at + index);
        // At the very end of the text the run before `at` is the one selected, and `end` is `at`.
        self.anchor = start;
        self.cursor = end;
    }

    fn select_all(&mut self) {
        self.anchor = 0;
        self.cursor = self.len;
    }

    fn clear(&mut self) {
        self.len = 0;
        self.cursor = 0;
        self.anchor = 0;
    }
}

/// How many editing steps Ctrl+Z goes back through.
const UNDO_DEPTH: usize = 6;

/// The message box at one moment: what Ctrl+Z and Ctrl+Shift+Z go between.
#[derive(Clone, Copy)]
struct Snap {
    bytes: [u8; MESSAGE_CAPACITY],
    len: u16,
    cursor: u16,
    anchor: u16,
}

impl Snap {
    const EMPTY: Snap = Snap { bytes: [0; MESSAGE_CAPACITY], len: 0, cursor: 0, anchor: 0 };
}

/// What the last edit of the message box was, to group typing into one undo step.
#[derive(Clone, Copy, PartialEq)]
enum Change {
    /// Typing letters in a row: one step until something else happens or a space ends the word.
    Type,
    Other,
}

/// What the editing keys do to a text box.
#[derive(Clone, Copy)]
enum Edit {
    Left { word: bool, extend: bool },
    Right { word: bool, extend: bool },
    Home { extend: bool },
    End { extend: bool },
    SelectAll,
    DeleteForward,
    DeleteWordBack,
    DeleteWordForward,
    DeleteToStart,
    DeleteToEnd,
    Backspace,
}

fn apply_edit<const N: usize>(field: &mut Field<N>, edit: Edit) {
    let (start, end) = field.selection();
    match edit {
        Edit::Left { word, extend } => {
            let to = if !extend && start != end { start } else if word { field.word_before(field.cursor) } else { field.previous_boundary(field.cursor) };
            field.move_to(to, extend);
        }
        Edit::Right { word, extend } => {
            let to = if !extend && start != end { end } else if word { field.word_after(field.cursor) } else { field.next_boundary(field.cursor) };
            field.move_to(to, extend);
        }
        Edit::Home { extend } => field.move_to(0, extend),
        Edit::End { extend } => field.move_to(field.len, extend),
        Edit::SelectAll => field.select_all(),
        Edit::DeleteForward => field.delete_forward(),
        Edit::DeleteWordBack => field.delete_word_back(),
        Edit::DeleteWordForward => field.delete_word_forward(),
        Edit::DeleteToStart => field.delete_to_start(),
        Edit::DeleteToEnd => field.delete_to_end(),
        Edit::Backspace => field.pop(),
    }
}

/// How far a message we sent has got: one grey tick, two grey ticks, two blue ticks.
#[derive(Clone, Copy, PartialEq)]
pub enum Delivery {
    Sent,
    Delivered,
    Read,
}

pub struct Message {
    pub chat: u8,
    pub outgoing: bool,
    pub delivery: Delivery,
    time: [u8; 5],
    len: u16,
    /// Height of the bubble at the current window width; see `Chat::resize`.
    height: u16,
    text: [u8; MESSAGE_CAPACITY],
    /// The connection's id for the message (empty until it is known), and the id of the message it answers.
    id: [u8; ID_CAPACITY],
    id_len: u8,
    reply: [u8; ID_CAPACITY],
    reply_len: u8,
    /// The emoji put on it, most used first, with how many people used each (0: no emoji), shown as a small pill
    /// under the bubble; `mine` is the one we put.
    pub tally: [u32; TALLY],
    pub counts: [u8; TALLY],
    pub mine: u32,
    /// Taken back by its sender: the text says so and nothing can be done with it.
    pub deleted: bool,
    /// Marked as a favourite: a small star before the time.
    pub starred: bool,
    /// The calendar day it was sent, in days since 1970-01-01 (Brasilia time).
    pub day: u16,
    /// What it carries besides text (`MEDIA_*`), the picture's size, and a short label (a file name or a duration).
    pub kind: u8,
    media_w: u16,
    media_h: u16,
    label: [u8; LABEL_CAPACITY],
    label_len: u8,
    /// The preview picture: 0 not asked for, 1 asked, 2 in `slot` of the pool, 3 there is none.
    pub thumb: u8,
    slot: u8,
}

impl Message {
    const EMPTY: Message = Message { chat: 0, outgoing: false, delivery: Delivery::Delivered, time: [b'0'; 5], len: 0, height: 0, text: [0; MESSAGE_CAPACITY], id: [0; ID_CAPACITY], id_len: 0, reply: [0; ID_CAPACITY], reply_len: 0, tally: [0; TALLY], counts: [0; TALLY], mine: 0, deleted: false, starred: false, day: 0, kind: 0, media_w: 0, media_h: 0, label: [0; LABEL_CAPACITY], label_len: 0, thumb: 0, slot: 0 };

    pub fn text(&self) -> &str {
        core::str::from_utf8(&self.text[..self.len as usize]).unwrap_or("")
    }

    pub fn label(&self) -> &str {
        core::str::from_utf8(&self.label[..self.label_len as usize]).unwrap_or("")
    }

    /// The size of the box that shows what the message carries: (width, height), (0, 0) for none.
    pub fn block(&self) -> (i32, i32) {
        match self.kind {
            MEDIA_IMAGE | MEDIA_VIDEO | MEDIA_STICKER => {
                let (w, h) = (self.media_w as i32, self.media_h as i32);
                let height = if w > 0 && h > 0 { MEDIA_W * h / w } else { MEDIA_W * 3 / 4 };
                (MEDIA_W, height.clamp(MEDIA_MIN_H, MEDIA_MAX_H))
            }
            MEDIA_AUDIO => (MEDIA_W - 10, AUDIO_H),
            MEDIA_DOCUMENT => (MEDIA_W - 10, DOCUMENT_H),
            _ => (0, 0),
        }
    }

    /// What the conversation list shows for a message without text.
    pub fn kind_word(&self) -> &'static str {
        match self.kind {
            MEDIA_IMAGE => "Foto",
            MEDIA_VIDEO => "Vídeo",
            MEDIA_AUDIO => "Mensagem de voz",
            MEDIA_DOCUMENT => "Documento",
            MEDIA_STICKER => "Figurinha",
            _ => "",
        }
    }

    /// Takes our emoji out of the tally.
    fn take_back_mine(&mut self) {
        if let Some(slot) = self.tally.iter().position(|&code| code != 0 && code == self.mine) {
            self.counts[slot] = self.counts[slot].saturating_sub(1);
            if self.counts[slot] == 0 {
                self.tally.copy_within(slot + 1.., slot);
                self.counts.copy_within(slot + 1.., slot);
                self.tally[TALLY - 1] = 0;
                self.counts[TALLY - 1] = 0;
            }
        }
        self.mine = 0;
    }

    /// Puts our emoji in the tally (a new one takes the last place when the pill is full).
    fn add_mine(&mut self, code: u32) {
        self.mine = code;
        let slot = self.tally.iter().position(|&emoji| emoji == code || emoji == 0).unwrap_or(TALLY - 1);
        if self.tally[slot] != code {
            self.tally[slot] = code;
            self.counts[slot] = 0;
        }
        self.counts[slot] = self.counts[slot].saturating_add(1);
    }

    pub fn time(&self) -> &str {
        core::str::from_utf8(&self.time).unwrap_or("")
    }

    pub fn id(&self) -> &str {
        core::str::from_utf8(&self.id[..self.id_len as usize]).unwrap_or("")
    }

    pub fn reply_id(&self) -> &str {
        core::str::from_utf8(&self.reply[..self.reply_len as usize]).unwrap_or("")
    }
}

/// `id` cut to fit `ID_CAPACITY` (ids are ASCII).
fn store_id(destination: &mut [u8; ID_CAPACITY], length: &mut u8, id: &str) {
    let len = id.len().min(ID_CAPACITY);
    destination[..len].copy_from_slice(&id.as_bytes()[..len]);
    *length = len as u8;
}

#[derive(Clone, Copy, PartialEq)]
pub enum Focus {
    Message,
    Search,
    /// The bar that looks for text in the open conversation.
    Find,
}

pub struct Chat {
    pub tab: Tab,
    pub panel_scroll: i32,
    /// The account name shown at the top of the settings screen.
    profile: Field<NAME_CAPACITY>,
    /// Whether the new-conversation panel replaces the conversation list.
    pub new_chat: bool,
    /// Whether the three-dots menu of the panel header is showing.
    pub menu_open: bool,
    /// The panel listing the favourite messages replaces the conversation list.
    pub starred_open: bool,
    /// Whether the bar that searches the open conversation is showing, what it looks for, and which
    /// of the matches (oldest first) is the current one.
    pub find_open: bool,
    find: Field<FIND_CAPACITY>,
    find_current: usize,
    /// Messages that arrived in the open conversation while it was scrolled up (the button that jumps down counts them).
    pub below_count: u8,
    /// The message the pointer is over (its reaction button shows), and the message an open emoji picker reacts to.
    pub hover: Option<usize>,
    /// Waiting for older messages of the open conversation (`OLDER`): when it was asked (0: not waiting) and
    /// how tall the conversation was then. `exhausted` marks the conversations that have no more.
    older_asked: i64,
    older_content: i32,
    exhausted: u64,
    /// The pool of preview pictures (RGB565, `THUMB_SIDE` wide rows), their sizes, and the next slot to reuse.
    thumbs: [[u16; THUMB_SIDE * THUMB_SIDE]; THUMB_SLOTS],
    thumb_size: [(u8, u8); THUMB_SLOTS],
    thumb_next: u8,
    /// When the preview asked for last was asked (0: none is pending).
    thumb_asked: i64,
    pub picker_reaction: Option<usize>,
    /// The message marked after a jump from a quote, until the next key, click or scroll.
    pub flash: Option<usize>,
    /// The message the next one answers (its id), while a reply is being written.
    reply_to: [u8; ID_CAPACITY],
    reply_to_len: u8,
    /// The message named by `reply_to` is being edited: its text is in the box and sending replaces it.
    pub editing: bool,
    /// The pop-up over a message opened with the right button: (message number, x, y).
    pub message_menu: Option<(usize, i32, i32)>,
    /// The caret is in its visible half of the blink; any key or click makes it visible again.
    pub caret_on: bool,
    /// The window has the keyboard focus (no caret, no blinking, while it does not).
    pub window_focused: bool,
    /// Earlier states of the message being typed, oldest first, for Ctrl+Z; and the ones undone, for Ctrl+Shift+Z.
    undo_states: [Snap; UNDO_DEPTH],
    undo_len: usize,
    redo_states: [Snap; UNDO_DEPTH],
    redo_len: usize,
    last_change: Change,
    /// Text copied with Ctrl+C or Ctrl+X, waiting for the window to put it on the clipboard.
    clip: [u8; MESSAGE_CAPACITY],
    clip_len: usize,
    clip_pending: bool,
    paste_wanted: bool,
    /// The phone-number keypad of the calls tab replaces its list.
    pub dialpad_open: bool,
    pub call_phase: CallPhase,
    pub muted: bool,
    call_name: Field<NAME_CAPACITY>,
    call_since: i64,
    call_reason: Field<BANNER_CAPACITY>,
    /// When the `Ended` screen closes by itself.
    call_until: i64,
    /// A message box over everything, closed with OK: what went wrong with a call.
    pub dialog_open: bool,
    /// The list of conversations a message is being forwarded to (Encaminhar), and the text going.
    pub forward_open: bool,
    forward_text: [u8; MESSAGE_CAPACITY],
    forward_len: usize,
    pub forward_scroll: i32,
    /// While the message box asks whether to delete message `n` for everyone.
    pub confirm_delete: Option<usize>,
    /// The conversation whose messages a message box asks to clear.
    pub confirm_clear: Option<usize>,
    /// The conversation whose removal from the list a message box asks to confirm.
    pub confirm_remove: Option<usize>,
    /// The dropdown of the three dots in the conversation header.
    pub chat_menu_open: bool,
    /// The contact's panel over the right edge of the conversation.
    pub info_open: bool,
    dialog_title: Field<64>,
    dialog_body: Field<192>,
    contacts: [Item; MAX_CONTACTS],
    contact_count: usize,
    my_status: Item,
    stories: [Item; MAX_STORIES],
    story_count: usize,
    channels: [Item; MAX_CHANNELS],
    channel_count: usize,
    suggestions: [Item; MAX_SUGGESTIONS],
    suggestion_count: usize,
    calls: [Call; MAX_CALLS],
    call_count: usize,
    favorites: [Call; MAX_FAVORITES],
    favorite_count: usize,
    /// Ranks handed out to conversations: the history counts down from `HISTORY_RANK`, live activity
    /// counts up from `LIVE_RANK`, so anything live outranks anything from the history.
    next_history_rank: u32,
    next_live_rank: u32,
    pub selected: usize,
    pub focus: Focus,
    pub list_scroll: i32,
    pub conversation_scroll: i32,
    pub search: Field<SEARCH_CAPACITY>,
    pub draft: Field<MESSAGE_CAPACITY>,
    /// A line of bad news from the core (for example that sending is not implemented yet), shown until the next key or click.
    pub banner: Field<BANNER_CAPACITY>,
    pub picker_open: bool,
    pub picker_row: usize,
    /// Whether a conversation is open; until one is, the right side shows the welcome card.
    pub open: bool,
    /// The "Não lidas" filter chip.
    pub unread_only: bool,
    conversations: [Conversation; MAX_CHATS],
    /// Conversations that match the search, in list order.
    pub visible: [u8; MAX_CHATS],
    pub visible_len: usize,
    pending_accent: Option<char>,
    /// False while the core is still loading history: those messages are not news.
    live: bool,
    messages: [Message; MAX_MESSAGES],
    message_count: usize,
    text_limit: i32,
    window_height: i32,
}

/// Where the conversation list ends and the open conversation begins: the icon rail plus a list
/// panel that takes 30% of the window.
pub fn sidebar_width(window_width: i32) -> i32 {
    RAIL_W + (window_width * 30 / 100).clamp(300, 575)
}

pub struct BubbleShape {
    /// Width of the text area, without the horizontal padding.
    pub width: i32,
    pub height: i32,
    /// Whether the time shares the last line of text instead of taking a row of its own.
    pub time_inline: bool,
}

/// Room the small star of a favourite message takes before the time.
pub const STAR_WIDTH: i32 = 14;

pub fn meta_width(time: &str, outgoing: bool, starred: bool) -> i32 {
    font::measure(time, META) + if outgoing { 4 + TICKS_WIDTH } else { 0 } + if starred { STAR_WIDTH } else { 0 }
}

/// `quote_width` is the width of the quoted message's block, or 0 when the bubble quotes nothing.
pub fn bubble_shape(text: &str, time: &str, outgoing: bool, starred: bool, text_limit: i32, quote_width: i32, block: (i32, i32)) -> BubbleShape {
    let meta = meta_width(time, outgoing, starred);
    let (mut count, mut widest, mut last) = (0, 0, 0);
    for line in font::lines(text, BODY, text_limit) {
        let width = font::measure(line, BODY);
        count += 1;
        widest = widest.max(width);
        last = width;
    }
    let time_inline = last + META_GAP + meta <= text_limit;
    let width = if time_inline { widest.max(last + META_GAP + meta) } else { widest.max(meta) }.max(quote_width).max(block.0);
    let height = 2 * BUBBLE_PAD_Y + count.max(1) * LINE_HEIGHT + if time_inline { 0 } else { META_ROW } + if quote_width > 0 { QUOTE_H } else { 0 } + if block.1 > 0 { block.1 + BLOCK_GAP } else { 0 };
    BubbleShape { width, height, time_inline }
}

fn fold(c: char) -> char {
    match c {
        'á' | 'à' | 'â' | 'ã' | 'Á' | 'À' | 'Â' | 'Ã' => 'a',
        'é' | 'ê' | 'É' | 'Ê' => 'e',
        'í' | 'Í' => 'i',
        'ó' | 'ô' | 'õ' | 'Ó' | 'Ô' | 'Õ' => 'o',
        'ú' | 'ü' | 'Ú' | 'Ü' => 'u',
        'ç' | 'Ç' => 'c',
        other => other.to_ascii_lowercase(),
    }
}

/// Where `query` first appears in `text` at or after byte `from`, ignoring case and accents: the byte
/// range of the match. Folding maps one character to one character, so the ranges line up.
pub fn find_from(text: &str, query: &str, from: usize) -> Option<(usize, usize)> {
    if query.is_empty() {
        return None;
    }
    for (start, _) in text[from..].char_indices() {
        let start = from + start;
        let mut rest = text[start..].char_indices().map(|(offset, c)| (offset, fold(c)));
        let mut end = start;
        let mut matched = true;
        for wanted in query.chars().map(fold) {
            match rest.next() {
                Some((offset, c)) if c == wanted => end = start + offset + text[start + offset..].chars().next().map_or(0, char::len_utf8),
                _ => {
                    matched = false;
                    break;
                }
            }
        }
        if matched {
            return Some((start, end));
        }
    }
    None
}

/// Whether `query` appears in `name`, ignoring case and accents.
fn matches(name: &str, query: &str) -> bool {
    name.char_indices().any(|(start, _)| {
        let mut rest = name[start..].chars().map(fold);
        query.chars().map(fold).all(|wanted| rest.next() == Some(wanted))
    })
}

/// Writes `SEND\t<chat>\t` at the start of `line` and returns its length.
fn write_send_head(line: &mut [u8], kind: &[u8], chat: usize) -> usize {
    line[..kind.len()].copy_from_slice(kind);
    let mut at = kind.len();
    if chat >= 10 {
        line[at] = b'0' + (chat / 10) as u8;
        at += 1;
    }
    line[at] = b'0' + (chat % 10) as u8;
    line[at + 1] = b'\t';
    at + 2
}

/// The accented letter a dead key and a base letter make together.
fn compose(accent: char, base: char) -> Option<char> {
    let pairs = match accent {
        '´' => "aáeéiíoóuúAÁEÉIÍOÓUÚcçCÇ",
        '`' => "aàeèiìoòuùAÀEÈIÌOÒUÙ",
        '^' => "aâeêiîoôuûAÂEÊIÎOÔUÛ",
        '~' => "aãoõnñAÃOÕNÑ",
        '¨' => "aäeëiïoöuüAÄEËIÏOÖUÜ",
        _ => return None,
    };
    let mut chars = pairs.chars();
    while let (Some(from), Some(to)) = (chars.next(), chars.next()) {
        if from == base {
            return Some(to);
        }
    }
    None
}

/// Today, in days since 1970-01-01 (Brasilia time).
pub fn today() -> u16 {
    ((sys::unix_seconds() + BRASILIA_OFFSET_SECONDS).div_euclid(86_400)) as u16
}

/// The time and the day of a `MSG` time field: "HH:MM", optionally followed by a space and the day
/// number; without one the message is from today (0).
fn split_stamp(time: &str) -> ([u8; 5], u16) {
    let mut stamp = [b' '; 5];
    let (clock, day) = time.split_once(' ').unwrap_or((time, ""));
    let len = clock.len().min(5);
    stamp[..len].copy_from_slice(&clock.as_bytes()[..len]);
    (stamp, day.trim().parse().unwrap_or(0))
}

const WEEKDAYS: [&str; 7] = ["DOMINGO", "SEGUNDA-FEIRA", "TERÇA-FEIRA", "QUARTA-FEIRA", "QUINTA-FEIRA", "SEXTA-FEIRA", "SÁBADO"];
const WEEKDAYS_SHORT: [&str; 7] = ["Dom", "Seg", "Ter", "Qua", "Qui", "Sex", "Sáb"];

/// (year, month, day) of a day count since 1970-01-01 (Howard Hinnant's civil-from-days).
fn civil_date(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(month <= 2), month, day)
}

/// "dd/mm/aaaa" in `buffer`.
fn date_text(day: u16, buffer: &mut [u8; 10]) -> &str {
    let (year, month, date) = civil_date(day as i64);
    let digits = [date / 10, date % 10, -1, month / 10, month % 10, -1, year / 1000, year / 100 % 10, year / 10 % 10, year % 10];
    for (slot, digit) in buffer.iter_mut().zip(digits) {
        *slot = if digit < 0 { b'/' } else { b'0' + digit as u8 };
    }
    core::str::from_utf8(&buffer[..]).unwrap_or("")
}

/// The chip above the messages of one day: HOJE, ONTEM, the weekday for the last week, else the date.
pub fn day_chip(day: u16, buffer: &mut [u8; 10]) -> &str {
    let ago = today().saturating_sub(day);
    match ago {
        0 => "HOJE",
        1 => "ONTEM",
        2..=6 => WEEKDAYS[(day as usize + 4) % 7],
        _ => date_text(day, buffer),
    }
}

/// What the conversation list shows for the last message: its time today, else Ontem, the weekday's
/// first letters for the last week, else the date.
pub fn list_day<'a>(day: u16, clock: &'a str, buffer: &'a mut [u8; 10]) -> &'a str {
    let ago = today().saturating_sub(day);
    match ago {
        0 => clock,
        1 => "Ontem",
        2..=6 => WEEKDAYS_SHORT[(day as usize + 4) % 7],
        _ => date_text(day, buffer),
    }
}

fn clock(unix_seconds: i64) -> [u8; 5] {
    let day = (unix_seconds + BRASILIA_OFFSET_SECONDS).rem_euclid(86_400);
    let (hours, minutes) = ((day / 3600) as u8, (day % 3600 / 60) as u8);
    [b'0' + hours / 10, b'0' + hours % 10, b':', b'0' + minutes / 10, b'0' + minutes % 10]
}

impl Chat {
    pub const fn new() -> Self {
        Self {
            tab: Tab::Chats,
            panel_scroll: 0,
            profile: Field::new(),
            new_chat: false,
            menu_open: false,
            starred_open: false,
            find_open: false,
            find: Field::new(),
            find_current: 0,
            below_count: 0,
            hover: None,
            older_asked: 0,
            older_content: 0,
            exhausted: 0,
            thumbs: [[0; THUMB_SIDE * THUMB_SIDE]; THUMB_SLOTS],
            thumb_size: [(0, 0); THUMB_SLOTS],
            thumb_next: 0,
            thumb_asked: 0,
            picker_reaction: None,
            flash: None,
            reply_to: [0; ID_CAPACITY],
            reply_to_len: 0,
            editing: false,
            message_menu: None,
            caret_on: true,
            window_focused: true,
            undo_states: [Snap::EMPTY; UNDO_DEPTH],
            undo_len: 0,
            redo_states: [Snap::EMPTY; UNDO_DEPTH],
            redo_len: 0,
            last_change: Change::Other,
            clip: [0; MESSAGE_CAPACITY],
            clip_len: 0,
            clip_pending: false,
            paste_wanted: false,
            dialpad_open: false,
            call_phase: CallPhase::Idle,
            muted: false,
            call_name: Field::new(),
            call_since: 0,
            call_reason: Field::new(),
            call_until: 0,
            dialog_open: false,
            confirm_delete: None,
            confirm_clear: None,
            confirm_remove: None,
            chat_menu_open: false,
            info_open: false,
            forward_open: false,
            forward_text: [0; MESSAGE_CAPACITY],
            forward_len: 0,
            forward_scroll: 0,
            dialog_title: Field::new(),
            dialog_body: Field::new(),
            contacts: [Item::EMPTY; MAX_CONTACTS],
            contact_count: 0,
            my_status: Item::EMPTY,
            stories: [Item::EMPTY; MAX_STORIES],
            story_count: 0,
            channels: [Item::EMPTY; MAX_CHANNELS],
            channel_count: 0,
            suggestions: [Item::EMPTY; MAX_SUGGESTIONS],
            suggestion_count: 0,
            calls: [Call::EMPTY; MAX_CALLS],
            call_count: 0,
            favorites: [Call::EMPTY; MAX_FAVORITES],
            favorite_count: 0,
            next_history_rank: HISTORY_RANK,
            next_live_rank: LIVE_RANK,
            selected: 0,
            focus: Focus::Message,
            list_scroll: 0,
            conversation_scroll: 0,
            search: Field::new(),
            draft: Field::new(),
            banner: Field::new(),
            picker_open: false,
            picker_row: 0,
            open: false,
            unread_only: false,
            conversations: [Conversation::EMPTY; MAX_CHATS],
            visible: [0; MAX_CHATS],
            visible_len: 0,
            pending_accent: None,
            live: false,
            messages: [Message::EMPTY; MAX_MESSAGES],
            message_count: 0,
            text_limit: 0,
            window_height: 0,
        }
    }

    /// Lays out the empty chat screen for the given window size; the core fills it in.
    pub fn start(&mut self, window_width: i32, window_height: i32) {
        self.update_visible();
        self.resize(window_width, window_height);
    }

    pub fn conversation(&self, index: usize) -> &Conversation {
        &self.conversations[index]
    }

    /// Forgets every conversation and message.
    pub fn reset(&mut self) {
        self.conversations = [Conversation::EMPTY; MAX_CHATS];
        self.next_history_rank = HISTORY_RANK;
        self.next_live_rank = LIVE_RANK;
        self.message_count = 0;
        self.older_asked = 0;
        self.exhausted = 0;
        self.thumb_asked = 0;
        self.live = false;
        self.open = false;
        self.selected = 0;
        self.list_scroll = 0;
        self.conversation_scroll = 0;
        self.call_count = 0;
        self.favorite_count = 0;
        self.story_count = 0;
        self.contact_count = 0;
        self.new_chat = false;
        self.channel_count = 0;
        self.suggestion_count = 0;
        self.panel_scroll = 0;
        self.update_visible();
    }

    pub fn add_call(&mut self, kind: CallKind, count: u8, time: &str, name: &str) {
        if self.call_count == MAX_CALLS {
            return;
        }
        let call = &mut self.calls[self.call_count];
        (call.kind, call.count) = (kind, count);
        store(&mut call.time, &mut call.time_len, time);
        store(&mut call.name, &mut call.name_len, name);
        self.call_count += 1;
    }

    pub fn set_profile(&mut self, name: &str) {
        self.profile.clear();
        for c in name.chars() {
            self.profile.push(c);
        }
    }

    pub fn profile_name(&self) -> &str {
        self.profile.text()
    }

    pub fn set_my_status(&mut self, time: &str) {
        self.my_status.set(0, 0, false, time, "", "");
    }

    pub fn my_status(&self) -> &Item {
        &self.my_status
    }

    /// A contact's status: `segments` parts in its ring, the last `unseen` of them not yet seen.
    pub fn add_story(&mut self, segments: u16, unseen: u16, time: &str, name: &str) {
        if self.story_count < MAX_STORIES {
            self.stories[self.story_count].set(segments, unseen, unseen == 0, time, name, "");
            self.story_count += 1;
        }
    }

    pub fn story(&self, index: usize) -> &Item {
        &self.stories[index]
    }

    pub fn add_channel(&mut self, unread: u16, time: &str, name: &str, preview: &str) {
        if self.channel_count < MAX_CHANNELS {
            self.channels[self.channel_count].set(unread, 0, false, time, name, preview);
            self.channel_count += 1;
        }
    }

    pub fn channel(&self, index: usize) -> &Item {
        &self.channels[index]
    }

    pub fn add_suggestion(&mut self, verified: bool, followers: &str, name: &str) {
        if self.suggestion_count < MAX_SUGGESTIONS {
            self.suggestions[self.suggestion_count].set(0, 0, verified, "", name, followers);
            self.suggestion_count += 1;
        }
    }

    pub fn suggestion(&self, index: usize) -> &Item {
        &self.suggestions[index]
    }

    /// Whether some status has not been seen yet (the dot on the rail).
    pub fn has_unseen_status(&self) -> bool {
        self.stories[..self.story_count].iter().any(|story| story.second > 0)
    }

    /// Whether some followed channel has unread updates.
    pub fn has_unread_channel(&self) -> bool {
        self.channels[..self.channel_count].iter().any(|channel| channel.first > 0)
    }

    pub fn open_dialpad(&mut self) {
        self.dialpad_open = true;
        self.search.clear();
        self.focus = Focus::Search;
    }

    pub fn close_dialpad(&mut self) {
        self.dialpad_open = false;
        self.search.clear();
        self.focus = Focus::Message;
    }

    /// A key of the on-screen keypad: a digit, `+`, or backspace (`\u{8}`).
    pub fn dial_key(&mut self, key: char) {
        if key == '\u{8}' {
            self.search.pop();
        } else {
            self.search.push(key);
        }
    }

    /// Calls favourite number `index`.
    pub fn call_favorite(&mut self, index: usize) {
        let mut name = [0u8; NAME_CAPACITY];
        let len = self.favorites[index].name_len as usize;
        name[..len].copy_from_slice(&self.favorites[index].name[..len]);
        if let Ok(name) = core::str::from_utf8(&name[..len]) {
            self.start_call(name);
        }
    }

    pub fn call_name(&self) -> &str {
        self.call_name.text()
    }

    fn tell_core(line: &[u8]) {
        sys::write_all(1, line);
    }

    /// We call `name`: the core is told, and the call rings until it says the other side answered.
    pub fn start_call(&mut self, name: &str) {
        if self.call_phase != CallPhase::Idle {
            return;
        }
        self.set_call_name(name);
        self.call_phase = CallPhase::Ringing;
        self.muted = false;
        let mut line = [0u8; NAME_CAPACITY + 8];
        line[..5].copy_from_slice(b"DIAL\t");
        let len = self.call_name.len;
        line[5..5 + len].copy_from_slice(&self.call_name.bytes[..len]);
        line[5 + len] = b'\n';
        Self::tell_core(&line[..6 + len]);
    }

    /// Someone calls us.
    pub fn incoming_call(&mut self, name: &str) {
        if self.call_phase == CallPhase::Idle {
            self.set_call_name(name);
            self.call_phase = CallPhase::Incoming;
            self.muted = false;
        }
    }

    fn set_call_name(&mut self, name: &str) {
        self.call_name.clear();
        for c in name.chars() {
            self.call_name.push(c);
        }
    }

    pub fn answer_call(&mut self) {
        if self.call_phase == CallPhase::Incoming {
            self.call_phase = CallPhase::Active;
            self.call_since = sys::unix_seconds();
            Self::tell_core(b"ANSWER\n");
        }
    }

    /// Declines, or ends, whatever call there is.
    pub fn hang_up(&mut self) {
        if self.call_phase != CallPhase::Idle {
            self.call_phase = CallPhase::Idle;
            Self::tell_core(b"HANGUP\n");
        }
    }

    /// The core says the call we placed was picked up.
    pub fn call_connected(&mut self) {
        if self.call_phase == CallPhase::Ringing {
            self.call_phase = CallPhase::Active;
            self.call_since = sys::unix_seconds();
        }
    }

    /// The call is over. With a `reason` and a call on screen, the screen stays two seconds to show it.
    pub fn call_ended(&mut self, reason: &str) {
        let on_screen = matches!(self.call_phase, CallPhase::Ringing | CallPhase::Active);
        if reason.is_empty() || !on_screen {
            self.call_phase = CallPhase::Idle;
            return;
        }
        self.call_reason.clear();
        for c in reason.chars() {
            self.call_reason.push(c);
        }
        self.call_phase = CallPhase::Ended;
        self.call_until = sys::unix_seconds() + 2;
    }

    pub fn call_reason(&self) -> &str {
        self.call_reason.text()
    }

    /// Whether the screen needs redrawing every second (a clock or a countdown is running).
    pub fn call_ticking(&self) -> bool {
        matches!(self.call_phase, CallPhase::Active | CallPhase::Ended)
    }

    /// Called once a second while `call_ticking`.
    pub fn tick(&mut self) {
        if self.call_phase == CallPhase::Ended && sys::unix_seconds() >= self.call_until {
            self.call_phase = CallPhase::Idle;
        }
    }

    /// The call could not be made: whatever was on screen goes, and a message box says why.
    pub fn call_failed(&mut self, title: &str, body: &str) {
        self.call_phase = CallPhase::Idle;
        self.dialog_title.clear();
        for c in title.chars() {
            self.dialog_title.push(c);
        }
        self.dialog_body.clear();
        for c in body.chars() {
            self.dialog_body.push(c);
        }
        self.dialog_open = true;
    }

    pub fn dialog_title(&self) -> &str {
        self.dialog_title.text()
    }

    pub fn dialog_body(&self) -> &str {
        self.dialog_body.text()
    }

    /// A call that just happened, at the top of the recents.
    pub fn add_call_first(&mut self, kind: CallKind, count: u8, time: &str, name: &str) {
        if self.call_count == MAX_CALLS {
            self.call_count -= 1;
        }
        self.calls[..=self.call_count].rotate_right(1);
        self.call_count += 1;
        let call = &mut self.calls[0];
        (call.kind, call.count) = (kind, count);
        store(&mut call.time, &mut call.time_len, time);
        store(&mut call.name, &mut call.name_len, name);
    }

    /// `mm:ss` (or `h:mm:ss`) of the active call, in `buffer`.
    pub fn call_clock<'a>(&self, buffer: &'a mut [u8; 8]) -> &'a str {
        let seconds = (sys::unix_seconds() - self.call_since).max(0);
        let (hours, minutes, secs) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
        let mut len = 0;
        if hours > 0 {
            buffer[len] = b'0' + (hours % 10) as u8;
            buffer[len + 1] = b':';
            len += 2;
        }
        for (value, colon) in [(minutes, true), (secs, false)] {
            buffer[len] = b'0' + (value / 10) as u8;
            buffer[len + 1] = b'0' + (value % 10) as u8;
            len += 2;
            if colon {
                buffer[len] = b':';
                len += 1;
            }
        }
        core::str::from_utf8(&buffer[..len]).unwrap_or("")
    }

    pub fn add_contact(&mut self, name: &str) {
        if self.contact_count < MAX_CONTACTS {
            self.contacts[self.contact_count].set(0, 0, false, "", name, "");
            self.contact_count += 1;
        }
    }

    /// The core is about to send the contacts that match the search box.
    pub fn clear_contacts(&mut self) {
        self.contact_count = 0;
        self.panel_scroll = self.panel_scroll.min(self.max_panel_scroll());
    }

    /// Asks the core for the contacts that match the search box (`CONTACTS<TAB>query`).
    fn request_contacts(&self) {
        let query = self.search.text().as_bytes();
        let mut line = [0u8; SEARCH_CAPACITY + 10];
        line[..9].copy_from_slice(b"CONTACTS\t");
        line[9..9 + query.len()].copy_from_slice(query);
        line[9 + query.len()] = b'\n';
        sys::write_all(1, &line[..10 + query.len()]);
    }

    pub fn contact(&self, index: usize) -> &Item {
        &self.contacts[index]
    }

    pub fn open_new_chat(&mut self) {
        self.new_chat = true;
        self.panel_scroll = 0;
        self.search.clear();
        self.focus = Focus::Search;
        self.update_visible();
    }

    pub fn close_new_chat(&mut self) {
        self.new_chat = false;
        self.panel_scroll = 0;
        self.search.clear();
        self.focus = Focus::Message;
        self.update_visible();
    }

    /// Opens the conversation with the contact if there already is one.
    pub fn pick_contact(&mut self, index: usize) {
        let name = self.contacts[index].name();
        if let Some(chat) = (0..MAX_CHATS).find(|&i| self.conversations[i].used && self.conversations[i].name() == name) {
            self.close_new_chat();
            self.select(chat);
            return;
        }
        // No conversation yet: the core makes one and answers with `SHOWCHAT`.
        let mut line = [0u8; NAME_CAPACITY + 10];
        line[..9].copy_from_slice(b"OPENCHAT\t");
        let len = name.len();
        line[9..9 + len].copy_from_slice(name.as_bytes());
        line[9 + len] = b'\n';
        sys::write_all(1, &line[..10 + len]);
    }

    /// Starts a conversation with the phone number typed in the search box (`OPENCHAT` with its digits).
    pub fn pick_number(&mut self) {
        let mut digits = [0u8; PHONE_DIGITS];
        let Some(count) = phone_digits(self.search.text(), &mut digits) else { return };
        let mut line = [0u8; PHONE_DIGITS + 10];
        line[..9].copy_from_slice(b"OPENCHAT\t");
        line[9..9 + count].copy_from_slice(&digits[..count]);
        line[9 + count] = b'\n';
        sys::write_all(1, &line[..10 + count]);
    }

    /// "Mensagens para mim": the conversation with ourselves (the core makes it when it does not exist).
    pub fn pick_self(&mut self) {
        if let Some(chat) = (0..MAX_CHATS).find(|&i| self.conversations[i].used && self.conversations[i].name() == SELF_CHAT) {
            self.close_new_chat();
            self.select(chat);
            return;
        }
        sys::write_all(1, b"OPENCHAT\t\n");
    }

    /// The core made (or found) conversation `chat`: leave the panel and show it.
    pub fn show_chat(&mut self, chat: usize) {
        if chat < MAX_CHATS && self.conversations[chat].used {
            self.close_new_chat();
            self.select(chat);
        }
    }

    /// Rows of the new-conversation panel with their top edge in content coordinates. While
    /// searching, only the matching contacts show. Returns the content height.
    pub fn contacts_place(&self, mut visit: impl FnMut(ContactsItem, i32)) -> i32 {
        let query = self.search.text();
        let mut y = NEW_CHAT_TOP;
        if query.is_empty() {
            for index in 0..NEW_CHAT_ACTIONS.len() {
                visit(ContactsItem::Action(index), y);
                y += NEW_CHAT_ACTION_H;
            }
            visit(ContactsItem::Myself, NEW_CHAT_SELF_TOP);
            y = NEW_CHAT_SELF_TOP + NEW_CHAT_ROW_H;
        }
        if phone_digits(query, &mut [0u8; PHONE_DIGITS]).is_some() {
            visit(ContactsItem::Number, y);
            y += NEW_CHAT_ROW_H;
        }
        let mut letter = '\0';
        for index in (0..self.contact_count).filter(|&i| matches(self.contacts[i].name(), query)) {
            let initial = initial_of(self.contacts[index].name());
            if initial != letter {
                letter = initial;
                visit(ContactsItem::Letter(letter), y);
                y += NEW_CHAT_ROW_H;
            }
            visit(ContactsItem::Contact(index), y);
            y += NEW_CHAT_ROW_H;
        }
        y
    }

    pub fn add_favorite(&mut self, name: &str) {
        if self.favorite_count == MAX_FAVORITES {
            return;
        }
        let call = &mut self.favorites[self.favorite_count];
        store(&mut call.name, &mut call.name_len, name);
        self.favorite_count += 1;
    }

    pub fn call(&self, index: usize) -> &Call {
        &self.calls[index]
    }

    pub fn favorite(&self, index: usize) -> &Call {
        &self.favorites[index]
    }

    /// Switches the rail tab; the search box starts empty on each one.
    pub fn set_tab(&mut self, tab: Tab) {
        if tab == self.tab {
            return;
        }
        self.tab = tab;
        self.dialpad_open = false;
        self.menu_open = false;
        self.starred_open = false;
        self.new_chat = false;
        self.panel_scroll = 0;
        self.search.clear();
        self.focus = Focus::Message;
        self.picker_open = false;
        self.update_visible();
    }

    /// Visits the rows of the calls screen that match the search box, with their top edge in
    /// scrolling content coordinates (the list's own top is `CALLS_TOP`). Returns the content height.
    pub fn calls_place(&self, mut visit: impl FnMut(CallsItem, i32)) -> i32 {
        let query = self.search.text();
        let mut y = CALLS_TOP;
        if (0..self.favorite_count).any(|i| matches(self.favorites[i].name(), query)) {
            visit(CallsItem::FavoritesHeader, y);
            y += FAVORITES_HEADER_H;
            for index in (0..self.favorite_count).filter(|&i| matches(self.favorites[i].name(), query)) {
                visit(CallsItem::Favorite(index), y);
                y += CALLS_ROW_H;
            }
        }
        if (0..self.call_count).any(|i| matches(self.calls[i].name(), query)) {
            visit(CallsItem::RecentsHeader, y);
            y += RECENTS_HEADER_H;
            for index in (0..self.call_count).filter(|&i| matches(self.calls[i].name(), query)) {
                visit(CallsItem::Recent(index), y);
                y += CALLS_ROW_H;
            }
        }
        y
    }

    /// Rows of the status screen with their top edge in content coordinates, and the content height.
    pub fn status_place(&self, mut visit: impl FnMut(StatusItem, i32)) -> i32 {
        visit(StatusItem::Mine, HEADER_H);
        let mut y = 142;
        for (seen, label) in [(false, StatusItem::RecentLabel), (true, StatusItem::SeenLabel)] {
            let mut any = false;
            for index in (0..self.story_count).filter(|&i| (self.stories[i].second == 0) == seen) {
                if !any {
                    visit(label, y);
                    y += STATUS_LABEL_H;
                    any = true;
                }
                visit(StatusItem::Story(index), y);
                y += STATUS_ROW_H;
            }
        }
        y
    }

    /// Rows of the channels screen, same convention.
    pub fn channels_place(&self, mut visit: impl FnMut(ChannelsItem, i32)) -> i32 {
        let query = self.search.text();
        let mut y = CALLS_TOP;
        for index in (0..self.channel_count).filter(|&i| matches(self.channels[i].name(), query)) {
            visit(ChannelsItem::Followed(index), y);
            y += CHANNEL_ROW_H;
        }
        visit(ChannelsItem::SuggestionsHeader, y);
        y += SUGGESTIONS_HEADER_H;
        for index in (0..self.suggestion_count).filter(|&i| matches(self.suggestions[i].name(), query)) {
            visit(ChannelsItem::Suggestion(index), y);
            y += SUGGESTION_ROW_H;
        }
        visit(ChannelsItem::DiscoverBar, y);
        y + DISCOVER_BAR_H
    }

    pub fn settings_height(&self) -> i32 {
        SETTINGS_ROWS_TOP + SETTINGS_ENTRIES.len() as i32 * SETTINGS_ROW_H + SETTINGS_LOGOUT_H + 24
    }

    pub fn max_panel_scroll(&self) -> i32 {
        let content = match self.tab {
            Tab::Calls => self.calls_place(|_, _| {}),
            Tab::Status => self.status_place(|_, _| {}),
            Tab::Channels => self.channels_place(|_, _| {}),
            Tab::Settings => self.settings_height(),
            Tab::Chats if self.new_chat => self.contacts_place(|_, _| {}),
            Tab::Chats if self.starred_open => HEADER_H + self.starred_count() as i32 * STARRED_ROW_H,
            Tab::Chats => 0,
        };
        (content - self.window_height).max(0)
    }

    // ---- favourite messages

    pub fn open_starred(&mut self) {
        self.starred_open = true;
        self.panel_scroll = 0;
        self.search.clear();
        self.focus = Focus::Message;
    }

    pub fn close_starred(&mut self) {
        self.starred_open = false;
        self.panel_scroll = 0;
    }

    pub fn starred_count(&self) -> usize {
        self.messages[..self.message_count].iter().filter(|message| message.starred).count()
    }

    /// The `nth` favourite message (newest first), by its number among all the messages.
    pub fn starred_nth(&self, nth: usize) -> Option<usize> {
        (0..self.message_count).rev().filter(|&index| self.messages[index].starred).nth(nth)
    }

    /// Opens the conversation of the `nth` favourite and shows the message.
    pub fn open_starred_message(&mut self, nth: usize) {
        let Some(index) = self.starred_nth(nth) else { return };
        let target = self.messages[index].chat as usize;
        self.close_starred();
        self.select(target);
        self.jump_to_message(index);
    }

    pub fn scroll_panel(&mut self, notches: i32) {
        self.panel_scroll = (self.panel_scroll + notches * LIST_SCROLL_STEP).clamp(0, self.max_panel_scroll());
    }

    /// Only the line under the name in the conversation header changes: "online", "digitando…"...
    pub fn set_status(&mut self, id: usize, status: &str) {
        if id < MAX_CHATS && self.conversations[id].used {
            let conversation = &mut self.conversations[id];
            store(&mut conversation.status, &mut conversation.status_len, status);
        }
    }

    /// Puts `chat` at the top of the list.
    fn bump(&mut self, chat: usize) {
        self.conversations[chat].rank = self.next_live_rank;
        self.next_live_rank += 1;
        self.update_visible();
    }

    pub fn set_conversation(&mut self, id: usize, unread: u8, time: &str, name: &str, status: &str) {
        if id >= MAX_CHATS {
            return;
        }
        let first = !self.conversations.iter().any(|c| c.used);
        let is_new = !self.conversations[id].used;
        let conversation = &mut self.conversations[id];
        conversation.used = true;
        conversation.unread = unread;
        store(&mut conversation.time, &mut conversation.time_len, time);
        store(&mut conversation.name, &mut conversation.name_len, name);
        store(&mut conversation.status, &mut conversation.status_len, status);
        if first {
            self.selected = id;
        }
        if is_new {
            if self.live {
                self.bump(id);
            } else {
                self.conversations[id].rank = self.next_history_rank;
                self.next_history_rank = self.next_history_rank.saturating_sub(1);
            }
        }
        self.update_visible();
    }

    /// A message from the core. An incoming one in a conversation that is not open counts as unread.
    pub fn receive(&mut self, chat: usize, outgoing: bool, read: bool, time: &str, id: &str, reply: &str, text: &str) {
        if chat >= MAX_CHATS || !self.conversations[chat].used {
            return;
        }
        let at_bottom = chat == self.selected && self.conversation_scroll >= self.max_conversation_scroll() - 8;
        let (stamp, day) = split_stamp(time);
        self.push_message(chat, outgoing, if read { Delivery::Read } else { Delivery::Delivered }, text, stamp, day, id, reply);
        if self.live && !outgoing && chat != self.selected {
            self.conversations[chat].unread = self.conversations[chat].unread.saturating_add(1);
        }
        if self.live {
            self.bump(chat);
        }
        if at_bottom {
            self.conversation_scroll = self.max_conversation_scroll();
        } else if self.live && !outgoing && chat == self.selected && self.open {
            self.below_count = self.below_count.saturating_add(1);
        }
    }

    // ---- photos, videos, voice messages and documents

    /// Message `id` of `chat` carries something besides text (`MEDIA`).
    pub fn set_media(&mut self, chat: usize, id: &str, kind: u8, width: u32, height: u32, label: &str) {
        let Some(index) = self.message_with_id(chat as u8, id) else { return };
        let message = &mut self.messages[index];
        message.kind = kind;
        message.media_w = width.min(u16::MAX as u32) as u16;
        message.media_h = height.min(u16::MAX as u32) as u16;
        store(&mut message.label, &mut message.label_len, label);
        self.recompute_height(index);
    }

    /// The preview picture of message `id` starts arriving (`THUMB`): it takes the next slot of the pool.
    pub fn thumb_start(&mut self, chat: usize, id: &str, width: usize, height: usize) {
        let Some(index) = self.message_with_id(chat as u8, id) else { return };
        if width == 0 || height == 0 || width > THUMB_SIDE || height > THUMB_SIDE {
            self.thumb_asked = 0;
            return;
        }
        let slot = self.thumb_next as usize % THUMB_SLOTS;
        self.thumb_next = ((slot + 1) % THUMB_SLOTS) as u8;
        // Whoever held the slot has to ask again if it is shown once more.
        for other in self.messages[..self.message_count].iter_mut() {
            if other.thumb == 2 && other.slot as usize == slot {
                other.thumb = 0;
            }
        }
        self.thumb_size[slot] = (width as u8, height as u8);
        self.messages[index].slot = slot as u8;
        self.messages[index].thumb = 1;
    }

    /// One row of pixels of the picture that is arriving, as hex of little-endian RGB565.
    pub fn thumb_row(&mut self, chat: usize, id: &str, row: usize, hex: &[u8]) {
        let Some(index) = self.message_with_id(chat as u8, id) else { return };
        let message = &mut self.messages[index];
        if message.thumb != 1 {
            return;
        }
        let slot = message.slot as usize;
        let (width, height) = (self.thumb_size[slot].0 as usize, self.thumb_size[slot].1 as usize);
        if row >= height || hex.len() < width * 4 {
            return;
        }
        let digit = |byte: u8| (byte as char).to_digit(16).unwrap_or(0) as u8;
        for column in 0..width {
            let at = column * 4;
            let low = digit(hex[at]) << 4 | digit(hex[at + 1]);
            let high = digit(hex[at + 2]) << 4 | digit(hex[at + 3]);
            self.thumbs[slot][row * THUMB_SIDE + column] = (high as u16) << 8 | low as u16;
        }
        if row + 1 == height {
            message.thumb = 2;
            self.thumb_asked = 0;
        }
    }

    /// The core has no preview for message `id` (`THUMBFAIL`).
    pub fn thumb_failed(&mut self, chat: usize, id: &str) {
        if let Some(index) = self.message_with_id(chat as u8, id) {
            self.messages[index].thumb = 3;
        }
        self.thumb_asked = 0;
    }

    /// The preview picture of `message`: its pixels (rows `THUMB_SIDE` apart), width and height.
    pub fn thumb_of(&self, message: &Message) -> Option<(&[u16], usize, usize)> {
        if message.thumb != 2 {
            return None;
        }
        let slot = message.slot as usize;
        let (width, height) = self.thumb_size[slot];
        Some((&self.thumbs[slot], width as usize, height as usize))
    }

    /// Asks the core for the preview of a photo or video that is on screen and has none (`THUMB chat id`),
    /// one at a time.
    pub fn request_thumbs(&mut self) {
        let now = sys::monotonic_ms();
        if !self.open || self.tab != Tab::Chats || self.selected >= MAX_CHATS || (self.thumb_asked != 0 && now - self.thumb_asked < 5000) {
            return;
        }
        let (top, bottom) = (self.conversation_scroll - HEADER_H, self.conversation_scroll + self.viewport_height() + HEADER_H);
        let mut wanted = None;
        self.place(|message, y, _| {
            let block = message.block();
            if wanted.is_none() && message.thumb == 0 && message.id_len > 0 && (message.kind == MEDIA_IMAGE || message.kind == MEDIA_VIDEO) && y + block.1 >= top && y <= bottom {
                wanted = Some((message.id, message.id_len as usize));
            }
        });
        let Some((id, id_len)) = wanted else { return };
        let chat = self.selected;
        let mut line = [0u8; ID_CAPACITY + 16];
        let head = write_send_head(&mut line, b"THUMB\t", chat);
        line[head..head + id_len].copy_from_slice(&id[..id_len]);
        line[head + id_len] = b'\n';
        sys::write_all(1, &line[..head + id_len + 1]);
        if let Some(index) = self.messages[..self.message_count].iter().position(|m| m.chat as usize == chat && m.id_len as usize == id_len && m.id[..id_len] == id[..id_len]) {
            self.messages[index].thumb = 1;
        }
        self.thumb_asked = now;
    }

    /// An older message of `chat` from the core (`PAST`): it goes before the conversation's first one.
    pub fn receive_older(&mut self, chat: usize, outgoing: bool, read: bool, time: &str, id: &str, reply: &str, text: &str) {
        if chat >= MAX_CHATS || !self.conversations[chat].used {
            return;
        }
        let (stamp, day) = split_stamp(time);
        self.push_message(chat, outgoing, if read { Delivery::Read } else { Delivery::Delivered }, text, stamp, day, id, reply);
        let last = self.message_count - 1;
        if let Some(first) = self.messages[..last].iter().position(|m| m.chat as usize == chat) {
            self.messages[first..=last].rotate_right(1);
        }
        // The numbers of the messages changed under these.
        self.flash = None;
        self.message_menu = None;
        self.hover = None;
    }

    /// Asks the core for messages older than the first one of the open conversation, when the view is at
    /// its top and there may be more (`OLDER chat id`).
    pub fn request_older_if_top(&mut self) {
        let now = sys::monotonic_ms();
        if !self.open || self.tab != Tab::Chats || self.conversation_scroll > 0 || self.selected >= 64 || self.exhausted >> self.selected & 1 == 1 {
            return;
        }
        if self.older_asked != 0 && now - self.older_asked < 4000 {
            return;
        }
        let chat = self.selected;
        let Some(first) = self.messages[..self.message_count].iter().find(|m| m.chat as usize == chat && m.id_len > 0) else { return };
        let (id, id_len) = (first.id, first.id_len as usize);
        let mut line = [0u8; ID_CAPACITY + 16];
        let head = write_send_head(&mut line, b"OLDER\t", chat);
        line[head..head + id_len].copy_from_slice(&id[..id_len]);
        line[head + id_len] = b'\n';
        sys::write_all(1, &line[..head + id_len + 1]);
        self.older_asked = now;
        self.older_content = self.place(|_, _, _| {});
    }

    /// The core has sent all it had for the open request (`PASTEND chat more`): keep the view on the
    /// same messages.
    pub fn older_done(&mut self, chat: usize, more: bool) {
        self.older_asked = 0;
        if !more && chat < 64 {
            self.exhausted |= 1 << chat;
        }
        if chat == self.selected {
            let grown = self.place(|_, _, _| {}) - self.older_content;
            self.conversation_scroll += grown.max(0);
        }
    }

    /// The other side read everything we sent in `chat`.
    pub fn mark_read(&mut self, chat: usize) {
        for message in self.messages[..self.message_count].iter_mut().filter(|m| m.chat as usize == chat && m.outgoing) {
            message.delivery = Delivery::Read;
        }
    }

    /// The other side's phone got what we sent in `chat`.
    pub fn mark_delivered(&mut self, chat: usize) {
        for message in self.messages[..self.message_count].iter_mut().filter(|m| m.chat as usize == chat && m.outgoing && m.delivery == Delivery::Sent) {
            message.delivery = Delivery::Delivered;
        }
    }

    /// The history is loaded: from now on, incoming messages in other conversations are unread.
    pub fn go_live(&mut self) {
        self.live = true;
    }

    pub fn set_unread_only(&mut self, unread_only: bool) {
        self.unread_only = unread_only;
        self.update_visible();
    }

    /// Nothing waits unread in `chat` any more; the core is told (`MARKREAD chat`) when something did.
    fn read_chat(&mut self, chat: usize) {
        if self.conversations[chat].unread == 0 {
            return;
        }
        self.conversations[chat].unread = 0;
        let line = [b'M', b'A', b'R', b'K', b'R', b'E', b'A', b'D', b'\t', b'0' + (chat / 10) as u8, b'0' + (chat % 10) as u8, b'\n'];
        sys::write_all(1, &line);
    }

    /// "Marcar todas como lidas" of the menu.
    pub fn mark_all_read(&mut self) {
        for chat in 0..MAX_CHATS {
            if self.conversations[chat].used {
                self.read_chat(chat);
            }
        }
        self.update_visible();
    }

    /// How many messages wait unread across all conversations.
    pub fn unread_total(&self) -> usize {
        self.conversations.iter().filter(|c| c.used).map(|c| c.unread as usize).sum()
    }

    pub fn toggle_picker(&mut self) {
        self.picker_open = !self.picker_open;
        self.picker_reaction = None;
    }

    pub fn close_picker(&mut self) {
        self.picker_open = false;
        self.picker_reaction = None;
    }

    /// Opens the emoji picker to choose a reaction to message `index` (the "+" of the reaction row).
    pub fn open_reaction_picker(&mut self, index: usize) {
        self.picker_open = true;
        self.picker_reaction = Some(index);
    }

    /// The emoji chosen in the picker becomes the reaction to the message it was opened for.
    pub fn react_with_picker(&mut self, emoji_index: usize) {
        if let Some(message) = self.picker_reaction.take() {
            let (first, _) = font::emoji_chars(emoji_index);
            self.react(message, first);
        }
        self.picker_open = false;
    }

    pub fn scroll_picker(&mut self, notches: i32) {
        let rows = font::emoji_total().div_ceil(PICKER_COLS);
        let last = rows.saturating_sub(PICKER_ROWS) as i32;
        self.picker_row = (self.picker_row as i32 + notches).clamp(0, last) as usize;
    }

    /// Puts emoji number `index` of the picker into the message being written.
    pub fn insert_emoji(&mut self, index: usize) {
        let (first, second) = font::emoji_chars(index);
        self.focus = Focus::Message;
        self.draft.push(first);
        if let Some(second) = second {
            self.draft.push(second);
        }
    }

    pub fn set_banner(&mut self, text: &str) {
        self.banner.clear();
        for c in text.chars() {
            self.banner.push(c);
        }
    }

    pub fn resize(&mut self, window_width: i32, window_height: i32) {
        self.window_height = window_height;
        let conversation_width = window_width - sidebar_width(window_width);
        self.text_limit = (conversation_width * 65 / 100).max(200) - 2 * BUBBLE_PAD_X;
        for index in 0..self.message_count {
            self.recompute_height(index);
        }
        self.list_scroll = self.list_scroll.min(self.max_list_scroll());
        self.conversation_scroll = self.conversation_scroll.min(self.max_conversation_scroll());
        self.panel_scroll = self.panel_scroll.min(self.max_panel_scroll());
    }

    pub fn text_limit(&self) -> i32 {
        self.text_limit
    }

    fn push_message(&mut self, chat: usize, outgoing: bool, delivery: Delivery, text: &str, time: [u8; 5], day: u16, id: &str, reply: &str) {
        if self.message_count == MAX_MESSAGES {
            self.messages.rotate_left(1);
            self.message_count -= 1;
        }
        let mut end = text.len().min(MESSAGE_CAPACITY);
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        let message = &mut self.messages[self.message_count];
        message.chat = chat as u8;
        message.outgoing = outgoing;
        message.delivery = delivery;
        message.time = time;
        message.len = end as u16;
        message.text[..end].copy_from_slice(&text.as_bytes()[..end]);
        store_id(&mut message.id, &mut message.id_len, id);
        store_id(&mut message.reply, &mut message.reply_len, reply);
        message.tally = [0; TALLY];
        message.counts = [0; TALLY];
        message.mine = 0;
        message.starred = false;
        message.deleted = false;
        message.day = if day == 0 { today() } else { day };
        message.kind = 0;
        message.label_len = 0;
        message.thumb = 0;
        let index = self.message_count;
        self.message_count += 1;
        self.recompute_height(index);
    }

    /// The height message `index` takes: its bubble, the quote it holds and the reactions under it.
    fn recompute_height(&mut self, index: usize) {
        let quote_width = self.quote_width(index);
        let message = &self.messages[index];
        let shape = bubble_shape(message.text(), message.time(), message.outgoing, message.starred, self.text_limit, quote_width, message.block());
        let reactions = if message.tally[0] != 0 { REACTION_H } else { 0 };
        self.messages[index].height = (shape.height + reactions) as u16;
    }

    /// The message of `chat` with connection id `id`.
    fn message_with_id(&self, chat: u8, id: &str) -> Option<usize> {
        if id.is_empty() {
            return None;
        }
        self.messages[..self.message_count].iter().position(|m| m.chat == chat && m.id() == id)
    }

    /// The message that `message` answers, when it is one we still have.
    pub fn quoted_by(&self, message: &Message) -> Option<&Message> {
        self.message_with_id(message.chat, message.reply_id()).map(|found| &self.messages[found])
    }

    /// Width the quote block of `message` needs (0 when it quotes nothing we have).
    pub fn quote_width_of(&self, message: &Message) -> i32 {
        let Some(quoted) = self.quoted_by(message) else { return 0 };
        (font::measure(quoted.text(), BODY).max(font::measure(self.author_of(quoted), META)) + 28).min(self.text_limit)
    }

    fn quote_width(&self, index: usize) -> i32 {
        self.quote_width_of(&self.messages[index])
    }

    /// Message number `index` among all the messages.
    pub fn index_message(&self, index: usize) -> &Message {
        &self.messages[index]
    }

    /// The number of `message` among all the messages (for the pop-up over a message).
    pub fn index_of(&self, message: &Message) -> usize {
        (message as *const Message as usize - self.messages.as_ptr() as usize) / core::mem::size_of::<Message>()
    }

    pub fn conversation_name(&self, chat: usize) -> &str {
        self.conversations[chat].name()
    }

    /// Who a message is from, as its quote shows: "Você", or the conversation's name.
    pub fn author_of(&self, message: &Message) -> &str {
        if message.outgoing {
            "Você"
        } else {
            self.conversations[message.chat as usize].name()
        }
    }

    /// Message `id` of `chat` was deleted for everyone (by us, or by the other side).
    pub fn mark_deleted(&mut self, chat: usize, id: &str, by_me: bool) {
        let Some(index) = self.message_with_id(chat as u8, id) else { return };
        let text = if by_me { "Você apagou esta mensagem" } else { "Esta mensagem foi apagada" };
        let message = &mut self.messages[index];
        message.len = text.len() as u16;
        message.text[..text.len()].copy_from_slice(text.as_bytes());
        message.deleted = true;
        message.tally = [0; TALLY];
        message.counts = [0; TALLY];
        message.mine = 0;
        message.starred = false;
        message.reply_len = 0;
        self.recompute_height(index);
    }

    /// Opens the list of conversations to forward message `index` to.
    pub fn open_forward(&mut self, index: usize) {
        let message = &self.messages[index];
        if message.deleted || message.len == 0 {
            return;
        }
        let len = message.len as usize;
        self.forward_text[..len].copy_from_slice(&message.text[..len]);
        self.forward_len = len;
        self.forward_scroll = 0;
        self.forward_open = true;
    }

    /// The conversations offered when forwarding, freshest first, and how many.
    pub fn forward_targets(&self) -> ([u8; MAX_CHATS], usize) {
        let mut targets = [0u8; MAX_CHATS];
        let mut count = 0;
        for (index, conversation) in self.conversations.iter().enumerate().filter(|(_, c)| c.used) {
            let mut at = count;
            while at > 0 && self.conversations[targets[at - 1] as usize].rank < conversation.rank {
                targets[at] = targets[at - 1];
                at -= 1;
            }
            targets[at] = index as u8;
            count += 1;
        }
        (targets, count)
    }

    pub fn scroll_forward(&mut self, notches: i32) {
        let (_, count) = self.forward_targets();
        let max = (count as i32 * FORWARD_ROW_H - FORWARD_LIST_H).max(0);
        self.forward_scroll = (self.forward_scroll + notches * FORWARD_ROW_H / 2).clamp(0, max);
    }

    /// Sends the text being forwarded to conversation `chat` and closes the list.
    pub fn forward_to(&mut self, chat: usize) {
        self.forward_open = false;
        if chat >= MAX_CHATS || !self.conversations[chat].used {
            return;
        }
        let len = self.forward_len;
        let mut line = [0u8; MESSAGE_CAPACITY + 16];
        let head = write_send_head(&mut line, b"SEND\t", chat);
        line[head..head + len].copy_from_slice(&self.forward_text[..len]);
        line[head + len] = b'\n';
        sys::write_all(1, &line[..head + len + 1]);
        let text = self.forward_text;
        if let Ok(text) = core::str::from_utf8(&text[..len]) {
            self.push_message(chat, true, Delivery::Sent, text, clock(sys::unix_seconds()), 0, "", "");
            self.bump(chat);
        }
        if chat == self.selected {
            self.conversation_scroll = self.max_conversation_scroll();
        }
    }

    /// Asks before deleting our message `index` for everyone (the message box has Cancel and Delete).
    pub fn ask_delete(&mut self, index: usize) {
        let message = &self.messages[index];
        if !message.outgoing || message.deleted || message.id_len == 0 {
            return;
        }
        self.dialog_title.clear();
        for c in "Apagar mensagem?".chars() {
            self.dialog_title.push(c);
        }
        self.dialog_body.clear();
        for c in "Esta mensagem será apagada para todos na conversa. Isso não pode ser desfeito.".chars() {
            self.dialog_body.push(c);
        }
        self.dialog_open = true;
        self.confirm_delete = Some(index);
    }

    /// Whether the message box asks something (its main button then deletes or clears).
    pub fn confirming(&self) -> bool {
        self.confirm_delete.is_some() || self.confirm_clear.is_some() || self.confirm_remove.is_some()
    }

    // ---- the conversation's menu

    /// "Silenciar notificações" / "Reativar notificações": the core is told (`MUTE chat 1|0`).
    pub fn toggle_silence(&mut self) {
        let chat = self.selected;
        let silenced = !self.conversations[chat].silenced;
        self.conversations[chat].silenced = silenced;
        write_command(b"MUTE\t", chat, if silenced { b"\t1\n" } else { b"\t0\n" });
    }

    /// "Limpar conversa": asks first.
    pub fn ask_clear(&mut self) {
        self.dialog_title.clear();
        for c in "Limpar esta conversa?".chars() {
            self.dialog_title.push(c);
        }
        self.dialog_body.clear();
        for c in "Todas as mensagens serão removidas desta conversa. As mensagens favoritas também.".chars() {
            self.dialog_body.push(c);
        }
        self.dialog_open = true;
        self.confirm_clear = Some(self.selected);
    }

    /// "Apagar conversa": asks first.
    pub fn ask_remove(&mut self) {
        self.dialog_title.clear();
        for c in "Apagar esta conversa?".chars() {
            self.dialog_title.push(c);
        }
        self.dialog_body.clear();
        for c in "A conversa e todas as suas mensagens serão removidas. Isso não pode ser desfeito.".chars() {
            self.dialog_body.push(c);
        }
        self.dialog_open = true;
        self.confirm_remove = Some(self.selected);
    }

    /// Takes `chat` off the list with its messages and tells the core (`DELETECHAT chat`).
    fn remove_conversation(&mut self, chat: usize) {
        self.clear_messages_of(chat);
        if self.open && self.selected == chat {
            self.draft.clear();
            self.forget_history();
            self.open = false;
            self.picker_open = false;
            self.find_open = false;
        }
        self.conversations[chat] = Conversation::EMPTY;
        write_command(b"DELETECHAT\t", chat, b"\n");
        self.update_visible();
    }

    /// Drops the messages of `chat` from the screen and tells the core (`CLEARCHAT chat`).
    fn clear_conversation(&mut self, chat: usize) {
        self.clear_messages_of(chat);
        write_command(b"CLEARCHAT\t", chat, b"\n");
    }

    fn clear_messages_of(&mut self, chat: usize) {
        let mut kept = 0;
        for index in 0..self.message_count {
            if self.messages[index].chat as usize != chat {
                self.messages.swap(kept, index);
                kept += 1;
            }
        }
        self.message_count = kept;
        self.flash = None;
        self.cancel_reply();
        self.message_menu = None;
        self.conversation_scroll = 0;
    }

    /// The answer to the message box: `true` is its main button (OK, or Delete when it asks).
    pub fn dialog_answer(&mut self, main_button: bool) {
        self.dialog_open = false;
        if let Some(chat) = self.confirm_remove.take() {
            if main_button {
                self.remove_conversation(chat);
            }
        }
        if let Some(chat) = self.confirm_clear.take() {
            if main_button {
                self.clear_conversation(chat);
            }
        }
        if let Some(index) = self.confirm_delete.take() {
            if main_button {
                self.delete_for_everyone(index);
            }
        }
    }

    /// Deletes our message `index` for everyone: the ui shows it as deleted and the core is told
    /// (`DELETE chat id`).
    pub fn delete_for_everyone(&mut self, index: usize) {
        let message = &self.messages[index];
        if !message.outgoing || message.deleted || message.id_len == 0 {
            return;
        }
        let (chat, id_len) = (message.chat as usize, message.id_len as usize);
        let mut line = [0u8; 64];
        let mut at = 0;
        for part in [b"DELETE\t".as_slice(), &[b'0' + (chat / 10) as u8, b'0' + (chat % 10) as u8], b"\t", &message.id[..id_len], b"\n"] {
            line[at..at + part.len()].copy_from_slice(part);
            at += part.len();
        }
        sys::write_all(1, &line[..at]);
        let id = message.id;
        let id = core::str::from_utf8(&id[..id_len]).unwrap_or("");
        self.mark_deleted(chat, id, true);
    }

    /// The reactions of message `id` of `chat` as the core counts them: the emoji we put (empty: none) and
    /// `emoji:count` pairs separated by spaces.
    pub fn set_reactions(&mut self, chat: usize, id: &str, mine: &str, counts: &str) {
        let Some(index) = self.message_with_id(chat as u8, id) else { return };
        let message = &mut self.messages[index];
        message.mine = mine.chars().next().map_or(0, |c| c as u32);
        message.tally = [0; TALLY];
        message.counts = [0; TALLY];
        for (slot, pair) in counts.split(' ').filter(|pair| !pair.is_empty()).take(TALLY).enumerate() {
            let Some((emoji, count)) = pair.rsplit_once(':') else { continue };
            message.tally[slot] = emoji.chars().next().map_or(0, |c| c as u32);
            message.counts[slot] = count.parse().unwrap_or(1);
        }
        self.recompute_height(index);
    }

    /// We react to message `index` with `emoji` (the same one again takes it back): the ui shows it and the
    /// core is told (`REACT chat id emoji`).
    pub fn react(&mut self, index: usize, emoji: char) {
        let message = &self.messages[index];
        if message.id_len == 0 || message.deleted {
            return;
        }
        let code = if message.mine == emoji as u32 { 0 } else { emoji as u32 };
        let (chat, id) = (message.chat as usize, message.id);
        let id = core::str::from_utf8(&id[..message.id_len as usize]).unwrap_or("");
        let mut utf8 = [0u8; 4];
        let shown = if code == 0 { "" } else { emoji.encode_utf8(&mut utf8) };
        let mut line = [0u8; 64];
        let mut at = 0;
        for part in [b"REACT\t".as_slice(), &[b'0' + (chat / 10) as u8, b'0' + (chat % 10) as u8], b"\t", id.as_bytes(), b"\t", shown.as_bytes(), b"\n"] {
            line[at..at + part.len()].copy_from_slice(part);
            at += part.len();
        }
        sys::write_all(1, &line[..at]);
        self.messages[index].take_back_mine();
        if code != 0 {
            self.messages[index].add_mine(code);
        }
        self.recompute_height(index);
    }

    /// Scrolls to message `index` and marks it for a moment (a click on a quote).
    pub fn jump_to_message(&mut self, index: usize) {
        let mut target = None;
        let wanted = self.index_of(&self.messages[index]);
        self.place(|message, y, _| {
            if self.index_of(message) == wanted {
                target = Some(y);
            }
        });
        if let Some(y) = target {
            self.conversation_scroll = (y - self.viewport_height() / 3).clamp(0, self.max_conversation_scroll());
            self.flash = Some(index);
        }
    }

    /// The core gave the connection id to what we sent last in `chat` (oldest message still without one).
    pub fn set_sent_id(&mut self, chat: usize, id: &str) {
        if let Some(message) = self.messages[..self.message_count].iter_mut().find(|m| m.chat as usize == chat && m.outgoing && m.id_len == 0) {
            store_id(&mut message.id, &mut message.id_len, id);
        }
    }

    // ---- favourites

    /// A favourite mark set by the core (`STARRED chat id 1|0`).
    pub fn set_starred(&mut self, chat: usize, id: &str, starred: bool) {
        let Some(index) = self.message_with_id(chat as u8, id) else { return };
        self.messages[index].starred = starred;
        self.recompute_height(index);
    }

    /// Marks (or unmarks) message `index` and tells the core (`STAR chat id 1|0`).
    pub fn toggle_star(&mut self, index: usize) {
        let message = &self.messages[index];
        if message.id_len == 0 || message.deleted {
            return;
        }
        let (chat, id, starred) = (message.chat as usize, message.id, !message.starred);
        let id = core::str::from_utf8(&id[..message.id_len as usize]).unwrap_or("");
        let mut line = [0u8; 64];
        let mut at = 0;
        for part in [b"STAR\t".as_slice(), &[b'0' + (chat / 10) as u8, b'0' + (chat % 10) as u8], b"\t", id.as_bytes(), b"\t", if starred { b"1" } else { b"0" }, b"\n"] {
            line[at..at + part.len()].copy_from_slice(part);
            at += part.len();
        }
        sys::write_all(1, &line[..at]);
        self.messages[index].starred = starred;
        self.recompute_height(index);
    }

    // ---- replying

    /// The message being answered, if a reply is being written.
    pub fn replying_to(&self) -> Option<&Message> {
        let id = core::str::from_utf8(&self.reply_to[..self.reply_to_len as usize]).ok()?;
        self.message_with_id(self.selected as u8, id).map(|index| &self.messages[index])
    }

    /// Height of the bar above the footer that shows the message being answered.
    pub fn reply_bar_h(&self) -> i32 {
        if self.replying_to().is_some() {
            REPLY_BAR_H
        } else {
            0
        }
    }

    pub fn start_reply(&mut self, index: usize) {
        let message = &self.messages[index];
        if message.id_len == 0 || message.deleted {
            return;
        }
        let (id, len) = (message.id, message.id_len);
        self.reply_to = id;
        self.reply_to_len = len;
        self.focus = Focus::Message;
    }

    pub fn cancel_reply(&mut self) {
        self.reply_to_len = 0;
        if self.editing {
            self.editing = false;
            self.draft.clear();
        }
    }

    /// Puts our message `index` in the box to change it.
    pub fn start_edit(&mut self, index: usize) {
        let message = &self.messages[index];
        if !message.outgoing || message.deleted || message.id_len == 0 {
            return;
        }
        let (id, len, text, text_len) = (message.id, message.id_len, message.text, message.len as usize);
        self.cancel_reply();
        self.reply_to = id;
        self.reply_to_len = len;
        self.editing = true;
        self.draft.set(&text[..text_len]);
        self.forget_history();
        self.focus = Focus::Message;
    }

    /// Sends the edited text (`EDIT chat id text`) and shows it in place of the old one.
    fn finish_edit(&mut self) {
        let (id, id_len, len) = (self.reply_to, self.reply_to_len as usize, self.draft.len);
        let Ok(id) = core::str::from_utf8(&id[..id_len]) else { return };
        if let Some(index) = self.message_with_id(self.selected as u8, id) {
            let mut line = [0u8; MESSAGE_CAPACITY + ID_CAPACITY + 16];
            let head = write_send_head(&mut line, b"EDIT\t", self.selected);
            line[head..head + id_len].copy_from_slice(id.as_bytes());
            line[head + id_len] = b'\t';
            let head = head + id_len + 1;
            line[head..head + len].copy_from_slice(&self.draft.bytes[..len]);
            line[head + len] = b'\n';
            sys::write_all(1, &line[..head + len + 1]);
            let message = &mut self.messages[index];
            message.text[..len].copy_from_slice(&self.draft.bytes[..len]);
            message.len = len as u16;
            self.recompute_height(index);
        }
        self.cancel_reply();
        self.forget_history();
    }

    /// The core says message `id` of `chat` now reads `text` (the other side edited it).
    pub fn set_text(&mut self, chat: usize, id: &str, text: &str) {
        let Some(index) = self.message_with_id(chat as u8, id) else { return };
        if self.messages[index].deleted {
            return;
        }
        let mut end = text.len().min(MESSAGE_CAPACITY);
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        let message = &mut self.messages[index];
        message.text[..end].copy_from_slice(&text.as_bytes()[..end]);
        message.len = end as u16;
        self.recompute_height(index);
    }

    /// Copies message `index`'s text to the clipboard.
    pub fn copy_message(&mut self, index: usize) {
        let text = self.messages[index].text();
        let len = text.len().min(MESSAGE_CAPACITY);
        let mut copy = [0u8; MESSAGE_CAPACITY];
        copy[..len].copy_from_slice(&text.as_bytes()[..len]);
        self.clip = copy;
        self.clip_len = len;
        self.clip_pending = true;
    }

    /// Visits the bubbles of the open conversation with their top edge inside the scrolling
    /// content, and whether each one starts a new run from the same sender. Returns the content height.
    pub fn place(&self, mut visit: impl FnMut(&Message, i32, bool)) -> i32 {
        let mut y = CONVERSATION_TOP - DAY_CHIP_ROW;
        let mut previous: Option<bool> = None;
        let mut previous_day: Option<u16> = None;
        for message in self.messages[..self.message_count].iter().filter(|m| m.chat as usize == self.selected) {
            let new_day = previous_day != Some(message.day);
            let starts_run = new_day || previous != Some(message.outgoing);
            if previous.is_some() {
                y += if starts_run { OTHER_SENDER_GAP } else { SAME_SENDER_GAP };
            }
            if new_day {
                y += DAY_CHIP_ROW;
            }
            visit(message, y, starts_run);
            y += message.height as i32;
            previous = Some(message.outgoing);
            previous_day = Some(message.day);
        }
        if previous.is_none() {
            y += DAY_CHIP_ROW;
        }
        y + CONVERSATION_BOTTOM
    }

    /// What the find bar looks for; empty while the bar is closed.
    pub fn find_query(&self) -> &str {
        if self.find_open {
            self.find.text()
        } else {
            ""
        }
    }

    pub fn find_caret(&self) -> (usize, (usize, usize)) {
        (self.find.cursor(), self.find.selection())
    }

    pub fn find_text(&self) -> &str {
        self.find.text()
    }

    /// Whether `message` (of the open conversation) contains what the find bar looks for.
    pub fn is_find_match(&self, message: &Message) -> bool {
        find_from(message.text(), self.find_query(), 0).is_some()
    }

    /// How many messages of the open conversation match.
    pub fn find_count(&self) -> usize {
        self.messages[..self.message_count].iter().filter(|m| m.chat as usize == self.selected && self.is_find_match(m)).count()
    }

    /// Which match is current (0 based, oldest first), if there are any.
    pub fn find_position(&self) -> Option<usize> {
        let count = self.find_count();
        (count > 0).then(|| self.find_current.min(count - 1))
    }

    /// The `n`th matching message of the open conversation, oldest first: its number among the
    /// conversation's messages.
    pub fn is_current_find(&self, message: &Message) -> bool {
        let Some(current) = self.find_position() else { return false };
        self.messages[..self.message_count]
            .iter()
            .filter(|m| m.chat as usize == self.selected && self.is_find_match(m))
            .nth(current)
            .is_some_and(|found| core::ptr::eq(found, message))
    }

    pub fn open_find(&mut self) {
        if !self.open || self.tab != Tab::Chats {
            return;
        }
        self.find_open = true;
        self.find.clear();
        self.find_current = 0;
        self.focus = Focus::Find;
        self.picker_open = false;
    }

    pub fn close_find(&mut self) {
        self.find_open = false;
        self.focus = Focus::Message;
    }

    /// The query changed: start at the newest match and show it.
    fn find_changed(&mut self) {
        self.find_current = self.find_count().saturating_sub(1);
        self.find_jump();
    }

    /// Moves to the next (`forward`, newer) or previous (older) match, wrapping around.
    pub fn find_step(&mut self, forward: bool) {
        let count = self.find_count();
        if count == 0 {
            return;
        }
        let current = self.find_current.min(count - 1);
        self.find_current = if forward { (current + 1) % count } else { (current + count - 1) % count };
        self.find_jump();
    }

    /// Scrolls the conversation so the current match is in view.
    fn find_jump(&mut self) {
        let Some(current) = self.find_position() else { return };
        let mut seen = 0;
        let mut target = None;
        self.place(|message, y, _| {
            if target.is_none() && self.is_find_match(message) {
                if seen == current {
                    target = Some(y);
                }
                seen += 1;
            }
        });
        if let Some(y) = target {
            self.conversation_scroll = (y - self.viewport_height() / 3).clamp(0, self.max_conversation_scroll());
        }
    }

    pub fn viewport_height(&self) -> i32 {
        self.window_height - HEADER_H - INPUT_H - self.reply_bar_h()
    }

    pub fn max_conversation_scroll(&self) -> i32 {
        (self.place(|_, _, _| {}) - self.viewport_height()).max(0)
    }

    pub fn max_list_scroll(&self) -> i32 {
        (self.visible_len as i32 * ROW_H - (self.window_height - LIST_TOP)).max(0)
    }

    pub fn last_message(&self, chat: usize) -> Option<&Message> {
        self.messages[..self.message_count].iter().rev().find(|m| m.chat as usize == chat)
    }

    /// The unsent text of conversation `index`: what is typed if it is the open one, else what was left.
    pub fn draft_of(&self, index: usize) -> &str {
        if self.open && index == self.selected {
            return self.draft.text();
        }
        let conversation = &self.conversations[index];
        core::str::from_utf8(&conversation.draft[..conversation.draft_len as usize]).unwrap_or("")
    }

    /// Puts the text being typed away with the open conversation, and takes out the new one's.
    /// Keeps what is typed with the open conversation.
    fn stash_draft(&mut self) {
        if self.open {
            let (len, previous) = (self.draft.len.min(MESSAGE_CAPACITY - 1), self.selected);
            self.conversations[previous].draft[..len].copy_from_slice(&self.draft.bytes[..len]);
            self.conversations[previous].draft_len = len as u8;
        }
    }

    /// Leaves the open conversation (Esc): what is typed stays as its draft.
    pub fn close_conversation(&mut self) {
        self.info_open = false;
        self.stash_draft();
        self.draft.clear();
        self.forget_history();
        self.open = false;
        self.picker_open = false;
        self.find_open = false;
    }

    /// Whether the conversation is scrolled to its newest message (or close enough).
    pub fn at_bottom(&self) -> bool {
        self.conversation_scroll >= self.max_conversation_scroll() - 8
    }

    pub fn scroll_to_bottom(&mut self) {
        self.conversation_scroll = self.max_conversation_scroll();
        self.below_count = 0;
    }

    fn swap_draft(&mut self, next: usize) {
        self.forget_history();
        self.stash_draft();
        self.draft.clear();
        let stored = self.conversations[next].draft;
        let len = self.conversations[next].draft_len as usize;
        self.draft.set(&stored[..len]);
        self.conversations[next].draft_len = 0;
    }

    pub fn select(&mut self, chat: usize) {
        self.cancel_reply();
        self.message_menu = None;
        self.below_count = 0;
        if self.find_open {
            self.close_find();
        }
        if chat != self.selected || !self.open {
            self.swap_draft(chat);
        }
        self.selected = chat;
        self.open = true;
        self.read_chat(chat);
        self.banner.clear();
        self.picker_open = false;
        self.focus = Focus::Message;
        self.conversation_scroll = self.max_conversation_scroll();
    }

    pub fn scroll_list(&mut self, notches: i32) {
        self.list_scroll = (self.list_scroll + notches * LIST_SCROLL_STEP).clamp(0, self.max_list_scroll());
    }

    pub fn scroll_conversation(&mut self, notches: i32) {
        self.flash = None;
        self.conversation_scroll = (self.conversation_scroll + notches * CONVERSATION_SCROLL_STEP).clamp(0, self.max_conversation_scroll());
        if self.at_bottom() {
            self.below_count = 0;
        }
    }

    fn update_visible(&mut self) {
        self.visible_len = 0;
        for (index, conversation) in self.conversations.iter().enumerate() {
            if conversation.used && (!self.unread_only || conversation.unread > 0) && matches(conversation.name(), self.search.text()) {
                // Insertion sort by rank, highest first: sixteen entries at most.
                let mut at = self.visible_len;
                while at > 0 && self.conversations[self.visible[at - 1] as usize].rank < conversation.rank {
                    self.visible[at] = self.visible[at - 1];
                    at -= 1;
                }
                self.visible[at] = index as u8;
                self.visible_len += 1;
            }
        }
        self.list_scroll = self.list_scroll.min(self.max_list_scroll());
        self.panel_scroll = self.panel_scroll.min(self.max_panel_scroll());
        if self.new_chat {
            self.request_contacts();
        }
    }

    fn insert(&mut self, c: char) {
        if self.dialpad_open && !(c.is_ascii_digit() || c == '+') {
            return;
        }
        match self.focus {
            Focus::Message => {
                self.remember(Change::Type);
                self.draft.push(c);
                // A space ends the word: the next letter starts a new undo step.
                if c == ' ' {
                    self.last_change = Change::Other;
                }
            }
            Focus::Search => {
                self.search.push(c);
                self.update_visible();
            }
            Focus::Find => {
                self.find.push(c);
                self.find_changed();
            }
        }
    }

    fn flush_accent(&mut self) {
        if let Some(accent) = self.pending_accent.take() {
            self.insert(accent);
        }
    }

    /// Sends the draft: the line goes to our stdout (the core will read it) and the message
    /// appears in the open conversation.
    pub fn send(&mut self) {
        let len = self.draft.len;
        if len == 0 || !self.open {
            return;
        }
        if self.editing {
            return self.finish_edit();
        }
        // An answer names the message it answers: `REPLY chat id text`, else `SEND chat text`.
        let mut reply = [0u8; ID_CAPACITY];
        let reply_len = self.replying_to().map_or(0, |quoted| {
            reply[..quoted.id_len as usize].copy_from_slice(&quoted.id[..quoted.id_len as usize]);
            quoted.id_len as usize
        });
        let mut line = [0u8; MESSAGE_CAPACITY + ID_CAPACITY + 16];
        let mut head = write_send_head(&mut line, if reply_len > 0 { b"REPLY\t" } else { b"SEND\t" }, self.selected);
        if reply_len > 0 {
            line[head..head + reply_len].copy_from_slice(&reply[..reply_len]);
            line[head + reply_len] = b'\t';
            head += reply_len + 1;
        }
        line[head..head + len].copy_from_slice(&self.draft.bytes[..len]);
        line[head + len] = b'\n';
        sys::write_all(1, &line[..head + len + 1]);

        let mut text = [0u8; MESSAGE_CAPACITY];
        text[..len].copy_from_slice(&self.draft.bytes[..len]);
        if let (Ok(text), Ok(reply)) = (core::str::from_utf8(&text[..len]), core::str::from_utf8(&reply[..reply_len])) {
            self.push_message(self.selected, true, Delivery::Sent, text, clock(sys::unix_seconds()), 0, "", reply);
            self.bump(self.selected);
        }
        self.draft.clear();
        self.forget_history();
        self.cancel_reply();
        self.conversation_scroll = self.max_conversation_scroll();
    }

    /// Whether a text box has the caret (the blink timer runs only then).
    pub fn editing(&self) -> bool {
        if self.dialog_open || self.call_phase != CallPhase::Idle {
            return false;
        }
        match self.focus {
            Focus::Search => true,
            Focus::Find => self.find_open,
            Focus::Message => self.open && self.tab == Tab::Chats && !self.new_chat,
        }
    }

    fn snap(&self) -> Snap {
        Snap { bytes: self.draft.bytes, len: self.draft.len as u16, cursor: self.draft.cursor as u16, anchor: self.draft.anchor as u16 }
    }

    fn restore(&mut self, state: &Snap) {
        self.draft.bytes = state.bytes;
        self.draft.len = state.len as usize;
        self.draft.cursor = state.cursor as usize;
        self.draft.anchor = state.anchor as usize;
    }

    /// Notes the message box as it is now, before an edit that Ctrl+Z should be able to take back.
    fn remember(&mut self, change: Change) {
        if change == Change::Type && self.last_change == Change::Type {
            return;
        }
        if self.undo_len == UNDO_DEPTH {
            self.undo_states.copy_within(1.., 0);
            self.undo_len -= 1;
        }
        self.undo_states[self.undo_len] = self.snap();
        self.undo_len += 1;
        self.redo_len = 0;
        self.last_change = change;
    }

    fn undo(&mut self) {
        if self.focus != Focus::Message || self.undo_len == 0 {
            return;
        }
        self.undo_len -= 1;
        self.redo_states[self.redo_len.min(UNDO_DEPTH - 1)] = self.snap();
        self.redo_len = (self.redo_len + 1).min(UNDO_DEPTH);
        let state = self.undo_states[self.undo_len];
        self.restore(&state);
        self.last_change = Change::Other;
    }

    fn redo(&mut self) {
        if self.focus != Focus::Message || self.redo_len == 0 {
            return;
        }
        self.redo_len -= 1;
        self.undo_states[self.undo_len.min(UNDO_DEPTH - 1)] = self.snap();
        self.undo_len = (self.undo_len + 1).min(UNDO_DEPTH);
        let state = self.redo_states[self.redo_len];
        self.restore(&state);
        self.last_change = Change::Other;
    }

    fn forget_history(&mut self) {
        self.undo_len = 0;
        self.redo_len = 0;
        self.last_change = Change::Other;
    }

    fn edit(&mut self, edit: Edit) {
        if self.focus == Focus::Message {
            let changes = !matches!(edit, Edit::Left { .. } | Edit::Right { .. } | Edit::Home { .. } | Edit::End { .. } | Edit::SelectAll);
            if changes && self.draft.len > 0 {
                self.remember(Change::Other);
            } else {
                self.last_change = Change::Other;
            }
        }
        match self.focus {
            Focus::Message => apply_edit(&mut self.draft, edit),
            Focus::Search => {
                apply_edit(&mut self.search, edit);
                self.update_visible();
            }
            Focus::Find => {
                apply_edit(&mut self.find, edit);
                self.find_changed();
            }
        }
    }

    /// The text selected in the box that has the caret.
    fn selected_text(&self) -> &str {
        match self.focus {
            Focus::Message => self.draft.selected_text(),
            Focus::Search => self.search.selected_text(),
            Focus::Find => self.find.selected_text(),
        }
    }

    fn copy_selection(&mut self) {
        let text = self.selected_text();
        let len = text.len().min(MESSAGE_CAPACITY);
        if len == 0 {
            return;
        }
        let mut copy = [0u8; MESSAGE_CAPACITY];
        copy[..len].copy_from_slice(&text.as_bytes()[..len]);
        self.clip = copy;
        self.clip_len = len;
        self.clip_pending = true;
    }

    /// Text the window should put on the clipboard now, if a copy or cut just happened.
    pub fn pending_copy(&mut self) -> Option<&str> {
        if !self.clip_pending {
            return None;
        }
        self.clip_pending = false;
        core::str::from_utf8(&self.clip[..self.clip_len]).ok()
    }

    /// Whether the window should ask the clipboard for its text now (Ctrl+V).
    pub fn take_paste_request(&mut self) -> bool {
        core::mem::take(&mut self.paste_wanted)
    }

    /// Text that arrived from the clipboard: typed at the caret, a line break counting as a space.
    pub fn paste(&mut self, text: &str) {
        if !self.editing() {
            return;
        }
        for c in text.chars() {
            self.insert(if matches!(c, '\n' | '\r' | '\t') { ' ' } else { c });
        }
        self.caret_on = true;
    }

    /// The pointer is dragged over a box's text with the button held: the selection grows to `index`.
    pub fn drag_field(&mut self, message: bool, index: usize) {
        if message {
            self.draft.move_to(index, true);
        } else {
            self.search.move_to(index, true);
        }
        self.caret_on = true;
    }

    /// A click on a box's text at byte `index`: the caret; a second click in a row selects the word,
    /// a third the whole text.
    /// A click in the find bar's text box.
    pub fn click_find(&mut self, index: usize, clicks: u8) {
        self.focus = Focus::Find;
        match clicks {
            1 => self.find.move_to(index, false),
            2 => self.find.select_word(index),
            _ => self.find.select_all(),
        }
        self.caret_on = true;
    }

    pub fn drag_find(&mut self, index: usize) {
        self.find.move_to(index, true);
        self.caret_on = true;
    }

    pub fn click_field(&mut self, message: bool, index: usize, clicks: u8) {
        match (message, clicks) {
            (true, 1) => self.draft.move_to(index, false),
            (true, 2) => self.draft.select_word(index),
            (true, _) => self.draft.select_all(),
            (false, 1) => self.search.move_to(index, false),
            (false, 2) => self.search.select_word(index),
            (false, _) => self.search.select_all(),
        }
        self.caret_on = true;
    }

    /// The caret of the message box or the search box, and its selection.
    pub fn message_caret(&self) -> (usize, (usize, usize)) {
        (self.draft.cursor(), self.draft.selection())
    }

    pub fn search_caret(&self) -> (usize, (usize, usize)) {
        (self.search.cursor(), self.search.selection())
    }

    fn edit_key(&mut self, key: EditKey, shift: bool, ctrl: bool) {
        let edit = match key {
            EditKey::Left => Edit::Left { word: ctrl, extend: shift },
            EditKey::Right => Edit::Right { word: ctrl, extend: shift },
            EditKey::Home => Edit::Home { extend: shift },
            EditKey::End => Edit::End { extend: shift },
            EditKey::Delete if ctrl => Edit::DeleteWordForward,
            EditKey::Delete => Edit::DeleteForward,
            EditKey::Backspace => Edit::DeleteWordBack,
        };
        self.edit(edit);
    }

    /// Ctrl and a letter: select all, delete by word or to either end, and the clipboard.
    fn ctrl_key(&mut self, letter: char, shift: bool) {
        match letter {
            'z' if shift => self.redo(),
            'z' => self.undo(),
            'y' => self.redo(),
            'a' => self.edit(Edit::SelectAll),
            'e' => self.edit(Edit::End { extend: false }),
            'w' => self.edit(Edit::DeleteWordBack),
            'u' => self.edit(Edit::DeleteToStart),
            'k' => self.edit(Edit::DeleteToEnd),
            'd' => self.edit(Edit::DeleteForward),
            'h' => self.edit(Edit::Backspace),
            'c' => self.copy_selection(),
            'x' => {
                self.copy_selection();
                self.edit(Edit::Backspace);
            }
            'v' => self.paste_wanted = true,
            'f' => self.open_find(),
            _ => {}
        }
    }

    pub fn key(&mut self, key: Key) {
        self.caret_on = true;
        self.flash = None;
        self.banner.clear();
        if self.forward_open {
            if matches!(key, Key::Escape) {
                self.forward_open = false;
            }
            return;
        }
        if self.dialog_open {
            match key {
                Key::Escape => self.dialog_answer(false),
                Key::Enter if !self.confirming() => self.dialog_open = false,
                _ => {}
            }
            return;
        }
        if self.message_menu.is_some() && matches!(key, Key::Escape) {
            self.message_menu = None;
            return;
        }
        if self.info_open && self.open && matches!(key, Key::Escape) && !self.chat_menu_open && !self.dialog_open {
            self.info_open = false;
            return;
        }
        if self.chat_menu_open && matches!(key, Key::Escape) {
            self.chat_menu_open = false;
            return;
        }
        if self.menu_open && matches!(key, Key::Escape) {
            self.menu_open = false;
            return;
        }
        if (!self.open || self.tab != Tab::Chats) && self.focus == Focus::Message {
            return;
        }
        if matches!(key, Key::Escape) && self.starred_open {
            self.close_starred();
            return;
        }
        if matches!(key, Key::Escape) && self.dialpad_open {
            self.close_dialpad();
            return;
        }
        if matches!(key, Key::Escape) && self.new_chat {
            self.close_new_chat();
            return;
        }
        if matches!(key, Key::Escape) && self.picker_open {
            self.picker_open = false;
            return;
        }
        match key {
            Key::Dead(accent) => match self.pending_accent.replace(accent) {
                Some(previous) if previous == accent => {
                    self.pending_accent = None;
                    self.insert(accent);
                }
                Some(previous) => self.insert(previous),
                None => {}
            },
            Key::Char(c) => match self.pending_accent.take() {
                Some(accent) => match compose(accent, c) {
                    Some(composed) => self.insert(composed),
                    None => {
                        self.insert(accent);
                        if c != ' ' {
                            self.insert(c);
                        }
                    }
                },
                None => self.insert(c),
            },
            other => {
                self.flush_accent();
                match other {
                    Key::Backspace => self.edit(Edit::Backspace),
                    Key::Edit { key, shift, ctrl } => self.edit_key(key, shift, ctrl),
                    Key::Ctrl(letter, shift) => self.ctrl_key(letter, shift),
                    Key::Enter if self.focus == Focus::Message => self.send(),
                    Key::Enter if self.focus == Focus::Find => self.find_step(false),
                    Key::Escape if self.focus == Focus::Find => self.close_find(),
                    Key::Up if self.focus == Focus::Find => self.find_step(false),
                    Key::Down if self.focus == Focus::Find => self.find_step(true),
                    Key::Escape if self.focus == Focus::Message && self.reply_to_len > 0 => self.cancel_reply(),
                    Key::Escape if self.focus == Focus::Message && self.open && self.tab == Tab::Chats => self.close_conversation(),
                    Key::Escape if self.focus == Focus::Search => {
                        self.search.clear();
                        self.update_visible();
                        self.focus = Focus::Message;
                    }
                    Key::Up => self.scroll_conversation(-1),
                    Key::Down => self.scroll_conversation(1),
                    Key::PageUp => self.scroll_conversation(-(self.viewport_height() / CONVERSATION_SCROLL_STEP).max(1)),
                    Key::PageDown => self.scroll_conversation((self.viewport_height() / CONVERSATION_SCROLL_STEP).max(1)),
                    _ => {}
                }
            }
        }
    }
}
