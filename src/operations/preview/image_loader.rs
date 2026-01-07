use crate::models::preview::{ImagePreview, PreviewSource};
use anyhow::Result;
use image::GenericImageView;
use std::path::Path;

/// Lädt Bild von Local (S3 requires S3Manager, use load_s3_image)
pub async fn load_image(source: PreviewSource) -> Result<ImagePreview> {
    match source {
        PreviewSource::LocalFile { path } => {
            let path_clone = path.clone();
            load_local_image(&path, PreviewSource::LocalFile { path: path_clone }).await
        }
        PreviewSource::S3Object { .. } => Err(anyhow::anyhow!(
            "S3 images require S3Manager, use load_s3_image"
        )),
    }
}

/// Lädt S3-Bild mit S3Manager
pub async fn load_s3_image(
    key: &str,
    bucket: &str,
    s3_manager: &crate::operations::s3::S3Manager,
) -> Result<ImagePreview> {
    let filename = key.split('/').next_back().unwrap_or("unknown").to_string();

    // Get total object size
    let file_size = s3_manager.get_object_size(key).await?;

    // Download entire image (images need to be complete to decode)
    let bytes = s3_manager.get_object_range(key, 0, file_size - 1).await?;

    // Load image from memory
    let img = image::load_from_memory(&bytes)?;
    let dimensions = img.dimensions();

    // Encode to PNG for ratatui-image
    let mut image_data = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut image_data),
        image::ImageFormat::Png,
    )?;

    Ok(ImagePreview::new(
        filename,
        PreviewSource::S3Object {
            key: key.to_string(),
            bucket: bucket.to_string(),
        },
        image_data,
        Some(dimensions),
    ))
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
