import { invoke } from "@tauri-apps/api/core";
import type { FolderStatus, GcsConfig, NasConfig, S3Config } from "./types";

export async function ping(): Promise<string> {
  return invoke<string>("ping");
}

export async function addFolder(path: string): Promise<void> {
  return invoke<void>("add_folder", { path });
}

export async function removeFolder(path: string): Promise<void> {
  return invoke<void>("remove_folder", { path });
}

export async function getWatchedFolders(): Promise<FolderStatus[]> {
  return invoke<FolderStatus[]>("get_watched_folders");
}

/**
 * Test connectivity to a configured provider.
 * @param providerName - "s3" | "gcs" | "nas"
 * @returns A human-readable success message, or throws on failure.
 */
export async function testProvider(providerName: string): Promise<string> {
  return invoke<string>("test_provider", { providerName });
}

/**
 * Persist an updated provider config block to config.toml.
 * The running daemon will pick up the change on next app restart.
 * @param provider - "s3" | "gcs" | "nas"
 * @param config   - The config object for that provider
 */
export async function setProviderConfig(
  provider: "s3",
  config: S3Config,
): Promise<void>;
export async function setProviderConfig(
  provider: "gcs",
  config: GcsConfig,
): Promise<void>;
export async function setProviderConfig(
  provider: "nas",
  config: NasConfig,
): Promise<void>;
export async function setProviderConfig(
  provider: string,
  config: S3Config | GcsConfig | NasConfig,
): Promise<void> {
  return invoke<void>("set_provider_config", {
    provider,
    configJson: JSON.stringify(config),
  });
}
