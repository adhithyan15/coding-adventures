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

/// E4 op-composition proof: a typed `string-append` result can feed
/// `string-ref` without falling back to the dynamic builtin path.
#[test]
fn twig_local_string_concat_can_feed_index() {
    let m = compile_source(
        "(let ((a \"AB\") (b \"CDE\") (i 3)) (string-ref (string-append a b) i))",
        "compat",
    )
    .expect("Twig must compile");
    let main = m.functions.iter().find(|f| f.name == "main").unwrap();
    assert_eq!(main.return_type, "i64");

    let concat = main
        .instructions
        .iter()
        .find(|i| i.op == "str_concat")
        .expect("string-append should lower to str_concat");
    let concat_dest = concat
        .dest
        .as_deref()
        .expect("str_concat should write a string temp");
    let index = main
        .instructions
        .iter()
        .find(|i| i.op == "str_index")
        .expect("string-ref should lower to str_index");
    assert!(
        matches!(index.srcs.first(), Some(Operand::Var(v)) if v == concat_dest),
        "str_index should consume the str_concat result; concat={concat:?}, index={index:?}",
    );
    assert!(
        !main.instructions.iter().any(|i| {
            i.op == "call_builtin"
                && matches!(&i.srcs[0], Operand::Var(s)
                    if s == "string-append" || s == "string-ref")
        }),
        "typed E4 string path must not use dynamic string builtins: {:?}",
        main.instructions,
    );

    for (name, errs) in [
        ("wasm", iir_to_wasm::validate::validate_for_wasm(&m)),
        ("jvm", iir_to_jvm_class_file::validate::validate_for_jvm(&m)),
        (
            "clr",
            iir_to_cil_bytecode::validate::validate_iir_for_clr(&m),
        ),
    ] {
        assert!(
            errs.is_empty(),
            "[{name}] should accept local `str_concat` feeding `str_index`; got {errs:?}",
            errs = errs
        );
    }
}

/// E4 op-composition proof: `string-length` can compute a typed integer
/// index that feeds `string-ref` without falling back to dynamic builtins.
#[test]
fn twig_local_string_length_can_compute_index() {
    let m = compile_source(
        "(let ((s \"ABCDE\")) (string-ref s (- (string-length s) 1)))",
        "compat",
    )
    .expect("Twig must compile");
    let main = m.functions.iter().find(|f| f.name == "main").unwrap();
    assert_eq!(main.return_type, "i64");

    let len = main
        .instructions
        .iter()
        .find(|i| i.op == "str_len")
        .expect("string-length should lower to str_len");
    let len_dest = len
        .dest
        .as_deref()
        .expect("str_len should write an integer temp");
    let sub = main
        .instructions
        .iter()
        .find(|i| i.op == "sub" && i.type_hint == "i64")
        .expect("computed index should lower to typed sub");
    let sub_dest = sub.dest.as_deref().expect("sub should write an index temp");
    assert!(
        matches!(sub.srcs.first(), Some(Operand::Var(v)) if v == len_dest),
        "sub should consume the str_len result; len={len:?}, sub={sub:?}",
    );
    let index = main
        .instructions
        .iter()
        .find(|i| i.op == "str_index")
        .expect("string-ref should lower to str_index");
    assert!(
        matches!(index.srcs.get(1), Some(Operand::Var(v)) if v == sub_dest),
        "str_index should consume the computed index; sub={sub:?}, index={index:?}",
    );
    assert!(
        !main.instructions.iter().any(|i| {
            i.op == "call_builtin"
                && matches!(&i.srcs[0], Operand::Var(s)
                    if s == "string-length" || s == "string-ref" || s == "-")
        }),
        "typed E4 computed-index path must not use dynamic builtins: {:?}",
        main.instructions,
    );

    for (name, errs) in [
        ("wasm", iir_to_wasm::validate::validate_for_wasm(&m)),
        ("jvm", iir_to_jvm_class_file::validate::validate_for_jvm(&m)),
        (
            "clr",
            iir_to_cil_bytecode::validate::validate_iir_for_clr(&m),
        ),
    ] {
        assert!(
            errs.is_empty(),
            "[{name}] should accept `str_len` computing a `str_index` operand; got {errs:?}",
            errs = errs
        );
    }
}

