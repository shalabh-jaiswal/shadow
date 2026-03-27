---
allowed-tools:
  - Read
  - Edit
  - Write
  - MultiEdit
  - Bash
  - Grep
  - Glob
---

# New Feature: $1

Implement the feature described above following Shadow's architecture and conventions.

## Steps

1. **Read context first**
   - Read `CLAUDE.md` for project conventions
   - Read `.claude/prd.md` to check if this feature is specified in the PRD
   - Read relevant existing files before writing any new code

2. **Plan before coding**
   - Identify which layer(s) are affected: Rust daemon, Tauri IPC, React frontend, or all three
   - List the files that need to be created or modified
   - Identify any new crate dependencies needed

3. **Implement in order**
   - Rust types and logic first (src-tauri/src/)
   - IPC command/event wiring second (ipc.rs + lib.rs)
   - Frontend types to match Rust structs (types.ts)
   - Frontend ipc.ts wrapper
   - React component/store/hook last (src/)

4. **Validate**
   - `cargo fmt --manifest-path src-tauri/Cargo.toml`
   - `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` — zero warnings
   - `cargo test --manifest-path src-tauri/Cargo.toml`
   - `npm run type-check` — zero errors
   - `npm run lint` — zero errors

5. **Summarise**
   - List files created/modified
   - Note any new dependencies added to Cargo.toml or package.json
   - Flag any open questions or deferred work
