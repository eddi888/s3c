use crate::config::ConfigManager;
use crate::list_model::{ItemData, ItemType, PanelItem, PanelListModel};
use crate::s3_ops::{S3Manager, S3Object};
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum PanelType {
    ProfileList,
    BucketList {
        profile: String,
    },
    S3Browser {
        profile: String,
        bucket: String,
        prefix: String,
    },
    LocalFilesystem {
        path: PathBuf,
    },
}

#[derive(Debug, Clone)]
pub struct LocalFile {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<std::time::SystemTime>,
}

pub struct Panel {
    pub panel_type: PanelType,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub visible_height: usize,
    pub list_model: PanelListModel,
    pub s3_manager: Option<S3Manager>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    DualPanel,
    ConfigForm,
    ProfileConfigForm,
    SortDialog,
    DeleteConfirmation,
    FilePreview,
    Input,
    Help,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActivePanel {
    Left,
    Right,
}

pub struct App {
    pub config_manager: ConfigManager,
    pub screen: Screen,
    pub left_panel: Panel,
    pub right_panel: Panel,
    pub active_panel: ActivePanel,
    pub input_buffer: String,
    pub input_prompt: String,
    pub error_message: String,
    pub success_message: String,
    pub should_quit: bool,
    pub prev_screen: Option<Screen>,
    pub config_form_profile: String,
    pub config_form_bucket: String,
    pub config_form_description: String,
    pub config_form_region: String,
    pub config_form_roles: Vec<String>,
    pub config_form_field: usize,
    pub config_form_cursor: usize,
    pub profile_form_name: String,
    pub profile_form_description: String,
    pub profile_form_setup_script: String,
    pub profile_form_field: usize,
    pub profile_form_cursor: usize,
    pub preview_content: String,
    pub preview_filename: String,
    pub preview_scroll_offset: usize,
    pub preview_file_size: i64,
    pub preview_is_s3: bool,
    pub preview_s3_key: String,
    pub preview_byte_offset: i64,
    pub preview_total_lines: Option<usize>,
    pub input_mode: InputMode,
    pub pending_script: Option<String>,
    pub pending_script_profile: Option<String>,
    pub pending_script_bucket: Option<Option<String>>,
    pub needs_terminal_for_script: bool,
    pub sort_dialog_selected: usize,
    pub delete_confirmation_path: String,
    pub delete_confirmation_name: String,
    pub delete_confirmation_is_dir: bool,
    pub delete_confirmation_button: usize,
    pub input_cursor_position: usize,
    pub rename_original_path: String,
    pub advanced_mode: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    None,
    CreateFolder,
    Filter,
    Rename,
    UploadPath {
        local_file_path: std::path::PathBuf,
        local_file_name: String,
    },
}

impl Panel {
    pub fn new_profile_list() -> Self {
        Self {
            panel_type: PanelType::ProfileList,
            selected_index: 0,
            scroll_offset: 0,
            visible_height: 10,
            list_model: PanelListModel::empty(),
            s3_manager: None,
        }
    }

