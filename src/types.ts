// Shared TypeScript types — matched exactly to Rust structs in src-tauri/src/

export interface DaemonConfig {
  debounce_ms: number;
  upload_workers: number;
  log_level: string;
  follow_symlinks: boolean;
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
  /** Optional key prefix prepended to every remote path */
  prefix: string;
}

export interface GcsConfig {
  enabled: boolean;
  bucket: string;
  project_id: string;
  /** Absolute path to the GCS service account JSON key file */
  credentials_path: string;
  /** Optional key prefix prepended to every remote path */
  prefix: string;
}

export interface AppConfig {
  daemon: DaemonConfig;
  machine: MachineConfig;
  nas: NasConfig;
  s3: S3Config;
  gcs: GcsConfig;
}

export interface FolderStatus {
  path: string;
  status: string;
}

export interface FileEvent {
  path: string;
  provider: string | null;
  error: string | null;
}
