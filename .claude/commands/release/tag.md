---
allowed-tools:
  - Bash
  - Read
  - Edit
---

# Release: $1

Cut a new release of Shadow at version $1 (format: v1.2.3).

## Pre-Release Checklist

1. **Verify branch and cleanliness**
```bash
   git branch --show-current      # confirm you are on main
   git status                     # must be clean, no uncommitted changes
   git log --oneline -5           # review recent commits look sane
```
Stop if anything is dirty.

2. **Run full quality gates**
```bash
   cargo fmt --manifest-path src-tauri/Cargo.toml --check
   cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
   cargo test --manifest-path src-tauri/Cargo.toml
   npm run type-check
   npm run lint
```
Stop if anything fails. Fix before proceeding.

3. **Bump version in tauri.conf.json**
   Open `src-tauri/tauri.conf.json` and change:
```json
   "version": "X.Y.Z"
```
to match $1 without the `v` prefix.

4. **Update MILESTONE.md**
   Mark the current milestone complete with today's date.

5. **Commit the version bump**
```bash
   git add src-tauri/tauri.conf.json .claude/MILESTONE.md
   git commit -m "chore: bump version to $1"
```

6. **Create the tag**
```bash
   git tag $1
```

7. **Push commit and tag together**
```bash
   git push origin main --follow-tags
```

8. **Confirm GitHub Actions triggered**
   Print this URL for the user to check:
```
   https://github.com/<owner>/shadow/actions
```
Three jobs should appear simultaneously:
- build-macos
- build-windows
- build-linux

After all three pass, publish-release creates the GitHub Release
with installers attached.

## Notes
- No code signing — users will see OS security warnings on first install
- macOS bypass: right-click app → Open → Open
- Windows bypass: More info → Run anyway
- Installers are unsigned but fully functional
- Auto-updater will notify existing users of the new version
```
