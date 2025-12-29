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

            // Get profile and bucket from inactive panel (S3 side)
            let (profile, bucket) = match &app.get_inactive_panel().panel_type {
                crate::app::PanelType::S3Browser {
                    profile, bucket, ..
                } => (Some(profile.clone()), Some(bucket.clone())),
                _ => (None, None),
            };

            let file_size = if let Ok(metadata) = std::fs::metadata(&path) {
                metadata.len()
            } else {
                0
            };

            // Queue-First: Always add to queue as Pending
            // Queue processing will start the transfer automatically
            let operation = crate::operations::FileOperation {
                operation_type: crate::operations::OperationType::Upload,
                source: path.display().to_string(),
                destination: s3_key.clone(),
                total_size: file_size,
                transferred: 0,
                status: crate::operations::OperationStatus::Pending, // Always Pending
                profile,
                bucket,
                dest_profile: None,
                dest_bucket: None,
            };

            // Add to queue - queue processing handles the rest
            app.file_operation_queue.push(operation);
        }
        _ => {}
    }
    Ok(())
}
