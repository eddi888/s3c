use anyhow::Result;
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::Terminal;

use crate::app::{self, App};
use crate::handlers::key_to_message;
use crate::ui;

/// Main application loop following The Elm Architecture (TEA)
pub async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    let mut last_render = std::time::Instant::now();
    let render_interval = std::time::Duration::from_millis(100); // Limit to 10 FPS for smooth rendering
    let mut needs_render = true;

    loop {
        // Process setup scripts (centralized function)
        if process_setup_script(app, terminal).await? {
            needs_render = true;
        }

        // Process background tasks (progress updates, completion, queue management)
        if process_background_tasks(app, terminal).await? {
            needs_render = true;
        }

        // Render only when needed and throttled
        let now = std::time::Instant::now();
        if needs_render && now.duration_since(last_render) >= render_interval {
            terminal.draw(|f| ui::draw(f, app))?;
            // Explicit flush for Windows terminal responsiveness
            #[cfg(target_os = "windows")]
            {
                use std::io::Write;
                let _ = std::io::stdout().flush();
            }
            last_render = now;
            needs_render = false;
        }

        if app.should_quit {
            break;
        }

        if event::poll(std::time::Duration::from_millis(25))? {
            match event::read()? {
                Event::Key(key) => {
                    // Ignore key release events (Windows sends both press and release)
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }

                    // Convert key to message using TEA pattern
                    if let Some(msg) = key_to_message(app, key.code, key.modifiers) {
                        // Process message and handle cascading messages
                        let mut current_msg = Some(msg);
                        while let Some(message) = current_msg {
                            current_msg = app::update(app, message).await?;
                        }
                        needs_render = true; // User input processed, need to render
                    }
                }
                Event::Resize(_, _) => {
                    // Terminal resized - force re-render
                    needs_render = true;
                }
                // Ignore Mouse, Focus, Paste events (Windows optimization)
                _ => {}
            }
        }
    }

    Ok(())
}

/// Process setup scripts that need terminal access
/// Returns true if a script was executed and render is needed
pub async fn process_setup_script<B: ratatui::backend::Backend>(
    app: &mut App,
    terminal: &mut ratatui::Terminal<B>,
) -> Result<bool> {
    use crossterm::{
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use std::io;

    if !app.script.needs_terminal {
        return Ok(false);
    }

    if let (Some(script), Some(profile), bucket_opt) = (
        app.script.pending_script.take(),
        app.script.pending_profile.take(),
        app.script.pending_bucket.take(),
    ) {
        app.script.needs_terminal = false;

        // Suspend TUI
        disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen)?;

        // Run script interactively
        println!("Running setup script: {script}");

        // Platform-specific shell execution
        #[cfg(target_os = "windows")]
        let status = std::process::Command::new("cmd")
            .arg("/C")
            .arg(&script)
            .status();

        #[cfg(not(target_os = "windows"))]
        let status = std::process::Command::new("sh")
            .arg("-c")
            .arg(&script)
            .status();

        // Resume TUI
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        terminal.clear()?;

        // Check result and continue
        match status {
            Ok(exit_status) if exit_status.success() => {
                if let Some(Some(bucket)) = bucket_opt {
                    // Continue loading bucket without script
                    crate::app::navigation::load_s3_bucket_no_script(app, profile, bucket).await?;
                } else {
                    // Just show bucket list for profile
                    let buckets = app.config_manager.get_buckets_for_profile(&profile);
                    let panel = app.get_active_panel();
                    panel.panel_type = crate::app::PanelType::BucketList { profile };
                    panel
                        .list_model
                        .set_items(crate::app::converters::buckets_to_items(buckets));
                    panel.selected_index = 0;
                }
            }
            Ok(_) => {
                app.show_error("Setup script failed");
            }
            Err(e) => {
                app.show_error(&format!("Failed to execute setup script: {e}"));
            }
        }
        return Ok(true);
    }

    Ok(false)
}

