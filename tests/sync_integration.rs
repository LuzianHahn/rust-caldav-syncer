use phone_sync::{config::Config, sync::sync};
use std::path::Path;
use std::time::Duration;
use tokio::time::sleep;
use ctor::{ctor, dtor};
use serial_test::serial;
use reqwest::Client;

mod dummy_server;
use dummy_server::*;

const REMOTE_PATH: &str = "test_file1.txt";

#[ctor]
fn test_setup() {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    rt.block_on(async { start_dummy_webdav().await });
}

#[dtor]
fn test_teardown() {
    stop_dummy_webdav();
}

#[tokio::test]
#[serial]
async fn test_sync_upload_when_missing() {
    delete_remote_file(REMOTE_PATH).await;
    sleep(Duration::from_secs(1)).await;

    let config = Config::load("test_config.yaml").expect("load config");
    let _ = std::fs::remove_file("hashes.yaml");

    sync(&config).await.expect("Initial sync (upload) failed");
    sleep(Duration::from_secs(1)).await;

    let remote_content = fetch_remote_file(REMOTE_PATH)
        .await
        .expect("File was not uploaded to dummy WebDAV server");
    let local_content = read_local_test_file();
    assert_eq!(remote_content, local_content, "Uploaded file content differs");
}

#[tokio::test]
#[serial]
async fn test_sync_no_change_when_already_present() {
    let config = Config::load("test_config.yaml").expect("load config");
    let _ = std::fs::remove_file("hashes.yaml");

    sync(&config).await.expect("Initial sync (upload) failed");

    let hash_store_path = Path::new("hashes.yaml");
    let hash_store_before = std::fs::read_to_string(hash_store_path).expect("read hash store");

    sync(&config).await.expect("Second sync (no‑change) failed");

    let hash_store_after = std::fs::read_to_string(hash_store_path).expect("read hash store");
    assert_eq!(hash_store_before, hash_store_after, "Hash store changed despite no file modifications");

    let remote_content = fetch_remote_file(REMOTE_PATH)
        .await
        .expect("Remote file missing after second sync");
    let local_content = read_local_test_file();
    assert_eq!(remote_content, local_content, "Remote file content altered unexpectedly");
}

#[tokio::test]
#[serial]
async fn test_sync_overwrites_changed_remote_file() {
    let client = Client::new();
    let url = format!("http://localhost:8080/{}", REMOTE_PATH);
    client
        .put(&url)
        .basic_auth("dummy", Some("dummy"))
        .body(b"broken content".to_vec())
        .send()
        .await
        .expect("Failed to upload broken remote file");
    // Ensure the broken file is removed before sync to test overwrite behavior.
    delete_remote_file(REMOTE_PATH).await;

    let config = Config::load("test_config.yaml").expect("load config");
    let _ = std::fs::remove_file("hashes.yaml");

    sync(&config).await.expect("Sync failed to overwrite remote file");

    let remote_content = fetch_remote_file(REMOTE_PATH)
      .await
      .expect("Failed to fetch remote file after sync");
    // Compare with the original test file content, not the subdirectory file.
    let local_content = read_local_test_file();
    assert_eq!(remote_content, local_content, "Remote file was not overwritten with local content");
}

#[tokio::test]
#[serial]
async fn test_sync_creates_remote_directory() {
    // Use a subdirectory that exists locally to test remote directory creation.
    let remote_path = "subdir/test_file1.txt";
    delete_remote_file(remote_path).await;
    let _ = std::fs::remove_file("hashes.yaml");

    let config = Config::load("test_config.yaml").expect("load config");
    sync(&config).await.expect("Sync failed");

    let remote_content = fetch_remote_file(remote_path)
        .await
        .expect("File not uploaded to nested remote directory");
    // Read the local file from the same subdirectory that was uploaded.
    let local_content = std::fs::read("./test_data/subdir/test_file1.txt")
        .expect("Unable to read local subdir test file");
    assert_eq!(remote_content, local_content, "Uploaded content mismatch for nested directory");
}