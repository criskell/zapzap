//! Everything that is drawn, and the hit test that goes with it.

use zapzap_qr::Qr;

use crate::canvas::Canvas;
use crate::chat::{
    find_from, FORWARD_LIST_H, FORWARD_ROW_H, QUICK_REACTIONS, QUOTE_H, REPLY_BAR_H,
    bubble_shape, meta_width, sidebar_width, CallKind, CallPhase, CallsItem, ChannelsItem, Chat, ContactsItem, STARRED_ROW_H, Delivery, Focus, StatusItem, Tab, BODY, CALLS_ROW_H, CALLS_VIEW_TOP, CHANNEL_ROW_H, SETTINGS_ENTRIES, SETTINGS_LOGOUT_H, SETTINGS_PHOTO_TOP, SETTINGS_ROWS_TOP, SETTINGS_ROW_H, STATUS_ROW_H, NEW_CHAT_ACTIONS, NEW_CHAT_ROW_H, BUBBLE_PAD_X, BUBBLE_PAD_Y, HEADER_H, INPUT_H, LINE_HEIGHT, LIST_TOP,
    META, PICKER_CELL, PICKER_COLS, PICKER_PAD, PICKER_ROWS, RAIL_W, ROW_H, TICKS_WIDTH,
};
use crate::font::{self, BOLD, MEDIUM};
use crate::icon_ids::Icon;
use crate::icons;

// Measured on WhatsApp Web's dark theme: rail and bars #1d1f1f, list panel #161717, accent #21c063,
// chips #103529 / #d9fdd3, highlights and the search pill are white at 10%, secondary text white at 60%.
// The bubble colours and the conversation body were not measured (no conversation was opened).
const BG_APP: u32 = 0x161717;
const BG_SIDEBAR: u32 = 0x161717;
const BG_PANEL: u32 = 0x1d1f1f;
const BG_CHAT: u32 = 0x161717;
const BG_SELECTED: u32 = 0x2d2e2e;
const BG_CHIP: u32 = 0x182229;
const RAIL_ACTIVE: u32 = 0x343535;
const INPUT_PILL: u32 = 0x242626;
const CHIP_ACTIVE: u32 = 0x103529;
const CHIP_ACTIVE_TEXT: u32 = 0xd9fdd3;
const CHIP_BORDER: u32 = 0x2d2e2e;
const BUBBLE_IN: u32 = 0x242626;
const BUBBLE_OUT: u32 = 0x144d37;
const TEXT_PRIMARY: u32 = 0xfafafa;
const TEXT_SECONDARY: u32 = 0xa2a3a3;
const TEXT_OUT_META: u32 = 0xa1b8af;
const DIVIDER: u32 = 0x2d2e2e;
const ACCENT: u32 = 0x21c063;
const ON_ACCENT: u32 = 0x0b0c0c;
const READ_TICK: u32 = 0x53bdeb;
const SCROLLBAR: u32 = 0x4a4b4b;
/// White at 60% and 20% over the panel colour.
const PANEL_SUBTLE: u32 = 0xa5a6a6;
const PANEL_ICON: u32 = 0x4a4c4c;
const MISSED: u32 = 0xfb5061;
const STORY_SEEN: u32 = 0x424445;
const VERIFIED: u32 = 0x007bfc;
const META_PURPLE: u32 = 0x9b5cf6;
const FOLLOW_BG: u32 = 0x103529;
const BANNER_BG: u32 = 0x4a2a2e;
const BANNER_TEXT: u32 = 0xf5c2c7;
const AVATAR_COLORS: [u32; 8] = [0x2e7d6b, 0x7b5ea7, 0xc2683c, 0x3c7fc2, 0xb5485a, 0x8a8f3a, 0x2f8f9d, 0x9a6b3a];

const NAME: u16 = 16;
const NAME_BOLD: u16 = 16 | BOLD;
const TITLE: u16 = 17 | BOLD;
const LIST_TITLE: u16 = 22 | BOLD;
const CHIP: u16 = 14 | MEDIUM;
const TINY_MEDIUM: u16 = 12 | MEDIUM;
const SECTION: u16 = 16 | MEDIUM;
const SCREEN_TITLE: u16 = 22;
const EMPTY_TITLE: u16 = 32;
const LABEL: u16 = 12;
/// The text box of the footer is 15 px in WhatsApp Web (measured); message text is smaller.
const INPUT_TEXT: u16 = 15;
const INITIAL_BIG: u16 = 20 | BOLD;
const INITIAL_SMALL: u16 = 17 | BOLD;
const PREVIEW: u16 = 14;
const NOTICE: u16 = 17;

const BUBBLE_MARGIN: i32 = 28;
const BUBBLE_RADIUS: i32 = 8;
const TAIL: i32 = 8;
const SEND_RADIUS: i32 = 20;

const QR_MODULE_PX: i32 = 6;
const QR_QUIET_MODULES: i32 = 4;
const QR_DARK: u32 = 0x000000;
const QR_LIGHT: u32 = 0xffffff;

pub fn notice(c: &mut Canvas, text: &str) {
    c.fill_rect(0, 0, c.w, c.h, BG_APP);
    centered(c, c.w / 2, c.h / 2, NOTICE, TEXT_SECONDARY, text);
}

pub fn login(c: &mut Canvas, qr: &Qr) {
    let side = (zapzap_qr::SIZE as i32 + 2 * QR_QUIET_MODULES) * QR_MODULE_PX;
    let left = (c.w - side) / 2;
    let top = (c.h - side) / 2;
    c.fill_rect(0, 0, c.w, c.h, BG_APP);
    c.fill_round_rect(left, top, side, side, 12, QR_LIGHT);

    let (origin_x, origin_y) = (left + QR_QUIET_MODULES * QR_MODULE_PX, top + QR_QUIET_MODULES * QR_MODULE_PX);
    let first = ((c.y0 - origin_y) / QR_MODULE_PX).max(0) as usize;
    let last = (((c.y0 + c.rows - origin_y) / QR_MODULE_PX + 1).max(0) as usize).min(zapzap_qr::SIZE);
    for module_y in first..last {
        for module_x in 0..zapzap_qr::SIZE {
            if qr.is_dark(module_x, module_y) {
                let (x, y) = (origin_x + module_x as i32 * QR_MODULE_PX, origin_y + module_y as i32 * QR_MODULE_PX);
                c.fill_rect(x, y, QR_MODULE_PX, QR_MODULE_PX, QR_DARK);
            }
        }
    }

    centered(c, c.w / 2, top - 28, TITLE, TEXT_PRIMARY, "Conecte o ZapZap ao WhatsApp");
    centered(c, c.w / 2, top + side + 32, PREVIEW, TEXT_SECONDARY, "Abra o WhatsApp > Aparelhos conectados > Conectar um aparelho");
}

pub fn chat_screen(c: &mut Canvas, chat: &Chat) {
    let side = sidebar_width(c.w);
    c.fill_rect(0, 0, c.w, c.h, BG_PANEL);
    match chat.tab {
        Tab::Chats => {
            if chat.open {
                conversation(c, side, chat);
            } else {
                welcome(c, side, chat);
            }
            if chat.new_chat {
                new_chat_sidebar(c, side, chat);
            } else if chat.starred_open {
                starred_sidebar(c, side, chat);
            } else {
                sidebar(c, side, chat);
            }
        }
        Tab::Calls => {
            calls_pane(c, side);
            if chat.dialpad_open {
                dialpad_sidebar(c, side, chat);
            } else {
                calls_sidebar(c, side, chat);
            }
        }
        Tab::Status => {
            empty_pane(c, side, Icon::StatusBig, "Compartilhe atualizações de status", "Compartilhe fotos, vídeos e textos que desaparecem após 24 horas.");
            status_sidebar(c, side, chat);
        }
        Tab::Channels => {
            empty_pane(
                c,
                side,
                Icon::ChannelsBig,
                "Encontrar canais",
                "Entretenimento, esportes, notícias, estilo de vida, pessoas e muito mais. Siga os canais do seu interesse.",
            );
            channels_sidebar(c, side, chat);
        }
        Tab::Settings => {
            welcome(c, side, chat);
            settings_sidebar(c, side, chat);
        }
    }
    rail(c, chat);
    if chat.menu_open {
        header_menu(c, side, chat.tab);
    }
    match chat.call_phase {
        CallPhase::Idle => {}
        CallPhase::Incoming => incoming_call(c, chat),
        CallPhase::Ringing | CallPhase::Active | CallPhase::Ended => call_screen(c, chat),
    }
    if chat.info_open && chat.open && chat.tab == Tab::Chats {
        info_panel(c, chat);
    }
    if chat.chat_menu_open {
        conversation_menu(c, chat);
    }
    if chat.forward_open {
        forward_dialog(c, chat);
    }
    if chat.dialog_open {
        dialog(c, chat);
    }
}

/// The dropdown of the three-dots button: (icon, label), and whether a divider comes after it.
const CHATS_MENU: [(Icon, &str, bool); 7] = [
    (Icon::MenuGroup, "Novo grupo", false),
    (Icon::MenuArchive, "Arquivadas", false),
    (Icon::MenuStar, "Mensagens favoritas", false),
    (Icon::MenuSelect, "Selecionar conversas", false),
    (Icon::MenuChats, "Marcar todas como lidas", true),
    (Icon::MenuLock, "Bloqueio do app", false),
    (Icon::MenuLogout, "Desconectar", false),
];
const STATUS_MENU: [(Icon, &str, bool); 1] = [(Icon::MenuLock, "Privacidade do Status", false)];

fn header_menu_width(entries: &[(Icon, &str, bool)]) -> i32 {
    let widest = entries.iter().map(|&(_, label, _)| font::measure_exact(label, PREVIEW)).max().unwrap_or(0);
    (widest * 2 + 64) / 128 + 61
}

/// The entry of the chats menu (by position) a click at (`x`, `y`) is on; entries after the divider are not clickable yet.
pub fn chats_menu_entry_at(x: i32, y: i32, side: i32) -> Option<usize> {
    let left = side - 107;
    if x < left || x >= left + header_menu_width(&CHATS_MENU) || y < 61 {
        return None;
    }
    let slot = (y - 61) / 36;
    (slot <= 4).then_some(slot as usize)
}

fn header_menu(c: &mut Canvas, side: i32, tab: Tab) {
    let entries: &[(Icon, &str, bool)] = if tab == Tab::Status { &STATUS_MENU } else { &CHATS_MENU };
    let width = header_menu_width(entries);
    let dividers = entries.iter().filter(|entry| entry.2).count() as i32;
    let height = 10 + entries.len() as i32 * 36 + dividers * 9;
    let (x, top) = (side - 107, 56);
    c.fill_round_rect(x, top, width, height, 16, CHIP_BORDER);
    c.fill_round_rect(x + 1, top + 1, width - 2, height - 2, 15, BG_SIDEBAR);
    let mut y = top + 5;
    for &(icon, label, divider) in entries {
        icons::draw(c, x + 16, y + 9, icon, TEXT_PRIMARY);
        font::draw(c, x + 45, y + 23, PREVIEW, TEXT_PRIMARY, label);
        y += 36;
        if divider {
            c.fill_rect(x + 13, y + 4, width - 26, 1, CHIP_BORDER);
            y += 9;
        }
    }
}

/// The dropdown of the three dots in the conversation header.
const CONVERSATION_MENU_W: i32 = 228;

fn conversation_menu_labels(chat: &Chat) -> [&'static str; 5] {
    ["Dados do contato", if chat.conversation(chat.selected).silenced { "Reativar notificações" } else { "Silenciar notificações" }, "Fechar conversa", "Limpar conversa", "Apagar conversa"]
}

fn conversation_menu(c: &mut Canvas, chat: &Chat) {
    let labels = conversation_menu_labels(chat);
    let (w, h) = (CONVERSATION_MENU_W, 10 + labels.len() as i32 * 36);
    let (x, top) = (c.w - w - 12, 56);
    c.fill_round_rect(x, top, w, h, 16, CHIP_BORDER);
    c.fill_round_rect(x + 1, top + 1, w - 2, h - 2, 15, BG_SIDEBAR);
    for (slot, label) in labels.into_iter().enumerate() {
        font::draw(c, x + 24, top + 5 + slot as i32 * 36 + 23, PREVIEW, TEXT_PRIMARY, label);
    }
}

/// The entry of the conversation menu a click at (`x`, `y`) is on.
pub fn conversation_menu_entry_at(x: i32, y: i32, window_w: i32) -> Option<usize> {
    let left = window_w - CONVERSATION_MENU_W - 12;
    if x < left || x >= left + CONVERSATION_MENU_W || y < 61 {
        return None;
    }
    let slot = (y - 61) / 36;
    (slot < 5).then_some(slot as usize)
}

// ---- the contact's panel

/// Width of the panel over the right edge of the conversation, and the height of its rows.
pub const INFO_W: i32 = 360;
const INFO_ROW_H: i32 = 56;
const INFO_ROWS_TOP: i32 = 344;

/// Whether a point is under the contact's panel (which then takes the click).
pub fn over_info(chat: &Chat, x: i32, window_w: i32) -> bool {
    chat.info_open && chat.open && chat.tab == Tab::Chats && x >= window_w - INFO_W
}

fn info_panel(c: &mut Canvas, chat: &Chat) {
    let x0 = c.w - INFO_W;
    let info = chat.conversation(chat.selected);
    c.fill_rect(x0, 0, INFO_W, c.h, BG_SIDEBAR);
    c.fill_rect(x0, 0, 1, c.h, DIVIDER);
    cross(c, x0 + 32, HEADER_H / 2, TEXT_PRIMARY);
    font::draw(c, x0 + 60, 37, NAME, TEXT_PRIMARY, "Dados do contato");
    c.fill_rect(x0 + 1, HEADER_H, INFO_W - 1, 1, DIVIDER);
    let center = x0 + INFO_W / 2;
    c.fill_circle(center, 164, 80, AVATAR_COLORS[chat.selected % AVATAR_COLORS.len()]);
    let mut utf8 = [0u8; 4];
    let initial = info.name().chars().next().unwrap_or('?').encode_utf8(&mut utf8);
    centered(c, center, 164 + 10, SCREEN_TITLE, TEXT_PRIMARY, initial);
    if font::measure(info.name(), SCREEN_TITLE) <= INFO_W - 48 {
        centered(c, center, 290, SCREEN_TITLE, TEXT_PRIMARY, info.name());
    } else {
        fitted(c, x0 + 24, 290, SCREEN_TITLE, TEXT_PRIMARY, info.name(), INFO_W - 48);
    }
    centered(c, center, 318, PREVIEW, TEXT_SECONDARY, info.status());
    c.fill_rect(x0 + 1, INFO_ROWS_TOP - 8, INFO_W - 1, 1, DIVIDER);
    let right = c.w - 24;
    for row in 0..4 {
        let y = INFO_ROWS_TOP + row * INFO_ROW_H;
        let baseline = y + 33;
        match row {
            0 => {
                font::draw(c, x0 + 24, baseline, NAME, TEXT_PRIMARY, "Mensagens favoritas");
                let mut digits = [0u8; 3];
                let text = number_text(chat.starred_count(), &mut digits);
                font::draw(c, right - font::measure(text, PREVIEW), baseline, PREVIEW, TEXT_SECONDARY, text);
            }
            1 => {
                font::draw(c, x0 + 24, baseline, NAME, TEXT_PRIMARY, "Silenciar notificações");
                let on = info.silenced;
                c.fill_round_rect(right - 36, y + 18, 36, 20, 10, if on { ACCENT } else { RAIL_ACTIVE });
                c.fill_circle(right - if on { 10 } else { 26 }, y + 28, 7, if on { ON_ACCENT } else { TEXT_SECONDARY });
            }
            2 => {
                c.fill_rect(x0 + 1, y - 8 + 4, INFO_W - 1, 1, DIVIDER);
                font::draw(c, x0 + 24, baseline + 8, NAME, MISSED, "Limpar conversa");
            }
            _ => font::draw(c, x0 + 24, baseline + 8, NAME, MISSED, "Apagar conversa"),
        }
    }
}

