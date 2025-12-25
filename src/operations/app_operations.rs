use crate::app::{App, PanelType};
use crate::models::list::{ItemData, ItemType, PanelItem};
use anyhow::Result;
use std::io::Read;

pub async fn confirm_delete(app: &mut App) -> Result<()> {
    let path = app.delete_confirmation.path.clone();
    let is_dir = app.delete_confirmation.is_dir;

    let panel_type = app.get_active_panel().panel_type.clone();

    match panel_type {
        PanelType::S3Browser { prefix, .. } => {
            if let Some(s3_manager) = &app.get_active_panel().s3_manager {
                s3_manager.delete_object(&path).await?;

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
    let panel = app.get_active_panel();
    let panel_type = panel.panel_type.clone();
    let selected_index = panel.selected_index;

    match panel_type {
        PanelType::S3Browser { .. } => {
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

                if let Some(s3_manager) = &app.get_active_panel().s3_manager {
                    match s3_manager.get_object_size(&key).await {
                        Ok(file_size) => {
                            let chunk_size = 100 * 1024;
                            let load_size = if file_size < chunk_size {
                                file_size
                            } else {
                                chunk_size
                            };

                            match s3_manager.get_object_range(&key, 0, load_size - 1).await {
                                Ok(bytes) => match String::from_utf8(bytes) {
                                    Ok(content) => {
                                        app.preview.filename = filename;
                                        app.preview.content = content;
                                        app.preview.scroll_offset = 0;
                                        app.preview.file_size = file_size;
                                        app.preview.is_s3 = true;
                                        app.preview.s3_key = key;
                                        app.preview.byte_offset = load_size;
                                        app.preview.total_lines = None;
                                        app.prev_screen = Some(app.screen.clone());
                                        app.screen = crate::app::Screen::FilePreview;
                                    }
                                    Err(_) => {
                                        app.show_error("File is not valid UTF-8 text");
                                    }
                                },
                                Err(e) => {
                                    app.show_error(&format!("Failed to preview: {e}"));
                                }
                            }
                        }
                        Err(e) => {
                            app.show_error(&format!("Failed to get file info: {e}"));
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
                size,
                ..
            }) = item
            {
                let file_path = file_path.clone();
                let file_name = name.clone();
                let file_size = size.unwrap_or(0);

                match std::fs::File::open(&file_path) {
                    Ok(file) => {
                        let mut buffer = Vec::new();
                        let max_bytes = 1024 * 1024;

                        match file.take(max_bytes as u64).read_to_end(&mut buffer) {
                            Ok(_) => match String::from_utf8(buffer) {
                                Ok(content) => {
                                    app.preview.filename = file_name;
                                    app.preview.content = content;
                                    app.preview.scroll_offset = 0;
                                    app.preview.file_size = file_size as i64;
                                    app.preview.is_s3 = false;
                                    app.preview.byte_offset = max_bytes as i64;
                                    app.prev_screen = Some(app.screen.clone());
                                    app.screen = crate::app::Screen::FilePreview;
                                }
                                Err(_) => {
                                    app.show_error("File is not valid UTF-8 text");
                                }
                            },
                            Err(e) => {
                                app.show_error(&format!("Failed to read file: {e}"));
                            }
                        }
                    }
                    Err(e) => {
                        app.show_error(&format!("Failed to open file: {e}"));
                    }
                }
            }
        }
        _ => {
            app.show_error("Preview only available for files");
        }
    }

    Ok(())
}

pub async fn load_more_preview_content(app: &mut App) -> Result<()> {
    if !app.preview.is_s3 {
        return Ok(());
    }

    if app.preview.byte_offset >= app.preview.file_size {
        return Ok(());
    }

    let chunk_size = 100 * 1024;
    let end_byte = (app.preview.byte_offset + chunk_size - 1).min(app.preview.file_size - 1);
    let s3_key = app.preview.s3_key.clone();
    let start_byte = app.preview.byte_offset;

    if let Some(s3_manager) = &app.get_active_panel().s3_manager {
        if let Ok(bytes) = s3_manager
            .get_object_range(&s3_key, start_byte, end_byte)
            .await
        {
            if let Ok(additional_content) = String::from_utf8(bytes) {
                app.preview.content.push_str(&additional_content);
                app.preview.byte_offset = end_byte + 1;
            }
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
                s3_manager.copy_object(&old_path, &new_path).await?;
                s3_manager.delete_object(&old_path).await?;

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
