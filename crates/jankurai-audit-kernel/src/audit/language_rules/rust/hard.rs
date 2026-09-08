use super::super::catalog::LanguageFinding;
use super::{finding, rust_files, STATIC_MUT_RE};
use crate::audit::{helpers::AuditContext, scan};

pub(super) fn hard_hit_for_line(
    file: &crate::model::FileInfo,
    line_no: usize,
    line: &str,
    full_text: &str,
) -> Option<LanguageFinding> {
    let lower = line.to_ascii_lowercase();

    if (lower.contains("pub unsafe fn")
        || lower.contains("pub(crate) unsafe fn")
        || lower.contains("pub(super) unsafe fn"))
        && !scan::public_unsafe_has_safety_docs(full_text, line_no)
    {
        return Some(finding(
            "rust.unsafe.public-fn-missing-safety-doc",
            "pub unsafe fn",
            file,
            line_no,
            line,
            "public unsafe API lacks a `# Safety` contract",
            "missing `# Safety` docs above the public unsafe item",
            "document caller obligations with a `# Safety` section",
            "NearbySafetyDocs",
        ));
    }

    if lower.contains("unsafe impl send") || lower.contains("unsafe impl sync") {
        return Some(finding(
            "rust.unsafe.impl-send-sync",
            "unsafe impl",
            file,
            line_no,
            line,
            "unsafe Send/Sync impl needs a thread-safety proof",
            "thread-safety proof is missing",
            "remove the unsafe impl or document and prove the synchronization invariant",
            "NearbySafetyDocs",
        ));
    }

    if (lower.contains("unsafe {") || lower.contains("unsafe{"))
        && !scan::line_has_nearby_safety_comment(full_text, line_no)
        && !scan::public_unsafe_has_safety_docs(full_text, line_no)
    {
        return Some(finding(
            "rust.unsafe.undocumented-block",
            "unsafe {",
            file,
            line_no,
            line,
            "unsafe block lacks a nearby local safety argument",
            "no nearby SAFETY comment was found",
            "add a precise `SAFETY:` comment or remove the unsafe block",
            "NearbySafetyComment",
        ));
    }

    if lower.contains("transmute(") {
        return Some(finding(
            "rust.unsafe.transmute",
            "transmute",
            file,
            line_no,
            line,
            "transmute hides layout, validity, or lifetime assumptions",
            "unsafe transmute appears without a local proof",
            "replace transmute with an explicit conversion that proves layout and validity",
            "NearbySafetyComment",
        ));
    }
    if lower.contains("assume_init(") {
        return Some(finding(
            "rust.unsafe.assume-init",
            "assume_init",
            file,
            line_no,
            line,
            "assume_init can read uninitialized memory",
            "MaybeUninit proof is missing",
            "initialize every field before converting from MaybeUninit",
            "NearbySafetyComment",
        ));
    }
    // Match `mem::zeroed(` specifically (the unsafe fabricate-invalid-value hazard the finding
    // text names), not the bare `zeroed(` substring, which also hit safe wrappers like
    // `BytesMut::zeroed(` / `MaybeUninit::zeroed(`. `mem::zeroed(` covers std::/core:: forms.
    if lower.contains("mem::zeroed(") {
        return Some(finding(
            "rust.unsafe.zeroed",
            "zeroed",
            file,
            line_no,
            line,
            "mem::zeroed can fabricate invalid values",
            "all-zero validity was not proven",
            "construct the type with a valid initializer instead of zeroing it",
            "NearbySafetyComment",
        ));
    }
    if lower.contains("get_unchecked(") {
        return Some(finding(
            "rust.unsafe.get-unchecked",
            "get_unchecked",
            file,
            line_no,
            line,
            "unchecked indexing needs a mechanical bounds proof",
            "bounds proof is missing",
            "replace unchecked access with a checked access path or prove the bounds locally",
            "NearbySafetyComment",
        ));
    }
    if lower.contains("unwrap_unchecked(") {
        return Some(finding(
            "rust.unsafe.unwrap-unchecked",
            "unwrap_unchecked",
            file,
            line_no,
            line,
            "unwrap_unchecked bypasses the option/result proof",
            "presence proof is missing",
            "use a checked path or add a local proof that the value is always present",
            "NearbySafetyComment",
        ));
    }
    if lower.contains("unreachable_unchecked(") {
        return Some(finding(
            "rust.unsafe.unreachable-unchecked",
            "unreachable_unchecked",
            file,
            line_no,
            line,
            "unreachable_unchecked assumes a proof that reviewers cannot infer",
            "control-flow proof is missing",
            "replace it with a checked branch or a documented invariant",
            "NearbySafetyComment",
        ));
    }
    if lower.contains("from_utf8_unchecked(") {
        return Some(finding(
            "rust.unsafe.from-utf8-unchecked",
            "from_utf8_unchecked",
            file,
            line_no,
            line,
            "unchecked UTF-8 conversion can fabricate invalid text",
            "UTF-8 validity is missing",
            "validate the bytes or keep the value as raw bytes",
            "NearbySafetyComment",
        ));
    }
    if lower.contains("box::from_raw(") {
        return Some(finding(
            "rust.unsafe.raw-parts",
            "Box::from_raw",
            file,
            line_no,
            line,
            "raw ownership conversion needs exact provenance and allocator proof",
            "ownership provenance is missing",
            "use the matching constructor/destructor pair or add a documented ownership proof",
            "NearbySafetyComment",
        ));
    }
    if lower.contains("vec::from_raw_parts(") {
        return Some(finding(
            "rust.unsafe.raw-parts",
            "Vec::from_raw_parts",
            file,
            line_no,
            line,
            "raw ownership conversion needs exact provenance and allocator proof",
            "ownership provenance is missing",
            "use the matching constructor/destructor pair or add a documented ownership proof",
            "NearbySafetyComment",
        ));
    }
    if lower.contains("cstring::from_raw(") {
        return Some(finding(
            "rust.unsafe.raw-parts",
            "CString::from_raw",
            file,
            line_no,
            line,
            "raw ownership conversion needs exact provenance and allocator proof",
            "ownership provenance is missing",
            "use the matching constructor/destructor pair or add a documented ownership proof",
            "NearbySafetyComment",
        ));
    }
    if lower.contains("from_raw_parts(") {
        return Some(finding(
            "rust.unsafe.raw-parts",
            "from_raw_parts",
            file,
            line_no,
            line,
            "raw ownership conversion needs exact provenance and allocator proof",
            "ownership provenance is missing",
            "use the matching constructor/destructor pair or add a documented ownership proof",
            "NearbySafetyComment",
        ));
    }
    if STATIC_MUT_RE.is_match(&lower) {
        return Some(finding(
            "rust.unsafe.static-mut",
            "static mut",
            file,
            line_no,
            line,
            "static mut exposes unsynchronized global mutation",
            "global mutation proof is missing",
            "replace the mutable static with explicit synchronization or scoped ownership",
            "NearbySafetyComment",
        ));
    }
    if lower.contains("repr(packed)") {
        return Some(finding(
            "rust.unsafe.repr-packed",
            "repr(packed)",
            file,
            line_no,
            line,
            "repr(packed) makes alignment-sensitive references dangerous",
            "alignment proof is missing",
            "remove packed layout or access fields through unaligned-safe primitives",
            "NearbySafetyComment",
        ));
    }
    if shell_command_is_dynamic(&lower) && !scan::is_fixed_safe_command_invocation(line) {
        return Some(finding(
            "rust.security.shell-c-dynamic",
            "shell execution",
            file,
            line_no,
            line,
            "shell execution with dynamic command text is unsafe by default",
            "shell command text is not proven safe",
            "pass argv values directly or use a bounded, allowlisted command path",
            "NearbyAsyncContext",
        ));
    }
    if lower.contains("unbounded_channel(") || lower.contains("unbounded(") {
        return Some(finding(
            "rust.async.unbounded-channel",
            "unbounded_channel",
            file,
            line_no,
            line,
            "unbounded channel or task creation needs explicit backpressure proof",
            "backpressure proof is missing",
            "use a bounded channel or document the queue bound and shutdown path",
            "NearbyAsyncContext",
        ));
    }
    if lower.contains("block_on(") && scan::function_context_contains_async(full_text, line_no) {
        return Some(finding(
            "rust.async.block_on-in-async",
            "block_on",
            file,
            line_no,
            line,
            "blocking inside async context can stall the executor",
            "async context proof is missing",
            "move the work out of async context or use an async-native path",
            "NearbyAsyncContext",
        ));
    }

    None
}