/// E4 function-call proof: a direct top-level Twig function whose body already
/// lowers to typed E4 string ops should carry that return type through the
/// caller's `call` instruction instead of falling back to `any`.
#[test]
fn twig_top_level_string_length_function_call_is_typed() {
    let m = compile_source(
        "(define (strlen) (string-length \"HELLO\")) (strlen)",
        "compat",
    )
    .expect("Twig must compile");

    let strlen = m
        .functions
        .iter()
        .find(|f| f.name == "strlen")
        .expect("module should contain the top-level function");
    assert_eq!(strlen.return_type, "i64");
    assert!(
        strlen
            .instructions
            .iter()
            .any(|i| i.op == "str_len" && i.type_hint == "i64"),
        "function body should lower string-length to typed str_len: {:?}",
        strlen.instructions,
    );
    assert!(
        strlen
            .instructions
            .iter()
            .any(|i| i.op == "ret" && i.type_hint == "i64"),
        "function ret should carry the inferred i64 type: {:?}",
        strlen.instructions,
    );

    let main = m.functions.iter().find(|f| f.name == "main").unwrap();
    assert_eq!(main.return_type, "i64");
    assert!(
        main.instructions.iter().any(|i| {
            i.op == "call"
                && i.type_hint == "i64"
                && matches!(i.srcs.first(), Some(Operand::Var(name)) if name == "strlen")
        }),
        "direct call should inherit strlen's concrete return type: {:?}",
        main.instructions,
    );
    assert!(
        !m.functions.iter().flat_map(|f| f.instructions.iter()).any(|i| {
            i.op == "call_builtin"
                && matches!(&i.srcs[0], Operand::Var(s) if s == "string-length")
        }),
        "typed E4 function path must not use dynamic string builtins",
    );

    for (name, errs) in [
        ("wasm", iir_to_wasm::validate::validate_for_wasm(&m)),
        ("jvm", iir_to_jvm_class_file::validate::validate_for_jvm(&m)),
        (
            "clr",
            iir_to_cil_bytecode::validate::validate_iir_for_clr(&m),
        ),
    ] {
        assert!(
            errs.is_empty(),
            "[{name}] should accept typed E4 string ops inside a direct top-level function; got {errs:?}",
            errs = errs
        );
    }
}

/// E4 parameter proof: a bare `str` parameter annotation is enough static
/// evidence for a top-level function body to use the typed E4 string path.
#[test]
fn twig_annotated_string_param_feeds_string_length_function() {
    let m = compile_source(
        "(define (strlen (s : str)) (string-length s)) (strlen \"HELLO\")",
        "compat",
    )
    .expect("Twig must compile");

    let strlen = m
        .functions
        .iter()
        .find(|f| f.name == "strlen")
        .expect("module should contain the top-level function");
    assert_eq!(strlen.params, vec![("s".to_string(), "str".to_string())]);
    assert_eq!(strlen.return_type, "i64");
    let len = strlen
        .instructions
        .iter()
        .find(|i| i.op == "str_len")
        .expect("string-length should lower to str_len over the annotated param");
    assert!(
        matches!(len.srcs.first(), Some(Operand::Var(name)) if name == "s"),
        "str_len should consume the annotated string parameter; got {len:?}",
    );
    assert!(
        !strlen.instructions.iter().any(|i| {
            i.op == "call_builtin"
                && matches!(&i.srcs[0], Operand::Var(name) if name == "string-length")
        }),
        "typed E4 parameter path must not use dynamic string-length builtins: {:?}",
        strlen.instructions,
    );

    let main = m.functions.iter().find(|f| f.name == "main").unwrap();
    assert_eq!(main.return_type, "i64");
    assert!(
        main.instructions.iter().any(|i| {
            i.op == "call"
                && i.type_hint == "i64"
                && matches!(i.srcs.first(), Some(Operand::Var(name)) if name == "strlen")
        }),
        "direct call should inherit strlen's concrete return type: {:?}",
        main.instructions,
    );

    for (name, errs) in [
        ("wasm", iir_to_wasm::validate::validate_for_wasm(&m)),
        ("jvm", iir_to_jvm_class_file::validate::validate_for_jvm(&m)),
        (
            "clr",
            iir_to_cil_bytecode::validate::validate_iir_for_clr(&m),
        ),
    ] {
        assert!(
            errs.is_empty(),
            "[{name}] should accept a typed E4 string op over an annotated string parameter; got {errs:?}",
            errs = errs
        );
    }
}

