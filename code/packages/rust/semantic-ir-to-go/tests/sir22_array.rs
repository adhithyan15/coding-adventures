//! SIR22 execution proof, base cut + APL addendum: `ArrayLit`/`Range`/
//! `MatMul`/`ElementwiseOp`/`Transpose`/`IndexGet`/`Stmt::IndexSet`
//! (Phase A Slice 2) and `Reduce`/`Scan`/`OuterProduct`/`Shape`/
//! `Reshape`/`IndexGenerator`/`IndexOf`/`Ravel`/`Catenate` (Phase A
//! Slice 3) on the Go backend — hand-builds `Module`s directly
//! (bypassing any frontend, since none targets this backend for SIR22
//! yet), emits Go, runs it with a real `go run`, and asserts stdout.
//! Mirrors `compile_and_run_division_ops.rs`'s pattern; skips (does not
//! fail) when no `go` is on `PATH`.
//!
//! Ported from `semantic-ir-to-javascript`'s own already-proven
//! `tests/sir22_array.rs` (and cross-checked against the sibling Ruby
//! backend's own port on `claude/sir22-slice2-ruby`) — same worked
//! examples, adapted to this backend's own value-type convention: EVERY
//! `_sir_ndarray_*` element is a `float64` (see `runtime.rs`'s own
//! module doc for why this backend, unlike Ruby's sibling port, does NOT
//! preserve native Integer/Float propagation), so an all-integer
//! computation here prints WITH a trailing `.0` (Go's own
//! `_sir_format_float` convention) rather than Ruby's bare integer form.
//!
//! Every test constructs `Module`s directly via `print_stmt`'s scalar
//! `IndexGet` reads (top-left/bottom-right element, etc.) — sidesteps
//! needing an NDArray display/format story, which is out of this
//! slice's scope, exactly as the JS/Ruby references' own tests do.

use semantic_ir::{
    Block, Effect, EffectSet, ElementwiseOpKind, Expr, Feature, FeatureManifest, Function,
    IndexArg, Metadata, Module, Scope, Span, Stmt,
};
use semantic_ir_to_go::compile;

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
                Expr::StrLit { value: "per_value".into(), span: s() },
                Expr::BoolLit { value: true, span: s() },
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
    let mut features = vec![Feature::ConsoleIO, Feature::Strings, Feature::Floats];
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
        metadata: Metadata::new()
            .with_source_language("test")
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
        span: s(),
    }
}

fn go_available() -> bool {
    std::process::Command::new("go")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Compile, write to a unique temp file, `go run` it. Returns `None` if
/// `go` is unavailable (caller should skip, not fail). Unique temp-file
/// names per call (PID + a monotonic counter, not just a per-test
/// `tag`) — a name collision would let concurrently-running `cargo test`
/// threads race on the same path, the same class of race this session's
/// `sir-conformance` division tests hit and fixed earlier in this arc
/// (see the C backend's `tests/compile_and_run_array_methods.rs` for the
/// identical PID+counter precedent).
fn run_raw(module: &Module, tag: &str) -> Option<std::process::Output> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static SEQ: AtomicUsize = AtomicUsize::new(0);

    if !go_available() {
        return None;
    }
    let artifact = compile(module).expect("module should compile to Go source");
    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let src_path = dir.join(format!("sir_go_array_{tag}_{nonce}_{seq}.go"));
    std::fs::write(&src_path, &artifact.source).expect("write temp source");
    let out = std::process::Command::new("go")
        .arg("run")
        .arg(&src_path)
        .output()
        .expect("invoke go run");
    let _ = std::fs::remove_file(&src_path);
    Some(out)
}

