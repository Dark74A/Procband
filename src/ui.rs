use crate::state::{AppState, ServiceStatus};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

/// Renders the complete TUI using the current application state.
pub fn render(frame: &mut Frame, state: &AppState) {
    // Divide the terminal into the service list, log pane, and help footer.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(state.services.len() as u16 + 2),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(frame.area());

    render_service_list(frame, state, chunks[0]);
    render_focused_logs(frame, state, chunks[1]);
    render_help_footer(frame, chunks[2]);
}

/// Renders the list of services and their current statuses.
fn render_service_list(frame: &mut Frame, state: &AppState, area: Rect) {
    let items: Vec<ListItem> = state
        .services
        .iter()
        .enumerate()
        .map(|(i, service)| {
            // Select a symbol, color, and text based on the service status.
            let (marker, color, status_text) = match &service.status {
                ServiceStatus::Starting => ("●", Color::Yellow, "STARTING".to_string()),
                ServiceStatus::Running { pid } => {
                    ("●", Color::Green, format!("RUNNING (pid {pid})"))
                }
                ServiceStatus::Exited { code } => ("○", Color::Gray, format!("EXITED ({code:?})")),
                ServiceStatus::Crashed { code } => ("✖", Color::Red, format!("CRASHED ({code:?})")),
                ServiceStatus::Failed { error } => ("✖", Color::Red, format!("FAILED: {error}")),
                ServiceStatus::Killed => ("■", Color::Magenta, "KILLED (by user)".to_string()),
            };

            let mut style = Style::default().fg(color);

            // Highlight the currently selected service.
            if i == state.focused {
                style = style.add_modifier(Modifier::BOLD | Modifier::REVERSED);
            }

            ListItem::new(Line::from(vec![
                Span::styled(format!("{marker} "), style),
                Span::styled(
                    format!("{:<12} {}", service.config.name, status_text),
                    style,
                ),
            ]))
        })
        .collect();

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("SERVICES"));

    frame.render_widget(list, area);
}

/// Renders the logs of the currently focused service.
fn render_focused_logs(frame: &mut Frame, state: &AppState, area: Rect) {
    let Some(service) = state.services.get(state.focused) else {
        return;
    };

    // Calculate how many log lines can fit inside the pane.
    let visible_height = area.height.saturating_sub(2) as usize;
    let total_lines = service.logs.len();

    // Prevent scrolling beyond the available log history.
    let max_scroll = total_lines.saturating_sub(visible_height);
    let scroll = state.scroll.min(max_scroll);

    // Calculate which section of the log buffer should be displayed.
    let end = total_lines.saturating_sub(scroll);
    let start = end.saturating_sub(visible_height);

    let lines: Vec<Line> = service
        .logs
        .iter()
        .skip(start)
        .take(end - start)
        .map(|log| Line::from(log.text.clone()))
        .collect();

    // Show whether the log view is currently following the latest output.
    let title = if scroll == 0 {
        format!("{} logs", service.config.name)
    } else {
        format!(
            "{} logs (scrolled, {scroll} lines back)",
            service.config.name
        )
    };

    let paragraph =
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(title));

    frame.render_widget(paragraph, area);
}

/// Renders the keyboard shortcut help bar at the bottom of the screen.
fn render_help_footer(frame: &mut Frame, area: Rect) {
    let help = Paragraph::new(Line::from(
        "q: quit   j/k or ↓/↑: switch service   PgUp/PgDn: scroll logs   x: kill   r: restart",
    ))
    .style(Style::default().fg(Color::DarkGray));

    frame.render_widget(help, area);
}
