fn main() {
    // Read .env from the workspace root (parent of src-tauri/) and forward
    // the Google Drive credentials as rustc-env so that option_env!() macros
    // in the main crate can see them at compile time.
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
                    // Strip surrounding quotes if the .env file used them.
                    let clean = val.trim().trim_matches('"').trim_matches('\'');
                    println!("cargo:rustc-env={}={}", key, clean);
                }
                _ => {}
            }
        }
    }

    tauri_build::build()
}