/// Process background transfer tasks and update progress
/// This handles all queue processing logic in one place
pub async fn process_background_tasks<B: ratatui::backend::Backend>(
    app: &mut App,
    terminal: &mut ratatui::Terminal<B>,
) -> Result<bool> {
    let mut needs_render = false;

    // Check background transfer task status
    if let Some(ref task) = app.background_transfer_task {
        // Update progress from atomic counter (while transfer is running)
        let current = task
            .progress_counter
            .load(std::sync::atomic::Ordering::Relaxed);
        if let Some(index) = app.current_transfer_index {
            if let Some(op) = app.file_operation_queue.get_mut(index) {
                if op.transferred != current {
                    op.transferred = current;
                    needs_render = true;
                }
            }
        }

        // Check if task is finished
        if task.task_handle.is_finished() {
            let task = app.background_transfer_task.take().unwrap();
            let mut operation = task.operation.lock().await.clone();

            // Ensure transferred shows 100% on completion
            if operation.status == crate::operations::OperationStatus::Completed {
                operation.transferred = operation.total_size;
            }

            // Update queue IMMEDIATELY with final status
            if let Some(index) = app.current_transfer_index {
                if let Some(op) = app.file_operation_queue.get_mut(index) {
                    *op = operation.clone();
                }
            }

            // Force render to show 100% BEFORE cleanup
            terminal.draw(|f| crate::ui::draw(f, app))?;

            // Handle completion/error
            match operation.status {
                crate::operations::OperationStatus::Completed => match &operation.operation_type {
                    crate::operations::OperationType::Download => {
                        app.show_success(&format!("Downloaded: {}", operation.source));
                        crate::app::navigation::reload_local_files(app).await?;
                    }
                    crate::operations::OperationType::Upload => {
                        app.show_success(&format!("Uploaded: {}", operation.source));
                        crate::app::navigation::reload_s3_browser(app).await?;
                    }
                    crate::operations::OperationType::Copy => {
                        app.show_success(&format!("Copied: {}", operation.source));
                        crate::app::navigation::reload_local_files(app).await?;
                    }
                    crate::operations::OperationType::S3Copy => {
                        app.show_success(&format!("S3 copy completed: {}", operation.source));
                        crate::app::navigation::reload_s3_browser(app).await?;
                    }
                    _ => {}
                },
                crate::operations::OperationStatus::Failed(ref err) => {
                    app.show_error(&format!("Transfer failed: {err}"));
                }
                _ => {}
            }

            // Clean up and start next transfer
            app.current_transfer_index = None;
            start_next_queued_transfer(app).await?;
            needs_render = true;
        }
    }

    // Check if no transfer is running but queue has pending items
    if app.background_transfer_task.is_none() && app.current_transfer_index.is_none() {
        start_next_queued_transfer(app).await?;
    }

    Ok(needs_render)
}

