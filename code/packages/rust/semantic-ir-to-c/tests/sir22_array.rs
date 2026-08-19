//! SIR22 array/matrix base-cut execution proof: `ArrayLit`/`Range`/
//! `MatMul`/`ElementwiseOp`/`Transpose`/`IndexGet`/`Stmt::IndexSet` on the C
//! backend — hand-builds a module calling each node directly (bypassing the
//! frontend, since no frontend targets this backend for SIR22 yet), emits
//! C, compiles with a real gcc/clang-style compiler, runs, asserts stdout.
//! Skips gracefully when no `cc` is present, mirroring
//! `compile_and_run_division_ops.rs`'s identical `find_cc`/`compile_and_link`
//! pattern (copied verbatim below, including its unique-per-process-and-
//! counter temp filenames — the exact race this session's own sibling test
//! files hit and fixed earlier in this arc).
//!
//! Ported from `semantic-ir-to-javascript`'s own already-proven
//! `tests/sir22_array.rs` (same worked examples as the sibling Ruby port's
//! `tests/sir22_array.rs`), adapted to THIS backend's own value-type
//! convention: unlike the Ruby port (which preserves native Integer/Float
//! propagation) this C port stores every `SirNDArray` element as a plain
//! `double` and always reads one back out as `_sir_float` — see
//! `runtime.rs`'s "SIR22 array/matrix domain" module doc for why — so an
//! all-integer computation here prints WITH a trailing ".0"
//! (`_sir_fmt_float`'s existing convention for every Float in this
//! backend), unlike the Ruby port's int-preserving bare `19`.
//!
//! Every test reads back a SCALAR element via `IndexGet` (top-left/
//! bottom-right, etc.) — sidesteps needing a full `[1 2; 3 4]`-style
//! `SirNDArray` display/format story, out of this slice's scope, exactly
//! like the JS/Ruby references' own test suites.

use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use semantic_ir::{
    Block, EffectSet, ElementwiseOpKind, Expr, Feature, FeatureManifest, Function, IndexArg,
    Metadata, Module, Scope, Span, Stmt, CURRENT_SIR_VERSION,
};

// ── compile/run harness — copied from `compile_and_run_division_ops.rs` ──

fn find_cc() -> Option<String> {
    if let Ok(cc) = std::env::var("SIR_CC") {
        if !cc.trim().is_empty() {
            return Some(cc);
        }
    }
    for cand in ["cc", "clang", "gcc"] {
        if Command::new(cand)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Some(cand.to_string());
        }
    }
    None
}

static COUNTER: AtomicU32 = AtomicU32::new(0);

fn compile_and_link(module: &Module) -> Option<(PathBuf, String)> {
    let cc = find_cc()?;
    let artifact = semantic_ir_to_c::compile(module).expect("C backend compile (no panic)");
    let dir = std::env::temp_dir();
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let stem = format!("sirc_array_{}_{}", std::process::id(), n);
    let cpath: PathBuf = dir.join(format!("{stem}.c"));
    let exe: PathBuf = dir.join(format!("{stem}{}", std::env::consts::EXE_SUFFIX));
    std::fs::File::create(&cpath)
        .and_then(|mut f| f.write_all(artifact.source.as_bytes()))
        .expect("write .c");
    let out = Command::new(&cc)
        .args(["-std=c99", "-Wall", "-Werror=unused-variable", "-o"])
        .arg(&exe)
        .arg(&cpath)
        .arg("-lm")
        .output()
        .expect("spawn cc");
    assert!(
        out.status.success(),
        "compile failed:\n{}\n--- source ---\n{}",
        String::from_utf8_lossy(&out.stderr),
        artifact.source
    );
    Some((exe, artifact.source))
}

fn run(module: &Module) -> Option<String> {
    let (exe, _src) = compile_and_link(module)?;
    let r = Command::new(&exe).output().expect("run");
    assert!(r.status.success(), "run failed (exit {:?})", r.status.code());
    Some(String::from_utf8_lossy(&r.stdout).replace("\r\n", "\n"))
}

