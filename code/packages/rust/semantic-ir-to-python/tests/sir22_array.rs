//! SIR22 base-cut execution proof: `ArrayLit`/`Range`/
//! `MatMul`/`ElementwiseOp`/`Transpose`/`IndexGet`/`Stmt::IndexSet` on the
//! Python backend — hand-builds a module calling each node directly
//! (bypassing the frontend, since no frontend targets this backend for
//! SIR22 yet), emits Python, runs it with a real `python3`/`python`
//! interpreter (with `coding-adventures-sir-runtime-core` and the new
//! `coding-adventures-sir-runtime-array` package on `PYTHONPATH`), and
//! asserts stdout. Mirrors `semantic-ir-to-ruby`'s own
//! `tests/sir22_array.rs` (itself ported from `semantic-ir-to-javascript`'s
//! already-proven `tests/sir22_array.rs`); skips (does not fail) when no
//! usable Python interpreter is on `PATH`.
//!
//! Unlike Ruby/JS (which inline their array runtime), this backend follows
//! the TypeScript backend's *imported-package* model — see
//! `semantic-ir-to-python/src/runtime.rs`'s `RUNTIME_ARRAY` doc comment —
//! so every test here must add `sir-runtime-array/src` to `PYTHONPATH`
//! alongside `sir-runtime-core/src`.
//!
//! Every test constructs `Module`s directly and reads back individual
//! elements via a scalar `IndexGet` (top-left/bottom-right element, etc.)
//! — sidesteps needing an `NDArray` display/format story, out of this
//! slice's scope, exactly as the JS/Ruby references' own tests do.

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

/// Probe whether `exe` is a usable Python interpreter, distinguishing a
/// genuinely-absent interpreter (and the Windows Store `python3` stub,
/// which refuses to run and exits non-zero) from a real one.
fn python_is_runnable(exe: &str) -> bool {
    std::process::Command::new(exe)
        .args(["-c", "pass"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Run emitted Python, returning stdout, or `None` to skip when no usable
/// interpreter is on `PATH`. `PYTHONPATH` includes both
/// `sir-runtime-core/src` (for `_sir_write`/`_sir_to_display`) and the new
/// `sir-runtime-array/src` (for every `_sir_array_*` helper this test file
/// exercises). Unique temp-file names per call (PID + a monotonic counter)
/// — a constant name would let concurrently-running `cargo test` threads
/// collide on the same path, matching the harness precedent this crate's
/// own `run_emitted_python` in `src/lib.rs` established.
fn run_array_program(m: &Module) -> Option<String> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);

    let exe = ["python3", "python"].into_iter().find(|e| python_is_runnable(e))?;

    let source = semantic_ir_to_python::compile(m).expect("python emit").source;

    // `sir-runtime-core` unconditionally imports across several sibling
    // per-concern packages at its own module-load time (pairs, for its
    // display convention; exceptions, for typed `SirError`), regardless
    // of whether a given emitted program actually uses those features —
    // matching the full sibling-package set this crate's own
    // `run_emitted_python` harness in `src/lib.rs` already discovered it
    // needs, plus the new `sir-runtime-array` package these tests exercise.
    let py_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../python");
    let pythonpath = std::env::join_paths([
        py_root.join("sir-runtime-core/src"),
        py_root.join("sir-runtime-pairs/src"),
        py_root.join("sir-runtime-oop/src"),
        py_root.join("sir-runtime-range/src"),
        py_root.join("sir-runtime-regex/src"),
        py_root.join("sir-runtime-exceptions/src"),
        py_root.join("sir-runtime-array/src"),
    ])
    .expect("join PYTHONPATH");

    let nonce = SEQ.fetch_add(1, Ordering::Relaxed);
    let file =
        std::env::temp_dir().join(format!("sir_py_array_{}_{}.py", std::process::id(), nonce));
    std::fs::write(&file, &source).expect("write temp python");
    let out = std::process::Command::new(exe)
        .arg(&file)
        .env("PYTHONPATH", &pythonpath)
        .output()
        .expect("spawn python");
    let _ = std::fs::remove_file(&file);

    assert!(
        out.status.success(),
        "emitted python failed under {exe}:\n{}\n--- source ---\n{source}",
        String::from_utf8_lossy(&out.stderr)
    );
    Some(String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n"))
}

#[test]
fn matmul_of_two_by_two_matrices_computes_the_right_product() {
    // [1 2; 3 4] * [5 6; 7 8] = [19 22; 43 50] (standard matrix product).
    // All-integer inputs stay `int` through this package's native
    // int/float propagation, so the printed result has no spurious ".0"
    // (see the runtime package's own module doc).
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
        None => eprintln!("skip: no python on PATH"),
    }
}

#[test]
fn elementwise_mul_with_a_bare_scalar_operand_broadcasts() {
    // MATLAB `A .* 2` -- matlab-to-semantic-ir emits the `2` as a bare
    // IntLit operand, unwrapped (not an ArrayLit), when exactly one side
    // is scalar. This is the exact shape `to_array_value`'s coercion
    // exists for; a regression there raises AttributeError instead of
    // computing [2 4; 6 8].
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
        None => eprintln!("skip: no python on PATH"),
    }
}

