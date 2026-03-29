# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

# Shadow — Project Memory

## What This App Does
Shadow is a real-time, cross-platform file backup desktop app. It watches folders using
OS-native kernel events and instantly backs up new or modified files to AWS S3, Google
Cloud Storage, and/or a NAS mount point. Built with Tauri 2 (Rust backend) + React/TypeScript
frontend. Targets macOS, Windows, and Linux.

## Stack at a Glance
- **Backend:** Rust (stable 1.78+), Tauri 2, tokio async runtime
- **Frontend:** React 18, TypeScript 5, Tailwind CSS 3, Zustand 4, Vite 5
- **Key crates:** notify v6, blake3, sled, aws-sdk-s3, google-cloud-storage, walkdir, tokio-retry, tracing
- **CI/CD:** GitHub Actions — tag-triggered release builds on 3 platforms

## Project Structure
```
shadow/
├── CLAUDE.md
├── .claude/                         # Claude Code configuration (this directory)
├── src/                             # React/TypeScript frontend
│   ├── components/
│   │   ├── layout/                  # Sidebar, Layout
│   │   ├── screens/                 # Dashboard, Folders, Providers, Settings
│   │   └── shared/                  # StatusBadge, ActivityFeed, ConfirmModal
│   ├── store/                       # Zustand stores
│   ├── hooks/                       # useActivityFeed, useProviderStatus
│   ├── ipc.ts                       # Typed Tauri invoke() wrappers
│   ├── types.ts                     # Shared TypeScript types
│   └── main.tsx
├── src-tauri/
│   ├── src/
│   │   ├── main.rs                  # Tauri app builder, tray, window
│   │   ├── lib.rs                   # Command registration
│   │   ├── config.rs                # AppConfig struct, load/save
│   │   ├── path_utils.rs            # remote_key() construction
│   │   ├── ipc.rs                   # All #[tauri::command] handlers
│   │   ├── daemon/
│   │   │   ├── mod.rs               # DaemonState, startup/shutdown
│   │   │   ├── watcher.rs           # notify-rs watcher
│   │   │   ├── debouncer.rs         # 200ms per-path debounce
│   │   │   ├── hasher.rs            # blake3 + sled hash store
│   │   │   ├── queue.rs             # tokio mpsc upload queue
│   │   │   └── scanner.rs           # Initial recursive scan
│   │   └── providers/
│   │       ├── mod.rs               # BackupProvider trait
│   │       ├── s3.rs
│   │       ├── gcs.rs
│   │       └── nas.rs
│   ├── Cargo.toml
│   └── tauri.conf.json
├── .github/workflows/release.yml
├── package.json
├── vite.config.ts
└── tailwind.config.js
```

## Build & Dev Commands
```bash
# Install dependencies (run once)
npm install

# Start dev server (hot reload for both frontend and Rust)
cargo tauri dev

# Build release for current platform
cargo tauri build

# Run all Rust tests
cargo test --manifest-path src-tauri/Cargo.toml

# Run a single Rust test by name
cargo test --manifest-path src-tauri/Cargo.toml <test_name>

# Run frontend type check
npm run type-check

# Run frontend linter
npm run lint

# Format Rust code
cargo fmt --manifest-path src-tauri/Cargo.toml

# Clippy (Rust linter)
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```

## Core Architecture Rules — ALWAYS Follow These

### Rust Backend
- ALL file I/O, hashing, uploading, and watching logic lives in Rust. Never in the frontend.
- Use `anyhow::Result` for error propagation internally. Convert to `String` only at the IPC boundary.
- Use `thiserror` for defining custom error types in library code.
- Every async function must be inside a `tokio::spawn` or a `#[tokio::main]` / Tauri command context.
- The `BackupProvider` trait MUST be used for all provider implementations — never call S3/GCS/NAS code directly from the daemon.
- Debounce window is 200ms. Never hardcode other values — read from `AppConfig`.
- blake3 hash MUST be checked before every upload. Never skip the hash check.
- sled DB path: `~/.shadow/hashdb/` — use the `dirs` crate to resolve the home directory cross-platform.
- Upload concurrency default is 4. Always enforce the configured cap via a `tokio::sync::Semaphore`.
- Retry policy: 3 attempts, exponential backoff 1s/4s/16s using `tokio-retry`.
- All IPC events MUST be emitted via `app_handle.emit()` — never return large data sets from commands, stream them as events instead.
- NEVER store secrets (AWS keys, GCS tokens) in config.toml. Read from env vars / credential chains only.

### Tauri IPC
- All Tauri commands are defined in `src-tauri/src/ipc.rs` and registered in `lib.rs`.
- Command names use snake_case. Event names use snake_case.
- Commands return `Result<T, String>` at the boundary — unwrap anyhow errors with `.map_err(|e| e.to_string())`.
- Frontend NEVER calls Tauri commands directly — all calls go through `src/ipc.ts` typed wrappers.

### React Frontend
- Components are functional only — no class components.
- State that crosses components lives in Zustand stores, not in useState prop-drilling.
- All Tauri event subscriptions are set up in custom hooks (`src/hooks/`), not inline in components.
- Tailwind only for styling — no inline styles, no CSS modules, no styled-components.
- Never use `any` in TypeScript. Use `unknown` and narrow types explicitly.
- All types shared with the Rust backend are defined in `src/types.ts` and match the Rust structs exactly.

### Remote Path Convention
```
<bucket_or_nas_root>/<machine_hostname>/<normalized_absolute_path>
# Example: my-backups/JOHNS-MAC/Users/john/Documents/report.pdf
# Windows paths: C:\Users\... → C/Users/...  (backslash → forward slash, colon stripped)
```

### Git Conventions
- Branch naming: `feat/`, `fix/`, `chore/`, `refactor/` prefixes
- Commit style: Conventional Commits — `feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`
- Never commit directly to `main`
- Version tags trigger CI/CD: `git tag v1.0.0 && git push origin v1.0.0`
- Version lives in `src-tauri/tauri.conf.json` — bump it before tagging

### Code Quality Gates
- Rust: `cargo clippy -- -D warnings` must pass with zero warnings
- Rust: `cargo fmt --check` must pass
- TypeScript: `npm run type-check` must pass with zero errors
- No `unwrap()` or `expect()` in production code paths — use proper error handling
- No `console.log` in committed frontend code — use the structured logging system

## Config File Location (per platform)
| Platform | Path |
|---|---|
| macOS | `~/Library/Application Support/shadow/config.toml` |
| Windows | `%APPDATA%\shadow\config.toml` |
| Linux | `~/.config/shadow/config.toml` |

## Key PRD Reference
Full PRD is at `.claude/prd.md`. When in doubt about requirements, consult it.
All functional requirements (FR-01 through FR-10) and non-functional requirements
(NFR-01 through NFR-10) are binding.

## Distribution & Signing
This is a personal-use app. Code signing is intentionally skipped.
- No Apple Developer certificate
- No Windows Authenticode certificate
- No TAURI_SIGNING_PRIVATE_KEY required in CI
- GitHub Actions builds unsigned installers only
- README documents the OS security warning bypass for users

## Current Milestone
See `.claude/MILESTONE.md` for the current active milestone and its exit criteria.
Update it when a milestone is completed.
