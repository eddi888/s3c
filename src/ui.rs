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
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
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
    f.render_widget(help, chunks[2]);
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

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.2} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.2} KB", size as f64 / KB as f64)
    } else {
        format!("{size} B")
    }
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

fn draw_delete_confirmation(f: &mut Frame, app: &App) {
    let area = centered_rect(70, 30, f.area());

    let block = Block::default()
        .title("Delete Confirmation")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(inner);

    // Question
    let item_type = if app.delete_confirmation_is_dir {
        "directory"
    } else {
        "file"
    };
    let question = Paragraph::new(format!("Do you really want to delete this {item_type}?"))
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center);
    f.render_widget(question, chunks[0]);

    // Path
    let path_text = Paragraph::new(app.delete_confirmation_path.clone())
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center);
    f.render_widget(path_text, chunks[1]);

    // Buttons
    let delete_style = if app.delete_confirmation_button == 0 {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Red)
    };

    let cancel_style = if app.delete_confirmation_button == 1 {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };

    let buttons = if app.delete_confirmation_button == 0 {
        Paragraph::new("[ DELETE ]  Cancel")
            .style(delete_style)
            .alignment(Alignment::Center)
    } else {
        Paragraph::new("Delete  [ CANCEL ]")
            .style(cancel_style)
            .alignment(Alignment::Center)
    };
    f.render_widget(buttons, chunks[2]);

    // Help
    let help = Paragraph::new("←/→ or Tab: Select | Enter: Confirm | Esc: Cancel")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
    f.render_widget(help, chunks[3]);
}

fn draw_sort_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 50, f.area());

    let block = Block::default()
        .title("Sort Options")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(inner);

    let options = [
        "Name A→Z (alphabetically ascending)",
        "Name Z→A (alphabetically descending)",
        "Size ↑ (small to large)",
        "Size ↓ (large to small)",
        "Date ↑ (oldest to newest)",
        "Date ↓ (newest to oldest)",
    ];

    for (i, option) in options.iter().enumerate() {
        let is_selected = i == app.sort_dialog_selected;
        let prefix = if is_selected { "● " } else { "○ " };
        let style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let text = Paragraph::new(format!("{prefix}{option}")).style(style);
        f.render_widget(text, chunks[i]);
    }

    let help = Paragraph::new("↑/↓: Select | Enter: Apply | Esc: Cancel")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
    f.render_widget(help, chunks[7]);
}

fn draw_profile_config_form(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());

    let title = Paragraph::new(format!("Profile Configuration: {}", app.profile_form_name))
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    let form_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(chunks[1]);

    // Description field
    let desc_style = if app.profile_form_field == 0 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let description = Paragraph::new(format!("Description: {}", app.profile_form_description))
        .style(desc_style)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(description, form_chunks[0]);

    // Render cursor for description field
    if app.profile_form_field == 0 {
        let cursor_x =
            form_chunks[0].x + 1 + "Description: ".len() as u16 + app.profile_form_cursor as u16;
        let cursor_y = form_chunks[0].y + 1;
        f.set_cursor_position((cursor_x, cursor_y));
    }

    // Setup Script field
    let script_style = if app.profile_form_field == 1 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let setup_script = Paragraph::new(format!("Setup Script: {}", app.profile_form_setup_script))
        .style(script_style)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(setup_script, form_chunks[1]);

    // Render cursor for setup script field
    if app.profile_form_field == 1 {
        let cursor_x =
            form_chunks[1].x + 1 + "Setup Script: ".len() as u16 + app.profile_form_cursor as u16;
        let cursor_y = form_chunks[1].y + 1;
        f.set_cursor_position((cursor_x, cursor_y));
    }

    // Buttons
    let save_style = if app.profile_form_field == 2 {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };
    let cancel_style = if app.profile_form_field == 3 {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Red)
    };

    let buttons = if app.profile_form_field == 2 {
        Paragraph::new("[ SAVE ]  Cancel")
            .style(save_style)
            .alignment(Alignment::Center)
    } else if app.profile_form_field == 3 {
        Paragraph::new("Save  [ CANCEL ]")
            .style(cancel_style)
            .alignment(Alignment::Center)
    } else {
        Paragraph::new("Save  Cancel").alignment(Alignment::Center)
    };
    f.render_widget(buttons, form_chunks[2]);

    let help =
        Paragraph::new("↑/↓: Navigate | Type: Edit field | Enter: Save/Cancel | Esc: Cancel")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, chunks[2]);
}

