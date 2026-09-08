use crate::config::{Config, ServiceConfig};
use crate::event::Event;
use crate::manager::{ManagedProcess, run_and_report};
use crate::state::AppState;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;

/// Maximum number of log lines stored for each service.
const DEFAULT_LOG_CAPACITY: usize = 1000;

/// Spawns one service as a separate Tokio task.
pub fn spawn_one(
    config: &ServiceConfig,
    tx: &UnboundedSender<Event>,
) -> Result<JoinHandle<()>, String> {
    // Create the OS process from the service configuration.
    let process = ManagedProcess::spawn(config)?;

    // Run the process and report its events through the channel.
    Ok(tokio::spawn(run_and_report(process, tx.clone())))
}

/// Starts all services and creates the shared application state and
/// event channel.
pub fn spawn_all(
    config: Config,
) -> (
    AppState,
    UnboundedReceiver<Event>,
    Vec<JoinHandle<()>>,
    UnboundedSender<Event>,
) {
    // Create the channel used by service tasks to send events to the TUI.
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    // Create initial state with all services marked as Starting.
    let state = AppState::new(&config.services, DEFAULT_LOG_CAPACITY);

    // Store one task handle for each service.
    let mut handles = Vec::with_capacity(config.services.len());

    // Start every configured service.
    for service in &config.services {
        let handle = spawn_one(service, &tx).expect("failed to spawn service");
        handles.push(handle);
    }

    // Keep the sender so new services can be restarted later.
    (state, rx, handles, tx)
}
