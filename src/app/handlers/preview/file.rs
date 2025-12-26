use crate::app::{App, Screen};
use crate::models::preview::PreviewSource;
use anyhow::Result;

/// Zeigt Text-Datei-Vorschau an
pub async fn show_file_content_preview(app: &mut App, source: PreviewSource) -> Result<()> {
    match crate::operations::preview::load_file_content(source).await {
        Ok(preview) => {
            app.file_content_preview = Some(preview);
            app.prev_screen = Some(app.screen.clone());
            app.screen = Screen::FileContentPreview;
            Ok(())
        }
        Err(e) => {
            app.show_error(&format!("Cannot preview file: {e}"));
            Err(e)
        }
    }
}

/// Scrollt in Text-Vorschau nach oben
pub fn scroll_file_preview_up(app: &mut App) {
    if let Some(ref mut preview) = app.file_content_preview {
        preview.scroll_offset = preview.scroll_offset.saturating_sub(1);
    }
}

/// Scrollt in Text-Vorschau nach unten
pub fn scroll_file_preview_down(app: &mut App) {
    if let Some(ref mut preview) = app.file_content_preview {
        // Use visual line count to limit scrolling
        let visual_line_count = preview.calculate_visual_line_count();
        if preview.scroll_offset < visual_line_count.saturating_sub(1) {
            preview.scroll_offset += 1;
        }
    }
}

/// Scrollt Seite nach oben
pub fn scroll_file_preview_page_up(app: &mut App, page_size: usize) {
    if let Some(ref mut preview) = app.file_content_preview {
        preview.scroll_offset = preview.scroll_offset.saturating_sub(page_size);
    }
}

/// Scrollt Seite nach unten
pub fn scroll_file_preview_page_down(app: &mut App, page_size: usize) {
    if let Some(ref mut preview) = app.file_content_preview {
        preview.scroll_offset += page_size;
    }
}

/// Springt zum Anfang der Datei (lädt head falls im Backward-Modus)
pub async fn scroll_file_preview_home(app: &mut App) -> Result<()> {
    let preview_info = match &app.file_content_preview {
        Some(p) => (
            p.source.clone(),
            p.preview_mode.clone(),
            p.content_start_offset,
        ),
        None => return Ok(()),
    };

    let (source, preview_mode, content_start_offset) = preview_info;

    // If in backward mode and not at beginning, load the head
    if preview_mode == crate::models::preview::PreviewMode::Backward && content_start_offset > 0 {
        let chunk_size = 100 * 1024;

        match source {
            crate::models::preview::PreviewSource::LocalFile { path } => {
                // Load first 100KB
                match crate::operations::preview::file_loader::load_local_file_range(
                    &path, 0, chunk_size,
                )
                .await
                {
                    Ok(head_content) => {
                        if let Some(ref mut preview) = app.file_content_preview {
                            preview.content = head_content;
                            preview.content_start_offset = 0;
                            preview.byte_offset = chunk_size;
                            preview.preview_mode = crate::models::preview::PreviewMode::Forward;
                            preview.scroll_offset = 0;
                            preview.chunk_load_count += 1;
                        }
                    }
                    Err(e) => {
                        app.show_error(&format!("Failed to load file head: {e}"));
                        return Err(e);
                    }
                }
            }
            crate::models::preview::PreviewSource::S3Object { key, .. } => {
                // Load first 100KB from S3
                if let Some(s3_manager) = &app.get_active_panel().s3_manager {
                    match s3_manager.get_object_range(&key, 0, chunk_size - 1).await {
                        Ok(bytes) => {
                            if let Ok(head_content) = String::from_utf8(bytes) {
                                if let Some(ref mut preview) = app.file_content_preview {
                                    preview.content = head_content;
                                    preview.content_start_offset = 0;
                                    preview.byte_offset = chunk_size;
                                    preview.preview_mode =
                                        crate::models::preview::PreviewMode::Forward;
                                    preview.scroll_offset = 0;
                                    preview.chunk_load_count += 1;
                                }
                            }
                        }
                        Err(e) => {
                            app.show_error(&format!("Failed to load S3 file head: {e}"));
                        }
                    }
                }
            }
        }
    } else {
        // Just scroll to top if already in forward mode or at beginning
        if let Some(ref mut preview) = app.file_content_preview {
            preview.scroll_offset = 0;
            preview.preview_mode = crate::models::preview::PreviewMode::Forward;
        }
    }

    Ok(())
}

