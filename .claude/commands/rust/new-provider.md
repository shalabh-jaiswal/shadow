---
allowed-tools:
  - Read
  - Write
  - Edit
  - Bash
  - Grep
  - Glob
---

# Implement New Backup Provider: $1

Scaffold a new backup provider named $1 that implements the `BackupProvider` trait.

## Steps

1. **Read the existing provider implementations for reference**
   ```
   src-tauri/src/providers/mod.rs
   src-tauri/src/providers/s3.rs
   src-tauri/src/providers/nas.rs
   ```

2. **Create the provider file**
   Create `src-tauri/src/providers/$1.rs` implementing:
   ```rust
   pub struct $1Provider { /* config fields */ }

   #[async_trait::async_trait]
   impl BackupProvider for $1Provider {
       fn name(&self) -> &'static str { "$1" }
       async fn upload(&self, local_path: &Path, remote_key: &str) -> anyhow::Result<()> { ... }
       async fn test_connection(&self) -> anyhow::Result<String> { ... }
   }
   ```

3. **Register in mod.rs** — add `pub mod $1;` and include in the provider enum/list

4. **Add config struct** in `src-tauri/src/config.rs`
   ```rust
   #[derive(Debug, Serialize, Deserialize, Clone, Default)]
   pub struct $1Config {
       pub enabled: bool,
       // ... provider-specific fields (NO secrets)
   }
   ```

5. **Add to config.toml schema** — document the new `[providers.$1]` section

6. **Wire IPC** — update `set_provider_config` and `get_provider_config` in `ipc.rs`

7. **Add TypeScript types** in `src/types.ts`

8. **Add provider card** in `src/components/screens/ProvidersScreen.tsx`

9. **Run quality gates**
   ```bash
   cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
   npm run type-check
   ```

## Rules
- Follow the `BackupProvider` trait exactly — no shortcuts
- No secrets in config struct
- `test_connection()` must do a real lightweight connectivity check
- Large file handling: decide upfront whether this provider needs multipart/chunked upload
