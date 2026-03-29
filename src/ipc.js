import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
// ── Commands ──────────────────────────────────────────────────────────────────
// All Tauri invoke() calls live here — never call invoke() directly in components.
export const ipc = {
    ping: () => invoke('ping'),
    addFolder: (path) => invoke('add_folder', { path }),
    removeFolder: (path) => invoke('remove_folder', { path }),
    getWatchedFolders: () => invoke('get_watched_folders'),
    testProvider: (providerName) => invoke('test_provider', { providerName }),
    setProviderConfig: (provider, configJson) => invoke('set_provider_config', { provider, configJson }),
    getConfig: () => invoke('get_config'),
    setDaemonConfig: (daemon, machine) => invoke('set_daemon_config', { daemon, machine }),
    getStats: () => invoke('get_stats'),
    clearHashStore: () => invoke('clear_hash_store'),
};
// ── Provider config helpers ───────────────────────────────────────────────────
export const providerConfig = {
    saveS3: (cfg) => ipc.setProviderConfig('s3', JSON.stringify(cfg)),
    saveGcs: (cfg) => ipc.setProviderConfig('gcs', JSON.stringify(cfg)),
    saveNas: (cfg) => ipc.setProviderConfig('nas', JSON.stringify(cfg)),
};
// ── Event subscriptions ───────────────────────────────────────────────────────
// Each helper returns a Promise<UnlistenFn>. Call the unlisten function on cleanup.
export const events = {
    onFileQueued: (cb) => listen('file_queued', (e) => cb(e.payload)),
    onFileUploading: (cb) => listen('file_uploading', (e) => cb(e.payload)),
    onFileUploaded: (cb) => listen('file_uploaded', (e) => cb(e.payload)),
    onFileSkipped: (cb) => listen('file_skipped', (e) => cb(e.payload)),
    onFileError: (cb) => listen('file_error', (e) => cb(e.payload)),
    onFileFailed: (cb) => listen('file_failed', (e) => cb(e.payload)),
    onProviderStatus: (cb) => listen('provider_status', (e) => cb(e.payload)),
};
