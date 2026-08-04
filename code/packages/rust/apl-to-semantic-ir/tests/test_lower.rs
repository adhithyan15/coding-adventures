use apl_to_semantic_ir::compile_source;
use semantic_ir::{ElementwiseOpKind, Expr, Feature, Function, Module, Stmt};

fn compile_ok(src: &str) -> Module {
    compile_source(src, "prog").unwrap_or_else(|e| panic!("expected lowering to succeed: {e}"))
}

fn compile_err(src: &str) -> String {
    compile_source(src, "prog")
        .err()
        .unwrap_or_else(|| panic!("expected lowering to fail for `{src}`"))
        .message
}

fn main_fn(m: &Module) -> &Function {
    m.functions.iter().find(|f| f.name == "main").expect("main function")
}

/// The `Expr` inside the sole `print(...)` wrapper of a bare-expression
/// top-level statement (see the "Auto-print" design in `lower.rs`).
fn printed_value(stmt: &Stmt) -> &Expr {
    match stmt {
        Stmt::ExprStmt {
            expr: Expr::BuiltinCall { name, args, .. },
            ..
        } if name == "print" => {
            assert_eq!(args.len(), 1, "print should take exactly one argument");
            &args[0]
        }
        other => panic!("expected ExprStmt(print(..)), got {other:?}"),
    }
}

// ── literals ─────────────────────────────────────────────────────────────

#[test]
fn bare_int_literal_is_wrapped_in_print() {
    let m = compile_ok("5\n");
    let main = main_fn(&m);
    assert_eq!(main.body.stmts.len(), 1);
    assert!(matches!(printed_value(&main.body.stmts[0]), Expr::IntLit { value: 5, .. }));
}

#[test]
fn float_literal_is_recognised_by_decimal_point() {
    let m = compile_ok("2.5\n");
    let main = main_fn(&m);
    match printed_value(&main.body.stmts[0]) {
        Expr::FloatLit { value, .. } => assert!((*value - 2.5).abs() < 1e-9),
        other => panic!("expected FloatLit, got {other:?}"),
    }
    assert!(m.manifest.iter().any(|f| f == Feature::Floats));
}

#[test]
fn high_minus_negative_integer_literal() {
    let m = compile_ok("¯3\n");
    let main = main_fn(&m);
    assert!(matches!(printed_value(&main.body.stmts[0]), Expr::IntLit { value: -3, .. }));
}

#[test]
fn high_minus_negative_float_literal() {
    let m = compile_ok("¯3.5\n");
    let main = main_fn(&m);
    match printed_value(&main.body.stmts[0]) {
        Expr::FloatLit { value, .. } => assert!((*value - (-3.5)).abs() < 1e-9),
        other => panic!("expected FloatLit, got {other:?}"),
    }
}

#[test]
fn stranded_literal_is_a_ravelled_single_row_array_lit_rank_1_vector() {
    // A stranded literal must lower to a genuine rank-1 vector, not the
    // genuinely rank-2 `[1, n]` "row vector" a bare single-row `ArrayLit`
    // represents on its own (see `semantic-ir`'s own `ArrayLit` doc comment
    // in `nodes.rs`). The frontend achieves this by wrapping the single-row
    // `ArrayLit` in `Expr::Ravel`, which flattens it down to a true rank-1
    // result -- see `lower_term`'s doc comment in `lower.rs` for the full
    // explanation of why the bare `ArrayLit` alone was the wrong shape.
    let m = compile_ok("1 2 3\n");
    let main = main_fn(&m);
    match printed_value(&main.body.stmts[0]) {
        Expr::Ravel { target, .. } => match &**target {
            Expr::ArrayLit { rows, .. } => {
                assert_eq!(rows.len(), 1, "stranded literal's inner ArrayLit must be a single row");
                assert_eq!(rows[0].len(), 3);
                assert!(matches!(rows[0][0], Expr::IntLit { value: 1, .. }));
                assert!(matches!(rows[0][1], Expr::IntLit { value: 2, .. }));
                assert!(matches!(rows[0][2], Expr::IntLit { value: 3, .. }));
            }
            other => panic!("expected Ravel's target to be ArrayLit, got {other:?}"),
        },
        other => panic!("expected Ravel wrapping ArrayLit, got {other:?}"),
    }
    assert!(m.manifest.iter().any(|f| f == Feature::NDArrays));
    assert!(m.manifest.iter().any(|f| f == Feature::ArrayColumnMajor));
    assert!(m.manifest.iter().any(|f| f == Feature::MatrixOps));
}

