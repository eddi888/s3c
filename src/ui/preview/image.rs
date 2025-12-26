use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::models::preview::ImageRenderMode;

/// Rendert Bild-Vorschau
pub fn draw_image_preview(f: &mut Frame, app: &App) {
    if let Some(ref preview) = app.image_preview {
        f.render_widget(Clear, f.area());

        let dimensions_str = if let Some((w, h)) = preview.dimensions {
            format!(" [{w}x{h}] ")
        } else {
            String::new()
        };

        let title = format!(
            " {}{} ({}) ",
            preview.filename,
            dimensions_str,
            preview.source_display()
        );

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let area = centered_rect(90, 90, f.area());
        let inner = block.inner(area);
        f.render_widget(block, area);

        match preview.render_mode {
            ImageRenderMode::Ascii => {
                if let Some(ref ascii) = preview.ascii_data {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Min(0), Constraint::Length(1)])
                        .split(inner);

                    let paragraph = Paragraph::new(ascii.as_str()).wrap(Wrap { trim: false });
                    f.render_widget(paragraph, chunks[0]);

                    // Info bar
                    let info = " ASCII Preview | Esc Close ";
                    let info_paragraph = Paragraph::new(info)
                        .style(Style::default().fg(Color::DarkGray))
                        .alignment(Alignment::Center);
                    f.render_widget(info_paragraph, chunks[1]);
                }
            }
            ImageRenderMode::NotSupported => {
                let msg = "Image preview not supported in this terminal.\n\nPress Esc to close.";
                let paragraph = Paragraph::new(msg)
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: false });
                f.render_widget(paragraph, inner);
            }
        }
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
