use std::path::Path;

/// Returns `true` if the file at `path` should be ignored and never backed up.
///
/// Matches against the filename only (not the full path). Patterns cover:
/// - Editor swap/lock files (vim, LibreOffice)
/// - OS metadata files (macOS, Windows)
/// - Office application temp files (Word, Excel, PowerPoint, etc.)
/// - Generic temp/partial-download files
pub fn should_ignore(path: &Path) -> bool {
    let name = match path.file_name().and_then(|n| n.to_str()) {
        Some(n) => n,
        None => return false,
    };

    // Exact filename matches (case-sensitive; these are well-known fixed names)
    matches!(
        name,
        ".DS_Store"
            | ".AppleDouble"
            | ".LSOverride"
            | "Thumbs.db"
            | "desktop.ini"
            | "ehthumbs.db"
            | "4913" // vim write-permission probe
    ) || is_vim_swap(name)
        || is_office_temp(name)
        || has_temp_extension(name)
        || has_trailing_tilde(name)
}

/// Vim swap files: .filename.swp, .filename.swx, .filename.swo, etc.
fn is_vim_swap(name: &str) -> bool {
    if !name.starts_with('.') {
        return false;
    }
    matches!(
        name.rsplit('.').next().unwrap_or(""),
        "swp" | "swx" | "swo" | "swn" | "swm" | "swa"
    )
}

/// Windows Office lock files: ~$document.docx, ~$sheet.xlsx, ~$pres.pptx, etc.
fn is_office_temp(name: &str) -> bool {
    name.starts_with("~$")
}

/// Generic temp/partial-download extensions.
fn has_temp_extension(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".tmp")
        || lower.ends_with(".temp")
        || lower.ends_with(".part")
        || lower.ends_with(".crdownload") // Chrome partial downloads
        || lower.ends_with(".~lock.")     // LibreOffice lock: .~lock.doc#
        || lower.contains(".~lock.")
}

/// Trailing-tilde backup files produced by many editors (e.g. `file.txt~`).
fn has_trailing_tilde(name: &str) -> bool {
    name.ends_with('~')
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn p(name: &str) -> PathBuf {
        PathBuf::from("/some/dir").join(name)
    }

    // --- should be ignored ---

    #[test]
    fn ignores_vim_swp() {
        assert!(should_ignore(&p(".document.swp")));
        assert!(should_ignore(&p(".notes.swx")));
        assert!(should_ignore(&p(".file.swo")));
    }

    #[test]
    fn ignores_vim_probe() {
        assert!(should_ignore(&p("4913")));
    }

    #[test]
    fn ignores_office_lock() {
        assert!(should_ignore(&p("~$report.docx")));
        assert!(should_ignore(&p("~$budget.xlsx")));
        assert!(should_ignore(&p("~$slides.pptx")));
    }

    #[test]
    fn ignores_tmp_extensions() {
        assert!(should_ignore(&p("cache.tmp")));
        assert!(should_ignore(&p("data.temp")));
        assert!(should_ignore(&p("video.part")));
        assert!(should_ignore(&p("installer.crdownload")));
    }

    #[test]
    fn ignores_libreoffice_lock() {
        assert!(should_ignore(&p(".~lock.document.odt#")));
    }

    #[test]
    fn ignores_trailing_tilde() {
        assert!(should_ignore(&p("file.txt~")));
        assert!(should_ignore(&p("script.py~")));
    }

    #[test]
    fn ignores_macos_metadata() {
        assert!(should_ignore(&p(".DS_Store")));
        assert!(should_ignore(&p(".AppleDouble")));
    }

    #[test]
    fn ignores_windows_metadata() {
        assert!(should_ignore(&p("Thumbs.db")));
        assert!(should_ignore(&p("desktop.ini")));
    }

    // --- should NOT be ignored ---

    #[test]
    fn keeps_normal_files() {
        assert!(!should_ignore(&p("document.docx")));
        assert!(!should_ignore(&p("photo.jpg")));
        assert!(!should_ignore(&p("notes.txt")));
        assert!(!should_ignore(&p("script.py")));
    }

    #[test]
    fn keeps_dotfiles() {
        assert!(!should_ignore(&p(".bashrc")));
        assert!(!should_ignore(&p(".gitconfig")));
    }

    #[test]
    fn keeps_files_with_swp_in_middle() {
        // "swp" only triggers when it's the final extension of a dot-prefixed name
        assert!(!should_ignore(&p("myswp.txt")));
        assert!(!should_ignore(&p("document.swp.bak")));
    }
}