#[test]
fn a_lone_number_is_a_scalar_not_an_array_lit() {
    let m = compile_ok("5\n");
    let main = main_fn(&m);
    assert!(matches!(printed_value(&main.body.stmts[0]), Expr::IntLit { .. }));
}

#[test]
fn parenthesised_grouping() {
    // (1+2)×3 -- the group is purely syntactic, no wrapper node.
    let m = compile_ok("(1+2)×3\n");
    let main = main_fn(&m);
    match printed_value(&main.body.stmts[0]) {
        Expr::ElementwiseOp { op: ElementwiseOpKind::Mul, lhs, rhs, .. } => {
            assert!(matches!(**lhs, Expr::ElementwiseOp { op: ElementwiseOpKind::Add, .. }));
            assert!(matches!(**rhs, Expr::IntLit { value: 3, .. }));
        }
        other => panic!("expected ElementwiseOp(Mul, ...), got {other:?}"),
    }
}

// ── all 12 dyadic scalar atoms ───────────────────────────────────────────

#[test]
fn every_dyadic_atom_lowers_to_elementwise_op() {
    let cases: &[(&str, ElementwiseOpKind)] = &[
        ("+", ElementwiseOpKind::Add),
        ("-", ElementwiseOpKind::Sub),
        ("×", ElementwiseOpKind::Mul),
        ("÷", ElementwiseOpKind::Div),
        ("⌈", ElementwiseOpKind::Max),
        ("⌊", ElementwiseOpKind::Min),
        ("=", ElementwiseOpKind::Eq),
        ("≠", ElementwiseOpKind::Ne),
        ("<", ElementwiseOpKind::Lt),
        ("≤", ElementwiseOpKind::Le),
        ("≥", ElementwiseOpKind::Ge),
        (">", ElementwiseOpKind::Gt),
    ];
    for (glyph, expected) in cases {
        let src = format!("3{glyph}4\n");
        let m = compile_ok(&src);
        let main = main_fn(&m);
        match printed_value(&main.body.stmts[0]) {
            Expr::ElementwiseOp { op, lhs, rhs, .. } => {
                assert_eq!(op, expected, "glyph `{glyph}` mapped to the wrong op");
                assert!(matches!(**lhs, Expr::IntLit { value: 3, .. }));
                assert!(matches!(**rhs, Expr::IntLit { value: 4, .. }));
            }
            other => panic!("`{src}`: expected ElementwiseOp, got {other:?}"),
        }
    }
}

#[test]
fn dyadic_atoms_lower_identically_for_literals_and_variables() {
    // Point 1 of the module doc: no scalar/array disambiguation -- `A+B`
    // and `3+4` produce the exact same node shape (ElementwiseOp), unlike
    // matlab-to-semantic-ir's literal-folding fast path.
    let m = compile_ok("A←3\nB←4\nA+B\n");
    let main = main_fn(&m);
    // stmts: [LetStarBinding(A,3), LetStarBinding(B,4), ExprStmt(print(...))]
    assert_eq!(main.body.stmts.len(), 3);
    match printed_value(&main.body.stmts[2]) {
        Expr::ElementwiseOp { op: ElementwiseOpKind::Add, lhs, rhs, .. } => {
            assert!(matches!(**lhs, Expr::VarRef { ref name, .. } if name == "A"));
            assert!(matches!(**rhs, Expr::VarRef { ref name, .. } if name == "B"));
        }
        other => panic!("expected ElementwiseOp, got {other:?}"),
    }
    assert!(m.manifest.iter().any(|f| f == Feature::MatrixOps));
    assert!(m.manifest.iter().any(|f| f == Feature::ArrayColumnMajor));
}

// ── monadic atoms: 6 valid, 6 invalid ────────────────────────────────────

