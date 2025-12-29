use crate::operations::s3::S3Object;

/// All possible actions/events in the application following The Elm Architecture (TEA)
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Message {
    // ===== Application Control =====
    Quit,
    NoOp,

    // ===== Navigation =====
    NavigateUp,
    NavigateDown,
    NavigatePageUp,
    NavigatePageDown,
    NavigateHome,
    NavigateEnd,
    EnterSelected,
    GoBack,

    // ===== Panel Management =====
    SwitchPanel,
    ToggleLocalFilesystem,

    // ===== UI State Changes =====
    ShowHelp,
    ShowSortDialog,
    ShowFilterPrompt,
    ShowConfigForm,
    ShowProfileConfigForm,
    ShowCreateFolderPrompt,
    ShowRenamePrompt,
    FilePreviewUp,
    FilePreviewDown,
    FilePreviewPageUp,
    FilePreviewPageDown,
    FilePreviewHome,
    FilePreviewEnd,
    LoadMoreFileContent,
    LoadPreviousFileContent,
    ToggleAdvancedMode,

    // ===== Sort Dialog =====
    SortDialogUp,
    SortDialogDown,
    ApplySort,

    // ===== Filter =====
    ApplyFilter {
        pattern: String,
    },

    // ===== File Operations =====
    CancelTransfer,
    ClearCompletedTransfers,
    DeleteFromQueue,
    QueueNavigateUp,
    QueueNavigateDown,
    ToggleQueueFocus,

    // ===== Delete Confirmation =====
    ShowDeleteConfirmation {
        path: String,
        name: String,
        is_dir: bool,
    },
    DeleteConfirmationLeft,
    DeleteConfirmationRight,
    ConfirmDelete,

    // ===== Config Form =====
    ConfigFormUp,
    ConfigFormDown,
    ConfigFormLeft,
    ConfigFormRight,
    ConfigFormHome,
    ConfigFormEnd,
    ConfigFormDelete,
    ConfigFormChar {
        c: char,
    },
    ConfigFormBackspace,
    ConfigFormAddRole,
    ConfigFormRemoveRole,
    SaveConfigForm,
    EditBucketConfig,
    DeleteBucketConfig,

    // ===== Profile Form =====
    ProfileFormUp,
    ProfileFormDown,
    ProfileFormLeft,
    ProfileFormRight,
    ProfileFormHome,
    ProfileFormEnd,
    ProfileFormDelete,
    ProfileFormChar {
        c: char,
    },
    ProfileFormBackspace,
    SaveProfileConfig,

    // ===== Input Mode =====
    InputChar {
        c: char,
        ctrl: bool,
    },
    InputBackspace,
    InputDelete,
    InputLeft,
    InputRight,
    InputHome,
    InputEnd,
    InputSubmit,
    InputCancel,

    // ===== File Operations =====
    CreateFolder {
        name: String,
    },
    RenameFile {
        old_path: String,
        new_path: String,
    },
    DeleteFile,
    CopyToOtherPanel,
    ViewFile,

    // ===== Async Operation Results =====
    S3ListComplete {
        objects: Vec<S3Object>,
    },
    S3OperationSuccess {
        message: String,
    },
    S3OperationFailed {
        error: String,
    },
    LocalOperationSuccess {
        message: String,
    },
    LocalOperationFailed {
        error: String,
    },
    UploadProgress {
        transferred: u64,
        total: u64,
    },
    UploadComplete {
        filename: String,
    },
    DownloadComplete {
        filename: String,
    },

    // ===== Script Execution =====
    RunSetupScript {
        script_path: String,
        profile: String,
        bucket: Option<String>,
    },
    ScriptCompleted {
        success: bool,
        profile: String,
        bucket: Option<String>,
    },

    // ===== Error/Success Messages =====
    ShowError {
        message: String,
    },
    ShowSuccess {
        message: String,
    },
    Clear,
}
