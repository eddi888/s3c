use crate::app::{App, Screen};
use crate::models::preview::PreviewSource;
use anyhow::Result;

/// Zeigt Bild-Vorschau an
pub async fn show_image_preview(app: &mut App, source: PreviewSource) -> Result<()> {
    match crate::operations::preview::load_image(source).await {
        Ok(preview) => {
            app.image_preview = Some(preview);
            app.prev_screen = Some(app.screen.clone());
            app.screen = Screen::ImagePreview;
            Ok(())
        }
        Err(e) => {
            app.show_error(&format!("Cannot preview image: {e}"));
            Err(e)
        }
    }
}

/// Prüft ob Datei ein Bild ist
pub fn is_image_file(filename: &str) -> bool {
    crate::operations::preview::image_loader::is_image_file(filename)
}
