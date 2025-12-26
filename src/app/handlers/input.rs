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
            let _name = local_file_name.clone();
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

            // Clone s3_manager to avoid borrow conflicts
            let s3_manager = app.get_inactive_panel().s3_manager.clone();

            if let Some(s3_manager) = s3_manager {
                // Use AtomicU64 for thread-safe progress tracking
                let transferred_counter = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
                let transferred_clone = transferred_counter.clone();

                let progress_callback: crate::operations::s3::ProgressCallback =
                    std::sync::Arc::new(tokio::sync::Mutex::new(move |transferred: u64| {
                        transferred_clone.store(transferred, std::sync::atomic::Ordering::Relaxed);
                    }));

                // Spawn upload in background task
                let path_clone = path.clone();
                let s3_key_clone = s3_key.clone();
                let operation_clone = operation.clone();

                let task_handle = tokio::spawn(async move {
                    let result = s3_manager
                        .upload_file_with_progress(
                            &path_clone,
                            &s3_key_clone,
                            Some(progress_callback),
                        )
                        .await;

                    // Update operation status
                    match result {
                        Ok(_) => {
                            operation_clone.lock().await.status =
                                crate::operations::OperationStatus::Completed;
                            Ok(())
                        }
                        Err(e) => {
                            operation_clone.lock().await.status =
                                crate::operations::OperationStatus::Failed(format!("{e}"));
                            Err(anyhow::anyhow!("Upload failed: {e}"))
                        }
                    }
                });

                // Store background task for UI to poll
                app.background_transfer_task = Some(crate::app::BackgroundTransferTask {
                    task_handle,
                    progress_counter: transferred_counter,
                    operation,
                });
            }
        }
        _ => {}
    }
    Ok(())
}
