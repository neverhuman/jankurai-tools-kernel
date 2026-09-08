mod advisory;
mod catalog;
mod hard;

use super::catalog::LanguageFinding;
use crate::audit::helpers::{product_code_files, AuditContext};
use crate::audit::scan;
use advisory::advisory_hit_for_line;
pub use catalog::catalog;
use hard::{hard_hit_for_line, lint_suppression_hits};
use once_cell::sync::Lazy;
use regex::Regex;

const HLT_RULE_ID: &str = "HLT-029-RUST-BAD-BEHAVIOR";

/// Matches the `static mut` *keyword* (a mutable global — the real HLT-029 hazard)
/// while excluding the `'static` lifetime: `&'static Mutex<…>` and `&'static mut T`
/// (a borrow, not a global) lowercase to `'static mut…`, which a plain substring
/// `contains("static mut")` flags as a false positive. The leading `[^'\w]` rejects a
/// preceding `'` (lifetime) or word char (identifier); the trailing `\b` rejects `mutex`.
static STATIC_MUT_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?:^|[^'\w])static\s+mut\b").expect("static-mut regex"));

#[derive(Debug, Clone, Copy, Default)]
pub struct RustSummary {
    pub hard_findings: usize,
    pub advisory_signals: usize,
}

pub fn summary(ctx: &AuditContext) -> RustSummary {
    RustSummary {
        hard_findings: findings(ctx).len(),
        advisory_signals: advisory_signals(ctx).len(),
    }
}

pub fn findings(ctx: &AuditContext) -> Vec<LanguageFinding> {
    let mut out = hard_findings(ctx);
    out.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then(a.line.unwrap_or(0).cmp(&b.line.unwrap_or(0)))
            .then(a.matched_term.cmp(b.matched_term))
    });
    out
}

pub fn advisory_signals(ctx: &AuditContext) -> Vec<LanguageFinding> {
    let mut out = Vec::new();
    for file in rust_files(ctx) {
        for (idx, line) in file.text.lines().enumerate() {
            if let Some(hit) = advisory_hit_for_line(&file, idx + 1, line) {
                out.push(hit);
            }
        }
    }
    out
}

fn hard_findings(ctx: &AuditContext) -> Vec<LanguageFinding> {
    let mut out = Vec::new();
    for file in rust_files(ctx) {
        for (idx, line) in file.text.lines().enumerate() {
            if let Some(hit) = hard_hit_for_line(&file, idx + 1, line, &file.text) {
                out.push(hit);
            }
        }
    }
    out.extend(lint_suppression_hits(ctx));
    out.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then(a.line.unwrap_or(0).cmp(&b.line.unwrap_or(0)))
            .then(a.matched_term.cmp(b.matched_term))
    });
    out
}

fn rust_files(ctx: &AuditContext) -> Vec<crate::model::FileInfo> {
    let zone_paths = crate::audit::helpers::generated_zone_suppression_paths(ctx);
    product_code_files(ctx)
        .into_iter()
        .filter(|file| {
            let rel = file.rel_path.to_ascii_lowercase();
            file.suffix == ".rs"
                && !scan::is_generated_or_reference_path(&file.rel_path)
                && !scan::is_test_or_example_path(&file.rel_path)
                && !rel.starts_with("crates/jankurai/")
                && !rel.starts_with("crates/jankurai-proofbind/")
                && !rel.starts_with("crates/jankurai-proofmark/")
                && !rel.starts_with("crates/jankurai-audit-kernel/")
                && !rel.starts_with("crates/jankurai-audit-dedup/")
                && !rel.starts_with("crates/jankurai-audit-analyzers/")
                && !rel.starts_with("crates/jankurai-fleet/")
                && !zone_paths
                    .iter()
                    .any(|zone| crate::audit::helpers::path_matches_prefix(&file.rel_path, zone))
        })
        .collect()
}

// Rust rule findings keep detector, source, proof-window, and repair text explicit.
#[allow(clippy::too_many_arguments)]
fn finding(
    matched_term: &'static str,
    detector_id: &'static str,
    file: &crate::model::FileInfo,
    line_no: impl Into<Option<usize>>,
    line: &str,
    problem: &str,
    reason: &str,
    agent_fix: &str,
    proof_window: &'static str,
) -> LanguageFinding {
    let snippet = line.trim().chars().take(160).collect::<String>();
    LanguageFinding::new(
        HLT_RULE_ID,
        matched_term,
        file.rel_path.clone(),
        line_no.into(),
        snippet.clone(),
        problem,
        reason,
        agent_fix,
        vec![
            format!("detector={detector_id}"),
            format!("proof-window={proof_window}"),
            format!("snippet={snippet}"),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::STATIC_MUT_RE;

    // Inputs are pre-lowercased, mirroring `hard_hit_for_line`'s `line.to_ascii_lowercase()`.
    fn flags(line: &str) -> bool {
        STATIC_MUT_RE.is_match(&line.to_ascii_lowercase())
    }

    #[test]
    fn static_mut_keyword_is_flagged() {
        assert!(flags("static mut GLOBAL: usize = 0;"));
        assert!(flags("    pub static mut COUNTER: u32 = 1;"));
        assert!(flags("pub(crate) static mut STATE: State = State::new();"));
    }

    #[test]
    fn static_lifetime_is_not_flagged() {
        // The HLT-029 false positive that forced workarounds (e.g. jekko's BalancerLock alias):
        // `'static mutex` / `'static mut <type>` borrows are not mutable global statics.
        assert!(!flags("    lock: &'static Mutex<Option<KeyBalancer>>,"));
        assert!(!flags("fn get() -> &'static Mutex<State> { &LOCK }"));
        assert!(!flags("let r: &'static mut T = leak(value);")); // a borrow, not a global
        assert!(!flags("type BalancerLock = Mutex<Option<KeyBalancer>>;"));
    }
}
