# Shadow

Shadow is a real-time, cross-platform file backup desktop app. It watches folders using OS-native kernel events and instantly backs up new or modified files to AWS S3, Google Cloud Storage, and/or a NAS mount point.

## Features

- **Real-time backup**: Monitors file changes using OS kernel events (no polling)
- **Multi-cloud support**: Backup to AWS S3, Google Cloud Storage, and NAS simultaneously
- **Cross-platform**: Runs on macOS, Windows, and Linux
- **Intelligent deduplication**: Uses blake3 hashing to skip unchanged files
- **System tray integration**: Runs in background with tray icon
- **Pause/resume**: Control backup process from tray menu
- **Auto-start**: Launch automatically at login (optional)
- **Auto-update**: Built-in update checker

## Download & Install

### macOS

1. Download the latest `.dmg` file from [Releases](https://github.com/YOUR_GITHUB_USERNAME/shadow/releases)
2. Open the DMG and drag Shadow to Applications
3. **Security**: On first launch, you may see "Shadow cannot be opened because it is from an unidentified developer"
   - Go to **System Preferences > Security & Privacy > General**
   - Click **"Open Anyway"** next to the Shadow warning
   - Alternatively, right-click the app and select **"Open"**, then click **"Open"** in the dialog

### Windows

1. Download the latest `.exe` installer from [Releases](https://github.com/YOUR_GITHUB_USERNAME/shadow/releases)
2. Run the installer
3. **Security**: If Windows Defender SmartScreen appears:
   - Click **"More info"**
   - Click **"Run anyway"**

### Linux

#### Ubuntu/Debian (.deb)
```bash
# Download the .deb file from Releases, then:
sudo dpkg -i shadow_*.deb
sudo apt-get install -f  # Fix any missing dependencies
```

#### Other Distributions (.AppImage)
```bash
# Download the .AppImage file from Releases, then:
chmod +x shadow_*.AppImage
./shadow_*.AppImage
```

## Configuration

Shadow stores its configuration in platform-specific locations:

| Platform | Configuration Path |
|---|---|
| **macOS** | `~/Library/Application Support/shadow/config.toml` |
| **Windows** | `%APPDATA%\shadow\config.toml` |
| **Linux** | `~/.config/shadow/config.toml` |

The configuration file is automatically created with default values when you first run Shadow. Use the Settings screen within the app to modify most settings.

### Provider Setup

#### AWS S3
- Credentials: Uses the AWS credential chain (environment variables, `~/.aws/credentials`, or IAM roles)
- Required permissions: `s3:PutObject`, `s3:GetObject`, `s3:ListBucket`

#### Google Cloud Storage
- Credentials: Uses Application Default Credentials or a service account JSON file
- Required permissions: `storage.objects.create`, `storage.objects.get`, `storage.buckets.get`

#### NAS
- Mount the NAS as a local directory (SMB, NFS, etc.)
- Shadow will write files directly to the mount point
- No additional credentials needed

## Building from Source

### Prerequisites
- **Node.js** 18+ and npm
- **Rust** 1.78+ (install via [rustup](https://rustup.rs/))
- **Platform-specific dependencies**:
  - **Linux**: `sudo apt-get install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libgtk-3-dev`

### Development
```bash
git clone https://github.com/YOUR_GITHUB_USERNAME/shadow.git
cd shadow
npm install
npm run tauri dev
```

### Production Build
```bash
npm run tauri build
```

Binaries will be created in `src-tauri/target/release/bundle/`.

### Testing
```bash
# Rust tests
cargo test --manifest-path src-tauri/Cargo.toml

# TypeScript type checking
npm run type-check

# Code formatting
cargo fmt --manifest-path src-tauri/Cargo.toml
npm run lint
```

## License

[Add your license here]