use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use ratatui_image::{picker::Picker, FontSize, StatefulImage};

use crate::app::App;

/// Rendert Bild-Vorschau
pub fn draw_image_preview(f: &mut Frame, app: &App) {
    f.render_widget(Clear, f.area());

    let title = if app.image_preview_loading {
        " Loading Image... ".to_string()
    } else if let Some(ref preview) = app.image_preview {
        let dimensions_str = if let Some((w, h)) = preview.dimensions {
            format!(" [{w}x{h}] ")
        } else {
            String::new()
        };
        format!(
            " {}{} ({}) ",
            preview.filename,
            dimensions_str,
            preview.source_display()
        )
    } else {
        " Image Preview ".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let area = centered_rect(90, 90, f.area());
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(inner);

    // Show loading or image
    if app.image_preview_loading {
        // Loading indicator
        let msg =
            "Loading image...\n\nThis may take a moment for large images.\n\nPress Esc to cancel.";
        let paragraph = Paragraph::new(msg)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(paragraph, chunks[0]);
    } else if let Some(ref preview) = app.image_preview {
        // Try to render image with ratatui-image
        if let Ok(dyn_img) = image::load_from_memory(&preview.image_data) {
            // Use Picker to detect terminal capabilities
            let mut picker =
                Picker::from_termios().unwrap_or_else(|_| Picker::new(FontSize::default()));

            // Create protocol with the image
            let mut protocol = picker.new_resize_protocol(dyn_img);

            // Render the image
            let image_widget = StatefulImage::new(None);
            f.render_stateful_widget(image_widget, chunks[0], &mut protocol);
        } else {
            // Fallback if image loading fails
            let msg = "Failed to load image.\n\nPress Esc to close.";
            let paragraph = Paragraph::new(msg).alignment(Alignment::Center);
            f.render_widget(paragraph, chunks[0]);
        }
    }

    // Info bar
    let info = " Image Preview | Esc Close ";
    let info_paragraph = Paragraph::new(info)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(info_paragraph, chunks[1]);
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