/// E4 direct-call evidence proof: a `main`-level direct call with a static
/// string actual can seed a top-level function's otherwise-unannotated string
/// parameter without opting into refinement annotations.
#[test]
fn twig_unannotated_string_param_direct_call_feeds_string_length_function() {
    let m = compile_source(
        "(define (strlen s) (string-length s)) (strlen \"HELLO\")",
        "compat",
    )
    .expect("Twig must compile");

    let strlen = m
        .functions
        .iter()
        .find(|f| f.name == "strlen")
        .expect("module should contain the top-level function");
    assert_eq!(strlen.params, vec![("s".to_string(), "str".to_string())]);
    assert!(
        strlen.param_refinements.iter().all(|r| r.is_none()),
        "call-site string evidence must not synthesize refinement annotations: {:?}",
        strlen.param_refinements,
    );
    assert_eq!(strlen.return_type, "i64");
    let len = strlen
        .instructions
        .iter()
        .find(|i| i.op == "str_len")
        .expect("string-length should lower to str_len over the inferred param");
    assert!(
        matches!(len.srcs.first(), Some(Operand::Var(name)) if name == "s"),
        "str_len should consume the inferred string parameter; got {len:?}",
    );
    assert!(
        !strlen.instructions.iter().any(|i| {
            i.op == "call_builtin"
                && matches!(&i.srcs[0], Operand::Var(name) if name == "string-length")
        }),
        "typed E4 inferred-parameter path must not use dynamic string-length builtins: {:?}",
        strlen.instructions,
    );

    let main = m.functions.iter().find(|f| f.name == "main").unwrap();
    assert_eq!(main.return_type, "i64");
    assert!(
        main.instructions.iter().any(|i| {
            i.op == "call"
                && i.type_hint == "i64"
                && matches!(i.srcs.first(), Some(Operand::Var(name)) if name == "strlen")
        }),
        "direct call should inherit strlen's concrete return type: {:?}",
        main.instructions,
    );

    for (name, errs) in [
        ("wasm", iir_to_wasm::validate::validate_for_wasm(&m)),
        ("jvm", iir_to_jvm_class_file::validate::validate_for_jvm(&m)),
        (
            "clr",
            iir_to_cil_bytecode::validate::validate_iir_for_clr(&m),
        ),
    ] {
        assert!(
            errs.is_empty(),
            "[{name}] should accept a typed E4 string op over an inferred string parameter; got {errs:?}",
            errs = errs
        );
    }
}

/// E4 direct-call evidence proof: a static string expression actual can seed
/// the same otherwise-unannotated top-level string parameter path as a string
/// literal actual.
#[test]
fn twig_unannotated_string_param_direct_call_accepts_static_string_expression_actual() {
    let m = compile_source(
        "(define (strlen x) (string-length x)) (strlen (substring (string-append \"HE\" \"LLO!\") 0 5))",
        "compat",
    )
    .expect("Twig must compile");

    let strlen = m
        .functions
        .iter()
        .find(|f| f.name == "strlen")
        .expect("module should contain the top-level function");
    assert_eq!(strlen.params, vec![("x".to_string(), "str".to_string())]);
    assert!(
        strlen.param_refinements.iter().all(|r| r.is_none()),
        "call-site string evidence must not synthesize refinement annotations: {:?}",
        strlen.param_refinements,
    );
    assert_eq!(strlen.return_type, "i64");
    let len = strlen
        .instructions
        .iter()
        .find(|i| i.op == "str_len")
        .expect("string-length should lower to str_len over the inferred param");
    assert!(
        matches!(len.srcs.first(), Some(Operand::Var(name)) if name == "x"),
        "str_len should consume the inferred string parameter; got {len:?}",
    );
    assert!(
        !strlen.instructions.iter().any(|i| {
            i.op == "call_builtin"
                && matches!(&i.srcs[0], Operand::Var(name) if name == "string-length")
        }),
        "typed E4 inferred-parameter path must not use dynamic string-length builtins: {:?}",
        strlen.instructions,
    );

    let main = m.functions.iter().find(|f| f.name == "main").unwrap();
    assert_eq!(main.return_type, "i64");
    assert!(
        main.instructions.iter().any(|i| i.op == "str_concat"),
        "static expression actual should materialise concat through typed str_concat: {:?}",
        main.instructions,
    );
    assert!(
        main.instructions.iter().any(|i| i.op == "str_slice"),
        "static expression actual should materialise substring through typed str_slice: {:?}",
        main.instructions,
    );
    assert!(
        main.instructions.iter().any(|i| {
            i.op == "call"
                && i.type_hint == "i64"
                && matches!(i.srcs.first(), Some(Operand::Var(name)) if name == "strlen")
        }),
        "direct call should inherit strlen's concrete return type: {:?}",
        main.instructions,
    );

    for (name, errs) in [
        ("wasm", iir_to_wasm::validate::validate_for_wasm(&m)),
        ("jvm", iir_to_jvm_class_file::validate::validate_for_jvm(&m)),
        (
            "clr",
            iir_to_cil_bytecode::validate::validate_iir_for_clr(&m),
        ),
    ] {
        assert!(
            errs.is_empty(),
            "[{name}] should accept a typed E4 string op over a static-expression-actual inferred string parameter; got {errs:?}",
            errs = errs
        );
    }
}

