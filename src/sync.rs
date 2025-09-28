use crate::config::Config;
use crate::hash_store::HashStore;
use crate::webdav_client::WebDavClient;
use log::warn;
use std::path::Path;
use walkdir::WalkDir;

pub async fn sync(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let client = WebDavClient::new(
        &config.webdav_url,
        config.username.as_deref(),
        config.password.as_deref(),
    )?;

    // Use configurable hash store path from config (defaults to "hashes.yaml")
    let hash_store_path = &config.hash_store_path;
    let mut hash_store = HashStore::load(hash_store_path)?;

    for folder in &config.folders {
        let folder_path = Path::new(folder);
        if !folder_path.exists() {
            warn!("Folder {} does not exist, skipping", folder);
            continue;
        }

        for entry in WalkDir::new(folder).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                let local_path = entry.path();
                let relative_path = local_path.strip_prefix(folder_path)?.to_string_lossy();

                let current_hash = HashStore::compute_hash(local_path).await?;
                let remote_path = relative_path.as_ref();

                if let Some(stored_hash) = hash_store.hashes.get(remote_path) {
                    if stored_hash == &current_hash {
                        continue; // no change
                    }
                }

                // upload
                client.upload_file(local_path, remote_path).await?;

                // update hash
                hash_store.hashes.insert(remote_path.to_string(), current_hash);
            }
        }
    }

    hash_store.save(hash_store_path)?;
    Ok(())
}