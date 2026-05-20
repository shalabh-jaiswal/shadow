# Google Drive Provider — Implementation Plan & Architecture Design

This document details the architectural decisions, data flows, and code structures required to integrate Google Drive (GDrive) as a background backup provider in the Shadow app. 

This design aims to provide a zero-configuration, 1-click authentication experience for non-technical users while adhering to strict local security standards.

---

## 1. Core Architectural Decisions

### A. OAuth 2.0 Desktop Loopback Server Flow
To connect a user's personal Google Drive without requiring them to set up Google Cloud Platform (GCP) credentials:
1. **Loopback Server:** The Tauri Rust backend starts a temporary local TCP listener (e.g., `http://127.0.0.1:40003` or dynamically allocated port) to receive the OAuth redirect code.
2. **System Browser Open:** The app opens the system browser to the Google OAuth Consent URL using Tauri's shell features.
3. **Token Exchange:** Once the user logs in and consents, Google redirects their browser to the local server, which captures the `code` parameter, shuts down the listener, and exchanges the code for tokens.

### B. Secure Token Storage (OS Keyring)
In compliance with Shadow's architecture guidelines, user credentials (the persistent **Refresh Token**) must never be written to `config.toml` or the local `sled` database in plaintext.
* **Storage Solution:** We use the Rust `keyring` crate to store the refresh token in the platform's native encrypted secure vault:
  - **macOS:** Apple Keychain Service
  - **Windows:** Windows Credential Manager
  - **Linux:** Secret Service / DBus Secret Service
* **Key Name:** Service: `"shadow-backup-gdrive"`, Account: `"user-refresh-token"`.

### C. Build-Time Environment Injection for Developer Keys
To keep the developer's Client ID and Client Secret out of the public Git repository:
* **Mechanism:** The Rust backend loads these secrets during the compilation phase using `env!("SHADOW_GDRIVE_CLIENT_ID")` and `env!("SHADOW_GDRIVE_CLIENT_SECRET")`.
* **Local Setup:** A `.env` file (added to `.gitignore`) is created on the developer machine containing these keys.
* **CI/CD:** Keys are stored as GitHub Repository Secrets and injected as environment variables during release compilation.

### D. File-Level Access Scope (`drive.file`)
The app requests the restricted `https://www.googleapis.com/auth/drive.file` scope.
* **Security Guard:** This ensures the application only has permission to view, edit, and delete folders/files *that the Shadow app itself has created*. It is blocked from reading the user's other personal Google Drive documents.
* **Verification Benefit:** Using this scope makes Google's App Verification process simple and free of costly third-party security audits.

### E. Graph-Based Folder Resolution (Node mapping)
Google Drive is a node-based file system using IDs, not flat absolute paths.
* **Directory Creation:** To upload a file to `Shadow/MyLaptop/Documents/file.txt`, the GDrive provider must recursively create/query directories to get their unique folder IDs (mimeType: `application/vnd.google-apps.folder`), building the tree.
* **Caching:** To avoid excessive API queries on every file upload, we cache local folder path strings to Google Drive folder ID mappings within an in-memory map and/or the local `sled` database.

---

## 2. Planned Code Implementations

### Backend (Rust)

#### 1. Dependencies (`src-tauri/Cargo.toml`)
```toml
keyring = "2.1"
reqwest = { version = "0.11", features = ["json", "multipart"] }
url = "2.5"
```

#### 2. Configuration (`src-tauri/src/config.rs`)
Add `GdriveConfig` struct and wire it to `AppConfig`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GdriveConfig {
    pub enabled: bool,
    pub root_folder_id: String, // Cached root backup folder ID on GDrive
    pub prefix: String,         // Optional user prefix
}
```

#### 3. Secure Token Helpers (`src-tauri/src/keyring_utils.rs`)
Manage OS Keychain read/write tasks:
```rust
use keyring::Entry;

pub fn save_refresh_token(token: &str) -> anyhow::Result<()> {
    let entry = Entry::new("shadow-backup-gdrive", "user-refresh-token")?;
    entry.set_password(token)?;
    Ok(())
}

pub fn get_refresh_token() -> anyhow::Result<String> {
    let entry = Entry::new("shadow-backup-gdrive", "user-refresh-token")?;
    Ok(entry.get_password()?)
}

pub fn delete_refresh_token() -> anyhow::Result<()> {
    let entry = Entry::new("shadow-backup-gdrive", "user-refresh-token")?;
    entry.delete_password()?;
    Ok(())
}
```

#### 4. OAuth Server (`src-tauri/src/oauth.rs`)
* Construct the consent URL containing:
  - `response_type=code`
  - `client_id`
  - `redirect_uri=http://127.0.0.1:40003`
  - `scope=https://www.googleapis.com/auth/drive.file`
  - `access_type=offline` & `prompt=consent` (ensures a Refresh Token is returned)
* Start local `TcpListener` on port `40003`, wait for response, extract the `code`, and exchange it via standard HTTPS POST with Google's endpoint: `https://oauth2.googleapis.com/token`.

#### 5. Google Drive Provider (`src-tauri/src/providers/gdrive.rs`)
Implement the `BackupProvider` trait:
```rust
pub struct GdriveProvider {
    client: reqwest::Client,
    // Holds the currently active (in-memory) access token and expiration time
    access_token: Arc<RwLock<Option<String>>>,
    token_expiry: Arc<RwLock<Option<std::time::Instant>>>,
}

#[async_trait::async_trait]
impl BackupProvider for GdriveProvider {
    fn name(&self) -> &'static str { "GDrive" }
    
    async fn upload(&self, local_path: &Path, remote_key: &str) -> anyhow::Result<()> {
        let token = self.get_valid_access_token().await?;
        let folder_id = self.resolve_parent_folder_id(remote_key, &token).await?;
        self.upload_file_to_folder(local_path, folder_id, &token).await?;
        Ok(())
    }
    
    async fn rename(&self, old_remote_key: &str, new_remote_key: &str) -> anyhow::Result<()> {
        // Find existing file ID, resolve new folder ID, update file parent metadata
        Ok(())
    }
    
    async fn test_connection(&self) -> anyhow::Result<String> {
        let token = self.get_valid_access_token().await?;
        // Lightweight call to search files with drive.file scope
        Ok("Connected".into())
    }
}
```

---

## 3. Frontend (React & TypeScript)

### Configuration Configs (`src/types.ts`)
```typescript
export interface GdriveConfig {
  enabled: boolean;
  root_folder_id: string;
  prefix: string;
}
```

### UI Screens (`src/components/screens/Providers.tsx`)
Render a Google Drive integration card in the providers list:
* **OAuth Button:** If the keyring has no token, display **[ Connect Google Drive ]**. When clicked, invoke `start_gdrive_auth`, which opens the browser and returns success when the flow completes.
* **Disconnect:** If authorized, show a success status badge and a **[ Disconnect ]** button (which calls `disconnect_gdrive` to wipe keys from the local OS vault).