/// Springt zum Ende der Datei (lädt tail falls nötig)
pub async fn scroll_file_preview_end(app: &mut App) -> Result<()> {
    let preview_info = match &app.file_content_preview {
        Some(p) => (p.source.clone(), p.byte_offset, p.file_size),
        None => return Ok(()),
    };

    let (source, byte_offset, file_size) = preview_info;

    // Check if we need to load the tail (if we haven't loaded the entire file)
    let needs_tail = byte_offset < file_size;

    if needs_tail {
        let chunk_size = 100 * 1024;
        // Load the tail of the file
        match source {
            crate::models::preview::PreviewSource::LocalFile { path } => {
                match crate::operations::preview::file_loader::load_local_file_tail(
                    &path, file_size,
                )
                .await
                {
                    Ok(tail_content) => {
                        if let Some(ref mut preview) = app.file_content_preview {
                            let start_offset = file_size.saturating_sub(chunk_size).max(0);
                            preview.content = tail_content;
                            preview.byte_offset = file_size; // Mark as fully loaded
                            preview.content_start_offset = start_offset;
                            preview.preview_mode = crate::models::preview::PreviewMode::Backward;
                            preview.chunk_load_count += 1;
                        }
                    }
                    Err(e) => {
                        app.show_error(&format!("Failed to load file tail: {e}"));
                        return Err(e);
                    }
                }
            }
            crate::models::preview::PreviewSource::S3Object { key, .. } => {
                // Load S3 tail
                let start_offset = file_size.saturating_sub(chunk_size).max(0);
                if let Some(s3_manager) = &app.get_active_panel().s3_manager {
                    match s3_manager
                        .get_object_range(&key, start_offset, file_size - 1)
                        .await
                    {
                        Ok(bytes) => {
                            if let Ok(tail_content) = String::from_utf8(bytes) {
                                if let Some(ref mut preview) = app.file_content_preview {
                                    preview.content = tail_content;
                                    preview.byte_offset = file_size; // Mark as fully loaded
                                    preview.content_start_offset = start_offset;
                                    preview.preview_mode =
                                        crate::models::preview::PreviewMode::Backward;
                                    preview.chunk_load_count += 1;
                                }
                            }
                        }
                        Err(e) => {
                            app.show_error(&format!("Failed to load S3 file tail: {e}"));
                        }
                    }
                }
            }
        }
    }

    // Now scroll to the end (last visual line)
    if let Some(ref mut preview) = app.file_content_preview {
        // Scroll to last visual line (including wrapped lines)
        let visual_line_count = preview.calculate_visual_line_count();
        preview.scroll_offset = visual_line_count.saturating_sub(1);
    }

    Ok(())
}

