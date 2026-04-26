use std::path::{Path, PathBuf};

/// Get the cross-platform spool directory for ad-hoc backup jobs.
pub fn get_jobs_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".shadow")
        .join("jobs")
}

pub fn remote_key(hostname: &str, local_path: &Path) -> String {
    let path_str = local_path.to_string_lossy().replace('\\', "/");

    // Strip leading slash (Unix) or normalize drive letter (Windows: "C:/..." -> "C/...")
    let normalized = if let Some(stripped) = path_str.strip_prefix('/') {
        stripped.to_string()
    } else if path_str.len() >= 2 && path_str.chars().nth(1) == Some(':') {
        path_str.replacen(':', "", 1)
    } else {
        path_str
    };

    format!("{}/{}", hostname, normalized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn unix_path() {
        let path = PathBuf::from("/Users/john/Documents/report.pdf");
        let key = remote_key("JOHNS-MAC", &path);
        assert_eq!(key, "JOHNS-MAC/Users/john/Documents/report.pdf");
    }

    #[test]
    fn windows_path() {
        let path = PathBuf::from("C:\\Users\\john\\report.pdf");
        let key = remote_key("JOHNS-PC", &path);
        assert_eq!(key, "JOHNS-PC/C/Users/john/report.pdf");
    }

    #[test]
    fn windows_forward_slash_path() {
        let path = PathBuf::from("C:/Users/john/report.pdf");
        let key = remote_key("JOHNS-PC", &path);
        assert_eq!(key, "JOHNS-PC/C/Users/john/report.pdf");
    }

    #[test]
    fn hostname_injected() {
        let path = PathBuf::from("/home/user/file.txt");
        let key = remote_key("my-host", &path);
        assert!(key.starts_with("my-host/"));
    }
}
