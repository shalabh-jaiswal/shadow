# Shadow — Development Milestones

## Completed Milestones

### ✅ M2 — Core Daemon
File watching, debouncing, blake3 hashing, upload queue with retry/backoff, NAS provider, IPC commands and events wired.

---

### ✅ M1 — Scaffold
Tauri 2 project init with Rust workspace, React + Tailwind CSS shell, IPC hello-world
(one command, one event), GitHub Actions skeleton (builds on all 3 platforms), basic tray icon.

**Exit criteria met:**
- [x] `cargo tauri dev` starts with no errors on macOS, Windows, and Linux
- [x] A test IPC command can be invoked from the React UI and returns a response
- [x] GitHub Actions workflow file exists and CI builds green binaries on all 3 platforms from a tag push
- [x] Tray icon appears with a "Quit" menu item

---

### ✅ M2 — Core Daemon

### Goal
Implement the full file-watching and upload pipeline for NAS, end-to-end. This milestone
proves the core loop works: detect → debounce → hash → queue → upload → confirm.

### Scope

#### Rust — Daemon Infrastructure (`src-tauri/src/daemon/`)
- **`watcher.rs`** — Wrap `notify` v6 with a recursive watcher. Subscribe to `Create` and
  `Modify` events only. Dispatch raw events onto a `tokio::sync::mpsc` channel for the debouncer.
- **`debouncer.rs`** — Per-path 200ms settling window using a `HashMap<PathBuf, JoinHandle>`.
  Cancel and restart the timer on each new event for the same path. Forward to the hash stage
  only after silence for the full window. Read `debounce_ms` from `AppConfig` — never hardcode.
- **`hasher.rs`** — Compute `blake3` hash of file bytes. Open `sled` DB at
  `~/.shadow/hashdb/` (use `dirs` crate). Compare against stored hash. If identical, skip.
  If different (or absent), forward to upload queue and write new hash on successful upload only.
- **`queue.rs`** — Bounded `tokio::sync::mpsc` channel (capacity 512). Spawn N worker tasks
  (default 4, read from `AppConfig`). Enforce concurrency cap with `tokio::sync::Semaphore`.
  Each worker calls `BackupProvider::upload()` for every enabled provider, independently.
  On failure: retry with exponential backoff — 3 attempts, delays 1s / 4s / 16s via `tokio-retry`.
  After exhausting retries, log permanent error and emit `file_failed` event.
- **`mod.rs`** — `DaemonState` struct holding `Arc<AppConfig>`, `AppHandle`, channel senders,
  and task handles. `start()` and `shutdown()` functions. `shutdown()` must drain the queue
  gracefully before returning.

#### Rust — NAS Provider (`src-tauri/src/providers/`)
- **`mod.rs`** — Define `BackupProvider` trait:
  ```rust
  #[async_trait::async_trait]
  pub trait BackupProvider: Send + Sync {
      fn name(&self) -> &'static str;
      async fn upload(&self, local_path: &Path, remote_key: &str) -> anyhow::Result<()>;
      async fn test_connection(&self) -> anyhow::Result<String>;
  }
  ```
- **`nas.rs`** — `NasProvider` implementation. Use `tokio::fs::copy` (or `std::io::copy` with
  a 1MB buffer in a `spawn_blocking`). Create all parent directories before copying. Verify the
  mount path exists and is writable in `test_connection()`.

#### Rust — Config & Path Utils
- **`config.rs`** — `AppConfig` struct matching the `config.toml` schema (see PRD §8.2).
  `load()` reads from the platform config directory (use `dirs::config_dir()`); creates
  defaults if absent. `save()` serializes back to TOML. Expose via `Arc<RwLock<AppConfig>>`.