/// E4 direct-call evidence proof: a single direct call can seed multiple
/// otherwise-unannotated top-level string parameters, letting the function body
/// lower a parameter-to-parameter string equality through typed `str_eq`.
#[test]
fn twig_unannotated_string_param_direct_call_accepts_multiple_string_params() {
    let m = compile_source(
        "(define (same a b) (if (string=? a b) 42 0)) (same \"OK\" (string-append \"O\" \"K\"))",
        "compat",
    )
    .expect("Twig must compile");

    let same = m
        .functions
        .iter()
        .find(|f| f.name == "same")
        .expect("module should contain the top-level function");
    assert_eq!(
        same.params,
        vec![
            ("a".to_string(), "str".to_string()),
            ("b".to_string(), "str".to_string()),
        ]
    );
    assert!(
        same.param_refinements.iter().all(|r| r.is_none()),
        "call-site string evidence must not synthesize refinement annotations: {:?}",
        same.param_refinements,
    );
    assert_eq!(same.return_type, "i64");
    let eq = same
        .instructions
        .iter()
        .find(|i| i.op == "str_eq")
        .expect("string=? should lower to str_eq over the inferred params");
    assert!(
        matches!(eq.srcs.as_slice(), [Operand::Var(a), Operand::Var(b)] if a == "a" && b == "b"),
        "str_eq should consume both inferred string parameters; got {eq:?}",
    );
    assert!(
        !same.instructions.iter().any(|i| {
            i.op == "call_builtin"
                && matches!(&i.srcs[0], Operand::Var(name) if name == "string=?")
        }),
        "typed E4 inferred-parameter path must not use dynamic string=? builtins: {:?}",
        same.instructions,
    );

    let main = m.functions.iter().find(|f| f.name == "main").unwrap();
    assert_eq!(main.return_type, "i64");
    assert!(
        main.instructions.iter().any(|i| i.op == "str_concat"),
        "second direct-call actual should materialise through typed str_concat: {:?}",
        main.instructions,
    );
    assert!(
        main.instructions.iter().any(|i| {
            i.op == "call"
                && i.type_hint == "i64"
                && matches!(i.srcs.first(), Some(Operand::Var(name)) if name == "same")
        }),
        "direct call should inherit same's concrete return type: {:?}",
        main.instructions,
    );

    for (name, errs) in [
        ("wasm", iir_to_wasm::validate::validate_for_wasm(&m)),
        ("jvm", iir_to_jvm_class_file::validate::validate_for_jvm(&m)),
        (
            "clr",
            iir_to_cil_bytecode::validate::validate_iir_for_clr(&m),
        ),
    ] {
        assert!(
            errs.is_empty(),
            "[{name}] should accept typed E4 string equality over multiple inferred string parameters; got {errs:?}",
            errs = errs
        );
    }
}

