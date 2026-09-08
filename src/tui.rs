use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io::{self, Stdout};

/// Terminal type used by the TUI.
pub type Tui = Terminal<CrosstermBackend<Stdout>>;

/// Initializes the terminal for the TUI.
///
/// Enables raw input mode, switches to the alternate screen, and installs
/// a panic hook to restore the terminal if the application crashes.
pub fn init() -> io::Result<Tui> {
    // Enable raw mode so key presses are received immediately.
    enable_raw_mode()?;

    // Use a separate screen so the user's normal terminal remains unchanged.
    execute!(io::stdout(), EnterAlternateScreen)?;

    // Restore the terminal before printing a panic message.
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = restore();
        default_panic(info);
    }));

    Terminal::new(CrosstermBackend::new(io::stdout()))
}

/// Restores the terminal to its normal state.
pub fn restore() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}
