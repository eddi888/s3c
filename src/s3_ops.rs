use anyhow::{Context, Result};
use aws_config::BehaviorVersion;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use chrono::{DateTime, Utc};
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone)]
pub struct S3Object {
    pub key: String,
    pub size: i64,
    pub last_modified: Option<DateTime<Utc>>,
    pub is_prefix: bool,
}

pub struct S3Manager {
    client: Client,
    bucket: String,
}

impl S3Manager {
    pub async fn new(
        profile_name: &str,
        bucket: String,
        role_chain: Vec<String>,
        region: &str,
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

        let client = Client::new(&config);

        Ok(Self { client, bucket })
    }

    pub async fn list_objects(&self, prefix: &str) -> Result<Vec<S3Object>> {
        let mut objects = Vec::new();
        let prefix = if prefix.is_empty() { "" } else { prefix };

        let resp = self
            .client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(prefix)
            .delimiter("/")
            .send()
            .await
            .map_err(|e| {
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

    pub async fn download_file(&self, key: &str, local_path: &Path) -> Result<()> {
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

        while let Some(bytes) = stream.try_next().await? {
            file.write_all(&bytes).await?;
        }

        file.flush().await?;
        Ok(())
    }

    pub async fn upload_file(&self, local_path: &Path, key: &str) -> Result<()> {
        let body = ByteStream::from_path(local_path)
            .await
            .context("Failed to read local file")?;

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(body)
            .send()
            .await
            .context("Failed to upload object")?;

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

        Ok(())
    }

    /// Copy object from another S3 bucket (server-side, no streaming)
    pub async fn copy_from_bucket(
        &self,
        source_bucket: &str,
        source_key: &str,
        dest_key: &str,
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

        Ok(())
    }

    /// Stream-based copy from another S3Manager (for cross-account/region)
    pub async fn stream_copy_from(
        &self,
        source_manager: &S3Manager,
        source_key: &str,
        dest_key: &str,
    ) -> Result<()> {
        let resp = source_manager
            .client
            .get_object()
            .bucket(&source_manager.bucket)
            .key(source_key)
            .send()
            .await
            .context("Failed to get source object")?;

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(dest_key)
            .body(resp.body)
            .send()
            .await
            .context("Failed to upload to destination")?;

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn move_object(&self, source_key: &str, dest_key: &str) -> Result<()> {
        self.copy_object(source_key, dest_key).await?;
        self.delete_object(source_key).await?;
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
        self.move_object(old_key, new_key).await
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
