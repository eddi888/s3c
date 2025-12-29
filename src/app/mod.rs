pub mod converters;
pub mod handlers;
pub mod navigation;
mod state;
mod update;

pub use state::*;
pub use update::update;

use crate::models::config::ConfigManager;
use crate::models::list::PanelListModel;
use crate::operations::s3::S3Manager;
use crate::operations::FileOperation;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum PanelType {
    ModeSelection,
    DriveSelection,
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
    FileContentPreview,
    ImagePreview,
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
    pub prev_screen: Option<Screen>,
    pub should_quit: bool,
    pub advanced_mode: bool,
    pub app_title: String,

    // UI Messages
    pub error_message: String,
    pub success_message: String,

    // Consolidated UI State
    pub config_form: ConfigFormState,
    pub profile_form: ProfileFormState,
    pub file_content_preview: Option<crate::models::preview::FileContentPreview>,
    pub image_preview: Option<crate::models::preview::ImagePreview>,
    pub delete_confirmation: DeleteConfirmationState,
    pub input: InputState,
    pub sort_dialog: SortDialogState,
    pub script: ScriptState,

    // File Operations
    pub file_operation_queue: Option<FileOperation>,
    pub background_transfer_task: Option<BackgroundTransferTask>,
}

/// Background file transfer task (non-blocking)
pub struct BackgroundTransferTask {
    pub task_handle: tokio::task::JoinHandle<anyhow::Result<()>>,
    pub progress_counter: std::sync::Arc<std::sync::atomic::AtomicU64>,
    pub operation: std::sync::Arc<tokio::sync::Mutex<FileOperation>>,
}

impl Panel {
    pub fn new_mode_selection() -> Self {
        Self {
            panel_type: PanelType::ModeSelection,
            selected_index: 0,
            scroll_offset: 0,
            visible_height: 10,
            list_model: PanelListModel::empty(),
            s3_manager: None,
        }
    }

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
                path: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            },
            selected_index: 0,
            scroll_offset: 0,
            visible_height: 10,
            list_model: PanelListModel::empty(),
            s3_manager: None,
        }
    }
}

impl App {
    pub fn new() -> anyhow::Result<Self> {
        let config_manager = ConfigManager::new()?;

        let mut app = Self {
            config_manager,
            screen: Screen::DualPanel,
            left_panel: Panel::new_mode_selection(),
            right_panel: Panel::new_local_filesystem(),
            active_panel: ActivePanel::Left,
            prev_screen: None,
            should_quit: false,
            advanced_mode: false,
            app_title: "s3c - S3 Commander".to_string(),
            error_message: String::new(),
            success_message: String::new(),
            config_form: ConfigFormState::default(),
            profile_form: ProfileFormState::default(),
            file_content_preview: None,
            image_preview: None,
            delete_confirmation: DeleteConfirmationState::default(),
            input: InputState::default(),
            sort_dialog: SortDialogState::default(),
            script: ScriptState::default(),
            file_operation_queue: None,
            background_transfer_task: None,
        };

        // Load local files for right panel
        if let PanelType::LocalFilesystem { path } = &app.right_panel.panel_type {
            let path_clone = path.clone();
            let has_parent = path_clone.parent().is_some();
            if let Ok(files) = navigation::read_local_directory(&path_clone) {
                app.right_panel
                    .list_model
                    .set_items(converters::local_files_to_items(files, has_parent));
            }
        }

        // Load mode selection items for left panel
        app.left_panel
            .list_model
            .set_items(converters::modes_to_items());

        Ok(app)
    }

    pub fn get_active_panel(&mut self) -> &mut Panel {
        match self.active_panel {
            ActivePanel::Left => &mut self.left_panel,
            ActivePanel::Right => &mut self.right_panel,
        }
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

    pub fn show_error(&mut self, message: &str) {
        self.error_message = message.to_string();
    }

    pub fn show_success(&mut self, message: &str) {
        self.success_message = message.to_string();
    }

    pub fn switch_panel(&mut self) {
        self.active_panel = match self.active_panel {
            ActivePanel::Left => ActivePanel::Right,
            ActivePanel::Right => ActivePanel::Left,
        };
    }

    pub fn go_back(&mut self) {
        if let Some(prev) = self.prev_screen.take() {
            self.screen = prev;
        } else {
            self.screen = Screen::DualPanel;
        }
    }
}
