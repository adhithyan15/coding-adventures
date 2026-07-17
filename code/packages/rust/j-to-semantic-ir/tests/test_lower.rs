use j_to_semantic_ir::compile_source;
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
        Stmt::ExprStmt { expr: Expr::BuiltinCall { name, args, .. }, .. } if name == "print" => {
            assert_eq!(args.len(), 1, "print should take exactly one argument");
            &args[0]
        }
        other => panic!("expected ExprStmt(print(..)), got {other:?}"),
    }
}

fn builtin_call<'a>(expr: &'a Expr, expected_name: &str) -> &'a [Expr] {
    match expr {
        Expr::BuiltinCall { name, args, .. } if name == expected_name => args,
        other => panic!("expected BuiltinCall({expected_name:?}, ..), got {other:?}"),
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
fn underscore_negative_integer_literal() {
    let m = compile_ok("_3\n");
    let main = main_fn(&m);
    assert!(matches!(printed_value(&main.body.stmts[0]), Expr::IntLit { value: -3, .. }));
}

#[test]
fn underscore_negative_float_literal_with_underscore_exponent() {
    // 1.5E_3 = 1.5e-3 -- both the mantissa-adjacent sign and the exponent's
    // own sign use the same underscore convention (j.tokens).
    let m = compile_ok("1.5E_3\n");
    let main = main_fn(&m);
    match printed_value(&main.body.stmts[0]) {
        Expr::FloatLit { value, .. } => assert!((*value - 1.5e-3).abs() < 1e-12),
        other => panic!("expected FloatLit, got {other:?}"),
    }
}

#[test]
fn stranded_literal_is_a_single_row_array_lit() {
    let m = compile_ok("1 2 3\n");
    let main = main_fn(&m);
    match printed_value(&main.body.stmts[0]) {
        Expr::ArrayLit { rows, .. } => {
            assert_eq!(rows.len(), 1, "stranded literal must be a single row (rank-1 vector)");
            assert_eq!(rows[0].len(), 3);
            assert!(matches!(rows[0][0], Expr::IntLit { value: 1, .. }));
            assert!(matches!(rows[0][1], Expr::IntLit { value: 2, .. }));
            assert!(matches!(rows[0][2], Expr::IntLit { value: 3, .. }));
        }
        other => panic!("expected ArrayLit, got {other:?}"),
    }
    assert!(m.manifest.iter().any(|f| f == Feature::NDArrays));
    assert!(m.manifest.iter().any(|f| f == Feature::ArrayColumnMajor));
}

#[test]
fn parenthesised_grouping() {
    let m = compile_ok("(1+2)*3\n");
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
        ("*", ElementwiseOpKind::Mul),
        ("%", ElementwiseOpKind::Div),
        ("<.", ElementwiseOpKind::Min),
        (">.", ElementwiseOpKind::Max),
        ("=", ElementwiseOpKind::Eq),
        ("~:", ElementwiseOpKind::Ne),
        ("<", ElementwiseOpKind::Lt),
        (">", ElementwiseOpKind::Gt),
        ("<:", ElementwiseOpKind::Le),
        (">:", ElementwiseOpKind::Ge),
    ];
    for (glyph, op) in cases {
        let src = format!("3{glyph}4\n");
        let m = compile_ok(&src);
        let main = main_fn(&m);
        match printed_value(&main.body.stmts[0]) {
            Expr::ElementwiseOp { op: got, .. } => {
                assert_eq!(got, op, "wrong op for `{glyph}`")
            }
            other => panic!("`{glyph}`: expected ElementwiseOp, got {other:?}"),
        }
    }
}

// ── monadic scalar atoms ─────────────────────────────────────────────────

#[test]
fn conjugate_plus_is_a_pass_through_no_op() {
    let m = compile_ok("+3\n");
    let main = main_fn(&m);
    assert!(matches!(printed_value(&main.body.stmts[0]), Expr::IntLit { value: 3, .. }));
}

#[test]
fn monadic_atoms_map_onto_well_known_builtins() {
    let cases: &[(&str, &str)] =
        &[("-", "neg"), ("*", "sign"), ("%", "recip"), (">.", "ceil"), ("<.", "floor")];
    for (glyph, name) in cases {
        let src = format!("{glyph}3\n");
        let m = compile_ok(&src);
        let main = main_fn(&m);
        let args = builtin_call(printed_value(&main.body.stmts[0]), name);
        assert!(matches!(args[0], Expr::IntLit { value: 3, .. }));
    }
}

