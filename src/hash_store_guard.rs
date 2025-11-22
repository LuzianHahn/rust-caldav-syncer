use crate::config::Config;
use std::error::Error;
use crate::hash_store::HashStore;
use crate::webdav_client::WebDavClient;
use std::path::PathBuf;

/// Guard that ensures the hash store is saved locally and uploaded to the remote
/// WebDAV server when it goes out of scope. This guarantees that the hash store
/// is persisted even if the sync operation aborts or times out.
pub struct HashStoreGuard {
    /// The in‑memory hash store that callers can mutate.
    pub hash_store: HashStore,
    client: WebDavClient,
    local_path: PathBuf,
    remote_path: String,
}

impl HashStoreGuard {
    /// Create a new guard. It downloads the remote hash store (if any) to a
    /// temporary file, loads it (or creates a new empty store), and prepares
    /// for later saving/uploading.
    pub async fn new(
        client: WebDavClient,
        config: &Config,
    ) -> Result<Self, Box<dyn Error>> {
        // Determine paths
        let local_path = PathBuf::from(&config.hash_store_path);
        let remote_path = config.remote_hash_path.clone();

        // Download remote hash store to a temporary location.
        let temp_remote_path = std::env::temp_dir().join("remote_hashes.yaml");
        let _ = client
            .download_file(&remote_path, &temp_remote_path)
            .await;

        // Load (or create) the hash store from the temporary file.
        let hash_store = HashStore::load(&temp_remote_path)?;

        // Clean up the temporary file – it is no longer needed.
        let _ = std::fs::remove_file(&temp_remote_path);

        Ok(Self {
            hash_store,
            client,
            local_path,
            remote_path,
        })
    }

    /// Get a mutable reference to the inner `HashStore`.
    pub fn hash_store_mut(&mut self) -> &mut HashStore {
        &mut self.hash_store
    }

    /// Ensure the hash store is uploaded to the remote location.
    /// This should be called before the guard is dropped to guarantee
    /// that the remote upload has completed.
    pub async fn finalize(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Save locally (ignore errors; Drop will also attempt to save)
        let _ = self.hash_store.save(&self.local_path);
        // Upload to remote
        self.client.upload_file(&self.local_path, &self.remote_path).await?;
        Ok(())
    }
}

impl Drop for HashStoreGuard {
    fn drop(&mut self) {
        // Save the hash store locally.
        if let Err(e) = self.hash_store.save(&self.local_path) {
            eprintln!("Failed to save hash store locally: {}", e);
        }

        // Upload the hash store to the remote location asynchronously.
        // We cannot block the current Tokio runtime inside an async context,
        // so we spawn a background task to perform the upload.
        let client = self.client.clone();
        let local = self.local_path.clone();
        let remote = self.remote_path.clone();
        tokio::spawn(async move {
            if let Err(e) = client.upload_file(&local, &remote).await {
                eprintln!("Failed to upload hash store to remote: {}", e);
            }
        });
    }
}