#[test]
fn monadic_plus_is_identity_no_wrapping() {
    let m = compile_ok("+1\n");
    let main = main_fn(&m);
    // No BuiltinCall wrapper at all -- the operand passes through unchanged.
    assert!(matches!(printed_value(&main.body.stmts[0]), Expr::IntLit { value: 1, .. }));
}

#[test]
fn monadic_minus_is_neg_builtin() {
    let m = compile_ok("-1\n");
    let main = main_fn(&m);
    match printed_value(&main.body.stmts[0]) {
        Expr::BuiltinCall { name, args, .. } => {
            assert_eq!(name, "neg");
            assert!(matches!(args[0], Expr::IntLit { value: 1, .. }));
        }
        other => panic!("expected BuiltinCall(\"neg\", ..), got {other:?}"),
    }
}

#[test]
fn monadic_times_is_sign_builtin() {
    let m = compile_ok("×1\n");
    let main = main_fn(&m);
    match printed_value(&main.body.stmts[0]) {
        Expr::BuiltinCall { name, .. } => assert_eq!(name, "sign"),
        other => panic!("expected BuiltinCall(\"sign\", ..), got {other:?}"),
    }
}

#[test]
fn monadic_divide_is_recip_builtin() {
    let m = compile_ok("÷1\n");
    let main = main_fn(&m);
    match printed_value(&main.body.stmts[0]) {
        Expr::BuiltinCall { name, .. } => assert_eq!(name, "recip"),
        other => panic!("expected BuiltinCall(\"recip\", ..), got {other:?}"),
    }
}

#[test]
fn monadic_ceiling_is_ceil_builtin() {
    let m = compile_ok("⌈1\n");
    let main = main_fn(&m);
    match printed_value(&main.body.stmts[0]) {
        Expr::BuiltinCall { name, .. } => assert_eq!(name, "ceil"),
        other => panic!("expected BuiltinCall(\"ceil\", ..), got {other:?}"),
    }
}

#[test]
fn monadic_floor_is_floor_builtin() {
    let m = compile_ok("⌊1\n");
    let main = main_fn(&m);
    match printed_value(&main.body.stmts[0]) {
        Expr::BuiltinCall { name, .. } => assert_eq!(name, "floor"),
        other => panic!("expected BuiltinCall(\"floor\", ..), got {other:?}"),
    }
}

#[test]
fn the_six_comparison_atoms_have_no_monadic_form() {
    for glyph in ["=", "≠", "<", "≤", "≥", ">"] {
        let src = format!("{glyph}1\n");
        let err = compile_err(&src);
        assert!(
            err.contains("no monadic form"),
            "`{src}` should reject with a clean 'no monadic form' error, got: {err}"
        );
    }
}

// ── reduce / scan (monadic-only) ─────────────────────────────────────────

#[test]
fn reduce_over_stranded_vector() {
    let m = compile_ok("+/1 2 3\n");
    let main = main_fn(&m);
    match printed_value(&main.body.stmts[0]) {
        Expr::Reduce { op, target, .. } => {
            assert_eq!(*op, ElementwiseOpKind::Add);
            // The stranded literal `1 2 3` now lowers to a `Ravel`-wrapped
            // `ArrayLit` (a genuine rank-1 vector), not a bare `ArrayLit`.
            assert!(matches!(**target, Expr::Ravel { .. }));
        }
        other => panic!("expected Reduce, got {other:?}"),
    }
    assert!(m.manifest.iter().any(|f| f == Feature::MatrixOps));
    assert!(m.manifest.iter().any(|f| f == Feature::ArrayColumnMajor));
}

#[test]
fn scan_over_stranded_vector() {
    let m = compile_ok("×\\1 2 3\n");
    let main = main_fn(&m);
    match printed_value(&main.body.stmts[0]) {
        Expr::Scan { op, .. } => assert_eq!(*op, ElementwiseOpKind::Mul),
        other => panic!("expected Scan, got {other:?}"),
    }
}

#[test]
fn reduce_used_dyadically_is_rejected() {
    let err = compile_err("3+/4\n");
    assert!(
        err.contains("reduce") && err.contains("dyadically"),
        "expected a clean dyadic-reduce rejection, got: {err}"
    );
}

#[test]
fn scan_used_dyadically_is_rejected() {
    let err = compile_err("3×\\4\n");
    assert!(
        err.contains("scan") && err.contains("dyadically"),
        "expected a clean dyadic-scan rejection, got: {err}"
    );
}