/// Start the next queued transfer if any are pending
pub async fn start_next_queued_transfer(app: &mut App) -> Result<()> {
    use std::sync::Arc;
    use tokio::sync::Mutex;

    // Find next pending operation in queue
    let next_pending_index = app
        .file_operation_queue
        .iter()
        .position(|op| op.status == crate::operations::OperationStatus::Pending);

    if let Some(index) = next_pending_index {
        // Get operation details
        let op = app.file_operation_queue[index].clone();

        // Mark as in progress
        app.file_operation_queue[index].status = crate::operations::OperationStatus::InProgress;
        app.current_transfer_index = Some(index);

        // Create Arc<Mutex<FileOperation>> for task
        let operation = Arc::new(Mutex::new(op.clone()));

        // Start transfer based on operation type
        match op.operation_type {
            crate::operations::OperationType::Download => {
                // Download: S3 → Local
                if let (Some(profile), Some(bucket)) = (&op.profile, &op.bucket) {
                    // Get bucket config to retrieve credentials info
                    let bucket_config = app
                        .config_manager
                        .get_buckets_for_profile(profile)
                        .into_iter()
                        .find(|b| &b.name == bucket);

                    if let Some(config) = bucket_config {
                        // Create S3Manager with stored credentials
                        match crate::operations::s3::S3Manager::new(
                            profile,
                            bucket.clone(),
                            config.role_chain.clone(),
                            &config.region,
                            config.endpoint_url.as_deref(),
                            config.path_style,
                        )
                        .await
                        {
                            Ok(s3_manager) => {
                                start_download_task(
                                    app,
                                    operation,
                                    s3_manager,
                                    op.source.clone(),
                                    op.destination.clone(),
                                )
                                .await;
                            }
                            Err(e) => {
                                app.file_operation_queue[index].status =
                                    crate::operations::OperationStatus::Failed(format!(
                                        "S3Manager creation failed: {e}"
                                    ));
                                app.current_transfer_index = None;
                            }
                        }
                    } else {
                        app.file_operation_queue[index].status =
                            crate::operations::OperationStatus::Failed(
                                "Bucket config not found".to_string(),
                            );
                        app.current_transfer_index = None;
                    }
                }
            }
            crate::operations::OperationType::Upload => {
                // Upload: Local → S3
                if let (Some(profile), Some(bucket)) = (&op.profile, &op.bucket) {
                    let bucket_config = app
                        .config_manager
                        .get_buckets_for_profile(profile)
                        .into_iter()
                        .find(|b| &b.name == bucket);

                    if let Some(config) = bucket_config {
                        match crate::operations::s3::S3Manager::new(
                            profile,
                            bucket.clone(),
                            config.role_chain.clone(),
                            &config.region,
                            config.endpoint_url.as_deref(),
                            config.path_style,
                        )
                        .await
                        {
                            Ok(s3_manager) => {
                                start_upload_task(
                                    app,
                                    operation,
                                    s3_manager,
                                    op.source.clone(),
                                    op.destination.clone(),
                                )
                                .await;
                            }
                            Err(e) => {
                                app.file_operation_queue[index].status =
                                    crate::operations::OperationStatus::Failed(format!(
                                        "S3Manager creation failed: {e}"
                                    ));
                                app.current_transfer_index = None;
                            }
                        }
                    } else {
                        app.file_operation_queue[index].status =
                            crate::operations::OperationStatus::Failed(
                                "Bucket config not found".to_string(),
                            );
                        app.current_transfer_index = None;
                    }
                }
            }
            crate::operations::OperationType::Copy => {
                // Local → Local: Copy file
                start_copy_task(app, operation, op.source.clone(), op.destination.clone()).await;
            }
            crate::operations::OperationType::S3Copy => {
                // S3 → S3: Copy between buckets/providers
                if let (
                    Some(src_profile),
                    Some(src_bucket),
                    Some(dest_profile),
                    Some(dest_bucket),
                ) = (&op.profile, &op.bucket, &op.dest_profile, &op.dest_bucket)
                {
                    // Get source bucket config
                    let src_bucket_config = app
                        .config_manager
                        .get_buckets_for_profile(src_profile)
                        .into_iter()
                        .find(|b| &b.name == src_bucket);

                    // Get destination bucket config
                    let dest_bucket_config = app
                        .config_manager
                        .get_buckets_for_profile(dest_profile)
                        .into_iter()
                        .find(|b| &b.name == dest_bucket);

                    if let (Some(src_config), Some(dest_config)) =
                        (src_bucket_config, dest_bucket_config)
                    {
                        // Create both S3 managers
                        let src_manager_result = crate::operations::s3::S3Manager::new(
                            src_profile,
                            src_bucket.clone(),
                            src_config.role_chain.clone(),
                            &src_config.region,
                            src_config.endpoint_url.as_deref(),
                            src_config.path_style,
                        )
                        .await;

                        let dest_manager_result = crate::operations::s3::S3Manager::new(
                            dest_profile,
                            dest_bucket.clone(),
                            dest_config.role_chain.clone(),
                            &dest_config.region,
                            dest_config.endpoint_url.as_deref(),
                            dest_config.path_style,
                        )
                        .await;

                        match (src_manager_result, dest_manager_result) {
                            (Ok(src_manager), Ok(dest_manager)) => {
                                // Extract keys from s3:// URLs
                                let source_key = op
                                    .source
                                    .strip_prefix(&format!("s3://{src_bucket}/"))
                                    .unwrap_or(&op.source)
                                    .to_string();
                                let dest_key = op
                                    .destination
                                    .strip_prefix(&format!("s3://{dest_bucket}/"))
                                    .unwrap_or(&op.destination)
                                    .to_string();

                                // Check if cross-profile (different credentials)
                                let is_cross_profile = src_profile != dest_profile;

                                start_s3_copy_task(
                                    app,
                                    operation,
                                    src_manager,
                                    dest_manager,
                                    src_bucket.clone(),
                                    source_key,
                                    dest_key,
                                    is_cross_profile,
                                )
                                .await;
                            }
                            (Err(e), _) | (_, Err(e)) => {
                                app.file_operation_queue[index].status =
                                    crate::operations::OperationStatus::Failed(format!(
                                        "S3Manager creation failed: {e}"
                                    ));
                                app.current_transfer_index = None;
                            }
                        }
                    } else {
                        app.file_operation_queue[index].status =
                            crate::operations::OperationStatus::Failed(
                                "Bucket config not found".to_string(),
                            );
                        app.current_transfer_index = None;
                    }
                }
            }
            _ => {
                // Other operation types not supported in queue yet
                app.file_operation_queue[index].status = crate::operations::OperationStatus::Failed(
                    "Operation type not supported in queue".to_string(),
                );
                app.current_transfer_index = None;
            }
        }
    }

    Ok(())
}

