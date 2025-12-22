use crate::config::ConfigManager;
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
    pub s3_objects: Vec<S3Object>,
    pub local_files: Vec<LocalFile>,
    pub s3_manager: Option<S3Manager>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    DualPanel,
    ConfigForm,
    ProfileConfigForm,
    FilePreview,
    Input,
    Error,
    Success,
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
    pub profile_form_name: String,
    pub profile_form_description: String,
    pub profile_form_setup_script: String,
    pub profile_form_field: usize,
    pub preview_content: String,
    pub preview_filename: String,
    pub input_mode: InputMode,
    pub pending_script: Option<String>,
    pub pending_script_profile: Option<String>,
    pub pending_script_bucket: Option<Option<String>>,
    pub needs_terminal_for_script: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    None,
    CreateFolder,
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
            s3_objects: Vec::new(),
            local_files: Vec::new(),
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
            s3_objects: Vec::new(),
            local_files: Vec::new(),
            s3_manager: None,
        }
    }
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
            profile_form_name: String::new(),
            profile_form_description: String::new(),
            profile_form_setup_script: String::new(),
            profile_form_field: 0,
            preview_content: String::new(),
            preview_filename: String::new(),
            input_mode: InputMode::None,
            pending_script: None,
            pending_script_profile: None,
            pending_script_bucket: None,
            needs_terminal_for_script: false,
        };

        // Load local files for right panel
        if let PanelType::LocalFilesystem { path } = &app.right_panel.panel_type {
            let path_clone = path.clone();
            if let Ok(files) = app.read_local_directory(&path_clone) {
                app.right_panel.local_files = files;
            }
        }

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

    fn get_panel_item_count(&self) -> usize {
        let panel = match self.active_panel {
            ActivePanel::Left => &self.left_panel,
            ActivePanel::Right => &self.right_panel,
        };

        match &panel.panel_type {
            PanelType::ProfileList => self.config_manager.aws_profiles.len(),
            PanelType::BucketList { profile } => {
                // Always has ".." entry + buckets
                self.config_manager.get_buckets_for_profile(profile).len() + 1
            }
            PanelType::S3Browser { .. } => {
                // Always has ".." entry + objects
                panel.s3_objects.len() + 1
            }
            PanelType::LocalFilesystem { path } => {
                let base = panel.local_files.len();
                if path.parent().is_some() {
                    base + 1
                } else {
                    base
                }
            }
        }
    }

    pub async fn enter_selected(&mut self) -> Result<()> {
        let panel_type = self.get_active_panel().panel_type.clone();
        let selected_index = self.get_active_panel().selected_index;

        match panel_type {
            PanelType::ProfileList => {
                if let Some(profile) = self
                    .config_manager
                    .aws_profiles
                    .get(selected_index)
                    .cloned()
                {
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
                    let panel = self.get_active_panel();
                    panel.panel_type = PanelType::BucketList { profile };
                    panel.selected_index = 0;
                }
            }
            PanelType::BucketList { profile: _ } => {
                // Check if ".." parent directory is selected
                if selected_index == 0 {
                    // Go back to ProfileList
                    let panel = self.get_active_panel();
                    panel.panel_type = PanelType::ProfileList;
                    panel.selected_index = 0;
                } else {
                    // Adjust index for actual buckets (skip ".." at index 0)
                    let bucket_index = selected_index - 1;
                    let profile_clone = if let PanelType::BucketList { profile } =
                        &self.get_active_panel().panel_type
                    {
                        profile.clone()
                    } else {
                        return Ok(());
                    };

                    let buckets = self.config_manager.get_buckets_for_profile(&profile_clone);
                    if let Some(bucket_config) = buckets.get(bucket_index) {
                        self.load_s3_bucket(profile_clone, bucket_config.name.clone())
                            .await?;
                    }
                }
            }
            PanelType::S3Browser {
                profile,
                bucket,
                prefix,
            } => {
                // Check if ".." is selected (always at index 0)
                if selected_index == 0 {
                    if prefix.is_empty() {
                        // At root - go back to BucketList
                        let panel = self.get_active_panel();
                        panel.panel_type = PanelType::BucketList { profile };
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
                } else {
                    // Adjust index for actual objects (skip ".." at index 0)
                    let obj_index = selected_index - 1;
                    let obj_key = self
                        .get_active_panel()
                        .s3_objects
                        .get(obj_index)
                        .filter(|obj| obj.is_prefix)
                        .map(|obj| obj.key.clone());

                    if let Some(key) = obj_key {
                        self.navigate_to_s3_prefix(profile, bucket, key).await?;
                    }
                }
            }
            PanelType::LocalFilesystem { path } => {
                // Check if ".." parent directory is selected
                if path.parent().is_some() && selected_index == 0 {
                    // Navigate to parent directory
                    if let Some(parent) = path.parent() {
                        self.navigate_to_local_dir(parent.to_path_buf()).await?;
                    }
                } else {
                    // Adjust index for actual files (skip ".." if present)
                    let file_index = if path.parent().is_some() {
                        selected_index - 1
                    } else {
                        selected_index
                    };
                    let file_path = self
                        .get_active_panel()
                        .local_files
                        .get(file_index)
                        .filter(|file| file.is_dir)
                        .map(|file| file.path.clone());

                    if let Some(target_path) = file_path {
                        self.navigate_to_local_dir(target_path).await?;
                    }
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
                panel.s3_objects = objects;
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
            panel.s3_objects = objects;
            panel.selected_index = 0;
        }
        Ok(())
    }

    async fn navigate_to_local_dir(&mut self, path: PathBuf) -> Result<()> {
        match self.read_local_directory(&path) {
            Ok(files) => {
                let panel = self.get_active_panel();
                panel.panel_type = PanelType::LocalFilesystem { path };
                panel.local_files = files;
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
            let panel = self.get_active_panel();
            *panel = Panel::new_profile_list();
        } else {
            let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
            if let Ok(files) = self.read_local_directory(&home) {
                let panel = self.get_active_panel();
                panel.panel_type = PanelType::LocalFilesystem { path: home };
                panel.local_files = files;
                panel.selected_index = 0;
            }
        }
    }

    pub fn show_error(&mut self, message: &str) {
        self.error_message = message.to_string();
        self.prev_screen = Some(self.screen.clone());
        self.screen = Screen::Error;
    }

    pub fn show_success(&mut self, message: &str) {
        self.success_message = message.to_string();
        self.prev_screen = Some(self.screen.clone());
        self.screen = Screen::Success;
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
            self.prev_screen = Some(self.screen.clone());
            self.screen = Screen::ConfigForm;
        }
    }

    pub fn edit_bucket_config(&mut self) {
        // Load existing bucket config for editing
        let panel_type = self.get_active_panel().panel_type.clone();
        let selected_index = self.get_active_panel().selected_index;

        if let PanelType::BucketList { profile } = panel_type {
            // Skip if ".." is selected (index 0)
            if selected_index == 0 {
                return;
            }

            let buckets = self.config_manager.get_buckets_for_profile(&profile);

            // Adjust index for actual buckets (skip ".." at index 0)
            let bucket_index = selected_index - 1;
            if let Some(bucket_config) = buckets.get(bucket_index) {
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
                self.prev_screen = Some(self.screen.clone());
                self.screen = Screen::ConfigForm;
            }
        }
    }

    pub fn show_profile_config_form(&mut self) {
        // Get current profile from active panel
        if let PanelType::ProfileList = self.get_active_panel().panel_type {
            let selected_index = self.get_active_panel().selected_index;
            if let Some(profile) = self
                .config_manager
                .aws_profiles
                .get(selected_index)
                .cloned()
            {
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
            // Skip if ".." is selected (index 0)
            if selected_index == 0 {
                return Ok(());
            }

            let buckets = self.config_manager.get_buckets_for_profile(&profile);

            // Adjust index for actual buckets (skip ".." at index 0)
            let bucket_index = selected_index - 1;
            if let Some(bucket_config) = buckets.get(bucket_index) {
                let bucket_name = bucket_config.name.clone();
                self.config_manager
                    .remove_bucket_from_profile(&profile, &bucket_name)?;

                // Update selected index if needed
                let panel = self.get_active_panel();
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
            (PanelType::S3Browser { prefix, .. }, PanelType::LocalFilesystem { path }) => {
                let offset = if prefix.is_empty() { 0 } else { 1 };
                let actual_index = if prefix.is_empty() {
                    source_selected
                } else if source_selected == 0 {
                    return Ok(()); // Skip ".." entry
                } else {
                    source_selected - offset
                };

                if let Some(s3_obj) = source_panel.s3_objects.get(actual_index) {
                    if s3_obj.is_prefix {
                        self.show_error("Cannot copy directories");
                        return Ok(());
                    }

                    let filename = s3_obj.key.split('/').next_back().unwrap_or(&s3_obj.key);
                    let local_path = path.join(filename);

                    if let Some(s3_manager) = &source_panel.s3_manager {
                        match s3_manager.download_file(&s3_obj.key, &local_path).await {
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
            (PanelType::LocalFilesystem { path }, PanelType::S3Browser { prefix, .. }) => {
                let offset = if path.parent().is_some() { 1 } else { 0 };
                let actual_index = if offset > 0 && source_selected == 0 {
                    return Ok(()); // Skip ".." entry
                } else {
                    source_selected.saturating_sub(offset)
                };

                if let Some(local_file) = source_panel.local_files.get(actual_index) {
                    if local_file.is_dir {
                        self.show_error("Cannot copy directories");
                        return Ok(());
                    }

                    // Default S3 key
                    let default_s3_key = if prefix.is_empty() {
                        local_file.name.clone()
                    } else {
                        let name = &local_file.name;
                        format!("{prefix}{name}")
                    };

                    // Prompt user for upload path
                    self.input_mode = InputMode::UploadPath {
                        local_file_path: local_file.path.clone(),
                        local_file_name: local_file.name.clone(),
                    };
                    self.input_buffer = default_s3_key;
                    self.input_prompt = "Upload to S3 path:".to_string();
                    self.prev_screen = Some(self.screen.clone());
                    self.screen = Screen::Input;
                }
            }

            _ => {
                self.show_error("Copy only supported between S3 and Local filesystem");
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
            match self.read_local_directory(&path) {
                Ok(files) => {
                    let panel = self.get_inactive_panel_mut();
                    panel.local_files = files;
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
                panel.s3_objects = objects;
            }
        }
        Ok(())
    }

    pub async fn view_file(&mut self) -> Result<()> {
        let panel = self.get_active_panel();
        let panel_type = panel.panel_type.clone();
        let selected_index = panel.selected_index;

        match panel_type {
            PanelType::S3Browser { prefix, .. } => {
                let offset = if prefix.is_empty() { 0 } else { 1 };
                let actual_index = if prefix.is_empty() {
                    selected_index
                } else if selected_index == 0 {
                    return Ok(()); // Skip ".." entry
                } else {
                    selected_index - offset
                };

                if let Some(s3_obj) = self.get_active_panel().s3_objects.get(actual_index) {
                    if s3_obj.is_prefix {
                        self.show_error("Cannot preview directories");
                        return Ok(());
                    }

                    let key = s3_obj.key.clone();
                    let filename = key.split('/').next_back().unwrap_or(&key).to_string();

                    if let Some(s3_manager) = &self.get_active_panel().s3_manager {
                        // Preview max 1MB of content
                        match s3_manager.get_object_content(&key, 1024 * 1024).await {
                            Ok(content) => {
                                self.preview_filename = filename;
                                self.preview_content = content;
                                self.prev_screen = Some(self.screen.clone());
                                self.screen = Screen::FilePreview;
                            }
                            Err(e) => {
                                self.show_error(&format!("Failed to preview: {e}"));
                            }
                        }
                    }
                }
            }
            PanelType::LocalFilesystem { path } => {
                let offset = if path.parent().is_some() { 1 } else { 0 };
                let actual_index = if offset > 0 && selected_index == 0 {
                    return Ok(()); // Skip ".." entry
                } else {
                    selected_index.saturating_sub(offset)
                };

                if let Some(local_file) = self.get_active_panel().local_files.get(actual_index) {
                    if local_file.is_dir {
                        self.show_error("Cannot preview directories");
                        return Ok(());
                    }

                    // Read only first 1MB to avoid loading huge files into memory
                    use std::io::Read;
                    match std::fs::File::open(&local_file.path) {
                        Ok(file) => {
                            let mut buffer = Vec::new();
                            let max_bytes = 1024 * 1024; // 1MB

                            match file.take(max_bytes as u64).read_to_end(&mut buffer) {
                                Ok(_) => match String::from_utf8(buffer) {
                                    Ok(content) => {
                                        self.preview_filename = local_file.name.clone();
                                        self.preview_content = content;
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

    pub fn prompt_create_folder(&mut self) {
        if matches!(
            self.get_active_panel().panel_type,
            PanelType::S3Browser { .. }
        ) {
            self.input_mode = InputMode::CreateFolder;
            self.input_buffer.clear();
            self.input_prompt = "Folder name:".to_string();
            self.prev_screen = Some(self.screen.clone());
            self.screen = Screen::Input;
        } else {
            self.show_error("Create folder only available in S3 browser");
        }
    }

    pub async fn create_folder(&mut self) -> Result<()> {
        let folder_name = self.input_buffer.trim().to_string();
        if folder_name.is_empty() {
            self.show_error("Folder name cannot be empty");
            return Ok(());
        }

        // Get prefix and check if we have s3_manager
        let (prefix_clone, has_s3_manager) = {
            let panel = self.get_active_panel();
            if let PanelType::S3Browser { prefix, .. } = &panel.panel_type {
                (Some(prefix.clone()), panel.s3_manager.is_some())
            } else {
                (None, false)
            }
        };

        if let Some(prefix) = prefix_clone {
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

                    self.show_success(&format!("Created folder: {folder_name}"));

                    let panel = self.get_active_panel();
                    panel.s3_objects = objects;
                }
            }
        }

        Ok(())
    }

    pub async fn delete_file(&mut self) -> Result<()> {
        let panel = self.get_active_panel();
        let panel_type = panel.panel_type.clone();
        let selected_index = panel.selected_index;

        match panel_type {
            PanelType::S3Browser { prefix, .. } => {
                let offset = if prefix.is_empty() { 0 } else { 1 };
                let actual_index = if prefix.is_empty() {
                    selected_index
                } else if selected_index == 0 {
                    return Ok(()); // Skip ".." entry
                } else {
                    selected_index - offset
                };

                // Get object key (allows deleting both files and folders)
                let s3_obj_key = self
                    .get_active_panel()
                    .s3_objects
                    .get(actual_index)
                    .map(|s3_obj| s3_obj.key.clone());

                if let Some(key) = s3_obj_key {
                    if let Some(s3_manager) = &self.get_active_panel().s3_manager {
                        match s3_manager.delete_object(&key).await {
                            Ok(_) => {
                                // Reload current panel
                                let prefix_clone = prefix.clone();
                                match s3_manager.list_objects(&prefix_clone).await {
                                    Ok(objects) => {
                                        self.show_success(&format!("Deleted: {key}"));

                                        let panel = self.get_active_panel();
                                        panel.s3_objects = objects;
                                        if panel.selected_index > 0 {
                                            panel.selected_index -= 1;
                                        }
                                    }
                                    Err(e) => {
                                        self.show_error(&format!("Failed to reload: {e}"));
                                    }
                                }
                            }
                            Err(e) => {
                                let error_msg = format!("{e}");
                                if error_msg.contains("AccessDenied") {
                                    self.show_error("Permission denied: You don't have delete rights for this object");
                                } else {
                                    self.show_error(&format!("Delete failed: {e}"));
                                }
                            }
                        }
                    }
                }
            }
            PanelType::LocalFilesystem { path } => {
                let offset = if path.parent().is_some() { 1 } else { 0 };
                let actual_index = if offset > 0 && selected_index == 0 {
                    return Ok(()); // Skip ".." entry
                } else {
                    selected_index.saturating_sub(offset)
                };

                if let Some(local_file) = self
                    .get_active_panel()
                    .local_files
                    .get(actual_index)
                    .cloned()
                {
                    if local_file.is_dir {
                        self.show_error("Cannot delete directories");
                        return Ok(());
                    }

                    match std::fs::remove_file(&local_file.path) {
                        Ok(_) => {
                            // Reload current panel
                            let path_clone = path.clone();
                            match self.read_local_directory(&path_clone) {
                                Ok(files) => {
                                    let name = &local_file.name;
                                    self.show_success(&format!("Deleted: {name}"));

                                    let panel = self.get_active_panel();
                                    panel.local_files = files;
                                    if panel.selected_index > 0 {
                                        panel.selected_index -= 1;
                                    }
                                }
                                Err(e) => {
                                    self.show_error(&format!("Failed to reload: {e}"));
                                }
                            }
                        }
                        Err(e) => {
                            let error_msg = format!("{e}");
                            if error_msg.contains("permission denied")
                                || error_msg.contains("Permission denied")
                            {
                                self.show_error(
                                    "Permission denied: You don't have delete rights for this file",
                                );
                            } else {
                                self.show_error(&format!("Delete failed: {e}"));
                            }
                        }
                    }
                }
            }
            _ => {
                self.show_error("Delete only available for files");
            }
        }

        Ok(())
    }
}