/// Which row of the panel a click is on (its close button is `Hit::InfoClose`).
fn info_row_at(y: i32) -> Option<usize> {
    let row = (y - INFO_ROWS_TOP) / INFO_ROW_H;
    (y >= INFO_ROWS_TOP && row < 4).then_some(row as usize)
}

// ---- text helpers

fn centered(c: &mut Canvas, center_x: i32, baseline: i32, style: u16, color: u32, text: &str) {
    font::draw(c, center_x - font::measure(text, style) / 2, baseline, style, color, text);
}

/// `text` cut to `max_width`, with an ellipsis when something was left out.
fn fitted(c: &mut Canvas, x: i32, baseline: i32, style: u16, color: u32, text: &str, max_width: i32) {
    if font::measure(text, style) <= max_width {
        return font::draw(c, x, baseline, style, color, text);
    }
    let room = max_width - font::measure("…", style);
    let mut end = 0;
    for (index, ch) in text.char_indices() {
        if font::measure(&text[..index + ch.len_utf8()], style) > room {
            break;
        }
        end = index + ch.len_utf8();
    }
    font::draw(c, x, baseline, style, color, &text[..end]);
    font::draw(c, x + font::measure(&text[..end], style), baseline, style, color, "…");
}

/// Selection colour behind highlighted text: the accent at about a third over the box.
const SELECTION: u32 = 0x245c3c;
/// Text found in the conversation: every match, and the current one.
const FIND_MATCH: u32 = 0x6b5a1e;
const FIND_CURRENT: u32 = 0xb08d1a;

/// The part of `text` shown in a box `max_width` wide, chosen so that the caret at byte `caret` is
/// inside it: (start, end) byte offsets. The text scrolls only as far as the caret needs.
fn visible_range(text: &str, caret: usize, style: u16, max_width: i32) -> (usize, usize) {
    let mut start = 0;
    while start < caret && font::measure(&text[start..caret], style) > max_width - 4 {
        start += text[start..].chars().next().map_or(1, char::len_utf8);
    }
    let mut end = text.len();
    while end > caret && font::measure(&text[start..end], style) > max_width {
        end -= text[..end].chars().next_back().map_or(1, char::len_utf8);
    }
    (start, end)
}

/// The text caret: a thin bar as tall as the text's ascent and descent, its top and height in `extent`.
fn draw_caret(c: &mut Canvas, x: i32, extent: (i32, i32)) {
    c.fill_rect(x, extent.0, 1, extent.1, TEXT_PRIMARY);
}

/// Draws a text box's content: the text (scrolled to keep the caret in view), the selection behind
/// it, and the blinking caret. `top` and `height` are the extent of the selection and the caret.
fn edit_text(c: &mut Canvas, text_x: i32, baseline: i32, style: u16, text: &str, caret: (usize, (usize, usize)), max_width: i32, extent: (i32, i32), show_caret: bool) {
    let (cursor, (from, to)) = caret;
    let (start, end) = visible_range(text, cursor, style, max_width);
    let x_at = |at: usize| text_x + font::measure(&text[start..at.clamp(start, end)], style);
    if from != to {
        let (x0, x1) = (x_at(from), x_at(to));
        c.fill_rect(x0, extent.0, x1 - x0, extent.1, SELECTION);
    }
    font::draw(c, text_x, baseline, style, TEXT_PRIMARY, &text[start..end]);
    if show_caret {
        draw_caret(c, x_at(cursor) + 1, extent);
    }
}

/// The byte offset a click at `dx` pixels from the start of the shown text lands on: the nearest gap
/// between characters.
fn caret_at(text: &str, caret: usize, style: u16, max_width: i32, dx: i32) -> usize {
    let (start, end) = visible_range(text, caret, style, max_width);
    let mut best = (start, i32::MAX);
    for (offset, _) in text[start..end].char_indices().chain(core::iter::once((end - start, ' '))) {
        let gap = (font::measure(&text[start..start + offset], style) - dx).abs();
        if gap < best.1 {
            best = (start + offset, gap);
        }
    }
    best.0
}

/// The byte offset in the message box that a click at `x` lands on.
pub fn message_caret_at(chat: &Chat, x: i32, window_w: i32) -> usize {
    let side = sidebar_width(window_w);
    let (box_x, box_w) = message_box(side, window_w);
    caret_at(chat.draft.text(), chat.message_caret().0, INPUT_TEXT, box_w - TEXT_INSET - 64, x - (box_x + TEXT_INSET))
}

/// The byte offset in the search box that a click at `x` lands on.
pub fn search_caret_at(chat: &Chat, x: i32, window_w: i32, icon: bool) -> usize {
    let (box_x, _, box_w, _) = search_box(sidebar_width(window_w));
    caret_at(chat.search.text(), chat.search_caret().0, PREVIEW, box_w - 80, x - (box_x + if icon { 48 } else { 14 }))
}

fn avatar(c: &mut Canvas, cx: i32, cy: i32, radius: i32, color: u32, name: &str) {
    c.fill_circle(cx, cy, radius, color);
    let mut utf8 = [0u8; 4];
    let initial = name.chars().next().unwrap_or('?').encode_utf8(&mut utf8);
    let (style, drop) = if radius >= 24 { (INITIAL_BIG, 7) } else { (INITIAL_SMALL, 6) };
    centered(c, cx, cy + drop, style, TEXT_PRIMARY, initial);
}

fn scrollbar(c: &mut Canvas, x: i32, top: i32, height: i32, offset: i32, max_offset: i32, content: i32) {
    if max_offset <= 0 {
        return;
    }
    let thumb = (height * height / content).clamp(32, height);
    c.fill_round_rect(x, top + (height - thumb) * offset / max_offset, 4, thumb, 2, SCROLLBAR);
}

// ---- icons, built from rectangles and circles

fn icon_smiley(c: &mut Canvas, cx: i32, cy: i32, color: u32, background: u32) {
    c.fill_circle(cx, cy, 10, color);
    c.fill_circle(cx, cy, 8, background);
    c.fill_circle(cx - 4, cy - 3, 1, color);
    c.fill_circle(cx + 4, cy - 3, 1, color);
    c.fill_rect(cx - 5, cy + 2, 2, 2, color);
    c.fill_rect(cx - 3, cy + 4, 6, 2, color);
    c.fill_rect(cx + 3, cy + 2, 2, 2, color);
}

fn icon_mic(c: &mut Canvas, cx: i32, cy: i32, color: u32) {
    c.fill_round_rect(cx - 4, cy - 10, 8, 14, 4, color);
    c.fill_rect(cx - 7, cy - 2, 2, 6, color);
    c.fill_rect(cx + 5, cy - 2, 2, 6, color);
    c.fill_rect(cx - 5, cy + 4, 10, 2, color);
    c.fill_rect(cx - 1, cy + 6, 2, 4, color);
    c.fill_rect(cx - 4, cy + 10, 8, 2, color);
}

fn icon_send(c: &mut Canvas, cx: i32, cy: i32) {
    c.fill_circle(cx, cy, SEND_RADIUS, ACCENT);
    for dy in -7..=7i32 {
        c.fill_rect(cx - 6, cy + dy, 14 * (7 - dy.abs()) / 7 + 1, 1, TEXT_PRIMARY);
    }
}

/// One check mark; two of them, offset, make the "delivered" double tick.
fn icon_video(c: &mut Canvas, cx: i32, cy: i32, color: u32) {
    c.fill_round_rect(cx - 10, cy - 6, 14, 12, 3, color);
    for i in 0..6 {
        c.fill_rect(cx + 5, cy - 3 - i / 2 + i / 3, 6, 6 + i / 2, color);
    }
}

fn icon_plus(c: &mut Canvas, cx: i32, cy: i32, color: u32) {
    c.fill_rect(cx - 1, cy - 8, 2, 16, color);
    c.fill_rect(cx - 8, cy - 1, 16, 2, color);
}

fn number_text(n: usize, buffer: &mut [u8; 3]) -> &str {
    let n = n.min(999);
    let mut len = 0;
    if n >= 100 {
        buffer[len] = b'0' + (n / 100) as u8;
        len += 1;
    }
    if n >= 10 {
        buffer[len] = b'0' + (n / 10 % 10) as u8;
        len += 1;
    }
    buffer[len] = b'0' + (n % 10) as u8;
    core::str::from_utf8(&buffer[..len + 1]).unwrap_or("")
}

// ---- sidebar

pub fn search_box(sidebar_w: i32) -> (i32, i32, i32, i32) {
    (RAIL_W + 21, HEADER_H + 2, sidebar_w - RAIL_W - 40, 40)
}

const CHIP_Y: i32 = 116;
const CHIP_H: i32 = 32;
const CHIP_PAD: i32 = 16;
const CHIP_GAP: i32 = 8;

/// The filter chips "Tudo" and "Não lidas", the second one carrying the unread count.
pub fn chip_rects(unread: usize) -> [(i32, i32, i32, i32); 2] {
    let first_w = font::measure("Tudo", CHIP) + 2 * CHIP_PAD;
    let mut digits = [0u8; 3];
    let count_w = if unread > 0 { 6 + font::measure(number_text(unread, &mut digits), META) } else { 0 };
    let second_w = font::measure("Não lidas", CHIP) + count_w + 2 * CHIP_PAD;
    let x = RAIL_W + 21;
    [(x, CHIP_Y, first_w, CHIP_H), (x + first_w + CHIP_GAP, CHIP_Y, second_w, CHIP_H)]
}

fn chips(c: &mut Canvas, chat: &Chat) {
    let unread = chat.unread_total();
    for (index, (x, y, w, h)) in chip_rects(unread).into_iter().enumerate() {
        let active = chat.unread_only == (index == 1);
        c.fill_round_rect(x, y, w, h, h / 2, CHIP_BORDER);
        c.fill_round_rect(x + 1, y + 1, w - 2, h - 2, h / 2 - 1, if active { CHIP_ACTIVE } else { BG_SIDEBAR });
        let color = if active { CHIP_ACTIVE_TEXT } else { TEXT_SECONDARY };
        let label = if index == 0 { "Tudo" } else { "Não lidas" };
        font::draw(c, x + CHIP_PAD, y + 21, CHIP, color, label);
        if index == 1 && unread > 0 {
            let mut digits = [0u8; 3];
            font::draw(c, x + CHIP_PAD + font::measure(label, CHIP) + 6, y + 21, META, TEXT_SECONDARY, number_text(unread, &mut digits));
        }
    }
}

/// The search pill under the header, with `placeholder` while it is empty.
fn search_field(c: &mut Canvas, side: i32, chat: &Chat, placeholder: &str, icon: bool) {
    let (x, y, box_w, box_h) = search_box(side);
    let focused = chat.focus == Focus::Search;
    let pill = BG_SELECTED;
    if focused {
        c.fill_round_rect(x - 2, y - 2, box_w + 4, box_h + 4, box_h / 2 + 2, ACCENT);
    }
    c.fill_round_rect(x, y, box_w, box_h, box_h / 2, pill);
    if icon {
        icons::draw_centered(c, x + 26, y + box_h / 2, Icon::SearchSmall, TEXT_SECONDARY);
    }
    let text_x = if icon { x + 48 } else { x + 14 };
    let baseline = y + box_h / 2 + 5;
    let query = chat.search.text();
    if query.is_empty() {
        fitted(c, text_x, baseline, PREVIEW, TEXT_SECONDARY, placeholder, box_w - 72);
        if focused && chat.caret_on && chat.window_focused {
            draw_caret(c, text_x, (baseline - 13, 17));
        }
    } else {
        edit_text(c, text_x, baseline, PREVIEW, query, chat.search_caret(), box_w - 80, (baseline - 13, 17), focused && chat.caret_on && chat.window_focused);
    }
}

fn sidebar(c: &mut Canvas, side: i32, chat: &Chat) {
    c.fill_rect(RAIL_W, 0, side - RAIL_W, c.h, BG_SIDEBAR);
    for (row, &index) in chat.visible[..chat.visible_len].iter().enumerate() {
        let y = LIST_TOP + row as i32 * ROW_H - chat.list_scroll;
        if y >= c.y0 + c.rows {
            break;
        }
        if y + ROW_H > c.y0 {
            conversation_row(c, side, y, index as usize, chat);
        }
    }
    if chat.visible_len == 0 {
        let text = if !chat.search.text().is_empty() {
            "Nenhuma conversa encontrada"
        } else if chat.unread_only {
            "Nenhuma conversa não lida"
        } else {
            "Nenhuma conversa ainda"
        };
        centered(c, (RAIL_W + side) / 2, LIST_TOP + 60, PREVIEW, TEXT_SECONDARY, text);
    }

    c.fill_rect(RAIL_W, 0, side - RAIL_W, LIST_TOP, BG_SIDEBAR);
    font::draw(c, RAIL_W + 20, 41, LIST_TITLE, TEXT_PRIMARY, "ZapZap");
    if chat.menu_open {
        c.fill_circle(side - 87, HEADER_H / 2, 20, BG_SELECTED);
    }
    icons::draw_centered(c, side - 87, HEADER_H / 2, Icon::Menu, TEXT_PRIMARY);
    c.fill_circle(side - 39, HEADER_H / 2, 20, ACCENT);
    icons::draw_centered(c, side - 39, HEADER_H / 2, Icon::NewChat, ON_ACCENT);

    search_field(c, side, chat, "Pesquisar ou começar uma nova conversa", true);
    chips(c, chat);
    scrollbar(c, side - 7, LIST_TOP, c.h - LIST_TOP, chat.list_scroll, chat.max_list_scroll(), chat.visible_len as i32 * ROW_H);
    c.fill_rect(side - 1, 0, 1, c.h, DIVIDER);
}

/// Centres of the rail buttons, top to bottom: chats, calls, status, channels.
const RAIL_TABS: [(Tab, i32); 4] = [(Tab::Chats, 30), (Tab::Calls, 74), (Tab::Status, 122), (Tab::Channels, 166)];
/// The rail button of a tab, and its icon: filled while it is the open one.