async fn start_download_task(
    app: &mut App,
    operation: std::sync::Arc<tokio::sync::Mutex<crate::operations::FileOperation>>,
    s3_manager: crate::operations::s3::S3Manager,
    s3_key: String,
    local_path: String,
) {
    use std::path::PathBuf;
    use std::sync::Arc;

    let transferred_counter = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let transferred_clone = transferred_counter.clone();

    let progress_callback: crate::operations::s3::ProgressCallback =
        Arc::new(tokio::sync::Mutex::new(move |transferred: u64| {
            transferred_clone.store(transferred, std::sync::atomic::Ordering::Relaxed);
        }));

    let operation_clone = operation.clone();
    let local_path_buf = PathBuf::from(local_path);
    let task_handle = tokio::spawn(async move {
        let result = s3_manager
            .download_file_with_progress(&s3_key, &local_path_buf, Some(progress_callback))
            .await;

        match result {
            Ok(_) => {
                operation_clone.lock().await.status = crate::operations::OperationStatus::Completed;
                Ok(())
            }
            Err(e) => {
                operation_clone.lock().await.status =
                    crate::operations::OperationStatus::Failed(format!("{e}"));
                Err(anyhow::anyhow!("Download failed: {e}"))
            }
        }
    });

    app.background_transfer_task = Some(crate::app::BackgroundTransferTask {
        task_handle,
        progress_counter: transferred_counter,
        operation,
    });
}

async fn start_upload_task(
    app: &mut App,
    operation: std::sync::Arc<tokio::sync::Mutex<crate::operations::FileOperation>>,
    s3_manager: crate::operations::s3::S3Manager,
    local_path: String,
    s3_key: String,
) {
    use std::path::PathBuf;
    use std::sync::Arc;

    let transferred_counter = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let transferred_clone = transferred_counter.clone();

    let progress_callback: crate::operations::s3::ProgressCallback =
        Arc::new(tokio::sync::Mutex::new(move |transferred: u64| {
            transferred_clone.store(transferred, std::sync::atomic::Ordering::Relaxed);
        }));

    let operation_clone = operation.clone();
    let path = PathBuf::from(local_path);

    let task_handle = tokio::spawn(async move {
        let result = s3_manager
            .upload_file_with_progress(&path, &s3_key, Some(progress_callback))
            .await;

        match result {
            Ok(_) => {
                operation_clone.lock().await.status = crate::operations::OperationStatus::Completed;
                Ok(())
            }
            Err(e) => {
                operation_clone.lock().await.status =
                    crate::operations::OperationStatus::Failed(format!("{e}"));
                Err(anyhow::anyhow!("Upload failed: {e}"))
            }
        }
    });

    app.background_transfer_task = Some(crate::app::BackgroundTransferTask {
        task_handle,
        progress_counter: transferred_counter,
        operation,
    });
}