#[test]
fn comparison_atoms_have_no_monadic_form() {
    for glyph in ["=", "~:", "<", ">", "<:", ">:"] {
        let src = format!("{glyph}3\n");
        let msg = compile_err(&src);
        assert!(msg.contains("no monadic form"), "`{glyph}`: {msg}");
    }
}

// ── $ i. , (shared with APL) ─────────────────────────────────────────────

#[test]
fn dollar_shape_monadic_and_reshape_dyadic() {
    let m = compile_ok("$3\n");
    assert!(matches!(printed_value(&main_fn(&m).body.stmts[0]), Expr::Shape { .. }));

    let m = compile_ok("2$1 2 3 4\n");
    match printed_value(&main_fn(&m).body.stmts[0]) {
        Expr::Reshape { shape, target, .. } => {
            assert!(matches!(**shape, Expr::IntLit { value: 2, .. }));
            assert!(matches!(**target, Expr::ArrayLit { .. }));
        }
        other => panic!("expected Reshape, got {other:?}"),
    }
}

#[test]
fn idot_index_generator_monadic_and_index_of_dyadic() {
    let m = compile_ok("i.5\n");
    assert!(matches!(printed_value(&main_fn(&m).body.stmts[0]), Expr::IndexGenerator { .. }));

    let m = compile_ok("(1 2 3)i.2\n");
    assert!(matches!(printed_value(&main_fn(&m).body.stmts[0]), Expr::IndexOf { .. }));
}

#[test]
fn ravel_monadic_and_catenate_dyadic() {
    let m = compile_ok(",3\n");
    assert!(matches!(printed_value(&main_fn(&m).body.stmts[0]), Expr::Ravel { .. }));

    let m = compile_ok("(1 2),(3 4)\n");
    assert!(matches!(printed_value(&main_fn(&m).body.stmts[0]), Expr::Catenate { .. }));
}

// ── # and ^ (genuinely new, no APL analogue) ─────────────────────────────

#[test]
fn hash_tally_monadic_and_replicate_dyadic() {
    let m = compile_ok("#1 2 3\n");
    let args = builtin_call(printed_value(&main_fn(&m).body.stmts[0]), "tally");
    assert!(matches!(args[0], Expr::ArrayLit { .. }));

    let m = compile_ok("2#3\n");
    let args = builtin_call(printed_value(&main_fn(&m).body.stmts[0]), "replicate");
    assert_eq!(args.len(), 2);
    assert!(matches!(args[0], Expr::IntLit { value: 2, .. }));
    assert!(matches!(args[1], Expr::IntLit { value: 3, .. }));
}

#[test]
fn caret_exp_monadic_and_pow_dyadic() {
    let m = compile_ok("^3\n");
    let args = builtin_call(printed_value(&main_fn(&m).body.stmts[0]), "exp");
    assert!(matches!(args[0], Expr::IntLit { value: 3, .. }));

    let m = compile_ok("2^3\n");
    match printed_value(&main_fn(&m).body.stmts[0]) {
        Expr::ElementwiseOp { op: ElementwiseOpKind::Pow, lhs, rhs, .. } => {
            assert!(matches!(**lhs, Expr::IntLit { value: 2, .. }));
            assert!(matches!(**rhs, Expr::IntLit { value: 3, .. }));
        }
        other => panic!("expected ElementwiseOp(Pow, ...), got {other:?}"),
    }
}

// ── reduce / scan ────────────────────────────────────────────────────────

#[test]
fn reduce_and_scan_are_monadic_only() {
    let m = compile_ok("+/1 2 3\n");
    match printed_value(&main_fn(&m).body.stmts[0]) {
        Expr::Reduce { op: ElementwiseOpKind::Add, .. } => {}
        other => panic!("expected Reduce(Add, ...), got {other:?}"),
    }

    let m = compile_ok("+\\1 2 3\n");
    match printed_value(&main_fn(&m).body.stmts[0]) {
        Expr::Scan { op: ElementwiseOpKind::Add, .. } => {}
        other => panic!("expected Scan(Add, ...), got {other:?}"),
    }
}

