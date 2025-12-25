use std::path::PathBuf;

/// State for the bucket/profile configuration form
#[derive(Debug, Clone, Default)]
pub struct ConfigFormState {
    pub profile: String,
    pub bucket: String,
    pub description: String,
    pub region: String,
    pub roles: Vec<String>,
    pub field: usize,
    pub cursor: usize,
}

/// State for the profile configuration form
#[derive(Debug, Clone, Default)]
pub struct ProfileFormState {
    pub name: String,
    pub description: String,
    pub setup_script: String,
    pub field: usize,
    pub cursor: usize,
}

/// State for file preview screen
#[derive(Debug, Clone, Default)]
pub struct PreviewState {
    pub content: String,
    pub filename: String,
    pub scroll_offset: usize,
    pub file_size: i64,
    pub is_s3: bool,
    pub s3_key: String,
    pub byte_offset: i64,
    pub total_lines: Option<usize>,
}

/// State for delete confirmation dialog
#[derive(Debug, Clone, Default)]
pub struct DeleteConfirmationState {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub button: usize,
}

/// State for generic input dialog
#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    None,
    CreateFolder,
    Filter,
    Rename,
    UploadPath {
        local_file_path: PathBuf,
        local_file_name: String,
    },
}

impl Default for InputMode {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Default)]
pub struct InputState {
    pub mode: InputMode,
    pub buffer: String,
    pub prompt: String,
    pub cursor_position: usize,
    pub rename_original_path: String,
}

/// State for sort dialog
#[derive(Debug, Clone, Default)]
pub struct SortDialogState {
    pub selected: usize,
}

/// State for pending script execution
#[derive(Debug, Clone, Default)]
pub struct ScriptState {
    pub pending_script: Option<String>,
    pub pending_profile: Option<String>,
    pub pending_bucket: Option<Option<String>>,
    pub needs_terminal: bool,
}