    pub fn new_local_filesystem() -> Self {
        Self {
            panel_type: PanelType::LocalFilesystem {
                path: std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/")),
            },
            selected_index: 0,
            scroll_offset: 0,
            visible_height: 10,
            list_model: PanelListModel::empty(),
            s3_manager: None,
        }
    }
}

// Helper functions to convert data to PanelItems
fn profiles_to_items(profiles: &[String]) -> Vec<PanelItem> {
    profiles
        .iter()
        .map(|profile| PanelItem {
            name: profile.clone(),
            item_type: ItemType::Directory,
            size: None,
            modified: None,
            data: ItemData::Profile(profile.clone()),
        })
        .collect()
}

pub fn buckets_to_items(buckets: Vec<crate::config::BucketConfig>) -> Vec<PanelItem> {
    let mut items = vec![PanelItem {
        name: "..".to_string(),
        item_type: ItemType::ParentDir,
        size: None,
        modified: None,
        data: ItemData::Profile("..".to_string()),
    }];

    items.extend(buckets.into_iter().map(|bucket| PanelItem {
        name: bucket.name.clone(),
        item_type: ItemType::Directory,
        size: None,
        modified: None,
        data: ItemData::Bucket(bucket),
    }));

    items
}

fn s3_objects_to_items(objects: Vec<S3Object>) -> Vec<PanelItem> {
    let mut items = vec![PanelItem {
        name: "..".to_string(),
        item_type: ItemType::ParentDir,
        size: None,
        modified: None,
        data: ItemData::Profile("..".to_string()),
    }];

    items.extend(objects.into_iter().map(|obj| PanelItem {
        name: if obj.is_prefix {
            // For directories, extract the last folder name
            obj.key
                .trim_end_matches('/')
                .rsplit_once('/')
                .map(|(_, name)| name.to_string())
                .unwrap_or_else(|| obj.key.trim_end_matches('/').to_string())
        } else {
            // For files, extract only the filename (last part after /)
            obj.key
                .rsplit_once('/')
                .map(|(_, name)| name.to_string())
                .unwrap_or_else(|| obj.key.clone())
        },
        item_type: if obj.is_prefix {
            ItemType::Directory
        } else {
            ItemType::File
        },
        size: if obj.is_prefix {
            None
        } else {
            Some(obj.size as u64)
        },
        modified: obj.last_modified,
        data: ItemData::S3Object(obj),
    }));

    items
}

fn local_files_to_items(files: Vec<LocalFile>, has_parent: bool) -> Vec<PanelItem> {
    let mut items = Vec::new();

    if has_parent {
        items.push(PanelItem {
            name: "..".to_string(),
            item_type: ItemType::ParentDir,
            size: None,
            modified: None,
            data: ItemData::Profile("..".to_string()),
        });
    }

    items.extend(files.into_iter().map(|file| {
        let modified = file.modified.and_then(|st| {
            st.duration_since(std::time::UNIX_EPOCH)
                .ok()
                .and_then(|d| chrono::DateTime::from_timestamp(d.as_secs() as i64, 0))
        });

        PanelItem {
            name: file.name.clone(),
            item_type: if file.is_dir {
                ItemType::Directory
            } else {
                ItemType::File
            },
            size: if file.is_dir { None } else { Some(file.size) },
            modified,
            data: ItemData::LocalFile {
                path: file.path,
                is_dir: file.is_dir,
            },
        }
    }));

    items
}

impl App {
    pub fn new() -> Result<Self> {
        let config_manager = ConfigManager::new()?;

        let mut app = Self {
            config_manager,
            screen: Screen::DualPanel,
            left_panel: Panel::new_profile_list(),
            right_panel: Panel::new_local_filesystem(),
            active_panel: ActivePanel::Left,
            input_buffer: String::new(),
            input_prompt: String::new(),
            error_message: String::new(),
            success_message: String::new(),
            should_quit: false,
            prev_screen: None,
            config_form_profile: String::new(),
            config_form_bucket: String::new(),
            config_form_description: String::new(),
            config_form_region: "eu-west-1".to_string(),
            config_form_roles: Vec::new(),
            config_form_field: 0,
            config_form_cursor: 0,
            profile_form_name: String::new(),
            profile_form_description: String::new(),
            profile_form_setup_script: String::new(),
            profile_form_field: 0,
            profile_form_cursor: 0,
            preview_content: String::new(),
            preview_filename: String::new(),
            preview_scroll_offset: 0,
            preview_file_size: 0,
            preview_is_s3: false,
            preview_s3_key: String::new(),
            preview_byte_offset: 0,
            preview_total_lines: None,
            input_mode: InputMode::None,
            pending_script: None,
            pending_script_profile: None,
            pending_script_bucket: None,
            needs_terminal_for_script: false,
            sort_dialog_selected: 0,
            delete_confirmation_path: String::new(),
            delete_confirmation_name: String::new(),
            delete_confirmation_is_dir: false,
            delete_confirmation_button: 0,
            input_cursor_position: 0,
            rename_original_path: String::new(),
            advanced_mode: false,
        };

        // Load local files for right panel
        if let PanelType::LocalFilesystem { path } = &app.right_panel.panel_type {
            let path_clone = path.clone();
            let has_parent = path_clone.parent().is_some();
            if let Ok(files) = app.read_local_directory(&path_clone) {
                app.right_panel
                    .list_model
                    .set_items(local_files_to_items(files, has_parent));
            }
        }

        // Load profiles for left panel
        let profiles = app.config_manager.aws_profiles.clone();
        app.left_panel
            .list_model
            .set_items(profiles_to_items(&profiles));

        Ok(app)
    }

    pub fn get_active_panel(&mut self) -> &mut Panel {
        match self.active_panel {
            ActivePanel::Left => &mut self.left_panel,
            ActivePanel::Right => &mut self.right_panel,
        }
    }

    pub fn switch_panel(&mut self) {
        self.active_panel = match self.active_panel {
            ActivePanel::Left => ActivePanel::Right,
            ActivePanel::Right => ActivePanel::Left,
        };
    }

    pub fn navigate_up(&mut self) {
        let panel = self.get_active_panel();
        if panel.selected_index > 0 {
            panel.selected_index -= 1;
            // Adjust scroll if selected moved above visible area
            if panel.selected_index < panel.scroll_offset {
                panel.scroll_offset = panel.selected_index;
            }
        }
    }

    pub fn navigate_down(&mut self) {
        let max = self.get_panel_item_count();
        let panel = self.get_active_panel();
        if panel.selected_index < max.saturating_sub(1) {
            panel.selected_index += 1;
        }
    }

    pub fn navigate_page_up(&mut self) {
        let panel = self.get_active_panel();
        let page_size = panel.visible_height;

        if panel.selected_index >= page_size {
            panel.selected_index -= page_size;
            panel.scroll_offset = panel.scroll_offset.saturating_sub(page_size);
        } else {
            panel.selected_index = 0;
            panel.scroll_offset = 0;
        }
    }

    pub fn navigate_page_down(&mut self) {
        let max = self.get_panel_item_count();
        let panel = self.get_active_panel();
        let page_size = panel.visible_height;

        if panel.selected_index + page_size < max {
            panel.selected_index += page_size;
            panel.scroll_offset =
                (panel.scroll_offset + page_size).min(max.saturating_sub(panel.visible_height));
        } else if max > 0 {
            panel.selected_index = max - 1;
            panel.scroll_offset = max.saturating_sub(panel.visible_height);
        }
    }

    pub fn navigate_home(&mut self) {
        let panel = self.get_active_panel();
        panel.selected_index = 0;
        panel.scroll_offset = 0;
    }

