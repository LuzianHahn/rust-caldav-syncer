use phone_sync::{
    config::Config,
    hash_store::HashStore,
    sync::{sync, sync_with_progress},
    webdav_client::WebDavClient,
};
use std::fs;
use tempfile::NamedTempFile;
use tokio::time::{sleep, Duration};
use serial_test::serial;
use serde_yaml;

use ctor::{ctor, dtor};

#[ctor]
fn test_setup() {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    rt.block_on(async { start_dummy_webdav().await });
}

#[dtor]
fn test_teardown() {
    stop_dummy_webdav();
}
const TEST_CONFIG: &str = "test_config.yaml";

mod dummy_server;
use dummy_server::*;

/// Integration test verifying that `sync` correctly downloads and utilizes a remote
/// `hashes.yaml` file. The test performs an initial sync to upload the test file,
/// then uploads a matching remote `hashes.yaml`. After removing the local copy,
/// a second sync should download the remote hash store and avoid re‑uploading the
/// unchanged file.
#[tokio::test]
#[serial]
async fn test_sync_uses_remote_hashes_yaml() {
    // Ensure a clean remote state.
    delete_remote_file("hashes.yaml").await;
    delete_remote_file(TEST_FILE).await;
    // Clean any local hash store.
    let _ = fs::remove_file("hashes.yaml");

    // Load configuration.
    let config = Config::load(TEST_CONFIG).expect("load config");

    // -------------------------------------------------------------------------
    // First sync: upload the test file and generate a local hash store.
    // -------------------------------------------------------------------------
    sync(&config).await.expect("initial sync failed");

    // Compute the hash of the test file (should match the entry in the local store).
    let local_hash = HashStore::compute_hash(format!("./test_data/{}", TEST_FILE))
        .await
        .expect("failed to compute hash");

    // -------------------------------------------------------------------------
    // Upload a matching remote `hashes.yaml`.
    // -------------------------------------------------------------------------
    // Build a HashStore containing the correct hash.
    let mut remote_store = HashStore::default();
    remote_store
        .regular_hashes
        .insert(TEST_FILE.to_string(), local_hash.clone());

    // Serialize to YAML.
    let yaml = serde_yaml::to_string(&remote_store).expect("failed to serialize hash store");

    // Write to a temporary file for upload.
    let mut temp_file = NamedTempFile::new().expect("failed to create temp file");
    std::io::Write::write_all(&mut temp_file, yaml.as_bytes())
        .expect("failed to write temp hash store");

    // Create a WebDav client and upload the remote hash store.
    let client = WebDavClient::new(
        &config.webdav_url,
        config.username.as_deref(),
        config.password.as_deref(),
        config.timeout_secs,
    )
    .expect("failed to create WebDav client");
    client
        .upload_file(temp_file.path(), &config.remote_hash_path)
        .await
        .expect("failed to upload remote hashes.yaml");

    // Ensure the server processes the uploaded hashes.yaml before the next sync.
    sleep(Duration::from_secs(1)).await;

    // -------------------------------------------------------------------------
    // Remove the local hash store to force a download on the next sync.
    // -------------------------------------------------------------------------
    let _ = fs::remove_file("hashes.yaml");

    // -------------------------------------------------------------------------
    // Second sync: should download the remote `hashes.yaml` and skip re‑upload.
    // -------------------------------------------------------------------------
    sync(&config).await.expect("second sync failed");

    // Verify that the local hash store now matches the remote one.
    let local_store = HashStore::load("hashes.yaml").expect("failed to load local hash store");
    assert_eq!(
        local_store.regular_hashes.get(TEST_FILE).unwrap(),
        &local_hash,
        "Local hash store does not match remote hash store"
    );

    // Ensure the remote test file still exists and has the expected content.
    let remote_content = fetch_remote_file(TEST_FILE)
        .await
        .expect("remote test file missing after second sync");
    let local_content = read_local_test_file();
    assert_eq!(
        remote_content, local_content,
        "Remote file content differs after second sync"
    );
}

#[tokio::test]
#[serial]
async fn test_sync_uses_remote_pseudo_hashes_yaml() {
    // Clean remote and local state.
    delete_remote_file("hashes.yaml").await;
    delete_remote_file(TEST_FILE).await;
    let _ = fs::remove_file("hashes.yaml");

    // Load configuration.
    let config = Config::load(TEST_CONFIG).expect("load config");

    // -------------------------------------------------------------------------
    // First sync: upload the test file and generate a local pseudo‑hash store.
    // -------------------------------------------------------------------------
    sync_with_progress(&config, false, true)
        .await
        .expect("initial pseudo sync failed");

    // Compute the pseudo‑hash of the test file.
    let local_hash = HashStore::compute_pseudo_hash(format!("./test_data/{}", TEST_FILE))
        .await
        .expect("failed to compute pseudo hash");

    // -------------------------------------------------------------------------
    // Upload a matching remote `hashes.yaml` containing the pseudo‑hash.
    // -------------------------------------------------------------------------
    let mut remote_store = HashStore::default();
    remote_store
        .pseudo_hashes
        .insert(TEST_FILE.to_string(), local_hash.clone());

    // Serialize to YAML.
    let yaml = serde_yaml::to_string(&remote_store).expect("failed to serialize hash store");

    // Write to a temporary file for upload.
    let mut temp_file = NamedTempFile::new().expect("failed to create temp file");
    std::io::Write::write_all(&mut temp_file, yaml.as_bytes())
        .expect("failed to write temp hash store");

    // Upload the remote hash store.
    let client = WebDavClient::new(
        &config.webdav_url,
        config.username.as_deref(),
        config.password.as_deref(),
        config.timeout_secs,
    )
    .expect("failed to create WebDav client");
    client
        .upload_file(temp_file.path(), &config.remote_hash_path)
        .await
        .expect("failed to upload remote hashes.yaml");

    // Give the server a moment to process the upload.
    sleep(Duration::from_secs(1)).await;

    // -------------------------------------------------------------------------
    // Remove the local hash store to force a download on the next sync.
    // -------------------------------------------------------------------------
    let _ = fs::remove_file("hashes.yaml");

    // -------------------------------------------------------------------------
    // Second sync: should download the remote `hashes.yaml` and skip re‑upload.
    // -------------------------------------------------------------------------
    sync_with_progress(&config, false, true)
        .await
        .expect("second pseudo sync failed");

    // Verify that the local store now matches the remote pseudo‑hash.
    let local_store = HashStore::load("hashes.yaml").expect("failed to load local hash store");
    assert_eq!(
        local_store
            .pseudo_hashes
            .get(TEST_FILE)
            .unwrap(),
        &local_hash,
        "Local pseudo‑hash store does not match remote pseudo‑hash store"
    );

    // Ensure the remote test file still exists and has the expected content.
    let remote_content = fetch_remote_file(TEST_FILE)
        .await
        .expect("remote test file missing after second sync");
    let local_content = read_local_test_file();
    assert_eq!(
        remote_content, local_content,
        "Remote file content differs after second sync"
    );
}