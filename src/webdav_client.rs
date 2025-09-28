use log::info;
use reqwest::{Client, Method, StatusCode};
use std::path::Path;
use tokio::fs as async_fs;

pub struct WebDavClient {
    client: Client,
    base_url: String,
    username: Option<String>,
    password: Option<String>,
}

impl WebDavClient {
    pub fn new(url: &str, username: Option<&str>, password: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> {
        let client = Client::new();
        Ok(Self {
            client,
            base_url: url.to_string(),
            username: username.map(|s| s.to_string()),
            password: password.map(|s| s.to_string()),
        })
    }

    // Ensure that a remote directory exists, creating it via MKCOL if necessary.
    async fn ensure_remote_dir(&self, remote_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
        if remote_dir.is_empty() {
            return Ok(());
        }

        let dir_url = format!("{}/{}/", self.base_url.trim_end_matches('/'), remote_dir.trim_end_matches('/'));
        let mut req = self.client.request(Method::from_bytes(b"MKCOL")?, &dir_url);
        if let (Some(user), Some(pass)) = (&self.username, &self.password) {
            req = req.basic_auth(user, Some(pass));
        }

        let resp = req.send().await?;
        let status = resp.status();
        if !status.is_success()
            && status != StatusCode::METHOD_NOT_ALLOWED
            && status != StatusCode::CONFLICT
        {
            let txt = resp.text().await.unwrap_or_default();
            return Err(format!(
                "Failed to create remote directory '{}': {} - {}",
                remote_dir,
                status,
                txt
            )
            .into());
        }
        Ok(())
    }

    pub async fn upload_file<P: AsRef<Path>>(
        &self,
        local_path: P,
        remote_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let content = async_fs::read(&local_path).await?;

        // Ensure the remote directory hierarchy exists
        if let Some(parent) = std::path::Path::new(remote_path).parent() {
            if let Some(dir_str) = parent.to_str() {
                self.ensure_remote_dir(dir_str).await?;
            }
        }

        // Ensure any existing remote file is removed before uploading (WebDAV PUT may not overwrite).
        let del_url = format!("{}/{}", self.base_url.trim_end_matches('/'), remote_path);
        let _ = self.client.delete(&del_url).send().await;
        let url = format!("{}/{}", self.base_url.trim_end_matches('/'), remote_path);
        let mut request = self.client.put(&url).body(content);
        if let (Some(user), Some(pass)) = (&self.username, &self.password) {
            request = request.basic_auth(user, Some(pass));
        }
        request.send().await?;
        info!("Uploaded {} to {}", local_path.as_ref().display(), remote_path);
        Ok(())
    }
}