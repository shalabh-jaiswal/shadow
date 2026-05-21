use crate::keyring_utils;
use crate::oauth;
use crate::providers::BackupProvider;
use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use url::Url;

pub struct GdriveProvider {
    client: reqwest::Client,
    access_token: Arc<RwLock<Option<String>>>,
    token_expiry: Arc<RwLock<Option<Instant>>>,
    refresh_mutex: Arc<Mutex<()>>,
    folder_cache: Arc<RwLock<HashMap<String, String>>>,
    root_folder_name: String,
}

impl GdriveProvider {
    pub fn new(root_folder_name: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            access_token: Arc::new(RwLock::new(None)),
            token_expiry: Arc::new(RwLock::new(None)),
            refresh_mutex: Arc::new(Mutex::new(())),
            folder_cache: Arc::new(RwLock::new(HashMap::new())),
            root_folder_name: if root_folder_name.is_empty() {
                "Shadow".to_string()
            } else {
                root_folder_name.to_string()
            },
        }
    }

    /// Fetches a valid, unexpired access token. Refreshes if necessary.
    async fn get_valid_access_token(&self) -> Result<String> {
        // Read lock check
        {
            let token_opt = self.access_token.read().await;
            let expiry_opt = self.token_expiry.read().await;
            if let (Some(token), Some(expiry)) = (&*token_opt, &*expiry_opt) {
                if *expiry > Instant::now() + Duration::from_secs(300) {
                    return Ok(token.clone());
                }
            }
        }

        // Acquire lock to refresh
        let _guard = self.refresh_mutex.lock().await;

        // Double check inside lock
        {
            let token_opt = self.access_token.read().await;
            let expiry_opt = self.token_expiry.read().await;
            if let (Some(token), Some(expiry)) = (&*token_opt, &*expiry_opt) {
                if *expiry > Instant::now() + Duration::from_secs(300) {
                    return Ok(token.clone());
                }
            }
        }

        let refresh_token = keyring_utils::get_refresh_token().map_err(|e| {
            anyhow!(
                "No Google Drive credentials found in OS vault: {}. Please connect Google Drive in Settings.",
                e
            )
        })?;

        let token_resp = oauth::refresh_access_token(&refresh_token).await?;

        let mut token_lock = self.access_token.write().await;
        let mut expiry_lock = self.token_expiry.write().await;

        *token_lock = Some(token_resp.access_token.clone());
        *expiry_lock = Some(Instant::now() + Duration::from_secs(token_resp.expires_in));

        Ok(token_resp.access_token)
    }

    /// Searches for a file or folder in Google Drive.
    async fn search_item(
        &self,
        name: &str,
        parent_id: &str,
        mime_type: Option<&str>,
        token: &str,
    ) -> Result<Option<String>> {
        // Escape single quotes in folder/file names to prevent syntax errors
        let escaped_name = name.replace('\'', "\\'");
        let mut query = format!(
            "'{}' in parents and name = '{}' and trashed = false",
            parent_id, escaped_name
        );
        if let Some(mt) = mime_type {
            query.push_str(&format!(" and mimeType = '{}'", mt));
        }

        let url = Url::parse_with_params(
            "https://www.googleapis.com/drive/v3/files",
            &[("q", query.as_str()), ("fields", "files(id, name)")],
        )?;

        let response: reqwest::Response = self
            .client
            .get(url)
            .bearer_auth(token)
            .send()
            .await?;

        if !response.status().is_success() {
            let err_text = response.text().await?;
            return Err(anyhow!("Google Drive search failed: {}", err_text));
        }

        #[derive(Deserialize)]
        struct FileId {
            id: String,
        }
        #[derive(Deserialize)]
        struct FileList {
            files: Vec<FileId>,
        }

        let list: FileList = response.json().await?;
        Ok(list.files.first().map(|f| f.id.clone()))
    }

    /// Creates a folder under a parent folder.
    async fn create_folder(&self, name: &str, parent_id: &str, token: &str) -> Result<String> {
        let url = "https://www.googleapis.com/drive/v3/files";
        let body = serde_json::json!({
            "name": name,
            "mimeType": "application/vnd.google-apps.folder",
            "parents": [parent_id]
        });

        let response: reqwest::Response = self
            .client
            .post(url)
            .bearer_auth(token)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let err_text = response.text().await?;
            return Err(anyhow!("Google Drive folder creation failed: {}", err_text));
        }

        #[derive(Deserialize)]
        struct CreatedFolder {
            id: String,
        }

        let folder: CreatedFolder = response.json().await?;
        Ok(folder.id)
    }

    /// Resolves the nested folder path to a GDrive folder ID.
    async fn resolve_parent_folder_id(&self, remote_key: &str, token: &str) -> Result<String> {
        let parts: Vec<&str> = remote_key.split('/').collect();
        if parts.len() <= 1 {
            return self.get_or_create_root_folder_id(token).await;
        }

        let folder_parts = &parts[..parts.len() - 1];
        let mut current_parent_id = self.get_or_create_root_folder_id(token).await?;
        let mut current_path = String::new();

        for part in folder_parts {
            if part.is_empty() {
                continue;
            }
            if current_path.is_empty() {
                current_path = part.to_string();
            } else {
                current_path.push_str("/");
                current_path.push_str(part);
            }

            // Check cache
            {
                let cache = self.folder_cache.read().await;
                if let Some(id) = cache.get(&current_path) {
                    current_parent_id = id.clone();
                    continue;
                }
            }

            // Mutex-like sync to prevent duplicate folder creation during concurrent uploads
            let folder_id = match self
                .search_item(
                    part,
                    &current_parent_id,
                    Some("application/vnd.google-apps.folder"),
                    token,
                )
                .await?
            {
                Some(id) => id,
                None => self.create_folder(part, &current_parent_id, token).await?,
            };

            let mut cache = self.folder_cache.write().await;
            cache.insert(current_path.clone(), folder_id.clone());
            current_parent_id = folder_id;
        }

        Ok(current_parent_id)
    }

    /// Gets or creates the root Shadow folder in Google Drive.
    async fn get_or_create_root_folder_id(&self, token: &str) -> Result<String> {
        {
            let cache = self.folder_cache.read().await;
            if let Some(id) = cache.get("") {
                return Ok(id.clone());
            }
        }

        let root_id = match self
            .search_item(
                &self.root_folder_name,
                "root",
                Some("application/vnd.google-apps.folder"),
                token,
            )
            .await?
        {
            Some(id) => id,
            None => {
                self.create_folder(&self.root_folder_name, "root", token)
                    .await?
            }
        };

        let mut cache = self.folder_cache.write().await;
        cache.insert("".to_string(), root_id.clone());
        Ok(root_id)
    }
}

