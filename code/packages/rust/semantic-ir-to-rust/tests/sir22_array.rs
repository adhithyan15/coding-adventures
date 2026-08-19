//! SIR22 execution proof: `ArrayLit`/`Range`/`MatMul`/`ElementwiseOp`/
//! `Transpose`/`IndexGet`/`Stmt::IndexSet` (the "base cut", Phase A
//! Slice 2) AND `Reduce`/`Scan`/`OuterProduct`/`Shape`/`Reshape`/
//! `IndexGenerator`/`IndexOf`/`Ravel`/`Catenate` (the 9-node "APL
//! addendum", Phase A Slice 3) on the Rust backend — hand-builds a module
//! calling each node directly (bypassing the frontend, since no frontend
//! targets this backend for SIR22 yet), emits Rust, compiles it with a
//! real `rustc`, runs the binary, and asserts stdout. Mirrors
//! `compile_and_run_floats.rs`'s pattern; skips (does not fail) when no
//! `rustc` is on `PATH` or no usable linker is present, exactly like every
//! other `compile_and_run_*` test in this crate.
//!
//! The base-cut tests were ported from `semantic-ir-to-javascript`'s own
//! already-proven `tests/sir22_array.rs` (and cross-checked against
//! `semantic-ir-to-ruby`'s own port of the same suite). Neither the JS nor
//! the Ruby backend has addendum execution tests yet (both still reject
//! the nine addendum nodes at this repo's current HEAD) — the addendum
//! tests below are new, hand-derived from `apl_runtime::builtins`'s own
//! ground-truth tests (`code/packages/rust/apl-runtime/src/builtins.rs`)
//! and this backend's own `runtime.rs` doc comments.
//!
//! This port's `array_*` runtime (`runtime.rs`) stores every element as
//! `f64` (mirroring the JS backend's `Float64Array`, not Ruby's
//! Int/Float-preserving choice — see `runtime.rs`'s own module doc for
//! why), so `Expr::IndexGet` on a scalar position always yields a
//! `Value::Float`. Every printed assertion below therefore expects the
//! `.0`-suffixed float rendering (`format_float` in `runtime.rs`), e.g.
//! `"19.0"` rather than `"19"` — a deliberate, documented divergence from
//! the Ruby reference's Integer-preserving output, not a bug.
//!
//! Every test constructs `Module`s directly via `print_stmt`'s scalar
//! `IndexGet` reads (top-left/bottom-right element, etc.) — sidesteps
//! needing an NDArray display/format story, which is out of this slice's
//! scope, exactly as the JS/Ruby references' own tests do.

use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

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

// ── SIR22 "APL addendum" node constructors ──────────────────────────

/// Build a genuine RANK-1 vector `Value` from a list of ints. `array_lit`
/// alone always produces rank 2 (`array_from_rows` unconditionally shapes
/// its output `[nrows, ncols]`, even for a single row) — this ravels a
/// single-row `ArrayLit` to collapse it to rank 1. This matters: several
/// addendum functions dispatch on rank (`array_reduce`/`array_scan`/
/// `array_outer`/`array_index_of`/`array_catenate` each have a DIFFERENT
/// code path for rank 1 than for rank 2), so a "vector" test must hand
/// them a value that is genuinely rank 1, not a 1-row rank-2 matrix that
/// would silently exercise the WRONG branch while still producing a
/// plausible-looking answer.
fn vector1(values: Vec<i64>) -> Expr {
    let row = array_lit(vec![values.into_iter().map(ilit).collect()]);
    Expr::Ravel { target: Box::new(row), span: s() }
}

fn reduce_expr(op: ElementwiseOpKind, target: Expr) -> Expr {
    Expr::Reduce { op, target: Box::new(target), span: s() }
}

fn scan_expr(op: ElementwiseOpKind, target: Expr) -> Expr {
    Expr::Scan { op, target: Box::new(target), span: s() }
}

fn outer_expr(op: ElementwiseOpKind, lhs: Expr, rhs: Expr) -> Expr {
    Expr::OuterProduct { op, lhs: Box::new(lhs), rhs: Box::new(rhs), span: s() }
}

fn shape_expr(target: Expr) -> Expr {
    Expr::Shape { target: Box::new(target), span: s() }
}

