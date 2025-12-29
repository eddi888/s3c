#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum OperationType {
    Upload,
    Download,
    Copy,
    Rename,
}

#[derive(Debug, Clone)]
pub struct FileOperation {
    pub operation_type: OperationType,
    pub source: String,
    pub destination: String,
    pub total_size: u64,
    pub transferred: u64,
    pub status: OperationStatus,
    // S3 credentials info for queued transfers
    pub profile: Option<String>, // Source profile (Download/Upload) or Source profile (S3→S3)
    pub bucket: Option<String>,  // Source bucket (Download/Upload) or Source bucket (S3→S3)
    #[allow(dead_code)] // Reserved for future queued S3→S3 copy feature
    pub dest_profile: Option<String>, // Destination profile (S3→S3 only)
    #[allow(dead_code)] // Reserved for future queued S3→S3 copy feature
    pub dest_bucket: Option<String>, // Destination bucket (S3→S3 only)
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum OperationStatus {
    Pending,
    InProgress,
    Completed,
    Cancelled,
    Failed(String),
}

impl FileOperation {
    pub fn progress_percentage(&self) -> u16 {
        if self.total_size == 0 {
            0
        } else {
            ((self.transferred as f64 / self.total_size as f64) * 100.0) as u16
        }
    }
}