#[test]
fn reduce_used_dyadically_is_rejected() {
    let msg = compile_err("3+/4\n");
    assert!(msg.contains("reduce"), "{msg}");
}

#[test]
fn scan_used_dyadically_is_rejected() {
    let msg = compile_err("3+\\4\n");
    assert!(msg.contains("scan"), "{msg}");
}

#[test]
fn non_scalar_verbs_cannot_take_reduce_or_scan() {
    for glyph in ["$", "i.", ",", "#", "^"] {
        let src = format!("{glyph}/1 2 3\n");
        let msg = compile_err(&src);
        assert!(msg.contains("not a scalar dyadic verb"), "`{glyph}`: {msg}");
    }
}

// ── compose (@) ──────────────────────────────────────────────────────────

#[test]
fn compose_monadic_applies_right_then_left() {
    // `@` composes two bare verb_exprs directly -- no parens needed (parens
    // are only for the LPAREN verb_train RPAREN train alternative).
    // -@%2 = -(%2) = neg(recip(2))
    let m = compile_ok("-@%2\n");
    let outer = builtin_call(printed_value(&main_fn(&m).body.stmts[0]), "neg");
    let inner = builtin_call(&outer[0], "recip");
    assert!(matches!(inner[0], Expr::IntLit { value: 2, .. }));
}

#[test]
fn compose_dyadic_applies_g_then_f_monadically() {
    // x (f@g) y = f (x g y): 3-@+4 = -(3+4) = neg(3+4)
    let m = compile_ok("3-@+4\n");
    let outer = builtin_call(printed_value(&main_fn(&m).body.stmts[0]), "neg");
    match &outer[0] {
        Expr::ElementwiseOp { op: ElementwiseOpKind::Add, lhs, rhs, .. } => {
            assert!(matches!(**lhs, Expr::IntLit { value: 3, .. }));
            assert!(matches!(**rhs, Expr::IntLit { value: 4, .. }));
        }
        other => panic!("expected ElementwiseOp(Add, ...), got {other:?}"),
    }
}

// ── hooks ────────────────────────────────────────────────────────────────

#[test]
fn hook_monadic_is_y_f_g_y() {
    // (+ *) 3 = 3 + (*3) = ElementwiseOp(Add, 3, BuiltinCall(sign, 3))
    let m = compile_ok("(+*)3\n");
    match printed_value(&main_fn(&m).body.stmts[0]) {
        Expr::ElementwiseOp { op: ElementwiseOpKind::Add, lhs, rhs, .. } => {
            assert!(matches!(**lhs, Expr::IntLit { value: 3, .. }));
            let sign_args = builtin_call(rhs, "sign");
            assert!(matches!(sign_args[0], Expr::IntLit { value: 3, .. }));
        }
        other => panic!("expected ElementwiseOp(Add, ...), got {other:?}"),
    }
}

#[test]
fn hook_dyadic_is_x_f_g_y() {
    // x (+ *) y = x + (*y): 3 (+*) 4 = 3 + sign(4)
    let m = compile_ok("3(+*)4\n");
    match printed_value(&main_fn(&m).body.stmts[0]) {
        Expr::ElementwiseOp { op: ElementwiseOpKind::Add, lhs, rhs, .. } => {
            assert!(matches!(**lhs, Expr::IntLit { value: 3, .. }));
            let sign_args = builtin_call(rhs, "sign");
            assert!(matches!(sign_args[0], Expr::IntLit { value: 4, .. }));
        }
        other => panic!("expected ElementwiseOp(Add, ...), got {other:?}"),
    }
}

#[test]
fn hook_with_a_bare_noun_tooth_is_rejected() {
    // `j.grammar`'s own header comment gives this exact example: two bare
    // NAMEs parse as a syntactically well-formed 2-tooth train even though
    // no real hook has an all-noun shape. Uses NAMEs, not NUMBERs -- two
    // juxtaposed NUMBER tokens would instead strand into a single `term`
    // (one tooth, not two), which is a different, unrelated grammar
    // production (see `stranded_literal_is_a_single_row_array_lit` above).
    let msg = compile_err("a=.3\nb=.4\n(a b)5\n");
    assert!(msg.contains("must be a verb"), "{msg}");
}

