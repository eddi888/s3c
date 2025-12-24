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
mod handlers;
mod list_model;
mod operations;
mod s3_ops;
mod ui;

use app::{App, Screen};
use handlers::{
    handle_config_form_input, handle_delete_confirmation_input, handle_dual_panel_input,
    handle_file_preview_input, handle_input_mode, handle_profile_config_form_input,
    handle_sort_dialog_input,
};

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
