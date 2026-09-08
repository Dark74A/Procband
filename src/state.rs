use crate::config::ServiceConfig;
use crate::event::Event;
use crate::log_buffer::{LogBuffer, LogLine};

/// Represents the current lifecycle state of a service.
#[derive(Debug, Clone)]
pub enum ServiceStatus {
    /// Process is being started.
    Starting,

    /// Process is running with its PID.
    Running { pid: u32 },

    /// Process exited normally.
    Exited { code: Option<i32> },

    /// Process exited with a non-zero code.
    Crashed { code: Option<i32> },

    /// An error occurred while managing the process.
    Failed { error: String },

    /// Process was explicitly stopped by the user.
    Killed,
}

/// Stores the configuration, status, and logs for one service.
pub struct ServiceState {
    pub config: ServiceConfig,
    pub status: ServiceStatus,
    pub logs: LogBuffer,
}

/// Holds all state required to render and control the TUI.
pub struct AppState {
    pub services: Vec<ServiceState>,

    /// Index of the currently focused service.
    pub focused: usize,

    /// Number of log lines scrolled up from the latest output.
    pub scroll: usize,

    /// Signals the main loop to exit.
    pub should_quit: bool,
}

impl AppState {
    /// Creates the initial application state for all configured services.
    pub fn new(configs: &[ServiceConfig], log_capacity: usize) -> Self {
        let services = configs
            .iter()
            .map(|c| ServiceState {
                config: c.clone(),
                status: ServiceStatus::Starting,
                logs: LogBuffer::new(log_capacity),
            })
            .collect();

        Self {
            services,
            focused: 0,
            scroll: 0,
            should_quit: false,
        }
    }

    /// Moves focus to the next service, wrapping around at the end.
    pub fn next_service(&mut self) {
        if !self.services.is_empty() {
            self.focused = (self.focused + 1) % self.services.len();
            self.scroll = 0;
        }
    }

    /// Moves focus to the previous service, wrapping around at the beginning.
    pub fn prev_service(&mut self) {
        if !self.services.is_empty() {
            self.focused = (self.focused + self.services.len() - 1) % self.services.len();
            self.scroll = 0;
        }
    }

    /// Scrolls toward older log lines.
    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll = self.scroll.saturating_add(amount);
    }

    /// Scrolls toward newer log lines, stopping at the latest output.
    pub fn scroll_down(&mut self, amount: usize) {
        self.scroll = self.scroll.saturating_sub(amount);
    }

    /// Directly changes a service's status.
    pub fn set_status(&mut self, index: usize, status: ServiceStatus) {
        if let Some(s) = self.services.get_mut(index) {
            s.status = status;
        }
    }

    /// Applies a process event to the application state.
    pub fn apply(&mut self, event: Event) {
        match event {
            // The process successfully started.
            Event::ProcessStarted { service, pid } => {
                if let Some(s) = self.find_mut(&service) {
                    s.status = ServiceStatus::Running { pid };
                }
            }

            // Add new output to the service's log buffer.
            Event::Log {
                service,
                stream,
                line,
                ..
            } => {
                if let Some(s) = self.find_mut(&service) {
                    s.logs.push(LogLine { stream, text: line });
                }
            }

            // A zero exit code means success; anything else is treated as a crash.
            Event::ProcessExited { service, code } => {
                if let Some(s) = self.find_mut(&service) {
                    s.status = match code {
                        Some(0) => ServiceStatus::Exited { code },
                        _ => ServiceStatus::Crashed { code },
                    };
                }
            }

            // Record process-management errors.
            Event::ProcessFailed { service, error } => {
                if let Some(s) = self.find_mut(&service) {
                    s.status = ServiceStatus::Failed { error };
                }
            }
        }
    }

    /// Finds a service by name and returns a mutable reference to it.
    fn find_mut(&mut self, name: &str) -> Option<&mut ServiceState> {
        self.services.iter_mut().find(|s| s.config.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::StreamKind;

    fn test_configs() -> Vec<ServiceConfig> {
        vec![
            ServiceConfig {
                name: "gateway".into(),
                command: "echo gateway".into(),
            },
            ServiceConfig {
                name: "auth".into(),
                command: "echo auth".into(),
            },
            ServiceConfig {
                name: "users".into(),
                command: "echo users".into(),
            },
        ]
    }

    #[test]
    fn starts_with_first_service_focused() {
        let state = AppState::new(&test_configs(), 100);

        assert_eq!(state.focused, 0);
        assert_eq!(state.scroll, 0);
        assert!(!state.should_quit);
    }

    #[test]
    fn next_service_wraps_around() {
        let configs = test_configs();
        let mut state = AppState::new(&configs, 100);

        state.next_service();
        assert_eq!(state.focused, 1);

        state.next_service();
        assert_eq!(state.focused, 2);

        state.next_service();
        assert_eq!(state.focused, 0);
    }

    #[test]
    fn previous_service_wraps_around() {
        let configs = test_configs();
        let mut state = AppState::new(&configs, 100);

        state.prev_service();

        assert_eq!(state.focused, 2);
    }

    #[test]
    fn changing_service_resets_scroll() {
        let configs = test_configs();
        let mut state = AppState::new(&configs, 100);

        state.scroll_up(10);
        assert_eq!(state.scroll, 10);

        state.next_service();

        assert_eq!(state.scroll, 0);
    }

    #[test]
    fn scrolling_down_saturates_at_zero() {
        let configs = test_configs();
        let mut state = AppState::new(&configs, 100);

        state.scroll_up(5);
        state.scroll_down(10);

        assert_eq!(state.scroll, 0);
    }

    #[test]
    fn process_started_sets_running_status() {
        let configs = test_configs();
        let mut state = AppState::new(&configs, 100);

        state.apply(Event::ProcessStarted {
            service: "gateway".into(),
            pid: 1234,
        });

        assert!(matches!(
            state.services[0].status,
            ServiceStatus::Running { pid: 1234 }
        ));
    }

    #[test]
    fn successful_exit_sets_exited_status() {
        let configs = test_configs();
        let mut state = AppState::new(&configs, 100);

        state.apply(Event::ProcessExited {
            service: "gateway".into(),
            code: Some(0),
        });

        assert!(matches!(
            state.services[0].status,
            ServiceStatus::Exited { code: Some(0) }
        ));
    }

    #[test]
    fn nonzero_exit_sets_crashed_status() {
        let configs = test_configs();
        let mut state = AppState::new(&configs, 100);

        state.apply(Event::ProcessExited {
            service: "gateway".into(),
            code: Some(1),
        });

        assert!(matches!(
            state.services[0].status,
            ServiceStatus::Crashed { code: Some(1) }
        ));
    }

    #[test]
    fn log_event_is_added_to_service() {
        let configs = test_configs();
        let mut state = AppState::new(&configs, 100);

        state.apply(Event::Log {
            service: "gateway".into(),
            stream: StreamKind::Stdout,
            line: "Server started".into(),
            at: std::time::Instant::now(),
        });

        assert_eq!(state.services[0].logs.len(), 1);
        assert_eq!(
            state.services[0].logs.iter().next().unwrap().text,
            "Server started"
        );
    }

    #[test]
    fn process_failure_sets_failed_status() {
        let configs = test_configs();
        let mut state = AppState::new(&configs, 100);

        state.apply(Event::ProcessFailed {
            service: "gateway".into(),
            error: "Permission denied".into(),
        });

        assert!(matches!(
            &state.services[0].status,
            ServiceStatus::Failed { error } if error == "Permission denied"
        ));
    }
}
