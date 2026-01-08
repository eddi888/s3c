use super::dialogs::{
    draw_config_form, draw_delete_confirmation, draw_error_overlay, draw_info_overlay,
    draw_input_dialog, draw_profile_config_form, draw_sort_dialog, draw_success_overlay,
};
use super::panels::draw_panel;
use super::preview::{draw_file_content_preview, draw_image_preview};
use super::widgets::draw_file_operation_queue;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::app::{ActivePanel, App, Screen};
use crate::menu::{get_advanced_menu, get_menu_items};

pub fn draw(f: &mut Frame, app: &mut App) {
    // Draw the base screen
    match app.screen {
        Screen::DualPanel => draw_dual_panel(f, app),
        Screen::ConfigForm => draw_config_form(f, app),
        Screen::ProfileConfigForm => draw_profile_config_form(f, app),
        Screen::SortDialog => draw_sort_dialog(f, app),
        Screen::DeleteConfirmation => draw_delete_confirmation(f, app),
        Screen::FileContentPreview => draw_file_content_preview(f, app),
        Screen::ImagePreview => draw_image_preview(f, app),
        Screen::Input => draw_input_dialog(f, app),
        Screen::Help => draw_help(f, app),
    }

    // Render error/success/info overlays on top of any screen
    if !app.error_message.is_empty() {
        draw_error_overlay(f, app);
    }
    if !app.success_message.is_empty() {
        draw_success_overlay(f, app);
    }
    if !app.info_message.is_empty() {
        draw_info_overlay(f, app);
    }
}

fn draw_dual_panel(f: &mut Frame, app: &mut App) {
    // Check if queue is active to adjust layout
    let has_queue = !app.file_operation_queue.is_empty();

    // Calculate dynamic queue height: 2 lines per operation + 2 for border
    // Max 5 operations visible (10 lines + 2 border = 12 lines max)
    let queue_height = if has_queue {
        let num_ops = app.file_operation_queue.len().min(5);
        (num_ops * 2 + 2) as u16
    } else {
        0
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if has_queue {
            vec![
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(queue_height), // Dynamic queue area
                Constraint::Length(1),
            ]
        } else {
            vec![
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(1),
            ]
        })
        .split(f.area());

    let title = Paragraph::new(app.app_title.as_str())
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    let panel_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    let is_left_active = app.active_panel == ActivePanel::Left;
    let is_right_active = app.active_panel == ActivePanel::Right;

    draw_panel(
        f,
        &app.config_manager,
        panel_chunks[0],
        &mut app.left_panel,
        is_left_active,
    );
    draw_panel(
        f,
        &app.config_manager,
        panel_chunks[1],
        &mut app.right_panel,
        is_right_active,
    );

    // Draw queue if active
    if has_queue {
        draw_file_operation_queue(f, app, chunks[2]);
    }

    // MC-style function key menu footer with proper colors - context dependent
    let active_panel = match app.active_panel {
        ActivePanel::Left => &app.left_panel,
        ActivePanel::Right => &app.right_panel,
    };

    // Get menu items using centralized system
    let menu_items: Vec<(&str, &str)> = if !app.advanced_menu.is_empty() {
        // Use custom menu for library usage
        app.advanced_menu.to_vec()
    } else if app.advanced_mode {
        // F9 pressed - use default advanced menu
        get_advanced_menu()
            .iter()
            .map(|item| (item.key, item.get_label(app, active_panel)))
            .collect()
    } else {
        // Normal mode - get panel-specific menu
        get_menu_items(app, active_panel)
            .iter()
            .map(|item| (item.key, item.get_label(app, active_panel)))
            .collect()
    };

    let mut spans = Vec::new();
    let width = chunks[2].width as usize;
    let item_count = menu_items.len();

    // Calculate equal width for each menu item
    let item_width = width / item_count;

    for (num, label) in menu_items.iter() {
        // Number: white text on black background (2 chars)
        spans.push(Span::styled(
            *num,
            Style::default().fg(Color::White).bg(Color::Black),
        ));

        // Label: black text on cyan background (left-aligned)
        spans.push(Span::styled(
            *label,
            Style::default().fg(Color::Black).bg(Color::Cyan),
        ));

        // Fill remaining space with cyan background to make equal width
        let used_chars = num.len() + label.len();
        let remaining = item_width.saturating_sub(used_chars);
        if remaining > 0 {
            spans.push(Span::styled(
                " ".repeat(remaining),
                Style::default().bg(Color::Cyan),
            ));
        }
    }

    let line = Line::from(spans);
    let help = Paragraph::new(line)
        .style(Style::default().bg(Color::Black))
        .block(Block::default());
    let footer_chunk = if has_queue { chunks[3] } else { chunks[2] };
    f.render_widget(help, footer_chunk);
}

fn draw_help(f: &mut Frame, _app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(f.area());

    let help_title = format!("{} - Help", _app.app_title);
    let title = Paragraph::new(help_title.as_str())
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    let help_text = vec![
        "Navigation:",
        "  ↑/↓         - Navigate in active panel",
        "  Tab         - Switch between left/right panel",
        "  Enter       - Open selected item (profile/folder/bucket)",
        "  Backspace   - Go to parent directory",
        "",
        "Function Keys:",
        "  F1          - Show this help",
        "  F2          - Sort (Name, Size, Date)",
        "  F3          - Edit (Profile/Bucket) / View file (S3/Filesystem)",
        "  F4          - Filter items",
        "  F5          - Copy from active to inactive panel",
        "  F6          - Rename file/folder (S3/Filesystem)",
        "  F7          - Create bucket config (BucketList) / Create folder (S3/Filesystem)",
        "  F8          - Delete selected item",
        "  F9          - Toggle Advanced Mode",
        "  F10         - Quit application",
        "  F12         - Toggle active panel between AWS-S3-Mode or local Filesystem",
        "",
        "General:",
        "  q/Esc       - Quit application / Close dialog",
    ];

    let help_paragraph = Paragraph::new(help_text.join("\n"))
        .block(Block::default().borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    f.render_widget(help_paragraph, chunks[1]);
}