fn rail(c: &mut Canvas, chat: &Chat) {
    c.fill_rect(0, 0, RAIL_W, c.h, BG_PANEL);
    for (tab, cy) in RAIL_TABS {
        let active = chat.tab == tab;
        if active {
            c.fill_circle(32, cy, 20, RAIL_ACTIVE);
        }
        let (icon, color) = match (tab, active) {
            (Tab::Chats, true) => (Icon::Chats, TEXT_PRIMARY),
            (Tab::Chats, false) => (Icon::ChatsOutline, TEXT_SECONDARY),
            (Tab::Calls, true) => (Icon::CallsFilled, TEXT_PRIMARY),
            (Tab::Calls, false) => (Icon::Calls, TEXT_SECONDARY),
            (Tab::Status, true) => (Icon::StatusFilled, TEXT_PRIMARY),
            (Tab::Status, false) => (Icon::Status, TEXT_SECONDARY),
            (Tab::Channels, true) => (Icon::ChannelsFilled, TEXT_PRIMARY),
            (Tab::Channels, false) => (Icon::Channels, TEXT_SECONDARY),
            (Tab::Settings, _) => continue,
        };
        icons::draw_centered(c, 32, cy, icon, color);
    }
    let unread = chat.unread_total();
    if unread > 0 {
        let mut digits = [0u8; 3];
        let text = number_text(unread.min(99), &mut digits);
        let w = (font::measure(text, TINY_MEDIUM) + 6).max(20);
        c.fill_round_rect(34, 4, w + 4, 24, 12, BG_PANEL);
        c.fill_round_rect(36, 6, w, 20, 10, ACCENT);
        centered(c, 36 + w / 2, 20, TINY_MEDIUM, ON_ACCENT, text);
    }
    for (unseen, cy) in [(chat.has_unseen_status(), 122), (chat.has_unread_channel(), 166)] {
        if unseen && chat.tab != Tab::Status && chat.tab != Tab::Channels {
            c.fill_circle(46, cy - 13, 6, BG_PANEL);
            c.fill_circle(46, cy - 13, 4, ACCENT);
        }
    }
    icons::draw_centered(c, 32, 210, Icon::Communities, TEXT_SECONDARY);
    icons::draw_centered(c, 32, 276, Icon::Meta, META_PURPLE);
    icons::draw_centered(c, 32, c.h - 74, Icon::Media, TEXT_SECONDARY);
    c.fill_rect(20, 243, 24, 1, DIVIDER);
    if chat.tab == Tab::Settings {
        c.fill_circle(32, c.h - 30, 20, RAIL_ACTIVE);
    }
    c.fill_circle(32, c.h - 30, 14, ACCENT);
    centered(c, 32, c.h - 25, CHIP, ON_ACCENT, "Z");
}

/// The rail tab under a click at height `y` in a window `window_h` tall.
pub fn tab_at(y: i32, window_h: i32) -> Option<Tab> {
    if (y - (window_h - 30)).abs() < 20 {
        return Some(Tab::Settings);
    }
    RAIL_TABS.iter().find(|&&(_, cy)| (y - cy).abs() < 20).map(|&(tab, _)| tab)
}

// ---- calls

fn calls_sidebar(c: &mut Canvas, side: i32, chat: &Chat) {
    c.fill_rect(RAIL_W, 0, side - RAIL_W, c.h, BG_SIDEBAR);
    let scroll = chat.panel_scroll;
    let content = chat.calls_place(|item, y| {
        let y = y - scroll;
        if y < c.y0 + c.rows && y + CALLS_ROW_H > c.y0 {
            calls_item(c, side, y, item, chat);
        }
    });
    c.fill_rect(RAIL_W, 0, side - RAIL_W, CALLS_VIEW_TOP, BG_SIDEBAR);
    font::draw(c, RAIL_W + 21, 39, SCREEN_TITLE, TEXT_PRIMARY, "Ligações");
    icons::draw_centered(c, side - 87, HEADER_H / 2, Icon::Dialpad, TEXT_PRIMARY);
    icons::draw_centered(c, side - 39, HEADER_H / 2, Icon::AddCall, TEXT_PRIMARY);
    search_field(c, side, chat, "Pesquisar nome, número ou @nomedeusuário", true);
    let view = c.h - CALLS_VIEW_TOP;
    scrollbar(c, side - 7, CALLS_VIEW_TOP, view, scroll, chat.max_panel_scroll(), content - CALLS_VIEW_TOP);
    c.fill_rect(side - 1, 0, 1, c.h, DIVIDER);
}

fn calls_item(c: &mut Canvas, side: i32, y: i32, item: CallsItem, chat: &Chat) {
    let text_x = RAIL_W + 88;
    match item {
        CallsItem::FavoritesHeader => {
            font::draw(c, RAIL_W + 21, y + 37, SECTION, TEXT_PRIMARY, "Favoritos");
            let label = "Mostrar tudo";
            let w = font::measure(label, CHIP) + 34;
            let x = side - 29 - w;
            c.fill_round_rect(x, y + 16, w, 32, 16, CHIP_BORDER);
            c.fill_round_rect(x + 1, y + 17, w - 2, 30, 15, BG_SIDEBAR);
            font::draw(c, x + 17, y + 37, CHIP, ACCENT, label);
        }
        CallsItem::RecentsHeader => font::draw(c, RAIL_W + 21, y + 33, SECTION, TEXT_PRIMARY, "Recentes"),
        CallsItem::Favorite(index) => {
            let call = chat.favorite(index);
            avatar(c, RAIL_W + 48, y + 36, 24, AVATAR_COLORS[index % AVATAR_COLORS.len()], call.name());
            fitted(c, text_x, y + 43, NAME, TEXT_PRIMARY, call.name(), side - 112 - 16 - text_x);
            icons::draw(c, side - 112, y + 24, Icon::Videocam, TEXT_PRIMARY);
            icons::draw(c, side - 64, y + 24, Icon::Calls, TEXT_PRIMARY);
        }
        CallsItem::Recent(index) => {
            let call = chat.call(index);
            avatar(c, RAIL_W + 48, y + 36, 24, AVATAR_COLORS[(index + 3) % AVATAR_COLORS.len()], call.name());
            let time_w = font::measure(call.time(), META);
            font::draw(c, side - 34 - time_w, y + 31, META, TEXT_SECONDARY, call.time());
            fitted(c, text_x, y + 31, NAME, TEXT_PRIMARY, call.name(), side - 34 - time_w - 8 - text_x);
            let (icon, label, color) = match call.kind {
                CallKind::Received => (Icon::PhoneIncoming, "Ligação recebida", TEXT_SECONDARY),
                CallKind::Missed => (Icon::PhoneIncoming, "Ligação perdida", MISSED),
                CallKind::Outgoing => (Icon::PhoneOutgoing, "Ligação efetuada", TEXT_SECONDARY),
            };
            icons::draw(c, text_x, y + 40, icon, color);
            font::draw(c, text_x + 19, y + 53, PREVIEW, color, label);
            if call.count > 1 {
                let mut digits = [0u8; 3];
                let after = text_x + 19 + font::measure(label, PREVIEW);
                font::draw(c, after, y + 53, PREVIEW, color, " (");
                let open = font::measure(" (", PREVIEW);
                font::draw(c, after + open, y + 53, PREVIEW, color, number_text(call.count as usize, &mut digits));
                let digits_w = font::measure(number_text(call.count as usize, &mut digits), PREVIEW);
                font::draw(c, after + open + digits_w, y + 53, PREVIEW, color, ")");
            }
        }
    }
}

// ---- new conversation

/// Tints of the stand-in avatar of a contact without a photo: (background, person).
const CONTACT_TINTS: [(u32, u32); 4] = [(0x092c3d, 0x53bdeb), (0x0e3a2a, 0x21c063), (0x362c1f, 0xffd279), (0x092642, 0x53a6fd)];

fn new_chat_sidebar(c: &mut Canvas, side: i32, chat: &Chat) {
    c.fill_rect(RAIL_W, 0, side - RAIL_W, c.h, BG_SIDEBAR);
    let scroll = chat.panel_scroll;
    let action_icons = [Icon::GroupAddFilled, Icon::PersonAddFilled, Icon::CommunitiesFilled];
    let content = chat.contacts_place(|item, y| {
        let y = y - scroll;
        if y >= c.y0 + c.rows || y + NEW_CHAT_ROW_H <= c.y0 {
            return;
        }
        match item {
            ContactsItem::Action(index) => {
                c.fill_circle(RAIL_W + 48, y + 32, 24, ACCENT);
                icons::draw_centered(c, RAIL_W + 48, y + 32, action_icons[index], ON_ACCENT);
                font::draw(c, RAIL_W + 88, y + 37, NAME, TEXT_PRIMARY, NEW_CHAT_ACTIONS[index]);
            }
            ContactsItem::Myself => {
                contact_photo(c, RAIL_W + 48, y + 36, 0);
                let mut label = [0u8; 64];
                let name = chat.profile_name();
                let text = join(&mut label, &["@", name, " (você)"]);
                fitted(c, RAIL_W + 88, y + 30, NAME, TEXT_PRIMARY, text, side - 20 - (RAIL_W + 88));
                font::draw(c, RAIL_W + 88, y + 54, PREVIEW, TEXT_SECONDARY, "Mensagens para mim");
            }
            ContactsItem::Number => {
                contact_photo(c, RAIL_W + 48, y + 36, 1);
                let mut label = [0u8; 64];
                let text = join(&mut label, &["Conversar com ", chat.search.text()]);
                fitted(c, RAIL_W + 88, y + 41, NAME, TEXT_PRIMARY, text, side - 20 - (RAIL_W + 88));
            }
            ContactsItem::Letter(letter) => {
                let mut utf8 = [0u8; 4];
                font::draw(c, RAIL_W + 27, y + 45, PREVIEW, TEXT_SECONDARY, letter.encode_utf8(&mut utf8));
            }
            ContactsItem::Contact(index) => {
                let contact = chat.contact(index);
                contact_photo(c, RAIL_W + 48, y + 36, index + 1);
                fitted(c, RAIL_W + 88, y + 41, NAME, TEXT_PRIMARY, contact.name(), side - 20 - (RAIL_W + 88));
            }
        }
    });
    c.fill_rect(RAIL_W, 0, side - RAIL_W, CALLS_VIEW_TOP, BG_SIDEBAR);
    icons::draw_centered(c, RAIL_W + 31, HEADER_H / 2, Icon::Back, TEXT_PRIMARY);
    font::draw(c, RAIL_W + 59, 37, NAME, TEXT_PRIMARY, "Nova conversa");
    icons::draw_centered(c, side - 29, HEADER_H / 2, Icon::Dialpad, TEXT_PRIMARY);
    search_field(c, side, chat, "Pesquisar nome, número ou @nomedeusuário", true);
    scrollbar(c, side - 7, CALLS_VIEW_TOP, c.h - CALLS_VIEW_TOP, scroll, chat.max_panel_scroll(), content - CALLS_VIEW_TOP);
    c.fill_rect(side - 1, 0, 1, c.h, DIVIDER);
}

// ---- favourite messages

fn starred_sidebar(c: &mut Canvas, side: i32, chat: &Chat) {
    c.fill_rect(RAIL_W, 0, side - RAIL_W, c.h, BG_SIDEBAR);
    let count = chat.starred_count();
    let scroll = chat.panel_scroll;
    let room = side - 20 - (RAIL_W + 20);
    for nth in 0..count {
        let y = HEADER_H + nth as i32 * STARRED_ROW_H - scroll;
        if y >= c.y0 + c.rows || y + STARRED_ROW_H <= c.y0 {
            continue;
        }
        let Some(index) = chat.starred_nth(nth) else { break };
        let message = chat.index_message(index);
        let time_w = font::measure(message.time(), META);
        fitted(c, RAIL_W + 20, y + 32, NAME, TEXT_PRIMARY, chat.conversation_name(message.chat as usize), room - time_w - 8);
        font::draw(c, side - 20 - time_w, y + 32, META, TEXT_SECONDARY, message.time());
        icons::draw(c, RAIL_W + 20, y + 44, Icon::StarSmall, TEXT_SECONDARY);
        let author = if message.outgoing { "Você: " } else { "" };
        let mut line = [0u8; 160];
        let first = message.text().lines().next().unwrap_or("");
        fitted(c, RAIL_W + 38, y + 56, PREVIEW, TEXT_SECONDARY, join(&mut line, &[author, first]), room - 18);
        c.fill_rect(RAIL_W + 20, y + STARRED_ROW_H - 1, side - RAIL_W - 40, 1, DIVIDER);
    }
    if count == 0 {
        centered(c, (RAIL_W + side) / 2, 180, PREVIEW, TEXT_SECONDARY, "Nenhuma mensagem favorita");
    }
    c.fill_rect(RAIL_W, 0, side - RAIL_W, HEADER_H, BG_SIDEBAR);
    icons::draw_centered(c, RAIL_W + 31, HEADER_H / 2, Icon::Back, TEXT_PRIMARY);
    font::draw(c, RAIL_W + 59, 37, NAME, TEXT_PRIMARY, "Mensagens favoritas");
    scrollbar(c, side - 7, HEADER_H, c.h - HEADER_H, scroll, chat.max_panel_scroll(), HEADER_H + count as i32 * STARRED_ROW_H - HEADER_H);
    c.fill_rect(side - 1, 0, 1, c.h, DIVIDER);
}

/// The stand-in for a contact's photo: a tinted disc with a person on it.
fn contact_photo(c: &mut Canvas, cx: i32, cy: i32, tint: usize) {
    let (background, person) = CONTACT_TINTS[tint % CONTACT_TINTS.len()];
    c.fill_circle(cx, cy, 24, background);
    icons::draw_centered(c, cx, cy, Icon::Person, person);
}

/// `parts` glued together in `buffer` (cut when it does not fit, at a character boundary).
fn join<'a>(buffer: &'a mut [u8], parts: &[&str]) -> &'a str {
    let mut len = 0;
    for part in parts {
        let mut end = part.len().min(buffer.len() - len);
        while !part.is_char_boundary(end) {
            end -= 1;
        }
        buffer[len..len + end].copy_from_slice(&part.as_bytes()[..end]);
        len += end;
    }
    core::str::from_utf8(&buffer[..len]).unwrap_or("")
}

// ---- phone keypad

const KEYS: [(char, &str); 11] = [('1', ""), ('2', "ABC"), ('3', "DEF"), ('4', "GHI"), ('5', "JKL"), ('6', "MNO"), ('7', "PQRS"), ('8', "TUV"), ('9', "WXYZ"), ('+', ""), ('0', "")];

/// Centre of the keypad key in `row` (0 to 3) and `column` (0 to 2), where the digit's baseline is
/// `centre + 6` for the first three rows.
fn key_center(side: i32, row: i32, column: i32) -> (i32, i32) {
    let pitch = (side - RAIL_W - 20) / 3;
    (RAIL_W + 11 + pitch * (2 * column + 1) / 2, [222, 306, 390, 492][row as usize])
}

fn dialpad_sidebar(c: &mut Canvas, side: i32, chat: &Chat) {
    c.fill_rect(RAIL_W, 0, side - RAIL_W, c.h, BG_SIDEBAR);
    icons::draw_centered(c, RAIL_W + 31, HEADER_H / 2, Icon::Back, TEXT_PRIMARY);
    font::draw(c, RAIL_W + 59, 37, NAME, TEXT_PRIMARY, "Telefone");
    search_field(c, side, chat, "Telefone", false);
    let hint = if chat.search.text().is_empty() { "Insira um número de telefone para iniciar uma ligação" } else { "Nenhum resultado encontrado" };
    centered(c, (RAIL_W + 1 + side - 9) / 2, 153, PREVIEW, TEXT_SECONDARY, hint);
    for row in 0..4 {
        for column in 0..3 {
            let (cx, cy) = key_center(side, row, column);
            match (row, column) {
                (3, 2) => icons::draw_centered(c, cx, cy, Icon::Backspace, TEXT_PRIMARY),
                _ => {
                    let (key, letters) = KEYS[(row * 3 + column) as usize];
                    let mut utf8 = [0u8; 4];
                    centered(c, cx, cy + 14, SCREEN_TITLE, TEXT_PRIMARY, key.encode_utf8(&mut utf8));
                    if !letters.is_empty() {
                        centered(c, cx, cy + 41, PREVIEW, TEXT_SECONDARY, letters);
                    }
                }
            }
        }
    }
    c.fill_rect(side - 1, 0, 1, c.h, DIVIDER);
}

