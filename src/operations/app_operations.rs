use crate::app::{App, PanelType};
use crate::models::list::{ItemData, ItemType, PanelItem};
use anyhow::Result;

/// Extract S3 key from path by removing s3://bucket/ prefix
fn extract_s3_key(path: &str) -> String {
    if let Some(stripped) = path.strip_prefix("s3://") {
        if let Some(slash_pos) = stripped.find('/') {
            stripped[slash_pos + 1..].to_string()
        } else {
            path.to_string()
        }
    } else {
        path.to_string()
    }
}

pub async fn confirm_delete(app: &mut App) -> Result<()> {
    let path = app.delete_confirmation.path.clone();
    let is_dir = app.delete_confirmation.is_dir;

    let panel_type = app.get_active_panel().panel_type.clone();

    match panel_type {
        PanelType::BucketList { profile: _ } => {
            // For bucket deletion, call the dedicated handler
            crate::app::handlers::forms::delete_bucket_config(app)?;
        }
        PanelType::S3Browser { prefix, .. } => {
            if let Some(s3_manager) = &app.get_active_panel().s3_manager {
                // Extract S3 key from path (remove s3://bucket/ prefix)
                let key = extract_s3_key(&path);

                s3_manager.delete_object(&key).await?;

                // Reload
                let objects = s3_manager.list_objects(&prefix).await?;
                let panel = app.get_active_panel();
                panel
                    .list_model
                    .set_items(crate::app::converters::s3_objects_to_items(objects));

                app.show_success(&format!("Deleted: {}", app.delete_confirmation.name));
            }
        }
        PanelType::LocalFilesystem { path: dir_path } => {
            if is_dir {
                std::fs::remove_dir_all(&path)?;
            } else {
                std::fs::remove_file(&path)?;
            }

            // Reload
            let has_parent = dir_path.parent().is_some();
            if let Ok(files) = crate::app::navigation::read_local_directory(&dir_path) {
                let panel = app.get_active_panel();
                panel
                    .list_model
                    .set_items(crate::app::converters::local_files_to_items(
                        files, has_parent,
                    ));
            }

            app.show_success(&format!("Deleted: {}", app.delete_confirmation.name));
        }
        _ => {}
    }

    Ok(())
}

pub async fn view_file(app: &mut App) -> Result<()> {
    use crate::app::handlers::preview::{
        is_image_file, show_file_content_preview, show_image_preview,
    };
    use crate::models::preview::PreviewSource;

    let panel = app.get_active_panel();
    let panel_type = panel.panel_type.clone();
    let selected_index = panel.selected_index;

    match panel_type {
        PanelType::S3Browser { bucket, .. } => {
            let item = app.get_active_panel().list_model.get_item(selected_index);

            if let Some(PanelItem {
                item_type: ItemType::File,
                data: ItemData::S3Object(s3_obj),
                name,
                ..
            }) = item
            {
                let key = s3_obj.key.clone();
                let filename = name.clone();
                let bucket = bucket.clone();

                // Check if image
                if is_image_file(&filename) {
                    let source = PreviewSource::S3Object { key, bucket };
                    show_image_preview(app, source).await?;
                } else {
                    // Text file - use new preview system
                    if let Some(s3_manager) = &app.get_active_panel().s3_manager {
                        match crate::operations::preview::file_loader::load_s3_file_content(
                            &key, &bucket, s3_manager,
                        )
                        .await
                        {
                            Ok(preview) => {
                                app.file_content_preview = Some(preview);
                                app.prev_screen = Some(app.screen.clone());
                                app.screen = crate::app::Screen::FileContentPreview;
                            }
                            Err(e) => {
                                app.show_error(&format!("Cannot preview file: {e}"));
                            }
                        }
                    }
                }
            }
        }
        PanelType::LocalFilesystem { .. } => {
            let item = app.get_active_panel().list_model.get_item(selected_index);

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
                let path = file_path.clone();
                let filename = name.clone();
                let path_str = path.display().to_string();

                // Check if image
                if is_image_file(&filename) {
                    let source = PreviewSource::LocalFile { path: path_str };
                    show_image_preview(app, source).await?;
                } else {
                    // Text file
                    let source = PreviewSource::LocalFile { path: path_str };
                    show_file_content_preview(app, source).await?;
                }
            }
        }
        _ => {
            app.show_error("Preview only available for files");
        }
    }

    Ok(())
}

pub async fn create_folder(app: &mut App, folder_name: String) -> Result<()> {
    if folder_name.trim().is_empty() {
        app.show_error("Folder name cannot be empty");
        return Ok(());
    }

    let panel_type = app.get_active_panel().panel_type.clone();

    match panel_type {
        PanelType::S3Browser { prefix, .. } => {
            let has_s3_manager = app.get_active_panel().s3_manager.is_some();

            if has_s3_manager {
                let folder_key = if prefix.is_empty() {
                    format!("{folder_name}/")
                } else {
                    format!("{prefix}{folder_name}/")
                };

                let panel = app.get_active_panel();
                if let Some(s3_manager) = &panel.s3_manager {
                    s3_manager.upload_empty_folder(&folder_key).await?;

                    let objects = s3_manager.list_objects(&prefix).await?;

                    let panel = app.get_active_panel();
                    panel
                        .list_model
                        .set_items(crate::app::converters::s3_objects_to_items(objects));
                }
            }
        }
        PanelType::LocalFilesystem { path } => {
            let new_folder_path = path.join(&folder_name);

            std::fs::create_dir(&new_folder_path)?;

            let has_parent = path.parent().is_some();
            if let Ok(files) = crate::app::navigation::read_local_directory(&path) {
                let panel = app.get_active_panel();
                panel
                    .list_model
                    .set_items(crate::app::converters::local_files_to_items(
                        files, has_parent,
                    ));
            }
        }
        _ => {}
    }

    Ok(())
}

pub async fn rename_file(app: &mut App, old_path: String, new_path: String) -> Result<()> {
    let panel_type = app.get_active_panel().panel_type.clone();

    match panel_type {
        PanelType::S3Browser { prefix, .. } => {
            if let Some(s3_manager) = &app.get_active_panel().s3_manager {
                // Extract S3 keys from paths (remove s3://bucket/ prefix)
                let old_key = extract_s3_key(&old_path);
                let new_key = extract_s3_key(&new_path);

                s3_manager.copy_object(&old_key, &new_key).await?;
                s3_manager.delete_object(&old_key).await?;

                let objects = s3_manager.list_objects(&prefix).await?;
                let panel = app.get_active_panel();
                panel
                    .list_model
                    .set_items(crate::app::converters::s3_objects_to_items(objects));

                app.show_success("File renamed successfully");
            }
        }
        PanelType::LocalFilesystem { path } => {
            std::fs::rename(&old_path, &new_path)?;

            let has_parent = path.parent().is_some();
            if let Ok(files) = crate::app::navigation::read_local_directory(&path) {
                let panel = app.get_active_panel();
                panel
                    .list_model
                    .set_items(crate::app::converters::local_files_to_items(
                        files, has_parent,
                    ));
            }

            app.show_success("File renamed successfully");
        }
        _ => {}
    }

    Ok(())
}
