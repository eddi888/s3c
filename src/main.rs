use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

mod app;
mod config;
mod list_model;
mod s3_ops;
mod ui;

use app::{App, Screen};

#[tokio::main]
async fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new()?;
    let res = run_app(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {err:?}");
    }

    Ok(())
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    loop {
        // Check if we need to run a script interactively
        if app.needs_terminal_for_script {
            if let (Some(script), Some(profile), bucket_opt) = (
                app.pending_script.take(),
                app.pending_script_profile.take(),
                app.pending_script_bucket.take(),
            ) {
                app.needs_terminal_for_script = false;

                // Suspend TUI
                disable_raw_mode()?;
                execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;

                // Run script interactively
                println!("Running setup script: {script}");
                let status = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(&script)
                    .status();

                // Resume TUI
                enable_raw_mode()?;
                execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
                terminal.clear()?;

                // Check result and continue
                match status {
                    Ok(exit_status) if exit_status.success() => {
                        if let Some(Some(bucket)) = bucket_opt {
                            // Continue loading bucket without script
                            app.load_s3_bucket_no_script(profile, bucket).await?;
                        } else {
                            // Just show bucket list for profile
                            let buckets = app.config_manager.get_buckets_for_profile(&profile);
                            let panel = app.get_active_panel();
                            panel.panel_type = crate::app::PanelType::BucketList { profile };
                            panel
                                .list_model
                                .set_items(crate::app::buckets_to_items(buckets));
                            panel.selected_index = 0;
                        }
                    }
                    Ok(_) => {
                        app.show_error("Setup script failed");
                    }
                    Err(e) => {
                        app.show_error(&format!("Failed to execute setup script: {e}"));
                    }
                }
            }
        }

        terminal.draw(|f| ui::draw(f, app))?;

        if app.should_quit {
            break;
        }

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Close error/success overlays with any key
                if !app.error_message.is_empty() {
                    app.error_message.clear();
                    continue;
                }
                if !app.success_message.is_empty() {
                    app.success_message.clear();
                    continue;
                }

                match app.screen {
                    Screen::DualPanel => {
                        handle_dual_panel_input(app, key.code).await?;
                    }
                    Screen::ConfigForm => {
                        handle_config_form_input(app, key.code)?;
                    }
                    Screen::ProfileConfigForm => {
                        handle_profile_config_form_input(app, key.code)?;
                    }
                    Screen::SortDialog => {
                        handle_sort_dialog_input(app, key.code)?;
                    }
                    Screen::DeleteConfirmation => {
                        handle_delete_confirmation_input(app, key.code).await?;
                    }
                    Screen::FilePreview => {
                        handle_file_preview_input(app, key.code).await?;
                    }
                    Screen::Input => {
                        handle_input_mode(app, key.code, key.modifiers).await?;
                    }
                    Screen::Help => {
                        app.go_back();
                    }
                }
            }
        }
    }

    Ok(())
}

async fn handle_dual_panel_input(app: &mut App, key: KeyCode) -> Result<()> {
    match key {
        KeyCode::Char('q') | KeyCode::F(10) => app.should_quit = true,
        KeyCode::Char('?') | KeyCode::F(1) => {
            app.prev_screen = Some(Screen::DualPanel);
            app.screen = Screen::Help;
        }
        KeyCode::F(12) => {
            // F12: Toggle between ProfileList and LocalFilesystem
            app.toggle_local_filesystem();
        }
        KeyCode::Up => app.navigate_up(),
        KeyCode::Down => app.navigate_down(),
        KeyCode::PageUp => app.navigate_page_up(),
        KeyCode::PageDown => app.navigate_page_down(),
        KeyCode::Home => app.navigate_home(),
        KeyCode::End => app.navigate_end(),
        KeyCode::Tab => app.switch_panel(),
        KeyCode::Enter => app.enter_selected().await?,
        KeyCode::F(2) => {
            // F2: Show Sort Dialog
            app.show_sort_dialog();
        }
        KeyCode::F(4) => {
            // F4: Filter
            app.prompt_filter();
        }
        KeyCode::F(7) => {
            // F7: Context-dependent - Create Bucket in BucketList, Mkdir in S3/Filesystem
            match &app.get_active_panel().panel_type {
                crate::app::PanelType::BucketList { .. } => {
                    app.show_config_form();
                }
                crate::app::PanelType::S3Browser { .. }
                | crate::app::PanelType::LocalFilesystem { .. } => {
                    app.prompt_create_folder();
                }
                _ => {}
            }
        }
        KeyCode::F(3) => {
            // F3: Context-dependent - Edit for Profile/Bucket, View for S3/Filesystem
            match &app.get_active_panel().panel_type {
                crate::app::PanelType::ProfileList => {
                    app.show_profile_config_form();
                }
                crate::app::PanelType::BucketList { .. } => {
                    app.edit_bucket_config();
                }
                crate::app::PanelType::S3Browser { .. }
                | crate::app::PanelType::LocalFilesystem { .. } => {
                    app.view_file().await?;
                }
            }
        }
        KeyCode::F(5) => {
            // F5: Copy File
            app.copy_to_other_panel().await?
        }
        KeyCode::F(6) => {
            // F6: Rename - in S3 Browser or Local Filesystem
            if matches!(
                app.get_active_panel().panel_type,
                crate::app::PanelType::S3Browser { .. }
                    | crate::app::PanelType::LocalFilesystem { .. }
            ) {
                app.prompt_rename();
            }
        }
        KeyCode::Delete | KeyCode::F(8) => app.delete_file().await?,
        KeyCode::F(9) => {
            // F9: Toggle Advanced Mode
            app.advanced_mode = !app.advanced_mode;
        }
        _ => {}
    }
    Ok(())
}

