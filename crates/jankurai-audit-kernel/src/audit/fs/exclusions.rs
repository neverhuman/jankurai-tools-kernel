use super::{is_inventory_protected_path, InventoryOptions};
use anyhow::{bail, Context, Result};
use std::collections::{BTreeSet, HashMap};
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

const EXCLUDED_DIRS: &[&str] = &[
    ".idea",
    ".mypy_cache",
    ".pytest_cache",
    ".ruff_cache",
    ".tox",
    ".venv",
    "__pycache__",
    "coverage",
    "dist",
    "node_modules",
    "vendor",
    "venv",
    ".witness",
];
const CARGO_CACHE_HEADER: &[u8] = b"Signature: 8a477f597d28d172789f06886806bc55\n# This file is a cache directory tag created by cargo.\n";

/// A single inventory uses one index snapshot. Cache evidence never hides an
/// indexed file or prevents walking an ancestor of an indexed file.
#[derive(Clone)]
pub(super) struct InventoryFilter {
    root: PathBuf,
    options: InventoryOptions,
    tracked: Option<Arc<BTreeSet<PathBuf>>>,
    cargo_outputs: Arc<Mutex<HashMap<PathBuf, Vec<PathBuf>>>>,
}

impl InventoryFilter {
    pub(super) fn new(root: &Path, options: &InventoryOptions) -> Self {
        Self {
            root: root.to_path_buf(),
            options: options.clone(),
            tracked: tracked_paths_and_parents(root).map(Arc::new),
            cargo_outputs: Arc::default(),
        }
    }

    pub(super) fn validate_selected_path(&self, path: &Path) -> Result<()> {
        let mut prefix = self.root.clone();
        for component in path.components() {
            let Component::Normal(name) = component else {
                bail!("unsafe inventory path: {}", path.display());
            };
            prefix.push(name);
            match std::fs::symlink_metadata(&prefix) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    bail!("symlink inventory path: {}", path.display());
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error).context("inspect inventory path"),
            }
        }
        Ok(())
    }

    pub(super) fn should_skip(&self, path: &Path) -> bool {
        if path
            .components()
            .any(|part| matches!(part.as_os_str().to_str(), Some(".git" | ".hg" | ".svn")))
        {
            return true;
        }
        let indexed = self
            .tracked
            .as_ref()
            .is_some_and(|paths| paths.contains(path));
        // A missing or unreadable index cannot establish that output is unowned.
        if !indexed && self.tracked.is_some() && self.in_cargo_output(path) {
            return true;
        }
        let rel = path.to_string_lossy().replace('\\', "/");
        if is_inventory_protected_path(&rel) {
            return false;
        }
        if rel.starts_with(".cursor/")
            && rel != ".cursor/rules"
            && !rel.starts_with(".cursor/rules/")
        {
            return true;
        }
        if self.options.excluded_paths.iter().any(|excluded| {
            rel == *excluded || rel.starts_with(&format!("{}/", excluded.trim_end_matches('/')))
        }) || self
            .options
            .extra_excluded_globs
            .as_ref()
            .is_some_and(|set| set.is_match(&rel))
        {
            return true;
        }
        if [".antigravity", "antigravity"]
            .iter()
            .any(|dir| rel == *dir || rel.starts_with(&format!("{dir}/")))
        {
            return true;
        }
        !indexed
            && path
                .components()
                .any(|part| EXCLUDED_DIRS.contains(&part.as_os_str().to_string_lossy().as_ref()))
    }

    fn in_cargo_output(&self, path: &Path) -> bool {
        path.ancestors()
            .filter(|path| !path.as_os_str().is_empty())
            .any(|candidate| {
                let mut cache = self
                    .cargo_outputs
                    .lock()
                    .unwrap_or_else(|error| error.into_inner());
                cache
                    .entry(candidate.to_path_buf())
                    .or_insert_with(|| cargo_output_paths(&self.root.join(candidate)))
                    .iter()
                    .any(|output| {
                        path.strip_prefix(candidate)
                            .is_ok_and(|rel| rel.starts_with(output))
                    })
            })
    }
}

fn tracked_paths_and_parents(root: &Path) -> Option<BTreeSet<PathBuf>> {
    let mut command = Command::new("git");
    // Do not inherit alternate index/repository/config selections or allow the
    // repository's fsmonitor setting to execute a hook during this read.
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("GIT_") {
            command.env_remove(name);
        }
    }
    let output = command
        .args([
            "--no-optional-locks",
            "-c",
            "core.fsmonitor=false",
            "ls-files",
            "--cached",
            "-z",
            "--",
        ])
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .current_dir(root)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let mut paths = BTreeSet::new();
    for bytes in output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|bytes| !bytes.is_empty())
    {
        // Non-UTF8 names are conservatively kept by disabling output pruning.
        let path = Path::new(std::str::from_utf8(bytes).ok()?);
        paths.extend(path.ancestors().map(Path::to_path_buf));
    }
    Some(paths)
}

fn cargo_output_paths(path: &Path) -> Vec<PathBuf> {
    let marker = path.join("CACHEDIR.TAG");
    if !regular(path, true)
        || !regular(&marker, false)
        || !regular(&path.join(".rustc_info.json"), false)
    {
        return Vec::new();
    }
    let mut header = [0; CARGO_CACHE_HEADER.len()];
    if std::fs::File::open(marker)
        .and_then(|mut file| file.read_exact(&mut header))
        .is_err()
        || header.as_slice() != CARGO_CACHE_HEADER
    {
        return Vec::new();
    }
    // A basename or marker alone is insufficient. Cargo creates both of these
    // directories in a profile's output layout, including target triples.
    let profile = |path: &Path| {
        regular(path, true)
            && regular(&path.join("deps"), true)
            && regular(&path.join(".fingerprint"), true)
    };
    let mut outputs = Vec::new();
    for name in ["debug", "release"] {
        if profile(&path.join(name)) {
            outputs.push(PathBuf::from(name));
        }
    }
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.filter_map(Result::ok) {
            if !regular(&entry.path(), true) {
                continue;
            }
            for name in ["debug", "release"] {
                if profile(&entry.path().join(name)) {
                    outputs.push(PathBuf::from(entry.file_name()).join(name));
                }
            }
        }
    }
    // Retain the container and unrelated authored siblings. Evidence covers
    // only these profile trees and the two Cargo metadata files.
    if !outputs.is_empty() {
        outputs.extend([
            PathBuf::from("CACHEDIR.TAG"),
            PathBuf::from(".rustc_info.json"),
        ]);
    }
    outputs
}

fn regular(path: &Path, directory: bool) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|metadata| {
        if directory {
            metadata.is_dir()
        } else {
            metadata.is_file()
        }
    })
}
