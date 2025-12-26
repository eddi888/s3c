/// Model für Text-basierte Datei-Vorschau (CSV, JSON, TXT, etc.)
#[derive(Debug, Clone)]
pub struct FileContentPreview {
    pub filename: String,
    pub content: String,
    pub file_size: i64,
    pub source: PreviewSource,
    pub scroll_offset: usize,
    #[allow(dead_code)]
    pub byte_offset: i64,
    pub preview_mode: PreviewMode,
    pub content_start_offset: i64, // Byte offset where current content starts in file
    pub chunk_load_count: u32,     // Number of chunks loaded (incremented on each load)
    pub viewport_width: u16,       // Width of viewport for calculating visual line wraps
}

#[derive(Debug, Clone, PartialEq)]
pub enum PreviewMode {
    Forward,  // Normal mode: Line X / Total
    Backward, // Tail mode: Line -X / LAST
}

#[derive(Debug, Clone)]
pub enum PreviewSource {
    LocalFile {
        path: String,
    },
    S3Object {
        #[allow(dead_code)]
        key: String,
        bucket: String,
    },
}

impl FileContentPreview {
    pub fn new(filename: String, content: String, file_size: i64, source: PreviewSource) -> Self {
        let byte_offset = content.len() as i64;
        Self {
            filename,
            content,
            file_size,
            source,
            scroll_offset: 0,
            byte_offset,
            preview_mode: PreviewMode::Forward,
            content_start_offset: 0,
            chunk_load_count: 1, // Initial load counts as 1
            viewport_width: 80,  // Default, will be updated by UI
        }
    }

    pub fn source_display(&self) -> String {
        match &self.source {
            PreviewSource::LocalFile { .. } => "Local".to_string(),
            PreviewSource::S3Object { bucket, .. } => format!("S3: {bucket}"),
        }
    }

    /// Berechnet die Gesamtzahl der visuellen Zeilen (inkl. Umbrüche)
    pub fn calculate_visual_line_count(&self) -> usize {
        if self.viewport_width == 0 {
            return self.content.lines().count();
        }

        let width = self.viewport_width as usize;
        let mut visual_lines = 0;

        for line in self.content.lines() {
            if line.is_empty() {
                visual_lines += 1;
            } else {
                // Berechne wie viele visuelle Zeilen diese Zeile benötigt
                let line_len = line.chars().count();
                visual_lines += line_len.div_ceil(width);
            }
        }

        visual_lines.max(1) // Mindestens 1 Zeile
    }

    /// Gibt visuelle Zeilen zurück (content mit Umbrüchen basierend auf viewport_width)
    pub fn get_visual_lines(&self) -> Vec<String> {
        if self.viewport_width == 0 {
            return self.content.lines().map(|s| s.to_string()).collect();
        }

        let width = self.viewport_width as usize;
        let mut visual_lines = Vec::new();

        for line in self.content.lines() {
            if line.is_empty() {
                visual_lines.push(String::new());
            } else {
                // Zeile in chunks der Breite width aufteilen
                let chars: Vec<char> = line.chars().collect();
                for chunk in chars.chunks(width) {
                    visual_lines.push(chunk.iter().collect());
                }
            }
        }

        if visual_lines.is_empty() {
            visual_lines.push(String::new());
        }

        visual_lines
    }
}
