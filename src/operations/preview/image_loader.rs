use crate::models::preview::{ImagePreview, PreviewSource};
use anyhow::Result;
use image::GenericImageView;
use std::path::Path;

/// Lädt Bild von Local oder S3
pub async fn load_image(source: PreviewSource) -> Result<ImagePreview> {
    match source {
        PreviewSource::LocalFile { path } => {
            let path_clone = path.clone();
            load_local_image(&path, PreviewSource::LocalFile { path: path_clone }).await
        }
        PreviewSource::S3Object { .. } => {
            // TODO: Download to temp file, dann load_local_image
            Err(anyhow::anyhow!("S3 image preview not yet implemented"))
        }
    }
}

async fn load_local_image(path: &str, source: PreviewSource) -> Result<ImagePreview> {
    let path_obj = Path::new(path);
    let filename = path_obj
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Load image with image crate
    let img = image::open(path)?;
    let dimensions = img.dimensions();

    // Encode to PNG bytes for ratatui-image
    let mut image_data = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut image_data),
        image::ImageFormat::Png,
    )?;

    Ok(ImagePreview::new(
        filename,
        source,
        image_data,
        Some(dimensions),
    ))
}

/// Prüft ob Datei ein Bild ist
pub fn is_image_file(filename: &str) -> bool {
    let ext = filename.split('.').next_back().unwrap_or("").to_lowercase();
    matches!(
        ext.as_str(),
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp"
    )
}
