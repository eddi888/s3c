use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::Terminal;
use std::io;

use crate::app::{self, App};
use crate::handlers::key_to_message;
use crate::ui;

/// Main application loop following The Elm Architecture (TEA)
pub async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    loop {
        // Check if we need to run a script interactively
        if app.script.needs_terminal {
            if let (Some(script), Some(profile), bucket_opt) = (
                app.script.pending_script.take(),
                app.script.pending_profile.take(),
                app.script.pending_bucket.take(),
            ) {
                app.script.needs_terminal = false;

                // Suspend TUI
                disable_raw_mode()?;
                execute!(io::stdout(), LeaveAlternateScreen)?;

                // Run script interactively
                println!("Running setup script: {script}");

                // Platform-specific shell execution
                #[cfg(target_os = "windows")]
                let status = std::process::Command::new("cmd")
                    .arg("/C")
                    .arg(&script)
                    .status();

                #[cfg(not(target_os = "windows"))]
                let status = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(&script)
                    .status();

                // Resume TUI
                enable_raw_mode()?;
                execute!(io::stdout(), EnterAlternateScreen)?;
                terminal.clear()?;

                // Check result and continue
                match status {
                    Ok(exit_status) if exit_status.success() => {
                        if let Some(Some(bucket)) = bucket_opt {
                            // Continue loading bucket without script
                            app::navigation::load_s3_bucket_no_script(app, profile, bucket).await?;
                        } else {
                            // Just show bucket list for profile
                            let buckets = app.config_manager.get_buckets_for_profile(&profile);
                            let panel = app.get_active_panel();
                            panel.panel_type = app::PanelType::BucketList { profile };
                            panel
                                .list_model
                                .set_items(app::converters::buckets_to_items(buckets));
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

        // Check background transfer task and update queue
        if let Some(task) = &mut app.background_transfer_task {
            // Update queue with current progress
            let current = task
                .progress_counter
                .load(std::sync::atomic::Ordering::Relaxed);
            if let Some(ref mut queue) = app.file_operation_queue {
                queue.transferred = current;
            }

            // Check if task is finished
            if task.task_handle.is_finished() {
                let task = app.background_transfer_task.take().unwrap();
                let mut operation = (*task.operation.lock().await).clone();

                // Ensure transferred shows 100% on completion (for fast small files)
                if operation.status == crate::operations::OperationStatus::Completed {
                    operation.transferred = operation.total_size;
                }

                app.file_operation_queue = Some(operation.clone());

                // Handle completion/error
                match operation.status {
                    crate::operations::OperationStatus::Completed => {
                        match &operation.operation_type {
                            crate::operations::OperationType::Download => {
                                app.show_success(&format!("Downloaded: {}", operation.source));
                                crate::app::navigation::reload_local_files(app).await?;
                            }
                            crate::operations::OperationType::Upload => {
                                app.show_success(&format!("Uploaded: {}", operation.source));
                                crate::app::navigation::reload_s3_browser(app).await?;
                            }
                            _ => {}
                        }
                    }
                    crate::operations::OperationStatus::Failed(ref err) => {
                        app.show_error(&format!("Transfer failed: {err}"));
                    }
                    _ => {}
                }
            }
        }

        terminal.draw(|f| ui::draw(f, app))?;

        if app.should_quit {
            break;
        }

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Ignore key release events (Windows sends both press and release)
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                // Convert key to message using TEA pattern
                if let Some(msg) = key_to_message(app, key.code, key.modifiers) {
                    // Process message and handle cascading messages
                    let mut current_msg = Some(msg);
                    while let Some(message) = current_msg {
                        current_msg = app::update(app, message).await?;
                    }
                }
            }
        }
    }

    Ok(())
}
