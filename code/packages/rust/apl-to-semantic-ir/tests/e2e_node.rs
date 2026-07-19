//! End-to-end round-trip: APL → SIR → JavaScript → `node`.
//!
//! Mirrors `matlab-to-semantic-ir`'s own `tests/e2e_node.rs` harness exactly:
//! lower an APL program to SIR with this crate, validate the module, emit
//! JavaScript with the merged `semantic-ir-to-javascript` backend (a
//! dev-dependency), write it to a temp file, and **execute it with
//! `node`**, asserting the printed output.
//!
//! Until `semantic-ir-to-javascript` gained real codegen for the SIR22
//! "APL addendum" (`Reduce`/`Scan`/`OuterProduct`/`Shape`/`Reshape`/
//! `IndexGenerator`/`IndexOf`/`Ravel`/`Catenate`), this file could not exist
//! at all: every one of this crate's own test programs uses at least one of
//! these nine nodes (unlike MATLAB, APL has no purely-scalar-literal escape
//! hatch — see `src/lower.rs`'s module doc comment, point 1 — even `3+4` is
//! an `ElementwiseOp`), and `compile()` used to reject all of them cleanly
//! rather than panic. Now that backend implements real codegen for all nine,
//! this file is the actual, node-executed proof the panic/rejection is gone
//! — not just that `compile()` returns `Ok`, but that the generated program
//! computes the RIGHT numbers.
//!
//! APL auto-prints a bare (non-assignment) top-level expression (see
//! `src/lower.rs`'s "Auto-print, not MATLAB-style suppression" section), so
//! every program below is a single bare expression with no explicit
//! `disp`-equivalent.
//!
//! ## Two representational quirks this file works around
//!
//! 1. **No display-formatting story for a printed vector/matrix was ever a
//!    concern here** — unlike `matlab-to-semantic-ir`'s own `e2e_node.rs`
//!    (which reads back individual elements via `IndexGet`, since MATLAB's
//!    `disp` on an un-indexed array had nowhere to route to), this backend's
//!    `formatSeen` now renders a raw `NDArray` using APL's OWN console
//!    convention (`ArrayRt.display`, a 1:1 port of `apl_runtime::value::
//!    display` — high-minus `¯`, space-separated vector, right-aligned
//!    matrix rows). This was necessary, not optional: APL has NO
//!    bracket-indexing surface syntax at all in this grammar (confirmed
//!    against `code/grammars/apl/apl.grammar` and `apl-to-semantic-ir`'s own
//!    lowering, which never constructs an `Expr::IndexGet`), so
//!    `matlab-to-semantic-ir`'s "read back one scalar via indexing"
//!    workaround is simply not expressible in APL source at all — auto-print
//!    of the whole value is the ONLY way any APL program can ever produce
//!    output. See `semantic-ir-to-javascript`'s `runtime.rs` `formatSeen`
//!    for where this is wired in.
//! 2. **A stranded numeric literal (`1 2 3`) lowers to `Expr::ArrayLit`
//!    with exactly ONE row** (`src/lower.rs`'s `lower_term`), which this
//!    backend's (unchanged, out-of-scope-for-this-PR) base-cut codegen
//!    turns into `__Sir.Array.fromRows([[1, 2, 3]])` — a genuine RANK-2
//!    `[1, 3]` "row matrix" at the runtime-value level, not a true rank-1
//!    `[3]` vector (contrast `apl-runtime`'s OWN tree-walking evaluator,
//!    which builds `Array::from_vec` for the identical source and gets a
//!    true `[3]`). `reduce`/`scan` happen to compute the identical numbers
//!    either way (their rank-2 branch folds/scans each row independently,
//!    and a lone row of length n coincides exactly with a rank-1 fold/scan
//!    of the same n elements), so `+/`/`+\` on stranded literals below are
//!    unaffected. `outer`, however, is scoped to rank <= 1 operands ONLY
//!    (matching `array_runtime::ops::outer`'s own identical restriction) —
//!    two `[1, n]`-shaped stranded literals both fail that check and throw.
//!    The outer-product test below therefore builds its operands from `⍳`
//!    (this crate's own `IndexGenerator`, which constructs a genuine rank-1
//!    `[n]` by direct `ndarray([n], ...)` construction, sidestepping
//!    `ArrayLit`/`fromRows` entirely) instead of two bare stranded
//!    literals. Dyadic `⍴` (reshape)'s shape ARGUMENT hits the identical
//!    wall — `reshape` rejects a rank > 1 shape argument (mirroring
//!    `apl_runtime::builtins::reshape`'s own check), and a bare `2 3`
//!    stranded literal IS rank 2 (`[1, 2]`) at the runtime-value level —
//!    so the matrix-construction tests below use `,2 3` (ravel of the
//!    stranded literal, which — like `outer` above — sidesteps the issue
//!    because `ravel` always constructs a genuine rank-1 result
//!    regardless of its input's rank) as the shape argument instead of the
//!    bare literal. This ArrayLit-representation gap is pre-existing
//!    (shipped in this crate's v0.1.0/v0.1.1, unrelated to and unchanged
//!    by the codegen work that unblocked this file) and out of scope to
//!    fix here — flagged separately, not patched around silently.

