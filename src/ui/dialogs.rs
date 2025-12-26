use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use super::helpers::centered_rect;
use crate::app::App;

pub fn draw_delete_confirmation(f: &mut Frame, app: &App) {
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

    let item_type = if app.delete_confirmation.is_dir {
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

    let path_text = Paragraph::new(app.delete_confirmation.path.clone())
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center);
    f.render_widget(path_text, chunks[1]);

    let delete_style = if app.delete_confirmation.button == 0 {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Red)
    };

    let cancel_style = if app.delete_confirmation.button == 1 {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };

    let buttons = if app.delete_confirmation.button == 0 {
        Paragraph::new("[ DELETE ]  Cancel")
            .style(delete_style)
            .alignment(Alignment::Center)
    } else {
        Paragraph::new("Delete  [ CANCEL ]")
            .style(cancel_style)
            .alignment(Alignment::Center)
    };
    f.render_widget(buttons, chunks[2]);

    let help = Paragraph::new("←/→ or Tab: Select | Enter: Confirm | Esc: Cancel")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
    f.render_widget(help, chunks[3]);
}

pub fn draw_sort_dialog(f: &mut Frame, app: &App) {
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
        let is_selected = i == app.sort_dialog.selected;
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

pub fn draw_profile_config_form(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());

    let title = Paragraph::new(format!("Profile Configuration: {}", app.profile_form.name))
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

    let desc_style = if app.profile_form.field == 0 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let description = Paragraph::new(format!("Description: {}", app.profile_form.description))
        .style(desc_style)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(description, form_chunks[0]);

    if app.profile_form.field == 0 {
        let cursor_x =
            form_chunks[0].x + 1 + "Description: ".len() as u16 + app.profile_form.cursor as u16;
        let cursor_y = form_chunks[0].y + 1;
        f.set_cursor_position((cursor_x, cursor_y));
    }

    let script_style = if app.profile_form.field == 1 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let setup_script = Paragraph::new(format!("Setup Script: {}", app.profile_form.setup_script))
        .style(script_style)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(setup_script, form_chunks[1]);

    if app.profile_form.field == 1 {
        let cursor_x =
            form_chunks[1].x + 1 + "Setup Script: ".len() as u16 + app.profile_form.cursor as u16;
        let cursor_y = form_chunks[1].y + 1;
        f.set_cursor_position((cursor_x, cursor_y));
    }

    let save_style = if app.profile_form.field == 2 {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };
    let cancel_style = if app.profile_form.field == 3 {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Red)
    };

    let buttons = if app.profile_form.field == 2 {
        Paragraph::new("[ SAVE ]  Cancel")
            .style(save_style)
            .alignment(Alignment::Center)
    } else if app.profile_form.field == 3 {
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

pub fn draw_config_form(f: &mut Frame, app: &App) {
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
        app.config_form.profile
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
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(chunks[1]);

    let bucket_style = if app.config_form.field == 0 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let bucket = Paragraph::new(format!("Bucket Name: {}", app.config_form.bucket))
        .style(bucket_style)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(bucket, form_chunks[0]);

    if app.config_form.field == 0 {
        let cursor_x =
            form_chunks[0].x + 1 + "Bucket Name: ".len() as u16 + app.config_form.cursor as u16;
        let cursor_y = form_chunks[0].y + 1;
        f.set_cursor_position((cursor_x, cursor_y));
    }

    let base_prefix_style = if app.config_form.field == 1 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let base_prefix = Paragraph::new(format!("Base Folder: {}", app.config_form.base_prefix))
        .style(base_prefix_style)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(base_prefix, form_chunks[1]);

    if app.config_form.field == 1 {
        let cursor_x =
            form_chunks[1].x + 1 + "Base Folder: ".len() as u16 + app.config_form.cursor as u16;
        let cursor_y = form_chunks[1].y + 1;
        f.set_cursor_position((cursor_x, cursor_y));
    }

    let desc_style = if app.config_form.field == 2 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let description = Paragraph::new(format!("Description: {}", app.config_form.description))
        .style(desc_style)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(description, form_chunks[2]);

    if app.config_form.field == 2 {
        let cursor_x =
            form_chunks[2].x + 1 + "Description: ".len() as u16 + app.config_form.cursor as u16;
        let cursor_y = form_chunks[2].y + 1;
        f.set_cursor_position((cursor_x, cursor_y));
    }

    let region_style = if app.config_form.field == 3 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let region = Paragraph::new(format!("Region: {}", app.config_form.region))
        .style(region_style)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(region, form_chunks[3]);

    if app.config_form.field == 3 {
        let cursor_x =
            form_chunks[3].x + 1 + "Region: ".len() as u16 + app.config_form.cursor as u16;
        let cursor_y = form_chunks[3].y + 1;
        f.set_cursor_position((cursor_x, cursor_y));
    }

    let roles_area = form_chunks[4];
    let role_block = Block::default().borders(Borders::ALL).title("Role ARNs");
    f.render_widget(role_block, roles_area);

    let inner_area = roles_area.inner(Margin {
        horizontal: 1,
        vertical: 1,
    });
    let role_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            std::iter::repeat_n(Constraint::Length(1), app.config_form.roles.len() + 1)
                .collect::<Vec<_>>(),
        )
        .split(inner_area);

    for (i, role) in app.config_form.roles.iter().enumerate() {
        let role_style = if app.config_form.field == i + 4 {
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

            if app.config_form.field == i + 4 {
                let cursor_x = role_chunks[i].x
                    + format!("[{}] ", i + 1).len() as u16
                    + app.config_form.cursor as u16;
                let cursor_y = role_chunks[i].y;
                f.set_cursor_position((cursor_x, cursor_y));
            }
        }
    }

    if !app.config_form.roles.is_empty() && app.config_form.roles.len() < role_chunks.len() {
        let help_text = Paragraph::new("Press + to add role, - to remove last")
            .style(Style::default().fg(Color::Gray));
        f.render_widget(help_text, role_chunks[app.config_form.roles.len()]);
    }

    let button_field = app.config_form.roles.len() + 4;
    let save_style = if app.config_form.field == button_field {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };
    let cancel_style = if app.config_form.field == button_field + 1 {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Red)
    };

    let buttons = if app.config_form.field == button_field {
        Paragraph::new("[ SAVE ]  Cancel")
            .style(save_style)
            .alignment(Alignment::Center)
    } else if app.config_form.field == button_field + 1 {
        Paragraph::new("Save  [ CANCEL ]")
            .style(cancel_style)
            .alignment(Alignment::Center)
    } else {
        Paragraph::new("Save  Cancel").alignment(Alignment::Center)
    };
    f.render_widget(buttons, form_chunks[5]);

    let help = Paragraph::new("↑/↓: Navigate | Type: Edit field | +: Add role | -: Remove role | Enter: Save/Cancel | Esc: Cancel")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, chunks[2]);
}

pub fn draw_input_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(70, 20, f.area());

    let cursor_x = area.x + 1 + app.input.cursor_position as u16;
    let cursor_y = area.y + 1;

    let input = Paragraph::new(app.input.buffer.as_str())
        .style(Style::default().fg(Color::Yellow))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(app.input.prompt.as_str())
                .style(Style::default().bg(Color::Black)),
        );

    f.render_widget(input, area);

    f.set_cursor_position((cursor_x.min(area.x + area.width - 2), cursor_y));
}

pub fn draw_error_overlay(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 20, f.area());

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

pub fn draw_success_overlay(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 20, f.area());

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