// ── module-building helpers ──────────────────────────────────────────────

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
fn bc(name: &str, args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall { name: name.into(), args, effects: EffectSet::PURE, span: s() }
}
/// `puts(arg)` — prints `arg` followed by a newline, via `__sys_write__`
/// (`"per_value"` terminator, `unpack_arrays: true`), mirroring
/// `compile_and_run_division_ops.rs`'s identical helper.
fn puts(arg: Expr) -> Stmt {
    Stmt::ExprStmt {
        expr: bc(
            "__sys_write__",
            vec![
                Expr::StrLit { value: "stdout".into(), span: s() },
                Expr::StrLit { value: "per_value".into(), span: s() },
                Expr::BoolLit { value: true, span: s() },
                arg,
            ],
        ),
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
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        }],
        globals: vec![],
        metadata: Metadata::new().with_sir_version(CURRENT_SIR_VERSION),
        span: s(),
    }
}

// ── base cut: real compile-and-run proof ─────────────────────────────────

#[test]
fn matmul_of_two_by_two_matrices_computes_the_right_product() {
    // [1 2; 3 4] * [5 6; 7 8] = [19 22; 43 50] (standard matrix product).
    // Every NDArray element is a `double` in this port (see the module doc
    // above), so the printed result carries a trailing ".0".
    let a = array_lit(vec![vec![ilit(1), ilit(2)], vec![ilit(3), ilit(4)]]);
    let b = array_lit(vec![vec![ilit(5), ilit(6)], vec![ilit(7), ilit(8)]]);
    let product = Expr::MatMul { lhs: Box::new(a), rhs: Box::new(b), span: s() };
    let m = array_module(vec![
        let_binding("p", product),
        puts(index_get(local("p"), vec![scalar(0), scalar(0)])),
        puts(index_get(local("p"), vec![scalar(1), scalar(1)])),
    ]);
    match run(&m) {
        Some(out) => assert_eq!(out, "19.0\n50.0\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn elementwise_mul_with_a_bare_scalar_operand_broadcasts() {
    // MATLAB `A .* 2` -- matlab-to-semantic-ir emits the `2` as a bare
    // IntLit operand, unwrapped (not an ArrayLit), when exactly one side is
    // scalar. This is the exact shape `_sir_array_coerce`'s coercion in
    // runtime.rs exists for; a regression there fails loudly (dereferencing
    // a non-`SIR_ARRAY` union member) instead of computing [2 4; 6 8].
    let a = array_lit(vec![vec![ilit(1), ilit(2)], vec![ilit(3), ilit(4)]]);
    let scaled = Expr::ElementwiseOp {
        op: ElementwiseOpKind::Mul,
        lhs: Box::new(a),
        rhs: Box::new(ilit(2)),
        span: s(),
    };
    let m = array_module(vec![
        let_binding("sc", scaled),
        puts(index_get(local("sc"), vec![scalar(0), scalar(0)])),
        puts(index_get(local("sc"), vec![scalar(1), scalar(1)])),
    ]);
    match run(&m) {
        Some(out) => assert_eq!(out, "2.0\n8.0\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn elementwise_div_always_true_divides_even_on_integer_operands() {
    // `ElementwiseOp(Div, ...)` always real-divides over `double`s
    // (`_sir_array_apply_op`'s `SIR_EW_DIV` case) -- distinct from this
    // backend's OWN bare `/` (`_sir_divide_v`), which FLOORS toward
    // negative infinity when both operands happen to be `SIR_INT`. A
    // regression that routed this op through `_sir_divide_v` instead would
    // print "3" (floored), not "3.5".
    let a = array_lit(vec![vec![ilit(7)]]);
    let divided = Expr::ElementwiseOp {
        op: ElementwiseOpKind::Div,
        lhs: Box::new(a),
        rhs: Box::new(ilit(2)),
        span: s(),
    };
    let m = array_module(vec![
        let_binding("d", divided),
        puts(index_get(local("d"), vec![scalar(0), scalar(0)])),
    ]);
    match run(&m) {
        Some(out) => assert_eq!(out, "3.5\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn transpose_of_a_two_by_three_matrix_swaps_rows_and_columns() {
    // [1 2 3; 4 5 6]' = [1 4; 2 5; 3 6]
    let a = array_lit(vec![vec![ilit(1), ilit(2), ilit(3)], vec![ilit(4), ilit(5), ilit(6)]]);
    let t = Expr::Transpose { target: Box::new(a), conjugate: true, span: s() };
    let m = array_module(vec![
        let_binding("t", t),
        puts(index_get(local("t"), vec![scalar(0), scalar(1)])),
        puts(index_get(local("t"), vec![scalar(2), scalar(1)])),
    ]);
    match run(&m) {
        Some(out) => assert_eq!(out, "4.0\n6.0\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn range_materializes_a_row_vector_read_by_linear_index() {
    // 1:2:9 -> [1 3 5 7 9], a 1x5 row vector. A single index argument reads
    // linearly (rank-1 IndexGet).
    let r = Expr::Range {
        start: Box::new(ilit(1)),
        step: Some(Box::new(ilit(2))),
        stop: Box::new(ilit(9)),
        span: s(),
    };
    let m = array_module(vec![
        let_binding("r", r),
        puts(index_get(local("r"), vec![scalar(0)])),
        puts(index_get(local("r"), vec![scalar(4)])),
    ]);
    match run(&m) {
        Some(out) => assert_eq!(out, "1.0\n9.0\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn range_with_default_step_materializes_a_unit_stride_row() {
    // 3:6 (no `step` field, None -> default 1) -> [3 4 5 6].
    let r = Expr::Range { start: Box::new(ilit(3)), step: None, stop: Box::new(ilit(6)), span: s() };
    let m = array_module(vec![
        let_binding("r", r),
        puts(index_get(local("r"), vec![scalar(0)])),
        puts(index_get(local("r"), vec![scalar(3)])),
    ]);
    match run(&m) {
        Some(out) => assert_eq!(out, "3.0\n6.0\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn whole_selector_reads_an_entire_row() {
    // A(1, :) on [1 2; 3 4] reads the whole second row [3 4], then a scalar
    // linear IndexGet reads its second element.
    let a = array_lit(vec![vec![ilit(1), ilit(2)], vec![ilit(3), ilit(4)]]);
    let row = index_get(a, vec![scalar(1), IndexArg::Whole]);
    let m = array_module(vec![
        let_binding("row", row),
        puts(index_get(local("row"), vec![scalar(1)])),
    ]);
    match run(&m) {
        Some(out) => assert_eq!(out, "4.0\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn index_set_mutates_in_place() {
    // A(1, 1) = 99 on [1 2; 3 4] -- IndexSet is a Stmt (in-place mutation),
    // not a pure Expr, per the SIR22 spec.
    let a = array_lit(vec![vec![ilit(1), ilit(2)], vec![ilit(3), ilit(4)]]);
    let m = array_module(vec![
        let_binding("a", a),
        Stmt::IndexSet {
            target: Box::new(local("a")),
            indices: vec![scalar(1), scalar(1)],
            value: Box::new(ilit(99)),
            span: s(),
        },
        puts(index_get(local("a"), vec![scalar(1), scalar(1)])),
    ]);
    match run(&m) {
        Some(out) => assert_eq!(out, "99.0\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn index_set_with_a_range_selector_overwrites_a_sub_range_linearly() {
    // A row vector 1:4 ([1 2 3 4]); A([1 2]) = [50 60] (a Range index arg
    // reusing an Expr::Range, resolved to positions [1, 2]) overwrites the
    // middle two elements in place -- exercises the `IndexArg::Range`
    // resolution path (`_sir_array_resolve_positions`'s `SIR_IDXARG_RANGE`
    // case), not just `Scalar`/`Whole`.
    let r = Expr::Range { start: Box::new(ilit(1)), step: None, stop: Box::new(ilit(4)), span: s() };
    let sel = Expr::Range { start: Box::new(ilit(1)), step: None, stop: Box::new(ilit(2)), span: s() };
    let replacement = array_lit(vec![vec![ilit(50), ilit(60)]]);
    let m = array_module(vec![
        let_binding("a", r),
        Stmt::IndexSet {
            target: Box::new(local("a")),
            indices: vec![IndexArg::Range(Box::new(sel))],
            value: Box::new(replacement),
            span: s(),
        },
        puts(index_get(local("a"), vec![scalar(0)])),
        puts(index_get(local("a"), vec![scalar(1)])),
        puts(index_get(local("a"), vec![scalar(2)])),
        puts(index_get(local("a"), vec![scalar(3)])),
    ]);
    match run(&m) {
        Some(out) => assert_eq!(out, "1.0\n50.0\n60.0\n4.0\n"),
        None => eprintln!("skip: no cc"),
    }
}

// ── SIR22 "APL addendum": real compile-and-run proof (Phase A Slice 3) ───
//
// The nine addendum node kinds now have real `_sir_array_*` runtime
// backing (see `runtime.rs`'s "SIR22 addendum" section) instead of the
// clean-rejection stubs Slice 2 left here — every test below hand-builds
// a module calling one of the nine directly, compiles+runs it, and reads
// results back through `IndexGet` (matching every base-cut test above),
// same "no frontend targets this backend for SIR22 yet" caveat.

#[test]
fn reduce_of_a_row_vector_folds_across_all_elements() {
    // [1 2 3 4], Reduce(Add) -> a single folded value (10), read back
    // through a 1-element linear IndexGet.
    let target = array_lit(vec![vec![ilit(1), ilit(2), ilit(3), ilit(4)]]);
    let reduced = Expr::Reduce { op: ElementwiseOpKind::Add, target: Box::new(target), span: s() };
    let m = array_module(vec![
        let_binding("r", reduced),
        puts(index_get(local("r"), vec![scalar(0)])),
    ]);
    match run(&m) {
        Some(out) => assert_eq!(out, "10.0\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn reduce_of_a_matrix_folds_each_row_independently() {
    // [1 2 3; 4 5 6], Reduce(Add) -> [6, 15] (row 0 sums to 6, row 1 to
    // 15) -- proves the row-independent fold AND the column-major
    // `col * rows + row` indexing the doc comment calls out as "the
    // single easiest place to introduce a wrong-answer bug": a
    // row/col swap here would silently transpose the input before
    // folding and produce a completely different pair of sums.
    let target = array_lit(vec![
        vec![ilit(1), ilit(2), ilit(3)],
        vec![ilit(4), ilit(5), ilit(6)],
    ]);
    let reduced = Expr::Reduce { op: ElementwiseOpKind::Add, target: Box::new(target), span: s() };
    let m = array_module(vec![
        let_binding("r", reduced),
        puts(index_get(local("r"), vec![scalar(0)])),
        puts(index_get(local("r"), vec![scalar(1)])),
    ]);
    match run(&m) {
        Some(out) => assert_eq!(out, "6.0\n15.0\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn scan_of_a_row_vector_keeps_every_running_fold() {
    // [1 2 3 4], Scan(Add) -> [1, 3, 6, 10] (running sums, same shape as
    // input) -- unlike Reduce, every intermediate is kept.
    let target = array_lit(vec![vec![ilit(1), ilit(2), ilit(3), ilit(4)]]);
    let scanned = Expr::Scan { op: ElementwiseOpKind::Add, target: Box::new(target), span: s() };
    let m = array_module(vec![
        let_binding("s", scanned),
        puts(index_get(local("s"), vec![scalar(0)])),
        puts(index_get(local("s"), vec![scalar(3)])),
    ]);
    match run(&m) {
        Some(out) => assert_eq!(out, "1.0\n10.0\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn outer_product_of_two_vectors_computes_every_pairwise_product() {
    // [1 2] outer* [10 20] -> [[10 20]; [20 40]] (2x2): out[i][j] =
    // a[i] * b[j]. Read all four corners via 2-arg IndexGet.
    let lhs = array_lit(vec![vec![ilit(1), ilit(2)]]);
    let rhs = array_lit(vec![vec![ilit(10), ilit(20)]]);
    let outer = Expr::OuterProduct {
        op: ElementwiseOpKind::Mul,
        lhs: Box::new(lhs),
        rhs: Box::new(rhs),
        span: s(),
    };
    let m = array_module(vec![
        let_binding("o", outer),
        puts(index_get(local("o"), vec![scalar(0), scalar(0)])),
        puts(index_get(local("o"), vec![scalar(0), scalar(1)])),
        puts(index_get(local("o"), vec![scalar(1), scalar(0)])),
        puts(index_get(local("o"), vec![scalar(1), scalar(1)])),
    ]);
    match run(&m) {
        Some(out) => assert_eq!(out, "10.0\n20.0\n20.0\n40.0\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn shape_of_a_scalar_is_the_empty_vector_not_a_scalar() {
    // `Shape(5)` must be a length-0 VECTOR ("⍴5" is "⍳0"-shaped), not a
    // scalar itself -- the trickiest addendum case to get right. Since
    // every test in this file reads results back via IndexGet (which
    // errors on an out-of-bounds read, so an empty result can't be
    // IndexGet'd directly), assert emptiness INDIRECTLY: take Shape of
    // the Shape -- `Shape(Shape(5))` reads back the ELEMENT COUNT of
    // `Shape(5)` itself (a length-1 vector containing that count, since
    // a 1x0 array has logical rank 1 here), which must be 0.0. A buggy
    // implementation that returned a genuine SCALAR for `Shape(5)`
    // instead of an empty vector would make the outer `Shape` call see
    // rank 0 and (per `_sir_array_shape`'s own `lr == 0` branch) return
    // ANOTHER empty vector -- so this also exercises the "scalar in,
    // empty vector out" path a second, independent way.
    let inner = Expr::Shape { target: Box::new(ilit(5)), span: s() };
    let outer = Expr::Shape { target: Box::new(inner), span: s() };
    let m = array_module(vec![
        let_binding("sh", outer),
        puts(index_get(local("sh"), vec![scalar(0)])),
    ]);
    match run(&m) {
        Some(out) => assert_eq!(out, "0.0\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn shape_of_a_matrix_is_its_dimensions() {
    // A 2x3 matrix's shape is the 2-element vector [2, 3].
    let target = array_lit(vec![
        vec![ilit(1), ilit(2), ilit(3)],
        vec![ilit(4), ilit(5), ilit(6)],
    ]);
    let sh = Expr::Shape { target: Box::new(target), span: s() };
    let m = array_module(vec![
        let_binding("sh", sh),
        puts(index_get(local("sh"), vec![scalar(0)])),
        puts(index_get(local("sh"), vec![scalar(1)])),
    ]);
    match run(&m) {
        Some(out) => assert_eq!(out, "2.0\n3.0\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn reshape_fills_row_major_then_transposes_into_column_major_storage() {
    // Source [1 2 3 4 5 6] (row-major) reshaped to a NON-SQUARE 2x3
    // target must read back as [[1 2 3]; [4 5 6]] (APL fills the target
    // in ROW-major order). This domain stores COLUMN-major, so a naive
    // port that handed the row-major fill straight to the matrix
    // constructor without transposing would silently produce
    // [[1 3 5]; [2 4 6]] instead -- same multiset of values, WRONG
    // positions, which a square reshape could hide but this non-square
    // one cannot (the two layouts disagree at (0, 1): 2 if correct, 3 if
    // the transpose step were skipped).
    let source = array_lit(vec![vec![ilit(1), ilit(2), ilit(3), ilit(4), ilit(5), ilit(6)]]);
    let target_shape = array_lit(vec![vec![ilit(2), ilit(3)]]);
    let reshaped = Expr::Reshape {
        shape: Box::new(target_shape),
        target: Box::new(source),
        span: s(),
    };
    let m = array_module(vec![
        let_binding("m", reshaped),
        puts(index_get(local("m"), vec![scalar(0), scalar(0)])),
        puts(index_get(local("m"), vec![scalar(0), scalar(1)])),
        puts(index_get(local("m"), vec![scalar(1), scalar(0)])),
        puts(index_get(local("m"), vec![scalar(1), scalar(2)])),
    ]);
    match run(&m) {
        Some(out) => assert_eq!(out, "1.0\n2.0\n4.0\n6.0\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn index_generator_produces_a_one_based_run() {
    // `⍳3` is `[1, 2, 3]` -- 1-based, the one deliberate exception to
    // this domain's otherwise-uniform 0-based indexing (ground-truthed
    // directly against `apl-runtime::builtins::index_generator_produces_
    // one_based_run`, not the stale 0-based claim in `semantic-ir`'s own
    // `Expr::IndexGenerator` doc comment).
    let gen = Expr::IndexGenerator { count: Box::new(ilit(3)), span: s() };
    let m = array_module(vec![
        let_binding("g", gen),
        puts(index_get(local("g"), vec![scalar(0)])),
        puts(index_get(local("g"), vec![scalar(2)])),
    ]);
    match run(&m) {
        Some(out) => assert_eq!(out, "1.0\n3.0\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn index_of_reports_a_one_based_position_or_length_plus_one_when_absent() {
    // haystack [10 20 30], needle [20 99]: 20 is at (0-based) position 1
    // -> 1-based 2; 99 is absent -> haystack.length + 1 = 4, NEVER -1.
    let haystack = array_lit(vec![vec![ilit(10), ilit(20), ilit(30)]]);
    let needle = array_lit(vec![vec![ilit(20), ilit(99)]]);
    let idx = Expr::IndexOf { haystack: Box::new(haystack), needle: Box::new(needle), span: s() };
    let m = array_module(vec![
        let_binding("ix", idx),
        puts(index_get(local("ix"), vec![scalar(0)])),
        puts(index_get(local("ix"), vec![scalar(1)])),
    ]);
    match run(&m) {
        Some(out) => assert_eq!(out, "2.0\n4.0\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn ravel_of_a_matrix_flattens_to_row_major_order() {
    // [1 2; 3 4] stores column-major internally (data = [1, 3, 2, 4]);
    // Ravel must read back ROW-major ([1, 2, 3, 4]), proving
    // `_sir_array_flatten_row_major` re-walks "row, then column" rather
    // than handing back the raw column-major buffer (which would give
    // [1, 3, 2, 4] instead).
    let target = array_lit(vec![vec![ilit(1), ilit(2)], vec![ilit(3), ilit(4)]]);
    let raveled = Expr::Ravel { target: Box::new(target), span: s() };
    let m = array_module(vec![
        let_binding("r", raveled),
        puts(index_get(local("r"), vec![scalar(0)])),
        puts(index_get(local("r"), vec![scalar(1)])),
        puts(index_get(local("r"), vec![scalar(2)])),
        puts(index_get(local("r"), vec![scalar(3)])),
    ]);
    match run(&m) {
        Some(out) => assert_eq!(out, "1.0\n2.0\n3.0\n4.0\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn catenate_of_two_vectors_concatenates_regardless_of_matching_length() {
    // [1 2] , [3 4 5] -> [1 2 3 4 5] -- vector catenate has no
    // equal-length requirement (unlike the matrix case below).
    let lhs = array_lit(vec![vec![ilit(1), ilit(2)]]);
    let rhs = array_lit(vec![vec![ilit(3), ilit(4), ilit(5)]]);
    let cat = Expr::Catenate { lhs: Box::new(lhs), rhs: Box::new(rhs), span: s() };
    let m = array_module(vec![
        let_binding("c", cat),
        puts(index_get(local("c"), vec![scalar(0)])),
        puts(index_get(local("c"), vec![scalar(4)])),
    ]);
    match run(&m) {
        Some(out) => assert_eq!(out, "1.0\n5.0\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn catenate_of_two_matrices_with_equal_rows_concatenates_columns() {
    // [1 2; 3 4] , [5; 6] (2x2 , 2x1, equal row counts) -> [1 2 5; 3 4 6]
    // (2x3): column/last-axis catenate.
    let lhs = array_lit(vec![vec![ilit(1), ilit(2)], vec![ilit(3), ilit(4)]]);
    let rhs = array_lit(vec![vec![ilit(5)], vec![ilit(6)]]);
    let cat = Expr::Catenate { lhs: Box::new(lhs), rhs: Box::new(rhs), span: s() };
    let m = array_module(vec![
        let_binding("c", cat),
        puts(index_get(local("c"), vec![scalar(0), scalar(0)])),
        puts(index_get(local("c"), vec![scalar(0), scalar(2)])),
        puts(index_get(local("c"), vec![scalar(1), scalar(2)])),
    ]);
    match run(&m) {
        Some(out) => assert_eq!(out, "1.0\n5.0\n6.0\n"),
        None => eprintln!("skip: no cc"),
    }
}

// ── malformed hand-built shapes: rejected cleanly, not a runtime panic ───

#[test]
fn ragged_array_lit_is_rejected_cleanly_at_compile_time() {
    let ragged = array_lit(vec![vec![ilit(1), ilit(2)], vec![ilit(3)]]);
    let m = array_module(vec![puts(ragged)]);
    let err = semantic_ir_to_c::compile(&m).expect_err("a ragged ArrayLit must be rejected");
    assert!(
        err.message.contains("ragged"),
        "error should mention raggedness: {}",
        err.message
    );
}

#[test]
fn index_get_with_three_indices_is_rejected_cleanly_at_compile_time() {
    let a = array_lit(vec![vec![ilit(1)]]);
    let m = array_module(vec![puts(index_get(a, vec![scalar(0), scalar(0), scalar(0)]))]);
    let err = semantic_ir_to_c::compile(&m).expect_err("a rank-3 IndexGet must be rejected");
    assert!(
        err.message.contains("IndexGet"),
        "error should name the rejected construct: {}",
        err.message
    );
}
