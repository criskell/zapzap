pub mod sys;
mod x11;

pub use x11::Window;

/// What a key press means once the keyboard layout is applied.
pub enum Key {
    Char(char),
    /// An accent waiting for the letter it goes on (`~` then `a` gives ã).
    Dead(char),
    Backspace,
    Enter,
    Escape,
    Up,
    Down,
    PageUp,
    PageDown,
    /// A cursor or deletion key with the modifiers that change what it does.
    Edit { key: EditKey, shift: bool, ctrl: bool },
    /// Ctrl and a letter (lowercase), and whether Shift was held too.
    Ctrl(char, bool),
    Other,
}

#[derive(Clone, Copy, PartialEq)]
pub enum EditKey {
    Left,
    Right,
    Home,
    End,
    Delete,
    /// Backspace with Ctrl held (plain Backspace is `Key::Backspace`).
    Backspace,
}

/// The mouse cursor shapes the window uses.
#[derive(Clone, Copy, PartialEq)]
pub enum Cursor {
    Arrow,
    /// The I-beam over text boxes.
    Text,
    /// The hand over buttons and rows that a click does something with.
    Pointer,
}

pub enum Event {
    Close,
    Redraw,
    Resize(u32, u32),
    Key(Key),
    Click { x: i32, y: i32 },
    /// The right button.
    RightClick { x: i32, y: i32 },
    Scroll { x: i32, y: i32, down: bool },
    /// The pointer moved over the window.
    Motion { x: i32, y: i32, pressed: bool },
    /// The window gained (true) or lost (false) the keyboard focus.
    Focus(bool),
    /// The clipboard text asked for with `Window::request_paste` has arrived (`Window::pasted`).
    Paste,
}
