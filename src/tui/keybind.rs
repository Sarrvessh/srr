use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Submit,
    Newline,
    Backspace,
    Delete,
    CursorLeft,
    CursorRight,
    CursorHome,
    CursorEnd,
    HistoryPrev,
    HistoryNext,
    CycleMode,
    Interrupt,
    Quit,
    ScrollUp,
    ScrollDown,
    ScrollToBottom,
    ClearInput,
    ExternalEditor,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingContext {
    Global,
    Input,
    Autocomplete,
    Approval,
    Streaming,
}

pub struct KeyBind {
    pub key: KeyEvent,
    pub action: Action,
    pub desc: &'static str,
}

fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

fn char_key(c: char, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), modifiers)
}

pub fn default_bindings() -> Vec<KeyBind> {
    vec![
        // Submit
        KeyBind { key: key(KeyCode::Enter, KeyModifiers::NONE), action: Action::Submit, desc: "Send message" },
        // Newline in input
        KeyBind { key: key(KeyCode::Enter, KeyModifiers::SHIFT), action: Action::Newline, desc: "Insert newline" },
        KeyBind { key: key(KeyCode::Char('j'), KeyModifiers::CONTROL), action: Action::Newline, desc: "Insert newline" },
        // Navigation
        KeyBind { key: key(KeyCode::Left, KeyModifiers::NONE), action: Action::CursorLeft, desc: "Move cursor left" },
        KeyBind { key: key(KeyCode::Right, KeyModifiers::NONE), action: Action::CursorRight, desc: "Move cursor right" },
        KeyBind { key: key(KeyCode::Home, KeyModifiers::NONE), action: Action::CursorHome, desc: "Move to start" },
        KeyBind { key: key(KeyCode::End, KeyModifiers::NONE), action: Action::CursorEnd, desc: "Move to end" },
        KeyBind { key: key(KeyCode::Up, KeyModifiers::NONE), action: Action::HistoryPrev, desc: "History / scroll up" },
        KeyBind { key: key(KeyCode::Down, KeyModifiers::NONE), action: Action::HistoryNext, desc: "History / scroll down" },
        // Delete
        KeyBind { key: key(KeyCode::Backspace, KeyModifiers::NONE), action: Action::Backspace, desc: "Delete before cursor" },
        KeyBind { key: key(KeyCode::Delete, KeyModifiers::NONE), action: Action::Delete, desc: "Delete at cursor" },
        // Mode
        KeyBind { key: char_key('\t', KeyModifiers::SHIFT), action: Action::CycleMode, desc: "Cycle agent/plan/yolo mode" },
        // Scrolling
        KeyBind { key: key(KeyCode::PageUp, KeyModifiers::NONE), action: Action::ScrollUp, desc: "Scroll up" },
        KeyBind { key: key(KeyCode::PageDown, KeyModifiers::NONE), action: Action::ScrollDown, desc: "Scroll down" },
        // External editor
        KeyBind { key: key(KeyCode::Char('g'), KeyModifiers::CONTROL), action: Action::ExternalEditor, desc: "Open in external editor" },
        // Scroll to bottom / re-enable auto-scroll
        KeyBind { key: key(KeyCode::End, KeyModifiers::CONTROL), action: Action::ScrollToBottom, desc: "Scroll to bottom / auto-scroll" },
        // Quit
        KeyBind { key: key(KeyCode::Esc, KeyModifiers::NONE), action: Action::Quit, desc: "Quit / back / interrupt" },
        // Interrupt streaming / clear input
        KeyBind { key: key(KeyCode::Char('c'), KeyModifiers::CONTROL), action: Action::Interrupt, desc: "Interrupt / clear input" },
    ]
}

pub fn lookup(key: KeyEvent, _ctx: BindingContext, bindings: &[KeyBind]) -> Action {
    for b in bindings {
        if b.key == key {
            return b.action;
        }
    }
    Action::None
}