/// E4 direct-call evidence proof: a static, non-escaping top-level string value
/// used as a `main`-level direct-call actual can seed the same
/// otherwise-unannotated string parameter path as a literal.
#[test]
fn twig_unannotated_string_param_direct_call_accepts_named_string_actual() {
    let m = compile_source(
        "(define s \"HELLO\") (define (strlen x) (string-length x)) (strlen s)",
        "compat",
    )
    .expect("Twig must compile");

    let strlen = m
        .functions
        .iter()
        .find(|f| f.name == "strlen")
        .expect("module should contain the top-level function");
    assert_eq!(strlen.params, vec![("x".to_string(), "str".to_string())]);
    assert!(
        strlen.param_refinements.iter().all(|r| r.is_none()),
        "call-site string evidence must not synthesize refinement annotations: {:?}",
        strlen.param_refinements,
    );
    assert_eq!(strlen.return_type, "i64");
    let len = strlen
        .instructions
        .iter()
        .find(|i| i.op == "str_len")
        .expect("string-length should lower to str_len over the inferred param");
    assert!(
        matches!(len.srcs.first(), Some(Operand::Var(name)) if name == "x"),
        "str_len should consume the inferred string parameter; got {len:?}",
    );
    assert!(
        !strlen.instructions.iter().any(|i| {
            i.op == "call_builtin"
                && matches!(&i.srcs[0], Operand::Var(name) if name == "string-length")
        }),
        "typed E4 inferred-parameter path must not use dynamic string-length builtins: {:?}",
        strlen.instructions,
    );

    let main = m.functions.iter().find(|f| f.name == "main").unwrap();
    assert_eq!(main.return_type, "i64");
    assert!(
        !main.instructions.iter().any(|i| {
            i.op == "call_builtin" && matches!(&i.srcs[0], Operand::Var(name) if name == "global_get")
        }),
        "non-escaping named string actual should stay in main as a typed register: {:?}",
        main.instructions,
    );

    for (name, errs) in [
        ("wasm", iir_to_wasm::validate::validate_for_wasm(&m)),
        ("jvm", iir_to_jvm_class_file::validate::validate_for_jvm(&m)),
        (
            "clr",
            iir_to_cil_bytecode::validate::validate_iir_for_clr(&m),
        ),
    ] {
        assert!(
            errs.is_empty(),
            "[{name}] should accept a typed E4 string op over a named-actual inferred string parameter; got {errs:?}",
            errs = errs
        );
    }
}

/// E4 direct-call evidence proof: a lexical string local in `main` can seed an
/// otherwise-unannotated top-level string parameter when the direct call occurs
/// in the lexical scope that keeps the actual as a typed `str` register.
#[test]
fn twig_unannotated_string_param_direct_call_accepts_let_string_actual() {
    let m = compile_source(
        "(define (strlen x) (string-length x)) (let ((s \"HELLO\")) (strlen s))",
        "compat",
    )
    .expect("Twig must compile");

    let strlen = m
        .functions
        .iter()
        .find(|f| f.name == "strlen")
        .expect("module should contain the top-level function");
    assert_eq!(strlen.params, vec![("x".to_string(), "str".to_string())]);
    assert!(
        strlen.param_refinements.iter().all(|r| r.is_none()),
        "call-site string evidence must not synthesize refinement annotations: {:?}",
        strlen.param_refinements,
    );
    assert_eq!(strlen.return_type, "i64");
    let len = strlen
        .instructions
        .iter()
        .find(|i| i.op == "str_len")
        .expect("string-length should lower to str_len over the inferred param");
    assert!(
        matches!(len.srcs.first(), Some(Operand::Var(name)) if name == "x"),
        "str_len should consume the inferred string parameter; got {len:?}",
    );

    let main = m.functions.iter().find(|f| f.name == "main").unwrap();
    assert_eq!(main.return_type, "i64");

    for (name, errs) in [
        ("wasm", iir_to_wasm::validate::validate_for_wasm(&m)),
        ("jvm", iir_to_jvm_class_file::validate::validate_for_jvm(&m)),
        (
            "clr",
            iir_to_cil_bytecode::validate::validate_iir_for_clr(&m),
        ),
    ] {
        assert!(
            errs.is_empty(),
            "[{name}] should accept a typed E4 string op over a lexical-actual inferred string parameter; got {errs:?}",
            errs = errs
        );
    }
}

