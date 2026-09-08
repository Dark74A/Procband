use crate::event::StreamKind;
use std::collections::VecDeque;

/// Represents one line of service output.
#[derive(Debug, Clone)]
pub struct LogLine {
    pub stream: StreamKind,
    pub text: String,
}

/// Stores a limited number of log lines for one service.
///
/// When the buffer is full, the oldest line is removed.
pub struct LogBuffer {
    lines: VecDeque<LogLine>,
    capacity: usize,
}

impl LogBuffer {
    /// Creates an empty log buffer with the given maximum capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            lines: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Adds a log line and removes the oldest line if the buffer is full.
    pub fn push(&mut self, line: LogLine) {
        if self.lines.len() == self.capacity {
            self.lines.pop_front();
        }

        self.lines.push_back(line);
    }

    /// Returns an iterator over the stored lines from oldest to newest.
    ///
    /// The concrete iterator type also supports reverse iteration with `.rev()`.
    pub fn iter(&self) -> std::collections::vec_deque::Iter<'_, LogLine> {
        self.lines.iter()
    }

    /// Returns the number of lines currently stored.
    pub fn len(&self) -> usize {
        self.lines.len()
    }
}

#[test]
fn capacity_one_keeps_only_latest_line() {
    let mut buf = LogBuffer::new(1);

    buf.push(LogLine {
        stream: StreamKind::Stdout,
        text: "first".into(),
    });

    buf.push(LogLine {
        stream: StreamKind::Stdout,
        text: "second".into(),
    });

    let texts: Vec<_> = buf.iter().map(|l| l.text.as_str()).collect();

    assert_eq!(texts, vec!["second"]);
}

#[test]
fn preserves_stdout_and_stderr() {
    let mut buf = LogBuffer::new(3);

    buf.push(LogLine {
        stream: StreamKind::Stdout,
        text: "normal output".into(),
    });

    buf.push(LogLine {
        stream: StreamKind::Stderr,
        text: "error output".into(),
    });

    assert!(matches!(
        buf.iter().nth(0).unwrap().stream,
        StreamKind::Stdout
    ));

    assert!(matches!(
        buf.iter().nth(1).unwrap().stream,
        StreamKind::Stderr
    ));
}