- **`path_utils.rs`** — `remote_key(root, hostname, local_path) -> String`.
  Strip leading `/` on Unix; strip drive letter colon on Windows (`C:` → `C`). Replace `\` with `/`.

#### IPC Events emitted during this milestone
All emitted via `app_handle.emit()`:

| Event | When |
|---|---|
| `file_queued` | File passed hash check, entered upload channel |
| `file_uploading` | Worker picks up the file (emit once per provider) |
| `file_uploaded` | Provider upload succeeded |
| `file_skipped` | Hash matched — no upload needed |
| `file_error` | Upload attempt failed (transient, will retry) |
| `file_failed` | All retries exhausted — permanent failure |

#### IPC Commands wired in this milestone
- `add_folder` — add path to config, start watching, trigger `scanner` (stubbed: just start watcher)
- `remove_folder` — remove from config, unregister watcher
- `get_watched_folders` — return `Vec<FolderStatus>`
- `test_provider` — for NAS only; S3/GCS stubbed returning "not configured"

### Exit Criteria
- [ ] A file created or modified inside a watched folder is copied to the configured NAS mount
      within 500ms of the write completing
- [ ] Files whose content has not changed are skipped (blake3 hash match confirmed by log)
- [ ] Upload failures retry with exponential backoff; permanent failures are logged as errors
- [ ] Daemon starts with `cargo tauri dev` and watches folders defined in `config.toml`
- [ ] `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` passes with zero warnings
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` passes (unit tests for `remote_key` and
      debouncer logic at minimum)

---

## Active: M6 — Polish & Release

---

## Completed: M3 — Cloud Providers

### Goal
Add AWS S3 and Google Cloud Storage as fully working backup destinations with large-file support.
All three providers (NAS + S3 + GCS) run simultaneously when enabled.

### Scope

#### S3 Provider (`src-tauri/src/providers/s3.rs`)
- Use `aws-sdk-s3` with `aws-config` credential chain (env vars → `~/.aws/credentials` → IAM role).
  Never read credentials from `config.toml`.
- Small files (≤10MB): `PutObject`.
- Large files (>10MB): multipart upload — `CreateMultipartUpload`, parallel 8MB `UploadPart`
  calls (up to 4 parts concurrent), then `CompleteMultipartUpload`. Abort on failure.
- `test_connection()`: call `HeadBucket` and return the bucket region or error message.
- Support optional custom endpoint override (for MinIO compatibility).

#### GCS Provider (`src-tauri/src/providers/gcs.rs`)
- Use `google-cloud-storage` crate with Application Default Credentials
  (env var `GOOGLE_APPLICATION_CREDENTIALS` → well-known file → metadata server).
- Small files (≤10MB): standard `upload_object`.
- Large files (>10MB): resumable upload session.
- `test_connection()`: list objects with `max_results=1` to verify credentials and bucket access.
- Optional `credentials_path` in config: if set, load service account JSON from that path.

#### IPC Commands added
- `set_provider_config` — persist non-secret provider settings (bucket, region, project_id,
  mount_path) to `config.toml`
- `get_provider_config` — return current provider config (no secrets)
- `test_provider` — fully implemented for S3 and GCS

#### Retry + Backoff (applies to all providers)
- Already wired in M2 queue worker; just ensure S3/GCS errors surface correctly as `anyhow::Error`.

### Exit Criteria
- [ ] A file ≤10MB uploads successfully to a configured S3 bucket with the correct remote key
- [ ] A file >10MB uploads to S3 using multipart (verify via S3 console or `aws s3api list-multipart-uploads`)
- [ ] A file uploads successfully to GCS (standard and resumable paths tested)
- [ ] All three providers can be enabled simultaneously; every file goes to all three
- [ ] `test_provider` returns a success message for valid credentials and a clear error for invalid ones
- [ ] No AWS/GCS credentials ever appear in `config.toml`

---

## M4 — Full UI

### Goal
Implement all four screens of the React frontend, wire real-time IPC events to the activity feed,
and complete tray icon behavior. User should be able to operate the app entirely through the UI.

### Scope

#### React Screens (`src/components/screens/`)

**Dashboard screen:**
- Summary bar: files backed up today, data transferred today, active uploads, queue depth
  (poll `get_stats` on mount and every 5s, or subscribe to a stats event)
