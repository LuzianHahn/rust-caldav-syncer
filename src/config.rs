use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub webdav_url: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub folders: Vec<String>,
    #[serde(default = "default_hash_path")]
    pub hash_store_path: String,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    #[serde(default = "default_target_dir")]
    pub target_dir: String,
}

impl Config {
    /// Load the configuration from a YAML file and validate its contents.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    /// Validate required configuration fields.
    /// Returns an error if any required field is missing or invalid.
    pub fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.webdav_url.trim().is_empty() {
            return Err("webdav_url cannot be empty".into());
        }
        if self.folders.is_empty() {
            return Err("folders list cannot be empty".into());
        }
        for folder in &self.folders {
            if folder.trim().is_empty() {
                return Err("folder path cannot be empty".into());
            }
        }
        Ok(())
    }
}

// Provide a default path for the hash store when not specified in the config file.
fn default_hash_path() -> String {
    "hashes.yaml".to_string()
}

fn default_timeout_secs() -> u64 {
    3
}

fn default_target_dir() -> String {
    "".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_valid_config() {
        let yaml = r#"
webdav_url: "http://example.com/webdav"
username: "user"
password: "pass"
folders:
  - "/path/to/folder1"
  - "/path/to/folder2"
"#;
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", yaml).unwrap();

        let config = Config::load(temp_file.path()).unwrap();
        assert_eq!(config.webdav_url, "http://example.com/webdav");
        assert_eq!(config.username, Some("user".to_string()));
        assert_eq!(config.password, Some("pass".to_string()));
        assert_eq!(config.folders, vec!["/path/to/folder1", "/path/to/folder2"]);
    }

    #[test]
    fn test_load_invalid_yaml() {
        let invalid_yaml = "invalid: yaml: content";
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", invalid_yaml).unwrap();

        let result = Config::load(temp_file.path());
        assert!(result.is_err());
    }
// Additional tests for configuration validation.
#[test]
fn test_load_missing_webdav_url() {
    let yaml = r#"
username: "user"
password: "pass"
folders:
  - "/path/to/folder1"
"#;
    let mut temp_file = tempfile::NamedTempFile::new().unwrap();
    std::io::Write::write_all(&mut temp_file, yaml.as_bytes()).unwrap();

    let result = Config::load(temp_file.path());
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("webdav_url"));
}

#[test]
fn test_load_empty_folders() {
    let yaml = r#"
webdav_url: "http://example.com/webdav"
folders: []
"#;
    let mut temp_file = tempfile::NamedTempFile::new().unwrap();
    std::io::Write::write_all(&mut temp_file, yaml.as_bytes()).unwrap();
    #[test]
    fn test_load_with_target_dir() {
        let yaml = r#"
webdav_url: "http://example.com/webdav"
username: "user"
password: "pass"
folders:
  - "/path/to/folder1"
target_dir: "remote/dir"
"#;
        let mut temp_file = tempfile::NamedTempFile::new().unwrap();
        write!(temp_file, "{}", yaml).unwrap();
        let config = Config::load(temp_file.path()).unwrap();
        assert_eq!(config.target_dir, "remote/dir");
    }

    #[test]
    fn test_load_without_target_dir_defaults_empty() {
        let yaml = r#"
webdav_url: "http://example.com/webdav"
folders:
  - "/path/to/folder1"
"#;
        let mut temp_file = tempfile::NamedTempFile::new().unwrap();
        write!(temp_file, "{}", yaml).unwrap();
        let config = Config::load(temp_file.path()).unwrap();
        assert_eq!(config.target_dir, "");
    }

    let result = Config::load(temp_file.path());
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("folders"));
}
}