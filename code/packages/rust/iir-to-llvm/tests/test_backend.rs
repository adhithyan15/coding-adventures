//! Integration tests for `iir-to-llvm` v0.1.0 (LLVM01 skeleton).
//!
//! These tests exercise the public surface — `validate_for_llvm` +
//! `lower_iir_to_llvm` + `IIRLlvmConfig` — and assert on the smallest
//! correctness signals available at this scope:
//!
//! 1. The output starts with either `;` (an LLVM comment, conventionally
//!    `; ModuleID = …`) or `target` (the LLVM `target triple = …`
//!    directive).  Anything else is malformed at first glance.
//! 2. Both header lines are present.
//! 3. Config defaults are sane.
//!
//! As instructions come online in v0.2.0+ this file grows accordingly.

use interpreter_ir::IIRModule;
use iir_to_llvm::{lower_iir_to_llvm, validate_for_llvm, IIRLlvmConfig, IIRLlvmError};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// An empty IIR module — no functions, no exports, no imports.  v0.1.0
/// doesn't lower instructions, so this is the canonical input fixture for
/// every test in this file.
fn empty_module() -> IIRModule {
    IIRModule {
        name: "demo".into(),
        functions: vec![],
        entry_point: None,
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    }
}

// ===========================================================================
// 1. Validator (stub) behaviour
// ===========================================================================

/// v0.1.0 validator returns an empty `Vec` — no rules yet.
#[test]
fn validate_returns_empty_for_empty_module() {
    assert!(validate_for_llvm(&empty_module()).is_empty());
}

// ===========================================================================
// 2. Output shape
// ===========================================================================

/// The emitted `.ll` contains a `; ModuleID = '<name>'` comment with the
/// configured module name.
#[test]
fn output_contains_module_id_comment() {
    let cfg = IIRLlvmConfig::new("hello_module");
    let ll = lower_iir_to_llvm(&empty_module(), &cfg).expect("lowering");
    assert!(
        ll.contains("; ModuleID = 'hello_module'"),
        "expected ModuleID comment with name 'hello_module'; got:\n{ll}"
    );
}

/// The emitted `.ll` contains a `target triple = "<triple>"` directive with
/// the configured triple.
#[test]
fn output_contains_target_triple() {
    let cfg = IIRLlvmConfig::default().with_target("riscv32-unknown-elf");
    let ll = lower_iir_to_llvm(&empty_module(), &cfg).expect("lowering");
    assert!(
        ll.contains("target triple = \"riscv32-unknown-elf\""),
        "expected target triple directive for riscv32; got:\n{ll}"
    );
}

/// **LLVM01 acceptance criterion**: the first non-blank line of the output
/// must begin with either `;` (a comment) or `target` (an LLVM directive).
/// Anything else is malformed at first glance and tells us the emitter is
/// off-by-one or producing garbage.
#[test]
fn output_starts_with_comment_or_target() {
    let ll = lower_iir_to_llvm(&empty_module(), &IIRLlvmConfig::default())
        .expect("lowering");
    let first = ll
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .expect("output must have at least one non-blank line");
    assert!(
        first.starts_with(';') || first.starts_with("target"),
        "first non-blank line must start with ';' or 'target'; got: {first:?}"
    );
}

// ===========================================================================
// 3. Config defaults
// ===========================================================================

/// The default target triple is a non-empty string so downstream `llc`
/// doesn't fall back to the build host (which would be nondeterministic).
#[test]
fn default_config_has_nonempty_triple() {
    let cfg = IIRLlvmConfig::default();
    assert!(
        !cfg.target_triple.is_empty(),
        "default target triple must be non-empty"
    );
    assert!(
        !cfg.module_name.is_empty(),
        "default module_name must be non-empty"
    );
}

/// `IIRLlvmConfig::new` sets the module name but leaves the triple at
/// default.  This is the smallest behavioural contract on the builder.
#[test]
fn new_sets_module_name_keeps_default_triple() {
    let cfg = IIRLlvmConfig::new("custom");
    assert_eq!(cfg.module_name, "custom");
    assert_eq!(cfg.target_triple, IIRLlvmConfig::default().target_triple);
}

// ===========================================================================
// 4. Error display
// ===========================================================================

/// Smoke-check that error variants `Display` without panicking.  Each variant
/// is its own assertion so a regression in one doesn't mask another.
#[test]
fn errors_display_without_panic() {
    let _ = format!("{}", IIRLlvmError::ValidationFailed(vec!["x".into()]));
    let _ = format!(
        "{}",
        IIRLlvmError::UnsupportedOp {
            function: "f".into(),
            op: "weird_op".into(),
        }
    );
    let _ = format!(
        "{}",
        IIRLlvmError::UnsupportedType {
            function: "f".into(),
            type_hint: "weird".into(),
        }
    );
    let _ = format!(
        "{}",
        IIRLlvmError::InvalidOperand {
            function: "f".into(),
            detail: "bad shape".into(),
        }
    );
}