- Activity feed: scrollable list capped at 200 entries, newest at top
  - Each row: status icon · filename · provider badge(s) · size · duration · timestamp
  - Filter bar: All | Success | Error | provider dropdown
  - Auto-scroll to new entries unless user has manually scrolled up
  - Empty state: "No activity yet. Add a folder to get started."

**Folders screen:**
- Table: Path | Status | File Count | Last Backup | Actions
- Status badges: `Scanning` (animated blue), `Active` (green), `Error` (red), `Paused` (gray)
- Per-folder progress bar during initial scan (`scan_progress` events)
- Add Folder button → native OS folder picker via `tauri::dialog::open`
- Remove button → `ConfirmModal`: "Stop watching [path]? Remote files will not be deleted."

**Providers screen:**
- One card per provider (S3, GCS, NAS)
- Enable/disable toggle; non-secret config fields; Test Connection button (async, shows badge)
- Inline validation: red border + tooltip when enabled with empty required fields
- Save writes via `set_provider_config`

**Settings screen:**
- **Machine Name** — text field for `config.machine.name`. Shown with a helper note:
  _"Used as the top-level prefix in all remote paths. Defaults to your OS hostname if left blank.
  Set this to avoid leaking your real machine name to cloud storage."_
  Save writes `machine.name` via a new `set_machine_name` IPC command (or via `set_daemon_config`).
- Upload Workers (1–16), Debounce Window (50–5000ms), Follow Symlinks toggle,
  Launch at Login toggle, Log Level dropdown
- Clear Hash Store button (danger, requires confirmation modal)
- App version (read from `tauri.conf.json` via `__APP_VERSION__` env)

#### Zustand Stores (`src/store/`)
- `foldersStore.ts` — watched folders list, scan progress per folder
- `activityStore.ts` — circular buffer of 200 log entries, filter state
- `providerStore.ts` — provider config and connection status
- `statsStore.ts` — daemon stats (files today, bytes today, queue depth)

#### Custom Hooks (`src/hooks/`)
- `useActivityFeed.ts` — subscribe to `file_queued`, `file_uploading`, `file_uploaded`,
  `file_skipped`, `file_error`, `file_failed` events; push to `activityStore`
- `useProviderStatus.ts` — subscribe to `provider_status` events; update `providerStore`

#### Tray
- Tooltip: "Shadow — Idle" / "Shadow — Uploading N files" / "Shadow — Error"
- Menu: Open Shadow | Separator | Quit
- Closing the main window hides it; daemon keeps running
- Clicking tray icon (or Open menu item) shows the window

### Exit Criteria
- [ ] All four screens render without TypeScript errors (`npm run type-check` passes)
- [ ] Activity feed updates in real time as files are backed up
- [ ] Folder add/remove works end-to-end through the UI
- [ ] Machine name can be set in Settings; remote paths immediately use the new name on next upload
- [ ] Provider config can be saved and persists across restarts
- [ ] Test Connection buttons work for all three providers
- [ ] Tray icon tooltip reflects daemon state; window close does not stop the daemon
- [ ] `npm run lint` passes with zero errors

---

## M5 — Initial Scan

### Goal
Implement the recursive initial scan that runs when a folder is first added. The scan must be
resumable — if the app is closed mid-scan and restarted, it picks up where it left off.

### Scope

#### Rust — Scanner (`src-tauri/src/daemon/scanner.rs`)
- Use `walkdir::WalkDir` to recursively enumerate all files under the added folder path.
  Respect `follow_symlinks` setting from `AppConfig` (one level deep when enabled).
- For each file found: check the `sled` hash store. If no entry exists, enqueue for upload.
  If an entry exists, compute current hash and enqueue only if hash has changed.
- Emit `scan_progress { folder, scanned, queued, total }` events periodically (every 100 files
  or 500ms, whichever comes first) for the UI progress bar.
- Emit `scan_complete { folder, total_files, total_bytes }` when finished.
- **Resume on restart:** The scan reads from `sled` to determine what is already uploaded.
  A file with a stored hash that matches its current content is considered done — skip it.
  A file with no stored hash is queued. This means a restart naturally resumes from where
  uploads left off, without any additional bookkeeping.
