---
name: rust-patterns
description: |
  Shadow-specific Rust patterns and conventions. Activates automatically when
  writing or editing Rust code in src-tauri/. Covers: async patterns, error
  handling with anyhow/thiserror, tokio usage, BackupProvider implementations,
  sled DB operations, blake3 hashing, notify-rs watcher setup, IPC command
  patterns, cross-platform path handling.
allowed-tools:
  - Read
  - Bash
---

# Rust Patterns for Shadow

## Error Handling Pattern

```rust
// ✅ CORRECT — internal functions use anyhow
use anyhow::{Context, Result};

async fn upload_file(path: &Path) -> Result<()> {
    let content = tokio::fs::read(path)
        .await
        .with_context(|| format!("Failed to read file: {}", path.display()))?;
    // ...
    Ok(())
}

// ✅ CORRECT — IPC boundary converts to String
#[tauri::command]
pub async fn add_folder(path: String) -> Result<(), String> {
    inner_add_folder(path).await.map_err(|e| e.to_string())
}

// ❌ WRONG — never unwrap in production
let content = std::fs::read(path).unwrap();
```

## BackupProvider Implementation Pattern

```rust
use async_trait::async_trait;
use std::path::Path;
use anyhow::Result;

#[async_trait]
pub trait BackupProvider: Send + Sync {
    fn name(&self) -> &'static str;
    async fn upload(&self, local_path: &Path, remote_key: &str) -> Result<()>;
    async fn test_connection(&self) -> Result<String>;
}

// Upload to all enabled providers in parallel
pub async fn upload_to_all(
    providers: &[Arc<dyn BackupProvider>],
    local_path: &Path,
    remote_key: &str,
) -> Vec<Result<()>> {
    let futures: Vec<_> = providers
        .iter()
        .map(|p| p.upload(local_path, remote_key))
        .collect();
    futures::future::join_all(futures).await
}
```

## Tokio Upload Queue Pattern

```rust
use tokio::sync::{mpsc, Semaphore};
use std::sync::Arc;

pub struct UploadQueue {
    tx: mpsc::Sender<UploadTask>,
}

impl UploadQueue {
    pub fn new(concurrency: usize, providers: Arc<Vec<Arc<dyn BackupProvider>>>) -> Self {
        let (tx, mut rx) = mpsc::channel::<UploadTask>(512);
        let sem = Arc::new(Semaphore::new(concurrency));

        tokio::spawn(async move {
            while let Some(task) = rx.recv().await {
                let sem = sem.clone();
                let providers = providers.clone();
                tokio::spawn(async move {
                    let _permit = sem.acquire().await.unwrap();
                    // process task
                });
            }
        });

        Self { tx }
    }
}
```

## Debouncer Pattern

```rust
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::time::{Duration, Instant};

pub struct Debouncer {
    pending: HashMap<PathBuf, Instant>,
    window: Duration,
}

impl Debouncer {
    pub fn new(window_ms: u64) -> Self {
        Self {
            pending: HashMap::new(),
            window: Duration::from_millis(window_ms),
        }
    }

    /// Returns Some(path) if the path has settled (no new events for `window`)
    pub fn event(&mut self, path: PathBuf) -> Option<PathBuf> {
        let now = Instant::now();
        self.pending.insert(path.clone(), now);
        None // caller polls drain() periodically
    }

    pub fn drain_settled(&mut self) -> Vec<PathBuf> {
        let now = Instant::now();
        let window = self.window;
        let settled: Vec<PathBuf> = self.pending
            .iter()
            .filter(|(_, t)| now.duration_since(**t) >= window)
            .map(|(p, _)| p.clone())
            .collect();
        for p in &settled {
            self.pending.remove(p);
        }
        settled
    }
}
```

## blake3 Hash Check Pattern

```rust
use blake3::Hasher;
use std::path::Path;
use anyhow::Result;

pub async fn hash_file(path: &Path) -> Result<String> {
    let path = path.to_owned();
    tokio::task::spawn_blocking(move || {
        let mut hasher = Hasher::new();
        let mut file = std::fs::File::open(&path)?;
        std::io::copy(&mut file, &mut hasher)?;
        Ok(hasher.finalize().to_hex().to_string())
    })
    .await?
}

pub fn hash_changed(db: &sled::Db, path: &Path, new_hash: &str) -> bool {
    let key = path.to_string_lossy();
    match db.get(key.as_bytes()) {
        Ok(Some(stored)) => stored.as_ref() != new_hash.as_bytes(),
        _ => true, // not found or error → treat as changed
    }
}

pub fn store_hash(db: &sled::Db, path: &Path, hash: &str) -> Result<()> {
    let key = path.to_string_lossy();
    db.insert(key.as_bytes(), hash.as_bytes())?;
    Ok(())
}
```

## Remote Key Construction Pattern

```rust
pub fn remote_key(root: &str, hostname: &str, local_path: &Path) -> String {
    let path_str = local_path.to_string_lossy().replace('\\', "/");
    let normalized = if let Some(s) = path_str.strip_prefix('/') {
        s.to_string()
    } else if path_str.len() >= 2 && path_str.chars().nth(1) == Some(':') {
        // Windows: "C:/..." → "C/..."
        path_str.replacen(':', "", 1).trim_start_matches('/').to_string()
    } else {
        path_str
    };
    format!("{}/{}/{}", root, hostname, normalized)
}
```

## Cross-Platform Config Path

```rust
use dirs::config_dir;
use std::path::PathBuf;

pub fn config_path() -> PathBuf {
    config_dir()
        .expect("Cannot determine config directory")
        .join("shadow")
        .join("config.toml")
}

pub fn hash_db_path() -> PathBuf {
    dirs::home_dir()
        .expect("Cannot determine home directory")
        .join(".shadow")
        .join("hashdb")
}
```

## Retry Pattern

```rust
use tokio_retry::{Retry, strategy::ExponentialBackoff};

pub async fn upload_with_retry(
    provider: &dyn BackupProvider,
    path: &Path,
    key: &str,
) -> anyhow::Result<()> {
    let strategy = ExponentialBackoff::from_millis(1000)
        .factor(4)
        .take(3); // 1s, 4s, 16s

    Retry::spawn(strategy, || provider.upload(path, key)).await
}
```