// ── forks ────────────────────────────────────────────────────────────────

#[test]
fn verb_left_fork_monadic_is_f_y_g_h_y() {
    // (+ * -) 3 = (+3) g=* (-3) = ElementwiseOp(Mul, 3, neg(3))
    let m = compile_ok("(+*-)3\n");
    match printed_value(&main_fn(&m).body.stmts[0]) {
        Expr::ElementwiseOp { op: ElementwiseOpKind::Mul, lhs, rhs, .. } => {
            assert!(matches!(**lhs, Expr::IntLit { value: 3, .. }));
            let neg_args = builtin_call(rhs, "neg");
            assert!(matches!(neg_args[0], Expr::IntLit { value: 3, .. }));
        }
        other => panic!("expected ElementwiseOp(Mul, ...), got {other:?}"),
    }
}

#[test]
fn verb_left_fork_dyadic() {
    // x (+ * -) y = (x+y) * (x-y): 3 (+*-) 4
    let m = compile_ok("3(+*-)4\n");
    match printed_value(&main_fn(&m).body.stmts[0]) {
        Expr::ElementwiseOp { op: ElementwiseOpKind::Mul, lhs, rhs, .. } => {
            assert!(matches!(**lhs, Expr::ElementwiseOp { op: ElementwiseOpKind::Add, .. }));
            assert!(matches!(**rhs, Expr::ElementwiseOp { op: ElementwiseOpKind::Sub, .. }));
        }
        other => panic!("expected ElementwiseOp(Mul, ...), got {other:?}"),
    }
}

#[test]
fn leading_noun_fork_monadic_is_n_g_h_y() {
    // (5 * -) 3 = 5 * (-3) = ElementwiseOp(Mul, 5, neg(3))
    let m = compile_ok("(5*-)3\n");
    match printed_value(&main_fn(&m).body.stmts[0]) {
        Expr::ElementwiseOp { op: ElementwiseOpKind::Mul, lhs, rhs, .. } => {
            assert!(matches!(**lhs, Expr::IntLit { value: 5, .. }));
            let neg_args = builtin_call(rhs, "neg");
            assert!(matches!(neg_args[0], Expr::IntLit { value: 3, .. }));
        }
        other => panic!("expected ElementwiseOp(Mul, ...), got {other:?}"),
    }
}

#[test]
fn leading_noun_fork_dyadic() {
    // x (5 * -) y = 5 * (x-y): 3 (5*-) 4
    let m = compile_ok("3(5*-)4\n");
    match printed_value(&main_fn(&m).body.stmts[0]) {
        Expr::ElementwiseOp { op: ElementwiseOpKind::Mul, lhs, rhs, .. } => {
            assert!(matches!(**lhs, Expr::IntLit { value: 5, .. }));
            assert!(matches!(**rhs, Expr::ElementwiseOp { op: ElementwiseOpKind::Sub, .. }));
        }
        other => panic!("expected ElementwiseOp(Mul, ...), got {other:?}"),
    }
}

#[test]
fn a_bare_noun_in_a_forks_non_leading_position_is_rejected() {
    let msg = compile_err("(+3-)5\n");
    assert!(msg.contains("must be a verb"), "{msg}");
}

// ── 4+-tooth trains (peel from the left) ─────────────────────────────────

#[test]
fn four_tooth_train_peels_into_a_hook_wrapping_a_fork() {
    // (+ * - %) 3 = (+ (* - %)) 3 -- outer hook: 3 + ((*-%)3)
    // inner fork (* - %): (*3) - (%3) = sign(3) - recip(3)
    let m = compile_ok("(+*-%)3\n");
    match printed_value(&main_fn(&m).body.stmts[0]) {
        Expr::ElementwiseOp { op: ElementwiseOpKind::Add, lhs, rhs, .. } => {
            assert!(matches!(**lhs, Expr::IntLit { value: 3, .. }));
            match &**rhs {
                Expr::ElementwiseOp { op: ElementwiseOpKind::Sub, lhs: inner_lhs, rhs: inner_rhs, .. } => {
                    let sign_args = builtin_call(inner_lhs, "sign");
                    assert!(matches!(sign_args[0], Expr::IntLit { value: 3, .. }));
                    let recip_args = builtin_call(inner_rhs, "recip");
                    assert!(matches!(recip_args[0], Expr::IntLit { value: 3, .. }));
                }
                other => panic!("expected inner ElementwiseOp(Sub, ...), got {other:?}"),
            }
        }
        other => panic!("expected outer ElementwiseOp(Add, ...), got {other:?}"),
    }
}