/// The keypad key under (`x`, `y`): a digit, `+`, or `\u{8}` for backspace.
fn key_at(side: i32, x: i32, y: i32) -> Option<char> {
    for row in 0..4 {
        for column in 0..3 {
            let (cx, cy) = key_center(side, row, column);
            if (x - cx).abs() < 45 && (y - cy).abs() < 36 {
                return Some(if (row, column) == (3, 2) { '\u{8}' } else { KEYS[(row * 3 + column) as usize].0 });
            }
        }
    }
    None
}

// ---- voice call

const INCOMING_H: i32 = 168;

/// The card that announces a call coming in, over a dimmed window.
fn incoming_call(c: &mut Canvas, chat: &Chat) {
    dim(c);
    let (w, h) = (500, INCOMING_H);
    let (x, y) = ((c.w - w) / 2, (c.h - h) / 2);
    c.fill_round_rect(x, y, w, h, 18, BG_PANEL);
    avatar(c, x + 24 + 32, y + 24 + 32, 32, AVATAR_COLORS[3], chat.call_name());
    font::draw(c, x + 24 + 80, y + 47, SCREEN_TITLE, TEXT_PRIMARY, chat.call_name());
    font::draw(c, x + 24 + 80, y + 70, PREVIEW, PANEL_SUBTLE, "Ligação de voz do WhatsApp");
    let (decline, answer) = incoming_buttons(c.w, c.h);
    c.fill_round_rect(decline.0, decline.1, decline.2, decline.3, 18, RAIL_ACTIVE);
    centered(c, decline.0 + decline.2 / 2, decline.1 + 23, CHIP, MISSED, "Recusar");
    c.fill_round_rect(answer.0, answer.1, answer.2, answer.3, 18, ACCENT);
    centered(c, answer.0 + answer.2 / 2, answer.1 + 23, CHIP, ON_ACCENT, "Atender");
}

/// (x, y, width, height) of the decline and answer buttons of the incoming call card.
fn incoming_buttons(window_w: i32, window_h: i32) -> ((i32, i32, i32, i32), (i32, i32, i32, i32)) {
    let (x, y) = ((window_w - 500) / 2, (window_h - INCOMING_H) / 2);
    let (decline_w, answer_w) = (font::measure("Recusar", CHIP) + 48, font::measure("Atender", CHIP) + 48);
    let right = x + 500 - 29;
    ((right - answer_w - 12 - decline_w, y + INCOMING_H - 21 - 36, decline_w, 36), (right - answer_w, y + INCOMING_H - 21 - 36, answer_w, 36))
}

fn dim(c: &mut Canvas) {
    let (first, last) = (c.y0.max(0), (c.y0 + c.rows).min(c.h));
    for y in first..last {
        for x in 0..c.w {
            c.blend(x, y, 0x000000, 82);
        }
    }
}

const CALL_CONTROL_R: i32 = 28;

/// Centres of the mute and hang-up buttons of the call screen.
fn call_controls(window_w: i32, window_h: i32) -> ((i32, i32), (i32, i32)) {
    let y = window_h - 88;
    ((window_w / 2 - 44, y), (window_w / 2 + 44, y))
}

/// The call in progress: who, for how long, and the two buttons. Covers the whole window.
fn call_screen(c: &mut Canvas, chat: &Chat) {
    c.fill_rect(0, 0, c.w, c.h, BG_APP);
    let cx = c.w / 2;
    let start = cx - (16 + 8 + font::measure("Suas ligações pessoais são protegidas com a criptografia de ponta a ponta", PREVIEW)) / 2;
    icons::draw(c, start, 32, Icon::Lock, PANEL_SUBTLE);
    font::draw(c, start + 24, 45, PREVIEW, PANEL_SUBTLE, "Suas ligações pessoais são protegidas com a criptografia de ponta a ponta");

    let middle = c.h / 2 - 40;
    avatar_big(c, cx, middle - 70, 64, chat.call_name());
    centered(c, cx, middle + 44, SCREEN_TITLE, TEXT_PRIMARY, chat.call_name());
    let mut buffer = [0u8; 8];
    let status = match chat.call_phase {
        CallPhase::Active => chat.call_clock(&mut buffer),
        CallPhase::Ended => chat.call_reason(),
        _ => "Chamando…",
    };
    centered(c, cx, middle + 76, 16, PANEL_SUBTLE, status);
    if chat.call_phase == CallPhase::Ended {
        return;
    }

    let (mute, end) = call_controls(c.w, c.h);
    c.fill_circle(mute.0, mute.1, CALL_CONTROL_R, if chat.muted { TEXT_PRIMARY } else { RAIL_ACTIVE });
    icon_mic(c, mute.0, mute.1, if chat.muted { BG_APP } else { TEXT_PRIMARY });
    if chat.muted {
        for i in -12..=12 {
            c.fill_rect(mute.0 + i, mute.1 + i, 2, 2, MISSED);
        }
    }
    c.fill_circle(end.0, end.1, CALL_CONTROL_R, MISSED);
    icons::draw_centered(c, end.0, end.1, Icon::CallEnd, TEXT_PRIMARY);
}

/// A large avatar disc with the initial of `name`, drawn with the biggest initial the atlas has.
fn avatar_big(c: &mut Canvas, cx: i32, cy: i32, radius: i32, name: &str) {
    c.fill_circle(cx, cy, radius, AVATAR_COLORS[3]);
    let mut utf8 = [0u8; 4];
    let initial = name.chars().next().unwrap_or('?').encode_utf8(&mut utf8);
    centered(c, cx, cy + 10, SCREEN_TITLE, TEXT_PRIMARY, initial);
}

// ---- message box

/// (x, y, width, height) of the message box, of its main button (OK, or Delete) and, when it asks,
/// of its Cancel button. The box is 500 px wide with 24 px of padding, its title 22 px, its body 14 px
/// (measured on WhatsApp Web's "microphone not found").
fn dialog_layout(window_w: i32, window_h: i32, chat: &Chat) -> ((i32, i32, i32, i32), (i32, i32, i32, i32), Option<(i32, i32, i32, i32)>) {
    let lines = font::lines(chat.dialog_body(), PREVIEW, 452).count().max(3) as i32;
    let height = 200 + 20 * lines;
    let (x, y) = ((window_w - 500) / 2, (window_h - height) / 2);
    let main_label = if chat.confirm_delete.is_some() || chat.confirm_remove.is_some() { "Apagar" } else if chat.confirm_clear.is_some() { "Limpar" } else { "OK" };
    let main_w = font::measure(main_label, CHIP) + 48;
    let top = y + height - 21 - 36;
    let main = (x + 500 - 29 - main_w, top, main_w, 36);
    let cancel = chat.confirming().then(|| {
        let cancel_w = font::measure("Cancelar", CHIP) + 48;
        (main.0 - 12 - cancel_w, top, cancel_w, 36)
    });
    ((x, y, 500, height), main, cancel)
}

fn dialog(c: &mut Canvas, chat: &Chat) {
    dim(c);
    let ((x, y, w, h), main, cancel) = dialog_layout(c.w, c.h, chat);
    c.fill_round_rect(x, y, w, h, 18, BG_PANEL);
    font::draw(c, x + 24, y + 43, SCREEN_TITLE, TEXT_PRIMARY, chat.dialog_title());
    for (index, line) in font::lines(chat.dialog_body(), PREVIEW, 452).enumerate() {
        font::draw(c, x + 24, y + 85 + index as i32 * 20, PREVIEW, PANEL_SUBTLE, line);
    }
    let label = if chat.confirm_delete.is_some() || chat.confirm_remove.is_some() { "Apagar" } else if chat.confirm_clear.is_some() { "Limpar" } else { "OK" };
    c.fill_round_rect(main.0, main.1, main.2, main.3, 18, if chat.confirming() { MISSED } else { ACCENT });
    centered(c, main.0 + main.2 / 2, main.1 + 23, CHIP, ON_ACCENT, label);
    if let Some(cancel) = cancel {
        c.fill_round_rect(cancel.0, cancel.1, cancel.2, cancel.3, 18, RAIL_ACTIVE);
        centered(c, cancel.0 + cancel.2 / 2, cancel.1 + 23, CHIP, TEXT_PRIMARY, "Cancelar");
    }
}

/// Which button of the message box a click is on: `Some(true)` the main one, `Some(false)` Cancel.
pub fn dialog_hit(x: i32, y: i32, window_w: i32, window_h: i32, chat: &Chat) -> Option<bool> {
    let (_, main, cancel) = dialog_layout(window_w, window_h, chat);
    let inside = |r: (i32, i32, i32, i32)| x >= r.0 && x < r.0 + r.2 && y >= r.1 && y < r.1 + r.3;
    if inside(main) {
        Some(true)
    } else if cancel.is_some_and(inside) {
        Some(false)
    } else {
        None
    }
}

/// The round "jump to the newest message" button, above the footer at the right.
fn scroll_button_center(window_w: i32, window_h: i32) -> (i32, i32) {
    (window_w - 44, window_h - INPUT_H - 34)
}

// ---- forwarding

/// (x, y, width, height) of the box that lists the conversations to forward to.
fn forward_rect(window_w: i32, window_h: i32) -> (i32, i32, i32, i32) {
    let (w, h) = (420, 104 + FORWARD_LIST_H);
    ((window_w - w) / 2, ((window_h - h) / 2).max(8), w, h)
}

fn forward_dialog(c: &mut Canvas, chat: &Chat) {
    dim(c);
    let (x, y, w, h) = forward_rect(c.w, c.h);
    c.fill_round_rect(x, y, w, h, 18, BG_PANEL);
    let (targets, count) = chat.forward_targets();
    let list_top = y + 64;
    for (row, &index) in targets[..count].iter().enumerate() {
        let top = list_top + row as i32 * FORWARD_ROW_H - chat.forward_scroll;
        if top + FORWARD_ROW_H <= list_top || top >= list_top + FORWARD_LIST_H {
            continue;
        }
        let info = chat.conversation(index as usize);
        avatar(c, x + 48, top + FORWARD_ROW_H / 2, 20, AVATAR_COLORS[index as usize % AVATAR_COLORS.len()], info.name());
        fitted(c, x + 84, top + FORWARD_ROW_H / 2 + 5, NAME, TEXT_PRIMARY, info.name(), w - 84 - 24);
    }
    // The list scrolls under the title and above the bottom edge of the box.
    c.fill_rect(x + 8, y, w - 16, 64, BG_PANEL);
    c.fill_round_rect(x, y, w, 40, 18, BG_PANEL);
    font::draw(c, x + 24, y + 43, SCREEN_TITLE, TEXT_PRIMARY, "Encaminhar para");
    c.fill_rect(x + 8, list_top + FORWARD_LIST_H, w - 16, h - 64 - FORWARD_LIST_H, BG_PANEL);
    c.fill_round_rect(x, y + h - 40, w, 40, 18, BG_PANEL);
}

/// What a click hit while the forwarding list is open.
fn forward_hit(x: i32, y: i32, window_w: i32, window_h: i32, chat: &Chat) -> Hit {
    let (bx, by, bw, bh) = forward_rect(window_w, window_h);
    if x < bx || x >= bx + bw || y < by || y >= by + bh {
        return Hit::ForwardOutside;
    }
    let (targets, count) = chat.forward_targets();
    let list_top = by + 64;
    if y >= list_top && y < list_top + FORWARD_LIST_H {
        let row = (y - list_top + chat.forward_scroll) / FORWARD_ROW_H;
        if (row as usize) < count {
            return Hit::ForwardTo(targets[row as usize] as usize);
        }
    }
    Hit::Nothing
}

/// Where the round reaction button of message `index` sits: beside its bubble, on the side away from the edge.
fn hover_button_center(chat: &Chat, index: usize, window_w: i32) -> Option<(i32, i32)> {
    let side = sidebar_width(window_w);
    let top = HEADER_H - chat.conversation_scroll;
    let mut found = None;
    chat.place(|message, at, _| {
        if chat.index_of(message) != index {
            return;
        }
        let quote_width = chat.quote_width_of(message);
        let shape = bubble_shape(message.text(), message.time(), message.outgoing, message.starred, chat.text_limit(), quote_width, message.block());
        let width = shape.width + 2 * BUBBLE_PAD_X;
        let left = if message.outgoing { window_w - BUBBLE_MARGIN - 4 - width } else { side + BUBBLE_MARGIN };
        let cx = if message.outgoing { left - 24 } else { left + width + 24 };
        found = Some((cx, top + at + 22));
    });
    found
}

/// The message the pointer is on, or whose reaction button it is on.
pub fn hover_at(chat: &Chat, x: i32, y: i32, window_w: i32) -> Option<usize> {
    if let Some(current) = chat.hover {
        if let Some((cx, cy)) = hover_button_center(chat, current, window_w) {
            if (x - cx).abs() <= 18 && (y - cy).abs() <= 18 {
                return Some(current);
            }
        }
    }
    message_at(chat, x, y, window_w)
}

/// A click on the reaction button of the message the pointer is over.
fn hover_button_hit(chat: &Chat, x: i32, y: i32, window_w: i32) -> Option<usize> {
    let index = chat.hover?;
    let (cx, cy) = hover_button_center(chat, index, window_w)?;
    ((x - cx).pow(2) + (y - cy).pow(2) <= 16 * 16).then_some(index)
}

/// Where the pop-up of the reaction button opens (over the button).
pub fn hover_button_menu_at(chat: &Chat, index: usize, window_w: i32) -> (i32, i32) {
    hover_button_center(chat, index, window_w).map_or((0, 0), |(cx, cy)| (cx - 30, cy + 18))
}

// ---- replying

/// The block that shows the message a bubble answers: a bar on the left, who wrote it, and its text.
fn quote_block(c: &mut Canvas, x: i32, y: i32, w: i32, author: &str, text: &str, quoted_is_mine: bool, inside_mine: bool) {
    let (background, bar) = (if inside_mine { 0x0f3a2b } else { 0x1b1d1d }, if quoted_is_mine { ACCENT } else { 0xa48ff5 });
    c.fill_round_rect(x, y, w, QUOTE_H - 6, 6, background);
    c.fill_round_rect(x, y, 5, QUOTE_H - 6, 2, bar);
    fitted(c, x + 13, y + 16, TINY_MEDIUM, bar, author, w - 22);
    fitted(c, x + 13, y + 32, BODY, TEXT_SECONDARY, text, w - 22);
}

/// The bar over the footer while a reply is being written: the message answered, and a close cross.
fn reply_bar(c: &mut Canvas, x0: i32, chat: &Chat) {
    let Some(quoted) = chat.replying_to() else { return };
    let y = c.h - INPUT_H - REPLY_BAR_H;
    c.fill_rect(x0, y, c.w - x0, REPLY_BAR_H, BG_SIDEBAR);
    let (card_x, card_w) = (x0 + 12, c.w - x0 - 24 - 44);
    let bar = if quoted.outgoing { ACCENT } else { 0xa48ff5 };
    c.fill_round_rect(card_x, y + 8, card_w, REPLY_BAR_H - 12, 8, INPUT_PILL);
    c.fill_round_rect(card_x, y + 8, 5, REPLY_BAR_H - 12, 2, bar);
    fitted(c, card_x + 16, y + 26, TINY_MEDIUM, bar, if chat.editing { "Editar mensagem" } else { chat.author_of(quoted) }, card_w - 28);
    fitted(c, card_x + 16, y + 44, BODY, TEXT_SECONDARY, quoted.text(), card_w - 28);
    cross(c, c.w - 34, y + REPLY_BAR_H / 2, TEXT_SECONDARY);
}