pub(super) fn lint_suppression_hits(ctx: &AuditContext) -> Vec<LanguageFinding> {
    let mut out = Vec::new();
    for file in rust_files(ctx) {
        let lower = file.text.to_ascii_lowercase();
        let mut matched_term = None;
        if lower.contains("#![allow(warnings)]")
            || lower.contains("#[allow(warnings)]")
            || lower.contains("allow(warnings)")
        {
            matched_term = Some("rust.supply.allow-warnings");
        } else if lower.contains("allow(clippy::all)") {
            matched_term = Some("rust.supply.allow-clippy-all");
        } else if lower.contains("allow(clippy::correctness)") {
            matched_term = Some("rust.supply.allow-clippy-correctness");
        } else if lower.contains("rustflags") && lower.contains("-a warnings") {
            matched_term = Some("rust.supply.rustflags-warn-suppression");
        }
        if let Some(term) = matched_term {
            let line = file
                .text
                .lines()
                .position(|candidate| {
                    candidate
                        .to_ascii_lowercase()
                        .contains(term.split('.').next_back().unwrap_or(term))
                })
                .map(|idx| idx + 1);
            let text = line
                .and_then(|line_no| {
                    file.text
                        .lines()
                        .nth(line_no - 1)
                        .map(|l| l.trim().to_string())
                })
                .unwrap_or_else(|| file.text.lines().next().unwrap_or("").trim().to_string());
            out.push(finding(
                "rust.supply.lint-suppression",
                term,
                &file,
                line,
                &text,
                "broad lint suppression hides review-relevant issues",
                "lint suppression proof is missing",
                "tighten the lint scope or remove the allow entirely",
                "NearbySafetyDocs",
            ));
            break;
        }
    }
    out
}

fn shell_command_is_dynamic(lower: &str) -> bool {
    (lower.contains("command::new(\"sh\"")
        || lower.contains("command::new(\"bash\"")
        || lower.contains("command::new(\"/bin/sh\"")
        || lower.contains("command::new(\"/bin/bash\""))
        && lower.contains(".arg(\"-c\")")
        && (lower.contains("user_input")
            || lower.contains("command_text")
            || lower.contains("shell_command")
            || lower.contains("format!(")
            || lower.contains("script")
            || lower.contains("cmd"))
}
