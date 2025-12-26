use crate::models::preview::{ImagePreview, ImageRenderMode, PreviewSource};
use anyhow::Result;
use std::path::Path;

/// Lädt Bild von Local oder S3
pub async fn load_image(source: PreviewSource) -> Result<ImagePreview> {
    let render_mode = detect_terminal_capabilities();

    match source {
        PreviewSource::LocalFile { path } => {
            let path_clone = path.clone();
            load_local_image(
                &path,
                render_mode,
                PreviewSource::LocalFile { path: path_clone },
            )
            .await
        }
        PreviewSource::S3Object { .. } => {
            // TODO: Download to temp file, dann load_local_image
            Err(anyhow::anyhow!("S3 image preview not yet implemented"))
        }
    }
}

async fn load_local_image(
    path: &str,
    mode: ImageRenderMode,
    source: PreviewSource,
) -> Result<ImagePreview> {
    let path_obj = Path::new(path);
    let filename = path_obj
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    match mode {
        ImageRenderMode::Ascii => {
            let ascii = convert_to_ascii(path).await?;
            Ok(ImagePreview::new(filename, source, mode, Some(ascii)))
        }
        ImageRenderMode::NotSupported => {
            Err(anyhow::anyhow!("Terminal does not support image rendering"))
        }
    }
}

fn detect_terminal_capabilities() -> ImageRenderMode {
    // Check terminal type - for now default to ASCII
    // Future: Check for Kitty, Sixel support
    ImageRenderMode::Ascii
}

async fn convert_to_ascii(path: &str) -> Result<String> {
    use image::GenericImageView;

    // Load image
    let img = image::open(path)?;

    // Create simple ASCII representation
    let (width, height) = img.dimensions();
    let aspect_ratio = width as f32 / height as f32;
    let target_width = 80;
    let target_height = (target_width as f32 / aspect_ratio / 2.0) as u32;

    let resized = img.resize_exact(
        target_width,
        target_height,
        image::imageops::FilterType::Triangle,
    );

    let mut ascii_art = String::new();
    let chars = " .:-=+*#%@";

    for y in 0..resized.height() {
        for x in 0..resized.width() {
            let pixel = resized.get_pixel(x, y);
            let brightness = (pixel[0] as u32 + pixel[1] as u32 + pixel[2] as u32) / 3;
            let char_index = (brightness * chars.len() as u32 / 256) as usize;
            let char_index = char_index.min(chars.len() - 1);
            ascii_art.push(chars.chars().nth(char_index).unwrap());
        }
        ascii_art.push('\n');
    }

    Ok(ascii_art)
}

/// Prüft ob Datei ein Bild ist
pub fn is_image_file(filename: &str) -> bool {
    let ext = filename.split('.').next_back().unwrap_or("").to_lowercase();
    matches!(
        ext.as_str(),
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp"
    )
}
