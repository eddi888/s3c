use super::dialogs::{
    draw_config_form, draw_delete_confirmation, draw_error_overlay, draw_input_dialog,
    draw_profile_config_form, draw_sort_dialog, draw_success_overlay,
};
use super::helpers::{centered_rect, format_size, truncate_string};
use super::widgets::draw_file_operation_queue;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{ActivePanel, App, Panel, PanelType, Screen};

pub fn draw(f: &mut Frame, app: &mut App) {
    // Draw the base screen
    match app.screen {
        Screen::DualPanel => draw_dual_panel(f, app),
        Screen::ConfigForm => draw_config_form(f, app),
        Screen::ProfileConfigForm => draw_profile_config_form(f, app),
        Screen::SortDialog => draw_sort_dialog(f, app),
        Screen::DeleteConfirmation => draw_delete_confirmation(f, app),
        Screen::FilePreview => draw_file_preview(f, app),
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
                Constraint::Length(4),  // Queue area
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

fn draw_panel(
    f: &mut Frame,
    config_manager: &crate::config::ConfigManager,
    area: Rect,
    panel: &mut Panel,
    is_active: bool,
) {
    let border_style = if is_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Gray)
    };

    let (title, items) = match &panel.panel_type {
        PanelType::ProfileList => {
            let title = "AWS Profiles".to_string();
            let items: Vec<ListItem> = panel
                .list_model
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    use crate::list_model::ItemData;

                    let profile_name = match &item.data {
                        ItemData::Profile(profile) => profile,
                        _ => &item.name,
                    };

                    let description = config_manager
                        .get_profile_config(profile_name)
                        .and_then(|p| p.description.as_ref());

                    let display = if let Some(desc) = description {
                        format!("👤 {profile_name} ({desc})")
                    } else {
                        format!("👤 {profile_name}")
                    };

                    let style = if i == panel.selected_index && is_active {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    ListItem::new(display).style(style)
                })
                .collect();
            (title, items)
        }
        PanelType::BucketList { profile } => {
            let title = format!("Buckets for: {profile}");
            let items: Vec<ListItem> = panel
                .list_model
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    use crate::list_model::{ItemData, ItemType};

                    let display = match &item.item_type {
                        ItemType::ParentDir => "📁 ..".to_string(),
                        _ => {
                            if let ItemData::Bucket(bucket_config) = &item.data {
                                match (
                                    &bucket_config.description,
                                    bucket_config.role_chain.is_empty(),
                                ) {
                                    (Some(desc), true) => {
                                        format!("🪣 {} ({})", bucket_config.name, desc)
                                    }
                                    (Some(desc), false) => format!(
                                        "🪣 {} ({}) [{}]",
                                        bucket_config.name,
                                        desc,
                                        bucket_config.role_chain.len()
                                    ),
                                    (None, true) => format!("🪣 {}", bucket_config.name),
                                    (None, false) => format!(
                                        "🪣 {} [{}]",
                                        bucket_config.name,
                                        bucket_config.role_chain.len()
                                    ),
                                }
                            } else {
                                item.name.clone()
                            }
                        }
                    };

                    let style = if i == panel.selected_index && is_active {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else if matches!(item.item_type, ItemType::ParentDir) {
                        Style::default().fg(Color::Blue)
                    } else {
                        Style::default()
                    };
                    ListItem::new(display).style(style)
                })
                .collect();
            (title, items)
        }
        PanelType::S3Browser {
            profile: _,
            bucket,
            prefix,
        } => {
            let title = format!("S3: {bucket}/{prefix}");
            let mut items: Vec<ListItem> = Vec::new();

            items.extend(panel.list_model.iter().enumerate().map(|(i, item)| {
                use crate::list_model::ItemType;

                let (icon_name, size_str, modified_str) = match &item.item_type {
                    ItemType::ParentDir => ("📁 ..".to_string(), "".to_string(), "".to_string()),
                    ItemType::Directory => {
                        let display_name = item.name.strip_prefix(prefix).unwrap_or(&item.name);
                        (
                            format!("📁 {display_name}"),
                            "<DIR>".to_string(),
                            "".to_string(),
                        )
                    }
                    ItemType::File => {
                        let display_name = item.name.strip_prefix(prefix).unwrap_or(&item.name);
                        let size_str = item.size.map(format_size).unwrap_or_default();
                        let modified_str = item
                            .modified
                            .map(|m| m.format("%Y-%m-%d %H:%M").to_string())
                            .unwrap_or_default();
                        (format!("📄 {display_name}"), size_str, modified_str)
                    }
                };

                // MC-style: Name (40 chars) | Size (10 chars) | Modified (16 chars)
                let display = format!(
                    "{:<40} {:>10}  {}",
                    truncate_string(&icon_name, 40),
                    size_str,
                    modified_str
                );

                let style = if i == panel.selected_index && is_active {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else if matches!(item.item_type, ItemType::Directory | ItemType::ParentDir) {
                    Style::default().fg(Color::Blue)
                } else {
                    Style::default()
                };
                ListItem::new(display).style(style)
            }));
            (title, items)
        }
        PanelType::LocalFilesystem { path } => {
            let title = format!("Local: {}", path.display());
            let mut items: Vec<ListItem> = Vec::new();

            items.extend(panel.list_model.iter().enumerate().map(|(i, item)| {
                use crate::list_model::ItemType;

                let (icon_name, size_str, modified_str) = match &item.item_type {
                    ItemType::ParentDir => ("📁 ..".to_string(), "".to_string(), "".to_string()),
                    ItemType::Directory => (
                        format!("📁 {}", item.name),
                        "<DIR>".to_string(),
                        "".to_string(),
                    ),
                    ItemType::File => {
                        let size_str = item.size.map(format_size).unwrap_or_default();
                        let modified_str = item
                            .modified
                            .map(|m| m.format("%Y-%m-%d %H:%M").to_string())
                            .unwrap_or_default();
                        (format!("📄 {}", item.name), size_str, modified_str)
                    }
                };

                // MC-style: Name (40 chars) | Size (10 chars) | Modified (16 chars)
                let display = format!(
                    "{:<40} {:>10}  {}",
                    truncate_string(&icon_name, 40),
                    size_str,
                    modified_str
                );

                let style = if i == panel.selected_index && is_active {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else if matches!(item.item_type, ItemType::Directory | ItemType::ParentDir) {
                    Style::default().fg(Color::Blue)
                } else {
                    Style::default()
                };
                ListItem::new(display).style(style)
            }));
            (title, items)
        }
    };

    // Calculate scrolling based on visible area
    let visible_height = area.height.saturating_sub(2) as usize; // Subtract borders
    panel.visible_height = visible_height; // Update panel's visible_height
    let total_items = items.len();

    // Calculate scroll offset to keep selected item visible
    let scroll_offset = if total_items > visible_height {
        let selected = panel.selected_index;
        if selected < panel.scroll_offset {
            selected
        } else if selected >= panel.scroll_offset + visible_height {
            selected.saturating_sub(visible_height - 1)
        } else {
            panel.scroll_offset
        }
    } else {
        0
    };

    // Slice items to only show visible portion
    let visible_items: Vec<ListItem> = items
        .into_iter()
        .skip(scroll_offset)
        .take(visible_height)
        .collect();

    // Add filter display to title if filter is active
    let title_with_filter = if let Some(filter_text) = panel.list_model.get_filter_display() {
        format!("{title} {filter_text} ")
    } else {
        title
    };

    let list = List::new(visible_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title_with_filter)
            .border_style(border_style),
    );

    f.render_widget(list, area);
}

