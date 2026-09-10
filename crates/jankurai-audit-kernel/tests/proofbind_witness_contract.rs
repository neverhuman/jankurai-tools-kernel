use jankurai_audit_kernel::validation::{validate_value, ArtifactSchema};
use serde_json::{json, Value};
use std::path::Path;

fn witness(surface_type: &str) -> Value {
    json!({
        "schema_version": "1.0.0",
        "standard_version": "0.9.0",
        "generated_at": "2026-09-10T00:00:00Z",
        "repo_root": ".",
        "git_head": "1111111111111111111111111111111111111111",
        "mode": "required",
        "changed_paths": ["tests/integration.rs"],
        "surfaces": [{
            "surface_id": "fixture-surface",
            "path": "tests/integration.rs",
            "symbol": "exercises_boundary",
            "surface_type": surface_type,
            "severity": "high",
            "risk_tags": ["changed_behavior", "typed_test_execution"],
            "owner": "tools",
            "owner_route": "tests/",
            "test_route": "tests/",
            "proof_lane": "integration",
            "required_rules": ["HLT-008-FALSE-GREEN-RISK"],
            "required_lanes": ["integration"],
            "repair_tasks": ["run the declared test lane"]
        }],
        "summary": {
            "changed_surface_count": 1,
            "high_or_critical_surface_count": 1,
            "by_surface_type": { surface_type: 1 },
            "by_owner": { "tools": 1 },
            "verdict": "block"
        }
    })
}

#[test]
fn embedded_witness_contract_accepts_test_execution_and_existing_surface_types() {
    for surface_type in [
        "test_execution",
        "rust_public_api",
        "authz_boundary",
        "input_boundary",
        "sql_query",
        "db_migration",
        "cli_command",
        "mcp_tool",
        "unsafe_or_process_sink",
        "business_invariant",
    ] {
        let value = witness(surface_type);
        validate_value(Path::new("."), ArtifactSchema::ProofBindWitness, &value).unwrap();
    }
}

#[test]
fn embedded_witness_contract_still_rejects_unknown_and_incomplete_surfaces() {
    for surface_type in ["", "test", "future_unknown_surface"] {
        assert!(validate_value(
            Path::new("."),
            ArtifactSchema::ProofBindWitness,
            &witness(surface_type),
        )
        .is_err());
    }
    let mut value = witness("test_execution");
    value["surfaces"][0].as_object_mut().unwrap().remove("required_lanes");
    assert!(validate_value(Path::new("."), ArtifactSchema::ProofBindWitness, &value).is_err());
}