fn reshape_expr(shape: Expr, target: Expr) -> Expr {
    Expr::Reshape { shape: Box::new(shape), target: Box::new(target), span: s() }
}

fn index_generator_expr(count: Expr) -> Expr {
    Expr::IndexGenerator { count: Box::new(count), span: s() }
}

fn index_of_expr(haystack: Expr, needle: Expr) -> Expr {
    Expr::IndexOf { haystack: Box::new(haystack), needle: Box::new(needle), span: s() }
}

fn ravel_expr(target: Expr) -> Expr {
    Expr::Ravel { target: Box::new(target), span: s() }
}

fn catenate_expr(lhs: Expr, rhs: Expr) -> Expr {
    Expr::Catenate { lhs: Box::new(lhs), rhs: Box::new(rhs), span: s() }
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

fn rustc_available() -> bool {
    Command::new("rustc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Compile `m` to Rust, run it with a real `rustc`, and return stdout — or
/// `None` to skip when no `rustc`/usable linker is on the host, matching
/// every other `compile_and_run_*` test in this crate exactly. Unique
/// temp-file names per call (PID + a monotonic counter) — a constant name
/// would let concurrently-running `cargo test` threads collide on the same
/// path, the exact race this crate's own test suite has hit and fixed
/// before (see e.g. `compile_and_run_array_aggregates.rs`'s sibling test
/// functions, each with their own literal filename prefix).
fn run_array_program(m: &Module) -> Option<String> {
    static SEQ: AtomicUsize = AtomicUsize::new(0);

    if !rustc_available() {
        return None;
    }

    let artifact = semantic_ir_to_rust::compile(m).expect("module should compile to Rust source");

    let dir = std::env::temp_dir();
    let nonce = format!("{}_{}", std::process::id(), SEQ.fetch_add(1, Ordering::Relaxed));
    let src_path = dir.join(format!("sir_rust_array_{nonce}.rs"));
    let bin_path =
        dir.join(format!("sir_rust_array_{nonce}{}", if cfg!(windows) { ".exe" } else { "" }));
    std::fs::write(&src_path, &artifact.source).expect("write temp source");

    let mut cmd = Command::new("rustc");
    cmd.arg("--edition").arg("2021").arg("-O");
    if let Ok(linker) = std::env::var("SIR_TEST_RUSTC_LINKER") {
        if !linker.is_empty() {
            cmd.arg("-C").arg(format!("linker={linker}"));
        }
    }
    let compile_out = cmd.arg(&src_path).arg("-o").arg(&bin_path).output().expect("invoke rustc");
    if !compile_out.status.success() {
        let stderr = String::from_utf8_lossy(&compile_out.stderr);
        if stderr.contains("linker") && (stderr.contains("not found") || stderr.contains("No such file")) {
            eprintln!("skipping: no usable linker on host\n{stderr}");
            let _ = std::fs::remove_file(&src_path);
            return None;
        }
        panic!(
            "emitted Rust failed to compile:\n--- stderr ---\n{stderr}\n--- source ---\n{}",
            artifact.source,
        );
    }

    let run_out = Command::new(&bin_path).output().expect("run compiled binary");
    let _ = std::fs::remove_file(&src_path);
    let _ = std::fs::remove_file(&bin_path);
    assert!(
        run_out.status.success(),
        "compiled binary exited non-zero:\n{}",
        String::from_utf8_lossy(&run_out.stderr)
    );
    Some(String::from_utf8_lossy(&run_out.stdout).replace("\r\n", "\n"))
}

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
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "19.0\n50.0\n"),
        None => eprintln!("skip: no usable rustc/linker on PATH"),
    }
}

#[test]
fn elementwise_mul_with_a_bare_scalar_operand_broadcasts() {
    // MATLAB `A .* 2` -- matlab-to-semantic-ir emits the `2` as a bare
    // IntLit operand, unwrapped (not an ArrayLit), when exactly one side
    // is scalar. This is the exact shape `array_coerce`'s
    // (`toArrayValue`) coercion in runtime.rs exists for; a regression
    // there panics instead of computing [2 4; 6 8].
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
        Some(out) => assert_eq!(out, "2.0\n8.0\n"),
        None => eprintln!("skip: no usable rustc/linker on PATH"),
    }
}

