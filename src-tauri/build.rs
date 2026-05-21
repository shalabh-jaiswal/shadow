fn main() {
    // For LOCAL development: load .env from the workspace root and forward the
    // Google Drive credentials to option_env!() via cargo:rustc-env.
    //
    // In CI: credentials are injected as real environment variables
    // (SHADOW_GDRIVE_CLIENT_ID / SHADOW_GDRIVE_CLIENT_SECRET set as GitHub Secrets).
    // option_env!() reads those directly — no need to echo them through build.rs,
    // which would risk exposing them in verbose build logs.
    let is_ci = std::env::var("CI").is_ok();

    if !is_ci {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let env_path = std::path::Path::new(&manifest_dir)
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join(".env");

        if let Ok(iter) = dotenvy::from_path_iter(&env_path) {
            for item in iter.flatten() {
                let (key, val) = item;
                match key.as_str() {
                    "SHADOW_GDRIVE_CLIENT_ID" | "SHADOW_GDRIVE_CLIENT_SECRET" => {
                        let clean = val.trim().trim_matches('"').trim_matches('\'');
                        println!("cargo:rustc-env={}={}", key, clean);
                    }
                    _ => {}
                }
            }
        }
    }

    // Always tell Cargo to rerun if these env vars change (covers CI too).
    println!("cargo:rerun-if-env-changed=SHADOW_GDRIVE_CLIENT_ID");
    println!("cargo:rerun-if-env-changed=SHADOW_GDRIVE_CLIENT_SECRET");

    tauri_build::build()
}

