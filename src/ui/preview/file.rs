use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::App;
use crate::ui::helpers::format_size;

/// Rendert Text-Datei-Vorschau
pub fn draw_file_content_preview(f: &mut Frame, app: &mut App) {
    if let Some(ref mut preview) = app.file_content_preview {
        f.render_widget(Clear, f.area());

        let title = format!(" {} ({}) ", preview.filename, preview.source_display());

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let area = centered_rect(90, 90, f.area());

        let inner = block.inner(area);
        f.render_widget(block, area);

        // Split for content and info bar
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(inner);

        // Update viewport width for visual line calculation
        preview.viewport_width = chunks[0].width;

        // Get visual lines (pre-wrapped based on viewport width)
        let visual_lines = preview.get_visual_lines();

        // Render content with scroll (using visual lines, no need for Wrap)
        let visible_lines: Vec<_> = visual_lines
            .iter()
            .skip(preview.scroll_offset)
            .take(chunks[0].height as usize)
            .map(|s| Line::from(s.as_str()))
            .collect();

        let paragraph = Paragraph::new(visible_lines);

        f.render_widget(paragraph, chunks[0]);

        // Info bar - show different format based on preview mode
        let line_info = match preview.preview_mode {
            crate::models::preview::PreviewMode::Forward => {
                // Normal mode: Line X / Total
                format!(
                    "Line {}/{}",
                    preview.scroll_offset + 1,
                    preview.content.lines().count()
                )
            }
            crate::models::preview::PreviewMode::Backward => {
                // Backward mode: Line -X / LAST
                let total_lines = preview.content.lines().count();
                let lines_from_end = total_lines.saturating_sub(preview.scroll_offset);
                format!("Line -{lines_from_end} / LAST")
            }
        };

        // Mode display
        let mode_str = match preview.preview_mode {
            crate::models::preview::PreviewMode::Forward => "FWD",
            crate::models::preview::PreviewMode::Backward => "BWD",
        };

        // Chunk status: Check if file is fully loaded in memory
        let is_fully_loaded =
            preview.content_start_offset == 0 && preview.byte_offset >= preview.file_size;
        let chunk_status = if is_fully_loaded { "FULL" } else { "CHUNK" };

        let info = format!(
            " {} | {} | {} | Chunks: {} | Size: {} | ↑↓ Scroll | Home/End/Esc ",
            line_info,
            mode_str,
            chunk_status,
            preview.chunk_load_count,
            format_size(preview.file_size as u64)
        );

        let info_paragraph = Paragraph::new(info)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(info_paragraph, chunks[1]);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
