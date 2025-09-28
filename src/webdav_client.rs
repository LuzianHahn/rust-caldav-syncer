use log::info;
use reqwest::Client;
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

    pub async fn upload_file<P: AsRef<Path>>(
        &self,
        local_path: P,
        remote_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let content = async_fs::read(&local_path).await?;
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