use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::helpers::format_size;
use crate::app::App;

pub fn draw_file_operation_queue(f: &mut Frame, app: &App, area: Rect) {
    if let Some(ref operation) = app.file_operation_queue {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("File Operations Queue")
            .border_style(Style::default().fg(Color::Yellow));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(0)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(inner);

        // Operation info line
        let op_type = match operation.operation_type {
            crate::operations::OperationType::Upload => "↑ Upload",
            crate::operations::OperationType::Download => "↓ Download",
            crate::operations::OperationType::Copy => "→ Copy",
            crate::operations::OperationType::Rename => "✎ Rename",
        };

        let status_icon = match &operation.status {
            crate::operations::OperationStatus::Pending => "⏸",
            crate::operations::OperationStatus::InProgress => "⟳",
            crate::operations::OperationStatus::Completed => "✓",
            crate::operations::OperationStatus::Failed(_) => "✗",
        };

        let status_color = match &operation.status {
            crate::operations::OperationStatus::Pending => Color::Yellow,
            crate::operations::OperationStatus::InProgress => Color::Cyan,
            crate::operations::OperationStatus::Completed => Color::Green,
            crate::operations::OperationStatus::Failed(_) => Color::Red,
        };

        // Format file sizes
        let transferred_str = format_size(operation.transferred);
        let total_str = format_size(operation.total_size);
        let percentage = operation.progress_percentage();

        let info_text = format!(
            "{} {} │ {} → {} │ {} / {} ({:3}%)",
            status_icon,
            op_type,
            truncate_filename(&operation.source, 25),
            truncate_filename(&operation.destination, 25),
            transferred_str,
            total_str,
            percentage
        );

        let info = Paragraph::new(info_text).style(Style::default().fg(status_color));
        f.render_widget(info, chunks[0]);

        // Progress bar
        let progress_bar = draw_progress_bar(percentage, chunks[1].width as usize);
        let progress =
            Paragraph::new(progress_bar).style(Style::default().fg(Color::Cyan).bg(Color::Black));
        f.render_widget(progress, chunks[1]);
    }
}

fn draw_progress_bar(percentage: u16, width: usize) -> String {
    let filled_width = ((width as f64 * percentage as f64) / 100.0) as usize;
    let empty_width = width.saturating_sub(filled_width);

    let filled = "█".repeat(filled_width);
    let empty = "░".repeat(empty_width);

    format!("{filled}{empty}")
}

fn truncate_filename(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        path.to_string()
    } else {
        // Show start and end of filename
        let start_len = max_len / 2 - 1;
        let end_len = max_len / 2 - 2;
        format!("{}...{}", &path[..start_len], &path[path.len() - end_len..])
    }
}
