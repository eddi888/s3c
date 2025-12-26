use super::PreviewSource;

/// Model für Bild-Vorschau
#[derive(Debug, Clone)]
pub struct ImagePreview {
    pub filename: String,
    pub source: PreviewSource,
    pub render_mode: ImageRenderMode,
    pub dimensions: Option<(u32, u32)>,
    pub ascii_data: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImageRenderMode {
    Ascii,
    #[allow(dead_code)]
    NotSupported,
}

impl ImagePreview {
    pub fn new(
        filename: String,
        source: PreviewSource,
        render_mode: ImageRenderMode,
        ascii_data: Option<String>,
    ) -> Self {
        Self {
            filename,
            source,
            render_mode,
            dimensions: None,
            ascii_data,
        }
    }

    pub fn source_display(&self) -> String {
        match &self.source {
            PreviewSource::LocalFile { .. } => "Local".to_string(),
            PreviewSource::S3Object { bucket, .. } => format!("S3: {bucket}"),
        }
    }
}
