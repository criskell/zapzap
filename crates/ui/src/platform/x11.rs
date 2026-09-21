//! X11 spoken directly over the Unix socket, with no libc and no heap.

use super::sys::{self, die, SockAddrUn};
use super::{Cursor, EditKey, Event, Key};

const OP_CREATE_WINDOW: u8 = 1;
const OP_CHANGE_WINDOW_ATTRIBUTES: u8 = 2;
const OP_OPEN_FONT: u8 = 45;
const OP_CREATE_GLYPH_CURSOR: u8 = 94;
const OP_MAP_WINDOW: u8 = 8;
const OP_INTERN_ATOM: u8 = 16;
const OP_GET_PROPERTY: u8 = 20;
const OP_SET_SELECTION_OWNER: u8 = 22;
const OP_CONVERT_SELECTION: u8 = 24;
const OP_SEND_EVENT: u8 = 25;
const OP_CHANGE_PROPERTY: u8 = 18;
const OP_CREATE_GC: u8 = 55;
const OP_GET_KEYBOARD_MAPPING: u8 = 101;
const OP_PUT_IMAGE: u8 = 72;

const EV_KEY_PRESS: u8 = 2;
const EV_FOCUS_IN: u8 = 9;
const EV_FOCUS_OUT: u8 = 10;
const EV_SELECTION_REQUEST: u8 = 30;
const EV_SELECTION_NOTIFY: u8 = 31;
const EV_BUTTON_PRESS: u8 = 4;
const EV_MOTION_NOTIFY: u8 = 6;
const EV_EXPOSE: u8 = 12;
const EV_CONFIGURE_NOTIFY: u8 = 22;
const EV_CLIENT_MESSAGE: u8 = 33;

const ATOM_ATOM: u32 = 4;
const ATOM_STRING: u32 = 31;
const ATOM_WM_NAME: u32 = 39;
const ATOM_WM_NORMAL_HINTS: u32 = 40;
const ATOM_WM_SIZE_HINTS: u32 = 41;

const CW_BACK_PIXEL: u32 = 0x2;
const CW_EVENT_MASK: u32 = 0x800;
const MASK_KEY_PRESS: u32 = 0x1;
const MASK_BUTTON_PRESS: u32 = 0x4;
const MASK_POINTER_MOTION: u32 = 0x40;
const CW_CURSOR: u32 = 0x4000;
/// Glyphs of the standard `cursor` font: the arrow and the text (I-beam) cursor.
const GLYPH_ARROW: u16 = 68;
const GLYPH_TEXT_BEAM: u16 = 152;
const GLYPH_HAND: u16 = 60;
const MASK_EXPOSURE: u32 = 0x8000;
const MASK_FOCUS_CHANGE: u32 = 0x200000;
const MASK_STRUCTURE_NOTIFY: u32 = 0x20000;

const FORMAT_Z_PIXMAP: u8 = 2;
const PUT_IMAGE_HEADER_BYTES: usize = 24;
const COOKIE_BYTES: usize = 16;
const AUTH_NAME: &[u8] = b"MIT-MAGIC-COOKIE-1";
const SOCKET_DIR: &[u8] = b"/tmp/.X11-unix/X";
const SETUP_KEPT_BYTES: usize = 512;

/// Fixed-capacity byte buffer for building requests without a heap.
struct Buf<const N: usize> {
    bytes: [u8; N],
    len: usize,
}

impl<const N: usize> Buf<N> {
    fn new() -> Self {
        Self { bytes: [0; N], len: 0 }
    }

    fn put(&mut self, data: &[u8]) -> &mut Self {
        self.bytes[self.len..self.len + data.len()].copy_from_slice(data);
        self.len += data.len();
        self
    }

    fn u16(&mut self, v: u16) -> &mut Self {
        self.put(&v.to_le_bytes())
    }

    fn u32(&mut self, v: u32) -> &mut Self {
        self.put(&v.to_le_bytes())
    }

