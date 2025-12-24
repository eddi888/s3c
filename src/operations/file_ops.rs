use crate::app::{App, InputMode, PanelType, Screen};
use crate::list_model::{ItemData, ItemType, PanelItem};
use crate::operations::{FileOperation, OperationStatus, OperationType};
use crate::s3_ops::ProgressCallback;
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

impl App {
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

                    if let Some(s3_manager) = &source_panel.s3_manager {
                        // Create progress callback that updates the queue
                        let op_clone = operation.clone();
                        let progress_callback: ProgressCallback = Arc::new(Mutex::new(move |transferred: u64| {
                            if let Ok(mut op) = op_clone.try_lock() {
                                op.transferred = transferred;
                            }
                        }));

                        match s3_manager
                            .download_file_with_progress(&key, &local_path, Some(progress_callback))
                            .await
                        {
                            Ok(_) => {
                                operation.lock().await.status = OperationStatus::Completed;
                                self.file_operation_queue = Some((*operation.lock().await).clone());
                                self.show_success(&format!("Downloaded: {filename}"));
                                self.reload_local_files().await?;
                            }
                            Err(e) => {
                                operation.lock().await.status = OperationStatus::Failed(format!("{e}"));
                                self.file_operation_queue = Some((*operation.lock().await).clone());
                                let error_msg = format!("{e}");
                                let path_display = local_path.display();
                                if error_msg.contains("Permission denied")
                                    || error_msg.contains("permission denied")
                                {
                                    self.show_error(&format!(
                                        "Permission denied: Cannot write to '{path_display}'"
                                    ));
                                } else {
                                    self.show_error(&format!("Download failed: {e}"));
                                }
                            }
                        }
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
                    self.input_mode = InputMode::UploadPath {
                        local_file_path: file_path.clone(),
                        local_file_name: name.clone(),
                    };
                    self.input_buffer = default_s3_key;
                    self.input_prompt = "Upload to S3 path:".to_string();
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
                                self.reload_s3_browser().await?;
                            }
                            Err(_) => {
                                // Fallback to stream-based copy (cross-account/region)
                                match dest_manager
                                    .stream_copy_from(source_manager, source_key, &dest_key)
                                    .await
                                {
                                    Ok(_) => {
                                        self.show_success(&format!("Copied: {name}"));
                                        self.reload_s3_browser().await?;
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

                    let result = self.copy_local_file_with_progress(
                        &source_file_path,
                        &dest_file_path,
                    ).await;

                    match result {
                        Ok(_) => {
                            if let Some(op) = &mut self.file_operation_queue {
                                op.status = OperationStatus::Completed;
                            }
                            self.show_success(&format!("Copied: {name}"));
                            self.reload_local_files().await?;
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

    pub async fn rename_file(&mut self) -> Result<()> {
        use crate::app::{local_files_to_items, s3_objects_to_items};
        
        let new_path = self.input_buffer.trim().to_string();
        let old_path = self.rename_original_path.clone();

        if new_path.is_empty() || new_path == old_path {
            return Ok(());
        }

        let panel_type = self.get_active_panel().panel_type.clone();

        match panel_type {
            PanelType::S3Browser { prefix, .. } => {
                // S3: Copy to new key, then delete old key with progress tracking
                let (file_size, has_manager) = {
                    let panel = self.get_active_panel();
                    if let Some(s3_manager) = &panel.s3_manager {
                        let size = match s3_manager.get_object_size(&old_path).await {
                            Ok(s) => s as u64,
                            Err(_) => 0,
                        };
                        (size, true)
                    } else {
                        (0, false)
                    }
                };
                
                if has_manager {
                    let operation = Arc::new(Mutex::new(FileOperation {
                        operation_type: OperationType::Rename,
                        source: old_path.clone(),
                        destination: new_path.clone(),
                        total_size: file_size,
                        transferred: 0,
                        status: OperationStatus::InProgress,
                    }));

                    self.file_operation_queue = Some((*operation.lock().await).clone());

                    let op_clone = operation.clone();
                    let progress_callback: ProgressCallback = Arc::new(Mutex::new(move |transferred: u64| {
                        if let Ok(mut op) = op_clone.try_lock() {
                            op.transferred = transferred;
                        }
                    }));

                    let result = {
                        let panel = self.get_active_panel();
                        if let Some(s3_manager) = &panel.s3_manager {
                            s3_manager.rename_object_with_progress(&old_path, &new_path, Some(progress_callback)).await
                        } else {
                            Err(anyhow::anyhow!("S3 manager not available"))
                        }
                    };

                    match result {
                        Ok(_) => {
                            operation.lock().await.status = OperationStatus::Completed;
                            self.file_operation_queue = Some((*operation.lock().await).clone());
                            
                            let panel = self.get_active_panel();
                            if let Some(s3_manager) = &panel.s3_manager {
                                match s3_manager.list_objects(&prefix).await {
                                    Ok(objects) => {
                                        let panel = self.get_active_panel();
                                        panel.list_model.set_items(s3_objects_to_items(objects));
                                    }
                                    Err(e) => {
                                        self.show_error(&format!("Failed to reload: {e}"));
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            operation.lock().await.status = OperationStatus::Failed(format!("{e}"));
                            self.file_operation_queue = Some((*operation.lock().await).clone());
                            
                            let error_msg = format!("{e}");
                            if error_msg.contains("AccessDenied") {
                                self.show_error("Rename failed: No delete permission. Old file might still exist!");
                            } else {
                                self.show_error(&format!("Rename failed: {e}"));
                            }
                        }
                    }
                }
            }
            PanelType::LocalFilesystem { path } => {
                use crate::app::local_files_to_items;
                
                let old_path_buf = PathBuf::from(&old_path);
                let new_path_buf = PathBuf::from(&new_path);

                let file_size = if let Ok(metadata) = std::fs::metadata(&old_path_buf) {
                    metadata.len()
                } else {
                    0
                };

                match std::fs::rename(&old_path_buf, &new_path_buf) {
                    Ok(_) => {
                        let has_parent = path.parent().is_some();
                        if let Ok(files) = self.read_local_directory(&path) {
                            let panel = self.get_active_panel();
                            panel.list_model.set_items(local_files_to_items(files, has_parent));
                        }
                    }
                    Err(e) => {
                        let error_msg = format!("{e}");
                        if error_msg.contains("cross-device") || error_msg.contains("Invalid cross-device link") {
                            let operation = Arc::new(Mutex::new(FileOperation {
                                operation_type: OperationType::Rename,
                                source: old_path_buf.display().to_string(),
                                destination: new_path_buf.display().to_string(),
                                total_size: file_size,
                                transferred: 0,
                                status: OperationStatus::InProgress,
                            }));

                            self.file_operation_queue = Some((*operation.lock().await).clone());

                            match self.copy_local_file_with_progress(&old_path_buf, &new_path_buf).await {
                                Ok(_) => {
                                    operation.lock().await.transferred = file_size / 2;
                                    self.file_operation_queue = Some((*operation.lock().await).clone());

                                    match std::fs::remove_file(&old_path_buf) {
                                        Ok(_) => {
                                            operation.lock().await.status = OperationStatus::Completed;
                                            operation.lock().await.transferred = file_size;
                                            self.file_operation_queue = Some((*operation.lock().await).clone());

                                            let has_parent = path.parent().is_some();
                                            if let Ok(files) = self.read_local_directory(&path) {
                                                let panel = self.get_active_panel();
                                                panel.list_model.set_items(local_files_to_items(files, has_parent));
                                            }
                                        }
                                        Err(e) => {
                                            operation.lock().await.status = OperationStatus::Failed(format!("{e}"));
                                            self.file_operation_queue = Some((*operation.lock().await).clone());
                                            self.show_error(&format!("Rename failed: Could not delete original file: {e}"));
                                        }
                                    }
                                }
                                Err(e) => {
                                    operation.lock().await.status = OperationStatus::Failed(format!("{e}"));
                                    self.file_operation_queue = Some((*operation.lock().await).clone());
                                    self.show_error(&format!("Rename failed: {e}"));
                                }
                            }
                        } else {
                            self.show_error(&format!("Rename failed: {e}"));
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    pub async fn delete_file(&mut self) -> Result<()> {
        let panel = self.get_active_panel();
        let panel_type = panel.panel_type.clone();
        let selected_index = panel.selected_index;

        match panel_type {
            PanelType::S3Browser { .. } => {
                let item = self.get_active_panel().list_model.get_item(selected_index);

                if let Some(PanelItem {
                    data: ItemData::S3Object(s3_obj),
                    name,
                    ..
                }) = item
                {
                    let key = s3_obj.key.clone();
                    let name = name.clone();

                    self.delete_confirmation_path = key;
                    self.delete_confirmation_name = name;
                    self.delete_confirmation_is_dir = false;
                    self.delete_confirmation_button = 0;
                    self.prev_screen = Some(self.screen.clone());
                    self.screen = Screen::DeleteConfirmation;
                }
            }
            PanelType::LocalFilesystem { .. } => {
                let item = self.get_active_panel().list_model.get_item(selected_index);

                if let Some(PanelItem {
                    item_type,
                    data:
                        ItemData::LocalFile {
                            path: file_path, ..
                        },
                    name,
                    ..
                }) = item
                {
                    let file_path = file_path.clone();
                    let name = name.clone();
                    let is_dir = matches!(item_type, ItemType::Directory);

                    self.delete_confirmation_path = file_path.display().to_string();
                    self.delete_confirmation_name = name;
                    self.delete_confirmation_is_dir = is_dir;
                    self.delete_confirmation_button = 0;
                    self.prev_screen = Some(self.screen.clone());
                    self.screen = Screen::DeleteConfirmation;
                }
            }
            PanelType::BucketList { .. } => {
                self.delete_bucket_config()?;
            }
            _ => {
                self.show_error("Delete only available for files and bucket configs");
            }
        }

        Ok(())
    }

    pub async fn confirm_delete(&mut self) -> Result<()> {
        use crate::app::{local_files_to_items, s3_objects_to_items};
        
        let panel = self.get_active_panel();
        let panel_type = panel.panel_type.clone();

        match panel_type {
            PanelType::S3Browser { prefix, .. } => {
                let key = self.delete_confirmation_path.clone();

                if let Some(s3_manager) = &self.get_active_panel().s3_manager {
                    match s3_manager.delete_object(&key).await {
                        Ok(_) => {
                            match s3_manager.list_objects(&prefix).await {
                                Ok(objects) => {
                                    let panel = self.get_active_panel();
                                    panel.list_model.set_items(s3_objects_to_items(objects));
                                    if panel.selected_index > 0 {
                                        panel.selected_index -= 1;
                                    }
                                    self.show_success(&format!(
                                        "Deleted: {}",
                                        self.delete_confirmation_name
                                    ));
                                }
                                Err(e) => {
                                    self.show_error(&format!("Failed to reload after delete: {e}"));
                                }
                            }
                        }
                        Err(e) => {
                            let error_msg = format!("{e}");
                            if error_msg.contains("AccessDenied") {
                                self.show_error(&format!(
                                    "Delete failed: No permission to delete '{}'",
                                    self.delete_confirmation_name
                                ));
                            } else {
                                self.show_error(&format!("Delete failed: {e}"));
                            }
                        }
                    }
                }
            }
            PanelType::LocalFilesystem { path } => {
                let file_path = PathBuf::from(self.delete_confirmation_path.clone());
                let is_dir = self.delete_confirmation_is_dir;

                let result = if is_dir {
                    std::fs::remove_dir_all(&file_path)
                } else {
                    std::fs::remove_file(&file_path)
                };

                match result {
                    Ok(_) => {
                        let has_parent = path.parent().is_some();
                        if let Ok(files) = self.read_local_directory(&path) {
                            let panel = self.get_active_panel();
                            panel
                                .list_model
                                .set_items(local_files_to_items(files, has_parent));
                            if panel.selected_index > 0 {
                                panel.selected_index -= 1;
                            }
                        }
                    }
                    Err(_) => {
                        // Ignore error - dialog will close anyway
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }
}
