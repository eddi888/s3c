use anyhow::{Context, Result};
use aws_config::BehaviorVersion;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use chrono::{DateTime, Utc};
use std::path::Path;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

pub type ProgressCallback = Arc<Mutex<dyn FnMut(u64) + Send>>;

#[derive(Debug, Clone)]
pub struct S3Object {
    pub key: String,
    pub size: i64,
    pub last_modified: Option<DateTime<Utc>>,
    pub is_prefix: bool,
}

#[derive(Clone)]
pub struct S3Manager {
    pub client: Client,
    pub bucket: String,
}

impl S3Manager {
    pub async fn new(
        profile_name: &str,
        bucket: String,
        role_chain: Vec<String>,
        region: &str,
        endpoint_url: Option<&str>,
        path_style: Option<bool>,
    ) -> Result<Self> {
        let region_str = region.to_string();

        // Load initial config from profile
        let mut config = aws_config::defaults(BehaviorVersion::latest())
            .profile_name(profile_name)
            .region(aws_config::Region::new(region_str.clone()))
            .load()
            .await;

        // Chain through multiple roles if provided
        for (index, role) in role_chain.iter().enumerate() {
            let sts_client = aws_sdk_sts::Client::new(&config);

            let assumed_role = sts_client
                .assume_role()
                .role_arn(role)
                .role_session_name(format!(
                    "s3c-chain-{}-{}",
                    index,
                    chrono::Utc::now().timestamp()
                ))
                .send()
                .await
                .context(format!(
                    "Failed to assume role (step {} of {}): {role}",
                    index + 1,
                    role_chain.len()
                ))?;

            if let Some(creds) = assumed_role.credentials() {
                // Create credentials provider from assumed role credentials
                use aws_credential_types::Credentials;
                use std::time::SystemTime;

                let expiration = SystemTime::try_from(*creds.expiration()).ok();

                let credentials = Credentials::new(
                    creds.access_key_id(),
                    creds.secret_access_key(),
                    Some(creds.session_token().to_string()),
                    expiration,
                    "AssumedRole",
                );

                // Build new config with these credentials
                config = aws_config::defaults(BehaviorVersion::latest())
                    .credentials_provider(credentials)
                    .region(aws_config::Region::new(region_str.clone()))
                    .load()
                    .await;
            }
        }

        // Build S3 client with optional custom endpoint and path style
        let mut s3_config_builder = aws_sdk_s3::config::Builder::from(&config);

        // Set custom endpoint for S3-compatible services (Hetzner, Minio, DigitalOcean, etc.)
        if let Some(endpoint) = endpoint_url {
            s3_config_builder = s3_config_builder.endpoint_url(endpoint);
        }

        // Force path-style URLs (required for Minio, Ceph)
        if path_style == Some(true) {
            s3_config_builder = s3_config_builder.force_path_style(true);
        }

        let client = Client::from_conf(s3_config_builder.build());

        Ok(Self { client, bucket })
    }

    pub async fn list_objects(&self, prefix: &str) -> Result<Vec<S3Object>> {
        let mut objects = Vec::new();
        let prefix = if prefix.is_empty() { "" } else { prefix };
        let mut continuation_token: Option<String> = None;

        loop {
            let mut request = self
                .client
                .list_objects_v2()
                .bucket(&self.bucket)
                .prefix(prefix)
                .delimiter("/");

            if let Some(token) = continuation_token {
                request = request.continuation_token(token);
            }

            let resp = request.send().await.map_err(|e| {
                let bucket = &self.bucket;
                anyhow::anyhow!("Failed to list objects in bucket '{bucket}': {e:?}")
            })?;

            for cp in resp.common_prefixes() {
                if let Some(prefix_str) = cp.prefix() {
                    objects.push(S3Object {
                        key: prefix_str.to_string(),
                        size: 0,
                        last_modified: None,
                        is_prefix: true,
                    });
                }
            }

            for obj in resp.contents() {
                if let Some(key) = obj.key() {
                    if key != prefix && !key.ends_with('/') {
                        objects.push(S3Object {
                            key: key.to_string(),
                            size: obj.size().unwrap_or(0),
                            last_modified: obj
                                .last_modified()
                                .map(|t| DateTime::from_timestamp(t.secs(), 0).unwrap_or_default()),
                            is_prefix: false,
                        });
                    }
                }
            }

            if resp.is_truncated().unwrap_or(false) {
                continuation_token = resp.next_continuation_token().map(|s| s.to_string());
            } else {
                break;
            }
        }

        objects.sort_by(|a, b| {
            if a.is_prefix && !b.is_prefix {
                std::cmp::Ordering::Less
            } else if !a.is_prefix && b.is_prefix {
                std::cmp::Ordering::Greater
            } else {
                a.key.cmp(&b.key)
            }
        });

        Ok(objects)
    }