fn draw_config_form(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());

    let title = Paragraph::new(format!(
        "Bucket Configuration for Profile: {}",
        app.config_form_profile
    ))
    .style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    let form_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(3),
        ])
        .split(chunks[1]);

    // Bucket name field
    let bucket_style = if app.config_form_field == 0 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let bucket = Paragraph::new(format!("Bucket Name: {}", app.config_form_bucket))
        .style(bucket_style)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(bucket, form_chunks[0]);

    // Render cursor for bucket name field
    if app.config_form_field == 0 {
        let cursor_x =
            form_chunks[0].x + 1 + "Bucket Name: ".len() as u16 + app.config_form_cursor as u16;
        let cursor_y = form_chunks[0].y + 1;
        f.set_cursor_position((cursor_x, cursor_y));
    }

    // Description field
    let desc_style = if app.config_form_field == 1 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let description = Paragraph::new(format!("Description: {}", app.config_form_description))
        .style(desc_style)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(description, form_chunks[1]);

    // Render cursor for description field
    if app.config_form_field == 1 {
        let cursor_x =
            form_chunks[1].x + 1 + "Description: ".len() as u16 + app.config_form_cursor as u16;
        let cursor_y = form_chunks[1].y + 1;
        f.set_cursor_position((cursor_x, cursor_y));
    }

    // Region field
    let region_style = if app.config_form_field == 2 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let region = Paragraph::new(format!("Region: {}", app.config_form_region))
        .style(region_style)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(region, form_chunks[2]);

    // Render cursor for region field
    if app.config_form_field == 2 {
        let cursor_x =
            form_chunks[2].x + 1 + "Region: ".len() as u16 + app.config_form_cursor as u16;
        let cursor_y = form_chunks[2].y + 1;
        f.set_cursor_position((cursor_x, cursor_y));
    }

    // Role ARNs - render each role separately with proper styling
    let roles_area = form_chunks[3];
    let role_block = Block::default().borders(Borders::ALL).title("Role ARNs");
    f.render_widget(role_block, roles_area);

    let inner_area = roles_area.inner(ratatui::layout::Margin {
        horizontal: 1,
        vertical: 1,
    });
    let role_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            std::iter::repeat_n(Constraint::Length(1), app.config_form_roles.len() + 1)
                .collect::<Vec<_>>(),
        )
        .split(inner_area);

    for (i, role) in app.config_form_roles.iter().enumerate() {
        let role_style = if app.config_form_field == i + 3 {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let role_text = format!("[{}] {}", i + 1, role);
        let role_para = Paragraph::new(role_text).style(role_style);
        if i < role_chunks.len() {
            f.render_widget(role_para, role_chunks[i]);

            // Render cursor for active role field
            if app.config_form_field == i + 3 {
                let cursor_x = role_chunks[i].x
                    + format!("[{}] ", i + 1).len() as u16
                    + app.config_form_cursor as u16;
                let cursor_y = role_chunks[i].y;
                f.set_cursor_position((cursor_x, cursor_y));
            }
        }
    }

    // Help text at bottom of roles area
    if !app.config_form_roles.is_empty() && app.config_form_roles.len() < role_chunks.len() {
        let help_text = Paragraph::new("Press + to add role, - to remove last")
            .style(Style::default().fg(Color::Gray));
        f.render_widget(help_text, role_chunks[app.config_form_roles.len()]);
    }

    // Buttons
    let button_field = app.config_form_roles.len() + 3;
    let save_style = if app.config_form_field == button_field {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };
    let cancel_style = if app.config_form_field == button_field + 1 {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Red)
    };

    let buttons = if app.config_form_field == button_field {
        Paragraph::new("[ SAVE ]  Cancel")
            .style(save_style)
            .alignment(Alignment::Center)
    } else if app.config_form_field == button_field + 1 {
        Paragraph::new("Save  [ CANCEL ]")
            .style(cancel_style)
            .alignment(Alignment::Center)
    } else {
        Paragraph::new("Save  Cancel").alignment(Alignment::Center)
    };
    f.render_widget(buttons, form_chunks[4]);

    let help = Paragraph::new("↑/↓: Navigate | Type: Edit field | +: Add role | -: Remove role | Enter: Save/Cancel | Esc: Cancel")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, chunks[2]);
}

fn draw_input_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(70, 20, f.area());

    // Calculate cursor position for rendering
    let cursor_x = area.x + 1 + app.input_cursor_position as u16;
    let cursor_y = area.y + 1;

    let input = Paragraph::new(app.input_buffer.as_str())
        .style(Style::default().fg(Color::Yellow))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(app.input_prompt.as_str())
                .style(Style::default().bg(Color::Black)),
        );

    f.render_widget(input, area);

    // Set cursor position
    f.set_cursor_position((cursor_x.min(area.x + area.width - 2), cursor_y));
}

fn draw_error_overlay(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 20, f.area());

    // Clear the area first to hide content below
    f.render_widget(ratatui::widgets::Clear, area);

    let error = Paragraph::new(app.error_message.as_str())
        .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("⚠ Error")
                .style(Style::default().bg(Color::Black)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(error, area);
}

fn draw_success_overlay(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 20, f.area());

    // Clear the area first to hide content below
    f.render_widget(ratatui::widgets::Clear, area);

    let success = Paragraph::new(app.success_message.as_str())
        .style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("✓ Success")
                .style(Style::default().bg(Color::Black)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(success, area);
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
