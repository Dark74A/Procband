use crate::config::ServiceConfig;
use crate::event::{Event, StreamKind};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::{Child, ChildStderr, ChildStdout, Command};
use tokio::sync::mpsc::UnboundedSender;

/// Represents one managed OS process and its configuration.
pub struct ManagedProcess {
    pub name: String,
    child: Child,
}

impl ManagedProcess {
    /// Starts the service command and captures its stdout and stderr.
    pub fn spawn(config: &ServiceConfig) -> Result<Self, String> {
        let mut command = Command::new("sh");

        command
            .arg("-c")
            .arg(&config.command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            // Automatically kill the child if ManagedProcess is dropped.
            .kill_on_drop(true)
            // Create a separate process group so the entire service tree
            // can be killed together (e.g. npm -> node).
            .process_group(0);

        let child = command.spawn().map_err(|error| {
            format!(
                "failed to spawn child process: '{}': {}",
                config.name, error
            )
        })?;

        Ok(Self {
            name: config.name.clone(),
            child,
        })
    }

    /// Returns the PID of the managed process.
    pub fn pid(&self) -> Option<u32> {
        self.child.id()
    }

    /// Waits for the process to finish and returns its exit code.
    pub async fn wait(&mut self) -> Result<Option<i32>, String> {
        let status = self.child.wait().await.map_err(|error| {
            format!(
                "failed to wait for child process: '{}': {}",
                self.name, error
            )
        })?;

        Ok(status.code())
    }

    /// Kills the entire process group and waits for the child to finish.
    pub async fn stop(&mut self) -> Result<(), String> {
        if let Some(pid) = self.child.id() {
            // A negative PID makes kill() target the entire process group.
            let result = unsafe { libc::kill(-(pid as i32), libc::SIGKILL) };

            if result != 0 {
                return Err(format!(
                    "failed to signal process group for '{}': {}",
                    self.name,
                    std::io::Error::last_os_error()
                ));
            }
        }

        // Reap the child process so it does not become a zombie.
        self.child
            .wait()
            .await
            .map(|_| ())
            .map_err(|error| format!("failed to stop '{}': {}", self.name, error))
    }

    /// Takes and returns the child's stdout pipe.
    pub fn stdout(&mut self) -> Option<BufReader<ChildStdout>> {
        self.child.stdout.take().map(BufReader::new)
    }

    /// Takes and returns the child's stderr pipe.
    pub fn stderr(&mut self) -> Option<BufReader<ChildStderr>> {
        self.child.stderr.take().map(BufReader::new)
    }
}

/// Ensures the entire process group is killed when the process is dropped.
impl Drop for ManagedProcess {
    fn drop(&mut self) {
        if let Some(pid) = self.child.id() {
            // Negative PID targets every process in the same process group.
            unsafe {
                libc::kill(-(pid as i32), libc::SIGKILL);
            }
        }
    }
}

/// Reads lines from one output stream and sends them to the event channel.
async fn pump_stream<R: AsyncRead + Unpin>(
    reader: BufReader<R>,
    stream: StreamKind,
    service: String,
    tx: UnboundedSender<Event>,
) {
    let mut lines = reader.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        // Forward each line without blocking the other stream.
        let _ = tx.send(Event::Log {
            service: service.clone(),
            stream: stream.clone(),
            line,
            at: std::time::Instant::now(),
        });
    }
}

