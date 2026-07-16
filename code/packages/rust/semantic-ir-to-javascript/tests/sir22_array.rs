//! End-to-end integration test: hand-built SIR22 nodes → JavaScript →
//! `node`.
//!
//! `src/lib.rs`'s own test module proves the emitted *shape* (exact
//! substring assertions on generated source, mirroring the TypeScript
//! backend's SIR22 tests-to-come). This file proves the emitted
//! *behaviour*: a `MatMul`/`ElementwiseOp`/`Transpose`/`Range`/
//! `IndexGet`/`IndexSet` node, run for real under Node.js, must actually
//! compute the right numbers — not just produce plausible-looking source.
//!
//! Node is optional at test time; when unavailable the test degrades to
//! a no-op rather than failing (mirroring `run_with_node.rs`/
//! `sir23_symbolic.rs`).

use std::path::PathBuf;
use std::process::Command;

use semantic_ir::{
    Block, EffectSet, ElementwiseOpKind, Expr, Feature, FeatureManifest, Function, IndexArg,
    Metadata, Module, Scope, Span, Stmt,
};
use semantic_ir_to_javascript::compile;

fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn sp() -> Span {
    Span::synthetic()
}

fn int(value: i64) -> Expr {
    Expr::IntLit { value, span: sp() }
}

fn local(name: &str) -> Expr {
    Expr::VarRef {
        name: name.into(),
        scope: Scope::Local,
        span: sp(),
    }
}

fn array_lit(rows: Vec<Vec<Expr>>) -> Expr {
    Expr::ArrayLit { rows, span: sp() }
}

fn bc(name: &str, args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall {
        name: name.into(),
        args,
        effects: EffectSet::PURE,
        span: sp(),
    }
}

fn print(arg: Expr) -> Stmt {
    Stmt::ExprStmt {
        expr: bc("print", vec![arg]),
        span: sp(),
    }
}

fn let_binding(name: &str, value: Expr) -> Stmt {
    Stmt::LetBinding {
        name: name.into(),
        sir_type: None,
        value,
        span: sp(),
    }
}

fn module_with_main(stmts: Vec<Stmt>, value: Expr, features: &[Feature]) -> Module {
    Module {
        name: "sir22".into(),
        manifest: FeatureManifest::from_features(features),
        imports: vec![],
        exports: vec![],
        functions: vec![Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts,
                value,
                span: sp(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: sp(),
        }],
        globals: vec![],
        metadata: Metadata::new()
            .with_source_language("handbuilt")
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
        span: sp(),
    }
}

/// Every test in this file uses `NDArrays` + `MatrixOps` + `ArrayColumnMajor`
/// together — a hand-built module never needs a fine-grained subset the way
/// a real frontend's per-construct feature-tracking might.
const ARRAY_FEATURES: &[Feature] = &[
    Feature::NDArrays,
    Feature::MatrixOps,
    Feature::ArrayColumnMajor,
];

fn run_module(module: &Module, tag: &str) -> Option<String> {
    let artifact = compile(module).expect("compile to javascript");
    if !node_available() {
        eprintln!("note: `node` unavailable — skipping execution for `{tag}`");
        return None;
    }
    let mut path: PathBuf = std::env::temp_dir();
    path.push(format!("sir_js_{}_{}.js", tag, std::process::id()));
    std::fs::write(&path, &artifact.source).expect("write temp js");
    let output = Command::new("node")
        .arg(&path)
        .output()
        .expect("spawn node");
    let _ = std::fs::remove_file(&path);
    assert!(
        output.status.success(),
        "node exited non-zero for `{tag}`:\nstdout: {}\nstderr: {}\nsource:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        artifact.source,
    );
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    Some(stdout.trim_end_matches(['\n', '\r']).to_string())
}

