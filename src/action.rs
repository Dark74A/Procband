use crossterm::event::{KeyCode, KeyEvent};

/// Actions that can be triggered by the user through the keyboard.
#[derive(Debug, Clone, Copy)]
pub enum Action {
    Quit,
    NextService,
    PrevService,
    ScrollUp,
    ScrollDown,
    KillFocused,
    RestartFocused,
    Noop,
}

/// Number of log lines moved by each scroll action.
pub const SCROLL_STEP: usize = 3;

/// Converts a terminal key press into an application-level action.
pub fn map_key(key: KeyEvent) -> Action {
    match key.code {
        // Quit with 'q' or Escape.
        KeyCode::Char('q') | KeyCode::Esc => Action::Quit,

        // Move between services.
        KeyCode::Down | KeyCode::Char('j') => Action::NextService,
        KeyCode::Up | KeyCode::Char('k') => Action::PrevService,

        // Scroll through the focused service's logs.
        KeyCode::PageUp => Action::ScrollUp,
        KeyCode::PageDown => Action::ScrollDown,

        // Kill or restart the focused service.
        KeyCode::Char('t') => Action::KillFocused,
        KeyCode::Char('r') => Action::RestartFocused,

        // Ignore unsupported keys.
        _ => Action::Noop,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn maps_quit_keys() {
        assert!(matches!(map_key(key(KeyCode::Char('q'))), Action::Quit));
        assert!(matches!(map_key(key(KeyCode::Esc)), Action::Quit));
    }

    #[test]
    fn maps_navigation_keys() {
        assert!(matches!(
            map_key(key(KeyCode::Char('j'))),
            Action::NextService
        ));

        assert!(matches!(map_key(key(KeyCode::Down)), Action::NextService));

        assert!(matches!(
            map_key(key(KeyCode::Char('k'))),
            Action::PrevService
        ));

        assert!(matches!(map_key(key(KeyCode::Up)), Action::PrevService));
    }

    #[test]
    fn maps_control_keys() {
        assert!(matches!(map_key(key(KeyCode::PageUp)), Action::ScrollUp));

        assert!(matches!(
            map_key(key(KeyCode::PageDown)),
            Action::ScrollDown
        ));

        assert!(matches!(
            map_key(key(KeyCode::Char('t'))),
            Action::KillFocused
        ));

        assert!(matches!(
            map_key(key(KeyCode::Char('r'))),
            Action::RestartFocused
        ));
    }

    #[test]
    fn unknown_key_is_noop() {
        assert!(matches!(map_key(key(KeyCode::Char('z'))), Action::Noop));
    }
}
