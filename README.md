# Shadow

Real-time, cross-platform file backup to AWS S3, Google Cloud Storage, and/or a NAS mount point. Built with Tauri 2 (Rust) + React/TypeScript.

---

## First-Time Setup

### 1. Install prerequisites

- [Rust](https://rustup.rs/) (stable 1.78+)
- [Node.js](https://nodejs.org/) 18+
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
4. Leave "Block all public access" checked.
5. Click **Create bucket**.

**Step 2 — Create an IAM user for Shadow**

1. Go to **IAM → Users → Create user**.
2. Name it `shadow-uploader` (or any name you prefer). No console access needed.
3. On the permissions step, choose **Attach policies directly** → **Create inline policy**.
4. Use the JSON editor and paste:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "s3:PutObject",
        "s3:GetObject",
        "s3:HeadObject"
      ],
      "Resource": "arn:aws:s3:::YOUR-BUCKET-NAME/*"
    },
    {
      "Effect": "Allow",
      "Action": "s3:HeadBucket",
      "Resource": "arn:aws:s3:::YOUR-BUCKET-NAME"
    }
  ]
}
```

5. Replace `YOUR-BUCKET-NAME` with your actual bucket name and save the policy.
6. Finish creating the user.

**Step 3 — Create an access key**

1. Open the newly created user → **Security credentials** tab.
2. Click **Create access key** → choose **Other** as use case.
3. Save the **Access Key ID** and **Secret Access Key** — you will not see the secret again.

**Step 4 — Add credentials to `~/.aws/credentials`**

Open (or create) `~/.aws/credentials` and add a `[shadow]` profile:

```ini
[shadow]
aws_access_key_id     = AKIAIOSFODNN7EXAMPLE
aws_secret_access_key = wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
```

**Step 5 — Configure Shadow**

Edit your Shadow config file (see [Config file location](#config-file-location)):

```toml
[s3]
enabled = true
bucket  = "shadow-backups-yourname"
region  = "us-east-1"
profile = "shadow"
prefix  = ""          # optional — prepended to every remote key
```

---

### 3. Google Cloud Storage setup

> Skip this section if you are not using GCS.

**Step 1 — Create a GCS bucket**

1. Sign in to the [GCP Console](https://console.cloud.google.com/storage).
2. Click **Create bucket**.
3. Choose a unique name (e.g. `shadow-backups-yourname`) and a region close to you.
4. Set access control to **Uniform**.
5. Click **Create**.

**Step 2 — Create a service account**

1. Go to **IAM & Admin → Service Accounts → Create service account**.
2. Name it `shadow-uploader` and click **Create and continue**.
3. Grant it the role **Storage Object Creator** (`roles/storage.objectCreator`).
   - If you also need Shadow to verify bucket access via `test_connection`, add **Storage Legacy Bucket Reader** (`roles/storage.legacyBucketReader`) as a second role.
4. Click **Done**.

**Step 3 — Download a JSON key**

1. Click the service account you just created → **Keys** tab.
2. Click **Add key → Create new key → JSON**.
3. Save the downloaded `.json` file somewhere safe (e.g. `~/.config/shadow/gcs-key.json`).

**Step 4 — Set the environment variable**

Shadow uses [Application Default Credentials](https://cloud.google.com/docs/authentication/application-default-credentials). Point the environment variable at your key file:

```bash
# Add to your shell profile (~/.zshrc, ~/.bashrc, etc.)
export GOOGLE_APPLICATION_CREDENTIALS="$HOME/.config/shadow/gcs-key.json"
```

Reload your shell or restart your terminal session for the change to take effect.

**Step 5 — Configure Shadow**

Edit your Shadow config file:

```toml
[gcs]
enabled = true
bucket  = "shadow-backups-yourname"
prefix  = ""    # optional — prepended to every remote key
```

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

Shadow will write files to `<mount_path>/<hostname>/<original_path>`.

---

## Config file location

| Platform | Path |
|---|---|
| macOS | `~/Library/Application Support/shadow/config.toml` |
| Windows | `%APPDATA%\shadow\config.toml` |
| Linux | `~/.config/shadow/config.toml` |

A default config is created on first launch. Edit it with any text editor.

---

## Running in development

```bash
# Start dev server (hot-reloads both Rust and React)
cargo tauri dev

# Run Rust tests
cargo test --manifest-path src-tauri/Cargo.toml

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
<bucket_or_nas_root>/<machine_hostname>/<normalized_absolute_path>
```

Example on macOS:
```
shadow-backups-yourname/JOHNS-MAC/Users/john/Documents/report.pdf
```

Windows paths have backslashes replaced with forward slashes and the colon stripped:
```
shadow-backups-yourname/JOHNS-PC/C/Users/john/Documents/report.pdf
```