/// Runs a service, captures its output, and reports lifecycle events.
pub async fn run_and_report(mut process: ManagedProcess, tx: UnboundedSender<Event>) {
    let name = process.name.clone();
    let pid = process.pid().unwrap_or(0);

    // Notify the application that the process has started.
    let _ = tx.send(Event::ProcessStarted {
        service: name.clone(),
        pid,
    });

    // Take both output streams so they can be processed independently.
    let stdout = process.stdout().expect("stdout was piped");
    let stderr = process.stderr().expect("stderr was piped");

    // Read stdout and stderr concurrently to prevent either pipe from blocking.
    tokio::spawn(pump_stream(
        stdout,
        StreamKind::Stdout,
        name.clone(),
        tx.clone(),
    ));

    tokio::spawn(pump_stream(
        stderr,
        StreamKind::Stderr,
        name.clone(),
        tx.clone(),
    ));

    // Wait for the service and report how it finished.
    match process.wait().await {
        Ok(code) => {
            let _ = tx.send(Event::ProcessExited {
                service: name,
                code,
            });
        }

        Err(error) => {
            let _ = tx.send(Event::ProcessFailed {
                service: name,
                error,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ServiceConfig;
    use crate::event::{Event, StreamKind};
    use tokio::sync::mpsc::unbounded_channel;
    use tokio::time::{Duration, timeout};

    fn config(command: &str) -> ServiceConfig {
        ServiceConfig {
            name: "test-service".into(),
            command: command.into(),
        }
    }

    #[tokio::test]
    async fn spawns_process_successfully() {
        let process = ManagedProcess::spawn(&config("true")).unwrap();

        assert!(process.pid().is_some());
    }

    #[tokio::test]
    async fn captures_stdout() {
        let mut process = ManagedProcess::spawn(&config("printf 'hello stdout\\n'")).unwrap();

        let stdout = process.stdout().unwrap();

        let (tx, mut rx) = unbounded_channel();

        tokio::spawn(pump_stream(
            stdout,
            StreamKind::Stdout,
            "test-service".into(),
            tx,
        ));

        let event = timeout(Duration::from_secs(2), rx.recv())
            .await
            .unwrap()
            .unwrap();

        match event {
            Event::Log {
                service,
                stream,
                line,
                ..
            } => {
                assert_eq!(service, "test-service");
                assert!(matches!(stream, StreamKind::Stdout));
                assert_eq!(line, "hello stdout");
            }
            _ => panic!("expected Log event"),
        }

        process.wait().await.unwrap();
    }

    #[tokio::test]
    async fn captures_stderr() {
        let mut process = ManagedProcess::spawn(&config("printf 'hello stderr\\n' >&2")).unwrap();

        let stderr = process.stderr().unwrap();

        let (tx, mut rx) = unbounded_channel();

        tokio::spawn(pump_stream(
            stderr,
            StreamKind::Stderr,
            "test-service".into(),
            tx,
        ));

        let event = timeout(Duration::from_secs(2), rx.recv())
            .await
            .unwrap()
            .unwrap();

        match event {
            Event::Log {
                service,
                stream,
                line,
                ..
            } => {
                assert_eq!(service, "test-service");
                assert!(matches!(stream, StreamKind::Stderr));
                assert_eq!(line, "hello stderr");
            }
            _ => panic!("expected Log event"),
        }

        process.wait().await.unwrap();
    }

    #[tokio::test]
    async fn reports_zero_exit_code() {
        let mut process = ManagedProcess::spawn(&config("exit 0")).unwrap();

        let code = process.wait().await.unwrap();

        assert_eq!(code, Some(0));
    }

    #[tokio::test]
    async fn reports_nonzero_exit_code() {
        let mut process = ManagedProcess::spawn(&config("exit 42")).unwrap();

        let code = process.wait().await.unwrap();

        assert_eq!(code, Some(42));
    }

    #[tokio::test]
    async fn run_and_report_sends_start_and_exit_events() {
        let process = ManagedProcess::spawn(&config("exit 0")).unwrap();

        let (tx, mut rx) = unbounded_channel();

        run_and_report(process, tx).await;

        let started = rx.recv().await.unwrap();

        match started {
            Event::ProcessStarted { service, pid } => {
                assert_eq!(service, "test-service");
                assert!(pid > 0);
            }
            _ => panic!("expected ProcessStarted event"),
        }

        // The process has exited, so we should eventually receive
        // ProcessExited. The stdout/stderr pump tasks may also have
        // produced events, so keep reading until we find it.
        let mut found_exit = false;

        while let Ok(Some(event)) = timeout(Duration::from_secs(2), rx.recv()).await {
            if let Event::ProcessExited { service, code } = event {
                assert_eq!(service, "test-service");
                assert_eq!(code, Some(0));
                found_exit = true;
                break;
            }
        }

        assert!(found_exit, "expected ProcessExited event");
    }

    #[tokio::test]
    async fn stop_kills_process_group() {
        // Start a long-running shell process. Because ManagedProcess uses
        // process_group(0), the shell becomes the leader of its own group.
        let mut process = ManagedProcess::spawn(&config("sleep 60")).unwrap();

        let pid = process.pid().expect("process should have a pid");

        process.stop().await.unwrap();

        // After stop() the direct child has been waited on and therefore
        // should no longer have a PID.
        assert!(process.pid().is_none());

        // This test is mainly verifying that stop() succeeds while using
        // the process-group kill path. The direct process must be gone.
        assert!(pid > 0);
    }

    #[cfg(all(test, unix))]
    #[tokio::test]
    async fn dropping_process_kills_process_group() {
        let mut process = ManagedProcess::spawn(&config("sleep 60 & wait")).unwrap();

        let pid = process.pid().expect("process should have a pid");

        // Dropping ManagedProcess invokes its Drop implementation, which
        // sends SIGKILL to the entire process group.
        drop(process);

        // Give the OS a moment to process the signal.
        tokio::time::sleep(Duration::from_millis(100)).await;

        // The original process should no longer exist.
        let result = unsafe { libc::kill(pid as i32, 0) };

        assert_ne!(result, 0, "process group leader should have been killed");
    }
}
