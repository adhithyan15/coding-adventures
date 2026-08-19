//! SIR22 execution proof, base cut + "APL addendum": `ArrayLit`/`Range`/
//! `MatMul`/`ElementwiseOp`/`Transpose`/`IndexGet`/`Stmt::IndexSet` (Phase A
//! Slice 2) and `Reduce`/`Scan`/`OuterProduct`/`Shape`/`Reshape`/
//! `IndexGenerator`/`IndexOf`/`Ravel`/`Catenate` (Phase A Slice 3) on the
//! Ruby backend — hand-builds a module calling each node directly
//! (bypassing the frontend, since no frontend targets this backend for
//! SIR22 yet), emits Ruby, runs it with a real `ruby` interpreter, and
//! asserts stdout. Mirrors `division_ops_tests.rs`'s pattern; skips (does
//! not fail) when no `ruby` is on `PATH`.
//!
//! Ported from `semantic-ir-to-javascript`'s own already-proven
//! `tests/sir22_array.rs` and `runtime.rs` addendum doc comments — same
//! worked examples, adapted to this backend's own value-type conventions
//! (Ruby preserves native Integer/Float type propagation rather than JS's
//! uniform-double storage; see `runtime.rs`'s own module doc for why an
//! all-integer computation here prints without a spurious `.0`).
//!
//! Every test constructs `Module`s directly via `print_stmt`'s scalar
//! `IndexGet` reads (top-left/bottom-right element, etc.) — sidesteps
//! needing an NDArray display/format story, which is out of this slice's
//! scope, exactly as the JS reference's own tests do.

use semantic_ir::{
    Block, Effect, EffectSet, ElementwiseOpKind, Expr, Feature, FeatureManifest, Function,
    IndexArg, Metadata, Module, Scope, Span, Stmt,
};

fn s() -> Span {
    Span::synthetic()
}
fn ilit(v: i64) -> Expr {
    Expr::IntLit { value: v, span: s() }
}
fn local(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Local, span: s() }
}
fn array_lit(rows: Vec<Vec<Expr>>) -> Expr {
    Expr::ArrayLit { rows, span: s() }
}
fn print_stmt(expr: Expr) -> Stmt {
    Stmt::ExprStmt {
        expr: Expr::BuiltinCall {
            name: "__sys_write__".into(),
            args: vec![
                Expr::StrLit { value: "stdout".into(), span: s() },
                Expr::StrLit { value: "once".into(), span: s() },
                Expr::BoolLit { value: false, span: s() },
                expr,
            ],
            effects: EffectSet::PURE.with(Effect::MayPrint),
            span: s(),
        },
        span: s(),
    }
}
fn let_binding(name: &str, value: Expr) -> Stmt {
    Stmt::LetBinding { name: name.into(), sir_type: None, value, span: s() }
}
fn scalar(i: i64) -> IndexArg {
    IndexArg::Scalar(Box::new(ilit(i)))
}
fn index_get(target: Expr, indices: Vec<IndexArg>) -> Expr {
    Expr::IndexGet { target: Box::new(target), indices, span: s() }
}

const ARRAY_FEATURES: &[Feature] =
    &[Feature::NDArrays, Feature::MatrixOps, Feature::ArrayColumnMajor];

fn array_module(stmts: Vec<Stmt>) -> Module {
    let mut features = vec![Feature::ConsoleIO, Feature::Strings];
    features.extend_from_slice(ARRAY_FEATURES);
    Module {
        name: "sir22array".into(),
        manifest: FeatureManifest::from_features(&features),
        imports: vec![],
        exports: vec![],
        functions: vec![Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block { stmts, value: Expr::NilLit { span: s() }, span: s() },
            effects: EffectSet::PURE.with(Effect::MayPrint),
            metadata: Metadata::new(),
            span: s(),
        }],
        globals: vec![],
        metadata: Metadata::new().with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
        span: s(),
    }
}

