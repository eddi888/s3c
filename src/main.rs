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
                            let panel = app.get_active_panel();
                            panel.panel_type = crate::app::PanelType::BucketList { profile };
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
                    Screen::FilePreview => {
                        // Any key exits preview
                        app.go_back();
                    }
                    Screen::Input => {
                        handle_input_mode(app, key.code, key.modifiers).await?;
                    }
                    Screen::Error | Screen::Success | Screen::Help => {
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
        KeyCode::Up => app.navigate_up(),
        KeyCode::Down => app.navigate_down(),
        KeyCode::PageUp => app.navigate_page_up(),
        KeyCode::PageDown => app.navigate_page_down(),
        KeyCode::Tab => app.switch_panel(),
        KeyCode::Enter => app.enter_selected().await?,
        KeyCode::Char('f') | KeyCode::Char('F') => app.toggle_local_filesystem(),
        KeyCode::Char('b') | KeyCode::Char('B') | KeyCode::F(2) => {
            // F2 Create: Bucket config in BucketList
            if matches!(
                app.get_active_panel().panel_type,
                crate::app::PanelType::BucketList { .. }
            ) {
                app.show_config_form();
            }
        }
        KeyCode::Char('p')
        | KeyCode::Char('P')
        | KeyCode::Char('e')
        | KeyCode::Char('E')
        | KeyCode::F(4) => {
            // F4 Edit: Profile config in ProfileList, Bucket config in BucketList
            if let crate::app::PanelType::ProfileList = app.get_active_panel().panel_type {
                app.show_profile_config_form();
            } else if matches!(
                app.get_active_panel().panel_type,
                crate::app::PanelType::BucketList { .. }
            ) {
                app.edit_bucket_config();
            }
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            // Delete bucket - only in BucketList
            if matches!(
                app.get_active_panel().panel_type,
                crate::app::PanelType::BucketList { .. }
            ) {
                app.delete_bucket_config()?;
            }
        }
        KeyCode::Char('v') | KeyCode::Char('V') | KeyCode::F(3) => app.view_file().await?,
        KeyCode::Char('c') | KeyCode::Char('C') | KeyCode::F(5) => {
            app.copy_to_other_panel().await?
        }
        KeyCode::Char('m') | KeyCode::Char('M') | KeyCode::F(7) => {
            // F7 Create Folder - only in S3 Browser
            if matches!(
                app.get_active_panel().panel_type,
                crate::app::PanelType::S3Browser { .. }
            ) {
                app.prompt_create_folder();
            }
        }
        KeyCode::Delete | KeyCode::F(8) => app.delete_file().await?,
        _ => {}
    }
    Ok(())
}

fn handle_profile_config_form_input(app: &mut App, key: KeyCode) -> Result<()> {
    match key {
        KeyCode::Up => {
            if app.profile_form_field > 0 {
                app.profile_form_field -= 1;
            }
        }
        KeyCode::Down => {
            if app.profile_form_field < 3 {
                app.profile_form_field += 1;
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
                // Description
                app.profile_form_description.push(c);
            } else if app.profile_form_field == 1 {
                // Setup Script
                app.profile_form_setup_script.push(c);
            }
        }
        KeyCode::Backspace => {
            if app.profile_form_field == 0 {
                app.profile_form_description.pop();
            } else if app.profile_form_field == 1 {
                app.profile_form_setup_script.pop();
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
            }
        }
        KeyCode::Down => {
            if app.config_form_field < max_field {
                app.config_form_field += 1;
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
                    app.config_form_bucket.push('+');
                } else if app.config_form_field == 1 {
                    app.config_form_description.push('+');
                } else if app.config_form_field == 2 {
                    app.config_form_region.push('+');
                } else if app.config_form_field > 2
                    && app.config_form_field <= app.config_form_roles.len() + 2
                {
                    let role_idx = app.config_form_field - 3;
                    if let Some(role) = app.config_form_roles.get_mut(role_idx) {
                        role.push('+');
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
                    app.config_form_bucket.push('-');
                } else if app.config_form_field == 1 {
                    app.config_form_description.push('-');
                } else if app.config_form_field == 2 {
                    app.config_form_region.push('-');
                } else if app.config_form_field > 2
                    && app.config_form_field <= app.config_form_roles.len() + 2
                {
                    let role_idx = app.config_form_field - 3;
                    if let Some(role) = app.config_form_roles.get_mut(role_idx) {
                        role.push('-');
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
                // Bucket name
                app.config_form_bucket.push(c);
            } else if app.config_form_field == 1 {
                // Description
                app.config_form_description.push(c);
            } else if app.config_form_field == 2 {
                // Region
                app.config_form_region.push(c);
            } else if app.config_form_field > 2
                && app.config_form_field <= app.config_form_roles.len() + 2
            {
                // Role ARN
                let role_idx = app.config_form_field - 3;
                if let Some(role) = app.config_form_roles.get_mut(role_idx) {
                    role.push(c);
                }
            }
        }
        KeyCode::Backspace => {
            if app.config_form_field == 0 {
                app.config_form_bucket.pop();
            } else if app.config_form_field == 1 {
                app.config_form_description.pop();
            } else if app.config_form_field == 2 {
                app.config_form_region.pop();
            } else if app.config_form_field > 2
                && app.config_form_field <= app.config_form_roles.len() + 2
            {
                let role_idx = app.config_form_field - 3;
                if let Some(role) = app.config_form_roles.get_mut(role_idx) {
                    role.pop();
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
                app.input_buffer.push(c);
            }
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Esc => {
            app.input_mode = crate::app::InputMode::None;
            app.go_back();
        }
        _ => {}
    }
    Ok(())
}
