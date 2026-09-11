use jankurai_audit_kernel::audit::{fs, helpers, scan};
use std::fs as disk;

fn inventory(path: &str, source: &str) -> jankurai_audit_kernel::model::FileInfo {
    let root = tempfile::tempdir().unwrap();
    let full = root.path().join(path);
    disk::create_dir_all(full.parent().unwrap()).unwrap();
    disk::write(&full, source).unwrap();
    let files = fs::inventory_repo(root.path()).unwrap();
    assert_eq!(files.len(), 1);
    let from_disk = files.into_iter().next().unwrap();
    let from_overlay = fs::file_info_from_candidate(path, source.as_bytes(), 4096);
    assert_eq!(from_disk.is_generated, from_overlay.is_generated);
    assert_eq!(from_disk.is_code, from_overlay.is_code);
    assert_eq!(from_disk.text, source);
    assert_eq!(from_overlay.text, source);
    from_disk
}

#[test]
fn authored_names_do_not_become_generated_zones() {
    for path in [
        "crates/kernel/tests/generated_zone_pairs.rs",
        "src/generated_policy.rs",
        "generated_parser/src/lib.rs",
        "generatedness/a.ts",
        "src/artifacts_manager.rs",
    ] {
        let file = inventory(path, "// TODO: adversarial fixture\n");
        assert!(!file.is_generated, "{path}");
        assert!(file.is_code, "{path}");
        let ctx = context(file);
        assert!(scan::generated_zone_issues(&ctx).is_empty(), "{path}");
    }
}

#[test]
fn conventional_output_directories_keep_generated_governance_checks() {
    for path in ["generated/client.ts", "sdk/gen/a.rs", "sdk/artifacts/a.js"] {
        let file = inventory(path, "// TODO: adversarial fixture\n");
        assert!(file.is_generated, "{path}");
        let hits = scan::generated_zone_issues(&context(file));
        assert!(hits
            .iter()
            .any(|hit| hit.problem.contains("ownership rules")));
        assert!(hits.iter().any(|hit| hit.problem.contains("TODO/stub")));
    }
}

#[test]
fn javascript_and_typescript_module_variants_are_complete_code_inputs() {
    for path in [
        "vite.config.mts",
        "vite.config.cts",
        "src/a.mjs",
        "src/a.cjs",
    ] {
        let file = inventory(path, "export default { server: { host: true } };\n");
        assert!(file.is_code, "{path}");
        assert!(!file.is_generated, "{path}");
    }
}

fn context(file: jankurai_audit_kernel::model::FileInfo) -> helpers::AuditContext {
    helpers::AuditContext {
        root: "/unused".into(),
        scope_files: vec![file.clone()],
        all_files: vec![file],
        scope_paths: vec![],
        self_audit: false,
        boundary_reclassifications: vec![],
        copy_code: None,
    }
}