/// E4 direct-call evidence proof: a sequential `let*` lexical string local
/// derived from an earlier local can seed an otherwise-unannotated top-level
/// string parameter when the direct call sees the derived value as a typed
/// `str` register.
#[test]
fn twig_unannotated_string_param_direct_call_accepts_let_star_derived_string_actual() {
    let m = compile_source(
        "(define (strlen x) (string-length x)) (let* ((a \"HE\") (b (string-append a \"LLO\"))) (strlen b))",
        "compat",
    )
    .expect("Twig must compile");

    let strlen = m
        .functions
        .iter()
        .find(|f| f.name == "strlen")
        .expect("module should contain the top-level function");
    assert_eq!(strlen.params, vec![("x".to_string(), "str".to_string())]);
    assert!(
        strlen.param_refinements.iter().all(|r| r.is_none()),
        "call-site string evidence must not synthesize refinement annotations: {:?}",
        strlen.param_refinements,
    );
    assert_eq!(strlen.return_type, "i64");
    let len = strlen
        .instructions
        .iter()
        .find(|i| i.op == "str_len")
        .expect("string-length should lower to str_len over the inferred param");
    assert!(
        matches!(len.srcs.first(), Some(Operand::Var(name)) if name == "x"),
        "str_len should consume the inferred string parameter; got {len:?}",
    );
    assert!(
        !strlen.instructions.iter().any(|i| {
            i.op == "call_builtin"
                && matches!(&i.srcs[0], Operand::Var(name) if name == "string-length")
        }),
        "typed E4 inferred-parameter path must not use dynamic string-length builtins: {:?}",
        strlen.instructions,
    );

    let main = m.functions.iter().find(|f| f.name == "main").unwrap();
    assert_eq!(main.return_type, "i64");
    assert!(
        main.instructions.iter().any(|i| i.op == "str_concat"),
        "derived let* actual should materialise through typed str_concat: {:?}",
        main.instructions,
    );
    assert!(
        main.instructions.iter().any(|i| {
            i.op == "call"
                && i.type_hint == "i64"
                && matches!(i.srcs.first(), Some(Operand::Var(name)) if name == "strlen")
        }),
        "direct call should inherit strlen's concrete return type: {:?}",
        main.instructions,
    );

    for (name, errs) in [
        ("wasm", iir_to_wasm::validate::validate_for_wasm(&m)),
        ("jvm", iir_to_jvm_class_file::validate::validate_for_jvm(&m)),
        (
            "clr",
            iir_to_cil_bytecode::validate::validate_iir_for_clr(&m),
        ),
    ] {
        assert!(
            errs.is_empty(),
            "[{name}] should accept a typed E4 string op over a derived let*-actual inferred string parameter; got {errs:?}",
            errs = errs
        );
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

/// Path-A increment 6a: `nil` literal and the implicit "empty
/// program returns nil" path now emit `const 0 [ref<LispyPair>]`
/// instead of `call_builtin "make_nil"`.  Every IIR-to-* backend
/// accepts the typed const (Phase 2 heap-lowering convention), so
/// these two minimal shapes flow through every backend.
#[test]
fn twig_nil_literal_accepted_by_every_backend() {
    let m = compile_source("nil", "compat").expect("Twig must compile");

    // The nil literal must emit `const 0 [ref<LispyPair>]`, not the
    // legacy `call_builtin "make_nil"`.
    let main = m.functions.iter().find(|f| f.name == "main").unwrap();
    assert!(
        main.instructions.iter().any(|i|
            i.op == "const" && i.type_hint == "ref<LispyPair>"
                && i.srcs[0] == Operand::Int(0)
        ),
        "increment 6a: `nil` must emit `const 0 [ref<LispyPair>]`; \
         got: {:?}", main.instructions,
    );
    assert!(
        !main.instructions.iter().any(|i|
            i.op == "call_builtin"
                && matches!(&i.srcs[0], Operand::Var(s) if s == "make_nil")
        ),
        "increment 6a: legacy `call_builtin \"make_nil\"` must be gone; \
         got: {:?}", main.instructions,
    );

    for (name, errs) in [
        ("wasm", iir_to_wasm::validate::validate_for_wasm(&m)),
        ("jvm",  iir_to_jvm_class_file::validate::validate_for_jvm(&m)),
        ("clr",  iir_to_cil_bytecode::validate::validate_iir_for_clr(&m)),
        ("beam", iir_to_beam::validate::validate_for_beam(&m)),
    ] {
        assert!(errs.is_empty(),
            "[{name}] validator should accept Twig `nil` after path-A \
             increment 6a; got {} error(s): {errs:?}",
            errs.len());
    }
}

/// Path-A increment 6a: the empty-program shape — no expressions, so
/// the compiler synthesises an implicit nil return — also flows
/// through every backend.
#[test]
fn twig_empty_program_accepted_by_every_backend() {
    let m = compile_source("", "compat").expect("Twig must compile");
    let main = m.functions.iter().find(|f| f.name == "main").unwrap();
    assert_eq!(main.return_type, "ref<LispyPair>",
        "empty program's main should return ref<LispyPair> (the nil tail)");

    for (name, errs) in [
        ("wasm", iir_to_wasm::validate::validate_for_wasm(&m)),
        ("jvm",  iir_to_jvm_class_file::validate::validate_for_jvm(&m)),
        ("clr",  iir_to_cil_bytecode::validate::validate_iir_for_clr(&m)),
        ("beam", iir_to_beam::validate::validate_for_beam(&m)),
    ] {
        assert!(errs.is_empty(),
            "[{name}] validator should accept empty Twig program; \
             got {} error(s): {errs:?}", errs.len());
    }
}

/// Path-A increment 6b: `cons` cells are now lowered to typed
/// `alloc` + `field_store` triples (matching the Phase 2 heap-lowering
/// convention).  Record/union constructors — which build cons chains
/// internally — now emit `alloc [ref<LispyPair>]` + two
/// `field_store [ref<LispyPair>]` per field instead of
/// `call_builtin "cons"`.
///
/// This test asserts the IR shape on the constructor function.  The
/// full-module backend validation is deferred to increment 6c, when
/// the matching `car`/`cdr` accessor functions also become typed.
#[test]
fn twig_record_constructor_emits_typed_alloc_and_field_store() {
    let m = compile_source(
        "(record Point (x : int) (y : int))",
        "compat",
    ).expect("Twig must compile");

    let point_fn = m.functions.iter().find(|f| f.name == "Point")
        .expect("Point constructor must be emitted");

    // The constructor must emit at least one typed `alloc` and at least
    // one typed `field_store`, and zero `call_builtin "cons"`.
    assert!(point_fn.instructions.iter().any(|i|
        i.op == "alloc" && i.type_hint == "ref<LispyPair>"),
        "Point constructor must emit typed `alloc [ref<LispyPair>]`; \
         got: {:?}", point_fn.instructions);
    assert!(point_fn.instructions.iter().any(|i|
        i.op == "field_store" && i.type_hint == "void"),
        "Point constructor must emit typed `field_store [void]` (matching \
         iir-builtin-lowering's Phase 2 convention); \
         got: {:?}", point_fn.instructions);
    assert!(
        !point_fn.instructions.iter().any(|i|
            i.op == "call_builtin"
                && matches!(&i.srcs[0], Operand::Var(s) if s == "cons")
        ),
        "increment 6b: legacy `call_builtin \"cons\"` must be gone from \
         the Point constructor; got: {:?}", point_fn.instructions,
    );

    // The constructor in isolation must satisfy every backend's IR
    // validator.  We build a single-function module and validate that.
    let mut constructor_only = m.clone();
    constructor_only.functions.retain(|f| f.name == "Point");
    for (name, errs) in [
        ("wasm", iir_to_wasm::validate::validate_for_wasm(&constructor_only)),
        ("jvm",  iir_to_jvm_class_file::validate::validate_for_jvm(&constructor_only)),
        ("clr",  iir_to_cil_bytecode::validate::validate_iir_for_clr(&constructor_only)),
        ("beam", iir_to_beam::validate::validate_for_beam(&constructor_only)),
    ] {
        assert!(errs.is_empty(),
            "[{name}] validator should accept the Point constructor \
             after path-A increment 6b; got {} error(s): {errs:?}",
            errs.len());
    }
}

/// Path-A increment 6c: `car` / `cdr` now lower to typed `field_load`
/// instead of `call_builtin "car"` / `"cdr"`.  Combined with 6a (nil)
/// and 6b (cons), the entire cons-cell vocabulary used by record /
/// union constructors and accessors is now typed, and the full record
/// program flows through every backend.
#[test]
fn twig_full_record_program_accepted_by_every_backend() {
    let m = compile_source(
        "(record Point (x : int) (y : int))",
        "compat",
    ).expect("Twig must compile");

    // The Point constructor and the point-x / point-y accessors should
    // contain typed `field_load [ref<any>]` (and zero `call_builtin
    // "car"` / `"cdr"`).
    let any_field_load = m.functions.iter()
        .flat_map(|f| f.instructions.iter())
        .any(|i| i.op == "field_load" && i.type_hint == "ref<any>");
    assert!(any_field_load,
        "increment 6c: expected at least one `field_load [ref<any>]` \
         in the record module");

    let leftover_car_cdr = m.functions.iter()
        .flat_map(|f| f.instructions.iter())
        .any(|i| i.op == "call_builtin" && matches!(
            &i.srcs[0], Operand::Var(s) if s == "car" || s == "cdr"
        ));
    assert!(!leftover_car_cdr,
        "increment 6c: legacy `call_builtin \"car\"`/\"cdr\" must be gone");

    // The full module must now satisfy every backend's validator.  We
    // exclude the `pair?` predicate function, which still uses
    // `call_builtin "pair?"` (out of scope for 6c).
    let mut without_pair_pred = m.clone();
    without_pair_pred.functions.retain(|f| !f.name.ends_with('?'));
    for (name, errs) in [
        ("wasm", iir_to_wasm::validate::validate_for_wasm(&without_pair_pred)),
        ("jvm",  iir_to_jvm_class_file::validate::validate_for_jvm(&without_pair_pred)),
        ("clr",  iir_to_cil_bytecode::validate::validate_iir_for_clr(&without_pair_pred)),
        ("beam", iir_to_beam::validate::validate_for_beam(&without_pair_pred)),
    ] {
        assert!(errs.is_empty(),
            "[{name}] validator should accept Twig record program \
             (constructor + accessors, predicate excluded) after path-A \
             increment 6c; got {} error(s): {errs:?}",
            errs.len());
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

/// E6d-6b: a union constructor must store its tag + fields **boxed** so that
/// `match` (which reads them back as boxed `DynValue`s and `unbox`es) round-trips
/// on the tagged backends. Concretely: every value written into the variant's
/// cons chain — the tag word and each field — is the result of a `box` op, not a
/// raw register. Without this the tagged backends `unbox` a raw word (`unbox(42)=5`).
#[test]
fn union_constructor_boxes_tag_and_fields() {
    let m = compile_source(
        "(union Opt (Some (v : int)) (None)) (match (Some 42) ((Some v) v) ((None) 0))",
        "u",
    )
    .expect("union program must compile");

    let some = m.functions.iter().find(|f| f.name == "Some").expect("constructor Some");

    // The set of registers produced by a `box` op in this function.
    let boxed: std::collections::HashSet<&str> = some
        .instructions
        .iter()
        .filter(|i| i.op == "box")
        .filter_map(|i| i.dest.as_deref())
        .collect();
    // `Some(v)` has one field + one tag ⇒ at least two `box`es.
    assert!(boxed.len() >= 2, "Some must box the tag and its field; boxes = {boxed:?}");

    // Every `field_store` that writes the car (index 0 — a value slot, not the
    // cdr link) must store a boxed register.
    for i in some.instructions.iter().filter(|i| i.op == "field_store") {
        let is_car = matches!(i.srcs.get(1), Some(Operand::Int(0)));
        if !is_car {
            continue; // cdr link (index 1) carries a cons pointer, not a boxed value
        }
        match i.srcs.get(2) {
            Some(Operand::Var(v)) => assert!(
                boxed.contains(v.as_str()),
                "field_store car must store a boxed value; stored {v:?}, boxes = {boxed:?}"
            ),
            other => panic!("field_store car value operand unexpected: {other:?}"),
        }
    }
}
