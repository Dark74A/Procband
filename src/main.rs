mod action; // Keyboard input and user actions
mod app; // Main event loop
mod config; // Procfile parsing
mod event; // Events sent by running services
mod log_buffer; // Bounded service log storage
mod manager; // Process creation and management
mod state; // Application state
mod supervisor; // Starts and manages all services
mod tui; // Terminal setup and cleanup
mod ui; // Terminal UI rendering

use config::Config;

#[tokio::main]
async fn main() {
    // Read the Procfile from the current directory.
    let input = std::fs::read_to_string("Procfile").expect("failed to read Procfile");

    // Parse the Procfile into service configurations.
    let config = Config::from_str(&input).expect("invalid Procfile");

    // Start all configured services and create the event channel
    // used to receive their logs and status updates.
    let (state, rx, handles, tx) = supervisor::spawn_all(config);

    // Set up the terminal for the TUI.
    let mut terminal = tui::init().expect("failed to initialize terminal");

    // Run the application until the user quits.
    let result = app::run(&mut terminal, state, rx, handles, tx).await;

    // Restore the terminal before exiting.
    tui::restore().expect("failed to restore terminal");

    // Report any error that caused the application to stop.
    if let Err(err) = result {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}
