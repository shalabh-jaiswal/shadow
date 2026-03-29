import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  AppConfig,
  DaemonConfig,
  DaemonStats,
  FileEvent,
  FolderStatus,
  GcsConfig,
  MachineConfig,
  NasConfig,
  ProviderStatusEvent,
  S3Config,
} from './types';

// ── Commands ──────────────────────────────────────────────────────────────────
// All Tauri invoke() calls live here — never call invoke() directly in components.

export const ipc = {
  ping: (): Promise<string> =>
    invoke<string>('ping'),

  addFolder: (path: string): Promise<void> =>
    invoke<void>('add_folder', { path }),

  removeFolder: (path: string): Promise<void> =>
    invoke<void>('remove_folder', { path }),

  getWatchedFolders: (): Promise<FolderStatus[]> =>
    invoke<FolderStatus[]>('get_watched_folders'),

  testProvider: (providerName: string): Promise<string> =>
    invoke<string>('test_provider', { providerName }),

  setProviderConfig: (provider: string, configJson: string): Promise<void> =>
    invoke<void>('set_provider_config', { provider, configJson }),

  getConfig: (): Promise<AppConfig> =>
    invoke<AppConfig>('get_config'),

  setDaemonConfig: (daemon: DaemonConfig, machine: MachineConfig): Promise<void> =>
    invoke<void>('set_daemon_config', { daemon, machine }),

  getStats: (): Promise<DaemonStats> =>
    invoke<DaemonStats>('get_stats'),

  clearHashStore: (): Promise<void> =>
    invoke<void>('clear_hash_store'),
} as const;

// ── Provider config helpers ───────────────────────────────────────────────────

export const providerConfig = {
  saveS3: (cfg: S3Config) => ipc.setProviderConfig('s3', JSON.stringify(cfg)),
  saveGcs: (cfg: GcsConfig) => ipc.setProviderConfig('gcs', JSON.stringify(cfg)),
  saveNas: (cfg: NasConfig) => ipc.setProviderConfig('nas', JSON.stringify(cfg)),
} as const;

// ── Event subscriptions ───────────────────────────────────────────────────────
// Each helper returns a Promise<UnlistenFn>. Call the unlisten function on cleanup.

export const events = {
  onFileQueued: (cb: (e: FileEvent) => void): Promise<UnlistenFn> =>
    listen<FileEvent>('file_queued', (e) => cb(e.payload)),

  onFileUploading: (cb: (e: FileEvent) => void): Promise<UnlistenFn> =>
    listen<FileEvent>('file_uploading', (e) => cb(e.payload)),

  onFileUploaded: (cb: (e: FileEvent) => void): Promise<UnlistenFn> =>
    listen<FileEvent>('file_uploaded', (e) => cb(e.payload)),

  onFileSkipped: (cb: (e: FileEvent) => void): Promise<UnlistenFn> =>
    listen<FileEvent>('file_skipped', (e) => cb(e.payload)),

  onFileError: (cb: (e: FileEvent) => void): Promise<UnlistenFn> =>
    listen<FileEvent>('file_error', (e) => cb(e.payload)),

  onFileFailed: (cb: (e: FileEvent) => void): Promise<UnlistenFn> =>
    listen<FileEvent>('file_failed', (e) => cb(e.payload)),

  onProviderStatus: (cb: (e: ProviderStatusEvent) => void): Promise<UnlistenFn> =>
    listen<ProviderStatusEvent>('provider_status', (e) => cb(e.payload)),
} as const;
