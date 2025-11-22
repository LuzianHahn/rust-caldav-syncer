use crate::config::Config;
use crate::hash_store::HashStore;
use crate::webdav_client::WebDavClient;
use crate::hash_store_guard::HashStoreGuard;
use indicatif::{ProgressBar, ProgressStyle};
use log::warn;
use std::path::Path;
use walkdir::WalkDir;

pub async fn sync(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    // Backward‑compatible wrapper without progress bar
    sync_with_progress(config, false, false).await
}

pub async fn sync_with_progress(
    config: &Config,
    show_progress: bool,
    use_pseudo_hash: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = WebDavClient::new(
        &config.webdav_url,
        config.username.as_deref(),
        config.password.as_deref(),
        config.timeout_secs,
    )?;

    // Path to local hash store file
    let hash_store_path = &config.hash_store_path;
    // Initialize guard which loads the remote hash store and prepares for syncing.
    let mut guard = HashStoreGuard::new(client.clone(), config).await?;
    let hash_store = guard.hash_store_mut();
    // Determine the file name of the local hash store so it can be ignored during sync.
    let hash_store_file_name = std::path::Path::new(hash_store_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    // Calculate total number of files for progress bar
    let total_files: usize = config
        .folders
        .iter()
        .filter_map(|folder| {
            let folder_path = Path::new(folder);
            if !folder_path.exists() {
                return None;
            }
            Some(
                WalkDir::new(folder)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                    .count(),
            )
        })
        .sum();

    let progress_bar: Option<ProgressBar> = if show_progress {
        let pb = ProgressBar::new(total_files as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({eta})")?
                .progress_chars("=> "),
        );
        pb.set_message("Syncing files");
        Some(pb)
    } else {
        None
    };

    for folder in &config.folders {
        let folder_path = Path::new(folder);
        if !folder_path.exists() {
            warn!("Folder {} does not exist, skipping", folder);
            continue;
        }

        // Collect file entries
        let mut file_entries: Vec<_> = WalkDir::new(folder)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .collect();

        // Sort deeper files first
        file_entries.sort_by_key(|e| {
            e.path()
                .strip_prefix(folder_path)
                .ok()
                .map(|p| p.components().count())
                .unwrap_or(0)
        });
        file_entries.reverse();

        for entry in file_entries {
            let local_path = entry.path();
            let relative_path = local_path.strip_prefix(folder_path)?.to_string_lossy();

            // Skip the hash store file itself to avoid uploading it.
            if entry.file_name().to_string_lossy() == hash_store_file_name {
                if let Some(pb) = &progress_bar {
                    pb.inc(1);
                }
                continue;
            }

            let current_hash = if use_pseudo_hash {
                HashStore::compute_pseudo_hash(local_path).await?
            } else {
                HashStore::compute_hash(local_path).await?
            };
            let remote_path = if config.target_dir.is_empty() {
                relative_path.to_string()
            } else {
                format!("{}/{}", config.target_dir.trim_end_matches('/'), relative_path)
            };
            
            // If the file's hash matches the stored hash, skip uploading.
            let remote_exists = client.file_exists(&remote_path).await?;
            let stored_hash = if use_pseudo_hash {
                hash_store.pseudo_hashes.get(&remote_path)
            } else {
                hash_store.regular_hashes.get(&remote_path)
            };
            if remote_exists && stored_hash == Some(&current_hash) {
                // Still update the progress bar to reflect that the file was processed.
                if let Some(pb) = &progress_bar {
                    pb.inc(1);
                }
                continue;
            }
            
            // upload
            client.upload_file(local_path, &remote_path).await?;
            
            // update progress bar
            if let Some(pb) = &progress_bar {
                pb.inc(1);
            }
            
            // update hash
            if use_pseudo_hash {
                hash_store
                    .pseudo_hashes
                    .insert(remote_path.to_string(), current_hash);
            } else {
                hash_store
                    .regular_hashes
                    .insert(remote_path.to_string(), current_hash);
            }
        }
    }

    if let Some(pb) = progress_bar {
        pb.finish_with_message("Sync complete");
    }
    // Ensure the hash store is saved and uploaded before returning.
    guard.finalize().await?;


    Ok(())
}