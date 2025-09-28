use phone_sync::{config::Config, sync::sync};
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use tokio::time::sleep;
use reqwest::Client;
use serial_test::serial;
use ctor::{ctor, dtor};

const TEST_FILE: &str = "test_file1.txt";
const REMOTE_PATH: &str = "test_file1.txt";

/// Starts the dummy WebDAV server, ensuring any previous instance is fully removed.
async fn start_dummy_webdav() {
    // Bring down any existing containers and remove orphaned resources.
    let _ = Command::new("docker-compose")
        .args(["down", "--remove-orphans", "-v"])
        .current_dir("./dummy_webdav")
        .output();

    // Give Docker a moment to clean up.
    sleep(Duration::from_secs(1)).await;

    // Bring the containers up, forcing recreation to avoid name conflicts.
    let status = Command::new("docker-compose")
        .args(["up", "-d", "--force-recreate"])
        .current_dir("./dummy_webdav")
        .status()
        .expect("Failed to start docker-compose");
    assert!(status.success(), "Failed to start dummy WebDAV server");

    // Wait for the service to become ready.
    // Try a few times to GET the root URL; only proceed when we get a successful response.
    let client = Client::new();
    let url = "http://localhost:8080/";
    for _ in 0..10 {
        match client.get(url).send().await {
            Ok(resp) if resp.status().is_success() => break,
            _ => {
                // Not ready yet; wait a bit.
                sleep(Duration::from_millis(500)).await;
            }
        }
    }
    // Additional pause to ensure the WebDAV service is fully ready for PUT/GET operations.
    sleep(Duration::from_secs(2)).await;
}

/// Stops the dummy WebDAV server and removes all related resources.
fn stop_dummy_webdav() {
    let _ = Command::new("docker-compose")
        .args(["down", "--remove-orphans", "-v"])
        .current_dir("./dummy_webdav")
        .output();
}
    
/// Global test setup: start dummy WebDAV server once before any tests run.
#[ctor]
fn test_setup() {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    rt.block_on(async {
        start_dummy_webdav().await;
    });
}
/// Global test teardown: stop dummy WebDAV server after all tests have finished.
#[dtor]
fn test_teardown() {
    stop_dummy_webdav();
}


/// Deletes a remote file from the dummy WebDAV server (ignores errors if absent).
async fn delete_remote_file() {
    let client = Client::new();
    let url = format!("http://localhost:8080/{}", REMOTE_PATH);
    let _ = client.delete(&url).send().await;
}

/// Retrieves a remote file's content from the dummy WebDAV server.
async fn fetch_remote_file() -> Option<Vec<u8>> {
    let client = Client::new();
    let url = format!("http://localhost:8080/{}", REMOTE_PATH);
    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => resp.bytes().await.ok().map(|b| b.to_vec()),
        _ => None,
    }
}

/// Reads the local test file's content.
fn read_local_test_file() -> Vec<u8> {
    std::fs::read(format!("./test_data/{}", TEST_FILE)).expect("Unable to read local test file")
}

/// Test case: upload when the remote file is missing.
#[serial]
#[tokio::test]
async fn test_sync_upload_when_missing() {
    // Ensure a clean server state.
    // Server started globally by test_setup
    delete_remote_file().await; // Remote file should not exist.
    // Give the server a moment to process the deletion.
    sleep(Duration::from_secs(1)).await;

    // Load configuration.
    let config = Config::load("test_config.yaml").expect("Failed to load test config");

    // Ensure any previous hash store is removed so sync does not think the file is already up‑to‑date.
    let _ = std::fs::remove_file("hashes.yaml");

    // First sync: should upload the missing file.
    sync(&config).await.expect("Initial sync (upload) failed");
    // Give the server a moment to finalize the PUT operation.
    sleep(Duration::from_secs(1)).await;

    // Verify upload succeeded.
    let remote_content = fetch_remote_file()
        .await
        .expect("File was not uploaded to dummy WebDAV server");
    let local_content = read_local_test_file();
    assert_eq!(remote_content, local_content, "Uploaded file content differs");

    // Clean up.
    // Server stopped globally by test_teardown
}

/// Test case: no changes when the file is already present on the server.
#[serial]
#[tokio::test]
async fn test_sync_no_change_when_already_present() {
    // Ensure a clean server state.
    // Server started globally by test_setup

    // Load configuration.
    let config = Config::load("test_config.yaml").expect("Failed to load test config");

    // Ensure any previous hash store is removed so sync does not think the file is already up‑to‑date.
    let _ = std::fs::remove_file("hashes.yaml");

    // First sync: upload the file.
    sync(&config).await.expect("Initial sync (upload) failed");

    // Capture hash store after first sync.
    let hash_store_path = Path::new("hashes.yaml");
    let hash_store_before = std::fs::read_to_string(hash_store_path).expect("Unable to read hash store");

    // Second sync: no changes expected.
    sync(&config).await.expect("Second sync (no‑change) failed");

    // Verify hash store unchanged.
    let hash_store_after = std::fs::read_to_string(hash_store_path).expect("Unable to read hash store");
    assert_eq!(
        hash_store_before, hash_store_after,
        "Hash store changed despite no file modifications"
    );

    // Verify remote file still matches local content.
    let remote_content = fetch_remote_file()
        .await
        .expect("Remote file missing after second sync");
    let local_content = read_local_test_file();
    assert_eq!(remote_content, local_content, "Remote file content altered unexpectedly");

    // Clean up.
    // Server stopped globally by test_teardown
}