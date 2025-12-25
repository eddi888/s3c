use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use super::helpers::{format_size, truncate_string};
use crate::app::{App, Panel, PanelType};

pub fn draw_panel(
    f: &mut Frame,
    config_manager: &crate::models::config::ConfigManager,
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
                    use crate::models::list::ItemData;

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
                    use crate::models::list::{ItemData, ItemType};

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
                use crate::models::list::ItemType;

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
                use crate::models::list::ItemType;

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

    let visible_height = area.height.saturating_sub(2) as usize;
    panel.visible_height = visible_height;
    let total_items = items.len();

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

    let visible_items: Vec<ListItem> = items
        .into_iter()
        .skip(scroll_offset)
        .take(visible_height)
        .collect();

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

pub fn draw_file_preview(f: &mut Frame, app: &App) {
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

    let line_count = app.preview.content.lines().count();

    let file_size_str = format_size(app.preview.file_size as u64);
    let loaded_size_str = format_size(app.preview.byte_offset as u64);

    let title_text = if app.preview.is_s3 && app.preview.byte_offset < app.preview.file_size {
        format!(
            "File Preview: {} ({} of {} loaded) - Line {}/{}+",
            app.preview.filename,
            loaded_size_str,
            file_size_str,
            app.preview.scroll_offset + 1,
            line_count
        )
    } else {
        format!(
            "File Preview: {} ({}) - Line {}/{}",
            app.preview.filename,
            file_size_str,
            app.preview.scroll_offset + 1,
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

    f.render_widget(ratatui::widgets::Clear, chunks[1]);

    let content_text = app.preview.content.replace('\t', "    ");

    let content = Paragraph::new(content_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Black)),
        )
        .style(Style::default().bg(Color::Black))
        .wrap(Wrap { trim: false })
        .scroll((app.preview.scroll_offset as u16, 0));
    f.render_widget(content, chunks[1]);

    let help = Paragraph::new("↑/↓: Scroll | PgUp/PgDn: Page | Home/End: Jump | ESC/q: Close")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, chunks[2]);
}
