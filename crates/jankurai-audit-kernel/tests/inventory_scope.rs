use jankurai_audit_kernel::audit::fs::{
    inventory_paths_detailed, inventory_repo_detailed, InventoryOptions,
};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

const CARGO_HEADER: &str = "Signature: 8a477f597d28d172789f06886806bc55\n# This file is a cache directory tag created by cargo.\n";

fn write(root: &Path, rel: &str, bytes: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(["-c", "core.fsmonitor=false"])
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn repo() -> TempDir {
    let repo = tempfile::tempdir().unwrap();
    git(repo.path(), &["init", "--quiet"]);
    write(repo.path(), "src/main.rs", "fn main() {}\n");
    git(repo.path(), &["add", "src/main.rs"]);
    repo
}

fn cargo_output(root: &Path, rel: &str) -> String {
    write(root, &format!("{rel}/CACHEDIR.TAG"), CARGO_HEADER);
    write(root, &format!("{rel}/.rustc_info.json"), "{}\n");
    std::fs::create_dir_all(root.join(format!("{rel}/debug/.fingerprint"))).unwrap();
    std::fs::create_dir_all(root.join(format!("{rel}/debug/deps"))).unwrap();
    let output = format!("{rel}/debug/build/dependency/out/generated.rs");
    write(root, &output, "pub const BUILD_OUTPUT: u8 = 1;\n");
    output
}

fn assert_scope(root: &Path, options: &InventoryOptions, present: &[&str], absent: &[&str]) {
    let full: BTreeSet<_> = inventory_repo_detailed(root, options)
        .unwrap()
        .files
        .into_iter()
        .map(|file| file.rel_path)
        .collect();
    let selected: Vec<_> = std::fs::read_dir(root)
        .unwrap()
        .filter(|entry| !entry.as_ref().unwrap().file_type().unwrap().is_symlink())
        .map(|entry| entry.unwrap().file_name().to_str().unwrap().to_owned())
        .collect();
    let paths: BTreeSet<_> = inventory_paths_detailed(root, &selected, options)
        .unwrap()
        .files
        .into_iter()
        .map(|file| file.rel_path)
        .collect();
    assert_eq!(
        full, paths,
        "full and selected-directory inventories diverged"
    );
    for (expected, subjects) in [(true, present), (false, absent)] {
        for subject in subjects {
            assert_eq!(full.contains(*subject), expected, "full: {subject}");
            let direct = inventory_paths_detailed(root, &[(*subject).into()], options).unwrap();
            assert_eq!(
                direct.files.iter().any(|file| file.rel_path == *subject),
                expected,
                "direct: {subject}"
            );
        }
    }
}

#[test]
fn authored_directory_names_and_ignored_source_remain_visible() {
    let repo = repo();
    let paths = [
        "src/build/main.rs",
        "ops/build/bundle.ts",
        "src/archive/read.rs",
        "src/backup/restore.rs",
        "src/target/resolve.rs",
        "build/new.rs",
    ];
    for path in paths {
        write(repo.path(), path, "authored source\n");
    }
    write(
        repo.path(),
        ".gitignore",
        "build/\narchive/\nbackup/\ntarget/\n",
    );
    write(repo.path(), ".ignore", "src/\nops/\nbuild/\n");
    git(repo.path(), &["add", "-f", "src", "ops"]);
    assert_scope(repo.path(), &InventoryOptions::default(), &paths, &[]);
}

#[test]
fn witnessed_cargo_outputs_are_excluded_inside_protected_source_and_custom_paths() {
    let repo = repo();
    let nested = cargo_output(repo.path(), "crates/fuzz/target");
    let custom = cargo_output(repo.path(), "custom-output");
    write(repo.path(), "crates/fuzz/src/main.rs", "fn main() {}\n");
    write(
        repo.path(),
        "crates/fuzz/target/src/authored.rs",
        "source\n",
    );
    write(repo.path(), "custom-output/handwritten.rs", "source\n");
    assert_scope(
        repo.path(),
        &InventoryOptions::default(),
        &[
            "src/main.rs",
            "crates/fuzz/src/main.rs",
            "crates/fuzz/target/src/authored.rs",
            "custom-output/handwritten.rs",
        ],
        &[
            &nested,
            &custom,
            "custom-output/CACHEDIR.TAG",
            "custom-output/.rustc_info.json",
        ],
    );
}

#[test]
fn indexed_descendants_override_cache_evidence_without_admitting_output_siblings() {
    let repo = repo();
    let output = cargo_output(repo.path(), "build");
    let source = "build/debug/build/authored.rs";
    write(repo.path(), source, "pub fn authored() {}\n");
    write(repo.path(), "build/handwritten.rs", "source\n");
    git(repo.path(), &["add", "-f", source]);
    assert_scope(
        repo.path(),
        &InventoryOptions::default(),
        &[source, "build/handwritten.rs"],
        &[&output],
    );
}

#[test]
fn incomplete_or_non_cargo_cache_evidence_cannot_hide_source() {
    let repo = repo();
    let marker_only = "target/marker-only/source.rs";
    write(repo.path(), "target/marker-only/CACHEDIR.TAG", CARGO_HEADER);
    write(repo.path(), marker_only, "authored\n");
    let layout_only = cargo_output(repo.path(), "build/layout-only");
    std::fs::remove_file(repo.path().join("build/layout-only/CACHEDIR.TAG")).unwrap();
    let generic = cargo_output(repo.path(), "build/generic-cache");
    write(
        repo.path(),
        "build/generic-cache/CACHEDIR.TAG",
        "Signature: 8a477f597d28d172789f06886806bc55\n# Another cache\n",
    );
    let no_metadata = cargo_output(repo.path(), "build/no-metadata");
    std::fs::remove_file(repo.path().join("build/no-metadata/.rustc_info.json")).unwrap();
    assert_scope(
        repo.path(),
        &InventoryOptions::default(),
        &[marker_only, &layout_only, &generic, &no_metadata],
        &[],
    );
}

#[test]
fn missing_git_index_custody_keeps_potential_source() {
    let repo = tempfile::tempdir().unwrap();
    let output = cargo_output(repo.path(), "build");
    assert_scope(repo.path(), &InventoryOptions::default(), &[&output], &[]);
}

#[test]
fn target_triple_profile_has_the_same_output_evidence() {
    let repo = repo();
    let original = cargo_output(repo.path(), "target");
    std::fs::create_dir_all(repo.path().join("target/x86_64-unknown-linux-musl")).unwrap();
    std::fs::rename(
        repo.path().join("target/debug"),
        repo.path().join("target/x86_64-unknown-linux-musl/debug"),
    )
    .unwrap();
    let output = original.replacen("target/debug", "target/x86_64-unknown-linux-musl/debug", 1);
    assert_scope(
        repo.path(),
        &InventoryOptions::default(),
        &["src/main.rs"],
        &[&output],
    );
}

#[test]
fn explicit_policy_still_excludes_unprotected_paths_and_cannot_hide_protected_source() {
    let repo = repo();
    for path in [
        "scratch/source.rs",
        "crates/source.rs",
        "node_modules/output.js",
        "node_modules/authored.js",
    ] {
        write(repo.path(), path, "source\n");
    }
    git(
        repo.path(),
        &["add", "scratch", "crates", "node_modules/authored.js"],
    );
    let options = InventoryOptions {
        excluded_paths: vec!["scratch".into(), "crates".into()],
        ..InventoryOptions::default()
    };
    assert_scope(
        repo.path(),
        &options,
        &["crates/source.rs", "node_modules/authored.js"],
        &["scratch/source.rs", "node_modules/output.js"],
    );
}

#[cfg(unix)]
#[test]
fn symlink_cache_marker_cannot_hide_source() {
    let repo = repo();
    let output = cargo_output(repo.path(), "build");
    write(repo.path(), "real-marker", CARGO_HEADER);
    std::fs::remove_file(repo.path().join("build/CACHEDIR.TAG")).unwrap();
    std::os::unix::fs::symlink("../real-marker", repo.path().join("build/CACHEDIR.TAG")).unwrap();
    assert_scope(repo.path(), &InventoryOptions::default(), &[&output], &[]);
}

#[cfg(unix)]
#[test]
fn symlink_profile_cannot_supply_output_evidence() {
    let repo = repo();
    cargo_output(repo.path(), "build");
    std::fs::rename(
        repo.path().join("build/debug"),
        repo.path().join("real-profile"),
    )
    .unwrap();
    std::os::unix::fs::symlink("../real-profile", repo.path().join("build/debug")).unwrap();
    write(repo.path(), "build/authored.rs", "source\n");
    assert_scope(
        repo.path(),
        &InventoryOptions::default(),
        &["build/authored.rs"],
        &[],
    );
}

#[cfg(unix)]
#[test]
fn inventory_does_not_execute_repository_fsmonitor() {
    use std::os::unix::fs::PermissionsExt;
    let repo = repo();
    write(
        repo.path(),
        "monitor.sh",
        "#!/bin/sh\nprintf invoked > monitor-invoked\n",
    );
    let script = repo.path().join("monitor.sh");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
    git(
        repo.path(),
        &["config", "core.fsmonitor", script.to_str().unwrap()],
    );
    let output = cargo_output(repo.path(), "crates/demo/target");
    assert_scope(
        repo.path(),
        &InventoryOptions::default(),
        &["src/main.rs"],
        &[&output],
    );
    assert!(!repo.path().join("monitor-invoked").exists());
}

#[cfg(unix)]
#[test]
fn symlink_files_directories_and_ancestors_do_not_admit_outside_bytes() {
    let repo = repo();
    let outside = tempfile::tempdir().unwrap();
    write(outside.path(), "outside.rs", "outside bytes\n");
    std::os::unix::fs::symlink(
        outside.path().join("outside.rs"),
        repo.path().join("link.rs"),
    )
    .unwrap();
    std::os::unix::fs::symlink(outside.path(), repo.path().join("linked-directory")).unwrap();
    assert_scope(
        repo.path(),
        &InventoryOptions::default(),
        &["src/main.rs"],
        &[],
    );
    let full = inventory_repo_detailed(repo.path(), &InventoryOptions::default()).unwrap();
    assert!(!full
        .files
        .iter()
        .any(|file| file.text.contains("outside bytes")));
    for path in ["link.rs", "linked-directory", "linked-directory/outside.rs"] {
        let error =
            inventory_paths_detailed(repo.path(), &[path.into()], &InventoryOptions::default())
                .unwrap_err();
        assert!(error.to_string().contains("symlink inventory path"));
    }
}

#[test]
fn absolute_and_parent_traversal_paths_do_not_admit_outside_bytes() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("repo");
    std::fs::create_dir(&root).unwrap();
    git(&root, &["init", "--quiet"]);
    write(&root, "src/main.rs", "source\n");
    write(parent.path(), "outside.rs", "outside bytes\n");
    for path in [
        "../outside.rs".to_owned(),
        "src/../../outside.rs".into(),
        parent
            .path()
            .join("outside.rs")
            .to_str()
            .unwrap()
            .to_owned(),
    ] {
        let error =
            inventory_paths_detailed(&root, &[path], &InventoryOptions::default()).unwrap_err();
        assert!(error.to_string().contains("unsafe inventory path"));
    }
    assert_scope(&root, &InventoryOptions::default(), &["src/main.rs"], &[]);
}
