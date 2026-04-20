// Shared TypeScript types — matched exactly to Rust structs in src-tauri/src/

// ── Config ────────────────────────────────────────────────────────────────────

export interface DaemonConfig {
  debounce_ms: number;
  upload_workers: number;
  log_level: string;
  follow_symlinks: boolean;
  start_on_login: boolean;
  scan_interval_mins: number;
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

export interface ScanProgressPayload {
  folder: string;
  scanned: number;
  queued: number;
  total: number;
  trigger: 'initial' | 'manual' | 'scheduled';
}

export interface ScanCompletePayload {
  folder: string;
  total_files: number;
  total_bytes: number;
  files_uploaded: number;
  files_skipped: number;
  trigger: 'initial' | 'manual' | 'scheduled';
}

export interface FolderStatus {
  path: string;
  status: string;
  /** Unix timestamp in milliseconds of the last successful upload. Null if never. */
  last_backup: number | null;
  scan_mode: 'full' | 'forward_only';
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

export type ActivityStatus = 'queued' | 'uploading' | 'uploaded' | 'skipped' | 'error' | 'failed' | 'renamed' | 'rename_error';

export interface ActivityEntry {
  id: string;
  timestamp: number;
  status: ActivityStatus;
  path: string;
  /** Basename of the path, for compact display. */
  filename: string;
  /** Map of provider name to its specific status for this file. */
  providers: Record<string, { status: ActivityStatus; error?: string | null }>;
  /** Global error for the entire file operation (if any). */
  error: string | null;
  /** Populated for renamed/rename_error events. */
  old_path?: string;
  new_path?: string;
}

export interface FileRenamedEvent {
  old_path: string;
  new_path: string;
  provider: string;
  old_remote_key: string;
  new_remote_key: string;
}

export interface FileRenameErrorEvent {
  old_path: string;
  new_path: string;
  provider: string;
  error: string;
}