fn reply_close_center(window_w: i32, window_h: i32) -> (i32, i32) {
    (window_w - 34, window_h - INPUT_H - REPLY_BAR_H / 2)
}

const MESSAGE_MENU_ITEMS: [&str; 6] = ["Responder", "Copiar", "Encaminhar", "Favoritar", "Editar", "Apagar para todos"];

/// How many entries the pop-up of message `index` has: deleting is offered on our own messages only.
fn message_menu_items(chat: &Chat, index: usize) -> usize {
    let message = chat.index_message(index);
    match (message.deleted, message.outgoing) {
        (true, _) => 2,
        (false, false) => 4,
        (false, true) => 6,
    }
}
/// Height of the row of emoji at the top of the pop-up over a message.
const REACTION_ROW_H: i32 = 44;

/// The pop-up over a message: (x, y, width, height), kept inside the window.
fn message_menu_rect(x: i32, y: i32, window_w: i32, window_h: i32, items: usize) -> (i32, i32, i32, i32) {
    let (w, h) = (212, 10 + REACTION_ROW_H + items as i32 * 36);
    (x.min(window_w - w - 8).max(0), y.min(window_h - h - 8).max(0), w, h)
}

fn message_menu(c: &mut Canvas, chat: &Chat) {
    let Some((index, at_x, at_y)) = chat.message_menu else { return };
    let items = message_menu_items(chat, index);
    let (x, y, w, h) = message_menu_rect(at_x, at_y, c.w, c.h, items);
    c.fill_round_rect(x, y, w, h, 16, CHIP_BORDER);
    c.fill_round_rect(x + 1, y + 1, w - 2, h - 2, 15, BG_SIDEBAR);
    for (index, emoji) in QUICK_REACTIONS.into_iter().enumerate() {
        if let Some(number) = font::emoji_index(emoji) {
            font::draw_emoji(c, x + 12 + index as i32 * 28, y + 5 + 13, number);
        }
    }
    // The seventh slot opens the whole emoji picker.
    let plus_x = x + 12 + QUICK_REACTIONS.len() as i32 * 28 + 9;
    c.fill_circle(plus_x, y + 5 + 22, 11, RAIL_ACTIVE);
    icon_plus(c, plus_x, y + 5 + 22, TEXT_PRIMARY);
    c.fill_rect(x + 12, y + 5 + REACTION_ROW_H - 1, w - 24, 1, CHIP_BORDER);
    for (slot, label) in MESSAGE_MENU_ITEMS.into_iter().take(items).enumerate() {
        let label = if slot == 3 && chat.index_message(index).starred { "Desfavoritar" } else { label };
        font::draw(c, x + 24, y + 5 + REACTION_ROW_H + slot as i32 * 36 + 23, PREVIEW, if slot == 5 { MISSED } else { TEXT_PRIMARY }, label);
    }
}

/// Which message the point (`x`, `y`) is on, by its number among all the messages.
pub fn message_at(chat: &Chat, x: i32, y: i32, window_w: i32) -> Option<usize> {
    let side = sidebar_width(window_w);
    let top = HEADER_H - chat.conversation_scroll;
    let mut found = None;
    chat.place(|message, at, _| {
        let quote_width = chat.quote_width_of(message);
        let shape = bubble_shape(message.text(), message.time(), message.outgoing, message.starred, chat.text_limit(), quote_width, message.block());
        let width = shape.width + 2 * BUBBLE_PAD_X;
        let left = if message.outgoing { window_w - BUBBLE_MARGIN - 4 - width } else { side + BUBBLE_MARGIN };
        let (from, to) = (top + at, top + at + shape.height);
        if y >= from && y < to && x >= left && x < left + width {
            found = Some(chat.index_of(message));
        }
    });
    found
}

/// The small pill under a bubble with the emoji that were put on it, each with its count when several people used it.
fn reactions_pill(c: &mut Canvas, bubble_x: i32, bubble_bottom: i32, bubble_w: i32, message: &crate::chat::Message) {
    let shown = message.tally.iter().take_while(|&&code| code != 0).count();
    if shown == 0 {
        return;
    }
    let mut w = 10;
    for slot in 0..shown {
        w += 20 + if message.counts[slot] > 1 { 10 } else { 0 };
    }
    let h = 24;
    let x = if message.outgoing { bubble_x + bubble_w - w - 8 } else { bubble_x + 8 };
    let y = bubble_bottom - 10;
    c.fill_round_rect(x, y, w, h, 12, DIVIDER);
    c.fill_round_rect(x + 1, y + 1, w - 2, h - 2, 11, BG_SIDEBAR);
    let mut at = x + 7;
    for slot in 0..shown {
        if let Some(index) = char::from_u32(message.tally[slot]).and_then(font::emoji_index) {
            font::draw_emoji(c, at, y + 3, index);
        }
        at += 20;
        if message.counts[slot] > 1 {
            let mut digits = [0u8; 3];
            font::draw(c, at - 2, y + 17, META, TEXT_SECONDARY, number_text(message.counts[slot] as usize, &mut digits));
            at += 10;
        }
    }
}

/// Click on a quote block: the message it quotes.
pub fn quote_at(chat: &Chat, x: i32, y: i32, window_w: i32) -> Option<usize> {
    let side = sidebar_width(window_w);
    let top = HEADER_H - chat.conversation_scroll;
    let mut found = None;
    chat.place(|message, at, _| {
        let Some(quoted) = chat.quoted_by(message) else { return };
        let quote_width = chat.quote_width_of(message);
        let shape = bubble_shape(message.text(), message.time(), message.outgoing, message.starred, chat.text_limit(), quote_width, message.block());
        let width = shape.width + 2 * BUBBLE_PAD_X;
        let left = if message.outgoing { window_w - BUBBLE_MARGIN - 4 - width } else { side + BUBBLE_MARGIN };
        let (bx, by) = (left + 6, top + at + 6);
        if x >= bx && x < bx + width - 12 && y >= by && y < by + QUOTE_H - 6 {
            found = Some(chat.index_of(quoted));
        }
    });
    found
}

// ---- find in the conversation

const FIND_BAR_H: i32 = 56;

/// Chevron pointing up or down, drawn with small squares.
fn chevron(c: &mut Canvas, cx: i32, cy: i32, up: bool, color: u32) {
    for step in 0..=6 {
        let y = if up { cy + 3 - step } else { cy - 3 + step };
        c.fill_rect(cx - 6 + step, y, 2, 2, color);
        c.fill_rect(cx + 5 - step, y, 2, 2, color);
    }
}

fn cross(c: &mut Canvas, cx: i32, cy: i32, color: u32) {
    for step in -5..=5 {
        c.fill_rect(cx + step, cy + step, 2, 2, color);
        c.fill_rect(cx + step, cy - step, 2, 2, color);
    }
}

/// The find bar's pieces: the text box (x, y, width, height) and the centres of its three buttons.
fn find_layout(x0: i32, window_w: i32) -> ((i32, i32, i32, i32), [(i32, i32); 3]) {
    let top = HEADER_H + 8;
    let text_box = (x0 + 16, top, window_w - x0 - 16 - 230, 40);
    let center_y = HEADER_H + FIND_BAR_H / 2;
    (text_box, [(window_w - 108, center_y), (window_w - 68, center_y), (window_w - 28, center_y)])
}

fn find_bar(c: &mut Canvas, x0: i32, chat: &Chat) {
    c.fill_rect(x0, HEADER_H, c.w - x0, FIND_BAR_H, BG_SIDEBAR);
    c.fill_rect(x0, HEADER_H + FIND_BAR_H - 1, c.w - x0, 1, DIVIDER);
    let ((x, y, w, h), [up, down, close]) = find_layout(x0, c.w);
    if chat.focus == Focus::Find {
        c.fill_round_rect(x - 2, y - 2, w + 4, h + 4, h / 2 + 2, ACCENT);
    }
    c.fill_round_rect(x, y, w, h, h / 2, BG_SELECTED);
    icons::draw_centered(c, x + 26, y + h / 2, Icon::SearchSmall, TEXT_SECONDARY);
    let baseline = y + h / 2 + 5;
    let query = chat.find_text();
    let show_caret = chat.focus == Focus::Find && chat.caret_on && chat.window_focused;
    if query.is_empty() {
        fitted(c, x + 48, baseline, PREVIEW, TEXT_SECONDARY, "Pesquisar", w - 72);
        if show_caret {
            draw_caret(c, x + 48, (baseline - 13, 17));
        }
    } else {
        edit_text(c, x + 48, baseline, PREVIEW, query, chat.find_caret(), w - 72, (baseline - 13, 17), show_caret);
    }
    // How many matches, and which one is showing.
    if !query.is_empty() {
        let mut digits = [0u8; 3];
        let mut other = [0u8; 3];
        let count = chat.find_count();
        let mut label = [0u8; 24];
        let text = match chat.find_position() {
            Some(position) => join(&mut label, &[number_text(position + 1, &mut digits), " de ", number_text(count, &mut other)]),
            None => "Nenhum resultado",
        };
        font::draw(c, c.w - 132 - font::measure(text, PREVIEW), baseline, PREVIEW, TEXT_SECONDARY, text);
    }
    let active = chat.find_count() > 0;
    chevron(c, up.0, up.1, true, if active { TEXT_PRIMARY } else { TEXT_SECONDARY });
    chevron(c, down.0, down.1, false, if active { TEXT_PRIMARY } else { TEXT_SECONDARY });
    cross(c, close.0, close.1, TEXT_PRIMARY);
}

/// What a click on the find bar hit.
pub enum FindHit {
    Box(i32),
    Up,
    Down,
    Close,
}

fn find_hit(x: i32, y: i32, x0: i32, window_w: i32) -> Option<FindHit> {
    if y < HEADER_H || y >= HEADER_H + FIND_BAR_H {
        return None;
    }
    let ((bx, by, bw, bh), [up, down, close]) = find_layout(x0, window_w);
    let near = |p: (i32, i32)| (x - p.0).abs() < 20 && (y - p.1).abs() < 20;
    if near(up) {
        Some(FindHit::Up)
    } else if near(down) {
        Some(FindHit::Down)
    } else if near(close) {
        Some(FindHit::Close)
    } else if x >= bx && x < bx + bw && y >= by && y < by + bh {
        Some(FindHit::Box(x))
    } else {
        None
    }
}

/// The byte offset in the find bar's text box that a click at `x` lands on.
pub fn find_caret_at(chat: &Chat, x: i32, window_w: i32) -> usize {
    let side = sidebar_width(window_w);
    let ((bx, _, bw, _), _) = find_layout(side, window_w);
    caret_at(chat.find_text(), chat.find_caret().0, PREVIEW, bw - 72, x - (bx + 48))
}

// ---- status, channels, settings

/// A ring around a status avatar, centred on (`cx`, `cy`): `segments` arcs with small gaps, the last
/// `unseen` of them green and the rest grey. Whole numbers only: the angle is a fixed-point
/// approximation of atan2, in 1/3142 of a turn (the circumference of WhatsApp's 104-unit SVG ring).
fn story_ring(c: &mut Canvas, cx: i32, cy: i32, segments: u16, unseen: u16) {
    const TURN: i32 = 3142;
    const RADIUS: i32 = 5908; // 23.08 px in 1/256 px
    const HALF_STROKE: i32 = 236;
    let segments = segments.max(1) as i32;
    let length = if segments == 1 { TURN } else { (TURN - 100 * segments) / segments };
    for y in (cy - 25).max(c.y0)..(cy + 25).min(c.y0 + c.rows) {
        for x in cx - 25..cx + 25 {
            let (dx, dy) = ((2 * x + 1 - 2 * cx) * 8, (2 * y + 1 - 2 * cy) * 8);
            let distance = isqrt((dx * dx + dy * dy) as u32 * 256) as i32;
            let coverage = (HALF_STROKE - (distance - RADIUS).abs() + 128).clamp(0, 256);
            if coverage == 0 {
                continue;
            }
            let position = turn_fraction(dy, dx, TURN);
            let mut color = None;
            for k in 0..segments {
                let start = if segments == 1 { 0 } else { 50 + k * (length + 100) - 20 };
                let end = if segments == 1 { TURN } else { start + length + 40 };
                if position >= start && position < end {
                    color = Some(if k >= segments - unseen as i32 { ACCENT } else { STORY_SEEN });
                    break;
                }
            }
            if let Some(color) = color {
                c.blend(x, y, color, (coverage * 255 / 256) as u8);
            }
        }
    }
}

