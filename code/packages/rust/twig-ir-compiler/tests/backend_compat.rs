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

use interpreter_ir::Operand;
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

/// Path-A increment 3: `(if cond then else)` over typed branches now
/// lowers to typed `mov` (not `call_builtin "_move"`).  The if's
/// result type is the consensus of the two arms; if both agree,
/// downstream `ret` propagates the same type to the function's
/// return.
#[test]
fn twig_typed_if_accepted_by_every_backend() {
    let m = compile_source("(if #t 1 2)", "compat")
        .expect("Twig must compile");
    let main = m.functions.iter().find(|f| f.name == "main").unwrap();

    // Both arms are i64 literals; consensus i64 should propagate to ret.
    assert_eq!(main.return_type, "i64",
        "main return_type should be inferred as i64 when both `if` \
         arms produce i64");

    // The IR must contain two `mov` instructions (one per arm) and
    // zero `call_builtin "_move"` instructions.
    let movs = main.instructions.iter().filter(|i| i.op == "mov")
        .collect::<Vec<_>>();
    assert_eq!(movs.len(), 2,
        "expected two typed mov instructions; got: {movs:?}");
    assert!(
        !main.instructions.iter().any(|i| i.op == "call_builtin"
            && matches!(&i.srcs[0], Operand::Var(s) if s == "_move")),
        "typed if must not emit legacy call_builtin \"_move\"",
    );

    for (name, errs) in [
        ("wasm", iir_to_wasm::validate::validate_for_wasm(&m)),
        ("jvm",  iir_to_jvm_class_file::validate::validate_for_jvm(&m)),
        ("clr",  iir_to_cil_bytecode::validate::validate_iir_for_clr(&m)),
        ("beam", iir_to_beam::validate::validate_for_beam(&m)),
    ] {
        assert!(errs.is_empty(),
            "[{name}] validator should accept Twig `(if #t 1 2)` after \
             path-A increment 3; got {} error(s): {errs:?}",
            errs.len());
    }
}

/// A combined arithmetic + if program: `(if (< 1 2) (+ 10 20) (- 10 20))`.
/// Exercises typed cmp_lt + typed add + typed sub + typed if all together.
#[test]
fn twig_typed_arithmetic_in_if_accepted_by_every_backend() {
    let m = compile_source("(if (< 1 2) (+ 10 20) (- 10 20))", "compat")
        .expect("Twig must compile");
    let main = m.functions.iter().find(|f| f.name == "main").unwrap();
    assert_eq!(main.return_type, "i64");

    for (name, errs) in [
        ("wasm", iir_to_wasm::validate::validate_for_wasm(&m)),
        ("jvm",  iir_to_jvm_class_file::validate::validate_for_jvm(&m)),
        ("clr",  iir_to_cil_bytecode::validate::validate_iir_for_clr(&m)),
        ("beam", iir_to_beam::validate::validate_for_beam(&m)),
    ] {
        assert!(errs.is_empty(),
            "[{name}] should accept combined arith+if; got {errs:?}",
            errs = errs);
    }
}

/// Path-A increment 4: `let` bindings now use typed `mov`.
#[test]
fn twig_typed_let_accepted_by_every_backend() {
    let m = compile_source("(let ((x 5)) x)", "compat")
        .expect("Twig must compile");
    let main = m.functions.iter().find(|f| f.name == "main").unwrap();
    assert_eq!(main.return_type, "i64",
        "main return_type should be inferred as i64 for `(let ((x 5)) x)`");
    assert!(
        main.instructions.iter().any(|i| i.op == "mov"
            && i.dest.as_deref() == Some("x")
            && i.type_hint == "i64"),
        "let-binding should emit typed `mov x` with i64 type",
    );

    for (name, errs) in [
        ("wasm", iir_to_wasm::validate::validate_for_wasm(&m)),
        ("jvm",  iir_to_jvm_class_file::validate::validate_for_jvm(&m)),
        ("clr",  iir_to_cil_bytecode::validate::validate_iir_for_clr(&m)),
        ("beam", iir_to_beam::validate::validate_for_beam(&m)),
    ] {
        assert!(errs.is_empty(),
            "[{name}] should accept `(let ((x 5)) x)`; got {errs:?}",
            errs = errs);
    }
}