async fn start_copy_task(
    app: &mut App,
    operation: std::sync::Arc<tokio::sync::Mutex<crate::operations::FileOperation>>,
    source_path: String,
    dest_path: String,
) {
    use std::path::PathBuf;
    use std::sync::Arc;

    let transferred_counter = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let transferred_clone = transferred_counter.clone();

    let operation_clone = operation.clone();
    let task_handle = tokio::spawn(async move {
        let source = PathBuf::from(&source_path);
        let dest = PathBuf::from(&dest_path);

        // Create parent directories for destination
        if let Some(parent) = dest.parent() {
            if let Err(e) = tokio::fs::create_dir_all(parent).await {
                operation_clone.lock().await.status = crate::operations::OperationStatus::Failed(
                    format!("Failed to create destination directory: {e}"),
                );
                return Err(anyhow::anyhow!("Failed to create directory: {e}"));
            }
        }

        // Copy file with progress tracking
        match tokio::fs::metadata(&source).await {
            Ok(_metadata) => {
                // Simple copy using tokio::fs::copy
                match tokio::fs::copy(&source, &dest).await {
                    Ok(bytes_copied) => {
                        transferred_clone.store(bytes_copied, std::sync::atomic::Ordering::Relaxed);
                        operation_clone.lock().await.status =
                            crate::operations::OperationStatus::Completed;
                        Ok(())
                    }
                    Err(e) => {
                        operation_clone.lock().await.status =
                            crate::operations::OperationStatus::Failed(format!("{e}"));
                        Err(anyhow::anyhow!("Copy failed: {e}"))
                    }
                }
            }
            Err(e) => {
                operation_clone.lock().await.status = crate::operations::OperationStatus::Failed(
                    format!("Source file not found: {e}"),
                );
                Err(anyhow::anyhow!("Source file error: {e}"))
            }
        }
    });

    app.background_transfer_task = Some(crate::app::BackgroundTransferTask {
        task_handle,
        progress_counter: transferred_counter,
        operation,
    });
}

#[allow(clippy::too_many_arguments)]
async fn start_s3_copy_task(
    app: &mut App,
    operation: std::sync::Arc<tokio::sync::Mutex<crate::operations::FileOperation>>,
    src_manager: crate::operations::s3::S3Manager,
    dest_manager: crate::operations::s3::S3Manager,
    src_bucket: String,
    source_key: String,
    dest_key: String,
    is_cross_profile: bool,
) {
    use std::sync::Arc;

    let transferred_counter = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let transferred_clone = transferred_counter.clone();

    let progress_callback: crate::operations::s3::ProgressCallback =
        Arc::new(tokio::sync::Mutex::new(move |transferred: u64| {
            transferred_clone.store(transferred, std::sync::atomic::Ordering::Relaxed);
        }));

    let operation_clone = operation.clone();

    let task_handle = tokio::spawn(async move {
        // For cross-profile, use stream-based copy directly (different credentials)
        if is_cross_profile {
            let stream_result = dest_manager
                .stream_copy_from_with_progress(
                    &src_manager,
                    &source_key,
                    &dest_key,
                    Some(progress_callback),
                )
                .await;

            match stream_result {
                Ok(_) => {
                    operation_clone.lock().await.status =
                        crate::operations::OperationStatus::Completed;
                    Ok(())
                }
                Err(e) => {
                    operation_clone.lock().await.status =
                        crate::operations::OperationStatus::Failed(format!("{e}"));
                    Err(anyhow::anyhow!("S3 copy failed: {e}"))
                }
            }
        } else {
            // Same profile: Try server-side copy first (faster, no data transfer)
            let result = dest_manager
                .copy_from_bucket_with_progress(
                    &src_bucket,
                    &source_key,
                    &dest_key,
                    Some(progress_callback.clone()),
                )
                .await;

            match result {
                Ok(_) => {
                    operation_clone.lock().await.status =
                        crate::operations::OperationStatus::Completed;
                    Ok(())
                }
                Err(_) => {
                    // Fallback to stream-based copy if server-side fails
                    let stream_result = dest_manager
                        .stream_copy_from_with_progress(
                            &src_manager,
                            &source_key,
                            &dest_key,
                            Some(progress_callback),
                        )
                        .await;

                    match stream_result {
                        Ok(_) => {
                            operation_clone.lock().await.status =
                                crate::operations::OperationStatus::Completed;
                            Ok(())
                        }
                        Err(e) => {
                            operation_clone.lock().await.status =
                                crate::operations::OperationStatus::Failed(format!("{e}"));
                            Err(anyhow::anyhow!("S3 copy failed: {e}"))
                        }
                    }
                }
            }
        }
    });

    app.background_transfer_task = Some(crate::app::BackgroundTransferTask {
        task_handle,
        progress_counter: transferred_counter,
        operation,
    });
}