#[test]
fn elementwise_div_always_true_divides_even_on_integer_operands() {
    // Unlike Add/Sub/Mul (which preserve int), Div always uses Python's
    // true-division `/` -- Python's `//` would floor, which would
    // silently disagree with MATLAB's `./` (always real division).
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
        None => eprintln!("skip: no python on PATH"),
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
        None => eprintln!("skip: no python on PATH"),
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
        None => eprintln!("skip: no python on PATH"),
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
        None => eprintln!("skip: no python on PATH"),
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
        None => eprintln!("skip: no python on PATH"),
    }
}

#[test]
fn index_set_in_expression_position_walrus_path_mutates_in_place() {
    // Same mutation as `index_set_mutates_in_place`, but with the
    // IndexSet forced into a block-as-expression (walrus-tuple) position
    // by making it the final statement before an `if`'s then-branch
    // value -- exercises `emit_block_as_expr`'s own `Stmt::IndexSet` arm,
    // a separate code path from `emit_stmt`'s.
    let a = array_lit(vec![vec![ilit(1), ilit(2)], vec![ilit(3), ilit(4)]]);
    let then_branch = Block {
        stmts: vec![Stmt::IndexSet {
            target: Box::new(local("a")),
            indices: vec![scalar(0), scalar(0)],
            value: Box::new(ilit(77)),
            span: s(),
        }],
        value: index_get(local("a"), vec![scalar(0), scalar(0)]),
        span: s(),
    };
    let else_branch =
        Block { stmts: vec![], value: Expr::IntLit { value: -1, span: s() }, span: s() };
    let m = array_module(vec![
        let_binding("a", a),
        print_stmt(Expr::If {
            cond: Box::new(Expr::BoolLit { value: true, span: s() }),
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
            span: s(),
        }),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "77\n"),
        None => eprintln!("skip: no python on PATH"),
    }
}

// ── SIR22 "APL addendum": real codegen, executed under a real python3 ──
//
// Unlike the base cut above, the addendum has no `ArrayLit`/`Range`-style
// node that produces a genuine RANK-1 vector directly (`ArrayLit` always
// lowers through `from_rows`, which is rank-2; `Range` is a rank-2 `1 x n`
// row per MATLAB's own colon convention) -- so these tests build rank-1
// operands by chaining `Ravel` (or another addendum node that itself
// returns rank-1, e.g. `Shape`/`IndexGenerator`/`Catenate`) over an
// `ArrayLit`, exactly the way a real APL frontend's own lowering would
// have to.

