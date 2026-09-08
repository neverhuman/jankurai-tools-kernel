use super::super::file_kinds::{is_code_file, is_text_candidate, suffix_of};
use crate::model::FileInfo;

/// How a candidate write changes the repo, used to overlay unsaved bytes onto
/// an in-memory inventory so the audit engine can score a file before it lands.
#[derive(Debug, Clone)]
pub enum OverlayOp {
    /// The path does not exist yet and is being created.
    Create,
    /// The path exists and is being overwritten.
    Modify,
    /// The path is being removed.
    Delete,
    /// The path is being renamed from `from` to the overlay's `rel_path`.
    Rename {
        /// Previous repo-relative path (forward-slash normalized).
        from: String,
    },
}

/// A single candidate file change to overlay onto an inventory.
#[derive(Debug, Clone)]
pub struct CandidateOverlay {
    /// Repo-relative path of the candidate (forward-slash normalized).
    pub rel_path: String,
    /// The change being applied.
    pub op: OverlayOp,
    /// Candidate bytes; `None` for [`OverlayOp::Delete`].
    pub bytes: Option<Vec<u8>>,
}

/// Builds a [`FileInfo`] from in-memory candidate bytes without touching disk.
/// Mirrors the classification [`super::file_seed`] applies to on-disk files so overlaid
/// and walked files are scored identically.
pub fn file_info_from_candidate(
    rel_path: &str,
    bytes: &[u8],
    text_capture_chars: usize,
) -> FileInfo {
    let rel_path = rel_path.replace('\\', "/");
    let name = rel_path
        .rsplit('/')
        .next()
        .unwrap_or(rel_path.as_str())
        .to_string();
    let suffix = suffix_of(&rel_path);
    let is_code = is_code_file(&name, &suffix);
    let is_generated = rel_path.split('/').any(|part| {
        part == "generated" || part.starts_with("generated") || part == "gen" || part == "artifacts"
    });
    let is_text = is_text_candidate(&name, &suffix, &rel_path);
    let (text, line_count) = if is_text {
        capture_candidate_text(bytes, text_capture_chars)
    } else {
        (String::new(), 0)
    };
    FileInfo {
        rel_path,
        name,
        suffix,
        size: bytes.len() as u64,
        line_count,
        text,
        is_generated,
        is_code,
    }
}

/// Captures a text sample and line count from candidate bytes, matching the
/// line-counting semantics of [`super::read_text_sample`].
fn capture_candidate_text(bytes: &[u8], max_capture_chars: usize) -> (String, usize) {
    let mut line_count = 0usize;
    let mut captured = String::new();
    for line in bytes.split_inclusive(|&b| b == b'\n') {
        line_count += 1;
        if captured.len() < max_capture_chars {
            let remaining = max_capture_chars - captured.len();
            let piece = if line.len() > remaining {
                &line[..remaining]
            } else {
                line
            };
            captured.push_str(&String::from_utf8_lossy(piece));
        }
    }
    (captured, line_count)
}

/// Applies a [`CandidateOverlay`] to an inventory in place: a create/modify
/// upserts the candidate, a delete removes the path, a rename removes the source
/// path and upserts the destination. The list is re-sorted by `rel_path` so
/// downstream consumers see a stable order.
pub fn apply_overlay(
    files: &mut Vec<FileInfo>,
    overlay: &CandidateOverlay,
    text_capture_chars: usize,
) {
    match &overlay.op {
        OverlayOp::Delete => {
            files.retain(|f| f.rel_path != overlay.rel_path);
        }
        OverlayOp::Rename { from } => {
            files.retain(|f| f.rel_path != *from && f.rel_path != overlay.rel_path);
            if let Some(bytes) = &overlay.bytes {
                files.push(file_info_from_candidate(
                    &overlay.rel_path,
                    bytes,
                    text_capture_chars,
                ));
            }
        }
        OverlayOp::Create | OverlayOp::Modify => {
            files.retain(|f| f.rel_path != overlay.rel_path);
            let bytes = overlay.bytes.as_deref().unwrap_or(&[]);
            files.push(file_info_from_candidate(
                &overlay.rel_path,
                bytes,
                text_capture_chars,
            ));
        }
    }
    files.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
}

#[cfg(test)]
mod overlay_tests {
    use super::*;

    fn sample(rel: &str) -> FileInfo {
        file_info_from_candidate(rel, b"existing\n", 4096)
    }

    #[test]
    fn create_inserts_candidate() {
        let mut files = vec![sample("src/a.rs")];
        let overlay = CandidateOverlay {
            rel_path: "src/b.rs".into(),
            op: OverlayOp::Create,
            bytes: Some(b"fn main() {}\n".to_vec()),
        };
        apply_overlay(&mut files, &overlay, 4096);
        assert_eq!(files.len(), 2);
        let added = files.iter().find(|f| f.rel_path == "src/b.rs").unwrap();
        assert!(added.text.contains("fn main"));
        assert!(added.is_code);
    }

    #[test]
    fn modify_replaces_candidate() {
        let mut files = vec![sample("src/a.rs")];
        let overlay = CandidateOverlay {
            rel_path: "src/a.rs".into(),
            op: OverlayOp::Modify,
            bytes: Some(b"changed\n".to_vec()),
        };
        apply_overlay(&mut files, &overlay, 4096);
        assert_eq!(files.len(), 1);
        assert!(files[0].text.contains("changed"));
    }

    #[test]
    fn delete_removes_candidate() {
        let mut files = vec![sample("src/a.rs"), sample("src/b.rs")];
        let overlay = CandidateOverlay {
            rel_path: "src/a.rs".into(),
            op: OverlayOp::Delete,
            bytes: None,
        };
        apply_overlay(&mut files, &overlay, 4096);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].rel_path, "src/b.rs");
    }

    #[test]
    fn rename_moves_candidate() {
        let mut files = vec![sample("src/source.rs")];
        let overlay = CandidateOverlay {
            rel_path: "src/target.rs".into(),
            op: OverlayOp::Rename {
                from: "src/source.rs".into(),
            },
            bytes: Some(b"renamed\n".to_vec()),
        };
        apply_overlay(&mut files, &overlay, 4096);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].rel_path, "src/target.rs");
        assert!(files[0].text.contains("renamed"));
    }

    #[test]
    fn candidate_line_count_matches_disk_semantics() {
        let info = file_info_from_candidate("src/a.rs", b"a\nb", 4096);
        assert_eq!(info.line_count, 2);
        let info = file_info_from_candidate("src/a.rs", b"a\nb\n", 4096);
        assert_eq!(info.line_count, 2);
    }
}
