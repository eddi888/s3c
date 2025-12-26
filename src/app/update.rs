use super::handlers;
use super::App;
use crate::message::Message;
use anyhow::Result;

/// Central update function following The Elm Architecture (TEA)
/// Takes current app state and a message, applies the change, and optionally returns another message
pub async fn update(app: &mut App, msg: Message) -> Result<Option<Message>> {
    match msg {
        // ===== Application Control =====
        Message::Quit => {
            app.should_quit = true;
            Ok(None)
        }
        Message::NoOp => Ok(None),

        // ===== Navigation =====
        Message::NavigateUp => {
            handlers::navigate_up(app);
            Ok(None)
        }
        Message::NavigateDown => {
            handlers::navigate_down(app);
            Ok(None)
        }
        Message::NavigatePageUp => {
            handlers::navigate_page_up(app);
            Ok(None)
        }
        Message::NavigatePageDown => {
            handlers::navigate_page_down(app);
            Ok(None)
        }
        Message::NavigateHome => {
            handlers::navigate_home(app);
            Ok(None)
        }
        Message::NavigateEnd => {
            handlers::navigate_end(app);
            Ok(None)
        }
        Message::EnterSelected => {
            crate::app::navigation::enter_selected(app).await?;
            Ok(None)
        }
        Message::GoBack => {
            app.go_back();
            Ok(None)
        }

        // ===== Panel Management =====
        Message::SwitchPanel => {
            app.switch_panel();
            Ok(None)
        }
        Message::ToggleLocalFilesystem => {
            crate::app::navigation::toggle_local_filesystem(app)?;
            Ok(None)
        }
        Message::ToggleAdvancedMode => {
            app.advanced_mode = !app.advanced_mode;
            Ok(None)
        }
        Message::ShowHelp => {
            app.prev_screen = Some(app.screen.clone());
            app.screen = super::Screen::Help;
            Ok(None)
        }

        // ===== File Preview Navigation =====
        Message::FilePreviewUp => {
            handlers::scroll_file_preview_up(app);
            // Auto-load previous content when near top (for Backward mode)
            if let Some(preview) = &app.file_content_preview {
                if preview.preview_mode == crate::models::preview::PreviewMode::Backward
                    && preview.scroll_offset < 50
                    && preview.content_start_offset > 0
                {
                    return Ok(Some(Message::LoadPreviousFileContent));
                }
            }
            Ok(None)
        }
        Message::FilePreviewDown => {
            handlers::scroll_file_preview_down(app);
            // Auto-load more content when near end (for S3 files)
            if let Some(preview) = &app.file_content_preview {
                let line_count = preview.content.lines().count();
                if line_count.saturating_sub(preview.scroll_offset) < 50 {
                    return Ok(Some(Message::LoadMoreFileContent));
                }
            }
            Ok(None)
        }
        Message::FilePreviewPageUp => {
            handlers::scroll_file_preview_page_up(app, 20);
            // Auto-load previous content when near top (for Backward mode)
            if let Some(preview) = &app.file_content_preview {
                if preview.preview_mode == crate::models::preview::PreviewMode::Backward
                    && preview.scroll_offset < 50
                    && preview.content_start_offset > 0
                {
                    return Ok(Some(Message::LoadPreviousFileContent));
                }
            }
            Ok(None)
        }
        Message::FilePreviewPageDown => {
            handlers::scroll_file_preview_page_down(app, 20);
            // Auto-load more content when near end (for S3 files)
            if let Some(preview) = &app.file_content_preview {
                let line_count = preview.content.lines().count();
                if line_count.saturating_sub(preview.scroll_offset) < 50 {
                    return Ok(Some(Message::LoadMoreFileContent));
                }
            }
            Ok(None)
        }
        Message::FilePreviewHome => {
            handlers::scroll_file_preview_home(app).await?;
            Ok(None)
        }
        Message::FilePreviewEnd => {
            handlers::scroll_file_preview_end(app).await?;
            Ok(None)
        }
        Message::LoadMoreFileContent => {
            handlers::load_more_file_content(app).await?;
            Ok(None)
        }
        Message::LoadPreviousFileContent => {
            handlers::load_previous_file_content(app).await?;
            Ok(None)
        }

        // ===== Sort Dialog =====
        Message::ShowSortDialog => {
            handlers::show_sort_dialog(app);
            Ok(None)
        }
        Message::SortDialogUp => {
            if app.sort_dialog.selected > 0 {
                app.sort_dialog.selected -= 1;
            }
            Ok(None)
        }
        Message::SortDialogDown => {
            if app.sort_dialog.selected < 5 {
                app.sort_dialog.selected += 1;
            }
            Ok(None)
        }
        Message::ApplySort => {
            handlers::apply_sort_selection(app);
            Ok(Some(Message::GoBack))
        }

        // ===== Filter =====
        Message::ShowFilterPrompt => {
            handlers::show_filter_prompt(app);
            Ok(None)
        }

        // ===== Config & Profile Forms =====
        Message::ShowConfigForm => {
            handlers::show_config_form(app);
            Ok(None)
        }
        Message::ShowProfileConfigForm => {
            handlers::show_profile_config_form(app);
            Ok(None)
        }
        Message::ShowCreateFolderPrompt => {
            handlers::show_create_folder_prompt(app);
            Ok(None)
        }
        Message::ShowRenamePrompt => {
            handlers::show_rename_prompt(app);
            Ok(None)
        }

        // ===== Delete Confirmation =====
        Message::DeleteConfirmationLeft => {
            if app.delete_confirmation.button > 0 {
                app.delete_confirmation.button -= 1;
            }
            Ok(None)
        }
        Message::DeleteConfirmationRight => {
            if app.delete_confirmation.button < 1 {
                app.delete_confirmation.button += 1;
            }
            Ok(None)
        }
        Message::ConfirmDelete => {
            if app.delete_confirmation.button == 0 {
                crate::operations::confirm_delete(app).await?;
            }
            Ok(Some(Message::GoBack))
        }

        // ===== Messages/Errors =====
        Message::ShowError { message } => {
            app.show_error(&message);
            Ok(None)
        }
        Message::ShowSuccess { message } => {
            app.show_success(&message);
            Ok(None)
        }
        Message::Clear => {
            app.error_message.clear();
            app.success_message.clear();
            Ok(None)
        }

        // ===== Config Form Messages =====
        Message::ConfigFormUp
        | Message::ConfigFormDown
        | Message::ConfigFormLeft
        | Message::ConfigFormRight
        | Message::ConfigFormHome
        | Message::ConfigFormEnd
        | Message::ConfigFormDelete
        | Message::ConfigFormChar { .. }
        | Message::ConfigFormBackspace
        | Message::ConfigFormAddRole
        | Message::ConfigFormRemoveRole => {
            handlers::handle_config_form_message(app, msg)?;
            Ok(None)
        }
        Message::SaveConfigForm => {
            handlers::save_config_form(app)?;
            Ok(Some(Message::GoBack))
        }
        Message::EditBucketConfig => {
            handlers::edit_bucket_config(app);
            Ok(None)
        }
        Message::DeleteBucketConfig => {
            handlers::delete_bucket_config(app)?;
            Ok(None)
        }

        // ===== Profile Form Messages =====
        Message::ProfileFormUp
        | Message::ProfileFormDown
        | Message::ProfileFormLeft
        | Message::ProfileFormRight
        | Message::ProfileFormHome
        | Message::ProfileFormEnd
        | Message::ProfileFormDelete
        | Message::ProfileFormChar { .. }
        | Message::ProfileFormBackspace => {
            handlers::handle_profile_form_message(app, msg)?;
            Ok(None)
        }
        Message::SaveProfileConfig => {
            handlers::save_profile_config(app)?;
            Ok(Some(Message::GoBack))
        }

        // ===== Input Messages =====
        Message::InputChar { c, ctrl } => {
            if !ctrl {
                // Find byte position for character index
                let byte_pos = app
                    .input
                    .buffer
                    .char_indices()
                    .nth(app.input.cursor_position)
                    .map(|(pos, _)| pos)
                    .unwrap_or(app.input.buffer.len());
                app.input.buffer.insert(byte_pos, c);
                app.input.cursor_position += 1;
            }
            Ok(None)
        }
        Message::InputBackspace => {
            if app.input.cursor_position > 0 {
                app.input.cursor_position -= 1;
                // Find byte position for character index
                if let Some((byte_pos, _)) = app
                    .input
                    .buffer
                    .char_indices()
                    .nth(app.input.cursor_position)
                {
                    app.input.buffer.remove(byte_pos);
                }
            }
            Ok(None)
        }
        Message::InputDelete => {
            // Find byte position for character index
            if let Some((byte_pos, _)) = app
                .input
                .buffer
                .char_indices()
                .nth(app.input.cursor_position)
            {
                app.input.buffer.remove(byte_pos);
            }
            Ok(None)
        }
        Message::InputLeft => {
            if app.input.cursor_position > 0 {
                app.input.cursor_position -= 1;
            }
            Ok(None)
        }
        Message::InputRight => {
            let char_count = app.input.buffer.chars().count();
            if app.input.cursor_position < char_count {
                app.input.cursor_position += 1;
            }
            Ok(None)
        }
        Message::InputHome => {
            app.input.cursor_position = 0;
            Ok(None)
        }
        Message::InputEnd => {
            app.input.cursor_position = app.input.buffer.chars().count();
            Ok(None)
        }
        Message::InputSubmit => {
            handlers::handle_input_submit(app).await?;
            Ok(Some(Message::GoBack))
        }
        Message::InputCancel => {
            app.input.mode = super::InputMode::None;
            Ok(Some(Message::GoBack))
        }

        // ===== File Operations =====
        Message::ViewFile => {
            crate::operations::view_file(app).await?;
            Ok(None)
        }
        Message::CreateFolder { name } => {
            crate::operations::create_folder(app, name).await?;
            Ok(None)
        }
        Message::RenameFile { old_path, new_path } => {
            crate::operations::rename_file(app, old_path, new_path).await?;
            Ok(None)
        }
        Message::DeleteFile => {
            handlers::show_delete_confirmation_dialog(app);
            Ok(None)
        }
        Message::CopyToOtherPanel => {
            app.copy_to_other_panel().await?;
            Ok(None)
        }
        Message::CancelTransfer => {
            if let Some(task) = app.background_transfer_task.take() {
                // Abort the background task
                task.task_handle.abort();

                // Update operation status
                let mut operation = task.operation.lock().await;
                operation.status = crate::operations::OperationStatus::Cancelled;
                let operation_type = operation.operation_type.clone();
                app.file_operation_queue = Some(operation.clone());

                app.show_error("Transfer cancelled by user");

                // Refresh panels to show partially transferred files
                match operation_type {
                    crate::operations::OperationType::Download => {
                        crate::app::navigation::reload_local_files(app).await?;
                    }
                    crate::operations::OperationType::Upload => {
                        crate::app::navigation::reload_s3_browser(app).await?;
                    }
                    crate::operations::OperationType::Copy => {
                        crate::app::navigation::reload_local_files(app).await?;
                    }
                    _ => {}
                }
            }
            Ok(None)
        }

        // Placeholder for remaining messages
        _ => Ok(None),
    }
}