/// Run emitted Python that is expected to FAIL (an uncaught, typed
/// `ValueError` propagating out of `main()`), proving a malformed/DoS-
/// shaped input is cleanly REJECTED -- inverting `run_array_program`'s
/// usual "must succeed" assumption. A silent, zero-exit-code "success"
/// here would mean the hazardous input was silently accepted instead of
/// raising -- exactly the failure mode this helper exists to catch.
fn run_array_program_expecting_failure(m: &Module, tag: &str, expected_stderr_substring: &str) {
    let exe = match ["python3", "python"].into_iter().find(|e| python_is_runnable(e)) {
        Some(exe) => exe,
        None => {
            eprintln!("skip: no python on PATH for `{tag}`");
            return;
        }
    };
    let source = semantic_ir_to_python::compile(m).expect("python emit").source;
    let py_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../python");
    let pythonpath = std::env::join_paths([
        py_root.join("sir-runtime-core/src"),
        py_root.join("sir-runtime-pairs/src"),
        py_root.join("sir-runtime-oop/src"),
        py_root.join("sir-runtime-range/src"),
        py_root.join("sir-runtime-regex/src"),
        py_root.join("sir-runtime-exceptions/src"),
        py_root.join("sir-runtime-array/src"),
    ])
    .expect("join PYTHONPATH");
    let file = std::env::temp_dir().join(format!("sir_py_array_fail_{}_{}.py", std::process::id(), tag));
    std::fs::write(&file, &source).expect("write temp python");
    let out = std::process::Command::new(exe)
        .arg(&file)
        .env("PYTHONPATH", &pythonpath)
        .output()
        .expect("spawn python");
    let _ = std::fs::remove_file(&file);
    assert!(
        !out.status.success(),
        "expected python to exit non-zero for `{tag}`, but it succeeded:\n{source}"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains(expected_stderr_substring),
        "got stderr:\n{stderr}\n--- source ---\n{source}"
    );
}

fn ravel(target: Expr) -> Expr {
    Expr::Ravel { target: Box::new(target), span: s() }
}