// ── outer product (dyadic-only) ──────────────────────────────────────────

#[test]
fn outer_product_dyadic_use() {
    let m = compile_ok("1∘.×2\n");
    let main = main_fn(&m);
    match printed_value(&main.body.stmts[0]) {
        Expr::OuterProduct { op, lhs, rhs, .. } => {
            assert_eq!(*op, ElementwiseOpKind::Mul);
            assert!(matches!(**lhs, Expr::IntLit { value: 1, .. }));
            assert!(matches!(**rhs, Expr::IntLit { value: 2, .. }));
        }
        other => panic!("expected OuterProduct, got {other:?}"),
    }
}

#[test]
fn outer_product_used_monadically_is_rejected() {
    let err = compile_err("∘.×1\n");
    assert!(
        err.contains("outer product") && err.contains("monadically"),
        "expected a clean monadic-outer rejection, got: {err}"
    );
}

#[test]
fn outer_product_accepts_two_bare_stranded_literals_as_genuine_rank_1_operands() {
    // Regression test for the stranded-literal-rank bug: before the fix, a
    // bare stranded literal lowered to a bare (genuinely rank-2, `[1, n]`)
    // `ArrayLit`, and `outer` is scoped to rank <= 1 operands only -- so
    // `1 2∘.×3 4` would compile and validate fine here (this crate's Rust
    // side never rank-checks; only `semantic-ir-to-javascript`'s runtime
    // does), but throw at `node` runtime. See `tests/e2e_node.rs`'s
    // `outer_product_of_two_bare_stranded_literals_runs_in_node` for the
    // actual node-executed proof; this test only checks the IR shape: both
    // operands must be `Expr::Ravel` (not a bare `Expr::ArrayLit`), which is
    // what makes them genuinely rank-1 by construction.
    let m = compile_ok("1 2∘.×3 4\n");
    let main = main_fn(&m);
    match printed_value(&main.body.stmts[0]) {
        Expr::OuterProduct { op, lhs, rhs, .. } => {
            assert_eq!(*op, ElementwiseOpKind::Mul);
            assert!(matches!(**lhs, Expr::Ravel { .. }), "lhs must be Ravel-wrapped, not bare ArrayLit");
            assert!(matches!(**rhs, Expr::Ravel { .. }), "rhs must be Ravel-wrapped, not bare ArrayLit");
        }
        other => panic!("expected OuterProduct, got {other:?}"),
    }
    let report = semantic_ir::validate(&m);
    assert!(report.is_ok(), "expected clean validation, got: {:?}", report.errors().collect::<Vec<_>>());
}

// ── ⍴ (shape / reshape) ───────────────────────────────────────────────────

#[test]
fn monadic_rho_is_shape() {
    let m = compile_ok("⍴1\n");
    let main = main_fn(&m);
    assert!(matches!(printed_value(&main.body.stmts[0]), Expr::Shape { .. }));
}

#[test]
fn dyadic_rho_is_reshape_with_a_as_shape_and_b_as_target() {
    // 2 3⍴1 -- shape=[2,3] (LHS), target=1 (RHS).
    let m = compile_ok("2 3⍴1\n");
    let main = main_fn(&m);
    match printed_value(&main.body.stmts[0]) {
        Expr::Reshape { shape, target, .. } => {
            // The stranded literal `2 3` now lowers to a `Ravel`-wrapped
            // `ArrayLit` (a genuine rank-1 vector), not a bare `ArrayLit`.
            assert!(matches!(**shape, Expr::Ravel { .. }));
            assert!(matches!(**target, Expr::IntLit { value: 1, .. }));
        }
        other => panic!("expected Reshape, got {other:?}"),
    }
}

