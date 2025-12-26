use crate::models::preview::{FileContentPreview, PreviewSource};
use anyhow::Result;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

/// Lädt Text-Datei-Inhalt von Local (S3 requires S3Manager, use load_s3_file_content)
pub async fn load_file_content(source: PreviewSource) -> Result<FileContentPreview> {
    match source {
        PreviewSource::LocalFile { ref path } => load_local_file(path).await,
        PreviewSource::S3Object { .. } => Err(anyhow::anyhow!(
            "S3 files require S3Manager, use load_s3_file_content"
        )),
    }
}

async fn load_local_file(path: &str) -> Result<FileContentPreview> {
    let path_obj = Path::new(path);
    let filename = path_obj
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut file = File::open(path).await?;
    let metadata = file.metadata().await?;
    let file_size = metadata.len() as i64;

    // Load first 100KB chunk (lazy loading like S3)
    let chunk_size = 100 * 1024;
    let load_size = (chunk_size as i64).min(file_size) as usize;

    let mut buffer = vec![0u8; load_size];
    let bytes_read = file.read(&mut buffer).await?;

    let content = String::from_utf8(buffer[..bytes_read].to_vec())
        .map_err(|_| anyhow::anyhow!("File is not valid UTF-8 text"))?;

    Ok(FileContentPreview::new(
        filename,
        content,
        file_size,
        PreviewSource::LocalFile {
            path: path.to_string(),
        },
    ))
}

async fn load_s3_object(
    key: &str,
    bucket: &str,
    s3_manager: &crate::operations::s3::S3Manager,
) -> Result<FileContentPreview> {
    let filename = extract_filename(key);
    let file_size = s3_manager.get_object_size(key).await?;

    let chunk_size = 100 * 1024;
    let load_size = if file_size < chunk_size {
        file_size
    } else {
        chunk_size
    };

    let bytes = s3_manager.get_object_range(key, 0, load_size - 1).await?;
    let content =
        String::from_utf8(bytes).map_err(|_| anyhow::anyhow!("File is not valid UTF-8 text"))?;

    Ok(FileContentPreview::new(
        filename,
        content,
        file_size,
        PreviewSource::S3Object {
            key: key.to_string(),
            bucket: bucket.to_string(),
        },
    ))
}

/// Lädt S3-Datei mit S3Manager (für öffentliche API)
pub async fn load_s3_file_content(
    key: &str,
    bucket: &str,
    s3_manager: &crate::operations::s3::S3Manager,
) -> Result<FileContentPreview> {
    load_s3_object(key, bucket, s3_manager).await
}

/// Lädt nächsten Chunk für lokale Datei
pub async fn load_more_local_file_content(
    path: &str,
    byte_offset: i64,
    file_size: i64,
) -> Result<String> {
    if byte_offset >= file_size {
        return Ok(String::new());
    }

    let mut file = File::open(path).await?;
    let chunk_size = 100 * 1024;
    let bytes_to_read = ((file_size - byte_offset) as usize).min(chunk_size);

    // Seek to offset
    use tokio::io::AsyncSeekExt;
    file.seek(std::io::SeekFrom::Start(byte_offset as u64))
        .await?;

    let mut buffer = vec![0u8; bytes_to_read];
    let bytes_read = file.read(&mut buffer).await?;

    let content = String::from_utf8(buffer[..bytes_read].to_vec())
        .map_err(|_| anyhow::anyhow!("File chunk is not valid UTF-8 text"))?;

    Ok(content)
}

/// Lädt einen bestimmten Bereich einer lokalen Datei
pub async fn load_local_file_range(
    path: &str,
    start_offset: i64,
    bytes_to_read: i64,
) -> Result<String> {
    if bytes_to_read == 0 {
        return Ok(String::new());
    }

    let mut file = File::open(path).await?;

    // Seek to start offset
    use tokio::io::AsyncSeekExt;
    file.seek(std::io::SeekFrom::Start(start_offset as u64))
        .await?;

    let mut buffer = vec![0u8; bytes_to_read as usize];
    let bytes_read = file.read(&mut buffer).await?;

    let content = String::from_utf8(buffer[..bytes_read].to_vec())
        .map_err(|_| anyhow::anyhow!("File range is not valid UTF-8 text"))?;

    Ok(content)
}

/// Lädt die letzten 100KB einer lokalen Datei (für "END" Taste)
pub async fn load_local_file_tail(path: &str, file_size: i64) -> Result<String> {
    if file_size == 0 {
        return Ok(String::new());
    }

    let mut file = File::open(path).await?;
    let chunk_size = 100 * 1024;

    // Calculate start position (last 100KB or from beginning)
    let start_offset = if file_size > chunk_size {
        file_size - chunk_size
    } else {
        0
    };

    let bytes_to_read = (file_size - start_offset) as usize;

    // Seek to start offset
    use tokio::io::AsyncSeekExt;
    file.seek(std::io::SeekFrom::Start(start_offset as u64))
        .await?;

    let mut buffer = vec![0u8; bytes_to_read];
    let bytes_read = file.read(&mut buffer).await?;

    let content = String::from_utf8(buffer[..bytes_read].to_vec())
        .map_err(|_| anyhow::anyhow!("File tail is not valid UTF-8 text"))?;

    Ok(content)
}

/// Extrahiert Dateinamen aus Pfad oder Key
pub fn extract_filename(path_or_key: &str) -> String {
    path_or_key
        .split('/')
        .next_back()
        .unwrap_or("unknown")
        .to_string()
}
