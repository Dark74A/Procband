use std::time::Instant;

/// Identifies which output stream produced a log line.
#[derive(Debug, Clone)]
pub enum StreamKind {
    Stdout,
    Stderr,
}

/// Events sent from process management to the application state.
#[derive(Debug, Clone)]
pub enum Event {
    /// A new line of output from a service.
    Log {
        service: String,
        stream: StreamKind,
        line: String,
        at: Instant,
    },

    /// A service process was successfully started.
    ProcessStarted { service: String, pid: u32 },

    /// A service process has exited.
    ProcessExited { service: String, code: Option<i32> },

    /// An error occurred while managing the process.
    ProcessFailed { service: String, error: String },
}
