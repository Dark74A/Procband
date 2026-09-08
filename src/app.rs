use crate::action::{Action, SCROLL_STEP, map_key};
use crate::event::Event;
use crate::state::{AppState, ServiceStatus};
use crate::supervisor;
use crate::tui::Tui;
use crossterm::event::{Event as CEvent, EventStream};
use futures::StreamExt;
use std::time::Duration;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use tokio::time::interval;

/// How often the terminal UI is redrawn.
const TICK_RATE: Duration = Duration::from_millis(100);

/// Main event loop of the application.
///
/// Waits for three types of events:
/// - Timer ticks → redraw the UI
/// - Keyboard input → perform user actions
/// - Process events → update service state and logs
pub async fn run(
    terminal: &mut Tui,
    mut state: AppState,
    mut rx: UnboundedReceiver<Event>,
    mut handles: Vec<JoinHandle<()>>,
    tx: UnboundedSender<Event>,
) -> std::io::Result<()> {
    let mut ticker = interval(TICK_RATE);
    let mut input = EventStream::new();

    while !state.should_quit {
        tokio::select! {
            // Periodically redraw the terminal.
            _ = ticker.tick() => {
                terminal.draw(|frame| crate::ui::render(frame, &state))?;
            }

            // Read keyboard input without blocking process events.
            maybe_event = input.next() => {
                if let Some(Ok(CEvent::Key(key))) = maybe_event {
                    apply_action(&mut state, &mut handles, &tx, map_key(key));
                }
            }

            // Process logs and service status changes.
            Some(event) = rx.recv() => {
                state.apply(event);
            }
        }
    }

    // Stop all service tasks when the application exits.
    // Dropping their ManagedProcess values also cleans up the processes.
    for handle in handles {
        handle.abort();
    }

    Ok(())
}

/// Applies a user action to the application state.
fn apply_action(
    state: &mut AppState,
    handles: &mut Vec<JoinHandle<()>>,
    tx: &UnboundedSender<Event>,
    action: Action,
) {
    match action {
        // Exit the application.
        Action::Quit => state.should_quit = true,

        // Navigate between services.
        Action::NextService => state.next_service(),
        Action::PrevService => state.prev_service(),

        // Scroll the focused service's logs.
        Action::ScrollUp => state.scroll_up(SCROLL_STEP),
        Action::ScrollDown => state.scroll_down(SCROLL_STEP),

        Action::KillFocused => {
            // Abort only the focused service's task.
            if let Some(handle) = handles.get(state.focused) {
                handle.abort();
                state.set_status(state.focused, ServiceStatus::Killed);
            }
        }

        Action::RestartFocused => {
            let idx = state.focused;

            // Keep the original configuration so the service can be spawned again.
            let Some(config) = state.services.get(idx).map(|s| s.config.clone()) else {
                return;
            };

            // Stop the currently running instance first.
            if let Some(old_handle) = handles.get(idx) {
                old_handle.abort();
            }

            // Spawn a new process using the same configuration.
            match supervisor::spawn_one(&config, tx) {
                Ok(new_handle) => {
                    // Replace the old task handle with the new one.
                    handles[idx] = new_handle;

                    // Show that the service is starting again.
                    state.set_status(idx, ServiceStatus::Starting);
                }

                // Show the error if the service could not be restarted.
                Err(error) => {
                    state.set_status(idx, ServiceStatus::Failed { error });
                }
            }
        }

        // Ignore unsupported keys.
        Action::Noop => {}
    }
}
