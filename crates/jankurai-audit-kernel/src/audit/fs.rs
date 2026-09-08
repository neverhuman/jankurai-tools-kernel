use crate::model::FileInfo;
use anyhow::Result;
use ignore::WalkBuilder;
use rayon::prelude::*;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

mod exclusions;
mod overlay;

use exclusions::InventoryFilter;
pub use overlay::{apply_overlay, file_info_from_candidate, CandidateOverlay, OverlayOp};

use super::file_kinds::{is_code_file, is_text_candidate, suffix_of};
pub use super::fs_policy::{InventoryOptions, InventoryResult, InventoryTimings};

const PROTECTED_DIR_PREFIXES: &[&str] = &[".github/", "agent/", "crates/", "tools/"];
const PROTECTED_ROOT_FILES: &[&str] = &[
    "AGENTS.md",
    "CHANGELOG.md",
    "CODEOWNERS",
    "CODE_OF_CONDUCT.md",
    "CONTRIBUTING.md",
    "Cargo.lock",
    "Cargo.toml",
    "Justfile",
    "README.md",
    "SECURITY.md",
    "SUPPORT.md",
    "VERSION",
    "package-lock.json",
    "package.json",
];
const READ_ONLY_EXCEPTION_DIR: &str = "docs/exceptions";

pub fn inventory_repo(root: &Path) -> Result<Vec<FileInfo>> {
    Ok(inventory_repo_detailed(root, &InventoryOptions::from_policy(root))?.files)
}

pub fn inventory_repo_for_paths(root: &Path, paths: &[String]) -> Result<Vec<FileInfo>> {
    let options = InventoryOptions::from_policy(root);
    Ok(inventory_paths_detailed(root, paths, &options)?.files)
}

pub fn inventory_repo_detailed(root: &Path, options: &InventoryOptions) -> Result<InventoryResult> {
    let walk_started = Instant::now();
    let mut paths: Vec<PathBuf> = Vec::new();
    let filter_root = root.to_path_buf();
    let filter = InventoryFilter::new(root, options);
    let walk_filter = filter.clone();
    let mut builder = WalkBuilder::new(root);
    builder
        .hidden(false)
        .ignore(false)
        .git_ignore(false)
        .git_exclude(false)
        .git_global(false)
        .max_depth(None);
    for entry in builder
        .filter_entry(move |entry| {
            let Ok(rel) = entry.path().strip_prefix(&filter_root) else {
                return true;
            };
            rel.as_os_str().is_empty() || !walk_filter.should_skip(rel)
        })
        .build()
    {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        let path = entry.path();
        if !entry.file_type().is_some_and(|kind| kind.is_file()) {
            continue;
        }
        let rel = match path.strip_prefix(root) {
            Ok(rel) => rel,
            Err(_) => continue,
        };
        if filter.should_skip(rel) {
            continue;
        }
        paths.push(rel.to_path_buf());
    }
    paths.sort();
    paths.dedup();
    let walk = walk_started.elapsed();

    inventory_from_paths(root, paths, options, walk)
}

pub fn inventory_paths_detailed(
    root: &Path,
    paths: &[String],
    options: &InventoryOptions,
) -> Result<InventoryResult> {
    let walk_started = Instant::now();
    let mut collected = Vec::new();
    let filter = InventoryFilter::new(root, options);
    for rel in paths {
        let rel = rel.trim().trim_start_matches("./");
        if rel.is_empty() {
            continue;
        }
        let rel_path = PathBuf::from(rel);
        filter.validate_selected_path(&rel_path)?;
        if filter.should_skip(&rel_path) {
            continue;
        }
        let abs = root.join(&rel_path);
        if abs.is_file() {
            collected.push(rel_path);
        } else if abs.is_dir() {
            let filter_root = root.to_path_buf();
            let walk_filter = filter.clone();
            let mut builder = WalkBuilder::new(&abs);
            builder
                .hidden(false)
                .ignore(false)
                .git_ignore(false)
                .git_exclude(false)
                .git_global(false);
            for entry in builder
                .filter_entry(move |entry| {
                    let Ok(rel) = entry.path().strip_prefix(&filter_root) else {
                        return true;
                    };
                    rel.as_os_str().is_empty() || !walk_filter.should_skip(rel)
                })
                .build()
            {
                let Ok(entry) = entry else {
                    continue;
                };
                let path = entry.path();
                if !entry.file_type().is_some_and(|kind| kind.is_file()) {
                    continue;
                }
                let Ok(rel) = path.strip_prefix(root) else {
                    continue;
                };
                if filter.should_skip(rel) {
                    continue;
                }
                collected.push(rel.to_path_buf());
            }
        }
    }
    collected.sort();
    collected.dedup();
    let walk = walk_started.elapsed();
    inventory_from_paths(root, collected, options, walk)
}