    #[allow(dead_code)]
    pub async fn download_file(&self, key: &str, local_path: &Path) -> Result<()> {
        self.download_file_with_progress(key, local_path, None)
            .await
    }

    pub async fn download_file_with_progress(
        &self,
        key: &str,
        local_path: &Path,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<()> {
        let resp = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .context("Failed to get object")?;

        if let Some(parent) = local_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let mut file = File::create(local_path).await?;
        let mut stream = resp.body;
        let mut total_transferred = 0u64;

        while let Some(bytes) = stream.try_next().await? {
            file.write_all(&bytes).await?;
            total_transferred += bytes.len() as u64;

            if let Some(ref callback) = progress_callback {
                let mut cb = callback.lock().await;
                cb(total_transferred);
            }
        }

        file.flush().await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn upload_file(&self, local_path: &Path, key: &str) -> Result<()> {
        self.upload_file_with_progress(local_path, key, None).await
    }

    pub async fn upload_file_with_progress(
        &self,
        local_path: &Path,
        key: &str,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<()> {
        let file_size = tokio::fs::metadata(local_path)
            .await
            .context("Failed to get file metadata")?
            .len();

        if let Some(ref callback) = progress_callback {
            let mut cb = callback.lock().await;
            cb(0);
        }

        // Use multipart upload for files larger than 5MB
        const MULTIPART_THRESHOLD: u64 = 5 * 1024 * 1024; // 5MB

        if file_size > MULTIPART_THRESHOLD {
            self.upload_file_multipart(local_path, key, file_size, progress_callback)
                .await
        } else {
            self.upload_file_simple(local_path, key, file_size, progress_callback)
                .await
        }
    }

    async fn upload_file_simple(
        &self,
        local_path: &Path,
        key: &str,
        file_size: u64,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<()> {
        use tokio::io::AsyncReadExt;

        // For small files, read entire file and upload in one request
        let mut file = File::open(local_path).await?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await?;

        // Report 50% after reading
        if let Some(ref callback) = progress_callback {
            let mut cb = callback.lock().await;
            cb(file_size / 2);
        }

        let body = ByteStream::from(buffer);

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to upload object: {e}"))?;

        // Report 100% completion
        if let Some(ref callback) = progress_callback {
            let mut cb = callback.lock().await;
            cb(file_size);
        }

        Ok(())
    }

    async fn upload_file_multipart(
        &self,
        local_path: &Path,
        key: &str,
        file_size: u64,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<()> {
        use tokio::io::AsyncReadExt;

        // Part size: 5MB minimum, use 10MB for better progress granularity
        const PART_SIZE: usize = 10 * 1024 * 1024; // 10MB

        // Step 1: Create multipart upload
        let multipart_upload = self
            .client
            .create_multipart_upload()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .context("Failed to create multipart upload")?;

        let upload_id = multipart_upload
            .upload_id()
            .context("Missing upload ID")?
            .to_string();

        let mut file = File::open(local_path).await?;
        let mut part_number = 1;
        let mut uploaded_parts = Vec::new();
        let mut total_uploaded = 0u64;

        // Step 2: Upload parts
        loop {
            let mut buffer = vec![0u8; PART_SIZE];
            let mut bytes_read_total = 0;

            // Read up to PART_SIZE bytes
            while bytes_read_total < PART_SIZE {
                let bytes_read = file.read(&mut buffer[bytes_read_total..]).await?;
                if bytes_read == 0 {
                    break; // EOF
                }
                bytes_read_total += bytes_read;
            }

            if bytes_read_total == 0 {
                break; // No more data
            }

            // Trim buffer to actual size read
            buffer.truncate(bytes_read_total);

            // Upload this part
            let upload_part_result = self
                .client
                .upload_part()
                .bucket(&self.bucket)
                .key(key)
                .upload_id(&upload_id)
                .part_number(part_number)
                .body(ByteStream::from(buffer))
                .send()
                .await;

            match upload_part_result {
                Ok(output) => {
                    // Store completed part info
                    uploaded_parts.push(
                        aws_sdk_s3::types::CompletedPart::builder()
                            .part_number(part_number)
                            .e_tag(output.e_tag().unwrap_or(""))
                            .build(),
                    );

                    total_uploaded += bytes_read_total as u64;

                    // Report progress
                    if let Some(ref callback) = progress_callback {
                        let mut cb = callback.lock().await;
                        cb(total_uploaded);
                    }

                    part_number += 1;
                }
                Err(e) => {
                    // Abort multipart upload on error
                    let _ = self
                        .client
                        .abort_multipart_upload()
                        .bucket(&self.bucket)
                        .key(key)
                        .upload_id(&upload_id)
                        .send()
                        .await;

                    return Err(anyhow::anyhow!("Failed to upload part {part_number}: {e}"));
                }
            }
        }

        // Step 3: Complete multipart upload
        let completed_multipart_upload = aws_sdk_s3::types::CompletedMultipartUpload::builder()
            .set_parts(Some(uploaded_parts))
            .build();

        self.client
            .complete_multipart_upload()
            .bucket(&self.bucket)
            .key(key)
            .upload_id(&upload_id)
            .multipart_upload(completed_multipart_upload)
            .send()
            .await
            .context("Failed to complete multipart upload")?;

        // Report 100% completion
        if let Some(ref callback) = progress_callback {
            let mut cb = callback.lock().await;
            cb(file_size);
        }

        Ok(())
    }

    pub async fn upload_empty_folder(&self, key: &str) -> Result<()> {
        // Create empty object with trailing slash to represent folder
        let body = ByteStream::from_static(b"");

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(body)
            .send()
            .await
            .context("Failed to create folder")?;

        Ok(())
    }

    /// Copy object within the same bucket (server-side, no streaming)
    pub async fn copy_object(&self, source_key: &str, dest_key: &str) -> Result<()> {
        self.copy_object_with_progress(source_key, dest_key, None)
            .await
    }

    /// Copy object with progress tracking
    pub async fn copy_object_with_progress(
        &self,
        source_key: &str,
        dest_key: &str,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<()> {
        // Get object size
        let object_size = self.get_object_size(source_key).await?;

        if let Some(ref callback) = progress_callback {
            let mut cb = callback.lock().await;
            cb(0);
        }

        // Use multipart copy for large files (>5MB)
        const MULTIPART_THRESHOLD: i64 = 5 * 1024 * 1024; // 5MB

        if object_size > MULTIPART_THRESHOLD {
            self.copy_object_multipart(source_key, dest_key, object_size, progress_callback)
                .await
        } else {
            self.copy_object_simple(source_key, dest_key, object_size, progress_callback)
                .await
        }
    }

    async fn copy_object_simple(
        &self,
        source_key: &str,
        dest_key: &str,
        object_size: i64,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<()> {
        let bucket = &self.bucket;
        let copy_source = format!("{bucket}/{source_key}");

        self.client
            .copy_object()
            .bucket(&self.bucket)
            .copy_source(&copy_source)
            .key(dest_key)
            .send()
            .await
            .context("Failed to copy object")?;

        // Report 100% completion
        if let Some(ref callback) = progress_callback {
            let mut cb = callback.lock().await;
            cb(object_size as u64);
        }

        Ok(())
    }

    async fn copy_object_multipart(
        &self,
        source_key: &str,
        dest_key: &str,
        object_size: i64,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<()> {
        // Part size: 50MB for better progress granularity
        const PART_SIZE: i64 = 50 * 1024 * 1024; // 50MB

        let bucket = &self.bucket;
        let copy_source = format!("{bucket}/{source_key}");

        // Step 1: Create multipart upload
        let multipart_upload = self
            .client
            .create_multipart_upload()
            .bucket(&self.bucket)
            .key(dest_key)
            .send()
            .await
            .context("Failed to create multipart upload")?;

        let upload_id = multipart_upload
            .upload_id()
            .context("Missing upload ID")?
            .to_string();

        let mut part_number = 1;
        let mut uploaded_parts = Vec::new();
        let mut total_copied = 0i64;

        // Step 2: Copy parts
        let num_parts = (object_size + PART_SIZE - 1) / PART_SIZE;

        for _ in 0..num_parts {
            let start_byte = total_copied;
            let end_byte = std::cmp::min(start_byte + PART_SIZE - 1, object_size - 1);
            let copy_source_range = format!("bytes={start_byte}-{end_byte}");

            let upload_part_result = self
                .client
                .upload_part_copy()
                .bucket(&self.bucket)
                .key(dest_key)
                .upload_id(&upload_id)
                .copy_source(&copy_source)
                .copy_source_range(&copy_source_range)
                .part_number(part_number)
                .send()
                .await;

            match upload_part_result {
                Ok(output) => {
                    // Store completed part info
                    if let Some(copy_result) = output.copy_part_result() {
                        uploaded_parts.push(
                            aws_sdk_s3::types::CompletedPart::builder()
                                .part_number(part_number)
                                .e_tag(copy_result.e_tag().unwrap_or(""))
                                .build(),
                        );
                    }

                    total_copied = end_byte + 1;

                    // Report progress (cap at 95% to leave room for completion step)
                    if let Some(ref callback) = progress_callback {
                        let mut cb = callback.lock().await;
                        let progress_pct =
                            (total_copied as f64 / object_size as f64 * 95.0).min(95.0);
                        let progress_bytes = (object_size as f64 * progress_pct / 100.0) as u64;
                        cb(progress_bytes);
                    }

                    part_number += 1;
                }
                Err(e) => {
                    // Abort multipart upload on error
                    let _ = self
                        .client
                        .abort_multipart_upload()
                        .bucket(&self.bucket)
                        .key(dest_key)
                        .upload_id(&upload_id)
                        .send()
                        .await;

                    return Err(anyhow::anyhow!("Failed to copy part {part_number}: {e}"));
                }
            }
        }

        // Step 3: Complete multipart upload (can take time for large files)
        let completed_multipart_upload = aws_sdk_s3::types::CompletedMultipartUpload::builder()
            .set_parts(Some(uploaded_parts))
            .build();

        self.client
            .complete_multipart_upload()
            .bucket(&self.bucket)
            .key(dest_key)
            .upload_id(&upload_id)
            .multipart_upload(completed_multipart_upload)
            .send()
            .await
            .context("Failed to complete multipart copy")?;

        // Report 100% completion AFTER complete_multipart_upload finishes
        if let Some(ref callback) = progress_callback {
            let mut cb = callback.lock().await;
            cb(object_size as u64);
        }

        Ok(())
    }

    /// Copy object from another S3 bucket (server-side, no streaming)
    #[allow(dead_code)]
    pub async fn copy_from_bucket(
        &self,
        source_bucket: &str,
        source_key: &str,
        dest_key: &str,
    ) -> Result<()> {
        self.copy_from_bucket_with_progress(source_bucket, source_key, dest_key, None)
            .await
    }

    /// Copy object from another bucket with progress tracking
    pub async fn copy_from_bucket_with_progress(
        &self,
        source_bucket: &str,
        source_key: &str,
        dest_key: &str,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<()> {
        if let Some(ref callback) = progress_callback {
            let mut cb = callback.lock().await;
            cb(0);
        }

        // Try to get object size from source bucket
        let object_size_result = self
            .client
            .head_object()
            .bucket(source_bucket)
            .key(source_key)
            .send()
            .await;

        let object_size = match object_size_result {
            Ok(resp) => resp.content_length().unwrap_or(0),
            Err(_) => 0, // Can't get size, use simple copy
        };

        // Use multipart copy for large files (>5MB)
        const MULTIPART_THRESHOLD: i64 = 5 * 1024 * 1024; // 5MB

        if object_size > MULTIPART_THRESHOLD {
            self.copy_from_bucket_multipart(
                source_bucket,
                source_key,
                dest_key,
                object_size,
                progress_callback,
            )
            .await
        } else {
            self.copy_from_bucket_simple(
                source_bucket,
                source_key,
                dest_key,
                object_size,
                progress_callback,
            )
            .await
        }
    }

    async fn copy_from_bucket_simple(
        &self,
        source_bucket: &str,
        source_key: &str,
        dest_key: &str,
        object_size: i64,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<()> {
        let copy_source = format!("{source_bucket}/{source_key}");

        self.client
            .copy_object()
            .bucket(&self.bucket)
            .copy_source(&copy_source)
            .key(dest_key)
            .send()
            .await
            .context("Failed to copy from another bucket")?;

        // Report 100% completion
        if let Some(ref callback) = progress_callback {
            let mut cb = callback.lock().await;
            if object_size > 0 {
                cb(object_size as u64);
            } else {
                cb(100); // Unknown size, fake completion
            }
        }

        Ok(())
    }

    async fn copy_from_bucket_multipart(
        &self,
        source_bucket: &str,
        source_key: &str,
        dest_key: &str,
        object_size: i64,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<()> {
        // Part size: 50MB for better progress granularity
        const PART_SIZE: i64 = 50 * 1024 * 1024; // 50MB

        let copy_source = format!("{source_bucket}/{source_key}");

        // Step 1: Create multipart upload
        let multipart_upload = self
            .client
            .create_multipart_upload()
            .bucket(&self.bucket)
            .key(dest_key)
            .send()
            .await
            .context("Failed to create multipart upload")?;

        let upload_id = multipart_upload
            .upload_id()
            .context("Missing upload ID")?
            .to_string();

        let mut part_number = 1;
        let mut uploaded_parts = Vec::new();
        let mut total_copied = 0i64;

        // Step 2: Copy parts
        let num_parts = (object_size + PART_SIZE - 1) / PART_SIZE;

        for _ in 0..num_parts {
            let start_byte = total_copied;
            let end_byte = std::cmp::min(start_byte + PART_SIZE - 1, object_size - 1);
            let copy_source_range = format!("bytes={start_byte}-{end_byte}");

            let upload_part_result = self
                .client
                .upload_part_copy()
                .bucket(&self.bucket)
                .key(dest_key)
                .upload_id(&upload_id)
                .copy_source(&copy_source)
                .copy_source_range(&copy_source_range)
                .part_number(part_number)
                .send()
                .await;

            match upload_part_result {
                Ok(output) => {
                    // Store completed part info
                    if let Some(copy_result) = output.copy_part_result() {
                        uploaded_parts.push(
                            aws_sdk_s3::types::CompletedPart::builder()
                                .part_number(part_number)
                                .e_tag(copy_result.e_tag().unwrap_or(""))
                                .build(),
                        );
                    }

                    total_copied = end_byte + 1;

                    // Report progress (cap at 95% to leave room for completion step)
                    if let Some(ref callback) = progress_callback {
                        let mut cb = callback.lock().await;
                        let progress_pct =
                            (total_copied as f64 / object_size as f64 * 95.0).min(95.0);
                        let progress_bytes = (object_size as f64 * progress_pct / 100.0) as u64;
                        cb(progress_bytes);
                    }

                    part_number += 1;
                }
                Err(e) => {
                    // Abort multipart upload on error
                    let _ = self
                        .client
                        .abort_multipart_upload()
                        .bucket(&self.bucket)
                        .key(dest_key)
                        .upload_id(&upload_id)
                        .send()
                        .await;

                    return Err(anyhow::anyhow!("Failed to copy part {part_number}: {e}"));
                }
            }
        }

        // Step 3: Complete multipart upload (can take time for large files)
        let completed_multipart_upload = aws_sdk_s3::types::CompletedMultipartUpload::builder()
            .set_parts(Some(uploaded_parts))
            .build();

        self.client
            .complete_multipart_upload()
            .bucket(&self.bucket)
            .key(dest_key)
            .upload_id(&upload_id)
            .multipart_upload(completed_multipart_upload)
            .send()
            .await
            .context("Failed to complete multipart copy")?;

        // Report 100% completion AFTER complete_multipart_upload finishes
        if let Some(ref callback) = progress_callback {
            let mut cb = callback.lock().await;
            cb(object_size as u64);
        }

        Ok(())
    }

    /// Stream-based copy from another S3Manager (for cross-account/region)
    #[allow(dead_code)]
    pub async fn stream_copy_from(
        &self,
        source_manager: &S3Manager,
        source_key: &str,
        dest_key: &str,
    ) -> Result<()> {
        self.stream_copy_from_with_progress(source_manager, source_key, dest_key, None)
            .await
    }

    /// Stream-based copy with progress tracking (for cross-account/region)
    pub async fn stream_copy_from_with_progress(
        &self,
        source_manager: &S3Manager,
        source_key: &str,
        dest_key: &str,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<()> {
        use tokio::io::AsyncReadExt;

        if let Some(ref callback) = progress_callback {
            let mut cb = callback.lock().await;
            cb(0);
        }

        // Get object from source
        let resp = source_manager
            .client
            .get_object()
            .bucket(&source_manager.bucket)
            .key(source_key)
            .send()
            .await
            .context("Failed to get source object")?;

        let object_size = resp.content_length().unwrap_or(0);

        // Read body in chunks and track progress
        let mut body = resp.body.into_async_read();
        let mut buffer = Vec::new();
        let mut chunk = vec![0u8; 1024 * 1024]; // 1MB chunks
        let mut total_read = 0u64;

        loop {
            let bytes_read = body.read(&mut chunk).await?;
            if bytes_read == 0 {
                break;
            }

            buffer.extend_from_slice(&chunk[..bytes_read]);
            total_read += bytes_read as u64;

            if let Some(ref callback) = progress_callback {
                let mut cb = callback.lock().await;
                cb(total_read);
            }
        }

        // Upload to destination
        let body = ByteStream::from(buffer);

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(dest_key)
            .body(body)
            .send()
            .await
            .context("Failed to upload to destination")?;

        // Report 100% completion
        if let Some(ref callback) = progress_callback {
            let mut cb = callback.lock().await;
            if object_size > 0 {
                cb(object_size as u64);
            } else {
                cb(total_read);
            }
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn move_object(&self, source_key: &str, dest_key: &str) -> Result<()> {
        self.move_object_with_progress(source_key, dest_key, None)
            .await
    }

    pub async fn move_object_with_progress(
        &self,
        source_key: &str,
        dest_key: &str,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<()> {
        // Get object size for progress tracking
        let object_size = self.get_object_size(source_key).await?;

        if let Some(ref callback) = progress_callback {
            let mut cb = callback.lock().await;
            cb(0);
        }

        // Try server-side copy first
        self.copy_object(source_key, dest_key).await?;

        // Update progress to 50% after copy
        if let Some(ref callback) = progress_callback {
            let mut cb = callback.lock().await;
            cb((object_size / 2) as u64);
        }

        // Delete original
        self.delete_object(source_key).await?;

        // Update progress to 100% after delete
        if let Some(ref callback) = progress_callback {
            let mut cb = callback.lock().await;
            cb(object_size as u64);
        }

        Ok(())
    }

    pub async fn delete_object(&self, key: &str) -> Result<()> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .context("Failed to delete object")?;

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn rename_object(&self, old_key: &str, new_key: &str) -> Result<()> {
        self.rename_object_with_progress(old_key, new_key, None)
            .await
    }

    pub async fn rename_object_with_progress(
        &self,
        old_key: &str,
        new_key: &str,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<()> {
        self.move_object_with_progress(old_key, new_key, progress_callback)
            .await
    }

    pub async fn get_object_size(&self, key: &str) -> Result<i64> {
        let resp = self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .context("Failed to get object metadata")?;

        Ok(resp.content_length().unwrap_or(0))
    }

    pub async fn get_object_range(&self, key: &str, start: i64, end: i64) -> Result<Vec<u8>> {
        let range = format!("bytes={start}-{end}");

        let resp = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .range(range)
            .send()
            .await
            .context("Failed to get object range")?;

        let bytes = resp.body.collect().await?.into_bytes();
        Ok(bytes.to_vec())
    }

    #[allow(dead_code)]
    pub async fn get_object_content(&self, key: &str, max_size: usize) -> Result<String> {
        let resp = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .context("Failed to get object")?;

        let bytes = resp.body.collect().await?.into_bytes();

        if bytes.len() > max_size {
            let len = bytes.len();
            let content = String::from_utf8_lossy(&bytes[..max_size]);
            return Ok(format!(
                "File too large to display ({len} bytes). Only showing first {max_size} bytes:\n\n{content}"
            ));
        }

        Ok(String::from_utf8_lossy(&bytes).to_string())
    }
}