#[test]
fn reduce_of_a_vector_left_folds_across_all_elements() {
    // +/1 2 3 4 = 10. The vector comes from `Ravel`-ing a 1x4 `ArrayLit`
    // (the addendum's own way to obtain a genuine rank-1 operand -- see
    // this section's module doc comment).
    let v = ravel(array_lit(vec![vec![ilit(1), ilit(2), ilit(3), ilit(4)]]));
    let r = Expr::Reduce { op: ElementwiseOpKind::Add, target: Box::new(v), span: s() };
    let m = array_module(vec![
        let_binding("r", r),
        print_stmt(index_get(local("r"), vec![scalar(0)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "10\n"),
        None => eprintln!("skip: no python on PATH"),
    }
}

#[test]
fn reduce_of_a_matrix_folds_each_row_independently() {
    // [[1,2,3],[4,5,6]] -- row 0 sums to 6, row 1 sums to 15. A
    // column/row indexing mixup in the column-major fold would instead
    // sum the COLUMNS (1+4=5, 2+5=7, 3+6=9) -- this pins the CORRECT,
    // row-independent answer, matching the reference implementation's own
    // "single easiest place to introduce a wrong-answer bug" warning.
    let a = array_lit(vec![vec![ilit(1), ilit(2), ilit(3)], vec![ilit(4), ilit(5), ilit(6)]]);
    let r = Expr::Reduce { op: ElementwiseOpKind::Add, target: Box::new(a), span: s() };
    let m = array_module(vec![
        let_binding("r", r),
        print_stmt(index_get(local("r"), vec![scalar(0)])),
        print_stmt(index_get(local("r"), vec![scalar(1)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "6\n15\n"),
        None => eprintln!("skip: no python on PATH"),
    }
}

#[test]
fn scan_of_a_vector_keeps_every_intermediate_result() {
    // +\1 2 3 4 = 1 3 6 10.
    let v = ravel(array_lit(vec![vec![ilit(1), ilit(2), ilit(3), ilit(4)]]));
    let sc = Expr::Scan { op: ElementwiseOpKind::Add, target: Box::new(v), span: s() };
    let m = array_module(vec![
        let_binding("sc", sc),
        print_stmt(index_get(local("sc"), vec![scalar(0)])),
        print_stmt(index_get(local("sc"), vec![scalar(3)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "1\n10\n"),
        None => eprintln!("skip: no python on PATH"),
    }
}

#[test]
fn outer_product_of_two_vectors() {
    // [1, 2] outer-times [10, 20, 30] -> [[10, 20, 30], [20, 40, 60]].
    let a = ravel(array_lit(vec![vec![ilit(1), ilit(2)]]));
    let b = ravel(array_lit(vec![vec![ilit(10), ilit(20), ilit(30)]]));
    let o = Expr::OuterProduct {
        op: ElementwiseOpKind::Mul,
        lhs: Box::new(a),
        rhs: Box::new(b),
        span: s(),
    };
    let m = array_module(vec![
        let_binding("o", o),
        print_stmt(index_get(local("o"), vec![scalar(0), scalar(0)])),
        print_stmt(index_get(local("o"), vec![scalar(1), scalar(2)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "10\n60\n"),
        None => eprintln!("skip: no python on PATH"),
    }
}

#[test]
fn shape_of_a_scalar_is_the_empty_vector_not_a_scalar() {
    // Critical APL semantics: `⍴5` is a length-0 vector, NOT a rank-0
    // scalar. A genuine rank-0 NDArray doesn't exist as a base-cut
    // literal (see this section's module doc comment), so it is obtained
    // here via `Reduce` on a one-element vector. The raw `NDArray` is
    // printed directly (via its `__repr__` fallback in `to_display`) so
    // its `shape` field -- not just its element count -- is checked.
    let one_elem_vec = ravel(array_lit(vec![vec![ilit(5)]]));
    let scalar_expr = Expr::Reduce { op: ElementwiseOpKind::Add, target: Box::new(one_elem_vec), span: s() };
    let sh = Expr::Shape { target: Box::new(scalar_expr), span: s() };
    let m = array_module(vec![print_stmt(sh)]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "NDArray(shape=(0,), data=[])\n"),
        None => eprintln!("skip: no python on PATH"),
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
        None => eprintln!("skip: no python on PATH"),
    }
}

#[test]
fn reshape_transposes_row_major_fill_into_column_major_storage() {
    // 2x3 -> 3x2. Source raveled row-major is [1, 2, 3, 4, 5, 6]; APL
    // fills the LAST axis fastest (row-major), so the 3x2 target's rows
    // must be (1,2), (3,4), (5,6) -- NOT (1,4), (2,5), (3,6) (the silent
    // transpose a naive column-major-order fill would produce instead).
    let a = array_lit(vec![vec![ilit(1), ilit(2), ilit(3)], vec![ilit(4), ilit(5), ilit(6)]]);
    let shape_vec = ravel(array_lit(vec![vec![ilit(3), ilit(2)]]));
    let r = Expr::Reshape { shape: Box::new(shape_vec), target: Box::new(a), span: s() };
    let m = array_module(vec![
        let_binding("r", r),
        print_stmt(index_get(local("r"), vec![scalar(0), scalar(0)])),
        print_stmt(index_get(local("r"), vec![scalar(0), scalar(1)])),
        print_stmt(index_get(local("r"), vec![scalar(2), scalar(1)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "1\n2\n6\n"),
        None => eprintln!("skip: no python on PATH"),
    }
}

#[test]
fn index_generator_is_one_based_unlike_index_get_index_set() {
    // ⍳4 = [1, 2, 3, 4] -- 1-based, unlike every 0-based index elsewhere
    // in this domain (`IndexGet`/`IndexSet`).
    let g = Expr::IndexGenerator { count: Box::new(ilit(4)), span: s() };
    let m = array_module(vec![
        let_binding("g", g),
        print_stmt(index_get(local("g"), vec![scalar(0)])),
        print_stmt(index_get(local("g"), vec![scalar(3)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "1\n4\n"),
        None => eprintln!("skip: no python on PATH"),
    }
}

#[test]
fn index_of_found_and_not_found_cases() {
    // 10 20 30 ⍳ 20 = 2 (1-based, found). 10 20 30 ⍳ 99 = 4
    // (haystack.length + 1 -- "not found" is a valid, always-in-range
    // position, never -1/undefined).
    let haystack = ravel(array_lit(vec![vec![ilit(10), ilit(20), ilit(30)]]));
    let found = Expr::IndexOf { haystack: Box::new(haystack.clone()), needle: Box::new(ilit(20)), span: s() };
    let not_found = Expr::IndexOf { haystack: Box::new(haystack), needle: Box::new(ilit(99)), span: s() };
    let m = array_module(vec![
        print_stmt(index_get(found, vec![scalar(0)])),
        print_stmt(index_get(not_found, vec![scalar(0)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "2\n4\n"),
        None => eprintln!("skip: no python on PATH"),
    }
}

#[test]
fn ravel_of_a_matrix_flattens_row_major() {
    // [[1,2,3],[4,5,6]] raveled row-major is [1,2,3,4,5,6] -- NOT the raw
    // column-major storage order ([1,4,2,5,3,6]). Reading position 1 (the
    // second element) distinguishes the two: row-major gives 2,
    // column-major would give 4.
    let a = array_lit(vec![vec![ilit(1), ilit(2), ilit(3)], vec![ilit(4), ilit(5), ilit(6)]]);
    let r = ravel(a);
    let m = array_module(vec![
        let_binding("r", r),
        print_stmt(index_get(local("r"), vec![scalar(1)])),
        print_stmt(index_get(local("r"), vec![scalar(5)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "2\n6\n"),
        None => eprintln!("skip: no python on PATH"),
    }
}

#[test]
fn catenate_of_two_vectors() {
    let a = ravel(array_lit(vec![vec![ilit(1), ilit(2)]]));
    let b = ravel(array_lit(vec![vec![ilit(3), ilit(4)]]));
    let c = Expr::Catenate { lhs: Box::new(a), rhs: Box::new(b), span: s() };
    let m = array_module(vec![
        let_binding("c", c),
        print_stmt(index_get(local("c"), vec![scalar(0)])),
        print_stmt(index_get(local("c"), vec![scalar(3)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "1\n4\n"),
        None => eprintln!("skip: no python on PATH"),
    }
}

#[test]
fn catenate_of_two_matrices_with_equal_row_counts() {
    // [[1,2],[3,4]] , [[5],[6]] -> [[1,2,5],[3,4,6]] (column/last-axis
    // catenate).
    let a = array_lit(vec![vec![ilit(1), ilit(2)], vec![ilit(3), ilit(4)]]);
    let b = array_lit(vec![vec![ilit(5)], vec![ilit(6)]]);
    let c = Expr::Catenate { lhs: Box::new(a), rhs: Box::new(b), span: s() };
    let m = array_module(vec![
        let_binding("c", c),
        print_stmt(index_get(local("c"), vec![scalar(0), scalar(2)])),
        print_stmt(index_get(local("c"), vec![scalar(1), scalar(2)])),
    ]);
    match run_array_program(&m) {
        Some(out) => assert_eq!(out, "5\n6\n"),
        None => eprintln!("skip: no python on PATH"),
    }
}

/// SECURITY regression, mirroring the exact bug class Slice 2's own
/// security review found in `matmul` (validating only the OUTPUT shape,
/// not a shared dimension fed by two INDEPENDENT operands): `index_of`'s
/// work is O(len(haystack) * len(needle)) -- each operand is individually
/// tiny relative to `MAX_ELEMENTS` (1 << 26 ~= 67,108,864), but their
/// PRODUCT (100,000,000 for two length-10,000 vectors) exceeds it. Must
/// raise a clean `ValueError` from `checked_shape_size` *before* the O(n²)
/// scan ever runs -- proven here by an actual non-zero exit from a real
/// `python3` process, not just a Rust-side assertion. Each vector is built
/// via `Range` + `Ravel` (not a 10,000-element `ArrayLit`) so the emitted
/// program -- not this test's own Rust source -- does the materializing.
#[test]
fn index_of_product_dos_guard_rejects_two_individually_legal_operands() {
    let long_vec = || {
        ravel(Expr::Range {
            start: Box::new(ilit(1)),
            step: None,
            stop: Box::new(ilit(10_000)),
            span: s(),
        })
    };
    let m = array_module(vec![let_binding(
        "r",
        Expr::IndexOf { haystack: Box::new(long_vec()), needle: Box::new(long_vec()), span: s() },
    )]);
    run_array_program_expecting_failure(
        &m,
        "index_of_product_dos",
        "exceeds the",
    );
}