    fn padded(&mut self, data: &[u8]) -> &mut Self {
        self.put(data);
        let pad = (4 - data.len() % 4) % 4;
        self.put(&[0; 3][..pad])
    }

    fn as_slice(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

pub struct Window {
    fd: i32,
    id: u32,
    gc: u32,
    depth: u8,
    max_request_bytes: usize,
    delete_atom: u32,
    /// Four keysyms per keycode: plain, shifted, and the same two with AltGr.
    keymap: [[u16; 4]; 256],
    atoms: Atoms,
    /// The arrow and the text cursor, and which one the window shows.
    cursors: [u32; 3],
    shown_cursor: Cursor,
    /// What this window put on the clipboard, served to whoever asks while it owns the selection.
    copied: [u8; CLIPBOARD_BYTES],
    copied_len: usize,
    /// What arrived from the clipboard after `request_paste`.
    pasted: [u8; CLIPBOARD_BYTES],
    pasted_len: usize,
}

/// The clipboard holds at most a message's worth of text.
const CLIPBOARD_BYTES: usize = 512;

#[derive(Default, Clone, Copy)]
struct Atoms {
    clipboard: u32,
    utf8_string: u32,
    targets: u32,
    /// The property the pasted text is delivered on.
    paste: u32,
}

impl Window {
    /// # Safety
    /// `envp` must be the environment block the kernel passed at process start.
    pub unsafe fn new(envp: *const *const u8, title: &[u8], width: u32, height: u32) -> Self {
        let display = sys::env(envp, b"DISPLAY").unwrap_or_else(|| die("DISPLAY is not set\n"));
        let colon = display.iter().rposition(|&b| b == b':').unwrap_or_else(|| die("bad DISPLAY\n"));
        let number = &display[colon + 1..];
        let number = &number[..number.iter().position(|&b| b == b'.').unwrap_or(number.len())];

        let mut addr = SockAddrUn { family: 1, path: [0; 108] };
        addr.path[..SOCKET_DIR.len()].copy_from_slice(SOCKET_DIR);
        addr.path[SOCKET_DIR.len()..SOCKET_DIR.len() + number.len()].copy_from_slice(number);
        let fd = sys::connect_unix(&addr);

        let cookie = read_cookie(envp, number);
        let setup = handshake(fd, cookie.as_ref());

        let mut window = Window {
            fd,
            id: setup.resource_base | 1,
            gc: setup.resource_base | 2,
            depth: setup.root_depth,
            max_request_bytes: setup.max_request_bytes,
            delete_atom: 0,
            keymap: [[0; 4]; 256],
            atoms: Atoms::default(),
            cursors: [setup.resource_base | 4, setup.resource_base | 5, setup.resource_base | 6],
            shown_cursor: Cursor::Arrow,
            copied: [0; CLIPBOARD_BYTES],
            copied_len: 0,
            pasted: [0; CLIPBOARD_BYTES],
            pasted_len: 0,
        };
        let protocols_atom = window.intern_atom(b"WM_PROTOCOLS");
        window.delete_atom = window.intern_atom(b"WM_DELETE_WINDOW");
        window.atoms = Atoms {
            clipboard: window.intern_atom(b"CLIPBOARD"),
            utf8_string: window.intern_atom(b"UTF8_STRING"),
            targets: window.intern_atom(b"TARGETS"),
            paste: window.intern_atom(b"ZAPZAP_PASTE"),
        };

        let mut body = Buf::<64>::new();
        // ZAPZAP_POSITION=x,y opens the window there (used to keep test windows on the second monitor).
        let (at_x, at_y) = sys::env(envp, b"ZAPZAP_POSITION").and_then(parse_position).unwrap_or((0, 0));
        body.u32(window.id).u32(setup.root).u16(at_x as u16).u16(at_y as u16).u16(width as u16).u16(height as u16);
        body.u16(0).u16(1).u32(0).u32(CW_BACK_PIXEL | CW_EVENT_MASK).u32(0x0c1317);
        body.u32(MASK_KEY_PRESS | MASK_BUTTON_PRESS | MASK_POINTER_MOTION | MASK_EXPOSURE | MASK_STRUCTURE_NOTIFY | MASK_FOCUS_CHANGE);
        window.request(OP_CREATE_WINDOW, 0, body.as_slice());

        window.change_property(ATOM_WM_NAME, ATOM_STRING, 8, title.len() as u32, title);
        window.change_property(protocols_atom, ATOM_ATOM, 32, 1, &window.delete_atom.to_le_bytes());
        if sys::env(envp, b"ZAPZAP_POSITION").is_some() {
            // WM_NORMAL_HINTS with the user-specified position and size, so the window manager keeps them.
            let mut hints = [0u8; 72];
            hints[..4].copy_from_slice(&3u32.to_le_bytes()); // USPosition | USSize
            for (index, value) in [at_x, at_y, width as i32, height as i32].into_iter().enumerate() {
                hints[4 + index * 4..8 + index * 4].copy_from_slice(&value.to_le_bytes());
            }
            window.change_property(ATOM_WM_NORMAL_HINTS, ATOM_WM_SIZE_HINTS, 32, 18, &hints);
        }
        // WM_HINTS with the input hint set: without it a window manager may never give this window the
        // keyboard focus (ICCCM's "passive input" model needs the client to ask for it).
        let hints_atom = window.intern_atom(b"WM_HINTS");
        let mut hints = [0u8; 36];
        hints[..4].copy_from_slice(&1u32.to_le_bytes()); // flags: InputHint
        hints[4..8].copy_from_slice(&1u32.to_le_bytes()); // input: true
        window.change_property(hints_atom, hints_atom, 32, 9, &hints);

        let mut body = Buf::<12>::new();
        body.u32(window.gc).u32(window.id).u32(0);
        window.request(OP_CREATE_GC, 0, body.as_slice());

        window.load_keymap(setup.min_keycode, setup.max_keycode);
        window.make_cursors(setup.resource_base | 3);
        window.apply_cursor(Cursor::Arrow);

        window.request(OP_MAP_WINDOW, 0, &window.id.to_le_bytes());
        window
    }

    pub fn next_event(&mut self) -> Event {
        loop {
            let mut ev = [0u8; 32];
            sys::read_exact(self.fd, &mut ev);
            match ev[0] & 0x7f {
                0 => die("X11 protocol error\n"),
                EV_EXPOSE if u16_at(&ev, 16) == 0 => return Event::Redraw,
                EV_KEY_PRESS => return Event::Key(self.key(ev[1], u16_at(&ev, 28))),
                // Only real changes of the keyboard focus: not grabs, and not focus moving between this
                // window and its own children or the pointer (details 2, 5, 6, 7).
                EV_MOTION_NOTIFY => return Event::Motion { x: i16_at(&ev, 24) as i32, y: i16_at(&ev, 26) as i32, pressed: u16_at(&ev, 28) & 0x100 != 0 },
                EV_FOCUS_IN | EV_FOCUS_OUT if ev[8] == 0 && matches!(ev[1], 0 | 1 | 3 | 4) => return Event::Focus(ev[0] & 0x7f == EV_FOCUS_IN),
                EV_SELECTION_REQUEST => self.serve_selection_request(&ev),
                EV_SELECTION_NOTIFY => {
                    if u32_at(&ev, 20) != 0 && self.read_paste() {
                        return Event::Paste;
                    }
                }
                EV_BUTTON_PRESS => {
                    let (x, y) = (i16_at(&ev, 24) as i32, i16_at(&ev, 26) as i32);
                    match ev[1] {
                        1 => return Event::Click { x, y },
                        3 => return Event::RightClick { x, y },
                        4 => return Event::Scroll { x, y, down: false },
                        5 => return Event::Scroll { x, y, down: true },
                        _ => {}
                    }
                }
                EV_CONFIGURE_NOTIFY => return Event::Resize(u16_at(&ev, 20) as u32, u16_at(&ev, 22) as u32),
                EV_CLIENT_MESSAGE if u32_at(&ev, 12) == self.delete_atom => return Event::Close,
                _ => {}
            }
        }
    }

    pub fn fd(&self) -> i32 {
        self.fd
    }

    pub fn has_pending(&self) -> bool {
        sys::has_pending(self.fd)
    }

    pub fn max_rows(&self, width: u32) -> usize {
        ((self.max_request_bytes - PUT_IMAGE_HEADER_BYTES) / (width as usize * 4)).max(1)
    }

    pub fn present(&mut self, pixels: &[u32], width: u32, y: u32, rows: u32) {
        let data = unsafe { core::slice::from_raw_parts(pixels.as_ptr().cast::<u8>(), pixels.len() * 4) };
        let mut header = Buf::<PUT_IMAGE_HEADER_BYTES>::new();
        header.put(&[OP_PUT_IMAGE, FORMAT_Z_PIXMAP]).u16(((PUT_IMAGE_HEADER_BYTES + data.len()) / 4) as u16);
        header.u32(self.id).u32(self.gc).u16(width as u16).u16(rows as u16).u16(0).u16(y as u16);
        header.put(&[0, self.depth, 0, 0]);
        sys::write_all_pair(self.fd, header.as_slice(), data);
    }

    /// Asks the server which keysyms each keycode produces. Done before the window is mapped, so
    /// the reply cannot be mixed up with an event.
    fn load_keymap(&mut self, min_keycode: u8, max_keycode: u8) {
        let count = max_keycode as usize - min_keycode as usize + 1;
        let mut body = Buf::<4>::new();
        body.put(&[min_keycode, count as u8]).u16(0);
        self.request(OP_GET_KEYBOARD_MAPPING, 0, body.as_slice());
        let mut head = [0u8; 32];
        sys::read_exact(self.fd, &mut head);
        if head[0] != 1 {
            die("GetKeyboardMapping failed\n");
        }
        let bytes_per_keycode = head[1] as usize * 4;
        for keycode in min_keycode as usize..=max_keycode as usize {
            let mut first = [0u8; 16];
            let kept = bytes_per_keycode.min(first.len());
            sys::read_exact(self.fd, &mut first[..kept]);
            let mut rest = bytes_per_keycode - kept;
            let mut scratch = [0u8; 64];
            while rest > 0 {
                let n = rest.min(scratch.len());
                sys::read_exact(self.fd, &mut scratch[..n]);
                rest -= n;
            }
            for (slot, keysym) in self.keymap[keycode].iter_mut().zip(first[..kept].chunks_exact(4)) {
                *slot = u16::try_from(u32::from_le_bytes([keysym[0], keysym[1], keysym[2], keysym[3]])).unwrap_or(0);
            }
        }
    }

    fn key(&self, keycode: u8, state: u16) -> Key {
        let (shift, caps_lock, level3, ctrl) = (state & 0x1 != 0, state & 0x2 != 0, state & 0x80 != 0, state & 0x4 != 0);
        let symbols = &self.keymap[keycode as usize];
        let base = if level3 { 2 } else { 0 };
        let is_letter = matches!(symbols[base], 0x61..=0x7a | 0xe0..=0xf6 | 0xf8..=0xfe);
        let upper = shift ^ (caps_lock && is_letter);
        let keysym = match symbols[base + usize::from(upper)] {
            0 => symbols[base],
            keysym => keysym,
        };
        // Ctrl and a letter is a command, whatever the case or layout level: the plain letter names it.
        if ctrl && matches!(symbols[0], 0x61..=0x7a) {
            return Key::Ctrl(symbols[0] as u8 as char, shift);
        }
        let edit = |key| Key::Edit { key, shift, ctrl };
        match keysym {
            0x20..=0x7e | 0xa0..=0xff => Key::Char(char::from_u32(keysym as u32).unwrap_or('?')),
            0xff08 if ctrl => edit(EditKey::Backspace),
            0xff50 => edit(EditKey::Home),
            0xff51 => edit(EditKey::Left),
            0xff53 => edit(EditKey::Right),
            0xff57 => edit(EditKey::End),
            0xffff => edit(EditKey::Delete),
            0xfe50 => Key::Dead('`'),
            0xfe51 => Key::Dead('´'),
            0xfe52 => Key::Dead('^'),
            0xfe53 => Key::Dead('~'),
            0xfe57 => Key::Dead('¨'),
            0xff08 => Key::Backspace,
            0xff0d | 0xff8d => Key::Enter,
            0xff1b => Key::Escape,
            0xff52 => Key::Up,
            0xff54 => Key::Down,
            0xff55 => Key::PageUp,
            0xff56 => Key::PageDown,
            _ => Key::Other,
        }
    }

    /// Creates the two cursors from the standard cursor font (black with a white outline).
    fn make_cursors(&mut self, font: u32) {
        let mut body = Buf::<16>::new();
        body.u32(font).u16(6).u16(0).padded(b"cursor");
        self.request(OP_OPEN_FONT, 0, body.as_slice());
        for (cursor, glyph) in self.cursors.into_iter().zip([GLYPH_ARROW, GLYPH_TEXT_BEAM, GLYPH_HAND]) {
            let mut body = Buf::<28>::new();
            body.u32(cursor).u32(font).u32(font).u16(glyph).u16(glyph + 1);
            body.u16(0).u16(0).u16(0).u16(0xffff).u16(0xffff).u16(0xffff);
            self.request(OP_CREATE_GLYPH_CURSOR, 0, body.as_slice());
        }
    }

    /// Shows `cursor` over the window (nothing is sent when it already shows).
    pub fn show_cursor(&mut self, cursor: Cursor) {
        if cursor != self.shown_cursor {
            self.apply_cursor(cursor);
        }
    }

    fn apply_cursor(&mut self, cursor: Cursor) {
        self.shown_cursor = cursor;
        let mut body = Buf::<12>::new();
        body.u32(self.id).u32(CW_CURSOR).u32(self.cursors[cursor as usize]);
        self.request(OP_CHANGE_WINDOW_ATTRIBUTES, 0, body.as_slice());
    }

    /// Puts `text` (cut to fit) on the clipboard: this window becomes its owner and answers requests.
    pub fn copy(&mut self, text: &str) {
        let mut len = text.len().min(CLIPBOARD_BYTES);
        while !text.is_char_boundary(len) {
            len -= 1;
        }
        self.copied[..len].copy_from_slice(&text.as_bytes()[..len]);
        self.copied_len = len;
        let mut body = Buf::<12>::new();
        body.u32(self.id).u32(self.atoms.clipboard).u32(0);
        self.request(OP_SET_SELECTION_OWNER, 0, body.as_slice());
    }

    /// Asks the clipboard owner for its text; `Event::Paste` follows when it arrives.
    pub fn request_paste(&mut self) {
        let mut body = Buf::<20>::new();
        body.u32(self.id).u32(self.atoms.clipboard).u32(self.atoms.utf8_string).u32(self.atoms.paste).u32(0);
        self.request(OP_CONVERT_SELECTION, 0, body.as_slice());
    }

    /// The text that arrived after `request_paste`.
    pub fn pasted(&self) -> &str {
        core::str::from_utf8(&self.pasted[..self.pasted_len]).unwrap_or("")
    }

    /// Reads the delivered property (and deletes it); false when it is empty or not text we can take.
    fn read_paste(&mut self) -> bool {
        let mut body = Buf::<24>::new();
        body.u32(self.id).u32(self.atoms.paste).u32(self.atoms.utf8_string).u32(0).u32((CLIPBOARD_BYTES / 4) as u32);
        self.request(OP_GET_PROPERTY, 1, body.as_slice());
        // The reply is the next thing that is not an event; events seen on the way are dropped.
        let mut head = [0u8; 32];
        loop {
            sys::read_exact(self.fd, &mut head);
            match head[0] {
                1 => break,
                0 => return false,
                _ => {}
            }
        }
        let (format, len) = (head[1], u32_at(&head, 16) as usize);
        let bytes = if format == 8 { len.min(CLIPBOARD_BYTES) } else { 0 };
        let padded = u32_at(&head, 4) as usize * 4;
        let mut scratch = [0u8; 64];
        let mut read = 0;
        while read < padded {
            let n = (padded - read).min(scratch.len());
            sys::read_exact(self.fd, &mut scratch[..n]);
            for (offset, &byte) in scratch[..n].iter().enumerate() {
                if read + offset < bytes {
                    self.pasted[read + offset] = byte;
                }
            }
            read += n;
        }
        self.pasted_len = bytes;
        bytes > 0
    }

    /// Another client wants the text we copied: send it as a property and tell them.
    fn serve_selection_request(&mut self, ev: &[u8; 32]) {
        let (time, requestor, selection, target, property) = (u32_at(ev, 4), u32_at(ev, 12), u32_at(ev, 16), u32_at(ev, 20), u32_at(ev, 24));
        let property = if property == 0 { target } else { property };
        let mut served = false;
        if selection == self.atoms.clipboard {
            if target == self.atoms.utf8_string || target == ATOM_STRING {
                let mut body = Buf::<{ 24 + CLIPBOARD_BYTES + 4 }>::new();
                body.u32(requestor).u32(property).u32(target).put(&[8, 0, 0, 0]).u32(self.copied_len as u32);
                let text = self.copied;
                body.padded(&text[..self.copied_len]);
                self.request(OP_CHANGE_PROPERTY, 0, body.as_slice());
                served = true;
            } else if target == self.atoms.targets {
                let mut list = Buf::<12>::new();
                list.u32(self.atoms.targets).u32(self.atoms.utf8_string).u32(ATOM_STRING);
                let mut body = Buf::<36>::new();
                body.u32(requestor).u32(property).u32(ATOM_ATOM).put(&[32, 0, 0, 0]).u32(3).put(list.as_slice());
                self.request(OP_CHANGE_PROPERTY, 0, body.as_slice());
                served = true;
            }
        }
        // SelectionNotify to the requestor: `property` when served, None (0) when refused.
        let mut notify = Buf::<32>::new();
        notify.put(&[EV_SELECTION_NOTIFY, 0]).u16(0).u32(time).u32(requestor).u32(selection).u32(target).u32(if served { property } else { 0 });
        notify.put(&[0; 8]);
        let mut body = Buf::<40>::new();
        body.u32(requestor).u32(0).put(notify.as_slice());
        self.request(OP_SEND_EVENT, 0, body.as_slice());
    }

    fn request(&mut self, opcode: u8, data: u8, body: &[u8]) {
        let mut head = Buf::<4>::new();
        head.put(&[opcode, data]).u16((1 + body.len() / 4) as u16);
        sys::write_all(self.fd, head.as_slice());
        sys::write_all(self.fd, body);
    }

    fn intern_atom(&mut self, name: &[u8]) -> u32 {
        let mut body = Buf::<64>::new();
        body.u16(name.len() as u16).u16(0).padded(name);
        self.request(OP_INTERN_ATOM, 0, body.as_slice());
        let mut reply = [0u8; 32];
        sys::read_exact(self.fd, &mut reply);
        if reply[0] != 1 {
            die("InternAtom failed\n");
        }
        u32_at(&reply, 8)
    }

    fn change_property(&mut self, property: u32, kind: u32, format: u8, items: u32, data: &[u8]) {
        let mut body = Buf::<128>::new();
        body.u32(self.id).u32(property).u32(kind).put(&[format, 0, 0, 0]).u32(items).padded(data);
        self.request(OP_CHANGE_PROPERTY, 0, body.as_slice());
    }
}

struct Setup {
    resource_base: u32,
    max_request_bytes: usize,
    root: u32,
    root_depth: u8,
    min_keycode: u8,
    max_keycode: u8,
}

fn handshake(fd: i32, cookie: Option<&[u8; COOKIE_BYTES]>) -> Setup {
    let mut req = Buf::<64>::new();
    req.put(&[b'l', 0]).u16(11).u16(0);
    match cookie {
        Some(cookie) => {
            req.u16(AUTH_NAME.len() as u16).u16(COOKIE_BYTES as u16).u16(0);
            req.padded(AUTH_NAME).padded(cookie);
        }
        None => {
            req.u16(0).u16(0).u16(0);
        }
    }
    sys::write_all(fd, req.as_slice());

    // The reply can be tens of KiB (every visual the server has); keep the head, drain the rest.
    let mut reply = [0u8; 8 + SETUP_KEPT_BYTES];
    sys::read_exact(fd, &mut reply[..8]);
    let status = reply[0];
    let mut remaining = u16_at(&reply, 6) as usize * 4;
    let kept = remaining.min(SETUP_KEPT_BYTES);
    sys::read_exact(fd, &mut reply[8..8 + kept]);
    remaining -= kept;
    let mut scratch = [0u8; 256];
    while remaining > 0 {
        let n = remaining.min(scratch.len());
        sys::read_exact(fd, &mut scratch[..n]);
        remaining -= n;
    }
    if status != 1 {
        die("X server refused the connection\n");
    }

    let vendor_len = u16_at(&reply, 24) as usize;
    let formats = reply[29] as usize;
    let screen = 40 + vendor_len.div_ceil(4) * 4 + formats * 8;
    if screen + 39 > 8 + kept {
        die("X setup reply too large\n");
    }
    Setup {
        resource_base: u32_at(&reply, 12),
        max_request_bytes: u16_at(&reply, 26) as usize * 4,
        root: u32_at(&reply, screen),
        root_depth: reply[screen + 38],
        min_keycode: reply[34],
        max_keycode: reply[35],
    }
}

/// Finds the MIT-MAGIC-COOKIE-1 entry for this display in the Xauthority file (first 4 KiB only).
unsafe fn read_cookie(envp: *const *const u8, display: &[u8]) -> Option<[u8; COOKIE_BYTES]> {
    let mut path = [0u8; 256];
    let path_len = match sys::env(envp, b"XAUTHORITY") {
        Some(p) if p.len() < path.len() => {
            path[..p.len()].copy_from_slice(p);
            p.len()
        }
        _ => {
            let home = sys::env(envp, b"HOME")?;
            let file = b"/.Xauthority";
            if home.len() + file.len() >= path.len() {
                return None;
            }
            path[..home.len()].copy_from_slice(home);
            path[home.len()..home.len() + file.len()].copy_from_slice(file);
            home.len() + file.len()
        }
    };
    let mut data = [0u8; 4096];
    let filled = sys::read_file(&path[..=path_len], &mut data);

    let mut rest = &data[..filled];
    while rest.len() >= 2 {
        rest = &rest[2..];
        let mut fields: [&[u8]; 4] = [&[]; 4];
        for field in &mut fields {
            let len = u16::from_be_bytes([*rest.first()?, *rest.get(1)?]) as usize;
            *field = rest.get(2..2 + len)?;
            rest = &rest[2 + len..];
        }
        let [_address, number, name, cookie] = fields;
        if name == AUTH_NAME && (number.is_empty() || number == display) && cookie.len() == COOKIE_BYTES {
            let mut found = [0u8; COOKIE_BYTES];
            found.copy_from_slice(cookie);
            return Some(found);
        }
    }
    None
}

/// `x,y` as two whole numbers.
fn parse_position(text: &[u8]) -> Option<(i32, i32)> {
    let comma = text.iter().position(|&b| b == b',')?;
    let number = |digits: &[u8]| digits.iter().try_fold(0i32, |total, &digit| digit.is_ascii_digit().then(|| total * 10 + (digit - b'0') as i32));
    Some((number(&text[..comma])?, number(&text[comma + 1..])?))
}

fn u16_at(buf: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([buf[at], buf[at + 1]])
}

fn i16_at(buf: &[u8], at: usize) -> i16 {
    i16::from_le_bytes([buf[at], buf[at + 1]])
}

fn u32_at(buf: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([buf[at], buf[at + 1], buf[at + 2], buf[at + 3]])
}