fn run(module: &Module, tag: &str) -> Option<String> {
    let out = run_raw(module, tag)?;
    assert!(
        out.status.success(),
        "emitted Go failed to compile/run: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    Some(String::from_utf8_lossy(&out.stdout).to_string())
}

// ── base cut: real codegen ────────────────────────────────────────────

#[test]
fn matmul_of_two_by_two_matrices_computes_the_right_product() {
    // [1 2; 3 4] * [5 6; 7 8] = [19 22; 43 50] (standard matrix product).
    let a = array_lit(vec![vec![ilit(1), ilit(2)], vec![ilit(3), ilit(4)]]);
    let b = array_lit(vec![vec![ilit(5), ilit(6)], vec![ilit(7), ilit(8)]]);
    let product = Expr::MatMul { lhs: Box::new(a), rhs: Box::new(b), span: s() };
    let m = array_module(vec![
        let_binding("p", product),
        print_stmt(index_get(local("p"), vec![scalar(0), scalar(0)])),
        print_stmt(index_get(local("p"), vec![scalar(1), scalar(1)])),
    ]);
    match run(&m, "matmul") {
        Some(out) => assert_eq!(out.lines().collect::<Vec<_>>(), vec!["19.0", "50.0"]),
        None => eprintln!("skip: no go on PATH"),
    }
}

#[test]
fn elementwise_mul_with_a_bare_scalar_operand_broadcasts() {
    // MATLAB `A .* 2` -- matlab-to-semantic-ir emits the `2` as a bare
    // IntLit operand, unwrapped (not an ArrayLit), when exactly one side
    // is scalar. This is the exact shape `_sir_ndarray_to_array_value`'s
    // coercion in runtime.rs exists for; a regression there panics with
    // a Go interface-conversion error instead of computing [2 4; 6 8].
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
    match run(&m, "elemwise_scalar") {
        Some(out) => assert_eq!(out.lines().collect::<Vec<_>>(), vec!["2.0", "8.0"]),
        None => eprintln!("skip: no go on PATH"),
    }
}

#[test]
fn elementwise_div_always_true_divides_even_on_integer_operands() {
    // 7 ./ 2 = 3.5 -- Div always real-divides (matches MATLAB `./`); this
    // backend's array elements are uniformly `float64` regardless, so
    // this mainly proves the `Div` dispatch arm is wired to `/`, not a
    // truncating integer division.
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
    match run(&m, "div_true") {
        Some(out) => assert_eq!(out.lines().collect::<Vec<_>>(), vec!["3.5"]),
        None => eprintln!("skip: no go on PATH"),
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
    match run(&m, "transpose") {
        Some(out) => assert_eq!(out.lines().collect::<Vec<_>>(), vec!["4.0", "6.0"]),
        None => eprintln!("skip: no go on PATH"),
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
    match run(&m, "range") {
        Some(out) => assert_eq!(out.lines().collect::<Vec<_>>(), vec!["1.0", "9.0"]),
        None => eprintln!("skip: no go on PATH"),
    }
}

#[test]
fn whole_selector_reads_an_entire_row() {
    // A(2, :) on [1 2; 3 4] (0-based: row 1) reads the whole second row
    // [3 4], then a scalar linear IndexGet reads its second element.
    let a = array_lit(vec![vec![ilit(1), ilit(2)], vec![ilit(3), ilit(4)]]);
    let row = index_get(a, vec![scalar(1), IndexArg::Whole]);
    let m = array_module(vec![
        let_binding("row", row),
        print_stmt(index_get(local("row"), vec![scalar(1)])),
    ]);
    match run(&m, "whole") {
        Some(out) => assert_eq!(out.lines().collect::<Vec<_>>(), vec!["4.0"]),
        None => eprintln!("skip: no go on PATH"),
    }
}

#[test]
fn index_set_mutates_in_place() {
    // A(1, 1) = 99 (0-based) on [1 2; 3 4] -- IndexSet is a Stmt
    // (in-place mutation), not a pure Expr, per the SIR22 spec.
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
    match run(&m, "index_set") {
        Some(out) => assert_eq!(out.lines().collect::<Vec<_>>(), vec!["99.0"]),
        None => eprintln!("skip: no go on PATH"),
    }
}

// ── DoS guard: shape-size cap enforced BEFORE allocation ───────────────

#[test]
fn matmul_output_shape_exceeding_the_element_cap_panics_cleanly() {
    // Two INDEPENDENT 1x9000 / 9000x1 operands (each individually far
    // under the 2^26 = 67,108,864-element cap) whose matmul OUTPUT shape
    // (9000x9000 = 81,000,000 elements) exceeds it -- exactly the
    // "product of two independently-bounded dimensions isn't itself
    // bounded" gap `_sir_ndarray_checked_shape_size` exists to close
    // BEFORE `make([]float64, outLen)` ever runs. A regression here
    // would either panic with a Go runtime OOM/negative-length-slice
    // crash (uncontrolled) instead of this controlled, readable message,
    // or (worse, if the overflow check were removed entirely) attempt
    // the huge allocation.
    let r = Expr::Range {
        start: Box::new(ilit(1)),
        step: None,
        stop: Box::new(ilit(9000)),
        span: s(),
    };
    let rt = Expr::Transpose { target: Box::new(r.clone()), conjugate: true, span: s() };
    let product = Expr::MatMul { lhs: Box::new(rt), rhs: Box::new(r), span: s() };
    let m = array_module(vec![Stmt::ExprStmt { expr: product, span: s() }]);
    match run_raw(&m, "matmul_overflow") {
        Some(out) => {
            assert!(
                !out.status.success(),
                "expected a clean panic (nonzero exit) for an over-cap shape, got success"
            );
            let stderr = String::from_utf8_lossy(&out.stderr);
            assert!(
                stderr.contains("exceeds the 67108864-element cap"),
                "expected the checkedShapeSize cap message on stderr, got:\n{stderr}"
            );
        }
        None => eprintln!("skip: no go on PATH"),
    }
}

// ── SIR22 "APL addendum": real codegen (Phase A Slice 3) ───────────────
//
// None of the addendum node kinds have direct literal syntax in this
// crate's hand-built `Module`s, so a genuine rank-1 "vector" (as opposed
// to a `[1, n]` row-vector-shaped MATRIX, which is what `array_lit` with
// one row and `Range` both produce) is built via nested `Catenate` of
// bare scalars -- `Catenate(scalar, scalar)` is itself one of the nodes
// under test, and is the only base-available way to reach a true rank-1
// shape here, mirroring how the JS/Ruby sibling ports' own SIR22
// addendum tests do the same thing.

fn cat(lhs: Expr, rhs: Expr) -> Expr {
    Expr::Catenate { lhs: Box::new(lhs), rhs: Box::new(rhs), span: s() }
}

/// `[a, b, c]` as a genuine rank-1 vector via nested scalar `Catenate`.
fn vec3(a: i64, b: i64, c: i64) -> Expr {
    cat(cat(ilit(a), ilit(b)), ilit(c))
}

#[test]
fn reduce_folds_a_vector_left_to_right() {
    // reduce(+, [1, 2, 3]) = ((1+2)+3) = 6, a rank-0 (scalar-shaped)
    // result -- read back via a two-scalar-index IndexGet, since
    // `_sir_ndarray_nrows`/`ncols` treat a rank-0 array as 1x1.
    let target = vec3(1, 2, 3);
    let reduced = Expr::Reduce { op: ElementwiseOpKind::Add, target: Box::new(target), span: s() };
    let m = array_module(vec![
        let_binding("r", reduced),
        print_stmt(index_get(local("r"), vec![scalar(0), scalar(0)])),
    ]);
    match run(&m, "reduce_vector") {
        Some(out) => assert_eq!(out.lines().collect::<Vec<_>>(), vec!["6.0"]),
        None => eprintln!("skip: no go on PATH"),
    }
}

#[test]
fn reduce_folds_each_row_of_a_matrix_independently() {
    // reduce(+, [1 2 3; 4 5 6]) = [6, 15] -- one folded value PER ROW,
    // not one grand total. This is the exact case the runtime's own doc
    // comment calls out as "the single easiest place to introduce a
    // wrong-answer bug" (column-major indexing inside the row loop) --
    // a row/col swap there would silently TRANSPOSE the fold instead of
    // panicking, so this test pins the correct row-independent result.
    let target = array_lit(vec![vec![ilit(1), ilit(2), ilit(3)], vec![ilit(4), ilit(5), ilit(6)]]);
    let reduced = Expr::Reduce { op: ElementwiseOpKind::Add, target: Box::new(target), span: s() };
    let m = array_module(vec![
        let_binding("r", reduced),
        print_stmt(index_get(local("r"), vec![scalar(0)])),
        print_stmt(index_get(local("r"), vec![scalar(1)])),
    ]);
    match run(&m, "reduce_matrix") {
        Some(out) => assert_eq!(out.lines().collect::<Vec<_>>(), vec!["6.0", "15.0"]),
        None => eprintln!("skip: no go on PATH"),
    }
}

#[test]
fn scan_keeps_every_intermediate_fold_result() {
    // scan(+, [1, 2, 3]) = [1, 3, 6] -- every prefix fold, not just the
    // last (that's `reduce`'s job).
    let target = vec3(1, 2, 3);
    let scanned = Expr::Scan { op: ElementwiseOpKind::Add, target: Box::new(target), span: s() };
    let m = array_module(vec![
        let_binding("s", scanned),
        print_stmt(index_get(local("s"), vec![scalar(0)])),
        print_stmt(index_get(local("s"), vec![scalar(1)])),
        print_stmt(index_get(local("s"), vec![scalar(2)])),
    ]);
    match run(&m, "scan_vector") {
        Some(out) => assert_eq!(out.lines().collect::<Vec<_>>(), vec!["1.0", "3.0", "6.0"]),
        None => eprintln!("skip: no go on PATH"),
    }
}

#[test]
fn outer_product_of_two_vectors_computes_every_pairwise_product() {
    // outer(*, [1, 2], [3, 4, 5]) -- a 2x3 matrix, out[i][j] = a[i]*b[j].
    let a = cat(ilit(1), ilit(2));
    let b = vec3(3, 4, 5);
    let outer =
        Expr::OuterProduct { op: ElementwiseOpKind::Mul, lhs: Box::new(a), rhs: Box::new(b), span: s() };
    let m = array_module(vec![
        let_binding("o", outer),
        print_stmt(index_get(local("o"), vec![scalar(0), scalar(0)])),
        print_stmt(index_get(local("o"), vec![scalar(1), scalar(2)])),
    ]);
    match run(&m, "outer_product") {
        // (row 0, col 0) = 1*3 = 3; (row 1, col 2) = 2*5 = 10.
        Some(out) => assert_eq!(out.lines().collect::<Vec<_>>(), vec!["3.0", "10.0"]),
        None => eprintln!("skip: no go on PATH"),
    }
}

#[test]
fn shape_of_a_scalar_is_the_empty_vector_not_a_scalar() {
    // `shape(5)` must be a length-0 VECTOR (rank 1, zero elements), never
    // a scalar -- the trickiest of the nine to get right, per the
    // runtime's own doc comment. Proven by taking `shape` TWICE:
    // `shape(shape(5))` reads the INNER shape's own dimensions. If
    // `shape(5)` is correctly a rank-1, length-0 array (`Shape: [0]`),
    // then `shape` of THAT is a length-1 vector containing `0` (one
    // dimension, of size 0) -- so index 0 of the double-shape reads
    // `0.0`. If `shape(5)` were wrongly a bare scalar (rank 0) instead,
    // `shape` of THAT would be a length-0 vector, and reading index 0 of
    // it would panic (out of bounds) -- a different, test-failing
    // outcome, not a silently-wrong number.
    let double_shape = Expr::Shape {
        target: Box::new(Expr::Shape { target: Box::new(ilit(5)), span: s() }),
        span: s(),
    };
    let m = array_module(vec![print_stmt(index_get(double_shape, vec![scalar(0)]))]);
    match run(&m, "shape_scalar") {
        Some(out) => assert_eq!(out.lines().collect::<Vec<_>>(), vec!["0.0"]),
        None => eprintln!("skip: no go on PATH"),
    }
}

#[test]
fn shape_of_a_matrix_reports_both_dimensions() {
    let target = array_lit(vec![vec![ilit(1), ilit(2), ilit(3)], vec![ilit(4), ilit(5), ilit(6)]]);
    let sh = Expr::Shape { target: Box::new(target), span: s() };
    let m = array_module(vec![
        let_binding("sh", sh),
        print_stmt(index_get(local("sh"), vec![scalar(0)])),
        print_stmt(index_get(local("sh"), vec![scalar(1)])),
    ]);
    match run(&m, "shape_matrix") {
        Some(out) => assert_eq!(out.lines().collect::<Vec<_>>(), vec!["2.0", "3.0"]),
        None => eprintln!("skip: no go on PATH"),
    }
}

#[test]
fn reshape_fills_row_major_then_transposes_into_column_major_storage() {
    // reshape([1 2 3; 4 5 6], [3, 2]) must equal [1 2; 3 4; 5 6] -- APL's
    // reshape fills the TARGET in row-major order from the source's own
    // row-major ravel (here that ravel is already [1,2,3,4,5,6], since
    // total element count matches exactly, no cyclic repeat needed), but
    // this domain stores COLUMN-major. A backend that forgot to
    // transpose the row-major-filled sequence before storing it would
    // instead produce [1 4; 2 5; 3 6] -- same six values, WRONG
    // positions, which is exactly why this needs a real numeric
    // assertion rather than just "did it crash".
    let target = array_lit(vec![vec![ilit(1), ilit(2), ilit(3)], vec![ilit(4), ilit(5), ilit(6)]]);
    let new_shape = cat(ilit(3), ilit(2));
    let reshaped =
        Expr::Reshape { shape: Box::new(new_shape), target: Box::new(target), span: s() };
    let m = array_module(vec![
        let_binding("re", reshaped),
        print_stmt(index_get(local("re"), vec![scalar(0), scalar(0)])),
        print_stmt(index_get(local("re"), vec![scalar(1), scalar(0)])),
        print_stmt(index_get(local("re"), vec![scalar(2), scalar(1)])),
    ]);
    match run(&m, "reshape") {
        // (0,0)=1, (1,0)=3 (NOT 2 -- that's the transposition bug), (2,1)=6.
        Some(out) => assert_eq!(out.lines().collect::<Vec<_>>(), vec!["1.0", "3.0", "6.0"]),
        None => eprintln!("skip: no go on PATH"),
    }
}

#[test]
fn index_generator_is_one_based_not_zero_based() {
    // `IndexGenerator(5)` (APL monadic `iota`) is `[1, 2, 3, 4, 5]` --
    // deliberately 1-based, unlike every 0-based index elsewhere in this
    // domain (`IndexGet`/`IndexSet`). Reading linear position 0 must
    // therefore be `1.0`, not `0.0`.
    let gen = Expr::IndexGenerator { count: Box::new(ilit(5)), span: s() };
    let m = array_module(vec![
        let_binding("g", gen),
        print_stmt(index_get(local("g"), vec![scalar(0)])),
        print_stmt(index_get(local("g"), vec![scalar(4)])),
    ]);
    match run(&m, "index_generator") {
        Some(out) => assert_eq!(out.lines().collect::<Vec<_>>(), vec!["1.0", "5.0"]),
        None => eprintln!("skip: no go on PATH"),
    }
}

#[test]
fn index_of_reports_one_based_position_or_length_plus_one_when_absent() {
    // haystack = [10, 20, 30]; needle = [20, 99]. 20 is found at 0-based
    // position 1, reported 1-based as `2`; 99 is absent, reported as
    // `haystack.length + 1 = 4` -- a valid, always-in-range sentinel,
    // never `-1`.
    let haystack = cat(cat(ilit(10), ilit(20)), ilit(30));
    let needle = cat(ilit(20), ilit(99));
    let idx = Expr::IndexOf { haystack: Box::new(haystack), needle: Box::new(needle), span: s() };
    let m = array_module(vec![
        let_binding("i", idx),
        print_stmt(index_get(local("i"), vec![scalar(0)])),
        print_stmt(index_get(local("i"), vec![scalar(1)])),
    ]);
    match run(&m, "index_of") {
        Some(out) => assert_eq!(out.lines().collect::<Vec<_>>(), vec!["2.0", "4.0"]),
        None => eprintln!("skip: no go on PATH"),
    }
}

#[test]
fn ravel_flattens_a_matrix_in_row_major_order() {
    let target = array_lit(vec![vec![ilit(1), ilit(2), ilit(3)], vec![ilit(4), ilit(5), ilit(6)]]);
    let raveled = Expr::Ravel { target: Box::new(target), span: s() };
    let m = array_module(vec![
        let_binding("rv", raveled),
        print_stmt(index_get(local("rv"), vec![scalar(0)])),
        print_stmt(index_get(local("rv"), vec![scalar(5)])),
    ]);
    match run(&m, "ravel") {
        Some(out) => assert_eq!(out.lines().collect::<Vec<_>>(), vec!["1.0", "6.0"]),
        None => eprintln!("skip: no go on PATH"),
    }
}

#[test]
fn catenate_joins_two_vectors_end_to_end() {
    let a = cat(ilit(1), ilit(2));
    let b = cat(ilit(3), ilit(4));
    let joined = Expr::Catenate { lhs: Box::new(a), rhs: Box::new(b), span: s() };
    let m = array_module(vec![
        let_binding("j", joined),
        print_stmt(index_get(local("j"), vec![scalar(0)])),
        print_stmt(index_get(local("j"), vec![scalar(3)])),
    ]);
    match run(&m, "catenate_vectors") {
        Some(out) => assert_eq!(out.lines().collect::<Vec<_>>(), vec!["1.0", "4.0"]),
        None => eprintln!("skip: no go on PATH"),
    }
}

#[test]
fn catenate_joins_two_matrices_with_equal_row_counts_along_columns() {
    // [1 2; 3 4] , [5 6; 7 8] -> [1 2 5 6; 3 4 7 8] (column/last-axis
    // catenate; both operands have 2 rows).
    let a = array_lit(vec![vec![ilit(1), ilit(2)], vec![ilit(3), ilit(4)]]);
    let b = array_lit(vec![vec![ilit(5), ilit(6)], vec![ilit(7), ilit(8)]]);
    let joined = Expr::Catenate { lhs: Box::new(a), rhs: Box::new(b), span: s() };
    let m = array_module(vec![
        let_binding("j", joined),
        print_stmt(index_get(local("j"), vec![scalar(0), scalar(2)])),
        print_stmt(index_get(local("j"), vec![scalar(1), scalar(3)])),
    ]);
    match run(&m, "catenate_matrices") {
        Some(out) => assert_eq!(out.lines().collect::<Vec<_>>(), vec!["5.0", "8.0"]),
        None => eprintln!("skip: no go on PATH"),
    }
}

// ── DoS guard: addendum shape-size cap enforced BEFORE allocation ──────

#[test]
fn outer_product_output_shape_exceeding_the_element_cap_panics_cleanly() {
    // Two INDEPENDENT 9000-element vectors (each individually far under
    // the 2^26-element cap) whose OUTER PRODUCT output shape
    // (9000x9000 = 81,000,000 elements) exceeds it -- same
    // "product of two independently-bounded dimensions isn't itself
    // bounded" gap `matmul_output_shape_exceeding_the_element_cap_panics_
    // cleanly` proves above, now for `_sir_ndarray_outer`.
    let a = Expr::IndexGenerator { count: Box::new(ilit(9000)), span: s() };
    let b = Expr::IndexGenerator { count: Box::new(ilit(9000)), span: s() };
    let outer =
        Expr::OuterProduct { op: ElementwiseOpKind::Add, lhs: Box::new(a), rhs: Box::new(b), span: s() };
    let m = array_module(vec![Stmt::ExprStmt { expr: outer, span: s() }]);
    match run_raw(&m, "outer_overflow") {
        Some(out) => {
            assert!(
                !out.status.success(),
                "expected a clean panic (nonzero exit) for an over-cap shape, got success"
            );
            let stderr = String::from_utf8_lossy(&out.stderr);
            assert!(
                stderr.contains("exceeds the 67108864-element cap"),
                "expected the checkedShapeSize cap message on stderr, got:\n{stderr}"
            );
        }
        None => eprintln!("skip: no go on PATH"),
    }
}
