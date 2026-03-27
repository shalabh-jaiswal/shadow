---
allowed-tools:
  - Bash
  - Read
  - Edit
---

# Cut a Release: $1

Create and push a release tag for version $1 (format: v1.2.3).

## Pre-Release Checklist

1. **Verify branch and cleanliness**
   ```bash
   git branch --show-current   # must be 'main'
   git status                  # must be clean
   git log --oneline -5        # review recent commits
   ```

2. **Run all quality gates**
   ```bash
   cargo fmt --manifest-path src-tauri/Cargo.toml --check
   cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
   cargo test --manifest-path src-tauri/Cargo.toml
   npm run type-check
   npm run lint
   ```
   **Stop if anything fails.**

3. **Bump version in tauri.conf.json**
   Open `src-tauri/tauri.conf.json` and update the `"version"` field to match $1 (without the `v` prefix).

4. **Commit the version bump**
   ```bash
   git add src-tauri/tauri.conf.json
   git commit -m "chore: bump version to $1"
   ```

5. **Create the tag**
   ```bash
   git tag $1
   ```

6. **Push commit and tag together**
   ```bash
   git push origin main --follow-tags
   ```

7. **Confirm GitHub Actions triggered**
   - Visit `https://github.com/<owner>/shadow/actions`
   - Verify the release workflow is running for tag $1
   - Three jobs should appear: build-macos, build-windows, build-linux

## After the Release
- Monitor the Actions run for any failures
- Once complete, check the GitHub Releases page for the new release artifacts
- Verify `.dmg`, `.exe`/`.msi`, and `.AppImage`/`.deb` are all attached
