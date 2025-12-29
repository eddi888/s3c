use super::{App, LocalFile, Panel, PanelType};
use crate::models::list::{ItemData, ItemType, PanelItem};
use anyhow::{Context, Result};
use std::path::PathBuf;

pub async fn enter_selected(app: &mut App) -> Result<()> {
    let panel_type = app.get_active_panel().panel_type.clone();
    let selected_index = app.get_active_panel().selected_index;

    match panel_type {
        PanelType::ModeSelection => {
            let item = app.get_active_panel().list_model.get_item(selected_index);
            if let Some(PanelItem {
                data: ItemData::Mode(mode),
                ..
            }) = item
            {
                match mode.as_str() {
                    "s3" => {
                        // Navigate to ProfileList
                        let profiles = app.config_manager.aws_profiles.clone();
                        let panel = app.get_active_panel();
                        panel.panel_type = PanelType::ProfileList;
                        panel
                            .list_model
                            .set_items(super::converters::profiles_to_items(&profiles));
                        panel.selected_index = 0;
                    }
                    "local" => {
                        // Navigate to LocalFilesystem
                        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
                        navigate_to_local_dir(app, home).await?;
                    }
                    _ => {}
                }
            }
        }
        PanelType::DriveSelection => {
            let item = app.get_active_panel().list_model.get_item(selected_index);

            match item {
                Some(PanelItem {
                    item_type: ItemType::ParentDir,
                    ..
                }) => {
                    // Navigate back to ModeSelection
                    let panel = app.get_active_panel();
                    panel.panel_type = PanelType::ModeSelection;
                    panel
                        .list_model
                        .set_items(super::converters::modes_to_items());
                    panel.selected_index = 0;
                }
                Some(PanelItem {
                    data: ItemData::Drive(drive),
                    ..
                }) => {
                    let drive_path = drive.clone();
                    navigate_to_local_dir(app, drive_path).await?;
                }
                _ => {}
            }
        }
        PanelType::ProfileList => {
            let item = app.get_active_panel().list_model.get_item(selected_index);

            match item {
                Some(PanelItem {
                    item_type: ItemType::ParentDir,
                    ..
                }) => {
                    // Navigate back to ModeSelection
                    let panel = app.get_active_panel();
                    panel.panel_type = PanelType::ModeSelection;
                    panel
                        .list_model
                        .set_items(super::converters::modes_to_items());
                    panel.selected_index = 0;
                }
                Some(PanelItem {
                    data: ItemData::Profile(profile),
                    ..
                }) => {
                    let profile = profile.clone();
                    if let Some(profile_config) = app.config_manager.get_profile_config(&profile) {
                        if let Some(script_path) = &profile_config.setup_script {
                            if !script_path.trim().is_empty() {
                                app.script.pending_script = Some(script_path.clone());
                                app.script.pending_profile = Some(profile.clone());
                                app.script.pending_bucket = None;
                                app.script.needs_terminal = true;
                                return Ok(());
                            }
                        }
                    }

                    let buckets = app.config_manager.get_buckets_for_profile(&profile);
                    let panel = app.get_active_panel();
                    panel.panel_type = PanelType::BucketList { profile };
                    panel
                        .list_model
                        .set_items(super::converters::buckets_to_items(buckets));
                    panel.selected_index = 0;
                }
                _ => {}
            }
        }
        PanelType::BucketList { profile } => {
            let item = app.get_active_panel().list_model.get_item(selected_index);

            match item {
                Some(PanelItem {
                    item_type: ItemType::ParentDir,
                    ..
                }) => {
                    let profiles = app.config_manager.aws_profiles.clone();
                    let panel = app.get_active_panel();
                    panel.panel_type = PanelType::ProfileList;
                    panel
                        .list_model
                        .set_items(super::converters::profiles_to_items(&profiles));
                    panel.selected_index = 0;
                }
                Some(PanelItem {
                    data: ItemData::Bucket(bucket_config),
                    ..
                }) => {
                    let bucket_name = bucket_config.name.clone();
                    load_s3_bucket(app, profile, bucket_name).await?;
                }
                _ => {}
            }
        }
        PanelType::S3Browser {
            profile,
            bucket,
            prefix,
        } => {
            let item = app.get_active_panel().list_model.get_item(selected_index);

            match item {
                Some(PanelItem {
                    item_type: ItemType::ParentDir,
                    ..
                }) => {
                    // Get base_prefix from bucket config
                    let buckets = app.config_manager.get_buckets_for_profile(&profile);
                    let base_prefix = buckets
                        .iter()
                        .find(|b| b.name == bucket)
                        .and_then(|b| b.base_prefix.clone())
                        .unwrap_or_default();

                    // Check if we're at or below base_prefix - if so, go back to bucket list
                    if prefix.is_empty() || prefix == base_prefix {
                        let panel = app.get_active_panel();
                        panel.panel_type = PanelType::BucketList { profile };
                        panel
                            .list_model
                            .set_items(super::converters::buckets_to_items(buckets));
                        panel.selected_index = 0;
                        panel.s3_manager = None;
                    } else {
                        let parent = prefix
                            .trim_end_matches('/')
                            .rsplit_once('/')
                            .map(|x| x.0)
                            .map(|s| format!("{s}/"))
                            .unwrap_or_default();

                        // Don't go higher than base_prefix
                        let target_prefix = if !base_prefix.is_empty() && parent < base_prefix {
                            base_prefix
                        } else {
                            parent
                        };

                        navigate_to_s3_prefix(app, profile, bucket, target_prefix).await?;
                    }
                }
                Some(PanelItem {
                    item_type: ItemType::Directory,
                    data: ItemData::S3Object(obj),
                    ..
                }) => {
                    let key = obj.key.clone();
                    navigate_to_s3_prefix(app, profile, bucket, key).await?;
                }
                _ => {}
            }
        }
        PanelType::LocalFilesystem { path: _ } => {
            let item = app.get_active_panel().list_model.get_item(selected_index);

            match item {
                Some(PanelItem {
                    item_type: ItemType::ParentDir,
                    ..
                }) => {
                    let parent_path = if let PanelType::LocalFilesystem { path } =
                        &app.get_active_panel().panel_type
                    {
                        path.parent().map(|p| p.to_path_buf())
                    } else {
                        None
                    };

                    if let Some(parent) = parent_path {
                        navigate_to_local_dir(app, parent).await?;
                    } else {
                        // No parent directory - check if Windows for drive selection
                        #[cfg(target_os = "windows")]
                        {
                            let drives = list_windows_drives();
                            if !drives.is_empty() {
                                let panel = app.get_active_panel();
                                panel.panel_type = PanelType::DriveSelection;
                                panel
                                    .list_model
                                    .set_items(super::converters::drives_to_items(drives));
                                panel.selected_index = 0;
                            } else {
                                // Fallback to ModeSelection if no drives found
                                let panel = app.get_active_panel();
                                panel.panel_type = PanelType::ModeSelection;
                                panel
                                    .list_model
                                    .set_items(super::converters::modes_to_items());
                                panel.selected_index = 0;
                            }
                        }
                        #[cfg(not(target_os = "windows"))]
                        {
                            // Linux: Navigate to ModeSelection
                            let panel = app.get_active_panel();
                            panel.panel_type = PanelType::ModeSelection;
                            panel
                                .list_model
                                .set_items(super::converters::modes_to_items());
                            panel.selected_index = 0;
                        }
                    }
                }
                Some(PanelItem {
                    item_type: ItemType::Directory,
                    data: ItemData::LocalFile { path, .. },
                    ..
                }) => {
                    let target_path = path.clone();
                    navigate_to_local_dir(app, target_path).await?;
                }
                _ => {}
            }
        }
    }

    Ok(())
}

