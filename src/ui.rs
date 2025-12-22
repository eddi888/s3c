use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{ActivePanel, App, Panel, PanelType, Screen};

pub fn draw(f: &mut Frame, app: &mut App) {
    match app.screen {
        Screen::DualPanel => draw_dual_panel(f, app),
        Screen::ConfigForm => draw_config_form(f, app),
        Screen::ProfileConfigForm => draw_profile_config_form(f, app),
        Screen::FilePreview => draw_file_preview(f, app),
        Screen::Input => draw_input_dialog(f, app),
        Screen::Error => draw_error_dialog(f, app),
        Screen::Success => draw_success_dialog(f, app),
        Screen::Help => draw_help(f, app),
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

    let menu_items = match &active_panel.panel_type {
        PanelType::ProfileList => vec![
            ("01", "Help(?)"),
            ("02", ""),
            ("03", ""),
            ("04", "Edit(P)"),
            ("05", ""),
            ("06", ""),
            ("07", ""),
            ("08", ""),
            ("09", "Menu"),
            ("10", "Exit(q)"),
        ],
        PanelType::BucketList { .. } => vec![
            ("01", "Help(?)"),
            ("02", "Create(B)"),
            ("03", ""),
            ("04", "Edit(E)"),
            ("05", ""),
            ("06", ""),
            ("07", ""),
            ("08", "Delete(D)"),
            ("09", "Menu"),
            ("10", "Exit(q)"),
        ],
        PanelType::S3Browser { .. } => vec![
            ("01", "Help(?)"),
            ("02", ""),
            ("03", "View(V)"),
            ("04", ""),
            ("05", "Copy(C)"),
            ("06", "Move"),
            ("07", "Mkdir(M)"),
            ("08", "Delete(Del)"),
            ("09", "Menu"),
            ("10", "Exit(q)"),
        ],
        PanelType::LocalFilesystem { .. } => vec![
            ("01", "Help(?)"),
            ("02", ""),
            ("03", "View(V)"),
            ("04", ""),
            ("05", "Copy(C)"),
            ("06", "Move"),
            ("07", ""),
            ("08", "Delete(Del)"),
            ("09", "Menu"),
            ("10", "Exit(q)"),
        ],
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
            let items: Vec<ListItem> = config_manager
                .aws_profiles
                .iter()
                .enumerate()
                .map(|(i, profile)| {
                    let description = config_manager
                        .get_profile_config(profile)
                        .and_then(|p| p.description.as_ref());

                    let display = if let Some(desc) = description {
                        format!("👤 {profile} ({desc})")
                    } else {
                        format!("👤 {profile}")
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
            let mut items: Vec<ListItem> = Vec::new();

            // Add parent directory entry to go back to ProfileList
            let style = if panel.selected_index == 0 && is_active {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Blue)
            };
            items.push(ListItem::new("📁 ..").style(style));

            let buckets = config_manager.get_buckets_for_profile(profile);
            items.extend(buckets.iter().enumerate().map(|(i, bucket_config)| {
                let display = match (
                    &bucket_config.description,
                    bucket_config.role_chain.is_empty(),
                ) {
                    (Some(desc), true) => format!("🪣 {} ({})", bucket_config.name, desc),
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
                };

                let adj_index = i + 1; // Adjust for ".." entry
                let style = if adj_index == panel.selected_index && is_active {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(display).style(style)
            }));
            (title, items)
        }
        PanelType::S3Browser {
            profile: _,
            bucket,
            prefix,
        } => {
            let title = format!("S3: {bucket}/{prefix}");
            let mut items: Vec<ListItem> = Vec::new();

            // Always add parent directory entry (go to parent prefix or back to BucketList)
            let style = if panel.selected_index == 0 && is_active {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Blue)
            };
            items.push(ListItem::new("📁 ..").style(style));

            let offset = if prefix.is_empty() { 0 } else { 1 };

            items.extend(panel.s3_objects.iter().enumerate().map(|(i, obj)| {
                let (name, size_str, modified_str) = if obj.is_prefix {
                    let name = obj.key.trim_end_matches('/');
                    let name = name.strip_prefix(prefix).unwrap_or(name);
                    (format!("📁 {name}"), "<DIR>".to_string(), "".to_string())
                } else {
                    let name = obj.key.strip_prefix(prefix).unwrap_or(&obj.key);
                    let size_str = format_size(obj.size as u64);
                    let modified_str = if let Some(modified) = &obj.last_modified {
                        modified.format("%Y-%m-%d %H:%M").to_string()
                    } else {
                        "".to_string()
                    };
                    (format!("📄 {name}"), size_str, modified_str)
                };

                // MC-style: Name (40 chars) | Size (10 chars) | Modified (16 chars)
                let display = format!(
                    "{:<40} {:>10}  {}",
                    truncate_string(&name, 40),
                    size_str,
                    modified_str
                );

                let adj_index = i + offset;
                let style = if adj_index == panel.selected_index && is_active {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else if obj.is_prefix {
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

            // Add parent directory entry if not in root
            let has_parent = path.parent().is_some();
            if has_parent {
                let style = if panel.selected_index == 0 && is_active {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Blue)
                };
                items.push(ListItem::new("📁 ..").style(style));
            }

            let offset = if has_parent { 1 } else { 0 };

            items.extend(panel.local_files.iter().enumerate().map(|(i, file)| {
                let (name, size_str, modified_str) = if file.is_dir {
                    (
                        format!("📁 {}", file.name),
                        "<DIR>".to_string(),
                        "".to_string(),
                    )
                } else {
                    let size_str = format_size(file.size);
                    let modified_str = if let Some(modified) = file.modified {
                        use std::time::UNIX_EPOCH;
                        if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
                            let secs = duration.as_secs() as i64;
                            if let Some(datetime) = chrono::DateTime::from_timestamp(secs, 0) {
                                datetime.format("%Y-%m-%d %H:%M").to_string()
                            } else {
                                "".to_string()
                            }
                        } else {
                            "".to_string()
                        }
                    } else {
                        "".to_string()
                    };
                    (format!("📄 {}", file.name), size_str, modified_str)
                };

                // MC-style: Name (40 chars) | Size (10 chars) | Modified (16 chars)
                let display = format!(
                    "{:<40} {:>10}  {}",
                    truncate_string(&name, 40),
                    size_str,
                    modified_str
                );

                let adj_index = i + offset;
                let style = if adj_index == panel.selected_index && is_active {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else if file.is_dir {
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

    let list = List::new(visible_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
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
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());

    let title = Paragraph::new(format!("File Preview: {}", app.preview_filename))
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    let content = Paragraph::new(app.preview_content.clone())
        .block(Block::default().borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    f.render_widget(content, chunks[1]);

    let help = Paragraph::new("Press any key to close preview")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, chunks[2]);
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

    // Role ARNs
    let mut role_text = String::from("Role ARNs:\n");
    for (i, role) in app.config_form_roles.iter().enumerate() {
        let prefix = if app.config_form_field == i + 3 {
            "> "
        } else {
            "  "
        };
        let style_marker = if app.config_form_field == i + 3 {
            "*"
        } else {
            ""
        };
        role_text.push_str(&format!("{}[{}]{} {}\n", prefix, i + 1, style_marker, role));
    }
    role_text.push_str("\nPress + to add role, - to remove last");

    let roles = Paragraph::new(role_text)
        .block(Block::default().borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    f.render_widget(roles, form_chunks[3]);

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

    let input = Paragraph::new(app.input_buffer.as_str())
        .style(Style::default().fg(Color::Yellow))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(app.input_prompt.as_str())
                .style(Style::default().bg(Color::Black)),
        );

    f.render_widget(input, area);
}

fn draw_error_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(70, 30, f.area());

    let error = Paragraph::new(app.error_message.as_str())
        .style(Style::default().fg(Color::Red))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Error")
                .style(Style::default().bg(Color::Black)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(error, area);
}

fn draw_success_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(70, 30, f.area());

    let success = Paragraph::new(app.success_message.as_str())
        .style(Style::default().fg(Color::Green))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Success")
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
        "Panel Operations:",
        "  F           - Toggle local filesystem view",
        "  C           - Copy from active to inactive panel",
        "",
        "General:",
        "  q           - Quit application",
        "  ?           - Show this help",
        "  Esc         - Close dialog/Go back",
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
