use crate::app::{App, InputMode, PanelType, Screen};
use crate::models::list::{ItemData, ItemType, PanelItem};
use crate::operations::s3::ProgressCallback;
use crate::operations::{FileOperation, OperationStatus, OperationType};
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

impl App {
    /// Copy file from active panel to inactive panel
    /// Supports: S3→Local, Local→S3, S3→S3, Local→Local
    pub async fn copy_to_other_panel(&mut self) -> Result<()> {
        let (source_panel, dest_panel) = match self.active_panel {
            crate::app::ActivePanel::Left => (&self.left_panel, &self.right_panel),
            crate::app::ActivePanel::Right => (&self.right_panel, &self.left_panel),
        };

        let source_type = source_panel.panel_type.clone();
        let dest_type = dest_panel.panel_type.clone();
        let source_selected = source_panel.selected_index;

        match (&source_type, &dest_type) {
            // S3 → Local: Download file
            (PanelType::S3Browser { prefix: _, .. }, PanelType::LocalFilesystem { path }) => {
                let item = source_panel.list_model.get_item(source_selected);

                if let Some(PanelItem {
                    item_type: ItemType::File,
                    data: ItemData::S3Object(s3_obj),
                    name,
                    ..
                }) = item
                {
                    let filename = name;
                    let local_path = path.join(filename);
                    let key = s3_obj.key.clone();
                    let file_size = s3_obj.size as u64;

                    // Create file operation
                    let operation = Arc::new(Mutex::new(FileOperation {
                        operation_type: OperationType::Download,
                        source: key.clone(),
                        destination: local_path.display().to_string(),
                        total_size: file_size,
                        transferred: 0,
                        status: OperationStatus::InProgress,
                    }));

                    // Store in queue for UI display
                    self.file_operation_queue = Some((*operation.lock().await).clone());

                    // Clone s3_manager to avoid borrow conflicts
                    let s3_manager = source_panel.s3_manager.clone();

                    if let Some(s3_manager) = s3_manager {
                        // Use AtomicU64 for thread-safe progress tracking
                        let transferred_counter = Arc::new(std::sync::atomic::AtomicU64::new(0));
                        let transferred_clone = transferred_counter.clone();

                        let progress_callback: ProgressCallback =
                            Arc::new(Mutex::new(move |transferred: u64| {
                                transferred_clone
                                    .store(transferred, std::sync::atomic::Ordering::Relaxed);
                            }));

                        // Spawn download in background task
                        let key_clone = key.clone();
                        let local_path_clone = local_path.clone();
                        let operation_clone = operation.clone();

                        let task_handle = tokio::spawn(async move {
                            let result = s3_manager
                                .download_file_with_progress(
                                    &key_clone,
                                    &local_path_clone,
                                    Some(progress_callback),
                                )
                                .await;

                            // Update operation status
                            match result {
                                Ok(_) => {
                                    operation_clone.lock().await.status =
                                        OperationStatus::Completed;
                                    Ok(())
                                }
                                Err(e) => {
                                    operation_clone.lock().await.status =
                                        OperationStatus::Failed(format!("{e}"));
                                    Err(anyhow::anyhow!("Download failed: {e}"))
                                }
                            }
                        });

                        // Store background task for UI to poll
                        self.background_transfer_task = Some(crate::app::BackgroundTransferTask {
                            task_handle,
                            progress_counter: transferred_counter,
                            operation,
                        });
                    }
                }
            }

            // Local → S3: Upload file (prompt for path)
            (PanelType::LocalFilesystem { path: _ }, PanelType::S3Browser { prefix, .. }) => {
                let item = source_panel.list_model.get_item(source_selected);

                if let Some(PanelItem {
                    item_type: ItemType::File,
                    data:
                        ItemData::LocalFile {
                            path: file_path, ..
                        },
                    name,
                    ..
                }) = item
                {
                    // Default S3 key
                    let default_s3_key = if prefix.is_empty() {
                        name.clone()
                    } else {
                        format!("{prefix}{name}")
                    };

                    // Prompt user for upload path
                    self.input.mode = InputMode::UploadPath {
                        local_file_path: file_path.clone(),
                        local_file_name: name.clone(),
                    };
                    self.input.buffer = default_s3_key;
                    self.input.cursor_position = self.input.buffer.chars().count();
                    self.input.prompt = "Upload to S3 path:".to_string();
                    self.prev_screen = Some(self.screen.clone());
                    self.screen = Screen::Input;
                }
            }

            // S3 → S3: Copy between buckets
            (
                PanelType::S3Browser {
                    bucket: source_bucket,
                    prefix: _source_prefix,
                    ..
                },
                PanelType::S3Browser {
                    prefix: dest_prefix,
                    ..
                },
            ) => {
                let item = source_panel.list_model.get_item(source_selected);

                if let Some(PanelItem {
                    item_type: ItemType::File,
                    data: ItemData::S3Object(s3_obj),
                    name,
                    ..
                }) = item
                {
                    let source_key = &s3_obj.key;

                    // Build destination key
                    let dest_key = if dest_prefix.is_empty() {
                        name.clone()
                    } else {
                        format!("{dest_prefix}{name}")
                    };

                    if let (Some(source_manager), Some(dest_manager)) =
                        (&source_panel.s3_manager, &dest_panel.s3_manager)
                    {
                        // Try server-side copy first (works for same-bucket and cross-bucket)
                        match dest_manager
                            .copy_from_bucket(source_bucket, source_key, &dest_key)
                            .await
                        {
                            Ok(_) => {
                                self.show_success(&format!("Copied: {name}"));
                                crate::app::navigation::reload_s3_browser(self).await?;
                            }
                            Err(_) => {
                                // Fallback to stream-based copy (cross-account/region)
                                match dest_manager
                                    .stream_copy_from(source_manager, source_key, &dest_key)
                                    .await
                                {
                                    Ok(_) => {
                                        self.show_success(&format!("Copied: {name}"));
                                        crate::app::navigation::reload_s3_browser(self).await?;
                                    }
                                    Err(e) => {
                                        self.show_error(&format!("Copy failed: {e}"));
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Local → Local: Copy file
            (
                PanelType::LocalFilesystem { path: _source_path },
                PanelType::LocalFilesystem { path: dest_path },
            ) => {
                let item = source_panel.list_model.get_item(source_selected);

                if let Some(PanelItem {
                    item_type: ItemType::File,
                    data:
                        ItemData::LocalFile {
                            path: source_file_path,
                            ..
                        },
                    name,
                    size,
                    ..
                }) = item
                {
                    let source_file_path = source_file_path.clone();
                    let name = name.clone();
                    let dest_file_path = dest_path.join(&name);
                    let file_size = size.unwrap_or(0);

                    // Create file operation
                    self.file_operation_queue = Some(FileOperation {
                        operation_type: OperationType::Copy,
                        source: source_file_path.display().to_string(),
                        destination: dest_file_path.display().to_string(),
                        total_size: file_size,
                        transferred: 0,
                        status: OperationStatus::InProgress,
                    });

                    let result = self
                        .copy_local_file_with_progress(&source_file_path, &dest_file_path)
                        .await;

                    match result {
                        Ok(_) => {
                            if let Some(op) = &mut self.file_operation_queue {
                                op.status = OperationStatus::Completed;
                            }
                            self.show_success(&format!("Copied: {name}"));
                            crate::app::navigation::reload_local_files(self).await?;
                        }
                        Err(e) => {
                            if let Some(op) = &mut self.file_operation_queue {
                                op.status = OperationStatus::Failed(format!("{e}"));
                            }
                            let error_msg = format!("{e}");
                            if error_msg.contains("Permission denied")
                                || error_msg.contains("permission denied")
                            {
                                self.show_error(&format!(
                                    "Permission denied: Cannot write to '{}'",
                                    dest_file_path.display()
                                ));
                            } else {
                                self.show_error(&format!("Copy failed: {e}"));
                            }
                        }
                    }
                }
            }

            _ => {
                self.show_error("Unsupported copy operation");
            }
        }

        Ok(())
    }

    /// Helper function to copy local files with progress tracking
    pub(crate) async fn copy_local_file_with_progress(
        &mut self,
        source: &PathBuf,
        dest: &PathBuf,
    ) -> Result<()> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let mut src_file = tokio::fs::File::open(source).await?;
        let mut dest_file = tokio::fs::File::create(dest).await?;

        let mut buffer = vec![0u8; 8 * 1024 * 1024]; // 8MB buffer
        let mut total_copied = 0u64;

        loop {
            let bytes_read = src_file.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }

            dest_file.write_all(&buffer[..bytes_read]).await?;
            total_copied += bytes_read as u64;

            if let Some(op) = &mut self.file_operation_queue {
                op.transferred = total_copied;
            }
        }

        dest_file.flush().await?;
        Ok(())
    }
}