#[async_trait::async_trait]
impl BackupProvider for GdriveProvider {
    fn name(&self) -> &'static str {
        "gdrive"
    }

    async fn upload(&self, local_path: &Path, remote_key: &str) -> Result<()> {
        let token = self.get_valid_access_token().await?;
        let parent_id = self.resolve_parent_folder_id(remote_key, &token).await?;
        let filename = remote_key.split('/').last().ok_or_else(|| {
            anyhow!("Invalid remote key: cannot extract filename")
        })?;

        // 1. Check if file already exists
        let existing_id = self.search_item(filename, &parent_id, None, &token).await?;

        let file_data = tokio::fs::read(local_path).await?;

        if let Some(id) = existing_id {
            // 2. Overwrite file contents (PATCH media)
            let url = format!(
                "https://www.googleapis.com/upload/drive/v3/files/{}?uploadType=media",
                id
            );
            let response: reqwest::Response = self
                .client
                .patch(&url)
                .bearer_auth(&token)
                .header("Content-Type", "application/octet-stream")
                .body(file_data)
                .send()
                .await?;

            if !response.status().is_success() {
                let err_text = response.text().await?;
                return Err(anyhow!("Failed to update Google Drive file: {}", err_text));
            }
        } else {
            // 3. Create new file (POST multipart)
            let url = "https://www.googleapis.com/upload/drive/v3/files?uploadType=multipart";
            let metadata = serde_json::json!({
                "name": filename,
                "parents": [parent_id]
            });

            let form = reqwest::multipart::Form::new()
                .part(
                    "metadata",
                    reqwest::multipart::Part::text(metadata.to_string())
                        .mime_str("application/json; charset=UTF-8")?,
                )
                .part(
                    "media",
                    reqwest::multipart::Part::bytes(file_data)
                        .mime_str("application/octet-stream")?,
                );

            let response: reqwest::Response = self
                .client
                .post(url)
                .bearer_auth(&token)
                .multipart(form)
                .send()
                .await?;

            if !response.status().is_success() {
                let err_text = response.text().await?;
                return Err(anyhow!("Failed to upload Google Drive file: {}", err_text));
            }
        }

        Ok(())
    }

    async fn rename(&self, old_remote_key: &str, new_remote_key: &str) -> Result<()> {
        let token = self.get_valid_access_token().await?;

        let old_filename = old_remote_key.split('/').last().ok_or_else(|| {
            anyhow!("Invalid old remote key: cannot extract filename")
        })?;
        let old_parent_id = self.resolve_parent_folder_id(old_remote_key, &token).await?;

        // Find file ID
        let file_id = self
            .search_item(old_filename, &old_parent_id, None, &token)
            .await?
            .ok_or_else(|| {
                anyhow!(
                    "Source file '{}' not found in GDrive folder '{}' to perform rename",
                    old_filename,
                    old_parent_id
                )
            })?;

        let new_filename = new_remote_key.split('/').last().ok_or_else(|| {
            anyhow!("Invalid new remote key: cannot extract filename")
        })?;
        let new_parent_id = self.resolve_parent_folder_id(new_remote_key, &token).await?;

        let mut url = format!("https://www.googleapis.com/drive/v3/files/{}", file_id);

        // Handle moving directory parents if changed
        if old_parent_id != new_parent_id {
            url.push_str(&format!(
                "?addParents={}&removeParents={}",
                new_parent_id, old_parent_id
            ));
        }

        let body = serde_json::json!({
            "name": new_filename
        });

        let response: reqwest::Response = self
            .client
            .patch(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let err_text = response.text().await?;
            return Err(anyhow!("Failed to rename/move Google Drive file: {}", err_text));
        }

        Ok(())
    }

    async fn test_connection(&self) -> Result<String> {
        let token = self.get_valid_access_token().await?;
        let root_id = self.get_or_create_root_folder_id(&token).await?;
        Ok(format!("Google Drive Connected. Root ID: {}", root_id))
    }
}