#[test]
fn elementwise_div_always_true_divides_even_on_integer_operands() {
    // `Div` always real-divides (7 / 2 = 3.5) -- never an integer floor,
    // matching MATLAB's `./` (and this backend's own `true_div`
    // precedent for the plain `/` builtin).
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
        None => eprintln!("skip: no usable rustc/linker on PATH"),
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
        Some(out) => assert_eq!(out, "4.0\n6.0\n"),
        None => eprintln!("skip: no usable rustc/linker on PATH"),
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
        Some(out) => assert_eq!(out, "1.0\n9.0\n"),
        None => eprintln!("skip: no usable rustc/linker on PATH"),
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
        Some(out) => assert_eq!(out, "4.0\n"),
        None => eprintln!("skip: no usable rustc/linker on PATH"),
    }
}

#[test]
fn index_set_mutates_in_place() {
    // A(1, 1) = 99 on [1 2; 3 4] -- IndexSet is a Stmt (in-place
    // mutation), not a pure Expr, per the SIR22 spec. Proves this
    // backend's `Value::NDArray(Rc<RefCell<SirNDArray>>)` handle mutates
    // through the SAME binding the caller already holds (`a`), exactly
    // like `Stmt::SeqSet` already does for `Value::Seq`.
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
        Some(out) => assert_eq!(out, "99.0\n"),
        None => eprintln!("skip: no usable rustc/linker on PATH"),
    }
}

// ── SIR22 "APL addendum": real execution proofs (Phase A Slice 3) ──────

