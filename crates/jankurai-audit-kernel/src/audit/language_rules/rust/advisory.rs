use super::super::catalog::LanguageFinding;
use super::finding;

pub(super) fn advisory_hit_for_line(
    file: &crate::model::FileInfo,
    line_no: usize,
    line: &str,
) -> Option<LanguageFinding> {
    let lower = line.to_ascii_lowercase();
    if lower.contains("clone(") || lower.contains(".clone()") {
        return Some(finding(
            "rust.review.clone-overuse",
            "clone",
            file,
            line_no,
            line,
            "clone may be hiding ownership or performance drift",
            "clone usage is review-worthy",
            "prefer borrowing or move semantics when the copy is not essential",
            "None",
        ));
    }
    if lower.contains("arc<mutex<") || lower.contains("arc<tokio::sync::mutex<") {
        return Some(finding(
            "rust.review.arc-mutex-default",
            "Arc<Mutex<_>>",
            file,
            line_no,
            line,
            "Arc<Mutex<_>> may be a default ownership escape hatch",
            "shared mutable state is review-worthy",
            "consider explicit ownership or a bounded message-passing boundary",
            "None",
        ));
    }
    if lower.contains("rc<refcell<") {
        return Some(finding(
            "rust.review.rc-refcell-default",
            "Rc<RefCell<_>>",
            file,
            line_no,
            line,
            "Rc<RefCell<_>> may be hiding interior mutability",
            "interior mutability is review-worthy",
            "encode the ownership model more directly or document the single-threaded invariant",
            "None",
        ));
    }
    if lower.contains(" as ") {
        return Some(finding(
            "rust.review.as-cast",
            "as cast",
            file,
            line_no,
            line,
            "broad as-casts can hide truncation or type dishonesty",
            "cast proof is review-worthy",
            "use TryFrom, helper newtypes, or a proof-oriented conversion",
            "None",
        ));
    }
    if lower.contains("repr(c)") || lower.contains("repr(c,") {
        return Some(finding(
            "rust.review.repr-c",
            "repr(C)",
            file,
            line_no,
            line,
            "repr(C) is a contract surface that needs review",
            "ABI layout proof is review-worthy",
            "document the ABI and ownership story next to the type definition",
            "None",
        ));
    }
    if lower.contains("pin<")
        || lower.contains("pin::new_unchecked")
        || lower.contains("pin_project")
    {
        return Some(finding(
            "rust.review.pin",
            "Pin",
            file,
            line_no,
            line,
            "Pin usage deserves a locality proof even when it is valid",
            "pinning proof is review-worthy",
            "document the pinning contract and keep the projection code small",
            "None",
        ));
    }
    if lower.contains("atomic") || lower.contains("ordering::relaxed") {
        return Some(finding(
            "rust.review.atomics",
            "atomics",
            file,
            line_no,
            line,
            "atomics need an explicit ordering story",
            "memory-order proof is review-worthy",
            "document the memory-order invariant next to the atomic",
            "None",
        ));
    }
    if lower.contains("unwrap(") || lower.contains("expect(") {
        return Some(finding(
            "rust.review.generic-unwrap",
            "unwrap/expect",
            file,
            line_no,
            line,
            "generic unwrap/expect in non-test Rust code deserves a proof check",
            "panic path is review-worthy",
            "replace the panic path with a typed error or a documented invariant",
            "None",
        ));
    }
    None
}