#[test]
fn matmul_of_two_by_two_matrices_computes_the_right_product() {
    // [1 2; 3 4] * [5 6; 7 8] = [19 22; 43 50] (standard matrix product).
    // Printed via two scalar IndexGet reads (top-left, bottom-right) --
    // sidesteps needing an NDArray display/format story, which is out of
    // this PR's scope.
    let a = array_lit(vec![vec![int(1), int(2)], vec![int(3), int(4)]]);
    let b = array_lit(vec![vec![int(5), int(6)], vec![int(7), int(8)]]);
    let product = Expr::MatMul {
        lhs: Box::new(a),
        rhs: Box::new(b),
        span: sp(),
    };
    let stmts = vec![
        let_binding("p", product),
        print(Expr::IndexGet {
            target: Box::new(local("p")),
            indices: vec![
                IndexArg::Scalar(Box::new(int(0))),
                IndexArg::Scalar(Box::new(int(0))),
            ],
            span: sp(),
        }),
        print(Expr::IndexGet {
            target: Box::new(local("p")),
            indices: vec![
                IndexArg::Scalar(Box::new(int(1))),
                IndexArg::Scalar(Box::new(int(1))),
            ],
            span: sp(),
        }),
    ];
    let module = module_with_main(stmts, int(0), ARRAY_FEATURES);
    if let Some(stdout) = run_module(&module, "matmul") {
        assert_eq!(stdout, "19\n50");
    }
}

#[test]
fn elementwise_mul_with_a_bare_scalar_operand_broadcasts() {
    // MATLAB `A .* 2` -- matlab-to-semantic-ir emits the `2` as a bare
    // IntLit operand, unwrapped (not an ArrayLit), when exactly one side
    // is scalar. This is the exact shape `toArrayValue`'s coercion in
    // runtime.rs exists for; a regression there crashes with a
    // `TypeError: Cannot read properties of undefined (reading 'length')`
    // instead of computing [2 4; 6 8].
    let a = array_lit(vec![vec![int(1), int(2)], vec![int(3), int(4)]]);
    let scaled = Expr::ElementwiseOp {
        op: ElementwiseOpKind::Mul,
        lhs: Box::new(a),
        rhs: Box::new(int(2)),
        span: sp(),
    };
    let stmts = vec![
        let_binding("s", scaled),
        print(Expr::IndexGet {
            target: Box::new(local("s")),
            indices: vec![
                IndexArg::Scalar(Box::new(int(0))),
                IndexArg::Scalar(Box::new(int(0))),
            ],
            span: sp(),
        }),
        print(Expr::IndexGet {
            target: Box::new(local("s")),
            indices: vec![
                IndexArg::Scalar(Box::new(int(1))),
                IndexArg::Scalar(Box::new(int(1))),
            ],
            span: sp(),
        }),
    ];
    let module = module_with_main(stmts, int(0), ARRAY_FEATURES);
    if let Some(stdout) = run_module(&module, "elementwise_scalar_broadcast") {
        assert_eq!(stdout, "2\n8");
    }
}

#[test]
fn transpose_swaps_rows_and_columns() {
    // transpose([1 2 3; 4 5 6]) = [1 4; 2 5; 3 6] -- read back element
    // (2, 0) (0-based), which should be the original (0, 2) = 3.
    let a = array_lit(vec![
        vec![int(1), int(2), int(3)],
        vec![int(4), int(5), int(6)],
    ]);
    let transposed = Expr::Transpose {
        target: Box::new(a),
        conjugate: false,
        span: sp(),
    };
    let stmts = vec![
        let_binding("t", transposed),
        print(Expr::IndexGet {
            target: Box::new(local("t")),
            indices: vec![
                IndexArg::Scalar(Box::new(int(2))),
                IndexArg::Scalar(Box::new(int(0))),
            ],
            span: sp(),
        }),
    ];
    let module = module_with_main(stmts, int(0), ARRAY_FEATURES);
    if let Some(stdout) = run_module(&module, "transpose") {
        assert_eq!(stdout, "3");
    }
}

#[test]
fn range_materializes_a_row_vector_with_the_matlab_colon_semantics() {
    // 1:2:10 -> [1, 3, 5, 7, 9] (5 elements; MATLAB's colon is inclusive
    // of the stop bound, so this must NOT stop one short). Read back the
    // last element via scalar IndexGet -- proves both the element count
    // (index 4 must be in bounds) and the inclusive-stop value (9, not
    // 7) are right.
    let r = Expr::Range {
        start: Box::new(int(1)),
        step: Some(Box::new(int(2))),
        stop: Box::new(int(10)),
        span: sp(),
    };
    let stmts = vec![
        let_binding("r", r),
        print(Expr::IndexGet {
            target: Box::new(local("r")),
            indices: vec![IndexArg::Scalar(Box::new(int(4)))],
            span: sp(),
        }),
    ];
    let module = module_with_main(stmts, int(0), ARRAY_FEATURES);
    if let Some(stdout) = run_module(&module, "range_scalar_readback") {
        assert_eq!(stdout, "9");
    }
}

