use super::dialogs::{
    draw_config_form, draw_delete_confirmation, draw_error_overlay, draw_input_dialog,
    draw_profile_config_form, draw_sort_dialog, draw_success_overlay,
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

use crate::app::{ActivePanel, App, PanelType, Screen};

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

    // Render error/success overlays on top of any screen
    if !app.error_message.is_empty() {
        draw_error_overlay(f, app);
    }
    if !app.success_message.is_empty() {
        draw_success_overlay(f, app);
    }
}

fn draw_dual_panel(f: &mut Frame, app: &mut App) {
    // Check if queue is active to adjust layout
    let has_queue = app.file_operation_queue.is_some();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if has_queue {
            vec![
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(4), // Queue area
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

    let title = Paragraph::new("s3c - S3 Commander")
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

    let menu_items = if app.advanced_mode {
        // Advanced Mode: nur F1 Help, F9 Back, F10 Quit
        vec![
            ("01", "Help"),
            ("02", ""),
            ("03", ""),
            ("04", ""),
            ("05", ""),
            ("06", ""),
            ("07", ""),
            ("08", ""),
            ("09", "Back"),
            ("10", "Quit"),
        ]
    } else {
        // Normal Mode
        match &active_panel.panel_type {
            PanelType::ProfileList => vec![
                ("01", "Help"),
                ("02", "Sort"),
                ("03", "Edit"),
                ("04", "Filter"),
                ("05", ""),
                ("06", ""),
                ("07", ""),
                ("08", ""),
                ("09", "Advanced"),
                ("10", "Quit"),
            ],
            PanelType::BucketList { .. } => vec![
                ("01", "Help"),
                ("02", "Sort"),
                ("03", "Edit Config"),
                ("04", "Filter"),
                ("05", ""),
                ("06", ""),
                ("07", "Add Bucket Conf"),
                ("08", "Del Bucket Conf"),
                ("09", "Advanced"),
                ("10", "Quit"),
            ],
            PanelType::S3Browser { .. } => vec![
                ("01", "Help"),
                ("02", "Sort"),
                ("03", "View"),
                ("04", "Filter"),
                ("05", "Copy File"),
                ("06", "Rename"),
                ("07", "Mkdir"),
                ("08", "Delete"),
                ("09", "Advanced"),
                ("10", "Quit"),
            ],
            PanelType::LocalFilesystem { .. } => vec![
                ("01", "Help"),
                ("02", "Sort"),
                ("03", "View"),
                ("04", "Filter"),
                ("05", "Copy File"),
                ("06", "Rename"),
                ("07", "Mkdir"),
                ("08", "Delete"),
                ("09", "Advanced"),
                ("10", "Quit"),
            ],
        }
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

    let title = Paragraph::new("S3 Commander - Help")
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
        "  F11         - Toggle active panel between AWS-S3-Mode or local Filesystem",
        "",
        "General:",
        "  q/Esc       - Quit application / Close dialog",
    ];

    let help_paragraph = Paragraph::new(help_text.join("\n"))
        .block(Block::default().borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    f.render_widget(help_paragraph, chunks[1]);
}
