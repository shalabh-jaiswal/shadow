# Shadow — Product Requirements Document
**Version:** 1.0  
**Date:** 2026  
**Platform:** macOS | Windows | Linux  
**Stack:** Tauri 2 | Rust | React | TypeScript  
**Status:** Ready for Development  
**Classification:** Confidential — For Development Use

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Goals & Non-Goals](#2-goals--non-goals)
3. [Users & Use Cases](#3-users--use-cases)
4. [System Architecture](#4-system-architecture)
5. [Functional Requirements](#5-functional-requirements)
6. [Non-Functional Requirements](#6-non-functional-requirements)
7. [Technology Stack](#7-technology-stack)
8. [Data Model & Configuration](#8-data-model--configuration)
9. [Upload Pipeline — Detailed Flow](#9-upload-pipeline--detailed-flow)
10. [Remote Path Construction](#10-remote-path-construction)
11. [Tauri IPC API Contract](#11-tauri-ipc-api-contract)
12. [UI Screen Specifications](#12-ui-screen-specifications)
13. [CI/CD — GitHub Actions Build Matrix](#13-cicd--github-actions-build-matrix)
14. [Recommended Project Structure](#14-recommended-project-structure)
15. [Development Milestones](#15-development-milestones)
16. [Open Questions & Decisions](#16-open-questions--decisions)
17. [Glossary](#17-glossary)

---

## 1. Executive Summary

Shadow is a lightweight, real-time file backup desktop application built with Tauri 2 (Rust backend) and a React/TypeScript frontend. It continuously watches user-designated folders on macOS, Windows, and Linux and automatically backs up any new or modified file to one or more configurable remote destinations: Amazon S3, Google Cloud Storage (GCS), or a Network Attached Storage (NAS) mount point.

The application runs as a lean background daemon with near-zero idle resource consumption. Files are uploaded with sub-second latency from detection to upload start. On first run, Shadow performs a full recursive backup of all watched folders; thereafter it operates incrementally, only uploading files whose content has changed. Deletions are intentionally not propagated to remote storage.

---

## 2. Goals & Non-Goals

### 2.1 Goals

- Detect file creation and modification events using OS-native kernel APIs — no polling.
- Begin uploading within 500ms of a file write completing.
- Support three backup destinations: AWS S3, Google Cloud Storage, NAS mount point.
- Allow the user to enable any combination of the three destinations simultaneously.
- Perform a full recursive backup on first launch for each watched folder.
- Incrementally back up only changed files on subsequent runs using content hashing (blake3).
- Present a clean desktop UI for managing watched folders, viewing status, and configuring destinations.
- Run as a resource-efficient background process: target <20 MB RAM idle, ~0% CPU idle.
- Produce native installers for macOS (.dmg), Windows (.msi / .exe), and Linux (.AppImage / .deb) via GitHub Actions CI/CD.

### 2.2 Non-Goals (v1.0)

- **No delete propagation** — files deleted locally are NOT deleted from remote storage.
- **No mobile support** (Android / iOS) — deferred to a future version.
- **No application-layer encryption** — users rely on bucket-level or transport-layer encryption (HTTPS / TLS).
- **No file versioning or restore UI** — this is a pure backup tool.
- **No peer-to-peer or LAN-sync features.**
- **No cloud-to-cloud replication.**

---

## 3. Users & Use Cases

### 3.1 Target Users

| User Type | Description | Primary Need |
|---|---|---|
| Developer / Engineer | Works with code repos, config files, build artifacts | Automatic offsite backup without manual steps |
| Creative Professional | Large media files — video, audio, design assets | Fast multipart upload, multiple backup destinations |
| Power User / IT Admin | Manages multiple machines, uses NAS | Centralized NAS backup with optional cloud redundancy |
| Researcher | Data files, notebooks, documents | Set-and-forget incremental backup with reliable consistency |

### 3.2 Primary Use Cases

1. User installs Shadow and opens it for the first time. They add a folder to the watch list and configure an S3 bucket. The app immediately starts a full backup of all existing files in the folder.
2. While working, the user saves a file inside a watched folder. Within 500ms the file is queued and uploading begins automatically in the background.
3. User wants cloud redundancy plus local NAS backup. They enable both S3 and NAS destinations. Every file change is uploaded to both simultaneously.
4. User opens the UI to check backup status, sees a live feed of recently backed-up files, and confirms the last backup time.
5. User adds a new folder to watch. The app immediately recursively scans and backs up its contents.
6. User removes a folder from the watch list. The app stops watching it but does not delete anything from remote storage.

---

## 4. System Architecture

### 4.1 High-Level Architecture

Shadow is structured as a Tauri 2 desktop application. The Rust process is the authoritative daemon — it owns all file-watching, hashing, queuing, and uploading logic. The React/TypeScript frontend is a thin UI shell that communicates with the Rust backend exclusively via Tauri's typed IPC command and event bridge.

```
┌─────────────────────────────────────────────────────────────────┐
│                     Tauri Desktop Application                   │
│  ┌──────────────────────┐    ┌────────────────────────────────┐ │
│  │   React/TS Frontend  │───▶│     Tauri IPC Bridge           │ │
│  │  - Folder manager    │◀───│  Commands: add_folder,         │ │
│  │  - Status feed       │    │  remove_folder, set_provider,  │ │
│  │  - Provider config   │    │  get_status, get_logs          │ │
│  │  - Log viewer        │    │  Events: file_queued,          │ │
│  └──────────────────────┘    │  file_uploaded, file_error     │ │
│                              └──────────────┬─────────────────┘ │
│                                             │                    │
│                              ┌──────────────▼─────────────────┐ │
│                              │       Rust Daemon Core          │ │
│                              │  ┌─────────────────────────┐   │ │
│                              │  │  notify-rs (FS Watcher) │   │ │
│                              │  │  FSEvents / inotify /   │   │ │
│                              │  │  ReadDirChangesW        │   │ │
│                              │  └────────────┬────────────┘   │ │
│                              │  ┌────────────▼────────────┐   │ │
│                              │  │  Debounce + Hash Check  │   │ │
│                              │  │  (blake3, 200ms window) │   │ │
│                              │  └────────────┬────────────┘   │ │
│                              │  ┌────────────▼────────────┐   │ │
│                              │  │  Upload Queue           │   │ │
│                              │  │  (tokio::mpsc channel)  │   │ │
│                              │  └────────────┬────────────┘   │ │
│                              │  ┌────────────▼────────────┐   │ │
│                              │  │  Provider Layer         │   │ │
│                              │  │  S3 | GCS | NAS         │   │ │
│                              │  └─────────────────────────┘   │ │
│                              └────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

### 4.2 Component Breakdown

| Component | Language / Crate | Responsibility |
|---|---|---|
| FS Watcher | Rust — `notify` v6 | Listen for create/modify events using OS kernel APIs. No polling. |
| Debouncer | Rust — `tokio` + `HashMap` | Coalesce rapid write events per file path with a 200ms settling window. |
| Hash Store | Rust — `blake3` + `sled` | Compute blake3 hash of each file; persist in embedded sled DB. Skip upload if hash unchanged. |
| Upload Queue | Rust — `tokio::sync::mpsc` | Bounded async channel feeding up to N concurrent upload tasks (default N=4). |
| Provider: S3 | Rust — `aws-sdk-s3` | Multipart upload for files >10MB. Standard PutObject for smaller files. |
| Provider: GCS | Rust — `google-cloud-storage` | Resumable upload for large files. Standard upload for small files. |
| Provider: NAS | Rust — `std::fs` | Direct file copy to mounted NAS path. No external SDK needed. |
| IPC Bridge | Tauri 2 commands/events | Type-safe async commands from UI to Rust. Real-time events pushed from Rust to UI. |
| UI Shell | React 18 + TypeScript | Folder management, provider configuration, real-time status feed, log viewer. |
| Config Store | Rust — `serde` + `toml` | Persist watched folders and provider settings to platform config directory. |
| Initial Scan | Rust — `walkdir` | Recursive directory walk on first-run or newly added folder; feeds into upload queue. |

---

## 5. Functional Requirements

### FR-01: Folder Watch Management

- The user can add one or more local folder paths to a persistent watch list via the UI.
- The user can remove a folder from the watch list. Watching stops immediately; nothing is deleted remotely.
- Watching is recursive — all files and subdirectories under a watched folder are included.
- The watch list persists across application restarts.
- Symlinks: by default, symlinks are followed one level deep. A config toggle allows disabling this.

### FR-02: Initial Full Backup

- When a folder is added to the watch list for the first time, the app immediately begins a full recursive scan.
- Every file found is hashed and compared against the hash store. Files with no stored hash are queued for upload.
- The UI displays progress during the initial scan (files found, files queued, files uploaded).
- If the app is closed mid-scan and restarted, the scan resumes from where it left off using the hash store state.

### FR-03: Incremental Backup (Change Detection)

- OS-native filesystem events (create, modify) trigger the backup pipeline — no polling.
- A per-file debounce window of 200ms is applied. The upload is triggered only after no further events are received for that file within the window.
- Before queuing an upload, the file is hashed with blake3. If the hash matches the stored value, the file is skipped.
- If the hash differs (or no hash is stored), the file is queued for upload and the hash store is updated on successful upload.

### FR-04: Remote Path Convention

All uploaded files are stored at the following remote path:

```
<configured_bucket_or_root>/<machine_hostname>/<absolute_local_path_of_file>
```

**Example:** If the bucket is `my-backups`, the machine hostname is `JOHNS-MACBOOK`, and the file is at `/Users/john/Documents/report.pdf`, the remote key is:

```
my-backups/JOHNS-MACBOOK/Users/john/Documents/report.pdf
```

On Windows, backslashes in paths are converted to forward slashes for the remote key.

### FR-05: Backup Providers

The user can enable any combination of the three providers. All enabled providers receive every upload independently and in parallel.

| Provider | Auth Method | Large File Strategy | Config Fields |
|---|---|---|---|
| AWS S3 | Env vars / `~/.aws/credentials` / IAM role | Multipart upload (>10MB, 8MB parts) | Bucket name, Region, optional endpoint override |
| Google Cloud Storage | Application Default Credentials / Service Account JSON | Resumable upload (>10MB) | Bucket name, Project ID, optional credentials path |
| NAS (mount point) | OS-level mount (SMB/NFS/AFP mounted before app starts) | Streaming file copy | Mount path (local directory that maps to NAS share) |

> **IMPORTANT:** Credentials for S3 and GCS are NEVER stored in the Shadow config file. The app reads credentials from the standard provider chains (AWS credential chain, GCP Application Default Credentials). The UI only stores non-secret configuration (bucket names, region, paths).

### FR-06: Upload Queue & Concurrency

- The upload queue is a bounded async channel (capacity: 512 items).
- Default concurrency: 4 parallel upload workers. User-configurable from 1 to 16.
- Each worker independently uploads to all enabled providers for its assigned file.
- On upload failure, retry with exponential backoff: 3 retries, delays of 1s / 4s / 16s.
- After 3 failures, the file is logged as a permanent error. The user can manually trigger a retry from the UI.

### FR-07: UI — Folder Management Screen

- List all watched folders with name, path, status (scanning / active / error), and last backup timestamp.
- Add folder button opens native OS folder picker.
- Remove folder button shows confirmation dialog before removing.
- Each folder row shows a mini progress indicator during initial scan.

### FR-08: UI — Provider Configuration Screen

- Per-provider toggle (enabled / disabled).
- Input fields for non-secret config (bucket, region, mount path).
- Test Connection button per provider — performs a lightweight check (e.g. `HeadBucket` for S3) and shows a success / failure badge.
- Visual status badge: Connected / Error / Not Configured.

### FR-09: UI — Status & Activity Feed

- Real-time scrolling feed showing the last 200 backup events (file queued, file uploaded, upload failed).
- Each event shows: timestamp, filename, provider(s), file size, duration.
- Summary bar: total files backed up today, total data transferred, active uploads count.
- Filter feed by provider or by status (all / success / error).

### FR-10: Background Operation

- On launch, the daemon starts immediately and begins watching all configured folders.
- The app minimizes to the system tray (macOS menu bar, Windows tray, Linux tray via tray icon).
- The main window can be closed without stopping the daemon; the daemon continues running.
- A tray icon tooltip shows current status (idle / uploading N files / error).
- The user can quit the daemon entirely from the tray menu.
- On OS startup, Shadow launches automatically (user-configurable toggle in settings).

---

## 6. Non-Functional Requirements

| ID | Requirement | Target |
|---|---|---|
| NFR-01 | Upload latency from file write completion to upload start | < 500ms under normal conditions |
| NFR-02 | Idle CPU usage (daemon running, no uploads) | ~0% (event-driven, no polling) |
| NFR-03 | Idle RAM usage (daemon running, no uploads) | < 20 MB RSS |
| NFR-04 | Hash computation overhead for a 100MB file | < 150ms (blake3) |
| NFR-05 | Time to start watching a new folder (empty) | < 200ms |
| NFR-06 | Installer binary size (each platform) | < 15 MB |
| NFR-07 | Upload worker concurrency (default / max) | 4 workers / 16 workers |
| NFR-08 | Upload queue capacity before back-pressure | 512 items |
| NFR-09 | Retry attempts on upload failure | 3 retries with exponential backoff (1s / 4s / 16s) |
| NFR-10 | Platforms supported | macOS 12+, Windows 10+, Ubuntu 20.04+ |

---

## 7. Technology Stack

### 7.1 Full Dependency Reference

| Layer | Technology | Version | Purpose |
|---|---|---|---|
| App Framework | Tauri | 2.x | Desktop shell, IPC bridge, tray, auto-updater |
| Backend Language | Rust | 1.78+ (stable) | Daemon core, all business logic |
| Async Runtime | tokio | 1.x | Async I/O, task scheduling, channels |
| FS Watching | notify | 6.x | OS-native kernel events (FSEvents / inotify / RDCW) |
| File Hashing | blake3 | 1.x | Content-addressed change detection |
| Local State DB | sled | 0.34 | Embedded key-value store for hash registry |
| Config Serialization | serde + toml | latest | Read/write config.toml |
| Directory Walk | walkdir | 2.x | Initial recursive scan of watched folders |
| AWS S3 Upload | aws-sdk-s3 | latest | S3 PutObject and multipart upload |
| GCS Upload | google-cloud-storage | 0.20+ | GCS standard and resumable upload |
| NAS Upload | std::fs (stdlib) | — | Local file copy to mounted NAS path |
| HTTP Client | reqwest | 0.12 | Underlying HTTP for cloud SDKs |
| Retry Logic | tokio-retry | 0.3 | Exponential backoff for failed uploads |
| Logging | tracing + tracing-subscriber | 0.1 | Structured async-aware logging |
| Hostname | hostname | 0.3 | Read machine hostname for remote path |
| Frontend Language | TypeScript | 5.x | Type-safe UI logic |
| Frontend Framework | React | 18.x | UI component tree |
| Styling | Tailwind CSS | 3.x | Utility-first CSS, no runtime overhead |
| UI State | Zustand | 4.x | Lightweight client-side state management |
| Build Tool | Vite | 5.x | Fast frontend bundler |
| CI/CD | GitHub Actions | — | Multi-platform build matrix |

### 7.2 Cargo.toml Dependencies (src-tauri/Cargo.toml)

```toml
[dependencies]
tauri           = { version = "2", features = ["tray-icon", "shell-open"] }
tokio           = { version = "1", features = ["full"] }
notify          = "6"
blake3          = "1"
sled            = "0.34"
serde           = { version = "1", features = ["derive"] }
serde_json      = "1"
toml            = "0.8"
walkdir         = "2"
aws-sdk-s3      = { version = "1", features = ["behavior-version-latest"] }
aws-config      = "1"
google-cloud-storage = "0.20"
reqwest         = { version = "0.12", features = ["stream"] }
tokio-retry     = "0.3"
tracing         = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
hostname        = "0.3"
anyhow          = "1"
thiserror       = "1"
```

---

## 8. Data Model & Configuration

### 8.1 Configuration File Location

| Platform | Path |
|---|---|
| macOS | `~/Library/Application Support/shadow/config.toml` |
| Windows | `%APPDATA%\shadow\config.toml` |
| Linux | `~/.config/shadow/config.toml` |

### 8.2 config.toml Schema

```toml
[daemon]
upload_concurrency = 4          # parallel upload workers (1–16)
debounce_ms        = 200        # file settle window in milliseconds (50–5000)
follow_symlinks    = true
start_on_login     = true
log_level          = "info"     # error | warn | info | debug

[machine]
hostname = ""                   # auto-detected on first run, user-overridable

[watched_folders]
paths = [
  "/Users/john/Documents",
  "/Users/john/Projects"
]

[providers.s3]
enabled  = false
bucket   = ""
region   = "us-east-1"
endpoint = ""                   # optional: custom endpoint (e.g. MinIO)

[providers.gcs]
enabled          = false
bucket           = ""
project_id       = ""
credentials_path = ""           # optional: path to service account JSON file

[providers.nas]
enabled    = false
mount_path = ""                 # e.g. /Volumes/MyNAS or Z:\Backup
```

> **SECURITY NOTE:** No secrets (access keys, passwords, tokens) are ever written to config.toml. Cloud credentials are read from environment variables and standard credential chain locations at runtime only.

### 8.3 Hash Registry — sled Embedded Database

**Location:** `~/.shadow/hashdb/` (user home, survives app reinstall)

**Schema:**
```
Key:   <absolute_local_file_path>   (UTF-8 string)
Value: <blake3_hex_hash>:<last_modified_unix_timestamp_ms>
```

**Behavior:**
- Loaded into memory at daemon startup.
- Written on every successful upload.
- Used to skip unchanged files instantly without re-reading file content.
- Clearing the hash store (via Settings UI) forces a full re-upload on next scan.

### 8.4 Rust Data Structures

```rust
// Config model
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub daemon: DaemonConfig,
    pub machine: MachineConfig,
    pub watched_folders: WatchedFolders,
    pub providers: ProvidersConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DaemonConfig {
    pub upload_concurrency: usize,   // default: 4
    pub debounce_ms: u64,            // default: 200
    pub follow_symlinks: bool,       // default: true
    pub start_on_login: bool,        // default: true
    pub log_level: String,           // default: "info"
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct S3Config {
    pub enabled: bool,
    pub bucket: String,
    pub region: String,
    pub endpoint: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GcsConfig {
    pub enabled: bool,
    pub bucket: String,
    pub project_id: String,
    pub credentials_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NasConfig {
    pub enabled: bool,
    pub mount_path: String,
}

// IPC event payloads
#[derive(Debug, Serialize, Clone)]
pub struct FileUploadedEvent {
    pub path: String,
    pub provider: String,
    pub duration_ms: u64,
    pub size_bytes: u64,
    pub remote_key: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct FileErrorEvent {
    pub path: String,
    pub provider: String,
    pub error: String,
    pub attempt: u32,
}

#[derive(Debug, Serialize, Clone)]
pub struct ScanProgressEvent {
    pub folder: String,
    pub scanned: u64,
    pub queued: u64,
    pub total: u64,
}
```

### 8.5 Provider Trait

```rust
#[async_trait::async_trait]
pub trait BackupProvider: Send + Sync {
    fn name(&self) -> &'static str;
    async fn upload(&self, local_path: &Path, remote_key: &str) -> anyhow::Result<()>;
    async fn test_connection(&self) -> anyhow::Result<String>;
}
```

Implementations: `S3Provider`, `GcsProvider`, `NasProvider`.

---

## 9. Upload Pipeline — Detailed Flow

```
FS Event (create / modify)
      │
      ▼
Debouncer (200ms window per path)
      │  file settled — no further events for 200ms
      ▼
Hash Check
      │  blake3(file_content) == stored_hash?
      ├─── YES ──▶ Skip (no upload needed, no action)
      │
      ▼  NO (hash differs or no stored hash)
Enqueue to Upload Channel (tokio::sync::mpsc, capacity 512)
      │
      ▼
Upload Worker (one of N concurrent workers, default N=4)
      │
      ├──▶ Provider S3   ──▶ success ─┐
      ├──▶ Provider GCS  ──▶ success ─┤──▶ Update hash store
      └──▶ Provider NAS  ──▶ success ─┘    Emit file_uploaded event to UI
                │
                ▼  any provider failure
         Retry with exponential backoff
         Attempt 1 → wait 1s  → retry
         Attempt 2 → wait 4s  → retry
         Attempt 3 → wait 16s → retry
                │
                ▼  all retries exhausted
         Log permanent error
         Emit file_failed event to UI
         Store in failed_uploads list (user can retry manually)
```

### 9.1 Large File Handling

- Files > 10MB: use multipart upload (S3) or resumable upload (GCS).
- Chunk size: 8MB per part.
- Parts are uploaded in parallel (up to 4 parts concurrently per file).
- NAS: streamed using `std::io::copy` with a 1MB buffer.

### 9.2 File Locking / Access Errors

- If a file cannot be read (locked by another process), retry once after 5 seconds.
- If still inaccessible, log as skipped with reason `file_locked`. Do not count as an error.
- File will be retried on the next modification event.

---

## 10. Remote Path Construction

### 10.1 Path Convention

```
<bucket_or_nas_root>/<machine_hostname>/<normalized_absolute_path>
```

### 10.2 Rust Implementation

```rust
pub fn remote_key(root: &str, hostname: &str, local_path: &Path) -> String {
    // Convert path to string and normalize separators
    let path_str = local_path
        .to_string_lossy()
        .replace('\\', "/");

    // Strip leading slash (Unix) or drive letter + colon (Windows: "C:/..." -> "C/...")
    let normalized = if let Some(stripped) = path_str.strip_prefix('/') {
        stripped.to_string()
    } else if path_str.len() >= 2 && path_str.chars().nth(1) == Some(':') {
        // Windows: "C:/Users/..." -> "C/Users/..."
        path_str.replacen(':', "", 1)
    } else {
        path_str
    };

    format!("{}/{}/{}", root, hostname, normalized)
}
```

### 10.3 Examples

| Platform | Local Path | Remote Key |
|---|---|---|
| macOS | `/Users/john/Documents/report.pdf` | `my-backups/JOHNS-MAC/Users/john/Documents/report.pdf` |
| Linux | `/home/john/projects/app/main.rs` | `my-backups/DEV-BOX/home/john/projects/app/main.rs` |
| Windows | `C:\Users\john\Documents\report.pdf` | `my-backups/JOHNS-PC/C/Users/john/Documents/report.pdf` |

---

## 11. Tauri IPC API Contract

### 11.1 Commands (UI → Rust)

All commands are invoked via `invoke()` from the React frontend.

```typescript
// TypeScript signatures (src/ipc.ts)
invoke('add_folder', { path: string }): Promise<void>
invoke('remove_folder', { path: string }): Promise<void>
invoke('get_watched_folders'): Promise<FolderStatus[]>
invoke('set_provider_config', { config: ProviderConfig }): Promise<void>
invoke('get_provider_config'): Promise<AllProviderConfig>
invoke('test_provider', { provider: 'S3' | 'GCS' | 'NAS' }): Promise<string>
invoke('get_activity_log', { limit: number }): Promise<LogEntry[]>
invoke('retry_failed', { filePath: string }): Promise<void>
invoke('get_stats'): Promise<DaemonStats>
invoke('set_daemon_config', { config: DaemonConfig }): Promise<void>
invoke('clear_hash_store'): Promise<void>
```

| Command | Parameters | Return | Description |
|---|---|---|---|
| `add_folder` | `path: String` | `Result<(), String>` | Add folder to watch list and trigger initial scan |
| `remove_folder` | `path: String` | `Result<(), String>` | Remove folder, stop watching immediately |
| `get_watched_folders` | — | `Vec<FolderStatus>` | List all watched folders with status |
| `set_provider_config` | `config: ProviderConfig` | `Result<(), String>` | Save provider settings to config file |
| `get_provider_config` | — | `AllProviderConfig` | Read current provider config (no secrets returned) |
| `test_provider` | `provider: String` | `Result<String, String>` | Test connectivity to a provider |
| `get_activity_log` | `limit: u32` | `Vec<LogEntry>` | Fetch recent activity entries |
| `retry_failed` | `file_path: String` | `Result<(), String>` | Manually retry a permanently failed upload |
| `get_stats` | — | `DaemonStats` | Files uploaded today, bytes transferred, queue depth |
| `set_daemon_config` | `config: DaemonConfig` | `Result<(), String>` | Update concurrency, debounce, login settings |
| `clear_hash_store` | — | `Result<(), String>` | Wipe hash DB — forces full re-upload on next scan |

### 11.2 Events (Rust → UI, real-time push)

All events are received via `listen()` in the React frontend.

```typescript
// TypeScript listener setup (src/ipc.ts)
listen<FileQueuedPayload>('file_queued', handler)
listen<FileUploadingPayload>('file_uploading', handler)
listen<FileUploadedPayload>('file_uploaded', handler)
listen<FileSkippedPayload>('file_skipped', handler)
listen<FileErrorPayload>('file_error', handler)
listen<FileFailedPayload>('file_failed', handler)
listen<ScanProgressPayload>('scan_progress', handler)
listen<ScanCompletePayload>('scan_complete', handler)
listen<ProviderStatusPayload>('provider_status', handler)
```

| Event Name | Payload | Description |
|---|---|---|
| `file_queued` | `{ path, size_bytes }` | File has passed hash check and entered the upload queue |
| `file_uploading` | `{ path, provider, progress_pct }` | Upload in progress (emitted periodically for large files) |
| `file_uploaded` | `{ path, provider, duration_ms, size_bytes, remote_key }` | File successfully uploaded to a provider |
| `file_skipped` | `{ path, reason }` | File skipped (hash match or file locked) |
| `file_error` | `{ path, provider, error, attempt }` | Upload attempt failed (may retry) |
| `file_failed` | `{ path, provider, error }` | All retries exhausted — permanent failure |
| `scan_progress` | `{ folder, scanned, queued, total }` | Initial scan progress update |
| `scan_complete` | `{ folder, total_files, total_bytes }` | Initial scan finished |
| `provider_status` | `{ provider, connected: bool, error? }` | Provider connectivity status changed |

### 11.3 TypeScript Type Definitions

```typescript
// src/types.ts

export interface FolderStatus {
  path: string;
  status: 'scanning' | 'active' | 'error' | 'paused';
  file_count: number;
  last_backup_at: string | null;   // ISO 8601
  scan_progress?: ScanProgress;
}

export interface ScanProgress {
  scanned: number;
  queued: number;
  total: number;
}

export interface LogEntry {
  id: string;
  timestamp: string;               // ISO 8601
  event_type: 'queued' | 'uploaded' | 'skipped' | 'error' | 'failed';
  path: string;
  filename: string;
  provider: string | null;
  size_bytes: number | null;
  duration_ms: number | null;
  error: string | null;
}

export interface DaemonStats {
  files_uploaded_today: number;
  bytes_transferred_today: number;
  active_uploads: number;
  queue_depth: number;
  failed_count: number;
}

export interface DaemonConfig {
  upload_concurrency: number;
  debounce_ms: number;
  follow_symlinks: boolean;
  start_on_login: boolean;
  log_level: 'error' | 'warn' | 'info' | 'debug';
}

export interface S3Config {
  enabled: boolean;
  bucket: string;
  region: string;
  endpoint: string;
}

export interface GcsConfig {
  enabled: boolean;
  bucket: string;
  project_id: string;
  credentials_path: string;
}

export interface NasConfig {
  enabled: boolean;
  mount_path: string;
}

export interface AllProviderConfig {
  s3: S3Config;
  gcs: GcsConfig;
  nas: NasConfig;
}
```

---

## 12. UI Screen Specifications

### 12.1 Navigation Structure

Single-window app. Left sidebar with four navigation items:

```
┌─────────────┬──────────────────────────────────────┐
│  Shadow │                                       │
│  ─────────  │   Main Content Area                  │
│  Dashboard  │                                       │
│  Folders    │                                       │
│  Providers  │                                       │
│  Settings   │                                       │
│             │                                       │
│  ● Active   │                                       │
└─────────────┴──────────────────────────────────────┘
```

Sidebar footer shows daemon status dot: green (active), yellow (uploading), red (error).

### 12.2 Dashboard Screen

**Summary Bar (top):**
- Files backed up today
- Data transferred today (human-readable: KB / MB / GB)
- Active uploads (count)
- Queue depth (count)

**Activity Feed:**
- Scrollable list, newest event at top, max 200 entries rendered.
- Each row: `[status icon] [filename] [provider badge(s)] [size] [duration] [timestamp]`
- Status icon colors: green = uploaded, yellow = queued/uploading, red = error/failed.
- Auto-scrolls to new events unless the user has manually scrolled up.
- Filter bar: All | Success | Error | [Provider dropdown]
- Empty state: "No activity yet. Add a folder to get started."

### 12.3 Folders Screen

**Folder Table columns:** Path | Status | File Count | Last Backup | Actions

**Status badges:**
- `Scanning` — animated spinner, blue
- `Active` — green dot
- `Error` — red dot with error tooltip
- `Paused` — gray dot

**Actions per row:**
- Remove button — shows confirmation modal: "Stop watching [path]? Remote files will not be deleted."

**During initial scan:** Progress bar per folder: `Scanning: 1,234 / 5,678 files uploaded`

**Add Folder button:** Opens native OS folder picker dialog (`tauri::dialog::open`).

### 12.4 Providers Screen

Three provider cards displayed vertically. Each card contains:

**AWS S3 Card:**
- Enable/disable toggle
- Bucket Name (text input, required if enabled)
- Region (text input, default: `us-east-1`)
- Custom Endpoint (text input, optional — for MinIO etc.)
- Test Connection button → async → "Connected ✓" or "Failed: [error message]"

**Google Cloud Storage Card:**
- Enable/disable toggle
- Bucket Name (text input, required if enabled)
- Project ID (text input, required if enabled)
- Credentials File Path (text input, optional — leave blank to use Application Default Credentials)
- Test Connection button

**NAS Card:**
- Enable/disable toggle
- Mount Path (text input, required if enabled — e.g. `/Volumes/MyNAS` or `Z:\`)
- Test Connection button → checks path is accessible and writable

Inline validation: if a provider is enabled and required fields are empty, highlight with red border and tooltip.

### 12.5 Settings Screen

| Setting | Control | Default | Notes |
|---|---|---|---|
| Upload Workers | Number input (1–16) | 4 | Concurrent upload tasks |
| Debounce Window | Number input (ms) | 200 | File settle time (50–5000ms) |
| Follow Symlinks | Toggle | On | Follow symlinks one level deep |
| Launch at Login | Toggle | On | Register as login item |
| Log Level | Dropdown | Info | Error / Warn / Info / Debug |
| Clear Hash Store | Danger button | — | Confirmation required. Forces full re-upload. |
| App Version | Read-only text | — | Show current version + check for updates link |

---

## 13. CI/CD — GitHub Actions Build Matrix

### 13.1 Workflow Overview

File: `.github/workflows/release.yml`  
Trigger: push to version tag matching `v*.*.*`  
Jobs run in parallel on three separate hosted runners.

| Job | Runner | Output Artifacts |
|---|---|---|
| `build-macos` | `macos-latest` (Apple Silicon + Intel) | `.dmg` (universal binary), `.app` |
| `build-windows` | `windows-latest` | `.msi` (WiX), `.exe` (NSIS) |
| `build-linux` | `ubuntu-22.04` | `.AppImage` (portable), `.deb` (Debian/Ubuntu) |

### 13.2 Full Workflow File

```yaml
name: Release

on:
  push:
    tags:
      - 'v*.*.*'

jobs:
  build-macos:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-apple-darwin,x86_64-apple-darwin

      - uses: actions/setup-node@v4
        with:
          node-version: '20'

      - name: Install frontend dependencies
        run: npm ci

      - name: Build Tauri app (universal binary)
        run: cargo tauri build --target universal-apple-darwin
        env:
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}

      - uses: actions/upload-artifact@v4
        with:
          name: shadow-macos
          path: src-tauri/target/universal-apple-darwin/release/bundle/

  build-windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable

      - uses: actions/setup-node@v4
        with:
          node-version: '20'

      - name: Install frontend dependencies
        run: npm ci

      - name: Build Tauri app
        run: cargo tauri build
        env:
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}

      - uses: actions/upload-artifact@v4
        with:
          name: shadow-windows
          path: src-tauri/target/release/bundle/

  build-linux:
    runs-on: ubuntu-22.04
    steps:
      - uses: actions/checkout@v4

      - name: Install Linux system dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y \
            libwebkit2gtk-4.1-dev \
            libgtk-3-dev \
            libayatana-appindicator3-dev \
            librsvg2-dev \
            patchelf

      - uses: dtolnay/rust-toolchain@stable

      - uses: actions/setup-node@v4
        with:
          node-version: '20'

      - name: Install frontend dependencies
        run: npm ci

      - name: Build Tauri app
        run: cargo tauri build
        env:
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}

      - uses: actions/upload-artifact@v4
        with:
          name: shadow-linux
          path: src-tauri/target/release/bundle/

  publish-release:
    needs: [build-macos, build-windows, build-linux]
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/download-artifact@v4

      - name: Create GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          files: |
            shadow-macos/**/*
            shadow-windows/**/*
            shadow-linux/**/*
          generate_release_notes: true
```

### 13.3 Required GitHub Secrets

| Secret Name | Description |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | Private key for Tauri auto-updater signing (generated with `cargo tauri signer generate`) |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password for the signing key |

> For macOS code signing and notarization, additional secrets are needed: `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`. These are only required to distribute outside of direct download (e.g. for Gatekeeper to pass without warning on unsigned builds users can right-click → Open).

---

## 14. Recommended Project Structure

```
shadow/
├── .github/
│   └── workflows/
│       └── release.yml
│
├── src/                              # React/TypeScript frontend
│   ├── components/
│   │   ├── layout/
│   │   │   ├── Sidebar.tsx
│   │   │   └── Layout.tsx
│   │   ├── screens/
│   │   │   ├── Dashboard.tsx
│   │   │   ├── FoldersScreen.tsx
│   │   │   ├── ProvidersScreen.tsx
│   │   │   └── SettingsScreen.tsx
│   │   └── shared/
│   │       ├── StatusBadge.tsx
│   │       ├── ActivityFeed.tsx
│   │       └── ConfirmModal.tsx
│   ├── store/
│   │   ├── foldersStore.ts           # Zustand — watched folders state
│   │   ├── activityStore.ts          # Zustand — activity feed state
│   │   ├── providerStore.ts          # Zustand — provider config state
│   │   └── statsStore.ts             # Zustand — daemon stats state
│   ├── hooks/
│   │   ├── useActivityFeed.ts        # Subscribe to file_* events
│   │   └── useProviderStatus.ts      # Subscribe to provider_status events
│   ├── ipc.ts                        # Typed Tauri invoke() wrappers
│   ├── types.ts                      # All shared TypeScript types
│   ├── App.tsx
│   └── main.tsx
│
├── src-tauri/
│   ├── src/
│   │   ├── main.rs                   # Tauri app builder, window setup, tray
│   │   ├── lib.rs                    # Tauri command registration
│   │   ├── config.rs                 # AppConfig struct, load/save logic
│   │   ├── path_utils.rs             # remote_key() construction
│   │   ├── ipc.rs                    # All #[tauri::command] handlers
│   │   │
│   │   ├── daemon/
│   │   │   ├── mod.rs                # DaemonState, startup/shutdown
│   │   │   ├── watcher.rs            # notify-rs watcher, event dispatch
│   │   │   ├── debouncer.rs          # Per-path 200ms debounce logic
│   │   │   ├── hasher.rs             # blake3 hashing + sled hash store
│   │   │   ├── queue.rs              # tokio mpsc upload queue + workers
│   │   │   └── scanner.rs            # Initial recursive scan (walkdir)
│   │   │
│   │   └── providers/
│   │       ├── mod.rs                # BackupProvider trait definition
│   │       ├── s3.rs                 # S3Provider implementation
│   │       ├── gcs.rs                # GcsProvider implementation
│   │       └── nas.rs                # NasProvider implementation
│   │
│   ├── icons/                        # App icons (all sizes, all platforms)
│   ├── Cargo.toml
│   ├── Cargo.lock
│   └── tauri.conf.json
│
├── package.json
├── vite.config.ts
├── tailwind.config.js
├── tsconfig.json
└── README.md
```

---

## 15. Development Milestones

| Milestone | Scope | Exit Criteria |
|---|---|---|
| **M1 — Scaffold** | Tauri 2 project init, Rust workspace, React + Tailwind shell, IPC hello-world, GitHub Actions skeleton | CI builds green binaries on all 3 platforms from a single git push |
| **M2 — Core Daemon** | FS watcher (notify-rs), debouncer, blake3 hasher, sled hash store, upload queue, NAS provider | Files written to a watched folder appear on a NAS mount within 500ms |
| **M3 — Cloud Providers** | S3 provider with multipart upload, GCS provider with resumable upload, credential chain integration, retry + backoff logic | Files correctly upload to S3 and GCS; files >10MB use multipart/resumable |
| **M4 — Full UI** | All 4 screens implemented, real-time activity feed wired to IPC events, tray icon + menu, login item toggle | User can fully manage folders and providers through the UI with no manual config editing |
| **M5 — Initial Scan** | walkdir recursive scan on folder add, resume-on-restart via hash store, scan progress events to UI | Folder with 10,000 files fully backs up with correct remote paths; restart mid-scan resumes correctly |
| **M6 — Polish & Release** | Error handling hardening, structured logging, Tauri auto-updater, code signing hooks in CI, README, installer polish | Signed installers published as GitHub Release artifacts; auto-update works end-to-end |

---

## 16. Open Questions & Decisions

| # | Question | Options | Recommendation |
|---|---|---|---|
| OQ-1 | App-level encryption before upload? | None (v1) / AES-256 client-side encryption | Defer to v2; document as roadmap item |
| OQ-2 | Should the hash store survive a full app uninstall? | Stored with app data (lost on uninstall) / Stored in user home dir | User home (`~/.shadow/hashdb/`) for durability across reinstalls |
| OQ-3 | How to handle files open / locked by another process? | Skip and retry later / Warn user | Retry once after 5s; log as skipped if still locked; retry on next modification event |
| OQ-4 | Maximum file size? | Unlimited / Hard cap (e.g. 50GB) | Unlimited; streaming multipart/resumable upload handles any size |
| OQ-5 | Auto-update mechanism? | Tauri built-in updater / Manual download | Tauri built-in updater with GitHub Releases as the update server |
| OQ-6 | File exclusion patterns? | None (v1) / Glob pattern list in config | Add `.gitignore`-style exclusion patterns in v1.1; mark as known gap in v1.0 |

---

## 17. Glossary

| Term | Definition |
|---|---|
| **Daemon** | The Rust background process that handles all file watching and uploading logic. Runs independently of the UI window. |
| **IPC** | Inter-Process Communication — the Tauri 2 typed bridge between the React UI and the Rust daemon (commands + events). |
| **Debounce** | Collapsing rapid successive file events for the same path into a single action after a settling delay (200ms). Prevents uploading a file 50 times while it is being written. |
| **blake3** | A cryptographic hash function used for content-addressed change detection. Extremely fast — orders of magnitude faster than SHA-256. |
| **sled** | An embedded key-value database (pure Rust) used to persist the file hash registry without requiring an external database server. |
| **notify-rs** | A Rust crate that wraps OS-native filesystem event APIs: FSEvents (macOS), inotify (Linux), ReadDirectoryChangesW (Windows). |
| **Multipart Upload** | Splitting a large file into chunks and uploading them in parallel, then assembling on the server. Used by S3 for files >10MB. |
| **Resumable Upload** | GCS equivalent of multipart upload. Allows interrupted uploads to resume from where they stopped. |
| **Remote Key** | The full path/key of a file in the remote storage bucket or NAS directory. Follows the convention `<root>/<hostname>/<local_path>`. |
| **Initial Scan** | The one-time recursive directory walk performed when a folder is first added to the watch list, backing up all existing files. |
| **Hash Store** | The local sled database mapping local file paths to their last-uploaded blake3 hash. The source of truth for what has been backed up. |
| **Provider** | One of the three supported backup destinations: AWS S3, Google Cloud Storage, or NAS. |
| **Universal Binary** | A macOS binary that contains native code for both Apple Silicon (arm64) and Intel (x86_64), created by combining both builds with `lipo`. |

---

*End of Document — Shadow PRD v1.0*
