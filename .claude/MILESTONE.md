# Shadow — Current Milestone

## Active: M1 — Scaffold

### Scope
- Tauri 2 project init with Rust workspace
- React + Tailwind CSS shell
- IPC hello-world (one command, one event)
- GitHub Actions skeleton (builds on all 3 platforms)
- Basic tray icon

### Exit Criteria
- [ ] `cargo tauri dev` starts with no errors on macOS, Windows, and Linux
- [ ] A test IPC command can be invoked from the React UI and returns a response
- [ ] GitHub Actions workflow file exists and CI builds green binaries on all 3 platforms from a tag push
- [ ] Tray icon appears with a "Quit" menu item

### Next Milestone: M2 — Core Daemon
FS watcher, debouncer, blake3 hasher, sled hash store, upload queue, NAS provider.
Exit: Files written to a watched folder appear on a NAS mount within 500ms.

---

## Completed Milestones
_None yet._

---

## Milestone Reference
| # | Name | Exit Criteria |
|---|---|---|
| M1 | Scaffold | CI green on all 3 platforms |
| M2 | Core Daemon | Files on NAS within 500ms |
| M3 | Cloud Providers | S3 + GCS working with multipart |
| M4 | Full UI | All 4 screens, tray, login item |
| M5 | Initial Scan | 10k file folder fully backed up |
| M6 | Polish & Release | Signed installers on GitHub Releases |