/// Lädt vorherigen Content (für Backward-Modus beim Aufwärts-Scrollen)
pub async fn load_previous_file_content(app: &mut App) -> Result<()> {
    let preview_info = match &app.file_content_preview {
        Some(p) => (
            p.source.clone(),
            p.content_start_offset,
            p.preview_mode.clone(),
        ),
        None => return Ok(()),
    };

    let (source, content_start_offset, preview_mode) = preview_info;

    // Only load previous if in backward mode and not at beginning
    if preview_mode != crate::models::preview::PreviewMode::Backward || content_start_offset == 0 {
        return Ok(());
    }

    let chunk_size = 100 * 1024;
    let new_start_offset = content_start_offset.saturating_sub(chunk_size).max(0);
    let bytes_to_load = content_start_offset - new_start_offset;

    if bytes_to_load == 0 {
        return Ok(());
    }

    match source {
        crate::models::preview::PreviewSource::LocalFile { path } => {
            match crate::operations::preview::file_loader::load_local_file_range(
                &path,
                new_start_offset,
                bytes_to_load,
            )
            .await
            {
                Ok(previous_content) => {
                    if let Some(ref mut preview) = app.file_content_preview {
                        let previous_lines = previous_content.lines().count();
                        // Prepend content and adjust scroll offset
                        preview.content = format!("{}{}", previous_content, preview.content);
                        preview.content_start_offset = new_start_offset;
                        preview.scroll_offset += previous_lines;
                        preview.chunk_load_count += 1;
                    }
                }
                Err(e) => {
                    app.show_error(&format!("Failed to load previous content: {e}"));
                }
            }
        }
        crate::models::preview::PreviewSource::S3Object { key, .. } => {
            if let Some(s3_manager) = &app.get_active_panel().s3_manager {
                match s3_manager
                    .get_object_range(&key, new_start_offset, content_start_offset - 1)
                    .await
                {
                    Ok(bytes) => {
                        if let Ok(previous_content) = String::from_utf8(bytes) {
                            if let Some(ref mut preview) = app.file_content_preview {
                                let previous_lines = previous_content.lines().count();
                                // Prepend content and adjust scroll offset
                                preview.content =
                                    format!("{}{}", previous_content, preview.content);
                                preview.content_start_offset = new_start_offset;
                                preview.scroll_offset += previous_lines;
                                preview.chunk_load_count += 1;
                            }
                        }
                    }
                    Err(e) => {
                        app.show_error(&format!("Failed to load previous S3 content: {e}"));
                    }
                }
            }
        }
    }

    Ok(())
}

/// Lädt mehr Content (Lazy Loading für große Dateien)
pub async fn load_more_file_content(app: &mut App) -> Result<()> {
    // Check if we have a file content preview
    let preview_info = match &app.file_content_preview {
        Some(p) => (p.source.clone(), p.byte_offset, p.file_size),
        None => return Ok(()),
    };

    let (source, byte_offset, file_size) = preview_info;

    // Check if we've already loaded everything
    if byte_offset >= file_size {
        return Ok(());
    }

    match source {
        crate::models::preview::PreviewSource::LocalFile { path } => {
            // Load more for local file
            match crate::operations::preview::file_loader::load_more_local_file_content(
                &path,
                byte_offset,
                file_size,
            )
            .await
            {
                Ok(additional_content) => {
                    if let Some(ref mut preview) = app.file_content_preview {
                        let chunk_size = additional_content.len() as i64;
                        preview.content.push_str(&additional_content);
                        preview.byte_offset += chunk_size;
                        preview.chunk_load_count += 1;
                    }
                }
                Err(e) => {
                    app.show_error(&format!("Failed to load more content: {e}"));
                }
            }
        }
        crate::models::preview::PreviewSource::S3Object { key, .. } => {
            // Load more for S3 file
            let chunk_size = 100 * 1024;
            let end_byte = (byte_offset + chunk_size - 1).min(file_size - 1);

            if let Some(s3_manager) = &app.get_active_panel().s3_manager {
                match s3_manager
                    .get_object_range(&key, byte_offset, end_byte)
                    .await
                {
                    Ok(bytes) => {
                        if let Ok(additional_content) = String::from_utf8(bytes) {
                            if let Some(ref mut preview) = app.file_content_preview {
                                preview.content.push_str(&additional_content);
                                preview.byte_offset = end_byte + 1;
                                preview.chunk_load_count += 1;
                            }
                        }
                    }
                    Err(e) => {
                        app.show_error(&format!("Failed to load more content: {e}"));
                    }
                }
            }
        }
    }

    Ok(())
}