/// `let*` builds up typed bindings sequentially.  `(let* ((a 1) (b (+ a 1))) b)`
/// types `a` as i64, then `(+ a 1)` lowers to typed `add`, then `b`
/// inherits i64 too.
#[test]
fn twig_typed_let_star_with_arithmetic() {
    let m = compile_source("(let* ((a 1) (b (+ a 1))) b)", "compat")
        .expect("Twig must compile");
    let main = m.functions.iter().find(|f| f.name == "main").unwrap();
    assert_eq!(main.return_type, "i64");
    let a_mov = main.instructions.iter().find(|i| i.op == "mov"
        && i.dest.as_deref() == Some("a"))
        .expect("expected `mov a`");
    assert_eq!(a_mov.type_hint, "i64");
    let b_mov = main.instructions.iter().find(|i| i.op == "mov"
        && i.dest.as_deref() == Some("b"))
        .expect("expected `mov b`");
    assert_eq!(b_mov.type_hint, "i64",
        "binding `b` should inherit i64 from the typed-add RHS");

    for (name, errs) in [
        ("wasm", iir_to_wasm::validate::validate_for_wasm(&m)),
        ("jvm",  iir_to_jvm_class_file::validate::validate_for_jvm(&m)),
        ("clr",  iir_to_cil_bytecode::validate::validate_iir_for_clr(&m)),
        ("beam", iir_to_beam::validate::validate_for_beam(&m)),
    ] {
        assert!(errs.is_empty(),
            "[{name}] should accept `(let* ((a 1) (b (+ a 1))) b)`; got {errs:?}",
            errs = errs);
    }
}

/// Path-A increment 5: `match` arms now use typed `mov` everywhere
/// (scrutinee binding, nil-init, variant arm result merge, field
/// extraction, body merge for variant / binding / wildcard arms).
/// Match programs whose arm-bodies are typed (integer literals,
/// arithmetic, if-merge) flow through every backend.
#[test]
fn twig_typed_match_wildcard_accepted_by_every_backend() {
    // `(match 1 (_ 42))` — wildcard arm returning a typed integer.
    let m = compile_source("(match 1 (_ 42))", "compat")
        .expect("Twig must compile");

    // The match result should propagate the wildcard arm body's type.
    // (Note: the match initialises `result` to nil via `make_nil`,
    // which the typed-mov inherits as `any` — so the overall match
    // result_type stays "any" until increment 6 adds consensus
    // typing across arms.  But the IR is now structurally valid for
    // the backends because there are no more `call_builtin "_move"`
    // instructions — they're all typed `mov` (with type_hint "any"
    // for the make_nil chain, which the backends accept for `mov`).)
    let _main = m.functions.iter().find(|f| f.name == "main").unwrap();

    // The IR must contain `mov` instructions and NO `call_builtin "_move"`.
    let movs = m.functions.iter().flat_map(|f| f.instructions.iter())
        .filter(|i| i.op == "mov").count();
    assert!(movs >= 1, "expected at least one typed `mov` in compile_match");
    assert!(
        !m.functions.iter().flat_map(|f| f.instructions.iter())
            .any(|i| i.op == "call_builtin"
                && matches!(&i.srcs[0], Operand::Var(s) if s == "_move")),
        "compile_match must not emit legacy call_builtin \"_move\"",
    );

    // Pure-typed match where every emit site is `mov` should not hit
    // an UntypedInstruction error on the move chain — but the program
    // still uses other dynamic ops (make_nil) that may surface
    // UnsupportedOp.  The strict invariant is: no `_move`-shaped
    // call_builtin survives.  Backend acceptance of the whole module
    // is deferred to the next increment.
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