#[test]
fn a_bare_noun_as_the_leading_tooth_of_a_wide_train_is_rejected() {
    // Unlike a 3-tooth fork, a 4+-tooth train's peeled leading tooth is
    // always a hook's `f` role, which must be a verb (only a FORK's
    // leading position ever accepts a noun).
    let msg = compile_err("(3*-%)5\n");
    assert!(msg.contains("must be a verb"), "{msg}");
}

// ── assignment ───────────────────────────────────────────────────────────

#[test]
fn assignment_is_silent_no_auto_print() {
    let m = compile_ok("a=.3\n");
    let main = main_fn(&m);
    assert_eq!(main.body.stmts.len(), 1);
    assert!(matches!(
        main.body.stmts[0],
        Stmt::LetStarBinding { .. }
    ));
}

#[test]
fn global_assignment_lowers_identically_to_local() {
    let m = compile_ok("a=:3\n");
    let main = main_fn(&m);
    assert!(matches!(main.body.stmts[0], Stmt::LetStarBinding { .. }));
}

#[test]
fn chained_assignment_unrolls_in_dependency_order() {
    let m = compile_ok("a=.b=.3\n");
    let main = main_fn(&m);
    assert_eq!(main.body.stmts.len(), 2);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { name, value, .. } => {
            assert_eq!(name, "b");
            assert!(matches!(value, Expr::IntLit { value: 3, .. }));
        }
        other => panic!("expected LetStarBinding(b, 3), got {other:?}"),
    }
    match &main.body.stmts[1] {
        Stmt::LetStarBinding { name, value, .. } => {
            assert_eq!(name, "a");
            assert!(matches!(value, Expr::VarRef { .. }));
        }
        other => panic!("expected LetStarBinding(a, VarRef(b)), got {other:?}"),
    }
}

#[test]
fn reassignment_emits_assign_and_observes_mutable_bindings() {
    let m = compile_ok("a=.3\na=.4\n");
    let main = main_fn(&m);
    assert_eq!(main.body.stmts.len(), 2);
    assert!(matches!(main.body.stmts[0], Stmt::LetStarBinding { .. }));
    assert!(matches!(main.body.stmts[1], Stmt::Assign { .. }));
    assert!(m.manifest.iter().any(|f| f == Feature::MutableBindings));
}

#[test]
fn undefined_variable_is_rejected() {
    let msg = compile_err("a\n");
    assert!(msg.contains("undefined variable"), "{msg}");
}

// ── errors and depth guards ──────────────────────────────────────────────

#[test]
fn parse_errors_propagate_cleanly() {
    assert!(compile_source("3+\n", "prog").is_err());
}

#[test]
fn train_wider_than_the_combinator_depth_cap_is_rejected() {
    // 20 teeth requires far more than 12 nested Hook levels to fold.
    let train: String = std::iter::repeat_n("+", 20).collect::<Vec<_>>().join(" ");
    let src = format!("({train})3\n");
    let msg = compile_err(&src);
    assert!(msg.contains("combinator levels"), "{msg}");
}

#[test]
fn a_train_within_the_combinator_depth_cap_still_succeeds() {
    // 5 teeth (2 hook levels + 1 fork) is comfortably within the cap.
    let m = compile_ok("(+*-%<.)3\n");
    let main = main_fn(&m);
    assert_eq!(main.body.stmts.len(), 1);
}

// ── a full multi-line program validates cleanly ──────────────────────────

#[test]
fn a_full_program_validates_via_semantic_ir_validate() {
    let m = compile_ok("a=.3+4\nb=.+/1 2 3\na*b\n");
    let report = semantic_ir::validate(&m);
    assert!(report.is_ok(), "validation failed: {:?}", report.issues);
}
