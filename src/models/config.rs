use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketConfig {
    pub name: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub role_chain: Vec<String>,
    #[serde(default = "default_region")]
    pub region: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub base_prefix: Option<String>,
}

fn default_region() -> String {
    "eu-west-1".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfig {
    pub name: String,
    pub buckets: Vec<BucketConfig>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub setup_script: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub profiles: Vec<ProfileConfig>,
}

pub struct ConfigManager {
    config_path: PathBuf,
    pub app_config: AppConfig,
    pub aws_profiles: Vec<String>,
}

impl ConfigManager {
    pub fn new() -> Result<Self> {
        let config_dir = dirs::home_dir()
            .context("Could not find home directory")?
            .join(".config")
            .join("s3c");

        fs::create_dir_all(&config_dir)?;
        let config_path = config_dir.join("config.json");

        let app_config = if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            AppConfig::default()
        };

        let aws_profiles = Self::load_aws_profiles()?;

        Ok(Self {
            config_path,
            app_config,
            aws_profiles,
        })
    }

    fn load_aws_profiles() -> Result<Vec<String>> {
        let home = dirs::home_dir().context("Could not find home directory")?;
        let credentials_path = home.join(".aws").join("credentials");

        if !credentials_path.exists() {
            return Ok(Vec::new());
        }

        let content =
            fs::read_to_string(&credentials_path).context("Failed to read AWS credentials file")?;

        let mut profiles = Vec::new();
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('[') && line.ends_with(']') {
                let profile = &line[1..line.len() - 1];
                profiles.push(profile.to_string());
            }
        }

        Ok(profiles)
    }

    pub fn save(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.app_config)?;
        fs::write(&self.config_path, json)?;
        Ok(())
    }

    pub fn get_profile_config(&self, profile_name: &str) -> Option<&ProfileConfig> {
        self.app_config
            .profiles
            .iter()
            .find(|p| p.name == profile_name)
    }

    pub fn add_bucket_to_profile(
        &mut self,
        profile_name: &str,
        bucket: String,
        role_chain: Vec<String>,
        region: String,
        description: Option<String>,
        base_prefix: Option<String>,
    ) -> Result<()> {
        let bucket_config = BucketConfig {
            name: bucket.clone(),
            role_chain,
            region,
            description,
            base_prefix,
        };

        if let Some(profile) = self
            .app_config
            .profiles
            .iter_mut()
            .find(|p| p.name == profile_name)
        {
            // Replace existing bucket or add new one
            if let Some(existing) = profile.buckets.iter_mut().find(|b| b.name == bucket) {
                *existing = bucket_config;
            } else {
                profile.buckets.push(bucket_config);
            }
        } else {
            self.app_config.profiles.push(ProfileConfig {
                name: profile_name.to_string(),
                buckets: vec![bucket_config],
                setup_script: None,
                description: None,
            });
        }
        self.save()?;
        Ok(())
    }

    pub fn remove_bucket_from_profile(&mut self, profile_name: &str, bucket: &str) -> Result<()> {
        if let Some(profile) = self
            .app_config
            .profiles
            .iter_mut()
            .find(|p| p.name == profile_name)
        {
            profile.buckets.retain(|b| b.name != bucket);
        }
        self.save()?;
        Ok(())
    }

    pub fn get_buckets_for_profile(&self, profile_name: &str) -> Vec<BucketConfig> {
        self.get_profile_config(profile_name)
            .map(|p| p.buckets.clone())
            .unwrap_or_default()
    }

    #[allow(dead_code)]
    pub fn get_bucket_config(&self, profile_name: &str, bucket_name: &str) -> Option<BucketConfig> {
        self.get_profile_config(profile_name)
            .and_then(|p| p.buckets.iter().find(|b| b.name == bucket_name).cloned())
    }

    #[allow(dead_code)]
    pub fn set_profile_setup_script(
        &mut self,
        profile_name: &str,
        script_path: Option<String>,
    ) -> Result<()> {
        if let Some(profile) = self
            .app_config
            .profiles
            .iter_mut()
            .find(|p| p.name == profile_name)
        {
            profile.setup_script = script_path;
        } else {
            self.app_config.profiles.push(ProfileConfig {
                name: profile_name.to_string(),
                buckets: Vec::new(),
                setup_script: script_path,
                description: None,
            });
        }
        self.save()?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn reload_aws_profiles(&mut self) -> Result<()> {
        self.aws_profiles = Self::load_aws_profiles()?;
        Ok(())
    }
}
