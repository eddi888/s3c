use crate::app::{App, InputMode, PanelType, Screen};
use crate::models::list::{ItemData, ItemType, PanelItem};
use crate::operations::{FileOperation, OperationStatus, OperationType};
use anyhow::Result;
use std::path::PathBuf;

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
            (
                PanelType::S3Browser {
                    profile,
                    bucket,
                    prefix: _,
                },
                PanelType::LocalFilesystem { path },
            ) => {
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

                    // Queue-First: Always add to queue as Pending
                    // Queue processing will start the transfer automatically
                    let operation = FileOperation {
                        operation_type: OperationType::Download,
                        source: key.clone(),
                        destination: local_path.display().to_string(),
                        total_size: file_size,
                        transferred: 0,
                        status: OperationStatus::Pending, // Always Pending
                        profile: Some(profile.clone()),
                        bucket: Some(bucket.clone()),
                        dest_profile: None,
                        dest_bucket: None,
                    };

                    // Add to queue - queue processing handles the rest
                    self.file_operation_queue.push(operation);
                }
            }

            // Local → S3: Upload file (prompt for path)
            (
                PanelType::LocalFilesystem { path: _ },
                PanelType::S3Browser {
                    profile: _,
                    bucket: _,
                    prefix,
                },
            ) => {
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
                    profile: source_profile,
                    bucket: source_bucket,
                    prefix: _source_prefix,
                },
                PanelType::S3Browser {
                    profile: dest_profile,
                    bucket: dest_bucket,
                    prefix: dest_prefix,
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
                    let file_size = s3_obj.size.max(0) as u64;

                    // Build destination key
                    let dest_key = if dest_prefix.is_empty() {
                        name.clone()
                    } else {
                        format!("{dest_prefix}{name}")
                    };

                    // Critical: Prevent copying S3 object to itself
                    if source_profile == dest_profile
                        && source_bucket == dest_bucket
                        && source_key == &dest_key
                    {
                        self.show_error("Cannot copy file to itself");
                        return Ok(());
                    }

                    // Queue the S3→S3 copy operation
                    let operation = FileOperation {
                        operation_type: OperationType::S3Copy,
                        source: format!("s3://{source_bucket}/{source_key}"),
                        destination: format!("s3://{dest_bucket}/{dest_key}"),
                        total_size: file_size,
                        transferred: 0,
                        status: OperationStatus::Pending,
                        profile: Some(source_profile.clone()),
                        bucket: Some(source_bucket.clone()),
                        dest_profile: Some(dest_profile.clone()),
                        dest_bucket: Some(dest_bucket.clone()),
                    };

                    self.file_operation_queue.push(operation);
                    // Don't show success message yet - will show when copy completes
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

                    // Critical: Prevent copying file to itself (would truncate to 0 bytes)
                    if source_file_path == dest_file_path {
                        self.show_error("Cannot copy file to itself");
                        return Ok(());
                    }

                    // Create file operation and add to queue
                    self.file_operation_queue.push(FileOperation {
                        operation_type: OperationType::Copy,
                        source: source_file_path.display().to_string(),
                        destination: dest_file_path.display().to_string(),
                        total_size: file_size,
                        transferred: 0,
                        status: OperationStatus::InProgress,
                        profile: None, // Local copy doesn't need S3 credentials
                        bucket: None,
                        dest_profile: None,
                        dest_bucket: None,
                    });
                    let queue_index = self.file_operation_queue.len() - 1;

                    let result = self
                        .copy_local_file_with_progress(&source_file_path, &dest_file_path)
                        .await;

                    match result {
                        Ok(_) => {
                            if let Some(op) = self.file_operation_queue.get_mut(queue_index) {
                                op.status = OperationStatus::Completed;
                            }
                            self.show_success(&format!("Copied: {name}"));
                            crate::app::navigation::reload_local_files(self).await?;
                        }
                        Err(e) => {
                            if let Some(op) = self.file_operation_queue.get_mut(queue_index) {
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

            // Update progress in queue (last item is the current copy operation)
            if let Some(op) = self.file_operation_queue.last_mut() {
                op.transferred = total_copied;
            }
        }

        dest_file.flush().await?;
        Ok(())
    }
}
