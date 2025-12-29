use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use super::helpers::{format_size, truncate_string};
use crate::app::{Panel, PanelType};

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
        PanelType::ModeSelection => {
            let title = "Select Mode".to_string();
            let items: Vec<ListItem> = panel
                .list_model
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    let icon = match item.name.as_str() {
                        name if name.starts_with("S3") => "📦",
                        name if name.starts_with("Local") => "📁",
                        _ => "📋",
                    };
                    let display = format!("{} {}", icon, item.name);

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
        PanelType::DriveSelection => {
            let title = "Windows Drives".to_string();
            let items: Vec<ListItem> = panel
                .list_model
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    use crate::models::list::ItemType;

                    let display = if matches!(item.item_type, ItemType::ParentDir) {
                        "📁 ..".to_string()
                    } else {
                        format!("💾 {}", item.name)
                    };

                    let style = if i == panel.selected_index && is_active {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else if matches!(item.item_type, ItemType::ParentDir) {
                        Style::default().fg(Color::LightBlue)
                    } else {
                        Style::default()
                    };
                    ListItem::new(display).style(style)
                })
                .collect();
            (title, items)
        }
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
                        Style::default().fg(Color::LightBlue)
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
                    Style::default().fg(Color::LightBlue)
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
                    Style::default().fg(Color::LightBlue)
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
