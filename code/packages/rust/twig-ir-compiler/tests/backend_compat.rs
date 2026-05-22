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

/// Path-A increment 2: `(+ 1 2)` with two i64 literal arguments now
/// lowers to a typed `add` (not `call_builtin "+"`).  Every IIR-to-*
/// backend therefore accepts the resulting module.  This test —
/// originally a boundary marker for increment 1's *rejection* — has
/// flipped to assert *acceptance*.
#[test]
fn twig_typed_arithmetic_accepted_by_every_backend() {
    let m = compile_source("(+ 1 2)", "compat").expect("Twig must compile");

    // Confirm the typed lowering fired (no call_builtin "+" should be
    // present; instead we expect a typed `add` instruction).
    let main = m.functions.iter().find(|f| f.name == "main").unwrap();
    assert!(
        main.instructions.iter().any(|i| i.op == "add" && i.type_hint == "i64"),
        "increment 2: `(+ 1 2)` must emit `add [i64]`; got: {:?}",
        main.instructions,
    );

    for (name, errs) in [
        ("wasm", iir_to_wasm::validate::validate_for_wasm(&m)),
        ("jvm",  iir_to_jvm_class_file::validate::validate_for_jvm(&m)),
        ("clr",  iir_to_cil_bytecode::validate::validate_iir_for_clr(&m)),
        ("beam", iir_to_beam::validate::validate_for_beam(&m)),
    ] {
        assert!(errs.is_empty(),
            "[{name}] validator should accept Twig `(+ 1 2)` after path-A \
             increment 2; got {} error(s): {errs:?}",
            errs.len());
    }
}

/// Comparison binaries (`= < > <= >=`) on i64 args also lower to typed
/// `cmp_*` mnemonics in increment 2.  Result type is `bool`.
#[test]
fn twig_typed_comparison_accepted_by_every_backend() {
    let m = compile_source("(< 1 2)", "compat").expect("Twig must compile");
    let main = m.functions.iter().find(|f| f.name == "main").unwrap();
    assert_eq!(main.return_type, "bool",
        "main return_type should be inferred as bool for `(< 1 2)`");
    assert!(
        main.instructions.iter().any(|i| i.op == "cmp_lt" && i.type_hint == "bool"),
        "increment 2: `(< 1 2)` must emit `cmp_lt [bool]`; got: {:?}",
        main.instructions,
    );

    for (name, errs) in [
        ("wasm", iir_to_wasm::validate::validate_for_wasm(&m)),
        ("jvm",  iir_to_jvm_class_file::validate::validate_for_jvm(&m)),
        ("clr",  iir_to_cil_bytecode::validate::validate_iir_for_clr(&m)),
        ("beam", iir_to_beam::validate::validate_for_beam(&m)),
    ] {
        assert!(errs.is_empty(),
            "[{name}] validator should accept Twig `(< 1 2)`; got {errs:?}",
            errs = errs);
    }
}

/// Pin the *current* boundary: arithmetic where at least one operand
/// has a dynamic type (comes from a `call_builtin` like `car`) still
/// emits `call_builtin "+"` and gets rejected by every backend.  When
/// a later increment adds runtime type guards or closure-call
/// inference, this test should flip.
#[test]
fn twig_arithmetic_over_dynamic_args_still_rejected() {
    // (+ (car (cons 1 2)) 3) — left arg is `any` from cons/car.
    let m = compile_source("(+ (car (cons 1 2)) 3)", "compat")
        .expect("Twig must compile");
    let wasm_errs = iir_to_wasm::validate::validate_for_wasm(&m);
    assert!(!wasm_errs.is_empty(),
        "dynamic-arg arithmetic should still be rejected; got: {wasm_errs:?}",
    );
    assert!(
        wasm_errs.iter().any(|e| e.contains("UntypedInstruction")
                            || e.contains("UnsupportedOp")),
        "expected UntypedInstruction or UnsupportedOp for dynamic `+`; \
         got: {wasm_errs:?}",
    );
}
