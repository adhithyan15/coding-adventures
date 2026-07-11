//! # wolfram-to-semantic-ir
//!
//! Wolfram CST → narrow-waist Semantic IR, **v0.1.0**.
//!
//! This is the first frontend to target
//! [SIR23](../../../specs/SIR23-symbolic-pattern-semantic-ir.md), the
//! symbolic-expression/pattern-matching domain extension of the SIR10
//! narrow-waist IR (Stream B of
//! [`HML01`](../../../specs/HML01-math-to-semantic-ir.md)). It consumes the
//! generic `GrammarASTNode` CST produced by the
//! `coding-adventures-wolfram-parser` crate and emits a
//! [`semantic_ir::Module`]. See `lower.rs`'s module doc comment for the
//! full scope and the "everything is data" design decision this frontend
//! makes throughout.
//!
//! ## Pipeline
//!
//! ```text
//! Wolfram source
//!    │
//!    ▼  coding_adventures_wolfram_parser::try_parse_wolfram(src)
//! parser::grammar_parser::GrammarASTNode   (generic CST)
//!    │
//!    ▼  wolfram_to_semantic_ir::compile
//! semantic_ir::Module                      (per SIR10 + SIR23)
//! ```
//!
//! ## Public API
//!
//! ```
//! use wolfram_to_semantic_ir::compile_source;
//! let module = compile_source("1 + 2\n", "demo").unwrap();
//! assert!(module.functions.iter().any(|f| f.name == "main"));
//! ```

mod lower;
pub use lower::{compile, WolframLowerError};

/// Recursion-depth cap `compile_source` parses under, and the enlarged
/// worker-thread stack size it parses *on*.
///
/// `wolfram-parser`'s own `MAX_RULE_DEPTH` doc comment explains why no
/// single cap value can simultaneously be safe on a bare default (~2 MiB)
/// stack *and* support realistic nesting: Wolfram's 20-rule-per-level
/// precedence cascade means even 40 levels of ordinary `(...)` nesting
/// needs ~840 `parse_rule` frames, already past that crate's own measured
/// bare-stack crash floor (~276) regardless of any cap. That doc comment
/// explicitly recommends one of two fixes for a caller in this position:
/// reuse `wolfram-runtime`'s own pattern (parse on an enlarged-stack worker
/// thread) or tighten the cap and accept a much lower nesting ceiling on a
/// bare thread. This crate takes the first option, reusing
/// `wolfram-runtime`'s own validated-safe deployment exactly (its
/// `EVAL_STACK_SIZE`) rather than inventing an unproven new stack size, so
/// `wolfram-parser`'s *default* `MAX_RULE_DEPTH` (2000, ~98 real nesting
/// levels) stays in force — comfortable headroom for realistic compiled
/// programs — while remaining provably safe.
const PARSE_STACK_SIZE: usize = 512 * 1024 * 1024;

/// Parse `source` as Wolfram and lower it into a [`semantic_ir::Module`] in
/// one step, mirroring every other `-to-semantic-ir` frontend's
/// `compile_source` convenience wrapper.
///
/// Unlike a plain call to [`compile`] (which trusts its caller already
/// parsed `source` safely), this function is the hardened entry point: it
/// spawns the parse-then-lower pipeline onto a worker thread with an
/// enlarged stack (see [`PARSE_STACK_SIZE`]'s doc comment), so pathologically
/// deep — but syntactically valid — Wolfram source fails with a clean
/// [`WolframLowerError`] instead of a native, uncatchable stack overflow on
/// an ordinary caller thread. Prefer this over `compile` for any input that
/// was not already parsed under an equivalent guard.
///
/// Both thread-spawn failure (OS resource exhaustion, e.g. hitting a
/// per-process thread-count or address-space limit under many concurrent
/// calls) and a worker-thread panic (from anywhere in the parse/lower
/// pipeline, not just a stack overflow — which is unrecoverable regardless
/// and aborts the whole process before `join` ever runs) are converted into
/// a [`WolframLowerError`] rather than propagated as a panic on the calling
/// thread — a bare `.expect()` on either would otherwise silently defeat
/// this function's own "fails cleanly" guarantee, caught during security
/// review.
pub fn compile_source(
    source: &str,
    module_name: &str,
) -> Result<semantic_ir::Module, WolframLowerError> {
    let source = source.to_string();
    let module_name = module_name.to_string();
    let handle = std::thread::Builder::new()
        .name("wolfram-to-semantic-ir-compile".to_string())
        .stack_size(PARSE_STACK_SIZE)
        .spawn(move || compile_on_this_thread(&source, &module_name))
        .map_err(|e| WolframLowerError {
            message: format!("failed to spawn wolfram-to-semantic-ir compile worker thread: {e}"),
            line: 1,
            column: 1,
        })?;
    match handle.join() {
        Ok(result) => result,
        Err(panic_payload) => Err(WolframLowerError {
            message: format!(
                "wolfram-to-semantic-ir compile worker thread panicked: {}",
                panic_message(&panic_payload)
            ),
            line: 1,
            column: 1,
        }),
    }
}

/// Extract a human-readable message from a `std::thread::Result::Err`
/// panic payload, which is `Box<dyn Any + Send>` and in practice almost
/// always a `&'static str` (a string-literal panic) or a `String` (a
/// formatted panic, e.g. via `panic!("{}", ...)`) — falls back to a fixed
/// placeholder for the rare payload that is neither.
fn panic_message(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic payload>".to_string()
    }
}

fn compile_on_this_thread(
    source: &str,
    module_name: &str,
) -> Result<semantic_ir::Module, WolframLowerError> {
    let tree = coding_adventures_wolfram_parser::try_parse_wolfram(source).map_err(|msg| {
        WolframLowerError {
            message: format!("parse error: {msg}"),
            line: 1,
            column: 1,
        }
    })?;
    compile(&tree, module_name)
}
