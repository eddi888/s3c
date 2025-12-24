use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};

use crate::app::{App, Screen};

pub async fn handle_dual_panel_input(app: &mut App, key: KeyCode) -> Result<()> {
    match key {
        KeyCode::Char('q') | KeyCode::F(10) => app.should_quit = true,
        KeyCode::Char('?') | KeyCode::F(1) => {
            app.prev_screen = Some(Screen::DualPanel);
            app.screen = Screen::Help;
        }
        KeyCode::F(12) => {
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
            app.show_sort_dialog();
        }
        KeyCode::F(4) => {
            app.prompt_filter();
        }
        KeyCode::F(7) => {
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
            app.copy_to_other_panel().await?
        }
        KeyCode::F(6) => {
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
            app.advanced_mode = !app.advanced_mode;
        }
        _ => {}
    }
    Ok(())
}

pub fn handle_sort_dialog_input(app: &mut App, key: KeyCode) -> Result<()> {
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

pub async fn handle_delete_confirmation_input(app: &mut App, key: KeyCode) -> Result<()> {
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
                app.confirm_delete().await?;
            }
            app.go_back();
        }
        KeyCode::Esc => {
            app.go_back();
        }
        _ => {}
    }
    Ok(())
}

pub fn handle_profile_config_form_input(app: &mut App, key: KeyCode) -> Result<()> {
    match key {
        KeyCode::Up => {
            if app.profile_form_field > 0 {
                app.profile_form_field -= 1;
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
                app.save_profile_config()?;
            } else if app.profile_form_field == 3 {
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

pub fn handle_config_form_input(app: &mut App, key: KeyCode) -> Result<()> {
    let max_field = app.config_form_roles.len() + 4;

    match key {
        KeyCode::Up => {
            if app.config_form_field > 0 {
                app.config_form_field -= 1;
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
            let button_field = app.config_form_roles.len() + 3;
            if app.config_form_field >= button_field {
                app.add_role_field();
            } else {
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
            let button_field = app.config_form_roles.len() + 3;
            if app.config_form_field >= button_field {
                app.remove_last_role();
            } else {
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
                app.save_config_form()?;
            } else if app.config_form_field == button_field + 1 {
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

pub async fn handle_input_mode(app: &mut App, key: KeyCode, modifiers: KeyModifiers) -> Result<()> {
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

                    let file_size = if let Ok(metadata) = std::fs::metadata(&path) {
                        metadata.len()
                    } else {
                        0
                    };

                    let operation = std::sync::Arc::new(tokio::sync::Mutex::new(crate::operations::FileOperation {
                        operation_type: crate::operations::OperationType::Upload,
                        source: path.display().to_string(),
                        destination: s3_key.clone(),
                        total_size: file_size,
                        transferred: 0,
                        status: crate::operations::OperationStatus::InProgress,
                    }));

                    app.file_operation_queue = Some((*operation.lock().await).clone());

                    if let Some(s3_manager) = &app.get_inactive_panel().s3_manager {
                        let op_clone = operation.clone();
                        let progress_callback: crate::s3_ops::ProgressCallback = 
                            std::sync::Arc::new(tokio::sync::Mutex::new(move |transferred: u64| {
                                if let Ok(mut op) = op_clone.try_lock() {
                                    op.transferred = transferred;
                                }
                            }));

                        match s3_manager.upload_file_with_progress(&path, &s3_key, Some(progress_callback)).await {
                            Ok(_) => {
                                operation.lock().await.status = crate::operations::OperationStatus::Completed;
                                app.file_operation_queue = Some((*operation.lock().await).clone());
                                app.show_success(&format!("Uploaded: {name}"));
                                app.reload_s3_browser().await?;
                            }
                            Err(e) => {
                                operation.lock().await.status = crate::operations::OperationStatus::Failed(format!("{e}"));
                                app.file_operation_queue = Some((*operation.lock().await).clone());
                                app.show_error(&format!("Upload failed: {e}"));
                            }
                        }
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

pub async fn handle_file_preview_input(app: &mut App, key: KeyCode) -> Result<()> {
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
            if app.preview_is_s3 && line_count.saturating_sub(app.preview_scroll_offset) < 50 {
                app.load_more_preview_content().await?;
            }
        }
        KeyCode::Home => {
            app.preview_scroll_offset = 0;
        }
        KeyCode::End => {
            app.preview_scroll_offset = line_count.saturating_sub(1);
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
