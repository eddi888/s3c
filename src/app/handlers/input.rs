use super::dialogs::apply_filter;
use crate::app::{App, InputMode};
use anyhow::Result;

pub async fn handle_input_submit(app: &mut App) -> Result<()> {
    match &app.input.mode {
        InputMode::CreateFolder => {
            let folder_name = app.input.buffer.trim().to_string();
            app.input.mode = InputMode::None;
            crate::operations::create_folder(app, folder_name).await?;
        }
        InputMode::Filter => {
            let pattern = app.input.buffer.trim().to_string();
            app.input.mode = InputMode::None;
            apply_filter(app, pattern);
        }
        InputMode::Rename => {
            let old_path = app.input.rename_original_path.clone();
            let new_path = app.input.buffer.clone();
            app.input.mode = InputMode::None;
            crate::operations::rename_file(app, old_path, new_path).await?;
        }
        InputMode::UploadPath {
            local_file_path,
            local_file_name,
        } => {
            let path = local_file_path.clone();
            let name = local_file_name.clone();
            let s3_key = app.input.buffer.clone();
            app.input.mode = InputMode::None;

            let file_size = if let Ok(metadata) = std::fs::metadata(&path) {
                metadata.len()
            } else {
                0
            };

            let operation =
                std::sync::Arc::new(tokio::sync::Mutex::new(crate::operations::FileOperation {
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
                let progress_callback: crate::operations::s3::ProgressCallback =
                    std::sync::Arc::new(tokio::sync::Mutex::new(move |transferred: u64| {
                        if let Ok(mut op) = op_clone.try_lock() {
                            op.transferred = transferred;
                        }
                    }));

                match s3_manager
                    .upload_file_with_progress(&path, &s3_key, Some(progress_callback))
                    .await
                {
                    Ok(_) => {
                        operation.lock().await.status =
                            crate::operations::OperationStatus::Completed;
                        app.file_operation_queue = Some((*operation.lock().await).clone());
                        app.show_success(&format!("Uploaded: {name}"));
                        crate::app::navigation::reload_s3_browser(app).await?;
                    }
                    Err(e) => {
                        operation.lock().await.status =
                            crate::operations::OperationStatus::Failed(format!("{e}"));
                        app.file_operation_queue = Some((*operation.lock().await).clone());
                        app.show_error(&format!("Upload failed: {e}"));
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}
