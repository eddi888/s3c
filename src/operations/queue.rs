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
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum OperationStatus {
    Pending,
    InProgress,
    Completed,
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
