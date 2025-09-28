use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use tokio::fs as async_fs;
use tokio::io::AsyncReadExt;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct HashStore {
    pub hashes: BTreeMap<String, String>,
}

impl HashStore {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        if path.as_ref().exists() {
            let content = fs::read_to_string(path)?;
            let store: HashStore = serde_yaml::from_str(&content)?;
            Ok(store)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_yaml::to_string(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub async fn compute_hash<P: AsRef<Path>>(path: P) -> Result<String, Box<dyn std::error::Error>> {
        let mut file = async_fs::File::open(path).await?;
        let mut hasher = Sha256::new();
        let mut buffer = [0; 8192];
        loop {
            let n = file.read(&mut buffer).await?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }
        let hash = hasher.finalize();
        Ok(format!("{:x}", hash))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_compute_hash() {
        let content = b"test content for hashing";
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(content).unwrap();

        let hash = HashStore::compute_hash(temp_file.path()).await.unwrap();
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // SHA256 hex length

        // Same content same hash
        let hash2 = HashStore::compute_hash(temp_file.path()).await.unwrap();
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_hash_store_load_save() {
        let mut store = HashStore::default();
        store.hashes.insert("file1".to_string(), "hash1".to_string());

        let temp_path = NamedTempFile::new().unwrap().path().to_path_buf();
        store.save(&temp_path).unwrap();

        let loaded = HashStore::load(&temp_path).unwrap();
        assert_eq!(loaded.hashes, store.hashes);
    }
}