use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::helpers::format_size;
use crate::app::App;

pub fn draw_file_operation_queue(f: &mut Frame, app: &App, area: Rect) {
    if app.file_operation_queue.is_empty() {
        return;
    }

    // Count running and queued transfers
    let running_count = app
        .file_operation_queue
        .iter()
        .filter(|op| op.status == crate::operations::OperationStatus::InProgress)
        .count();
    let queued_count = app
        .file_operation_queue
        .iter()
        .filter(|op| op.status == crate::operations::OperationStatus::Pending)
        .count();

    // Build title with counts, scroll position, and hints
    let total_ops = app.file_operation_queue.len();
    let scroll_info = if total_ops > 5 {
        format!(
            " [{}/{}]",
            app.selected_queue_index.saturating_add(1).min(total_ops),
            total_ops
        )
    } else {
        String::new()
    };

    let focus_indicator = if app.queue_focused { " [FOCUSED]" } else { "" };

    let title = if running_count > 0 {
        format!(
            "File Operations ({running_count} running, {queued_count} queued){scroll_info}{focus_indicator} - 'q' focus | '↑↓' scroll | 'x' cancel | 'd' delete | 'c' clear"
        )
    } else {
        format!(
            "File Operations ({queued_count} queued){scroll_info}{focus_indicator} - 'q' focus | '↑↓' scroll | 'd' delete | 'c' clear"
        )
    };

    let border_color = if app.queue_focused {
        Color::Cyan // Highlighted border when focused
    } else {
        Color::Yellow
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Calculate constraints for each operation (2 lines each)
    // Show max 5 operations (newest first)
    let num_operations = app.file_operation_queue.len().min(5);
    let mut constraints = vec![];
    for _ in 0..num_operations {
        constraints.push(Constraint::Length(1)); // Info line
        constraints.push(Constraint::Length(1)); // Progress bar
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints(constraints)
        .split(inner);

    // Calculate scroll window: show 5 operations centered around selected_queue_index
    let total_count = app.file_operation_queue.len();
    let selected = app.selected_queue_index.min(total_count.saturating_sub(1));

    // Calculate window start (try to center selected item)
    let window_start = if total_count <= 5 {
        0
    } else {
        // Center the selected item in the window
        selected
            .saturating_sub(2)
            .min(total_count.saturating_sub(5))
    };

    let window_end = (window_start + 5).min(total_count);

    // Draw operations in window (reversed order - newest first)
    for (i, operation) in app
        .file_operation_queue
        .iter()
        .rev()
        .skip(total_count.saturating_sub(window_end))
        .take(window_end - window_start)
        .enumerate()
    {
        let chunk_idx = i * 2;

        // Calculate actual index in reversed queue
        let actual_rev_index = total_count.saturating_sub(window_end) + i;
        let is_selected = (total_count - 1 - actual_rev_index) == selected;

        // Operation info line
        let op_type = match operation.operation_type {
            crate::operations::OperationType::Upload => "↑ Upload",
            crate::operations::OperationType::Download => "↓ Download",
            crate::operations::OperationType::Copy => "→ Copy",
            crate::operations::OperationType::S3Copy => "⇄ S3 Copy",
            crate::operations::OperationType::Rename => "✎ Rename",
        };

        let status_icon = match &operation.status {
            crate::operations::OperationStatus::Pending => "⏸",
            crate::operations::OperationStatus::InProgress => "⟳",
            crate::operations::OperationStatus::Completed => "✓",
            crate::operations::OperationStatus::Cancelled => "⊗",
            crate::operations::OperationStatus::Failed(_) => "✗",
        };

        let status_color = match &operation.status {
            crate::operations::OperationStatus::Pending => Color::Yellow,
            crate::operations::OperationStatus::InProgress => Color::Cyan,
            crate::operations::OperationStatus::Completed => Color::Green,
            crate::operations::OperationStatus::Cancelled => Color::Yellow,
            crate::operations::OperationStatus::Failed(_) => Color::Red,
        };

        // Format file sizes
        let transferred_str = format_size(operation.transferred);
        let total_str = format_size(operation.total_size);
        let percentage = operation.progress_percentage();

        // Add selection indicator if this is the selected item
        let selection_mark = if is_selected { "►" } else { " " };

        // Calculate dynamic path width based on available space
        // Fixed parts: selection_mark (2) + status_icon (2) + op_type (12) + separators (8) + size (20) + percentage (7) = 51
        let available_width = chunks[chunk_idx].width.saturating_sub(51) as usize;
        let path_width = (available_width / 2).max(15); // At least 15 chars per path

        let info_text = format!(
            "{} {} {} │ {} → {} │ {} / {} ({:3}%)",
            selection_mark,
            status_icon,
            op_type,
            truncate_filename(&operation.source, path_width),
            truncate_filename(&operation.destination, path_width),
            transferred_str,
            total_str,
            percentage
        );

        let info_style = if is_selected {
            // High contrast: Black background + bright White text + BOLD
            Style::default()
                .fg(Color::White)
                .bg(Color::Black)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(status_color)
        };

        let info = Paragraph::new(info_text).style(info_style);
        f.render_widget(info, chunks[chunk_idx]);

        // Progress bar
        let progress_bar = draw_progress_bar(percentage, chunks[chunk_idx + 1].width as usize);
        let progress =
            Paragraph::new(progress_bar).style(Style::default().fg(Color::Cyan).bg(Color::Black));
        f.render_widget(progress, chunks[chunk_idx + 1]);
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