fn handle_sort_dialog_input(app: &mut App, key: KeyCode) -> Result<()> {
    match key {
        KeyCode::Up => {
            if app.sort_dialog_selected > 0 {
                app.sort_dialog_selected -= 1;
            }
        }
        KeyCode::Down => {
            if app.sort_dialog_selected < 5 {
                app.sort_dialog_selected += 1;
            }
        }
        KeyCode::Enter => {
            app.apply_sort_selection();
            app.go_back();
        }
        KeyCode::Esc => {
            app.go_back();
        }
        _ => {}
    }
    Ok(())
}

async fn handle_delete_confirmation_input(app: &mut App, key: KeyCode) -> Result<()> {
    match key {
        KeyCode::Left => {
            if app.delete_confirmation_button > 0 {
                app.delete_confirmation_button -= 1;
            }
        }
        KeyCode::Right => {
            if app.delete_confirmation_button < 1 {
                app.delete_confirmation_button += 1;
            }
        }
        KeyCode::Tab => {
            app.delete_confirmation_button = (app.delete_confirmation_button + 1) % 2;
        }
        KeyCode::Enter => {
            if app.delete_confirmation_button == 0 {
                // DELETE button selected
                app.confirm_delete().await?;
            }
            // Both DELETE and CANCEL close the dialog
            app.go_back();
        }
        KeyCode::Esc => {
            app.go_back();
        }
        _ => {}
    }
    Ok(())
}

fn handle_profile_config_form_input(app: &mut App, key: KeyCode) -> Result<()> {
    match key {
        KeyCode::Up => {
            if app.profile_form_field > 0 {
                app.profile_form_field -= 1;
                // Update cursor position for new field
                app.profile_form_cursor = match app.profile_form_field {
                    0 => app.profile_form_description.len(),
                    1 => app.profile_form_setup_script.len(),
                    _ => 0,
                };
            }
        }
        KeyCode::Down => {
            if app.profile_form_field < 3 {
                app.profile_form_field += 1;
                // Update cursor position for new field
                app.profile_form_cursor = match app.profile_form_field {
                    0 => app.profile_form_description.len(),
                    1 => app.profile_form_setup_script.len(),
                    _ => 0,
                };
            }
        }
        KeyCode::Left => {
            if app.profile_form_cursor > 0 {
                app.profile_form_cursor -= 1;
            }
        }
        KeyCode::Right => {
            let max_cursor = match app.profile_form_field {
                0 => app.profile_form_description.len(),
                1 => app.profile_form_setup_script.len(),
                _ => 0,
            };
            if app.profile_form_cursor < max_cursor {
                app.profile_form_cursor += 1;
            }
        }
        KeyCode::Home => {
            app.profile_form_cursor = 0;
        }
        KeyCode::End => {
            app.profile_form_cursor = match app.profile_form_field {
                0 => app.profile_form_description.len(),
                1 => app.profile_form_setup_script.len(),
                _ => 0,
            };
        }
        KeyCode::Delete => {
            if app.profile_form_field == 0
                && app.profile_form_cursor < app.profile_form_description.len()
            {
                app.profile_form_description.remove(app.profile_form_cursor);
            } else if app.profile_form_field == 1
                && app.profile_form_cursor < app.profile_form_setup_script.len()
            {
                app.profile_form_setup_script
                    .remove(app.profile_form_cursor);
            }
        }
        KeyCode::Enter => {
            if app.profile_form_field == 2 {
                // Save
                app.save_profile_config()?;
            } else if app.profile_form_field == 3 {
                // Cancel
                app.go_back();
            }
        }
        KeyCode::Char(c) => {
            if app.profile_form_field == 0 {
                app.profile_form_description
                    .insert(app.profile_form_cursor, c);
                app.profile_form_cursor += 1;
            } else if app.profile_form_field == 1 {
                app.profile_form_setup_script
                    .insert(app.profile_form_cursor, c);
                app.profile_form_cursor += 1;
            }
        }
        KeyCode::Backspace => {
            if app.profile_form_cursor > 0 {
                if app.profile_form_field == 0 {
                    app.profile_form_cursor -= 1;
                    app.profile_form_description.remove(app.profile_form_cursor);
                } else if app.profile_form_field == 1 {
                    app.profile_form_cursor -= 1;
                    app.profile_form_setup_script
                        .remove(app.profile_form_cursor);
                }
            }
        }
        KeyCode::Esc => app.go_back(),
        _ => {}
    }
    Ok(())
}

