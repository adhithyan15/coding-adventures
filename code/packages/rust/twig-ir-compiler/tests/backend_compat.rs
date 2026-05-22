//! Twig → IIR-to-* backend acceptance tests (Path A, increment 1).
//!
//! Before this PR the simplest possible Twig program — `42` — was
//! rejected by every IIR-to-* backend's validator because every
//! instruction carried `type_hint = "any"`.  After this PR, integer
//! and boolean literals (plus the `ret` instructions that consume
//! them) carry concrete type hints (`"i64"` / `"bool"`), and all four
//! backends accept the resulting module.
//!
//! Larger Twig programs — those using arithmetic, lambdas, lists,
//! strings beyond simple literals, etc. — still emit some `"any"`
//! instructions and remain rejected by the IIR-to-* validators.
//! Subsequent path-A increments will narrow that gap.

use twig_ir_compiler::compile_source;

/// `42` — the smallest Twig program — must now reach every backend's
/// validator without errors.
#[test]
fn twig_int_literal_accepted_by_every_backend() {
    let m = compile_source("42", "compat").expect("Twig must compile");

    // Sanity: the main function's return_type must be the literal's
    // inferred type, not the legacy `"any"`.  Without this, the
    // backends would lower `ret` to a generic untyped variant.
    let main = m.functions.iter().find(|f| f.name == "main")
        .expect("module must have main");
    assert_eq!(main.return_type, "i64",
        "main return_type should be inferred as i64 for literal `42`");

    for (name, errs) in [
        ("wasm", iir_to_wasm::validate::validate_for_wasm(&m)),
        ("jvm",  iir_to_jvm_class_file::validate::validate_for_jvm(&m)),
        ("clr",  iir_to_cil_bytecode::validate::validate_iir_for_clr(&m)),
        ("beam", iir_to_beam::validate::validate_for_beam(&m)),
    ] {
        assert!(errs.is_empty(),
            "[{name}] validator should accept Twig `42` after path-A \
             increment 1; got {} error(s): {errs:?}",
            errs.len());
    }
}

/// `#t` (boolean literal) — same chain as the integer test.
#[test]
fn twig_bool_literal_accepted_by_every_backend() {
    let m = compile_source("#t", "compat").expect("Twig must compile");
    let main = m.functions.iter().find(|f| f.name == "main").unwrap();
    assert_eq!(main.return_type, "bool",
        "main return_type should be inferred as bool for literal `#t`");

    for (name, errs) in [
        ("wasm", iir_to_wasm::validate::validate_for_wasm(&m)),
        ("jvm",  iir_to_jvm_class_file::validate::validate_for_jvm(&m)),
        ("clr",  iir_to_cil_bytecode::validate::validate_iir_for_clr(&m)),
        ("beam", iir_to_beam::validate::validate_for_beam(&m)),
    ] {
        assert!(errs.is_empty(),
            "[{name}] validator should accept Twig `#t`; got {errs:?}",
            errs = errs);
    }
}

/// Path-A increment 1 deliberately does NOT type arithmetic / call_builtin
/// dispatches.  This test pins down the *current* boundary: a program
/// like `(+ 1 2)` still emits `call_builtin` with `type_hint "any"`,
/// so every backend still rejects it.  When a future increment lowers
/// the `+` builtin to a typed `add_i64`, this test should flip.
///
/// Keeping the test as-is (asserting rejection) makes the boundary
/// visible in CI — accidentally extending typing into builtin-call
/// territory without updating this test would surface as a failure.
#[test]
fn twig_arithmetic_still_rejected_in_increment_1() {
    let m = compile_source("(+ 1 2)", "compat").expect("Twig must compile");
    let wasm_errs = iir_to_wasm::validate::validate_for_wasm(&m);
    assert!(!wasm_errs.is_empty(),
        "increment 1 does NOT yet type arithmetic — `(+ 1 2)` should still \
         hit an UntypedInstruction error on call_builtin; got: {wasm_errs:?}",
    );
    // The error must be `UntypedInstruction` (on `call_builtin`),
    // not something else (e.g. a missing operand).
    assert!(
        wasm_errs.iter().any(|e| e.contains("UntypedInstruction")),
        "expected UntypedInstruction for `(+ 1 2)`; got: {wasm_errs:?}",
    );
}