#[test]
fn index_set_mutates_in_place() {
    // A(0, 1) = 99 on [1 2; 3 4] -> reading it back must see 99, not the
    // original 2 -- proves IndexSet is a real in-place mutation, not a
    // no-op or a copy-and-discard.
    let a = array_lit(vec![vec![int(1), int(2)], vec![int(3), int(4)]]);
    let stmts = vec![
        let_binding("a", a),
        Stmt::IndexSet {
            target: Box::new(local("a")),
            indices: vec![
                IndexArg::Scalar(Box::new(int(0))),
                IndexArg::Scalar(Box::new(int(1))),
            ],
            value: Box::new(int(99)),
            span: sp(),
        },
        print(Expr::IndexGet {
            target: Box::new(local("a")),
            indices: vec![
                IndexArg::Scalar(Box::new(int(0))),
                IndexArg::Scalar(Box::new(int(1))),
            ],
            span: sp(),
        }),
    ];
    let module = module_with_main(stmts, int(0), ARRAY_FEATURES);
    if let Some(stdout) = run_module(&module, "index_set") {
        assert_eq!(stdout, "99");
    }
}

#[test]
fn index_set_with_whole_column_broadcasts_a_scalar() {
    // A(:, 0) = 7 on [1 2; 3 4] -> column 0 becomes [7; 7]; column 1
    // (untouched) stays [2; 4].
    let a = array_lit(vec![vec![int(1), int(2)], vec![int(3), int(4)]]);
    let stmts = vec![
        let_binding("a", a),
        Stmt::IndexSet {
            target: Box::new(local("a")),
            indices: vec![IndexArg::Whole, IndexArg::Scalar(Box::new(int(0)))],
            value: Box::new(int(7)),
            span: sp(),
        },
        print(Expr::IndexGet {
            target: Box::new(local("a")),
            indices: vec![
                IndexArg::Scalar(Box::new(int(0))),
                IndexArg::Scalar(Box::new(int(0))),
            ],
            span: sp(),
        }),
        print(Expr::IndexGet {
            target: Box::new(local("a")),
            indices: vec![
                IndexArg::Scalar(Box::new(int(1))),
                IndexArg::Scalar(Box::new(int(0))),
            ],
            span: sp(),
        }),
        print(Expr::IndexGet {
            target: Box::new(local("a")),
            indices: vec![
                IndexArg::Scalar(Box::new(int(0))),
                IndexArg::Scalar(Box::new(int(1))),
            ],
            span: sp(),
        }),
    ];
    let module = module_with_main(stmts, int(0), ARRAY_FEATURES);
    if let Some(stdout) = run_module(&module, "index_set_whole_broadcast") {
        assert_eq!(stdout, "7\n7\n2");
    }
}

#[test]
fn matmul_with_non_conformable_shapes_throws_a_catchable_error() {
    // [1 2] (1x2) * [1 2] (1x2) -- inner dimensions (2 vs 1) disagree, so
    // `__Sir.Array.matmul` must throw a plain JS `Error` (with a message
    // naming the disagreement) rather than silently produce a
    // wrong-shaped result. Left uncaught here, so it propagates out of
    // `main()` and node exits non-zero -- inverts `run_module`'s usual
    // success assumption for this one case, since a *clean* rejection is
    // exactly what this test wants to see.
    let a = array_lit(vec![vec![int(1), int(2)]]);
    let b = array_lit(vec![vec![int(1), int(2)]]);
    let stmts = vec![let_binding(
        "p",
        Expr::MatMul {
            lhs: Box::new(a),
            rhs: Box::new(b),
            span: sp(),
        },
    )];
    let module = module_with_main(stmts, int(0), ARRAY_FEATURES);
    let artifact = compile(&module).expect("compile to javascript");
    if !node_available() {
        eprintln!("note: `node` unavailable — skipping execution for `matmul_non_conformable`");
        return;
    }
    let mut path: PathBuf = std::env::temp_dir();
    path.push(format!(
        "sir_js_matmul_non_conformable_{}.js",
        std::process::id()
    ));
    std::fs::write(&path, &artifact.source).expect("write temp js");
    let output = Command::new("node")
        .arg(&path)
        .output()
        .expect("spawn node");
    let _ = std::fs::remove_file(&path);
    assert!(
        !output.status.success(),
        "expected node to exit non-zero on a non-conformable matmul, but it succeeded:\n{}",
        artifact.source
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("matmul: inner dimensions disagree"),
        "got stderr:\n{stderr}"
    );
}
