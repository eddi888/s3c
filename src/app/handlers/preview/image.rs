use crate::app::{App, Screen};
use crate::models::preview::PreviewSource;
use anyhow::Result;

/// Zeigt Bild-Vorschau an (sofort mit Loading-State, dann async laden)
pub async fn show_image_preview(app: &mut App, source: PreviewSource) -> Result<()> {
    // Sofort in Preview-Screen wechseln mit Loading-State
    app.prev_screen = Some(app.screen.clone());
    app.screen = Screen::ImagePreview;
    app.image_preview_loading = true;
    app.image_preview = None;

    // Erstelle oneshot channel für async result
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.image_preview_receiver = Some(rx);

    // Image im Hintergrund laden (nicht-blockierend)
    tokio::spawn(async move {
        let result = crate::operations::preview::load_image(source).await;
        let _ = tx.send(result); // Ergebnis zurück senden
    });

    Ok(())
}

/// Prüft ob Image-Loading abgeschlossen ist (wird im Event-Loop gecheckt)
pub fn check_image_loading_complete(app: &mut App) -> bool {
    if !app.image_preview_loading {
        return false;
    }

    if let Some(rx) = app.image_preview_receiver.as_mut() {
        // Non-blocking check ob Result verfügbar ist
        match rx.try_recv() {
            Ok(Ok(preview)) => {
                // Image erfolgreich geladen
                app.image_preview = Some(preview);
                app.image_preview_loading = false;
                app.image_preview_receiver = None;
                true
            }
            Ok(Err(e)) => {
                // Fehler beim Laden
                app.show_error(&format!("Cannot load image: {e}"));
                app.image_preview_loading = false;
                app.image_preview_receiver = None;
                app.go_back();
                true
            }
            Err(_) => {
                // Noch nicht fertig, weiter warten
                false
            }
        }
    } else {
        false
    }
}

/// Prüft ob Datei ein Bild ist
pub fn is_image_file(filename: &str) -> bool {
    crate::operations::preview::image_loader::is_image_file(filename)
}