async fn load_s3_bucket(app: &mut App, profile: String, bucket: String) -> Result<()> {
    load_s3_bucket_no_script(app, profile, bucket).await
}

pub async fn load_s3_bucket_no_script(
    app: &mut App,
    profile: String,
    bucket: String,
) -> Result<()> {
    let buckets = app.config_manager.get_buckets_for_profile(&profile);
    let bucket_config = buckets
        .iter()
        .find(|b| b.name == bucket)
        .context("Bucket config not found")?;

    let s3_manager = match crate::operations::s3::S3Manager::new(
        &profile,
        bucket.clone(),
        bucket_config.role_chain.clone(),
        &bucket_config.region,
        bucket_config.endpoint_url.as_deref(),
        bucket_config.path_style,
    )
    .await
    {
        Ok(manager) => manager,
        Err(e) => {
            let error_msg = format!("{e}");
            if error_msg.contains("AccessDenied") {
                app.show_error(&format!(
                    "Access denied to bucket '{bucket}': Check permissions"
                ));
            } else {
                app.show_error(&format!("Failed to connect to bucket '{bucket}': {e}"));
            }
            return Ok(());
        }
    };

    // Use base_prefix if configured
    let initial_prefix = bucket_config.base_prefix.clone().unwrap_or_default();

    match s3_manager.list_objects(&initial_prefix).await {
        Ok(objects) => {
            let panel = app.get_active_panel();
            panel.panel_type = PanelType::S3Browser {
                profile,
                bucket,
                prefix: initial_prefix,
            };
            panel
                .list_model
                .set_items(super::converters::s3_objects_to_items(objects));
            panel.selected_index = 0;
            panel.s3_manager = Some(s3_manager);
        }
        Err(e) => {
            let error_msg = format!("{e}");
            if error_msg.contains("NoSuchBucket") {
                app.show_error(&format!(
                    "Bucket '{bucket}' does not exist or is in wrong region"
                ));
            } else if error_msg.contains("AccessDenied") {
                app.show_error(&format!(
                    "Access denied to bucket '{bucket}': Check permissions"
                ));
            } else {
                app.show_error(&format!("Failed to list bucket '{bucket}': {e}"));
            }
        }
    }

    Ok(())
}