fn handle_config_form_input(app: &mut App, key: KeyCode) -> Result<()> {
    let max_field = app.config_form_roles.len() + 4; // bucket + description + region + roles + save + cancel

    match key {
        KeyCode::Up => {
            if app.config_form_field > 0 {
                app.config_form_field -= 1;
                // Update cursor position for new field
                app.config_form_cursor = match app.config_form_field {
                    0 => app.config_form_bucket.len(),
                    1 => app.config_form_description.len(),
                    2 => app.config_form_region.len(),
                    _ if app.config_form_field <= app.config_form_roles.len() + 2 => {
                        let role_idx = app.config_form_field - 3;
                        app.config_form_roles
                            .get(role_idx)
                            .map(|r| r.len())
                            .unwrap_or(0)
                    }
                    _ => 0,
                };
            }
        }
        KeyCode::Down => {
            if app.config_form_field < max_field {
                app.config_form_field += 1;
                // Update cursor position for new field
                app.config_form_cursor = match app.config_form_field {
                    0 => app.config_form_bucket.len(),
                    1 => app.config_form_description.len(),
                    2 => app.config_form_region.len(),
                    _ if app.config_form_field <= app.config_form_roles.len() + 2 => {
                        let role_idx = app.config_form_field - 3;
                        app.config_form_roles
                            .get(role_idx)
                            .map(|r| r.len())
                            .unwrap_or(0)
                    }
                    _ => 0,
                };
            }
        }
        KeyCode::Left => {
            if app.config_form_cursor > 0 {
                app.config_form_cursor -= 1;
            }
        }
        KeyCode::Right => {
            let max_cursor = match app.config_form_field {
                0 => app.config_form_bucket.len(),
                1 => app.config_form_description.len(),
                2 => app.config_form_region.len(),
                _ if app.config_form_field <= app.config_form_roles.len() + 2 => {
                    let role_idx = app.config_form_field - 3;
                    app.config_form_roles
                        .get(role_idx)
                        .map(|r| r.len())
                        .unwrap_or(0)
                }
                _ => 0,
            };
            if app.config_form_cursor < max_cursor {
                app.config_form_cursor += 1;
            }
        }
        KeyCode::Home => {
            app.config_form_cursor = 0;
        }
        KeyCode::End => {
            app.config_form_cursor = match app.config_form_field {
                0 => app.config_form_bucket.len(),
                1 => app.config_form_description.len(),
                2 => app.config_form_region.len(),
                _ if app.config_form_field <= app.config_form_roles.len() + 2 => {
                    let role_idx = app.config_form_field - 3;
                    app.config_form_roles
                        .get(role_idx)
                        .map(|r| r.len())
                        .unwrap_or(0)
                }
                _ => 0,
            };
        }
        KeyCode::Delete => {
            if app.config_form_field == 0 && app.config_form_cursor < app.config_form_bucket.len() {
                app.config_form_bucket.remove(app.config_form_cursor);
            } else if app.config_form_field == 1
                && app.config_form_cursor < app.config_form_description.len()
            {
                app.config_form_description.remove(app.config_form_cursor);
            } else if app.config_form_field == 2
                && app.config_form_cursor < app.config_form_region.len()
            {
                app.config_form_region.remove(app.config_form_cursor);
            } else if app.config_form_field > 2
                && app.config_form_field <= app.config_form_roles.len() + 2
            {
                let role_idx = app.config_form_field - 3;
                if let Some(role) = app.config_form_roles.get_mut(role_idx) {
                    if app.config_form_cursor < role.len() {
                        role.remove(app.config_form_cursor);
                    }
                }
            }
        }
        KeyCode::Char('+') => {
            // Only add role when not in a text input field
            let button_field = app.config_form_roles.len() + 3;
            if app.config_form_field >= button_field {
                app.add_role_field();
            } else {
                // In text field, treat as regular character
                if app.config_form_field == 0 {
                    app.config_form_bucket.insert(app.config_form_cursor, '+');
                    app.config_form_cursor += 1;
                } else if app.config_form_field == 1 {
                    app.config_form_description
                        .insert(app.config_form_cursor, '+');
                    app.config_form_cursor += 1;
                } else if app.config_form_field == 2 {
                    app.config_form_region.insert(app.config_form_cursor, '+');
                    app.config_form_cursor += 1;
                } else if app.config_form_field > 2
                    && app.config_form_field <= app.config_form_roles.len() + 2
                {
                    let role_idx = app.config_form_field - 3;
                    if let Some(role) = app.config_form_roles.get_mut(role_idx) {
                        role.insert(app.config_form_cursor, '+');
                        app.config_form_cursor += 1;
                    }
                }
            }
        }
        KeyCode::Char('-') => {
            // Only remove role when not in a text input field
            let button_field = app.config_form_roles.len() + 3;
            if app.config_form_field >= button_field {
                app.remove_last_role();
            } else {
                // In text field, treat as regular character
                if app.config_form_field == 0 {
                    app.config_form_bucket.insert(app.config_form_cursor, '-');
                    app.config_form_cursor += 1;
                } else if app.config_form_field == 1 {
                    app.config_form_description
                        .insert(app.config_form_cursor, '-');
                    app.config_form_cursor += 1;
                } else if app.config_form_field == 2 {
                    app.config_form_region.insert(app.config_form_cursor, '-');
                    app.config_form_cursor += 1;
                } else if app.config_form_field > 2
                    && app.config_form_field <= app.config_form_roles.len() + 2
                {
                    let role_idx = app.config_form_field - 3;
                    if let Some(role) = app.config_form_roles.get_mut(role_idx) {
                        role.insert(app.config_form_cursor, '-');
                        app.config_form_cursor += 1;
                    }
                }
            }
        }
        KeyCode::Enter => {
            let button_field = app.config_form_roles.len() + 3;
            if app.config_form_field == button_field {
                // Save
                app.save_config_form()?;
            } else if app.config_form_field == button_field + 1 {
                // Cancel
                app.go_back();
            }
        }
        KeyCode::Char(c) => {
            if app.config_form_field == 0 {
                app.config_form_bucket.insert(app.config_form_cursor, c);
                app.config_form_cursor += 1;
            } else if app.config_form_field == 1 {
                app.config_form_description
                    .insert(app.config_form_cursor, c);
                app.config_form_cursor += 1;
            } else if app.config_form_field == 2 {
                app.config_form_region.insert(app.config_form_cursor, c);
                app.config_form_cursor += 1;
            } else if app.config_form_field > 2
                && app.config_form_field <= app.config_form_roles.len() + 2
            {
                let role_idx = app.config_form_field - 3;
                if let Some(role) = app.config_form_roles.get_mut(role_idx) {
                    role.insert(app.config_form_cursor, c);
                    app.config_form_cursor += 1;
                }
            }
        }
        KeyCode::Backspace => {
            if app.config_form_cursor > 0 {
                if app.config_form_field == 0 {
                    app.config_form_cursor -= 1;
                    app.config_form_bucket.remove(app.config_form_cursor);
                } else if app.config_form_field == 1 {
                    app.config_form_cursor -= 1;
                    app.config_form_description.remove(app.config_form_cursor);
                } else if app.config_form_field == 2 {
                    app.config_form_cursor -= 1;
                    app.config_form_region.remove(app.config_form_cursor);
                } else if app.config_form_field > 2
                    && app.config_form_field <= app.config_form_roles.len() + 2
                {
                    let role_idx = app.config_form_field - 3;
                    if let Some(role) = app.config_form_roles.get_mut(role_idx) {
                        app.config_form_cursor -= 1;
                        role.remove(app.config_form_cursor);
                    }
                }
            }
        }
        KeyCode::Esc => app.go_back(),
        _ => {}
    }
    Ok(())
}

