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
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    }
}

// Provide a default path for the hash store when not specified in the config file.
fn default_hash_path() -> String {
    "hashes.yaml".to_string()
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
}