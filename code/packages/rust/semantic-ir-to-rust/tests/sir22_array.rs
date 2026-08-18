//! SIR22 base-cut execution proof: `ArrayLit`/`Range`/`MatMul`/
//! `ElementwiseOp`/`Transpose`/`IndexGet`/`Stmt::IndexSet` on the Rust
//! backend — hand-builds a module calling each node directly (bypassing
//! the frontend, since no frontend targets this backend for SIR22 yet),
//! emits Rust, compiles it with a real `rustc`, runs the binary, and
//! asserts stdout. Mirrors `compile_and_run_floats.rs`'s pattern; skips
//! (does not fail) when no `rustc` is on `PATH` or no usable linker is
//! present, exactly like every other `compile_and_run_*` test in this
//! crate.
//!
//! Ported from `semantic-ir-to-javascript`'s own already-proven
//! `tests/sir22_array.rs` (and cross-checked against
//! `semantic-ir-to-ruby`'s own port of the same suite) — same worked
//! examples, adapted to this backend's own value-type convention: this
//! port's `array_*` runtime (`runtime.rs`) stores every element as `f64`
//! (mirroring the JS backend's `Float64Array`, not Ruby's Int/Float-
//! preserving choice — see `runtime.rs`'s own module doc for why), so
//! `Expr::IndexGet` on a scalar position always yields a `Value::Float`.
//! Every printed assertion below therefore expects the `.0`-suffixed
//! float rendering (`format_float` in `runtime.rs`), e.g. `"19.0"` rather
//! than `"19"` — a deliberate, documented divergence from the Ruby
//! reference's Integer-preserving output, not a bug.
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

// ── SIR22 "APL addendum": deferred, rejected cleanly (compile-time) ────

#[test]
fn reduce_node_is_rejected_cleanly_not_a_compile_time_panic() {
    // `Reduce` shares NDArrays/MatrixOps/ArrayColumnMajor with the base
    // cut, so the ordinary feature-flag check alone can't reject it --
    // proves the dedicated `reject_sir22_addendum` pre-emit scan does,
    // with a clean `Err`, not an `emit_expr` internal-bug panic.
    let target = array_lit(vec![vec![ilit(1), ilit(2), ilit(3)]]);
    let m = array_module(vec![print_stmt(Expr::Reduce {
        op: ElementwiseOpKind::Add,
        target: Box::new(target),
        span: s(),
    })]);
    let err = semantic_ir_to_rust::compile(&m).expect_err("Reduce must be rejected, not emitted");
    assert!(
        err.message.contains("Reduce"),
        "error should name the rejected node: {}",
        err.message
    );
}