async fn handle_input_mode(app: &mut App, key: KeyCode, modifiers: KeyModifiers) -> Result<()> {
    match key {
        KeyCode::Enter => {
            match &app.input_mode {
                crate::app::InputMode::CreateFolder => {
                    app.create_folder().await?;
                    app.input_mode = crate::app::InputMode::None;
                    app.go_back();
                }
                crate::app::InputMode::Filter => {
                    app.apply_filter();
                    app.input_mode = crate::app::InputMode::None;
                    app.go_back();
                }
                crate::app::InputMode::Rename => {
                    app.rename_file().await?;
                    app.input_mode = crate::app::InputMode::None;
                    app.go_back();
                }
                crate::app::InputMode::UploadPath {
                    local_file_path,
                    local_file_name,
                } => {
                    let path = local_file_path.clone();
                    let name = local_file_name.clone();
                    let s3_key = app.input_buffer.clone();
                    app.input_mode = crate::app::InputMode::None;
                    app.go_back();

                    // Perform upload with custom path
                    if let Some(s3_manager) = &app.get_inactive_panel().s3_manager {
                        s3_manager.upload_file(&path, &s3_key).await?;
                        app.show_success(&format!("Uploaded: {name}"));

                        // Reload destination panel
                        app.reload_s3_browser().await?;
                    }
                }
                _ => {
                    app.go_back();
                }
            }
        }
        KeyCode::Left => {
            if app.input_cursor_position > 0 {
                app.input_cursor_position -= 1;
            }
        }
        KeyCode::Right => {
            if app.input_cursor_position < app.input_buffer.len() {
                app.input_cursor_position += 1;
            }
        }
        KeyCode::Home => {
            app.input_cursor_position = 0;
        }
        KeyCode::End => {
            app.input_cursor_position = app.input_buffer.len();
        }
        KeyCode::Delete => {
            if app.input_cursor_position < app.input_buffer.len() {
                app.input_buffer.remove(app.input_cursor_position);
            }
        }
        KeyCode::Char(c) => {
            if modifiers.contains(KeyModifiers::CONTROL) {
                match c {
                    'c' | 'C' => {
                        app.input_mode = crate::app::InputMode::None;
                        app.go_back();
                    }
                    _ => {}
                }
            } else {
                app.input_buffer.insert(app.input_cursor_position, c);
                app.input_cursor_position += 1;
            }
        }
        KeyCode::Backspace => {
            if app.input_cursor_position > 0 {
                app.input_cursor_position -= 1;
                app.input_buffer.remove(app.input_cursor_position);
            }
        }
        KeyCode::Esc => {
            app.input_mode = crate::app::InputMode::None;
            app.go_back();
        }
        _ => {}
    }
    Ok(())
}

