//! Filename and path helpers.

use std::path::{Path, PathBuf};

/// Replace characters that are illegal in Windows/macOS file and folder names.
pub fn sanitize_name(name: &str) -> String {
    let mut cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => ' ',
            c if (c as u32) < 32 => ' ',
            c => c,
        })
        .collect();

    // Windows rejects names ending in a dot or a space.
    while cleaned.ends_with('.') || cleaned.ends_with(' ') {
        cleaned.pop();
    }

    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "untitled".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Reject paths that try to escape their parent (`..`, absolute roots in
/// unexpected places). Used before trusting a path that came from the UI.
pub fn is_plausible_dir(path: &str) -> bool {
    let candidate = Path::new(path);
    if path.trim().is_empty() {
        return false;
    }
    candidate.is_absolute() && !candidate.components().any(|c| matches!(c, std::path::Component::ParentDir))
}

/// Join `name` onto `root` and make sure the result stays inside `root`.
pub fn safe_join(root: &Path, name: &str) -> Option<PathBuf> {
    let sanitized = sanitize_name(name);
    let joined = root.join(sanitized);
    if joined.starts_with(root) {
        Some(joined)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_illegal_characters() {
        assert_eq!(sanitize_name("a/b:c*?\"<>|"), "a b c");
        assert_eq!(sanitize_name("name.."), "name");
        assert_eq!(sanitize_name("   "), "untitled");
    }

    #[test]
    fn rejects_parent_traversal() {
        assert!(!is_plausible_dir("../etc"));
        assert!(!is_plausible_dir("relative/videos"));
        assert!(!is_plausible_dir(""));

        let absolute = if cfg!(windows) {
            r"C:\Users\someone\Videos"
        } else {
            "/home/someone/videos"
        };
        assert!(is_plausible_dir(absolute));
    }

    #[test]
    fn safe_join_stays_inside_root() {
        let root = Path::new("base");
        assert_eq!(safe_join(root, "sub"), Some(root.join("sub")));

        // Slashes are neutralised by sanitize_name, so a traversal attempt
        // collapses into a single path segment and cannot escape the root.
        let escaped = safe_join(root, "../../etc").expect("still confined");
        assert!(escaped.starts_with(root));
        assert_eq!(escaped.components().count(), root.components().count() + 1);
    }
}