#[test]
fn reduce_of_a_vector_folds_across_all_elements() {
    // +/[1 2 3 4] = 10 -- a rank-0 result, read back with a single linear
    // index (`array_index_get` supports that regardless of rank).
    let m = array_module(vec![
        let_binding("r", reduce_expr(ElementwiseOpKind::Add, vector1(vec![1, 2, 3, 4]))),
        print_stmt(index_get(local("r"), vec![scalar(0)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "10.0\n"),
        None => eprintln!("skip: no usable rustc/linker on PATH"),
    }
}

#[test]
fn reduce_of_a_matrix_folds_each_row_independently() {
    // [[1 2] [3 4] [5 6]] (3 rows x 2 cols) reduced with `+` folds EACH ROW
    // independently: row sums [3, 7, 11], a rank-1 [3] result -- not the
    // column sums [9, 12] a row/col-swapped column-major indexing bug
    // would produce. `runtime.rs`'s own doc comment on `array_reduce`
    // calls this "the single easiest place to introduce a wrong-answer
    // bug" in the whole addendum -- this test pins it.
    let matrix =
        array_lit(vec![vec![ilit(1), ilit(2)], vec![ilit(3), ilit(4)], vec![ilit(5), ilit(6)]]);
    let m = array_module(vec![
        let_binding("r", reduce_expr(ElementwiseOpKind::Add, matrix)),
        print_stmt(index_get(local("r"), vec![scalar(0)])),
        print_stmt(index_get(local("r"), vec![scalar(1)])),
        print_stmt(index_get(local("r"), vec![scalar(2)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "3.0\n7.0\n11.0\n"),
        None => eprintln!("skip: no usable rustc/linker on PATH"),
    }
}

#[test]
fn scan_of_a_vector_keeps_every_running_fold() {
    // +\[1 2 3 4] = [1 3 6 10] -- every prefix sum, not just the last.
    let m = array_module(vec![
        let_binding("s", scan_expr(ElementwiseOpKind::Add, vector1(vec![1, 2, 3, 4]))),
        print_stmt(index_get(local("s"), vec![scalar(0)])),
        print_stmt(index_get(local("s"), vec![scalar(3)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "1.0\n10.0\n"),
        None => eprintln!("skip: no usable rustc/linker on PATH"),
    }
}

#[test]
fn outer_product_of_two_vectors_computes_every_pairwise_product() {
    // [1 2] outer* [3 4 5] -- a [2, 3] result; column-major storage means
    // out[j*m+i] = a[i]*b[j], so as a matrix: row0 = [3, 4, 5],
    // row1 = [6, 8, 10].
    let m = array_module(vec![
        let_binding(
            "o",
            outer_expr(ElementwiseOpKind::Mul, vector1(vec![1, 2]), vector1(vec![3, 4, 5])),
        ),
        print_stmt(index_get(local("o"), vec![scalar(0), scalar(0)])),
        print_stmt(index_get(local("o"), vec![scalar(1), scalar(2)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "3.0\n10.0\n"),
        None => eprintln!("skip: no usable rustc/linker on PATH"),
    }
}

#[test]
fn shape_of_a_scalar_is_the_empty_vector_not_a_scalar() {
    // `shape(5)` must be the EMPTY vector (rank 1, length 0) -- NOT a
    // scalar (the trickiest part of this domain's `shape` semantics).
    // Proven by taking `shape` a SECOND time: `shape(shape(5))` reads the
    // dimensions of that empty vector, which is itself the single-element
    // vector `[0]` (one dimension, of size 0), so `index_get(0)` reads
    // back `0.0`. Had `array_shape` wrongly returned a genuine SCALAR for
    // `shape(5)` instead (rank 0, not rank 1 length 0), the outer `shape`
    // call would ALSO see rank 0 and (per `array_shape`'s own "a scalar
    // has 0 dimensions" rule) return the empty vector again -- at which
    // point `index_get(0)` finds no element at position 0 and RAISES
    // ("out of bounds"), which `run_array_program` surfaces as a non-zero
    // exit and fails the test. So a clean `"0.0"` here is only reachable
    // through the correct rank-1-length-0 representation.
    let m = array_module(vec![
        let_binding("sh", shape_expr(shape_expr(ilit(5)))),
        print_stmt(index_get(local("sh"), vec![scalar(0)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "0.0\n"),
        None => eprintln!("skip: no usable rustc/linker on PATH"),
    }
}

#[test]
fn shape_of_a_matrix_is_a_two_element_vector() {
    // shape([[1 2 3][4 5 6]]) (2 rows x 3 cols) = [2, 3].
    let matrix = array_lit(vec![vec![ilit(1), ilit(2), ilit(3)], vec![ilit(4), ilit(5), ilit(6)]]);
    let m = array_module(vec![
        let_binding("sh", shape_expr(matrix)),
        print_stmt(index_get(local("sh"), vec![scalar(0)])),
        print_stmt(index_get(local("sh"), vec![scalar(1)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "2.0\n3.0\n"),
        None => eprintln!("skip: no usable rustc/linker on PATH"),
    }
}

#[test]
fn reshape_fills_row_major_then_transposes_into_column_major_storage() {
    // Source [[1 2 3][4 5 6]] (2x3) ravels (row-major) to [1 2 3 4 5 6].
    // Reshaping that sequence into a [3, 2] target must fill ROW-major
    // (APL's convention: last axis fastest) THEN transpose into this
    // domain's column-major storage -- so as a matrix, the [3, 2] result
    // reads back as rows [1,2], [3,4], [5,6] (the row-major
    // reinterpretation of the flat sequence). A reshape that (wrongly)
    // handed the row-major `filled` sequence straight to the column-major
    // constructor would instead read back as [1,4], [2,5], [3,6] -- a
    // DIFFERENT, still-plausible-looking (same multiset of values) wrong
    // answer. This test checks POSITIONS, not just membership, which is
    // exactly what `array_reshape`'s own "CRITICAL" doc comment warns is
    // needed.
    let source = array_lit(vec![vec![ilit(1), ilit(2), ilit(3)], vec![ilit(4), ilit(5), ilit(6)]]);
    let target_shape = vector1(vec![3, 2]);
    let m = array_module(vec![
        let_binding("re", reshape_expr(target_shape, source)),
        print_stmt(index_get(local("re"), vec![scalar(0), scalar(0)])),
        print_stmt(index_get(local("re"), vec![scalar(0), scalar(1)])),
        print_stmt(index_get(local("re"), vec![scalar(1), scalar(0)])),
        print_stmt(index_get(local("re"), vec![scalar(2), scalar(1)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "1.0\n2.0\n3.0\n6.0\n"),
        None => eprintln!("skip: no usable rustc/linker on PATH"),
    }
}

#[test]
fn index_generator_is_one_based_not_zero_based() {
    // Iota 4 = [1 2 3 4] -- 1-based, unlike every 0-based index elsewhere
    // in this domain (a deliberate, documented APL surface-syntax
    // exception, not a bug -- see `array_index_generator`'s doc comment).
    let m = array_module(vec![
        let_binding("ix", index_generator_expr(ilit(4))),
        print_stmt(index_get(local("ix"), vec![scalar(0)])),
        print_stmt(index_get(local("ix"), vec![scalar(3)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "1.0\n4.0\n"),
        None => eprintln!("skip: no usable rustc/linker on PATH"),
    }
}

#[test]
fn index_of_reports_a_one_based_position_when_found() {
    // [10 20 30] index-of [20] -- 20 is the SECOND element, so index-of
    // reports the 1-based position 2 (not the 0-based 1).
    let m = array_module(vec![
        let_binding("found", index_of_expr(vector1(vec![10, 20, 30]), vector1(vec![20]))),
        print_stmt(index_get(local("found"), vec![scalar(0)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "2.0\n"),
        None => eprintln!("skip: no usable rustc/linker on PATH"),
    }
}

#[test]
fn index_of_reports_haystack_length_plus_one_when_not_found() {
    // [10 20 30] index-of [99] -- 99 is absent, so index-of reports
    // `haystack.len() + 1 == 4`, never `-1`/a sentinel outside the valid
    // range ("not found" is a valid, always-in-range position).
    let m = array_module(vec![
        let_binding("missing", index_of_expr(vector1(vec![10, 20, 30]), vector1(vec![99]))),
        print_stmt(index_get(local("missing"), vec![scalar(0)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "4.0\n"),
        None => eprintln!("skip: no usable rustc/linker on PATH"),
    }
}

#[test]
fn ravel_flattens_a_matrix_in_row_major_order() {
    // ravel([[1 2 3][4 5 6]]) = [1 2 3 4 5 6] -- row-major order (last
    // axis fastest), even though the matrix itself is stored column-major.
    let matrix = array_lit(vec![vec![ilit(1), ilit(2), ilit(3)], vec![ilit(4), ilit(5), ilit(6)]]);
    let m = array_module(vec![
        let_binding("flat", ravel_expr(matrix)),
        print_stmt(index_get(local("flat"), vec![scalar(0)])),
        print_stmt(index_get(local("flat"), vec![scalar(1)])),
        print_stmt(index_get(local("flat"), vec![scalar(5)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "1.0\n2.0\n6.0\n"),
        None => eprintln!("skip: no usable rustc/linker on PATH"),
    }
}

#[test]
fn catenate_of_two_vectors_concatenates_end_to_end() {
    // [1 2] catenate [3 4 5] = [1 2 3 4 5]
    let m = array_module(vec![
        let_binding("cat", catenate_expr(vector1(vec![1, 2]), vector1(vec![3, 4, 5]))),
        print_stmt(index_get(local("cat"), vec![scalar(0)])),
        print_stmt(index_get(local("cat"), vec![scalar(4)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "1.0\n5.0\n"),
        None => eprintln!("skip: no usable rustc/linker on PATH"),
    }
}

#[test]
fn catenate_of_two_matrices_with_equal_row_counts_joins_columns() {
    // [[1 2][3 4]] catenate [[5][6]] -- both have 2 rows, so catenate
    // joins along columns: [[1 2 5][3 4 6]].
    let a = array_lit(vec![vec![ilit(1), ilit(2)], vec![ilit(3), ilit(4)]]);
    let b = array_lit(vec![vec![ilit(5)], vec![ilit(6)]]);
    let m = array_module(vec![
        let_binding("cat", catenate_expr(a, b)),
        print_stmt(index_get(local("cat"), vec![scalar(0), scalar(0)])),
        print_stmt(index_get(local("cat"), vec![scalar(0), scalar(2)])),
        print_stmt(index_get(local("cat"), vec![scalar(1), scalar(2)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "1.0\n5.0\n6.0\n"),
        None => eprintln!("skip: no usable rustc/linker on PATH"),
    }
}