/// Run emitted Ruby, returning stdout, or `None` to skip when no `ruby` is
/// on `PATH`. Unique temp-file names per call (PID + a monotonic counter)
/// — a constant name would let concurrently-running `cargo test` threads
/// collide on the same path, the exact race this session's own
/// `sir-conformance` division tests hit and fixed earlier in this arc.
fn run_array_program(m: &Module) -> Option<String> {
    use std::io::Write;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static SEQ: AtomicUsize = AtomicUsize::new(0);

    let source = semantic_ir_to_ruby::compile(m).expect("ruby emit").source;
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "sir_ruby_array_{}_{}.rb",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::File::create(&path).ok()?.write_all(source.as_bytes()).ok()?;
    let out = std::process::Command::new("ruby").arg(&path).output().ok()?;
    let _ = std::fs::remove_file(&path);
    assert!(
        out.status.success(),
        "emitted ruby exited non-zero:\n{}\n--- source ---\n{source}",
        String::from_utf8_lossy(&out.stderr)
    );
    Some(String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n"))
}

#[test]
fn matmul_of_two_by_two_matrices_computes_the_right_product() {
    // [1 2; 3 4] * [5 6; 7 8] = [19 22; 43 50] (standard matrix product).
    // All-integer inputs stay Integer through Ruby's native `+`/`*`, so
    // the printed result has no spurious ".0" (see runtime.rs's own doc).
    let a = array_lit(vec![vec![ilit(1), ilit(2)], vec![ilit(3), ilit(4)]]);
    let b = array_lit(vec![vec![ilit(5), ilit(6)], vec![ilit(7), ilit(8)]]);
    let product = Expr::MatMul { lhs: Box::new(a), rhs: Box::new(b), span: s() };
    let m = array_module(vec![
        let_binding("p", product),
        print_stmt(index_get(local("p"), vec![scalar(0), scalar(0)])),
        print_stmt(index_get(local("p"), vec![scalar(1), scalar(1)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "19\n50\n"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn elementwise_mul_with_a_bare_scalar_operand_broadcasts() {
    // MATLAB `A .* 2` -- matlab-to-semantic-ir emits the `2` as a bare
    // IntLit operand, unwrapped (not an ArrayLit), when exactly one side
    // is scalar. This is the exact shape `sir_array_to_array_value`'s
    // coercion in runtime.rs exists for; a regression there raises
    // NoMethodError instead of computing [2 4; 6 8].
    let a = array_lit(vec![vec![ilit(1), ilit(2)], vec![ilit(3), ilit(4)]]);
    let scaled = Expr::ElementwiseOp {
        op: ElementwiseOpKind::Mul,
        lhs: Box::new(a),
        rhs: Box::new(ilit(2)),
        span: s(),
    };
    let m = array_module(vec![
        let_binding("sc", scaled),
        print_stmt(index_get(local("sc"), vec![scalar(0), scalar(0)])),
        print_stmt(index_get(local("sc"), vec![scalar(1), scalar(1)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "2\n8\n"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn elementwise_div_always_true_divides_even_on_integer_operands() {
    // Unlike Add/Sub/Mul (which preserve Integer), Div forces a Float
    // result -- Ruby's native Integer#/ floors, which would silently
    // disagree with MATLAB's `./` (always real division).
    let a = array_lit(vec![vec![ilit(7)]]);
    let divided = Expr::ElementwiseOp {
        op: ElementwiseOpKind::Div,
        lhs: Box::new(a),
        rhs: Box::new(ilit(2)),
        span: s(),
    };
    let m = array_module(vec![
        let_binding("d", divided),
        print_stmt(index_get(local("d"), vec![scalar(0), scalar(0)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "3.5\n"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn transpose_of_a_two_by_three_matrix_swaps_rows_and_columns() {
    // [1 2 3; 4 5 6]' = [1 4; 2 5; 3 6]
    let a = array_lit(vec![vec![ilit(1), ilit(2), ilit(3)], vec![ilit(4), ilit(5), ilit(6)]]);
    let t = Expr::Transpose { target: Box::new(a), conjugate: true, span: s() };
    let m = array_module(vec![
        let_binding("t", t),
        print_stmt(index_get(local("t"), vec![scalar(0), scalar(1)])),
        print_stmt(index_get(local("t"), vec![scalar(2), scalar(1)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "4\n6\n"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn range_materializes_a_row_vector_read_by_linear_index() {
    // 1:2:9 -> [1 3 5 7 9], a 1x5 row vector. A single index argument
    // reads linearly (rank-1 IndexGet).
    let r = Expr::Range {
        start: Box::new(ilit(1)),
        step: Some(Box::new(ilit(2))),
        stop: Box::new(ilit(9)),
        span: s(),
    };
    let m = array_module(vec![
        let_binding("r", r),
        print_stmt(index_get(local("r"), vec![scalar(0)])),
        print_stmt(index_get(local("r"), vec![scalar(4)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "1\n9\n"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn whole_selector_reads_an_entire_row() {
    // A(1, :) on [1 2; 3 4] reads the whole second row [3 4], then a
    // scalar linear IndexGet reads its second element.
    let a = array_lit(vec![vec![ilit(1), ilit(2)], vec![ilit(3), ilit(4)]]);
    let row = index_get(a, vec![scalar(1), IndexArg::Whole]);
    let m = array_module(vec![
        let_binding("row", row),
        print_stmt(index_get(local("row"), vec![scalar(1)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "4\n"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn index_set_mutates_in_place() {
    // A(1, 1) = 99 on [1 2; 3 4] -- IndexSet is a Stmt (in-place
    // mutation), not a pure Expr, per the SIR22 spec.
    let a = array_lit(vec![vec![ilit(1), ilit(2)], vec![ilit(3), ilit(4)]]);
    let m = array_module(vec![
        let_binding("a", a),
        Stmt::IndexSet {
            target: Box::new(local("a")),
            indices: vec![scalar(1), scalar(1)],
            value: Box::new(ilit(99)),
            span: s(),
        },
        print_stmt(index_get(local("a"), vec![scalar(1), scalar(1)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "99\n"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

// ── SIR22 "APL addendum" (Phase A Slice 3): real execution proofs ──────
//
// `ArrayLit` always builds RANK-2 storage (even a single-row literal is a
// `1xn` matrix, matching MATLAB's own row-vector convention -- see
// `sir_array_from_rows`), and so does `Range`/`IndexGet`'s `Whole`
// selector. But several addendum functions (`Reduce`/`OuterProduct`'s
// operands, `Reshape`'s `shape` argument, `IndexOf`'s `haystack`) require
// a genuine RANK-1 vector (`shape == [n]`, not `[1, n]`). `Ravel`/`Shape`
// are themselves the addendum's own way to produce one (their outputs are
// always rank <= 1) -- so most tests below route a `1xn` `ArrayLit`
// through `Ravel` first to get a real vector operand, exactly as a
// compiled APL program would.

#[test]
fn reduce_folds_a_vector_left_to_right() {
    // +/[1 2 3 4] = 10 (left fold: ((1+2)+3)+4). The vector operand comes
    // from `Ravel` (a `1x4` `ArrayLit` is rank-2, not rank-1).
    let row = array_lit(vec![vec![ilit(1), ilit(2), ilit(3), ilit(4)]]);
    let vector = Expr::Ravel { target: Box::new(row), span: s() };
    let reduced = Expr::Reduce {
        op: ElementwiseOpKind::Add,
        target: Box::new(vector),
        span: s(),
    };
    let m = array_module(vec![print_stmt(index_get(reduced, vec![scalar(0)]))]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "10\n"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn reduce_on_a_matrix_folds_each_row_independently() {
    // +/ on [[1 2 3];[4 5 6]] folds EACH ROW across its columns
    // independently -- [1+2+3, 4+5+6] = [6, 15]. This is the exact
    // column-major-indexing case `runtime.rs`'s own doc comment warns is
    // "the single easiest place to introduce a wrong-answer bug": getting
    // `row`/`col` swapped here would silently transpose the result
    // instead of raising.
    let a = array_lit(vec![vec![ilit(1), ilit(2), ilit(3)], vec![ilit(4), ilit(5), ilit(6)]]);
    let reduced = Expr::Reduce { op: ElementwiseOpKind::Add, target: Box::new(a), span: s() };
    let m = array_module(vec![
        let_binding("r", reduced),
        print_stmt(index_get(local("r"), vec![scalar(0)])),
        print_stmt(index_get(local("r"), vec![scalar(1)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "6\n15\n"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn scan_on_a_vector_keeps_every_prefix_fold() {
    // +\[1 2 3 4] = [1, 3, 6, 10] -- every prefix sum, not just the last
    // (unlike `Reduce`, which only keeps the final fold).
    let row = array_lit(vec![vec![ilit(1), ilit(2), ilit(3), ilit(4)]]);
    let vector = Expr::Ravel { target: Box::new(row), span: s() };
    let scanned = Expr::Scan { op: ElementwiseOpKind::Add, target: Box::new(vector), span: s() };
    let m = array_module(vec![
        let_binding("sc", scanned),
        print_stmt(index_get(local("sc"), vec![scalar(0)])),
        print_stmt(index_get(local("sc"), vec![scalar(1)])),
        print_stmt(index_get(local("sc"), vec![scalar(2)])),
        print_stmt(index_get(local("sc"), vec![scalar(3)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "1\n3\n6\n10\n"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn outer_product_applies_op_to_every_pair() {
    // [1 2] outer+ [10 20 30] -> [[11 21 31];[12 22 32]] (shape [2,3]:
    // out(i,j) = a[i] + b[j]).
    let va = Expr::Ravel { target: Box::new(array_lit(vec![vec![ilit(1), ilit(2)]])), span: s() };
    let vb = Expr::Ravel {
        target: Box::new(array_lit(vec![vec![ilit(10), ilit(20), ilit(30)]])),
        span: s(),
    };
    let outer = Expr::OuterProduct {
        op: ElementwiseOpKind::Add,
        lhs: Box::new(va),
        rhs: Box::new(vb),
        span: s(),
    };
    let m = array_module(vec![
        let_binding("o", outer),
        print_stmt(index_get(local("o"), vec![scalar(0), scalar(0)])),
        print_stmt(index_get(local("o"), vec![scalar(1), scalar(2)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "11\n32\n"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn shape_of_a_scalar_is_the_empty_vector_not_a_scalar() {
    // ⍴5 must be a length-0 VECTOR (shape=[0], data=[]), never a scalar
    // wrapping the value 0. Applying `Shape` a SECOND time distinguishes
    // the two: the correct result's OWN shape is [1] (one dimension,
    // whose value is 0), directly readable -- whereas a wrong "scalar
    // containing 0" implementation (shape=[], data=[0]) would make the
    // SECOND `Shape` call produce an EMPTY vector instead (shape=[0],
    // data=[]), which raises on this test's own `index_get` -- so a
    // regression here fails LOUDLY (non-zero exit), not silently.
    let inner = Expr::Shape { target: Box::new(ilit(5)), span: s() };
    let outer = Expr::Shape { target: Box::new(inner), span: s() };
    let m = array_module(vec![print_stmt(index_get(outer, vec![scalar(0)]))]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "0\n"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn shape_of_a_matrix_returns_its_dimensions() {
    let a = array_lit(vec![vec![ilit(1), ilit(2), ilit(3)], vec![ilit(4), ilit(5), ilit(6)]]);
    let sh = Expr::Shape { target: Box::new(a), span: s() };
    let m = array_module(vec![
        let_binding("sh", sh),
        print_stmt(index_get(local("sh"), vec![scalar(0)])),
        print_stmt(index_get(local("sh"), vec![scalar(1)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "2\n3\n"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn reshape_fills_row_major_then_stores_column_major() {
    // Reshape [1 2 3 4 5 6] (a row-major source) into a 2x3 matrix. APL's
    // reshape fills the LAST axis fastest (row-major): row 0 gets
    // [1 2 3], row 1 gets [4 5 6]. This domain's storage is column-major,
    // so a regression that hands the row-major `filled` sequence straight
    // to storage would silently TRANSPOSE the result -- `get(0,1)` would
    // read 3 instead of 2, `get(1,0)` would read 2 instead of 4 -- a wrong
    // answer that still looks plausible (right multiset of values, wrong
    // positions).
    let dims_source = array_lit(vec![
        vec![ilit(0), ilit(0), ilit(0)],
        vec![ilit(0), ilit(0), ilit(0)],
    ]); // a throwaway 2x3 matrix, used only for its `Shape` ([2, 3]) --
        // `Shape`'s own output is a genuine rank-1 vector, which is what
        // `Reshape`'s `shape` argument requires (rank <= 1).
    let shape_arg = Expr::Shape { target: Box::new(dims_source), span: s() };
    let data_source =
        array_lit(vec![vec![ilit(1), ilit(2), ilit(3), ilit(4), ilit(5), ilit(6)]]);
    let reshaped = Expr::Reshape {
        shape: Box::new(shape_arg),
        target: Box::new(data_source),
        span: s(),
    };
    let m = array_module(vec![
        let_binding("r", reshaped),
        print_stmt(index_get(local("r"), vec![scalar(0), scalar(1)])),
        print_stmt(index_get(local("r"), vec![scalar(1), scalar(0)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "2\n4\n"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn index_generator_is_one_based_not_zero_based() {
    // ⍳5 = [1 2 3 4 5] -- a deliberate exception to this domain's
    // otherwise-universal 0-based indexing (`apl-runtime`'s own
    // `index_generator_produces_one_based_run` test is the ground truth
    // this ports; the SIR22 spec's/`nodes.rs`'s own prose calling this
    // "0-based" is stale relative to the shipped `apl-runtime`/JS-backend
    // behaviour, which this backend matches for cross-backend parity).
    let ig = Expr::IndexGenerator { count: Box::new(ilit(5)), span: s() };
    let m = array_module(vec![
        let_binding("g", ig),
        print_stmt(index_get(local("g"), vec![scalar(0)])),
        print_stmt(index_get(local("g"), vec![scalar(4)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "1\n5\n"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn index_of_finds_and_reports_not_found_as_length_plus_one() {
    // haystack [10 20 30] ⍳ needle [20 99]: 20 is at (1-based) position 2;
    // 99 is not found, reported as haystack.length + 1 = 4 -- NEVER -1 or
    // nil, always a valid in-range position.
    let haystack = Expr::Ravel {
        target: Box::new(array_lit(vec![vec![ilit(10), ilit(20), ilit(30)]])),
        span: s(),
    };
    let needle = Expr::Ravel {
        target: Box::new(array_lit(vec![vec![ilit(20), ilit(99)]])),
        span: s(),
    };
    let found =
        Expr::IndexOf { haystack: Box::new(haystack), needle: Box::new(needle), span: s() };
    let m = array_module(vec![
        let_binding("f", found),
        print_stmt(index_get(local("f"), vec![scalar(0)])),
        print_stmt(index_get(local("f"), vec![scalar(1)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "2\n4\n"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn ravel_flattens_a_matrix_in_row_major_order() {
    // ,[[1 2];[3 4];[5 6]] = [1 2 3 4 5 6] -- row-major order (last axis
    // fastest), even though the source is stored column-major.
    let a = array_lit(vec![
        vec![ilit(1), ilit(2)],
        vec![ilit(3), ilit(4)],
        vec![ilit(5), ilit(6)],
    ]);
    let raveled = Expr::Ravel { target: Box::new(a), span: s() };
    let m = array_module(vec![
        let_binding("v", raveled),
        print_stmt(index_get(local("v"), vec![scalar(0)])),
        print_stmt(index_get(local("v"), vec![scalar(1)])),
        print_stmt(index_get(local("v"), vec![scalar(2)])),
        print_stmt(index_get(local("v"), vec![scalar(3)])),
        print_stmt(index_get(local("v"), vec![scalar(4)])),
        print_stmt(index_get(local("v"), vec![scalar(5)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "1\n2\n3\n4\n5\n6\n"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn catenate_of_two_vectors_concatenates_end_to_end() {
    let a = Expr::Ravel { target: Box::new(array_lit(vec![vec![ilit(1), ilit(2)]])), span: s() };
    let b = Expr::Ravel {
        target: Box::new(array_lit(vec![vec![ilit(3), ilit(4), ilit(5)]])),
        span: s(),
    };
    let cat = Expr::Catenate { lhs: Box::new(a), rhs: Box::new(b), span: s() };
    let m = array_module(vec![
        let_binding("c", cat),
        print_stmt(index_get(local("c"), vec![scalar(0)])),
        print_stmt(index_get(local("c"), vec![scalar(4)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "1\n5\n"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn catenate_of_two_matrices_with_equal_rows_appends_columns() {
    // [[1 2];[3 4]] , [[5];[6]] -> [[1 2 5];[3 4 6]] (equal row counts;
    // `rhs`'s columns are appended after `lhs`'s own).
    let a = array_lit(vec![vec![ilit(1), ilit(2)], vec![ilit(3), ilit(4)]]);
    let b = array_lit(vec![vec![ilit(5)], vec![ilit(6)]]);
    let cat = Expr::Catenate { lhs: Box::new(a), rhs: Box::new(b), span: s() };
    let m = array_module(vec![
        let_binding("c", cat),
        print_stmt(index_get(local("c"), vec![scalar(0), scalar(0)])),
        print_stmt(index_get(local("c"), vec![scalar(0), scalar(2)])),
        print_stmt(index_get(local("c"), vec![scalar(1), scalar(2)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "1\n5\n6\n"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}