#[test]
fn reshape_accepts_bare_stranded_literal_shape_and_target_as_genuine_rank_1_operands() {
    // Regression test for the stranded-literal-rank bug: before the fix, a
    // bare stranded literal used as dyadic `⍴`'s shape ARGUMENT (here, `2
    // 3`) lowered to a bare (genuinely rank-2, `[1, n]`) `ArrayLit`, and
    // `reshape` requires its shape argument to be rank <= 1 -- so this
    // would compile and validate fine here (rank-checking is a
    // `semantic-ir-to-javascript` runtime concern, not this crate's), but
    // throw at `node` runtime. See `tests/e2e_node.rs`'s
    // `reshape_with_bare_stranded_literal_shape_and_target_runs_in_node`
    // for the actual node-executed proof; this test only checks the IR
    // shape: the shape argument must be `Expr::Ravel` (not a bare
    // `Expr::ArrayLit`), which is what makes it genuinely rank-1 by
    // construction.
    let m = compile_ok("2 3⍴1 2 3 4 5 6\n");
    let main = main_fn(&m);
    match printed_value(&main.body.stmts[0]) {
        Expr::Reshape { shape, target, .. } => {
            assert!(matches!(**shape, Expr::Ravel { .. }), "shape must be Ravel-wrapped, not bare ArrayLit");
            assert!(matches!(**target, Expr::Ravel { .. }), "target must be Ravel-wrapped, not bare ArrayLit");
        }
        other => panic!("expected Reshape, got {other:?}"),
    }
    let report = semantic_ir::validate(&m);
    assert!(report.is_ok(), "expected clean validation, got: {:?}", report.errors().collect::<Vec<_>>());
}

// ── ⍳ (index generator / index-of) ───────────────────────────────────────

#[test]
fn monadic_iota_is_index_generator() {
    let m = compile_ok("⍳3\n");
    let main = main_fn(&m);
    match printed_value(&main.body.stmts[0]) {
        Expr::IndexGenerator { count, .. } => assert!(matches!(**count, Expr::IntLit { value: 3, .. })),
        other => panic!("expected IndexGenerator, got {other:?}"),
    }
}

#[test]
fn dyadic_iota_is_index_of_with_a_as_haystack_and_b_as_needle() {
    let m = compile_ok("3⍳1\n");
    let main = main_fn(&m);
    match printed_value(&main.body.stmts[0]) {
        Expr::IndexOf { haystack, needle, .. } => {
            assert!(matches!(**haystack, Expr::IntLit { value: 3, .. }));
            assert!(matches!(**needle, Expr::IntLit { value: 1, .. }));
        }
        other => panic!("expected IndexOf, got {other:?}"),
    }
}

// ── , (ravel / catenate) ──────────────────────────────────────────────────

#[test]
fn monadic_ravel() {
    let m = compile_ok(",1\n");
    let main = main_fn(&m);
    match printed_value(&main.body.stmts[0]) {
        Expr::Ravel { target, .. } => assert!(matches!(**target, Expr::IntLit { value: 1, .. })),
        other => panic!("expected Ravel, got {other:?}"),
    }
}

#[test]
fn dyadic_catenate() {
    let m = compile_ok("1,2\n");
    let main = main_fn(&m);
    match printed_value(&main.body.stmts[0]) {
        Expr::Catenate { lhs, rhs, .. } => {
            assert!(matches!(**lhs, Expr::IntLit { value: 1, .. }));
            assert!(matches!(**rhs, Expr::IntLit { value: 2, .. }));
        }
        other => panic!("expected Catenate, got {other:?}"),
    }
}

// ── ⍴/⍳/, rejecting operator decoration ──────────────────────────────────

#[test]
fn rho_decorated_with_reduce_is_rejected() {
    let err = compile_err("⍴/1\n");
    assert!(
        err.contains("⍴") && err.contains("scalar dyadic function"),
        "expected a clean rejection naming ⍴, got: {err}"
    );
}

#[test]
fn iota_decorated_with_scan_is_rejected() {
    let err = compile_err("⍳\\1\n");
    assert!(err.contains("⍳"), "expected a clean rejection naming ⍳, got: {err}");
}

#[test]
fn ravel_decorated_with_outer_is_rejected() {
    let err = compile_err("∘.,1 2\n");
    assert!(err.contains(","), "expected a clean rejection naming ',', got: {err}");
}

// ── assignment: first occurrence vs. reassignment ────────────────────────