use std::fs::OpenOptions;
use std::io::Write as _;
use std::process::Command;

use apl_to_semantic_ir::compile_source;

fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn run_via_node(name: &str, src: &str) -> String {
    let module = compile_source(src, "prog").unwrap_or_else(|e| panic!("lowering failed: {e}"));
    let report = semantic_ir::validate(&module);
    assert!(
        report.is_ok(),
        "SIR validation failed for {name}: {:?}",
        report.issues
    );
    let artifact =
        semantic_ir_to_javascript::compile(&module).expect("backend emit should succeed");

    let mut path = std::env::temp_dir();
    path.push(format!("apl_sir_e2e_{name}_{}.js", std::process::id()));
    // `create_new` fails if the path already exists (including as a
    // symlink) instead of following it -- unlike `std::fs::write`, which
    // truncates through a symlink at this predictable, shared-temp-dir
    // path. Each test uses a unique name+PID, so this should never
    // legitimately collide; if it does, failing loudly is correct for a
    // test rather than silently overwriting whatever the path pointed to.
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .expect("create temp js (create_new, not following an existing symlink)");
    file.write_all(artifact.source.as_bytes())
        .expect("write temp js");
    drop(file);

    let output = Command::new("node").arg(&path).output().expect("spawn node");
    let _ = std::fs::remove_file(&path);

    assert!(
        output.status.success(),
        "node failed for {name}: stderr=\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn reduce_add_on_a_stranded_vector_runs_in_node() {
    if !node_available() {
        eprintln!("skipping reduce_add_on_a_stranded_vector_runs_in_node: `node` not available");
        return;
    }
    // `+/1 2 3 4` -- APL reduce with Add, the textbook `+/` example.
    let out = run_via_node("reduce_add", "+/1 2 3 4\n");
    assert_eq!(out, "10");
}

#[test]
fn reduce_with_a_non_add_op_runs_in_node() {
    if !node_available() {
        eprintln!("skipping reduce_with_a_non_add_op_runs_in_node: `node` not available");
        return;
    }
    // `⌈/3 1 4 1 5` -- max-reduce, proving the op dispatch inside
    // `__Sir.Array.reduce` isn't hardcoded to Add (it reuses `applyOp`,
    // the same dispatch table `elementwise` uses).
    let out = run_via_node("reduce_max", "⌈/3 1 4 1 5\n");
    assert_eq!(out, "5");
}

#[test]
fn scan_then_reduce_with_a_non_commutative_op_proves_prefix_order_in_node() {
    if !node_available() {
        eprintln!(
            "skipping scan_then_reduce_with_a_non_commutative_op_proves_prefix_order_in_node: `node` not available"
        );
        return;
    }
    // `+\1 2 3` (scan) produces the running-total vector [1, 3, 6]. This
    // backend has no whole-vector print convention exercised here (see
    // this file's module doc comment) other than APL's own `display`
    // formatting, which DOES handle a printed vector -- but composing the
    // scan's result through `-/` (reduce with Sub, non-commutative) gives
    // a single number that only comes out right if the THREE prefix sums
    // were computed in the correct left-to-right order: `1 - 3 - 6 = -8`.
    // A scan that silently reversed order, or that folded from the wrong
    // seed, would not produce this exact value.
    let out = run_via_node("scan_then_reduce", "-/+\\1 2 3\n");
    assert_eq!(out, "¯8");
}

#[test]
fn outer_product_of_two_iota_vectors_runs_in_node() {
    if !node_available() {
        eprintln!("skipping outer_product_of_two_iota_vectors_runs_in_node: `node` not available");
        return;
    }
    // `(⍳2)∘.×(⍳3)` -- outer product between two GENUINE rank-1 vectors
    // (`⍳n` constructs a true `[n]`-shaped NDArray directly, unlike a bare
    // stranded literal -- see this file's module doc comment, point 2, for
    // why two raw stranded literals are not usable as `outer`'s operands
    // today). `⍳2 = [1, 2]`, `⍳3 = [1, 2, 3]`; ravelling the outer product
    // and reduce-summing it (`+/,`) gives `(1+2) * (1+2+3) = 3 * 6 = 18` --
    // every one of the 6 pairwise products counted exactly once, which is
    // what proves `outer`'s non-square (2x3, not 2x2) case is wired
    // correctly end to end.
    let out = run_via_node("outer_product", "+/,(⍳2)∘.×(⍳3)\n");
    assert_eq!(out, "18");
}

#[test]
fn shape_of_a_reshaped_matrix_runs_in_node() {
    if !node_available() {
        eprintln!("skipping shape_of_a_reshaped_matrix_runs_in_node: `node` not available");
        return;
    }
    // `⍴(,2 3)⍴⍳6` -- dyadic `⍴` (reshape) builds a real 2x3 matrix from
    // `⍳6 = [1..6]`, then monadic `⍴` (shape) reads its dimensions back as
    // a vector, printed "2 3". The shape ARGUMENT is `,2 3` (ravel of the
    // stranded literal), not the bare literal `2 3` -- a raw stranded
    // literal lowers to a `[1, 2]`-shaped `ArrayLit` (see this file's
    // module doc comment, point 2), which `reshape` correctly rejects as
    // rank > 1 (mirroring `apl_runtime::builtins::reshape`'s own identical
    // "shape argument must be rank <= 1" check); `,2 3` ravels it down to
    // a genuine rank-1 `[2]` vector first, which reshape then accepts.
    let out = run_via_node("shape_of_reshape", "⍴(,2 3)⍴⍳6\n");
    assert_eq!(out, "2 3");
}

#[test]
fn index_generator_runs_in_node() {
    if !node_available() {
        eprintln!("skipping index_generator_runs_in_node: `node` not available");
        return;
    }
    // `⍳5` -- monadic index generator, 1-based per APL's own surface
    // semantics: `[1, 2, 3, 4, 5]`, printed space-separated.
    let out = run_via_node("index_generator", "⍳5\n");
    assert_eq!(out, "1 2 3 4 5");
}

#[test]
fn ravel_of_a_reshaped_matrix_runs_in_node() {
    if !node_available() {
        eprintln!("skipping ravel_of_a_reshaped_matrix_runs_in_node: `node` not available");
        return;
    }
    // `,(,2 3)⍴⍳6` -- monadic ravel flattens the same reshaped 2x3 matrix
    // (see the previous test for why the shape argument is `,2 3`, not the
    // bare literal `2 3`) back to a rank-1 vector in ROW-major order:
    // [1, 2, 3, 4, 5, 6]. Column-major storage means a bug that ravelled
    // straight from the backing buffer instead of walking row-then-column
    // would print "1 4 2 5 3 6" instead -- this is exactly the subtlety
    // `flattenRowMajor`'s own doc comment in `runtime.rs` calls out.
    let out = run_via_node("ravel_of_reshape", ",(,2 3)⍴⍳6\n");
    assert_eq!(out, "1 2 3 4 5 6");
}
