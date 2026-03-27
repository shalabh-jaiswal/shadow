---
name: tauri-ipc
description: |
  Tauri 2 IPC patterns for Shadow. Activates when writing Tauri commands,
  events, app_handle usage, or frontend invoke/listen calls. Covers: command
  registration, event emission, typed wrappers in ipc.ts, event cleanup,
  app_handle passing patterns, state management via Tauri managed state.
allowed-tools:
  - Read
---

# Tauri IPC Patterns for Shadow

## Rust Command Definition Pattern

```rust
// src-tauri/src/ipc.rs

use tauri::State;
use crate::daemon::DaemonState;

// ✅ CORRECT — always return Result<T, String> at IPC boundary
#[tauri::command]
pub async fn add_folder(
    path: String,
    state: State<'_, DaemonState>,
) -> Result<(), String> {
    state.add_folder(path).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_watched_folders(
    state: State<'_, DaemonState>,
) -> Result<Vec<FolderStatus>, String> {
    state.get_folders().await.map_err(|e| e.to_string())
}
```

## Command Registration Pattern

```rust
// src-tauri/src/lib.rs

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(DaemonState::new())
        .invoke_handler(tauri::generate_handler![
            ipc::add_folder,
            ipc::remove_folder,
            ipc::get_watched_folders,
            ipc::set_provider_config,
            ipc::get_provider_config,
            ipc::test_provider,
            ipc::get_activity_log,
            ipc::retry_failed,
            ipc::get_stats,
            ipc::set_daemon_config,
            ipc::clear_hash_store,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

## Event Emission from Rust

```rust
// Always emit events via app_handle, not window handle
// Pass app_handle to daemon via Arc or store as Tauri managed state

use tauri::Manager;

fn emit_file_uploaded(app: &tauri::AppHandle, payload: FileUploadedEvent) {
    app.emit("file_uploaded", payload)
        .unwrap_or_else(|e| tracing::warn!("Failed to emit event: {}", e));
}

// Event payload structs must derive Serialize
#[derive(serde::Serialize, Clone)]
pub struct FileUploadedEvent {
    pub path: String,
    pub provider: String,
    pub duration_ms: u64,
    pub size_bytes: u64,
    pub remote_key: String,
}
```

## TypeScript IPC Wrapper Pattern (src/ipc.ts)

```typescript
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import type {
  FolderStatus, AllProviderConfig, LogEntry,
  DaemonStats, DaemonConfig, ProviderConfig
} from './types';

// Commands — all invoke() calls live here, nowhere else
export const ipc = {
  addFolder: (path: string) =>
    invoke<void>('add_folder', { path }),

  removeFolder: (path: string) =>
    invoke<void>('remove_folder', { path }),

  getWatchedFolders: () =>
    invoke<FolderStatus[]>('get_watched_folders'),

  setProviderConfig: (config: ProviderConfig) =>
    invoke<void>('set_provider_config', { config }),

  getProviderConfig: () =>
    invoke<AllProviderConfig>('get_provider_config'),

  testProvider: (provider: 'S3' | 'GCS' | 'NAS') =>
    invoke<string>('test_provider', { provider }),

  getActivityLog: (limit: number) =>
    invoke<LogEntry[]>('get_activity_log', { limit }),

  retryFailed: (filePath: string) =>
    invoke<void>('retry_failed', { filePath }),

  getStats: () =>
    invoke<DaemonStats>('get_stats'),

  setDaemonConfig: (config: DaemonConfig) =>
    invoke<void>('set_daemon_config', { config }),

  clearHashStore: () =>
    invoke<void>('clear_hash_store'),
};

// Event subscriptions — helpers for hooks to use
export const events = {
  onFileUploaded: (cb: (p: FileUploadedPayload) => void): Promise<UnlistenFn> =>
    listen('file_uploaded', e => cb(e.payload as FileUploadedPayload)),

  onFileQueued: (cb: (p: FileQueuedPayload) => void): Promise<UnlistenFn> =>
    listen('file_queued', e => cb(e.payload as FileQueuedPayload)),

  onFileFailed: (cb: (p: FileFailedPayload) => void): Promise<UnlistenFn> =>
    listen('file_failed', e => cb(e.payload as FileFailedPayload)),

  onScanProgress: (cb: (p: ScanProgressPayload) => void): Promise<UnlistenFn> =>
    listen('scan_progress', e => cb(e.payload as ScanProgressPayload)),

  onProviderStatus: (cb: (p: ProviderStatusPayload) => void): Promise<UnlistenFn> =>
    listen('provider_status', e => cb(e.payload as ProviderStatusPayload)),
};
```

## Event Subscription Hook Pattern (src/hooks/)

```typescript
// src/hooks/useActivityFeed.ts
import { useEffect } from 'react';
import { events } from '../ipc';
import { useActivityStore } from '../store/activityStore';

// ✅ CORRECT — subscribe in hook, clean up on unmount
export function useActivityFeed() {
  const addEntry = useActivityStore(s => s.addEntry);

  useEffect(() => {
    // Store the promise so we can clean up
    const unlistenUploaded = events.onFileUploaded((payload) => {
      addEntry({ type: 'uploaded', ...payload, timestamp: new Date().toISOString() });
    });

    const unlistenFailed = events.onFileFailed((payload) => {
      addEntry({ type: 'failed', ...payload, timestamp: new Date().toISOString() });
    });

    return () => {
      unlistenUploaded.then(fn => fn());
      unlistenFailed.then(fn => fn());
    };
  }, []); // empty deps — subscribe once on mount
}
```