#[test]
fn first_assignment_is_a_let_star_binding() {
    let m = compile_ok("A←5\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { name, value, .. } => {
            assert_eq!(name, "A");
            assert!(matches!(value, Expr::IntLit { value: 5, .. }));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
    // A pure assignment prints nothing.
    assert_eq!(main.body.stmts.len(), 1);
}

#[test]
fn reassignment_is_assign_not_let() {
    let m = compile_ok("A←1\nA←2\n");
    let main = main_fn(&m);
    assert!(matches!(main.body.stmts[0], Stmt::LetStarBinding { .. }));
    assert!(matches!(main.body.stmts[1], Stmt::Assign { .. }));
    assert!(m.manifest.iter().any(|f| f == Feature::MutableBindings));
}

#[test]
fn chained_assignment_unrolls_into_two_statements_in_dependency_order() {
    // A←B←3 -- per the module doc's "Chained assignment" design: B is bound
    // first (to 3), then A is bound to a VarRef("B"), not a duplicated 3.
    let m = compile_ok("A←B←3\n");
    let main = main_fn(&m);
    assert_eq!(main.body.stmts.len(), 2);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { name, value, .. } => {
            assert_eq!(name, "B");
            assert!(matches!(value, Expr::IntLit { value: 3, .. }));
        }
        other => panic!("expected LetStarBinding(B, 3), got {other:?}"),
    }
    match &main.body.stmts[1] {
        Stmt::LetStarBinding { name, value, .. } => {
            assert_eq!(name, "A");
            assert!(matches!(value, Expr::VarRef { name, .. } if name == "B"));
        }
        other => panic!("expected LetStarBinding(A, VarRef(B)), got {other:?}"),
    }
}

// ── undefined variable ────────────────────────────────────────────────────

#[test]
fn referencing_an_undefined_variable_is_a_clean_error() {
    let err = compile_err("A+1\n");
    assert!(err.contains("undefined variable"), "got: {err}");
}

#[test]
fn self_referential_first_assignment_is_rejected() {
    // A←A+1 as the very first use of A: the RHS is lowered before A is bound
    // (mirrors apl-runtime's own runtime evaluation order), so this is
    // "undefined variable", not a self-reference.
    let err = compile_err("A←A+1\n");
    assert!(err.contains("undefined variable"), "got: {err}");
}

// ── parse-error propagation ────────────────────────────────────────────────

#[test]
fn a_parse_error_is_mapped_into_an_apl_lower_error() {
    let err = compile_source("←←←\n", "prog");
    assert!(err.is_err());
    let e = err.unwrap_err();
    assert!(e.message.contains("parse error"));
    assert_eq!(e.line, 1);
    assert_eq!(e.column, 1);
}

// ── full end-to-end: multi-line program validates cleanly ────────────────

#[test]
fn a_multi_line_program_compiles_and_validates_cleanly() {
    let src = "A←3\nB←4\nA+B\n+/1 2 3\nA←A+1\n";
    let m = compile_ok(src);
    let report = semantic_ir::validate(&m);
    assert!(
        report.is_ok(),
        "expected the lowered module to validate cleanly, got: {:?}",
        report.errors().collect::<Vec<_>>()
    );

    let main = main_fn(&m);
    // A←3, B←4, print(A+B), print(+/1 2 3), A←A+1 (reassignment)
    assert_eq!(main.body.stmts.len(), 5);
    assert!(matches!(main.body.stmts[0], Stmt::LetStarBinding { .. }));
    assert!(matches!(main.body.stmts[1], Stmt::LetStarBinding { .. }));
    assert!(matches!(main.body.stmts[4], Stmt::Assign { .. }));
    assert!(m.manifest.iter().any(|f| f == Feature::MutableBindings));
    assert!(m.manifest.iter().any(|f| f == Feature::DynamicTyping));
}

#[test]
fn an_empty_program_compiles_and_validates_cleanly() {
    let m = compile_ok("");
    let report = semantic_ir::validate(&m);
    assert!(report.is_ok(), "empty program should validate cleanly: {:?}", report.issues);
    assert!(main_fn(&m).body.stmts.is_empty());
}

#[test]
fn comment_and_blank_lines_are_skipped_not_errors() {
    let m = compile_ok("⍝ just a comment\n\nA←1\n");
    let main = main_fn(&m);
    assert_eq!(main.body.stmts.len(), 1);
    assert!(matches!(main.body.stmts[0], Stmt::LetStarBinding { .. }));
}