fn draw_file_preview(f: &mut Frame, app: &App) {
    // Clear the entire screen to hide content below
    f.render_widget(ratatui::widgets::Clear, f.area());

    let base_block = Block::default()
        .borders(Borders::NONE)
        .style(Style::default().bg(Color::Black));
    f.render_widget(base_block, f.area());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());

    let line_count = app.preview_content.lines().count();

    // Format file size
    let file_size_str = format_size(app.preview_file_size as u64);
    let loaded_size_str = format_size(app.preview_byte_offset as u64);

    let title_text = if app.preview_is_s3 && app.preview_byte_offset < app.preview_file_size {
        format!(
            "File Preview: {} ({} of {} loaded) - Line {}/{}+",
            app.preview_filename,
            loaded_size_str,
            file_size_str,
            app.preview_scroll_offset + 1,
            line_count
        )
    } else {
        format!(
            "File Preview: {} ({}) - Line {}/{}",
            app.preview_filename,
            file_size_str,
            app.preview_scroll_offset + 1,
            line_count
        )
    };

    let title = Paragraph::new(title_text)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    // Clear content area before rendering to prevent scroll artifacts
    f.render_widget(ratatui::widgets::Clear, chunks[1]);

    // Convert tabs to spaces to ensure black background rendering
    let content_text = app.preview_content.replace('\t', "    ");

    let content = Paragraph::new(content_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Black)),
        )
        .style(Style::default().bg(Color::Black))
        .wrap(Wrap { trim: false })
        .scroll((app.preview_scroll_offset as u16, 0));
    f.render_widget(content, chunks[1]);

    let help = Paragraph::new("↑/↓: Scroll | PgUp/PgDn: Page | Home/End: Jump | ESC/q: Close")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, chunks[2]);
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

