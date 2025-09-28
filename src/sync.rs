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
        config.timeout_secs,
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

        // Collect all file entries first to control upload order.
        let mut file_entries: Vec<_> = WalkDir::new(folder)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .collect();

        // Sort entries so that deeper (nested) files are uploaded before shallower ones.
        file_entries.sort_by_key(|e| {
            e.path()
                .strip_prefix(folder_path)
                .ok()
                .map(|p| p.components().count())
                .unwrap_or(0)
        });
        // Reverse to have deepest paths first.
        file_entries.reverse();

        for entry in file_entries {
            let local_path = entry.path();
            let relative_path = local_path.strip_prefix(folder_path)?.to_string_lossy();

            let current_hash = HashStore::compute_hash(local_path).await?;
            let remote_path = relative_path.as_ref();

            // Always upload the file; hash store will be updated accordingly.

            // upload
            client.upload_file(local_path, remote_path).await?;

            // update hash
            hash_store.hashes.insert(remote_path.to_string(), current_hash);
        }
    }

    hash_store.save(hash_store_path)?;
    Ok(())
}