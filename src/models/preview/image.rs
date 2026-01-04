use super::PreviewSource;

/// Model für Bild-Vorschau
#[derive(Debug, Clone)]
pub struct ImagePreview {
    pub filename: String,
    pub source: PreviewSource,
    pub image_data: Vec<u8>,
    pub dimensions: Option<(u32, u32)>,
}

impl ImagePreview {
    pub fn new(
        filename: String,
        source: PreviewSource,
        image_data: Vec<u8>,
        dimensions: Option<(u32, u32)>,
    ) -> Self {
        Self {
            filename,
            source,
            image_data,
            dimensions,
        }
    }

    pub fn source_display(&self) -> String {
        match &self.source {
            PreviewSource::LocalFile { .. } => "Local".to_string(),
            PreviewSource::S3Object { bucket, .. } => format!("S3: {bucket}"),
        }
    }
}
