// Shared TypeScript types — matched exactly to Rust structs in src-tauri/src/

// ── Config ────────────────────────────────────────────────────────────────────

export interface DaemonConfig {
  debounce_ms: number;
  upload_workers: number;
  log_level: string;
  follow_symlinks: boolean;
  start_on_login: boolean;
}

export interface MachineConfig {
  /** Overrides the OS hostname in remote paths. Leave empty to use the OS hostname. */
  name: string;
}

export interface NasConfig {
  enabled: boolean;
  mount_path: string;
}

export interface S3Config {
  enabled: boolean;
  bucket: string;
  region: string;
  /** Named profile in ~/.aws/credentials (e.g. "shadow") */
  profile: string;
  prefix: string;
}

export interface GcsConfig {
  enabled: boolean;
  bucket: string;
  project_id: string;
  /** Absolute path to the GCS service account JSON key file */
  credentials_path: string;
  prefix: string;
}

export interface AppConfig {
  daemon: DaemonConfig;
  machine: MachineConfig;
  nas: NasConfig;
  s3: S3Config;
  gcs: GcsConfig;
}

// ── IPC event payloads ────────────────────────────────────────────────────────

export interface FolderStatus {
  path: string;
  status: string;
  /** Unix timestamp in milliseconds of the last successful upload. Null if never. */
  last_backup: number | null;
}

export interface FileEvent {
  path: string;
  provider: string | null;
  error: string | null;
}

export interface ProviderStatusEvent {
  provider: string;
  status: 'ok' | 'error';
  error?: string | null;
}

// ── Stats ─────────────────────────────────────────────────────────────────────

export interface DaemonStats {
  files_uploaded: number;
  bytes_uploaded: number;
  active_uploads: number;
  queue_depth: number;
}

// ── Activity feed ─────────────────────────────────────────────────────────────

export type ActivityStatus = 'queued' | 'uploading' | 'uploaded' | 'skipped' | 'error' | 'failed';

export interface ActivityEntry {
  id: string;
  timestamp: number;
  status: ActivityStatus;
  path: string;
  /** Basename of the path, for compact display. */
  filename: string;
  provider: string | null;
  error: string | null;
}
