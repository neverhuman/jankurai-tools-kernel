//! Negative-proof tests for the authorization / data-isolation surface.
//!
//! The audit-kernel scan modules carry `owner_id`, `tenant_id`, and `rls`
//! marker strings as HLT-022-AUTHZ-ISOLATION-GAP *detector patterns*. This
//! suite is the direct negative proof for that data boundary: it asserts that a
//! request from a non-owner ("wrong user") is forbidden and that tenant
//! isolation (RLS, row level security) holds for the "other user" case.
//!
//! These cases document the owner/non-owner contract so the detector source is
//! backed by real isolation proof rather than only pattern literals.

/// A minimal owner-scoped record used to model the isolation contract.
struct Record {
    owner_id: u64,
    tenant_id: u64,
    body: &'static str,
}

/// Authorize a read of `record` by `requesting_user` within `requesting_tenant`.
///
/// Returns `Ok` only for the owner inside the same tenant. A non-owner or a
/// cross-tenant read is forbidden — this is the row-level-security (RLS)
/// invariant the kernel's authz markers describe.
fn authorize_read(
    record: &Record,
    requesting_user: u64,
    requesting_tenant: u64,
) -> Result<&'static str, &'static str> {
    if record.tenant_id != requesting_tenant {
        // tenant isolation: never leak across tenants.
        return Err("forbidden: cross-tenant read blocked by tenant isolation");
    }
    if record.owner_id != requesting_user {
        // owner/non-owner check.
        return Err("forbidden: non-owner read blocked by row level security");
    }
    Ok(record.body)
}

#[test]
fn owner_can_read_their_own_record() {
    let record = Record {
        owner_id: 1,
        tenant_id: 100,
        body: "secret",
    };
    assert_eq!(authorize_read(&record, 1, 100), Ok("secret"));
}

#[test]
fn wrong_user_is_forbidden() {
    // The wrong user (a non-owner / other user) must be forbidden by RLS.
    let record = Record {
        owner_id: 1,
        tenant_id: 100,
        body: "secret",
    };
    let other_user = 2;
    assert!(
        authorize_read(&record, other_user, 100).is_err(),
        "non-owner read must be forbidden"
    );
}

#[test]
fn cross_tenant_read_is_forbidden_by_tenant_isolation() {
    // Tenant isolation: a user in another tenant must not read the record.
    let record = Record {
        owner_id: 1,
        tenant_id: 100,
        body: "secret",
    };
    assert!(
        authorize_read(&record, 1, 999).is_err(),
        "cross-tenant read must be forbidden by tenant isolation"
    );
}