async fn handle_file_preview_input(app: &mut App, key: KeyCode) -> Result<()> {
    let line_count = app.preview_content.lines().count();

    match key {
        KeyCode::Up => {
            if app.preview_scroll_offset > 0 {
                app.preview_scroll_offset -= 1;
            }
        }
        KeyCode::Down => {
            if app.preview_scroll_offset < line_count.saturating_sub(1) {
                app.preview_scroll_offset += 1;
            }
            // Check if we need to load more content (within 50 lines of end)
            if app.preview_is_s3 && line_count.saturating_sub(app.preview_scroll_offset) < 50 {
                app.load_more_preview_content().await?;
            }
        }
        KeyCode::PageUp => {
            app.preview_scroll_offset = app.preview_scroll_offset.saturating_sub(20);
        }
        KeyCode::PageDown => {
            app.preview_scroll_offset =
                (app.preview_scroll_offset + 20).min(line_count.saturating_sub(1));
            // Check if we need to load more content (within 50 lines of end)
            if app.preview_is_s3 && line_count.saturating_sub(app.preview_scroll_offset) < 50 {
                app.load_more_preview_content().await?;
            }
        }
        KeyCode::Home => {
            app.preview_scroll_offset = 0;
        }
        KeyCode::End => {
            app.preview_scroll_offset = line_count.saturating_sub(1);
            // Load more content when jumping to end
            if app.preview_is_s3 {
                app.load_more_preview_content().await?;
            }
        }
        KeyCode::Esc | KeyCode::Char('q') => {
            app.preview_scroll_offset = 0;
            app.go_back();
        }
        _ => {}
    }
    Ok(())
}