fn inventory_from_paths(
    root: &Path,
    paths: Vec<PathBuf>,
    options: &InventoryOptions,
    walk: Duration,
) -> Result<InventoryResult> {
    let metadata_started = Instant::now();
    let mut seeds: Vec<FileSeed> = paths
        .par_iter()
        .filter_map(|rel| file_seed(root, rel))
        .collect();
    seeds.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    let metadata = metadata_started.elapsed();

    let text_started = Instant::now();
    let mut files: Vec<FileInfo> = seeds
        .into_par_iter()
        .map(|seed| {
            let (text, line_count) = if seed.is_text {
                read_text_sample(&root.join(&seed.rel), options.text_capture_chars)
                    .unwrap_or_default()
            } else {
                (String::new(), 0)
            };
            FileInfo {
                rel_path: seed.rel_path,
                name: seed.name,
                suffix: seed.suffix,
                size: seed.size,
                line_count,
                text,
                is_generated: seed.is_generated,
                is_code: seed.is_code,
            }
        })
        .collect();
    files.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    let text_capture = text_started.elapsed();

    Ok(InventoryResult {
        files,
        timings: InventoryTimings {
            walk_ms: walk.as_millis(),
            metadata_ms: metadata.as_millis(),
            text_capture_ms: text_capture.as_millis(),
        },
    })
}

fn file_seed(root: &Path, rel: &Path) -> Option<FileSeed> {
    let abs = root.join(rel);
    let meta = std::fs::symlink_metadata(&abs).ok()?;
    if !meta.is_file() {
        return None;
    }
    let rel_path = rel.to_string_lossy().replace('\\', "/");
    let name = abs
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    let suffix = suffix_of(&rel_path);
    let is_code = is_code_file(&name, &suffix);
    let is_generated = rel_path.split('/').any(|part| {
        part == "generated" || part.starts_with("generated") || part == "gen" || part == "artifacts"
    });
    let is_text = is_text_candidate(&name, &suffix, &rel_path);
    Some(FileSeed {
        rel: rel.to_path_buf(),
        rel_path,
        name,
        suffix,
        size: meta.len(),
        is_text,
        is_generated,
        is_code,
    })
}

fn normalize_repo_path(path: &str) -> String {
    path.trim()
        .trim_start_matches("./")
        .replace('\\', "/")
        .trim_matches('/')
        .to_string()
}

pub fn is_inventory_protected_path(path: &str) -> bool {
    let rel = normalize_repo_path(path);
    PROTECTED_ROOT_FILES.contains(&rel.as_str())
        || PROTECTED_DIR_PREFIXES.iter().any(|prefix| {
            let trimmed = prefix.trim_end_matches('/');
            rel == trimmed || rel.starts_with(prefix)
        })
}

pub fn is_read_only_exception_path(path: &str) -> bool {
    let rel = normalize_repo_path(path);
    rel == READ_ONLY_EXCEPTION_DIR
        || rel.starts_with(&format!("{READ_ONLY_EXCEPTION_DIR}/"))
        || rel.contains(&format!("/{READ_ONLY_EXCEPTION_DIR}/"))
        || rel.ends_with(&format!("/{READ_ONLY_EXCEPTION_DIR}"))
}

pub fn is_generated_zone_protected_path(path: &str) -> bool {
    let rel = normalize_repo_path(path);
    if rel == ".jankurai/repo-score.json"
        || rel == ".jankurai/repo-score.md"
        || rel == "agent/repo-score.json"
        || rel == "agent/repo-score.md"
        || rel.starts_with("agent/baselines/")
        // jankurai's own badge output (declared in generated-zones.toml), same class as repo-score.*
        || (rel.starts_with("agent/jankurai-badge")
            && (rel.ends_with(".svg") || rel.ends_with(".json") || rel.ends_with(".md")))
        || rel == "Cargo.lock"
        || rel == "package-lock.json"
        || rel == "pnpm-lock.yaml"
        || rel == "yarn.lock"
    {
        return false;
    }
    is_inventory_protected_path(&rel)
}

fn read_text_sample(path: &Path, max_capture_chars: usize) -> Result<(String, usize)> {
    let file = std::fs::File::open(path)?;
    let mut reader = std::io::BufReader::new(file);
    let mut line_count = 0usize;
    let mut captured = String::new();
    let mut line = Vec::new();
    loop {
        line.clear();
        let read = reader.read_until(b'\n', &mut line)?;
        if read == 0 {
            break;
        }
        line_count += 1;
        if captured.len() < max_capture_chars {
            let remaining = max_capture_chars - captured.len();
            let mut piece = line.as_slice();
            if piece.len() > remaining {
                piece = &piece[..remaining];
            }
            captured.push_str(&String::from_utf8_lossy(piece));
        }
    }
    Ok((captured, line_count))
}

struct FileSeed {
    rel: PathBuf,
    rel_path: String,
    name: String,
    suffix: String,
    size: u64,
    is_text: bool,
    is_generated: bool,
    is_code: bool,
}