fn isqrt(value: u32) -> u32 {
    let (mut low, mut high) = (0u32, 46341u32);
    while low < high {
        let mid = (low + high + 1) / 2;
        if mid * mid <= value {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    low
}

/// Angle of the vector (`dx`, `dy`), measured clockwise on the screen from the positive x axis, in
/// units of 1/`turn` of a full turn. Accurate to about 0.2%, which is under a tenth of a pixel on a ring.
fn turn_fraction(dy: i32, dx: i32, turn: i32) -> i32 {
    let (ax, ay) = (dx.abs(), dy.abs());
    if ax == 0 && ay == 0 {
        return 0;
    }
    let (small, large) = (ax.min(ay), ax.max(ay));
    // atan(t) ~ t * (pi/4 + 0.273 * (1 - t)) for t in [0, 1], t in 1/4096 units, result in 1/4096 rad.
    let t = small * 4096 / large;
    let eighth = t * (3217 + 1118 * (4096 - t) / 4096) / 4096; // 0.7854 and 0.273 in 1/4096
    let quarter = 6434; // pi/2 in 1/4096
    let angle = if ay > ax { quarter - eighth } else { eighth };
    let angle = match (dx >= 0, dy >= 0) {
        (true, true) => angle,
        (false, true) => 2 * quarter - angle,
        (false, false) => 2 * quarter + angle,
        (true, false) => 4 * quarter - angle,
    };
    angle * turn / (4 * quarter)
}

fn empty_pane(c: &mut Canvas, side: i32, icon: Icon, title: &str, hint: &str) {
    let cx = (side + c.w) / 2 + 2;
    let lines = font::lines(hint, 16, 560).count() as i32;
    let height = 91 + 52 + 24 * lines;
    let top = (c.h - height) / 2 - 22;
    icons::draw(c, cx - 32, top, icon, PANEL_ICON);
    centered(c, cx, top + 122, EMPTY_TITLE, TEXT_PRIMARY, title);
    for (index, line) in font::lines(hint, 16, 560).enumerate() {
        centered(c, cx, top + 160 + index as i32 * 24, 16, PANEL_SUBTLE, line);
    }
}

/// The header shared by the list screens: the title and the two round buttons.
fn panel_header(c: &mut Canvas, side: i32, title: &str) {
    c.fill_rect(RAIL_W, 0, side - RAIL_W, HEADER_H, BG_SIDEBAR);
    font::draw(c, RAIL_W + 21, 39, SCREEN_TITLE, TEXT_PRIMARY, title);
}

fn status_sidebar(c: &mut Canvas, side: i32, chat: &Chat) {
    c.fill_rect(RAIL_W, 0, side - RAIL_W, c.h, BG_SIDEBAR);
    let scroll = chat.panel_scroll;
    let content = chat.status_place(|item, y| {
        let y = y - scroll;
        if y >= c.y0 + c.rows || y + STATUS_ROW_H <= c.y0 {
            return;
        }
        match item {
            StatusItem::Mine => {
                let (cx, cy) = (RAIL_W + 44, y + 36);
                story_ring(c, cx, cy, 1, 0);
                c.fill_circle(cx, cy, 20, AVATAR_COLORS[0]);
                centered(c, cx, cy + 6, INITIAL_SMALL, TEXT_PRIMARY, "Z");
                c.fill_circle(cx + 16, cy + 13, 11, BG_SIDEBAR);
                c.fill_circle(cx + 16, cy + 13, 8, ACCENT);
                icons::draw_centered(c, cx + 16, cy + 13, Icon::Add, ON_ACCENT);
                font::draw(c, RAIL_W + 79, y + 32, NAME, TEXT_PRIMARY, "Meu status");
                font::draw(c, RAIL_W + 79, y + 50, PREVIEW, TEXT_SECONDARY, chat.my_status().time());
            }
            StatusItem::RecentLabel => font::draw(c, RAIL_W + 27, y + 45, PREVIEW, TEXT_SECONDARY, "RECENTE"),
            StatusItem::SeenLabel => font::draw(c, RAIL_W + 27, y + 45, PREVIEW, TEXT_SECONDARY, "VISTO"),
            StatusItem::Story(index) => {
                let story = chat.story(index);
                let (cx, cy) = (RAIL_W + 44, y + 34);
                story_ring(c, cx, cy, story.first, story.second);
                c.fill_circle(cx, cy, 20, AVATAR_COLORS[(index + 1) % AVATAR_COLORS.len()]);
                let mut utf8 = [0u8; 4];
                let initial = story.name().chars().next().unwrap_or('?').encode_utf8(&mut utf8);
                centered(c, cx, cy + 6, INITIAL_SMALL, TEXT_PRIMARY, initial);
                fitted(c, RAIL_W + 79, y + 31, NAME, TEXT_PRIMARY, story.name(), side - RAIL_W - 79 - 40);
                fitted(c, RAIL_W + 79, y + 53, PREVIEW, TEXT_SECONDARY, story.time(), side - RAIL_W - 79 - 40);
            }
        }
    });
    panel_header(c, side, "Status");
    if chat.menu_open {
        c.fill_circle(side - 87, HEADER_H / 2, 20, BG_SELECTED);
    }
    icons::draw_centered(c, side - 87, HEADER_H / 2, Icon::Menu, TEXT_PRIMARY);
    icons::draw_centered(c, side - 39, HEADER_H / 2, Icon::AddCircle, TEXT_PRIMARY);
    scrollbar(c, side - 7, HEADER_H, c.h - HEADER_H, scroll, chat.max_panel_scroll(), content - HEADER_H);
    c.fill_rect(side - 1, 0, 1, c.h, DIVIDER);
}

fn channels_sidebar(c: &mut Canvas, side: i32, chat: &Chat) {
    c.fill_rect(RAIL_W, 0, side - RAIL_W, c.h, BG_SIDEBAR);
    let scroll = chat.panel_scroll;
    let right = side - 34;
    let content = chat.channels_place(|item, y| {
        let y = y - scroll;
        if y >= c.y0 + c.rows || y + CHANNEL_ROW_H <= c.y0 {
            return;
        }
        match item {
            ChannelsItem::Followed(index) => {
                let channel = chat.channel(index);
                avatar(c, RAIL_W + 48, y + 36, 24, AVATAR_COLORS[(index + 2) % AVATAR_COLORS.len()], channel.name());
                let unread = channel.first > 0;
                let time_w = font::measure(channel.time(), TINY_MEDIUM);
                font::draw(c, right - time_w, y + 31, TINY_MEDIUM, if unread { ACCENT } else { TEXT_SECONDARY }, channel.time());
                fitted(c, RAIL_W + 87, y + 30, NAME, TEXT_PRIMARY, channel.name(), right - time_w - 8 - (RAIL_W + 87));
                let mut room = right - (RAIL_W + 87);
                if unread {
                    let mut digits = [0u8; 3];
                    let text = number_text(channel.first as usize, &mut digits);
                    let w = (font::measure(text, TINY_MEDIUM) + 8).max(24);
                    c.fill_round_rect(right - w, y + 39, w, 20, 10, ACCENT);
                    centered(c, right - w / 2, y + 53, TINY_MEDIUM, ON_ACCENT, text);
                    room -= w + 8;
                }
                fitted(c, RAIL_W + 87, y + 54, PREVIEW, TEXT_SECONDARY, channel.text(), room);
            }
            ChannelsItem::SuggestionsHeader => font::draw(c, RAIL_W + 21, y + 31, CHIP, TEXT_SECONDARY, "Encontrar canais para seguir"),
            ChannelsItem::Suggestion(index) => {
                let suggestion = chat.suggestion(index);
                avatar(c, RAIL_W + 48, y + 36, 24, AVATAR_COLORS[(index + 5) % AVATAR_COLORS.len()], suggestion.name());
                let button_w = font::measure("Seguir", CHIP) + 32;
                let button_x = side - 31 - button_w;
                let name_room = button_x - 12 - (RAIL_W + 88) - if suggestion.flag { 20 } else { 0 };
                fitted(c, RAIL_W + 88, y + 31, NAME, TEXT_PRIMARY, suggestion.name(), name_room);
                if suggestion.flag {
                    let shown = font::measure(suggestion.name(), NAME).min(name_room);
                    icons::draw(c, RAIL_W + 88 + shown + 2, y + 18, Icon::Verified, VERIFIED);
                }
                fitted(c, RAIL_W + 88, y + 53, PREVIEW, TEXT_SECONDARY, suggestion.text(), name_room);
                c.fill_round_rect(button_x, y + 20, button_w, 32, 16, FOLLOW_BG);
                font::draw(c, button_x + 16, y + 41, CHIP, CHIP_ACTIVE_TEXT, "Seguir");
            }
            ChannelsItem::DiscoverBar => {
                let (x, top, w) = (RAIL_W + 11, y + 20, side - RAIL_W - 30);
                c.fill_round_rect(x, top, w, 40, 20, CHIP_BORDER);
                c.fill_round_rect(x + 1, top + 1, w - 2, 38, 19, BG_SIDEBAR);
                let label = "Descobrir mais";
                let start = x + w / 2 - (20 + 8 + font::measure(label, CHIP)) / 2;
                icons::draw(c, start, top + 10, Icon::Grid, ACCENT);
                font::draw(c, start + 28, top + 25, CHIP, ACCENT, label);
            }
        }
    });
    c.fill_rect(RAIL_W, 0, side - RAIL_W, CALLS_VIEW_TOP, BG_SIDEBAR);
    panel_header(c, side, "Canais");
    icons::draw_centered(c, side - 39, HEADER_H / 2, Icon::AddCircle, TEXT_PRIMARY);
    search_field(c, side, chat, "Pesquisar", true);
    scrollbar(c, side - 7, CALLS_VIEW_TOP, c.h - CALLS_VIEW_TOP, scroll, chat.max_panel_scroll(), content - CALLS_VIEW_TOP);
    c.fill_rect(side - 1, 0, 1, c.h, DIVIDER);
}

/// The photo that stands in for the profile picture: a grey disc with a person in it.
fn default_photo(c: &mut Canvas, cx: i32, cy: i32) {
    let radius = 60;
    c.fill_circle(cx, cy, radius, PANEL_ICON);
    c.fill_circle(cx, cy - 14, 22, PANEL_SUBTLE);
    // Shoulders: a wide circle, kept inside the disc.
    for y in (cy + 14).max(c.y0)..(cy + radius - 1).min(c.y0 + c.rows) {
        let (dy_disc, dy_body) = (y - cy, y - (cy + 66));
        let half_disc = isqrt(((radius - 1) * (radius - 1) - dy_disc * dy_disc).max(0) as u32) as i32;
        let half_body = isqrt((46 * 46 - dy_body * dy_body).max(0) as u32) as i32;
        let half = half_disc.min(half_body);
        c.fill_rect(cx - half, y, 2 * half, 1, PANEL_SUBTLE);
    }
}

fn settings_sidebar(c: &mut Canvas, side: i32, chat: &Chat) {
    c.fill_rect(RAIL_W, 0, side - RAIL_W, c.h, BG_SIDEBAR);
    let scroll = chat.panel_scroll;
    default_photo(c, (RAIL_W + side) / 2 - 4, SETTINGS_PHOTO_TOP + 60 - scroll);
    let icons_by_row = [Icon::AccountCircle, Icon::Key, Icon::LockBig, Icon::ChatsOutline, Icon::Notifications, Icon::Keyboard, Icon::Help];
    for (index, ((title, hint), icon)) in SETTINGS_ENTRIES.into_iter().zip(icons_by_row).enumerate() {
        let y = SETTINGS_ROWS_TOP + index as i32 * SETTINGS_ROW_H - scroll;
        if y >= c.y0 + c.rows || y + SETTINGS_ROW_H <= c.y0 {
            continue;
        }
        icons::draw(c, RAIL_W + 37, y + 22, icon, TEXT_SECONDARY);
        font::draw(c, RAIL_W + 83, y + 29, NAME, TEXT_PRIMARY, title);
        font::draw(c, RAIL_W + 83, y + 51, PREVIEW, TEXT_SECONDARY, hint);
    }
    let y = SETTINGS_ROWS_TOP + SETTINGS_ENTRIES.len() as i32 * SETTINGS_ROW_H - scroll;
    icons::draw(c, RAIL_W + 37, y + 18, Icon::Logout, MISSED);
    font::draw(c, RAIL_W + 83, y + 35, NAME, MISSED, "Desconectar");
    c.fill_rect(RAIL_W + 29, y + SETTINGS_LOGOUT_H + 12, side - RAIL_W - 67, 1, DIVIDER);

    c.fill_rect(RAIL_W, 0, side - RAIL_W, CALLS_VIEW_TOP, BG_SIDEBAR);
    let name = if chat.profile_name().is_empty() { "Configurações" } else { chat.profile_name() };
    panel_header(c, side, name);
    search_field(c, side, chat, "Pesquisar", true);
    scrollbar(c, side - 7, CALLS_VIEW_TOP, c.h - CALLS_VIEW_TOP, scroll, chat.max_panel_scroll(), chat.settings_height() - CALLS_VIEW_TOP);
    c.fill_rect(side - 1, 0, 1, c.h, DIVIDER);
}

const PANE_BUTTONS: [(&str, Icon); 3] = [("Iniciar ligação", Icon::VideoCall), ("Novo link de ligação", Icon::Link), ("Ligar para um telefone", Icon::Dialpad)];

/// Horizontal centres of the three round buttons of the calls pane.
fn pane_button_centers(side: i32, window_w: i32) -> [i32; 3] {
    let cx = (side + window_w) / 2 + 2;
    let gap = 16 * 64;
    let widths = PANE_BUTTONS.map(|(label, _)| font::measure_exact(label, LABEL) + 16 * 64);
    let mut x64 = cx * 64 - (widths.iter().sum::<i32>() + 2 * gap) / 2;
    let mut centers = [0; 3];
    for (center, width) in centers.iter_mut().zip(widths) {
        *center = (x64 + width / 2 + 32) / 64;
        x64 += width + gap;
    }
    centers
}

/// The right side of the calls screen: what to do with calls, and the encryption note at the bottom.
fn calls_pane(c: &mut Canvas, side: i32) {
    let cx = (side + c.w) / 2 + 2;
    let top = (c.h - 316) / 2 - 12;
    icons::draw(c, cx - 32, top, Icon::VideocamBig, PANEL_ICON);
    centered(c, cx, top + 120, EMPTY_TITLE, TEXT_PRIMARY, "Ligações de voz e vídeo");
    let hint = "Compartilhe sua tela, reaja com emojis e muito mais com até 32 pessoas.";
    for (index, line) in font::lines(hint, 16, 480).enumerate() {
        centered(c, cx, top + 158 + index as i32 * 24, 16, PANEL_SUBTLE, line);
    }

    let centers = pane_button_centers(side, c.w);
    for ((label, icon), center) in PANE_BUTTONS.into_iter().zip(centers) {
        c.fill_circle(center, top + 268, 24, RAIL_ACTIVE);
        icons::draw_centered(c, center, top + 268, icon, TEXT_PRIMARY);
        centered(c, center, top + 313, LABEL, TEXT_PRIMARY, label);
    }

    let note = "Suas ligações pessoais são protegidas com a criptografia de ponta a ponta";
    let start = cx - (16 + 8 + font::measure(note, PREVIEW)) / 2;
    icons::draw(c, start, c.h - 58, Icon::Lock, PANEL_SUBTLE);
    font::draw(c, start + 24, c.h - 45, PREVIEW, PANEL_SUBTLE, note);
}

fn conversation_row(c: &mut Canvas, side: i32, y: i32, index: usize, chat: &Chat) {
    let info = chat.conversation(index);
    let selected = chat.open && index == chat.selected;
    let (left, right) = (RAIL_W + 12, side - 12);
    if selected {
        c.fill_round_rect(left, y + 2, right - left, ROW_H - 4, 16, BG_SELECTED);
    }
    avatar(c, RAIL_W + 48, y + ROW_H / 2, 24, AVATAR_COLORS[index % AVATAR_COLORS.len()], info.name());

    let last = chat.last_message(index);
    let mut day_buffer = [0u8; 10];
    let time = last.map_or(info.time(), |message| crate::chat::list_day(message.day, message.time(), &mut day_buffer));
    let preview = last.map_or("", |message| if message.text().is_empty() { message.kind_word() } else { message.text() });
    let unread = info.unread > 0;
    let text_x = RAIL_W + 86;
    let time_w = font::measure(time, META);
    font::draw(c, right - 12 - time_w, y + 31, META, if unread { ACCENT } else { TEXT_SECONDARY }, time);
    fitted(c, text_x, y + 31, NAME, TEXT_PRIMARY, info.name(), right - 12 - time_w - 8 - text_x);

    let badge_w = if unread { (font::measure(number_text(info.unread as usize, &mut [0u8; 3]), META) + 10).max(20) } else { 0 };
    // The crossed speaker of a silenced conversation sits before the badge.
    let silenced_room = if info.silenced { 22 } else { 0 };
    let badge_room = if unread { badge_w + 12 } else { 0 } + silenced_room;
    let room = right - 12 - badge_room - text_x;
    let draft = chat.draft_of(index);
    if draft.is_empty() {
        fitted(c, text_x, y + 54, PREVIEW, TEXT_SECONDARY, preview, room);
    } else {
        // An unsent message shows in green ahead of the last one, as WhatsApp does.
        let label = "Rascunho:";
        let label_w = font::measure(label, PREVIEW) + 4;
        font::draw(c, text_x, y + 54, PREVIEW, ACCENT, label);
        fitted(c, text_x + label_w, y + 54, PREVIEW, TEXT_SECONDARY, draft, room - label_w);
    }
    if info.silenced {
        let x = right - 12 - if unread { badge_w + 6 } else { 0 } - 16;
        icons::draw(c, x, y + 41, Icon::Muted, TEXT_SECONDARY);
    }
    if unread {
        let mut digits = [0u8; 3];
        let text = number_text(info.unread as usize, &mut digits);
        let w = badge_w;
        c.fill_round_rect(right - 12 - w, y + 40, w, 20, 10, ACCENT);
        centered(c, right - 12 - w / 2, y + 54, META, ON_ACCENT, text);
    }
}

/// What the right side shows until a conversation is opened: a card about the calls tab, and
/// four shortcuts under it. The picture on the card is drawn from shapes, not copied.
fn welcome(c: &mut Canvas, x0: i32, _chat: &Chat) {
    let cx = (x0 + c.w) / 2;
    let top = (c.h - 468) / 2 + 8;
    c.fill_round_rect(cx - 176, top, 352, 364, 24, BG_SIDEBAR);
    // The laptop: a lid with a phone on it, and the base.
    let art = top + 40;
    c.fill_round_rect(cx - 50, art, 100, 66, 8, CHIP_ACTIVE_TEXT);
    c.fill_round_rect(cx - 45, art + 5, 90, 56, 5, CHIP_ACTIVE);
    c.fill_round_rect(cx - 62, art + 68, 124, 6, 3, CHIP_ACTIVE_TEXT);
    c.fill_round_rect(cx - 36, art + 12, 30, 42, 4, ACCENT);
    c.fill_round_rect(cx - 2, art + 12, 38, 42, 4, INPUT_PILL);
    icons::draw_centered(c, cx + 17, art + 33, Icon::Calls, ACCENT);
    for (index, line) in ["As ligações de voz e vídeo já", "estão disponíveis"].into_iter().enumerate() {
        centered(c, cx, top + 201 + index as i32 * 28, SCREEN_TITLE, TEXT_PRIMARY, line);
    }
    for (index, line) in ["Agora você pode fazer e participar de ligações", "no WhatsApp Web."].into_iter().enumerate() {
        centered(c, cx, top + 259 + index as i32 * 20, PREVIEW, TEXT_SECONDARY, line);
    }
    let label = "Acessar a aba Ligações";
    let width = font::measure(label, CHIP) + 32;
    c.fill_round_rect(cx - width / 2, top + 308, width, 32, 16, FOLLOW_BG);
    centered(c, cx, top + 329, CHIP, CHIP_ACTIVE_TEXT, label);

    let shortcuts = [("Enviar documento", Icon::Description), ("Adicionar contato", Icon::PersonAdd), ("Nova ligação", Icon::VideoCall), ("Perguntar à Meta AI", Icon::Meta)];
    let gap = 32 * 64;
    let widths = shortcuts.map(|(label, _)| font::measure_exact(label, LABEL));
    let mut x64 = cx * 64 - (widths.iter().sum::<i32>() + 3 * gap) / 2;
    for ((label, icon), width) in shortcuts.into_iter().zip(widths) {
        let center = (x64 + width / 2 + 32) / 64;
        c.fill_round_rect(center - 30, top + 396, 60, 48, 24, RAIL_ACTIVE);
        icons::draw_centered(c, center, top + 420, icon, if matches!(icon, Icon::Meta) { META_PURPLE } else { TEXT_PRIMARY });
        centered(c, center, top + 464, LABEL, TEXT_PRIMARY, label);
        x64 += width + gap;
    }
}

// ---- conversation

fn conversation(c: &mut Canvas, x0: i32, chat: &Chat) {
    let w = c.w - x0;
    c.fill_rect(x0, 0, w, c.h, BG_CHAT);
    let content_top = HEADER_H - chat.conversation_scroll;

    let text_limit = chat.text_limit();
    let mut chip_day = None;
    let content = chat.place(|message, y, starts_run| {
        if chip_day != Some(message.day) {
            chip_day = Some(message.day);
            day_chip(c, x0, w, content_top + y - crate::chat::DAY_CHIP_ROW, message.day);
        }
        bubble(c, x0, content_top + y, message, starts_run, text_limit, chat)
    });
    // A conversation without messages still says what day it is.
    if chip_day.is_none() {
        day_chip(c, x0, w, content_top + crate::chat::CONVERSATION_TOP - crate::chat::DAY_CHIP_ROW, crate::chat::today());
    }

    let info = chat.conversation(chat.selected);
    c.fill_rect(x0, 0, w, HEADER_H, BG_SIDEBAR);
    avatar(c, x0 + 36, HEADER_H / 2, 20, AVATAR_COLORS[chat.selected % AVATAR_COLORS.len()], info.name());
    font::draw(c, x0 + 71, 27, NAME_BOLD, TEXT_PRIMARY, info.name());
    font::draw(c, x0 + 71, 46, META, TEXT_SECONDARY, info.status());
    icon_video(c, c.w - 204, HEADER_H / 2, TEXT_PRIMARY);
    icons::draw_centered(c, c.w - 148, HEADER_H / 2, Icon::Calls, TEXT_PRIMARY);
    icons::draw_centered(c, c.w - 92, HEADER_H / 2, Icon::Search, TEXT_PRIMARY);
    if chat.chat_menu_open {
        c.fill_circle(c.w - 36, HEADER_H / 2, 20, BG_SELECTED);
    }
    icons::draw_centered(c, c.w - 36, HEADER_H / 2, Icon::Menu, TEXT_PRIMARY);

    if !chat.banner.text().is_empty() {
        c.fill_rect(x0, HEADER_H, w, 30, BANNER_BG);
        centered(c, x0 + w / 2, HEADER_H + 20, PREVIEW, BANNER_TEXT, chat.banner.text());
    }
    if chat.find_open {
        find_bar(c, x0, chat);
    }
    reply_bar(c, x0, chat);
    if !chat.at_bottom() {
        let (cx, cy) = scroll_button_center(c.w, c.h - chat.reply_bar_h());
        c.fill_circle(cx, cy, 21, DIVIDER);
        c.fill_circle(cx, cy, 20, BG_SIDEBAR);
        chevron(c, cx, cy, false, TEXT_SECONDARY);
        if chat.below_count > 0 {
            let mut digits = [0u8; 3];
            let text = number_text(chat.below_count as usize, &mut digits);
            let w = (font::measure(text, TINY_MEDIUM) + 10).max(20);
            c.fill_round_rect(cx + 8, cy - 32, w, 20, 10, ACCENT);
            centered(c, cx + 8 + w / 2, cy - 18, TINY_MEDIUM, ON_ACCENT, text);
        }
    }
    input_bar(c, x0, chat);
    scrollbar(c, c.w - 7, HEADER_H, chat.viewport_height(), chat.conversation_scroll, chat.max_conversation_scroll(), content);
    message_menu(c, chat);
    if chat.picker_open {
        emoji_picker(c, x0, chat);
    }
}

pub fn picker_rect(sidebar_w: i32, window_h: i32) -> (i32, i32, i32, i32) {
    let (w, h) = (PICKER_COLS as i32 * PICKER_CELL + 2 * PICKER_PAD, PICKER_ROWS as i32 * PICKER_CELL + 2 * PICKER_PAD);
    (sidebar_w + 60, window_h - INPUT_H - h - 6, w, h)
}

fn emoji_picker(c: &mut Canvas, x0: i32, chat: &Chat) {
    let (x, y, w, h) = picker_rect(x0, c.h);
    c.fill_round_rect(x - 1, y - 1, w + 2, h + 2, 13, BG_SELECTED);
    c.fill_round_rect(x, y, w, h, 12, BG_PANEL);
    let total = font::emoji_total();
    for row in 0..PICKER_ROWS {
        for col in 0..PICKER_COLS {
            let index = (chat.picker_row + row) * PICKER_COLS + col;
            if index < total {
                let (cell_x, cell_y) = (x + PICKER_PAD + col as i32 * PICKER_CELL + (PICKER_CELL - 18) / 2, y + PICKER_PAD + row as i32 * PICKER_CELL + (PICKER_CELL - 18) / 2);
                font::draw_emoji(c, cell_x, cell_y, index);
            }
        }
    }
    let last_row = total.div_ceil(PICKER_COLS).saturating_sub(PICKER_ROWS);
    scrollbar(c, x + w - 6, y + PICKER_PAD, h - 2 * PICKER_PAD, chat.picker_row as i32, last_row as i32, total.div_ceil(PICKER_COLS) as i32 * PICKER_CELL);
}

pub fn in_picker(x: i32, y: i32, window_w: i32, window_h: i32) -> bool {
    let (px, py, w, h) = picker_rect(sidebar_width(window_w), window_h);
    x >= px && x < px + w && y >= py && y < py + h
}

fn bubble(c: &mut Canvas, x0: i32, y: i32, message: &crate::chat::Message, starts_run: bool, text_limit: i32, chat: &Chat) {
    let quote_width = chat.quote_width_of(message);
    let shape = bubble_shape(message.text(), message.time(), message.outgoing, message.starred, text_limit, quote_width, message.block());
    // The reactions pill hangs below the bubble, so its rows count too.
    if y >= c.y0 + c.rows || y + shape.height + crate::chat::REACTION_H <= c.y0 {
        return;
    }
    let width = shape.width + 2 * BUBBLE_PAD_X;
    let x = if message.outgoing { c.w - BUBBLE_MARGIN - 4 - width } else { x0 + BUBBLE_MARGIN };
    let flashed = chat.flash == Some(chat.index_of(message));
    let color = match (message.outgoing, flashed) {
        (true, false) => BUBBLE_OUT,
        (true, true) => 0x1f6a4a,
        (false, false) => BUBBLE_IN,
        (false, true) => 0x3b3f3f,
    };
    c.fill_round_rect(x, y, width, shape.height, BUBBLE_RADIUS, color);
    if starts_run {
        for i in 0..TAIL {
            let reach = TAIL - i;
            let left = if message.outgoing { x + width - TAIL } else { x - reach };
            c.fill_rect(left, y + i, reach + TAIL, 1, color);
        }
    }

    if chat.hover == Some(chat.index_of(message)) && !message.deleted && message.id().len() > 0 {
        let bx = if message.outgoing { x - 24 } else { x + width + 24 };
        c.fill_circle(bx, y + 22, 15, BG_SIDEBAR);
        icon_smiley(c, bx, y + 22, TEXT_SECONDARY, BG_SIDEBAR);
    }
    let query = chat.find_query();
    let highlight = if chat.is_current_find(message) { FIND_CURRENT } else { FIND_MATCH };
    let quote_offset = match chat.quoted_by(message) {
        Some(quoted) => {
            quote_block(c, x + 6, y + 6, width - 12, chat.author_of(quoted), quoted.text(), quoted.outgoing, message.outgoing);
            QUOTE_H
        }
        None => 0,
    };
    let (_, block_h) = message.block();
    let quote_offset = if block_h > 0 {
        media_block(c, x + BUBBLE_PAD_X, y + BUBBLE_PAD_Y + quote_offset, width - 2 * BUBBLE_PAD_X, block_h, message, chat);
        quote_offset + block_h + crate::chat::BLOCK_GAP
    } else {
        quote_offset
    };
    for (index, line) in font::lines(message.text(), BODY, text_limit).enumerate() {
        let baseline = y + BUBBLE_PAD_Y + 14 + quote_offset + index as i32 * LINE_HEIGHT;
        // What the find bar looks for is marked behind the text.
        let mut from = 0;
        while let Some((start, end)) = find_from(line, query, from) {
            let left = x + BUBBLE_PAD_X + font::measure(&line[..start], BODY);
            c.fill_round_rect(left - 1, baseline - 13, font::measure(&line[start..end], BODY) + 2, 17, 3, highlight);
            from = end;
        }
        font::draw(c, x + BUBBLE_PAD_X, baseline, BODY, if message.deleted { TEXT_SECONDARY } else { TEXT_PRIMARY }, line);
    }

    let baseline = y + shape.height - BUBBLE_PAD_Y - if shape.time_inline { 5 } else { 3 };
    let right = x + width - BUBBLE_PAD_X;
    let time_x = right - meta_width(message.time(), message.outgoing, false);
    font::draw(c, time_x, baseline, META, if message.outgoing { TEXT_OUT_META } else { TEXT_SECONDARY }, message.time());
    if message.starred {
        icons::draw(c, time_x - crate::chat::STAR_WIDTH + 1, baseline - 11, Icon::StarSmall, if message.outgoing { TEXT_OUT_META } else { TEXT_SECONDARY });
    }
    if message.outgoing {
        let (icon, tick) = match message.delivery {
            Delivery::Sent => (Icon::Check, TEXT_OUT_META),
            Delivery::Delivered => (Icon::CheckDouble, TEXT_OUT_META),
            Delivery::Read => (Icon::CheckDouble, READ_TICK),
        };
        let ticks_x = right - TICKS_WIDTH;
        icons::draw(c, ticks_x, baseline - 13, icon, tick);
    }
    reactions_pill(c, x, y + shape.height, width, message);
}

/// The box of what a message carries besides text: the preview picture of a photo or video (a plain
/// box until it arrives), a voice message's player, a document's name.
fn media_block(c: &mut Canvas, x: i32, y: i32, w: i32, h: i32, message: &crate::chat::Message, chat: &Chat) {
    use crate::chat::{MEDIA_AUDIO, MEDIA_DOCUMENT};
    if y >= c.y0 + c.rows || y + h <= c.y0 {
        return;
    }
    let inside = if message.outgoing { 0x0f3a2b } else { 0x1b1d1d };
    match message.kind {
        MEDIA_AUDIO => {
            c.fill_round_rect(x, y, w, h, 8, inside);
            c.fill_circle(x + 24, y + h / 2, 16, TEXT_SECONDARY);
            // The play triangle.
            for column in 0..10 {
                let half = (10 - column) * 6 / 10;
                c.fill_rect(x + 19 + column, y + h / 2 - half, 1, 2 * half + 1, inside);
            }
            c.fill_rect(x + 52, y + h / 2, w - 68, 2, TEXT_SECONDARY);
            c.fill_circle(x + 52, y + h / 2 + 1, 4, TEXT_SECONDARY);
            font::draw(c, x + 52, y + h - 6, META, TEXT_SECONDARY, message.label());
        }
        MEDIA_DOCUMENT => {
            c.fill_round_rect(x, y, w, h, 8, inside);
            icons::draw_centered(c, x + 28, y + h / 2, Icon::Description, TEXT_SECONDARY);
            fitted(c, x + 54, y + h / 2 + 5, BODY, TEXT_PRIMARY, message.label(), w - 66);
        }
        kind => {
            c.fill_round_rect(x, y, w, h, 6, 0x232626);
            match chat.thumb_of(message) {
                Some((pixels, tw, th)) => draw_thumb(c, x, y, w, h, pixels, tw, th),
                None => centered(c, x + w / 2, y + h / 2 + 5, PREVIEW, TEXT_SECONDARY, message.kind_word()),
            }
            if kind == crate::chat::MEDIA_VIDEO {
                c.fill_circle(x + w / 2, y + h / 2, 22, 0x161717);
                for column in 0..14 {
                    let half = (14 - column) * 9 / 14;
                    c.fill_rect(x + w / 2 - 6 + column, y + h / 2 - half, 1, 2 * half + 1, TEXT_PRIMARY);
                }
                let label = message.label();
                let chip = font::measure(label, META) + 12;
                c.fill_round_rect(x + 6, y + h - 24, chip, 18, 6, 0x161717);
                font::draw(c, x + 12, y + h - 10, META, TEXT_PRIMARY, label);
            }
        }
    }
}

/// `pixels` (RGB565 rows `THUMB_SIDE` apart, `tw` by `th` in use) stretched over the box, smoothed, with
/// the corners left out so they show the rounded box beneath.
fn draw_thumb(c: &mut Canvas, x: i32, y: i32, w: i32, h: i32, pixels: &[u16], tw: usize, th: usize) {
    const RADIUS: i32 = 6;
    let side = crate::chat::THUMB_SIDE;
    let unpack = |p: u16| ((p >> 11) as u32 * 255 / 31, ((p >> 5) & 63) as u32 * 255 / 63, (p & 31) as u32 * 255 / 31);
    for row in y.max(c.y0)..(y + h).min(c.y0 + c.rows) {
        let dy = row - y;
        // Position in the picture in 1/256 pixels, from the centre of the destination pixel.
        let fy = (((dy * 2 + 1) * th as i32 * 128 / h) - 128).clamp(0, (th as i32 - 1) * 256);
        let (y0, wy) = ((fy >> 8) as usize, (fy & 255) as u32);
        let y1 = (y0 + 1).min(th - 1);
        for dx in 0..w {
            let (cx, cy) = (dx.min(w - 1 - dx), dy.min(h - 1 - dy));
            if cx < RADIUS && cy < RADIUS && (RADIUS - cx - 1).pow(2) + (RADIUS - cy - 1).pow(2) > RADIUS * RADIUS {
                continue;
            }
            let fx = (((dx * 2 + 1) * tw as i32 * 128 / w) - 128).clamp(0, (tw as i32 - 1) * 256);
            let (x0, wx) = ((fx >> 8) as usize, (fx & 255) as u32);
            let x1 = (x0 + 1).min(tw - 1);
            let (a, b, cc, d) = (unpack(pixels[y0 * side + x0]), unpack(pixels[y0 * side + x1]), unpack(pixels[y1 * side + x0]), unpack(pixels[y1 * side + x1]));
            let mix = |pa: u32, pb: u32, pc: u32, pd: u32| {
                let top = pa * (256 - wx) + pb * wx;
                let bottom = pc * (256 - wx) + pd * wx;
                (top * (256 - wy) + bottom * wy) >> 16
            };
            let color = mix(a.0, b.0, cc.0, d.0) << 16 | mix(a.1, b.1, cc.1, d.1) << 8 | mix(a.2, b.2, cc.2, d.2);
            c.put(x + dx, row, color);
        }
    }
}

/// The chip that opens a day, `top` being the top of its row.
fn day_chip(c: &mut Canvas, x0: i32, w: i32, top: i32, day: u16) {
    if top + crate::chat::DAY_CHIP_ROW < c.y0 || top > c.y0 + c.rows {
        return;
    }
    let mut buffer = [0u8; 10];
    let label = crate::chat::day_chip(day, &mut buffer);
    let chip_w = font::measure(label, META) + 24;
    c.fill_round_rect(x0 + w / 2 - chip_w / 2, top + 14, chip_w, 26, 8, BG_CHIP);
    centered(c, x0 + w / 2, top + 31, META, TEXT_SECONDARY, label);
}

/// The pill of the footer: attach and emoji buttons, the text, and the mic or send button, all inside.
pub fn message_box(sidebar_w: i32, window_w: i32) -> (i32, i32) {
    (sidebar_w + 12, window_w - sidebar_w - 24)
}

const TEXT_INSET: i32 = 91;

fn send_center(window_w: i32, window_h: i32) -> (i32, i32) {
    (window_w - 38, window_h - INPUT_H / 2)
}

fn input_bar(c: &mut Canvas, x0: i32, chat: &Chat) {
    let y = c.h - INPUT_H;
    c.fill_rect(x0, y, c.w - x0, INPUT_H, BG_SIDEBAR);
    let (box_x, box_w) = message_box(x0, c.w);
    c.fill_round_rect(box_x, y + 6, box_w, 52, 26, INPUT_PILL);
    icon_plus(c, x0 + 36, y + INPUT_H / 2, TEXT_SECONDARY);
    icon_smiley(c, x0 + 76, y + INPUT_H / 2, TEXT_SECONDARY, INPUT_PILL);

    let baseline = y + INPUT_H / 2 + 5;
    let draft = chat.draft.text();
    let text_x = box_x + TEXT_INSET;
    if draft.is_empty() {
        font::draw(c, text_x, baseline, INPUT_TEXT, TEXT_SECONDARY, "Digite uma mensagem");
        if chat.focus == Focus::Message && chat.caret_on && chat.window_focused {
            draw_caret(c, text_x, (baseline - 14, 18));
        }
    } else {
        let show_caret = chat.focus == Focus::Message && chat.caret_on && chat.window_focused;
        edit_text(c, text_x, baseline, INPUT_TEXT, draft, chat.message_caret(), box_w - TEXT_INSET - 64, (baseline - 14, 18), show_caret);
    }
    let (send_x, send_y) = send_center(c.w, c.h);
    if draft.is_empty() {
        icon_mic(c, send_x, send_y, TEXT_SECONDARY);
    } else {
        icon_send(c, send_x, send_y);
    }
}

// ---- hit testing

/// A click on the right side of the calls tab: only the keypad button does anything.
fn pane_hit(x: i32, y: i32, side: i32, window_w: i32, window_h: i32) -> Hit {
    let top = (window_h - 316) / 2 - 12;
    let centers = pane_button_centers(side, window_w);
    if (x - centers[2]).pow(2) + (y - (top + 268)).pow(2) <= 24 * 24 {
        Hit::OpenDialpad
    } else {
        Hit::Nothing
    }
}

pub enum Hit {
    Row(usize),
    Chip(usize),
    SearchBox(i32),
    MessageBox(i32),
    Send,
    EmojiButton,
    Emoji(usize),
    EmojiPanel,
    Conversation,
    Tab(Tab),
    NewChat,
    Menu,
    OpenDialpad,
    Key(char),
    Answer,
    Decline,
    Mute,
    HangUp,
    Call(usize),
    Back,
    Contact(usize),
    SelfChat,
    NumberChat,
    Starred(usize),
    FindOpen,
    ChatMenu,
    HeaderInfo,
    InfoClose,
    InfoRow(usize),
    InfoPanel,
    Find(FindHit),
    ScrollBottom,
    ReplyClose,
    Reaction(usize),
    ReactionMore,
    HoverButton(usize),
    Quote(usize),
    MenuReply,
    MenuCopy,
    MenuForward,
    MenuStar,
    MenuEdit,
    MenuDelete,
    ForwardTo(usize),
    ForwardOutside,
    Nothing,
}

pub fn hit_test(x: i32, y: i32, window_w: i32, window_h: i32, chat: &Chat) -> Hit {
    let side = sidebar_width(window_w);
    if chat.forward_open {
        return forward_hit(x, y, window_w, window_h, chat);
    }
    match chat.call_phase {
        CallPhase::Idle => {}
        CallPhase::Incoming => {
            let inside = |r: (i32, i32, i32, i32)| x >= r.0 && x < r.0 + r.2 && y >= r.1 && y < r.1 + r.3;
            let (decline, answer) = incoming_buttons(window_w, window_h);
            return if inside(answer) {
                Hit::Answer
            } else if inside(decline) {
                Hit::Decline
            } else {
                Hit::Nothing
            };
        }
        CallPhase::Ended => return Hit::Nothing,
        CallPhase::Ringing | CallPhase::Active => {
            let (mute, end) = call_controls(window_w, window_h);
            let near = |p: (i32, i32)| (x - p.0).pow(2) + (y - p.1).pow(2) <= CALL_CONTROL_R * CALL_CONTROL_R;
            return if near(mute) {
                Hit::Mute
            } else if near(end) {
                Hit::HangUp
            } else {
                Hit::Nothing
            };
        }
    }
    if x < RAIL_W {
        return tab_at(y, window_h).map_or(Hit::Nothing, Hit::Tab);
    }
    if chat.tab == Tab::Calls {
        if chat.dialpad_open {
            if x < side && y < HEADER_H {
                return if x < RAIL_W + 55 { Hit::Back } else { Hit::Nothing };
            }
            let (box_x, box_y, box_w, box_h) = search_box(side);
            if x >= box_x && x < box_x + box_w && y >= box_y && y < box_y + box_h {
                return Hit::SearchBox(x);
            }
            return if x < side { key_at(side, x, y).map_or(Hit::Nothing, Hit::Key) } else { pane_hit(x, y, side, window_w, window_h) };
        }
        if x < side && y < HEADER_H && (x - (side - 87)).abs() < 20 {
            return Hit::OpenDialpad;
        }
        if x < side {
            let mut hit = Hit::Nothing;
            chat.calls_place(|item, top| {
                let top = top - chat.panel_scroll;
                if let CallsItem::Favorite(index) = item {
                    if y >= top && y < top + CALLS_ROW_H && y >= CALLS_VIEW_TOP && x >= side - 130 {
                        hit = Hit::Call(index);
                    }
                }
            });
            if !matches!(hit, Hit::Nothing) {
                return hit;
            }
        } else {
            return pane_hit(x, y, side, window_w, window_h);
        }
    }
    if chat.tab != Tab::Chats {
        let (box_x, box_y, box_w, box_h) = search_box(side);
        let in_box = x >= box_x && x < box_x + box_w && y >= box_y && y < box_y + box_h;
        if chat.tab == Tab::Status && (x - (side - 87)).pow(2) + (y - HEADER_H / 2).pow(2) <= 20 * 20 {
            return Hit::Menu;
        }
        return if in_box && chat.tab != Tab::Status { Hit::SearchBox(x) } else { Hit::Nothing };
    }
    if x < side && chat.starred_open {
        if y < HEADER_H {
            return if x < RAIL_W + 55 { Hit::Back } else { Hit::Nothing };
        }
        let row = (y - HEADER_H + chat.panel_scroll) / STARRED_ROW_H;
        return if row >= 0 && (row as usize) < chat.starred_count() { Hit::Starred(row as usize) } else { Hit::Nothing };
    }
    if x < side && chat.new_chat {
        if y < HEADER_H {
            return if x < RAIL_W + 55 { Hit::Back } else { Hit::Nothing };
        }
        let (box_x, box_y, box_w, box_h) = search_box(side);
        if x >= box_x && x < box_x + box_w && y >= box_y && y < box_y + box_h {
            return Hit::SearchBox(x);
        }
        let mut hit = Hit::Nothing;
        chat.contacts_place(|item, top| {
            let top = top - chat.panel_scroll;
            if y >= CALLS_VIEW_TOP && y >= top && y < top + NEW_CHAT_ROW_H {
                match item {
                    ContactsItem::Contact(index) => hit = Hit::Contact(index),
                    ContactsItem::Myself => hit = Hit::SelfChat,
                    ContactsItem::Number => hit = Hit::NumberChat,
                    _ => {}
                }
            }
        });
        return hit;
    }
    if x < side {
        if (x - (side - 39)).pow(2) + (y - HEADER_H / 2).pow(2) <= 20 * 20 {
            return Hit::NewChat;
        }
        if (x - (side - 87)).pow(2) + (y - HEADER_H / 2).pow(2) <= 20 * 20 {
            return Hit::Menu;
        }
        let (box_x, box_y, box_w, box_h) = search_box(side);
        if x >= box_x && x < box_x + box_w && y >= box_y && y < box_y + box_h {
            return Hit::SearchBox(x);
        }
        for (index, (chip_x, chip_y, chip_w, chip_h)) in chip_rects(chat.unread_total()).into_iter().enumerate() {
            if x >= chip_x && x < chip_x + chip_w && y >= chip_y && y < chip_y + chip_h {
                return Hit::Chip(index);
            }
        }
        if y >= LIST_TOP {
            let row = ((y - LIST_TOP + chat.list_scroll) / ROW_H) as usize;
            if row < chat.visible_len {
                return Hit::Row(chat.visible[row] as usize);
            }
        }
        return Hit::Nothing;
    }
    if !chat.open {
        return Hit::Nothing;
    }
    if over_info(chat, x, window_w) {
        if y < HEADER_H {
            return if x < window_w - INFO_W + 60 { Hit::InfoClose } else { Hit::InfoPanel };
        }
        return info_row_at(y).map_or(Hit::InfoPanel, Hit::InfoRow);
    }
    if y < HEADER_H && x < window_w - 230 && x >= side {
        return Hit::HeaderInfo;
    }
    if y < HEADER_H && (x - (window_w - 36)).pow(2) + (y - HEADER_H / 2).pow(2) <= 20 * 20 {
        return Hit::ChatMenu;
    }
    if y < HEADER_H && (x - (window_w - 92)).pow(2) + (y - HEADER_H / 2).pow(2) <= 20 * 20 {
        return Hit::FindOpen;
    }
    if let Some((index, at_x, at_y)) = chat.message_menu {
        let (mx, my, mw, mh) = message_menu_rect(at_x, at_y, window_w, window_h, message_menu_items(chat, index));
        if x >= mx && x < mx + mw && y >= my + 5 && y < my + mh - 5 {
            if y < my + 5 + REACTION_ROW_H {
                let slot = ((x - (mx + 8)) / 28).clamp(0, QUICK_REACTIONS.len() as i32);
                return if slot as usize == QUICK_REACTIONS.len() { Hit::ReactionMore } else { Hit::Reaction(slot as usize) };
            }
            let slot = (y - (my + 5 + REACTION_ROW_H)) / 36;
            return match slot {
                0 => Hit::MenuReply,
                1 => Hit::MenuCopy,
                2 => Hit::MenuForward,
                3 => Hit::MenuStar,
                4 => Hit::MenuEdit,
                _ => Hit::MenuDelete,
            };
        }
    }
    if chat.find_open {
        if let Some(hit) = find_hit(x, y, side, window_w) {
            return Hit::Find(hit);
        }
    }
    if chat.picker_open && in_picker(x, y, window_w, window_h) {
        let (px, py, _, _) = picker_rect(side, window_h);
        let (col, row) = ((x - px - PICKER_PAD) / PICKER_CELL, (y - py - PICKER_PAD) / PICKER_CELL);
        let index = (chat.picker_row + row.max(0) as usize) * PICKER_COLS + col.max(0) as usize;
        let in_grid = x >= px + PICKER_PAD && y >= py + PICKER_PAD && (col as usize) < PICKER_COLS && (row as usize) < PICKER_ROWS;
        return if in_grid && index < font::emoji_total() { Hit::Emoji(index) } else { Hit::EmojiPanel };
    }
    if y >= window_h - INPUT_H {
        let (smiley_x, smiley_y) = (side + 76, window_h - INPUT_H / 2);
        if (x - smiley_x).pow(2) + (y - smiley_y).pow(2) <= 16 * 16 {
            return Hit::EmojiButton;
        }
        let (send_x, send_y) = send_center(window_w, window_h);
        if !chat.draft.text().is_empty() && (x - send_x).pow(2) + (y - send_y).pow(2) <= SEND_RADIUS * SEND_RADIUS {
            return Hit::Send;
        }
        let (box_x, box_w) = message_box(side, window_w);
        return if x >= box_x && x < box_x + box_w && y >= window_h - INPUT_H + 6 { Hit::MessageBox(x) } else { Hit::Nothing };
    }
    if let Some(index) = hover_button_hit(chat, x, y, window_w) {
        return Hit::HoverButton(index);
    }
    if chat.replying_to().is_some() {
        let (cx, cy) = reply_close_center(window_w, window_h);
        if (x - cx).abs() < 20 && (y - cy).abs() < 20 {
            return Hit::ReplyClose;
        }
    }
    if !chat.at_bottom() {
        let (cx, cy) = scroll_button_center(window_w, window_h - chat.reply_bar_h());
        if (x - cx).pow(2) + (y - cy).pow(2) <= 21 * 21 {
            return Hit::ScrollBottom;
        }
    }
    if y >= HEADER_H {
        return quote_at(chat, x, y, window_w).map_or(Hit::Conversation, Hit::Quote);
    }
    Hit::Nothing
}