async fn navigate_to_s3_prefix(
    app: &mut App,
    profile: String,
    bucket: String,
    prefix: String,
) -> Result<()> {
    let panel = app.get_active_panel();
    if let Some(ref s3_manager) = panel.s3_manager {
        let objects = s3_manager.list_objects(&prefix).await?;
        panel.panel_type = PanelType::S3Browser {
            profile,
            bucket,
            prefix,
        };
        panel
            .list_model
            .set_items(super::converters::s3_objects_to_items(objects));
        panel.selected_index = 0;
    }
    Ok(())
}

async fn navigate_to_local_dir(app: &mut App, path: PathBuf) -> Result<()> {
    // Always show ".." to allow navigation back to ModeSelection at root
    let has_parent = true;
    match read_local_directory(&path) {
        Ok(files) => {
            let panel = app.get_active_panel();
            panel.panel_type = PanelType::LocalFilesystem { path };
            panel
                .list_model
                .set_items(super::converters::local_files_to_items(files, has_parent));
            panel.selected_index = 0;
        }
        Err(e) => {
            let error_msg = format!("{e}");
            let path_display = path.display();
            if error_msg.contains("Permission denied") || error_msg.contains("permission denied") {
                app.show_error(&format!(
                    "Permission denied: Cannot access '{path_display}'"
                ));
            } else {
                app.show_error(&format!("Cannot access '{path_display}': {e}"));
            }
        }
    }
    Ok(())
}

pub fn read_local_directory(path: &PathBuf) -> Result<Vec<LocalFile>> {
    let mut files = Vec::new();

    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        let file_name = entry.file_name().to_string_lossy().to_string();
        let modified = metadata.modified().ok();

        files.push(LocalFile {
            name: file_name,
            path: entry.path(),
            is_dir: metadata.is_dir(),
            size: metadata.len(),
            modified,
        });
    }

    files.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });

    Ok(files)
}

pub fn toggle_local_filesystem(app: &mut App) -> Result<()> {
    let is_local = matches!(
        app.get_active_panel().panel_type,
        PanelType::LocalFilesystem { .. }
    );

    if is_local {
        let profiles = app.config_manager.aws_profiles.clone();
        let panel = app.get_active_panel();
        *panel = Panel::new_profile_list();
        panel
            .list_model
            .set_items(super::converters::profiles_to_items(&profiles));
    } else {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let has_parent = home.parent().is_some();
        if let Ok(files) = read_local_directory(&home) {
            let panel = app.get_active_panel();
            panel.panel_type = PanelType::LocalFilesystem { path: home };
            panel
                .list_model
                .set_items(super::converters::local_files_to_items(files, has_parent));
            panel.selected_index = 0;
        }
    }
    Ok(())
}

pub async fn reload_local_files(app: &mut App) -> Result<()> {
    let path_clone =
        if let PanelType::LocalFilesystem { path } = &app.get_inactive_panel().panel_type {
            Some(path.clone())
        } else {
            None
        };

    if let Some(path) = path_clone {
        let has_parent = path.parent().is_some();
        match read_local_directory(&path) {
            Ok(files) => {
                let panel = app.get_inactive_panel_mut();
                panel
                    .list_model
                    .set_items(super::converters::local_files_to_items(files, has_parent));
            }
            Err(e) => {
                let error_msg = format!("{e}");
                let path_display = path.display();
                if error_msg.contains("Permission denied")
                    || error_msg.contains("permission denied")
                {
                    app.show_error(&format!(
                        "Permission denied: Cannot reload '{path_display}'"
                    ));
                } else {
                    app.show_error(&format!("Cannot reload '{path_display}': {e}"));
                }
            }
        }
    }
    Ok(())
}

#[allow(dead_code)]
pub fn list_windows_drives() -> Vec<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        use std::fs;
        ('A'..='Z')
            .filter_map(|letter| {
                let drive = PathBuf::from(format!("{letter}:\\"));
                // Check if drive exists by trying to read metadata
                if fs::metadata(&drive).is_ok() {
                    Some(drive)
                } else {
                    None
                }
            })
            .collect()
    }
    #[cfg(not(target_os = "windows"))]
    {
        Vec::new()
    }
}

pub async fn reload_s3_browser(app: &mut App) -> Result<()> {
    let panel = app.get_inactive_panel_mut();
    if let PanelType::S3Browser { prefix, .. } = &panel.panel_type {
        let prefix_clone = prefix.clone();
        if let Some(s3_manager) = &panel.s3_manager {
            let objects = s3_manager.list_objects(&prefix_clone).await?;
            panel
                .list_model
                .set_items(super::converters::s3_objects_to_items(objects));
        }
    }
    Ok(())
}
