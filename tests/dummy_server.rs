use reqwest::Client;
use std::process::Command;
use std::time::Duration;
use tokio::time::sleep;

/// Name of the test file used in the integration tests.
pub const TEST_FILE: &str = "test_file1.txt";

/// Starts the dummy WebDAV server, ensuring any previous instance is fully removed.
pub async fn start_dummy_webdav() {
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
    let client = Client::new();
    let url = "http://localhost:8080/";
    for _ in 0..10 {
        match client.get(url).basic_auth("TestAccount1", Some("TestPassword1")).send().await {
            Ok(resp) if resp.status().is_success() => break,
            _ => sleep(Duration::from_millis(500)).await,
        }
    }
    // Additional pause to ensure the WebDAV service is fully ready for PUT/GET operations.
    sleep(Duration::from_secs(2)).await;
}

/// Stops the dummy WebDAV server and removes all related resources.
pub fn stop_dummy_webdav() {
    let _ = Command::new("docker-compose")
        .args(["down", "--remove-orphans", "-v"])
        .current_dir("./dummy_webdav")
        .output();
}

/// Deletes a remote file from the dummy WebDAV server (ignores errors if absent).
pub async fn delete_remote_file(remote_path: &str) {
    let client = Client::new();
    let url = format!("http://localhost:8080/{}", remote_path);
    let _ = client.delete(&url).basic_auth("TestAccount1", Some("TestPassword1")).send().await;
}

/// Retrieves a remote file's content from the dummy WebDAV server.
pub async fn fetch_remote_file(remote_path: &str) -> Option<Vec<u8>> {
    let client = Client::new();
    let url = format!("http://localhost:8080/{}", remote_path);
    match client.get(&url).basic_auth("TestAccount1", Some("TestPassword1")).send().await {
        Ok(resp) if resp.status().is_success() => resp.bytes().await.ok().map(|b| b.to_vec()),
        _ => None,
    }
}

/// Reads the local test file's content.
pub fn read_local_test_file() -> Vec<u8> {
    std::fs::read(format!("./test_data/{}", TEST_FILE))
        .expect("Unable to read local test file")
}