    pub fn navigate_end(&mut self) {
        let max = self.get_panel_item_count();
        let panel = self.get_active_panel();

        if max > 0 {
            panel.selected_index = max - 1;
            panel.scroll_offset = max.saturating_sub(panel.visible_height);
        }
    }

    fn get_panel_item_count(&self) -> usize {
        let panel = match self.active_panel {
            ActivePanel::Left => &self.left_panel,
            ActivePanel::Right => &self.right_panel,
        };
        panel.list_model.len()
    }

    pub async fn enter_selected(&mut self) -> Result<()> {
        let panel_type = self.get_active_panel().panel_type.clone();
        let selected_index = self.get_active_panel().selected_index;

        match panel_type {
            PanelType::ProfileList => {
                let item = self.get_active_panel().list_model.get_item(selected_index);
                if let Some(PanelItem {
                    data: ItemData::Profile(profile),
                    ..
                }) = item
                {
                    let profile = profile.clone();
                    // Execute setup script if configured for this profile
                    if let Some(profile_config) = self.config_manager.get_profile_config(&profile) {
                        if let Some(script_path) = &profile_config.setup_script {
                            if !script_path.trim().is_empty() {
                                // Mark that we need to run a script interactively
                                self.pending_script = Some(script_path.clone());
                                self.pending_script_profile = Some(profile.clone());
                                self.pending_script_bucket = None; // No bucket yet
                                self.needs_terminal_for_script = true;
                                return Ok(());
                            }
                        }
                    }

                    // Switch to bucket list for this profile
                    let buckets = self.config_manager.get_buckets_for_profile(&profile);
                    let panel = self.get_active_panel();
                    panel.panel_type = PanelType::BucketList { profile };
                    panel.list_model.set_items(buckets_to_items(buckets));
                    panel.selected_index = 0;
                }
            }
            PanelType::BucketList { profile } => {
                let item = self.get_active_panel().list_model.get_item(selected_index);

                match item {
                    Some(PanelItem {
                        item_type: ItemType::ParentDir,
                        ..
                    }) => {
                        // Go back to ProfileList
                        let profiles = self.config_manager.aws_profiles.clone();
                        let panel = self.get_active_panel();
                        panel.panel_type = PanelType::ProfileList;
                        panel.list_model.set_items(profiles_to_items(&profiles));
                        panel.selected_index = 0;
                    }
                    Some(PanelItem {
                        data: ItemData::Bucket(bucket_config),
                        ..
                    }) => {
                        let bucket_name = bucket_config.name.clone();
                        self.load_s3_bucket(profile, bucket_name).await?;
                    }
                    _ => {}
                }
            }
            PanelType::S3Browser {
                profile,
                bucket,
                prefix,
            } => {
                let item = self.get_active_panel().list_model.get_item(selected_index);

                match item {
                    Some(PanelItem {
                        item_type: ItemType::ParentDir,
                        ..
                    }) => {
                        if prefix.is_empty() {
                            // At root - go back to BucketList
                            let buckets = self.config_manager.get_buckets_for_profile(&profile);
                            let panel = self.get_active_panel();
                            panel.panel_type = PanelType::BucketList { profile };
                            panel.list_model.set_items(buckets_to_items(buckets));
                            panel.selected_index = 0;
                            panel.s3_manager = None;
                        } else {
                            // Navigate to parent prefix
                            let parent = prefix
                                .trim_end_matches('/')
                                .rsplit_once('/')
                                .map(|x| x.0)
                                .map(|s| format!("{s}/"))
                                .unwrap_or_default();
                            self.navigate_to_s3_prefix(profile, bucket, parent).await?;
                        }
                    }
                    Some(PanelItem {
                        item_type: ItemType::Directory,
                        data: ItemData::S3Object(obj),
                        ..
                    }) => {
                        let key = obj.key.clone();
                        self.navigate_to_s3_prefix(profile, bucket, key).await?;
                    }
                    _ => {}
                }
            }
            PanelType::LocalFilesystem { path: _ } => {
                let item = self.get_active_panel().list_model.get_item(selected_index);

                match item {
                    Some(PanelItem {
                        item_type: ItemType::ParentDir,
                        ..
                    }) => {
                        // Navigate to parent directory
                        let parent_path = if let PanelType::LocalFilesystem { path } =
                            &self.get_active_panel().panel_type
                        {
                            path.parent().map(|p| p.to_path_buf())
                        } else {
                            None
                        };

                        if let Some(parent) = parent_path {
                            self.navigate_to_local_dir(parent).await?;
                        }
                    }
                    Some(PanelItem {
                        item_type: ItemType::Directory,
                        data: ItemData::LocalFile { path, .. },
                        ..
                    }) => {
                        let target_path = path.clone();
                        self.navigate_to_local_dir(target_path).await?;
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    async fn load_s3_bucket(&mut self, profile: String, bucket: String) -> Result<()> {
        // Script is already executed when profile is selected
        self.load_s3_bucket_no_script(profile, bucket).await
    }

    pub async fn load_s3_bucket_no_script(
        &mut self,
        profile: String,
        bucket: String,
    ) -> Result<()> {
        let buckets = self.config_manager.get_buckets_for_profile(&profile);
        let bucket_config = buckets
            .iter()
            .find(|b| b.name == bucket)
            .context("Bucket config not found")?;

        let s3_manager = match S3Manager::new(
            &profile,
            bucket.clone(),
            bucket_config.role_chain.clone(),
            &bucket_config.region,
        )
        .await
        {
            Ok(manager) => manager,
            Err(e) => {
                let error_msg = format!("{e}");
                if error_msg.contains("AccessDenied") {
                    self.show_error(&format!(
                        "Access denied to bucket '{bucket}': Check permissions"
                    ));
                } else {
                    self.show_error(&format!("Failed to connect to bucket '{bucket}': {e}"));
                }
                return Ok(());
            }
        };

        match s3_manager.list_objects("").await {
            Ok(objects) => {
                let panel = self.get_active_panel();
                panel.panel_type = PanelType::S3Browser {
                    profile,
                    bucket,
                    prefix: String::new(),
                };
                panel.list_model.set_items(s3_objects_to_items(objects));
                panel.selected_index = 0;
                panel.s3_manager = Some(s3_manager);
            }
            Err(e) => {
                let error_msg = format!("{e}");
                if error_msg.contains("NoSuchBucket") {
                    self.show_error(&format!(
                        "Bucket '{bucket}' does not exist or is in wrong region"
                    ));
                } else if error_msg.contains("AccessDenied") {
                    self.show_error(&format!(
                        "Access denied to bucket '{bucket}': Check permissions"
                    ));
                } else {
                    self.show_error(&format!("Failed to list bucket '{bucket}': {e}"));
                }
            }
        }

        Ok(())
    }

    async fn navigate_to_s3_prefix(
        &mut self,
        profile: String,
        bucket: String,
        prefix: String,
    ) -> Result<()> {
        let panel = self.get_active_panel();
        if let Some(ref s3_manager) = panel.s3_manager {
            let objects = s3_manager.list_objects(&prefix).await?;
            panel.panel_type = PanelType::S3Browser {
                profile,
                bucket,
                prefix,
            };
            panel.list_model.set_items(s3_objects_to_items(objects));
            panel.selected_index = 0;
        }
        Ok(())
    }

    async fn navigate_to_local_dir(&mut self, path: PathBuf) -> Result<()> {
        let has_parent = path.parent().is_some();
        match self.read_local_directory(&path) {
            Ok(files) => {
                let panel = self.get_active_panel();
                panel.panel_type = PanelType::LocalFilesystem { path };
                panel
                    .list_model
                    .set_items(local_files_to_items(files, has_parent));
                panel.selected_index = 0;
            }
            Err(e) => {
                let error_msg = format!("{e}");
                let path_display = path.display();
                if error_msg.contains("Permission denied")
                    || error_msg.contains("permission denied")
                {
                    self.show_error(&format!(
                        "Permission denied: Cannot access '{path_display}'"
                    ));
                } else {
                    self.show_error(&format!("Cannot access '{path_display}': {e}"));
                }
            }
        }
        Ok(())
    }

    fn read_local_directory(&self, path: &PathBuf) -> Result<Vec<LocalFile>> {
        let mut files = Vec::new();

        for entry in fs::read_dir(path)? {
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

    pub fn toggle_local_filesystem(&mut self) {
        let is_local = matches!(
            self.get_active_panel().panel_type,
            PanelType::LocalFilesystem { .. }
        );

        if is_local {
            let profiles = self.config_manager.aws_profiles.clone();
            let panel = self.get_active_panel();
            *panel = Panel::new_profile_list();
            panel.list_model.set_items(profiles_to_items(&profiles));
        } else {
            let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
            let has_parent = home.parent().is_some();
            if let Ok(files) = self.read_local_directory(&home) {
                let panel = self.get_active_panel();
                panel.panel_type = PanelType::LocalFilesystem { path: home };
                panel
                    .list_model
                    .set_items(local_files_to_items(files, has_parent));
                panel.selected_index = 0;
            }
        }
    }

    pub fn show_error(&mut self, message: &str) {
        self.error_message = message.to_string();
    }

    pub fn show_success(&mut self, message: &str) {
        self.success_message = message.to_string();
    }

    pub async fn load_more_preview_content(&mut self) -> Result<()> {
        if !self.preview_is_s3 {
            return Ok(()); // Local files loaded fully on open
        }

        if self.preview_byte_offset >= self.preview_file_size {
            return Ok(()); // Already loaded everything
        }

        // Load next 100KB chunk
        let chunk_size = 100 * 1024;
        let end_byte = (self.preview_byte_offset + chunk_size - 1).min(self.preview_file_size - 1);
        let s3_key = self.preview_s3_key.clone();
        let start_byte = self.preview_byte_offset;

        if let Some(s3_manager) = &self.get_active_panel().s3_manager {
            match s3_manager
                .get_object_range(&s3_key, start_byte, end_byte)
                .await
            {
                Ok(bytes) => {
                    match String::from_utf8(bytes) {
                        Ok(additional_content) => {
                            self.preview_content.push_str(&additional_content);
                            self.preview_byte_offset = end_byte + 1;
                        }
                        Err(_) => {
                            // Stop loading if we hit non-UTF8 content
                        }
                    }
                }
                Err(_) => {
                    // Silently fail - user can retry by scrolling more
                }
            }
        }

        Ok(())
    }

    pub fn go_back(&mut self) {
        if let Some(prev) = self.prev_screen.take() {
            self.screen = prev;
        } else {
            self.screen = Screen::DualPanel;
        }
    }

    pub fn show_config_form(&mut self) {
        // Get current profile from active panel for new bucket
        if let PanelType::BucketList { profile } = &self.get_active_panel().panel_type {
            self.config_form_profile = profile.clone();
            self.config_form_bucket = String::new();
            self.config_form_description = String::new();
            self.config_form_region = "eu-west-1".to_string();
            self.config_form_roles = vec![String::new()];
            self.config_form_field = 0;
            self.config_form_cursor = 0;
            self.prev_screen = Some(self.screen.clone());
            self.screen = Screen::ConfigForm;
        }
    }

    pub fn edit_bucket_config(&mut self) {
        // Load existing bucket config for editing
        let panel_type = self.get_active_panel().panel_type.clone();
        let selected_index = self.get_active_panel().selected_index;

        if let PanelType::BucketList { profile } = panel_type {
            let item = self.get_active_panel().list_model.get_item(selected_index);

            if let Some(PanelItem {
                item_type: ItemType::ParentDir,
                ..
            }) = item
            {
                return; // Skip ".." parent directory
            }

            if let Some(PanelItem {
                data: ItemData::Bucket(bucket_config),
                ..
            }) = item
            {
                let bucket_config = bucket_config.clone();

                self.config_form_profile = profile;
                self.config_form_bucket = bucket_config.name.clone();
                self.config_form_description =
                    bucket_config.description.clone().unwrap_or_default();
                self.config_form_region = bucket_config.region.clone();
                self.config_form_roles = if bucket_config.role_chain.is_empty() {
                    vec![String::new()]
                } else {
                    bucket_config.role_chain.clone()
                };
                self.config_form_field = 0;
                self.config_form_cursor = 0;
                self.prev_screen = Some(self.screen.clone());
                self.screen = Screen::ConfigForm;
            }
        }
    }

    pub fn show_profile_config_form(&mut self) {
        // Get current profile from active panel
        if let PanelType::ProfileList = self.get_active_panel().panel_type {
            let selected_index = self.get_active_panel().selected_index;
            let item = self.get_active_panel().list_model.get_item(selected_index);

            if let Some(PanelItem {
                data: ItemData::Profile(profile),
                ..
            }) = item
            {
                let profile = profile.clone();
                self.profile_form_name = profile.clone();

                // Load existing data if available
                if let Some(profile_config) = self.config_manager.get_profile_config(&profile) {
                    self.profile_form_description =
                        profile_config.description.clone().unwrap_or_default();
                    self.profile_form_setup_script =
                        profile_config.setup_script.clone().unwrap_or_default();
                } else {
                    self.profile_form_description = String::new();
                    self.profile_form_setup_script = String::new();
                }

                self.profile_form_field = 0;
                self.profile_form_cursor = 0;
                self.prev_screen = Some(self.screen.clone());
                self.screen = Screen::ProfileConfigForm;
            }
        }
    }

    pub fn save_profile_config(&mut self) -> Result<()> {
        let description = if self.profile_form_description.trim().is_empty() {
            None
        } else {
            Some(self.profile_form_description.clone())
        };

        let setup_script = if self.profile_form_setup_script.trim().is_empty() {
            None
        } else {
            Some(self.profile_form_setup_script.clone())
        };

        // Update or create profile config
        if let Some(profile_config) = self
            .config_manager
            .app_config
            .profiles
            .iter_mut()
            .find(|p| p.name == self.profile_form_name)
        {
            profile_config.description = description;
            profile_config.setup_script = setup_script;
        } else {
            self.config_manager
                .app_config
                .profiles
                .push(crate::config::ProfileConfig {
                    name: self.profile_form_name.clone(),
                    buckets: Vec::new(),
                    setup_script,
                    description,
                });
        }

        self.config_manager.save()?;
        self.show_success("Profile configuration saved!");
        Ok(())
    }

    pub fn delete_bucket_config(&mut self) -> Result<()> {
        // Delete selected bucket from profile
        let panel_type = self.get_active_panel().panel_type.clone();
        let selected_index = self.get_active_panel().selected_index;

        if let PanelType::BucketList { profile } = panel_type {
            let item = self.get_active_panel().list_model.get_item(selected_index);

            if let Some(PanelItem {
                item_type: ItemType::ParentDir,
                ..
            }) = item
            {
                return Ok(()); // Skip ".." parent directory
            }

            if let Some(PanelItem {
                data: ItemData::Bucket(bucket_config),
                ..
            }) = item
            {
                let bucket_name = bucket_config.name.clone();

                self.config_manager
                    .remove_bucket_from_profile(&profile, &bucket_name)?;

                // Reload bucket list
                let buckets = self.config_manager.get_buckets_for_profile(&profile);
                let panel = self.get_active_panel();
                panel.list_model.set_items(buckets_to_items(buckets));

                // Update selected index if needed
                if panel.selected_index > 0 {
                    panel.selected_index -= 1;
                }

                self.show_success(&format!("Bucket '{bucket_name}' deleted!"));
            }
        }
        Ok(())
    }

    pub fn save_config_form(&mut self) -> Result<()> {
        if !self.config_form_bucket.trim().is_empty() {
            let roles: Vec<String> = self
                .config_form_roles
                .iter()
                .filter(|r| !r.trim().is_empty())
                .cloned()
                .collect();

            let description = if self.config_form_description.trim().is_empty() {
                None
            } else {
                Some(self.config_form_description.clone())
            };

            // Check if bucket already exists (editing mode)
            let buckets = self
                .config_manager
                .get_buckets_for_profile(&self.config_form_profile);
            let bucket_exists = buckets.iter().any(|b| b.name == self.config_form_bucket);

            if bucket_exists {
                // Update existing bucket
                self.config_manager.remove_bucket_from_profile(
                    &self.config_form_profile,
                    &self.config_form_bucket,
                )?;
            }

            self.config_manager.add_bucket_to_profile(
                &self.config_form_profile,
                self.config_form_bucket.clone(),
                roles,
                self.config_form_region.clone(),
                description,
            )?;

            self.show_success("Bucket configuration saved!");
        }
        Ok(())
    }

    pub fn add_role_field(&mut self) {
        self.config_form_roles.push(String::new());
    }

    pub fn remove_last_role(&mut self) {
        if self.config_form_roles.len() > 1 {
            self.config_form_roles.pop();
            if self.config_form_field >= 2 + self.config_form_roles.len() {
                self.config_form_field = 2 + self.config_form_roles.len() - 1;
            }
        }
    }

    pub async fn copy_to_other_panel(&mut self) -> Result<()> {
        let (source_panel, dest_panel) = match self.active_panel {
            ActivePanel::Left => (&self.left_panel, &self.right_panel),
            ActivePanel::Right => (&self.right_panel, &self.left_panel),
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

                    if let Some(s3_manager) = &source_panel.s3_manager {
                        match s3_manager.download_file(&key, &local_path).await {
                            Ok(_) => {
                                self.show_success(&format!("Downloaded: {filename}"));

                                // Reload destination panel
                                self.reload_local_files().await?;
                            }
                            Err(e) => {
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
                    ..
                }) = item
                {
                    let dest_file_path = dest_path.join(name);

                    match fs::copy(source_file_path, &dest_file_path) {
                        Ok(_) => {
                            self.show_success(&format!("Copied: {name}"));
                            self.reload_local_files().await?;
                        }
                        Err(e) => {
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

    pub fn get_inactive_panel(&self) -> &Panel {
        match self.active_panel {
            ActivePanel::Left => &self.right_panel,
            ActivePanel::Right => &self.left_panel,
        }
    }

    pub fn get_inactive_panel_mut(&mut self) -> &mut Panel {
        match self.active_panel {
            ActivePanel::Left => &mut self.right_panel,
            ActivePanel::Right => &mut self.left_panel,
        }
    }

    async fn reload_local_files(&mut self) -> Result<()> {
        // Get path first to avoid borrow checker issues
        let path_clone =
            if let PanelType::LocalFilesystem { path } = &self.get_inactive_panel().panel_type {
                Some(path.clone())
            } else {
                None
            };

        if let Some(path) = path_clone {
            let has_parent = path.parent().is_some();
            match self.read_local_directory(&path) {
                Ok(files) => {
                    let panel = self.get_inactive_panel_mut();
                    panel
                        .list_model
                        .set_items(local_files_to_items(files, has_parent));
                }
                Err(e) => {
                    let error_msg = format!("{e}");
                    let path_display = path.display();
                    if error_msg.contains("Permission denied")
                        || error_msg.contains("permission denied")
                    {
                        self.show_error(&format!(
                            "Permission denied: Cannot reload '{path_display}'"
                        ));
                    } else {
                        self.show_error(&format!("Cannot reload '{path_display}': {e}"));
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn reload_s3_browser(&mut self) -> Result<()> {
        let panel = self.get_inactive_panel_mut();
        if let PanelType::S3Browser { prefix, .. } = &panel.panel_type {
            let prefix_clone = prefix.clone();
            if let Some(s3_manager) = &panel.s3_manager {
                let objects = s3_manager.list_objects(&prefix_clone).await?;
                panel.list_model.set_items(s3_objects_to_items(objects));
            }
        }
        Ok(())
    }

    pub async fn view_file(&mut self) -> Result<()> {
        let panel = self.get_active_panel();
        let panel_type = panel.panel_type.clone();
        let selected_index = panel.selected_index;

        match panel_type {
            PanelType::S3Browser { prefix: _, .. } => {
                let item = self.get_active_panel().list_model.get_item(selected_index);

                if let Some(PanelItem {
                    item_type: ItemType::File,
                    data: ItemData::S3Object(s3_obj),
                    name,
                    ..
                }) = item
                {
                    let key = s3_obj.key.clone();
                    let filename = name.clone();

                    if let Some(s3_manager) = &self.get_active_panel().s3_manager {
                        // Get file size first
                        match s3_manager.get_object_size(&key).await {
                            Ok(file_size) => {
                                // Initial chunk: 100KB or file size, whichever is smaller
                                let chunk_size = 100 * 1024; // 100KB
                                let load_size = if file_size < chunk_size {
                                    file_size
                                } else {
                                    chunk_size
                                };

                                match s3_manager.get_object_range(&key, 0, load_size - 1).await {
                                    Ok(bytes) => {
                                        match String::from_utf8(bytes) {
                                            Ok(content) => {
                                                self.preview_filename = filename;
                                                self.preview_content = content;
                                                self.preview_scroll_offset = 0;
                                                self.preview_file_size = file_size;
                                                self.preview_is_s3 = true;
                                                self.preview_s3_key = key;
                                                self.preview_byte_offset = load_size;
                                                self.preview_total_lines = None; // Unknown until full file loaded
                                                self.prev_screen = Some(self.screen.clone());
                                                self.screen = Screen::FilePreview;
                                            }
                                            Err(_) => {
                                                self.show_error("File is not valid UTF-8 text");
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        self.show_error(&format!("Failed to preview: {e}"));
                                    }
                                }
                            }
                            Err(e) => {
                                self.show_error(&format!("Failed to get file info: {e}"));
                            }
                        }
                    }
                }
            }
            PanelType::LocalFilesystem { path: _ } => {
                let item = self.get_active_panel().list_model.get_item(selected_index);

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

                    // Read only first 1MB to avoid loading huge files into memory
                    use std::io::Read;
                    match std::fs::File::open(&file_path) {
                        Ok(file) => {
                            let mut buffer = Vec::new();
                            let max_bytes = 1024 * 1024; // 1MB

                            match file.take(max_bytes as u64).read_to_end(&mut buffer) {
                                Ok(_) => match String::from_utf8(buffer) {
                                    Ok(content) => {
                                        self.preview_filename = file_name;
                                        self.preview_content = content;
                                        self.preview_scroll_offset = 0;
                                        self.preview_file_size = file_size as i64;
                                        self.preview_is_s3 = false;
                                        self.preview_byte_offset = max_bytes as i64;
                                        self.prev_screen = Some(self.screen.clone());
                                        self.screen = Screen::FilePreview;
                                    }
                                    Err(_) => {
                                        self.show_error("File is not valid UTF-8 text");
                                    }
                                },
                                Err(e) => {
                                    self.show_error(&format!("Failed to read file: {e}"));
                                }
                            }
                        }
                        Err(e) => {
                            self.show_error(&format!("Failed to open file: {e}"));
                        }
                    }
                }
            }
            _ => {
                self.show_error("Preview only available for files");
            }
        }

        Ok(())
    }

    pub fn show_sort_dialog(&mut self) {
        // Get current sort to pre-select it in dialog
        let current_sort = {
            let panel = self.get_active_panel();
            panel.list_model.get_current_sort()
        };

        // Map SortCriteria to dialog index (0-5)
        use crate::list_model::SortCriteria;
        self.sort_dialog_selected = match current_sort {
            SortCriteria::NameAsc => 0,
            SortCriteria::NameDesc => 1,
            SortCriteria::SizeAsc => 2,
            SortCriteria::SizeDesc => 3,
            SortCriteria::ModifiedAsc => 4,
            SortCriteria::ModifiedDesc => 5,
        };

        self.prev_screen = Some(self.screen.clone());
        self.screen = Screen::SortDialog;
    }

    pub fn apply_sort_selection(&mut self) {
        use crate::list_model::SortCriteria;

        let sort = match self.sort_dialog_selected {
            0 => SortCriteria::NameAsc,
            1 => SortCriteria::NameDesc,
            2 => SortCriteria::SizeAsc,
            3 => SortCriteria::SizeDesc,
            4 => SortCriteria::ModifiedAsc,
            5 => SortCriteria::ModifiedDesc,
            _ => SortCriteria::NameAsc,
        };

        let panel = self.get_active_panel();
        panel.list_model.set_sort(sort);
        panel.selected_index = 0;
    }

    pub fn prompt_filter(&mut self) {
        self.input_mode = InputMode::Filter;
        self.input_buffer.clear();
        self.input_prompt =
            "Filter (use * as wildcard and empty for reset) press ENTER to apply:".to_string();
        self.prev_screen = Some(self.screen.clone());
        self.screen = Screen::Input;
    }

    pub fn apply_filter(&mut self) {
        use crate::list_model::FilterCriteria;

        let pattern = self.input_buffer.trim().to_string();

        if pattern.is_empty() {
            // Clear filter
            let panel = self.get_active_panel();
            panel.list_model.set_filter(FilterCriteria::default());
            panel.selected_index = 0;
        } else {
            // Apply filter
            let filter = FilterCriteria {
                name_pattern: Some(pattern.clone()),
                show_files: true,
                show_dirs: true,
            };
            let panel = self.get_active_panel();
            panel.list_model.set_filter(filter);
            panel.selected_index = 0;
        }
    }

    pub fn prompt_create_folder(&mut self) {
        let panel_type = &self.get_active_panel().panel_type;
        if matches!(
            panel_type,
            PanelType::S3Browser { .. } | PanelType::LocalFilesystem { .. }
        ) {
            self.input_mode = InputMode::CreateFolder;
            self.input_buffer.clear();
            self.input_cursor_position = 0;
            self.input_prompt = "Folder name:".to_string();
            self.prev_screen = Some(self.screen.clone());
            self.screen = Screen::Input;
        } else {
            self.show_error("Create folder only available in S3 browser or local filesystem");
        }
    }

    pub fn prompt_rename(&mut self) {
        let panel = self.get_active_panel();
        let panel_type = panel.panel_type.clone();
        let selected_index = panel.selected_index;

        match panel_type {
            PanelType::S3Browser { .. } => {
                let item = self.get_active_panel().list_model.get_item(selected_index);
                if let Some(PanelItem {
                    data: ItemData::S3Object(s3_obj),
                    ..
                }) = item
                {
                    let key = s3_obj.key.clone();
                    self.rename_original_path = key.clone();
                    self.input_buffer = key;
                    self.input_cursor_position = self.input_buffer.len();
                    self.input_mode = InputMode::Rename;
                    self.input_prompt = "Rename to:".to_string();
                    self.prev_screen = Some(self.screen.clone());
                    self.screen = Screen::Input;
                }
            }
            PanelType::LocalFilesystem { .. } => {
                let item = self.get_active_panel().list_model.get_item(selected_index);
                if let Some(PanelItem {
                    data: ItemData::LocalFile { path, .. },
                    ..
                }) = item
                {
                    let path_str = path.display().to_string();
                    self.rename_original_path = path_str.clone();
                    self.input_buffer = path_str;
                    self.input_cursor_position = self.input_buffer.len();
                    self.input_mode = InputMode::Rename;
                    self.input_prompt = "Rename to:".to_string();
                    self.prev_screen = Some(self.screen.clone());
                    self.screen = Screen::Input;
                }
            }
            _ => {}
        }
    }

    pub async fn create_folder(&mut self) -> Result<()> {
        let folder_name = self.input_buffer.trim().to_string();
        if folder_name.is_empty() {
            self.show_error("Folder name cannot be empty");
            return Ok(());
        }

        let panel_type = self.get_active_panel().panel_type.clone();

        match panel_type {
            PanelType::S3Browser { prefix, .. } => {
                let has_s3_manager = self.get_active_panel().s3_manager.is_some();

                if has_s3_manager {
                    let folder_key = if prefix.is_empty() {
                        format!("{folder_name}/")
                    } else {
                        format!("{prefix}{folder_name}/")
                    };

                    let panel = self.get_active_panel();
                    if let Some(s3_manager) = &panel.s3_manager {
                        // Create empty object with trailing slash to represent folder
                        s3_manager.upload_empty_folder(&folder_key).await?;

                        // Reload panel
                        let objects = s3_manager.list_objects(&prefix).await?;

                        let panel = self.get_active_panel();
                        panel.list_model.set_items(s3_objects_to_items(objects));
                    }
                }
            }
            PanelType::LocalFilesystem { path } => {
                let new_folder_path = path.join(&folder_name);

                std::fs::create_dir(&new_folder_path)?;

                // Reload directory
                let has_parent = path.parent().is_some();
                if let Ok(files) = self.read_local_directory(&path) {
                    let panel = self.get_active_panel();
                    panel
                        .list_model
                        .set_items(local_files_to_items(files, has_parent));
                }
            }
            _ => {}
        }

        Ok(())
    }

    pub async fn rename_file(&mut self) -> Result<()> {
        let new_path = self.input_buffer.trim().to_string();
        let old_path = self.rename_original_path.clone();

        if new_path.is_empty() || new_path == old_path {
            return Ok(());
        }

        let panel_type = self.get_active_panel().panel_type.clone();

        match panel_type {
            PanelType::S3Browser { prefix, .. } => {
                // S3: Copy to new key, then delete old key
                if let Some(s3_manager) = &self.get_active_panel().s3_manager {
                    // Copy object to new key
                    match s3_manager.copy_object(&old_path, &new_path).await {
                        Ok(_) => {
                            // Delete old key
                            match s3_manager.delete_object(&old_path).await {
                                Ok(_) => {
                                    // Reload panel
                                    match s3_manager.list_objects(&prefix).await {
                                        Ok(objects) => {
                                            let panel = self.get_active_panel();
                                            panel
                                                .list_model
                                                .set_items(s3_objects_to_items(objects));
                                        }
                                        Err(e) => {
                                            self.show_error(&format!("Failed to reload: {e}"));
                                        }
                                    }
                                }
                                Err(e) => {
                                    let error_msg = format!("{e}");
                                    if error_msg.contains("AccessDenied") {
                                        self.show_error("Rename failed: No delete permission. Old file still exists!");
                                    } else {
                                        self.show_error(&format!("Failed to delete old file: {e}. File was copied but old file remains!"));
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            self.show_error(&format!("Rename failed: {e}"));
                        }
                    }
                }
            }
            PanelType::LocalFilesystem { path } => {
                let old_path_buf = std::path::PathBuf::from(&old_path);
                let new_path_buf = std::path::PathBuf::from(&new_path);

                // Try native rename (works if same filesystem)
                match std::fs::rename(&old_path_buf, &new_path_buf) {
                    Ok(_) => {
                        // Reload panel
                        let has_parent = path.parent().is_some();
                        if let Ok(files) = self.read_local_directory(&path) {
                            let panel = self.get_active_panel();
                            panel
                                .list_model
                                .set_items(local_files_to_items(files, has_parent));
                        }
                    }
                    Err(e) => {
                        self.show_error(&format!("Rename failed: {e}"));
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

                    // Show confirmation dialog
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

                    // Show confirmation dialog
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
        let panel = self.get_active_panel();
        let panel_type = panel.panel_type.clone();

        match panel_type {
            PanelType::S3Browser { prefix, .. } => {
                let key = self.delete_confirmation_path.clone();

                if let Some(s3_manager) = &self.get_active_panel().s3_manager {
                    match s3_manager.delete_object(&key).await {
                        Ok(_) => {
                            // Reload current panel
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
                let file_path = std::path::PathBuf::from(self.delete_confirmation_path.clone());
                let is_dir = self.delete_confirmation_is_dir;

                let result = if is_dir {
                    std::fs::remove_dir_all(&file_path)
                } else {
                    std::fs::remove_file(&file_path)
                };

                match result {
                    Ok(_) => {
                        // Reload current panel
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