- Run the scan in a `tokio::spawn` task so it does not block the watcher or upload queue.

#### IPC
- `add_folder` — after registering the watcher, immediately spawn the scanner task for the new path.
- Scan progress events wired to `foldersStore` in the frontend (already set up in M4).

### Exit Criteria
- [ ] Adding a folder with 10,000 files triggers a full scan; all files are uploaded with
      correct remote keys (verified by spot-checking a sample against S3/GCS/NAS)
- [ ] Killing the app mid-scan and restarting resumes without re-uploading already-uploaded files
- [ ] `scan_progress` events drive the per-folder progress bar in the Folders screen
- [ ] `scan_complete` fires when the scan finishes and the status badge changes to `Active`
- [ ] Scan does not block the live watcher — file changes during a scan are still detected and queued

---

## M6 — Polish & Release

### Goal
Harden error handling, add structured logging, wire the Tauri auto-updater, complete code
signing in CI, and ship signed installer artifacts as a GitHub Release.

### Scope

#### Error Handling Hardening
- Audit every `ipc.rs` command for missing error paths; all must return `Result<T, String>`
  with a meaningful message
- File-locked handling: if `open()` fails with a permission/lock error, retry once after 5s;
  if still failing, emit `file_skipped { path, reason: "file_locked" }` — do not count as error
- Validate `config.toml` on load; if a field is out of range (e.g. `debounce_ms < 50`),
  clamp to the valid range and log a warning

#### Structured Logging
- Initialize `tracing_subscriber` with `EnvFilter` reading from `AppConfig.daemon.log_level`
- Log all upload attempts, retries, hash skips, and errors at appropriate levels
- Expose recent log lines through `get_activity_log` IPC command (last N entries from an
  in-memory ring buffer, not sled)
- No `console.log` anywhere in the frontend — all status surfaced via IPC events

#### Tauri Auto-Updater
- Configure `tauri-plugin-updater` in `tauri.conf.json` pointing to GitHub Releases as the
  update endpoint (use the Tauri v2 updater JSON endpoint format)
- Add "Check for Updates" to the tray menu and Settings screen
- Sign update bundles using `TAURI_SIGNING_PRIVATE_KEY` (already in CI secrets from M1)

#### Code Signing in CI (`.github/workflows/release.yml`)
- **macOS:** Add `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`,
  `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` secrets; enable notarization step
- **Windows:** Add code signing certificate if available; otherwise document that Defender
  SmartScreen will warn on first run and how to work around it
- **Linux:** AppImage and .deb are unsigned; document this as acceptable for v1.0

#### Release Checklist tasks
- Bump version in `src-tauri/tauri.conf.json` (single source of truth)
- Tag `v1.0.0` → CI builds and publishes to GitHub Releases automatically
- Verify all three platform installers download and install successfully
- Verify auto-update works: install v0.9.x (a test build), confirm v1.0.0 update is offered
  and applies cleanly

### Exit Criteria
- [ ] Signed `.dmg` (macOS), `.msi` (Windows), `.AppImage` + `.deb` (Linux) published as
      GitHub Release artifacts from a `v*.*.*` tag push
- [ ] Auto-updater successfully delivers an update from a prior version to the release version
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] `npm run type-check` passes
- [ ] `npm run lint` passes
- [ ] No `unwrap()` or `expect()` in production code paths (enforced by clippy custom lint or
      manual audit)
- [ ] App idles at <20MB RSS with no active uploads (measured on each platform)

---

## Milestone Reference

| # | Name | Key Deliverable | Status |
|---|---|---|---|
| M1 | Scaffold | CI green on all 3 platforms | ✅ Done |
| M2 | Core Daemon | Files on NAS within 500ms | ✅ Done |
| M3 | Cloud Providers | S3 + GCS multipart/resumable | ✅ Done |
| M4 | Full UI | All 4 screens wired to IPC | ✅ Done |
| M5 | Initial Scan | 10k-file folder fully backed up | ✅ Done |
| M6 | Polish & Release | Unsigned installers on GitHub Releases | ✅ Done |
