# Shadow

Real-time, cross-platform file backup to AWS S3, Google Cloud Storage, and/or a NAS mount point. Built with Tauri 2 (Rust) + React/TypeScript.

---

## First-Time Setup

### 1. Install prerequisites

- [Rust](https://rustup.rs/) (stable 1.78+)
- [Node.js](https://nodejs.org/) 20+
- [Tauri CLI](https://tauri.app/start/prerequisites/)

```bash
npm install
```

---

### 2. AWS S3 setup

> Skip this section if you are not using S3.

**Step 1 — Create an S3 bucket**

1. Sign in to the [AWS Console](https://console.aws.amazon.com/s3).
2. Click **Create bucket**.
3. Choose a unique bucket name (e.g. `shadow-backups-yourname`) and a region close to you (e.g. `us-east-1`).
4. Leave **Block all public access** checked.
5. Click **Create bucket**.

**Step 2 — Create an IAM user for Shadow**

1. Go to **IAM → Users → Create user**.
2. Name it `shadow-uploader`. Leave **Provide user access to the AWS Management Console** unchecked — Shadow only needs programmatic access.
3. On the permissions step, choose **Attach policies directly → Create inline policy**.
4. Switch to the **JSON** editor and paste:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "ShadowBackup",
      "Effect": "Allow",
      "Action": [
        "s3:ListBucket",
        "s3:GetObject",
        "s3:PutObject"
      ],
      "Resource": [
        "arn:aws:s3:::YOUR-BUCKET-NAME",
        "arn:aws:s3:::YOUR-BUCKET-NAME/*"
      ]
    }
  ]
}
```

5. Replace `YOUR-BUCKET-NAME` with your actual bucket name.
6. Name the policy `shadow-uploader-policy` and save it.
7. Finish creating the user.

> **Why these three actions?**
> `s3:ListBucket` — lets Shadow verify the bucket is accessible (test connection).
> `s3:GetObject` — covers both read and HeadObject API calls.
> `s3:PutObject` — uploads files.

**Step 3 — Create an access key**

1. Open the newly created user → **Security credentials** tab.
2. Click **Create access key** → choose **Local code** as the use case.
3. Save the **Access Key ID** and **Secret Access Key** — the secret is shown only once. Download the CSV as a backup and store it somewhere safe outside the project.

**Step 4 — Add credentials to `~/.aws/credentials`**

Open (or create) `~/.aws/credentials`:

```bash
# macOS / Linux
mkdir -p ~/.aws
nano ~/.aws/credentials
```

```powershell
# Windows
mkdir ~\.aws
notepad ~\.aws\credentials
```

Add a `[shadow]` profile:

```ini
[shadow]
aws_access_key_id     = AKIAIOSFODNN7EXAMPLE
aws_secret_access_key = wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
```

Also create `~/.aws/config` (same location):

```ini
[profile shadow]
region = us-east-1
```

> If you already have other AWS profiles in `~/.aws/credentials`, simply add the `[shadow]` block alongside them. Shadow will use only this profile.

**Step 5 — Configure Shadow**

Edit your Shadow config file (see [Config file location](#config-file-location)):

```toml
[s3]
enabled = true
bucket  = "shadow-backups-yourname"
region  = "us-east-1"
profile = "shadow"
endpoint = ""     # optional — for S3-compatible services like MinIO
```

---

### 3. Google Cloud Storage setup

> Skip this section if you are not using GCS.

**Step 1 — Create a GCP project**

1. Sign in to the [GCP Console](https://console.cloud.google.com).
2. Click the project dropdown at the top → **New Project**.
3. Name it (e.g. `shadow-backup`) and click **Create**.

> If you already have an existing GCP project you want to use, skip this step.

**Step 2 — Enable billing**

GCS requires a billing account even for very low usage (personal backup costs cents per month).

```
GCP Console → Billing → link a billing account to your project
```

**Step 3 — Enable the Cloud Storage API**

```
APIs & Services → Library → search "Cloud Storage JSON API" → Enable
```

**Step 4 — Create a GCS bucket**

1. Go to **Cloud Storage → Buckets → Create bucket**.
2. Choose a globally unique name (e.g. `shadow-backups-yourname`) and a region close to you.
3. Set access control to **Uniform**.
4. Click **Create**.

**Step 5 — Create a service account**

1. Go to **IAM & Admin → Service Accounts → Create service account**.
2. Name it `shadow-uploader` and click **Create and continue**.
3. Grant it the role **Storage Object Admin** (`roles/storage.objectAdmin`).

> **Why Object Admin?** `Storage Object Creator` can only create new objects — it cannot overwrite an existing backup of the same file. Object Admin allows create, overwrite, and read, which Shadow needs for incremental backups.

4. Click **Done**.

**Step 6 — Download a JSON key**

1. Click the `shadow-uploader` service account → **Keys** tab.
2. Click **Add key → Create new key → JSON**.
3. Save the downloaded `.json` file:

```bash
mkdir -p ~/.config/shadow
mv ~/Downloads/your-key-file.json ~/.config/shadow/gcs-key.json
chmod 600 ~/.config/shadow/gcs-key.json
```

The `chmod 600` restricts the key file to your user only.

**Step 7 — Configure Shadow**

Edit your Shadow config file (see [Config file location](#config-file-location)):

```toml
[gcs]
enabled          = true
bucket           = "shadow-backups-yourname"
project_id       = "shadow-backup"
credentials_path = "/Users/yourname/.config/shadow/gcs-key.json"
```

> Shadow reads the GCS key file directly from `credentials_path` — no environment variable export needed. The credential never leaves Shadow's config.

---

### 4. NAS setup

> Skip this section if you are not using a NAS mount.

Mount your NAS volume so it appears as a local directory, then configure Shadow:

```toml
[nas]
enabled    = true
mount_path = "/Volumes/MyNAS/Shadow"   # macOS example
# mount_path = "Z:\\Shadow"            # Windows example
```

Shadow will write files to `<mount_path>/<machine_name>/<original_path>`.

---

## Config file location

| Platform | Path |
|---|---|
| macOS | `~/Library/Application Support/shadow/config.toml` |
| Windows | `%APPDATA%\shadow\config.toml` |
| Linux | `~/.config/shadow/config.toml` |

A default config is created on first launch. Edit it with any text editor.

### Full config.toml reference

```toml
[machine]
name = "home-mac"    # friendly name used in remote paths — defaults to system hostname if empty

[daemon]
debounce_ms    = 200     # milliseconds to wait after last file event before uploading (50–5000)
upload_workers = 4       # parallel upload workers (1–16)
log_level      = "info"  # error | warn | info | debug
follow_symlinks = false

[watched_folders]
paths = [
  "/Users/yourname/Documents",
  "/Users/yourname/Projects"
]

[s3]
enabled  = false
bucket   = ""
region   = "us-east-1"
profile  = "shadow"     # AWS credentials profile from ~/.aws/credentials
endpoint = ""           # optional: custom endpoint for S3-compatible services (e.g. MinIO)

[gcs]
enabled          = false
bucket           = ""
project_id       = ""
credentials_path = ""   # absolute path to GCS service account JSON key file

[nas]
enabled    = false
mount_path = ""
```

---

## Running in development

```bash
# Install frontend dependencies (first time or after package.json changes)
npm install

# Start dev server (hot-reloads both Rust and React)
cargo tauri dev

# Run Rust tests
cargo test --manifest-path src-tauri/Cargo.toml

# Lint Rust code
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings

# Type-check frontend
npm run type-check

# Lint frontend
npm run lint
```

## Building a release

```bash
cargo tauri build
```

Produces a platform-native installer in `src-tauri/target/release/bundle/`.

---

## Remote path convention

Files are stored at:

```
<bucket_or_nas_root>/<machine_name>/<normalized_absolute_path>
```

The `machine_name` is taken from `[machine] name` in your config (defaults to system hostname if not set).

**macOS / Linux example:**
```
shadow-backups-yourname/home-mac/Users/john/Documents/report.pdf
```

**Windows example** — backslashes replaced with forward slashes, drive colon stripped:
```
shadow-backups-yourname/home-pc/C/Users/john/Documents/report.pdf
```

Files from different machines are automatically separated by their machine name, so multiple machines can safely back up to the same bucket.