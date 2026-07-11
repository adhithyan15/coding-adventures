//! Tests for the `load("name")` runtime directive (Track M2).
//!
//! Acceptance contract mirrored from `macsyma-truly-finish-plan.md` §M1,
//! which Python implemented in commit `dc78e0931`.  Each test here maps
//! 1:1 to a Python test in `test_load_package.py` so a future audit can
//! walk the two files side-by-side.
//!
//!   - Without `load`, orthopoly heads round-trip unevaluated.
//!   - After `load("orthopoly")`, the closed-form polynomial fires.
//!   - Unknown names raise `MacsymaUserError` with a helpful message.
//!   - Re-loading is idempotent.
//!   - Loaded state is per-session (two backends stay independent).
//!   - Regression: non-orthopoly ops still work without a load.
//!
//! Errors panic with a `MacsymaUserError`-formatted message because the
//! existing Rust handler signature returns `IRNode`, not `Result`.  The
//! tests use `std::panic::catch_unwind` to assert the failure path.

use std::panic;

use coding_adventures_macsyma_runtime::{
    macsyma_name_table, MacsymaSession, MacsymaUserError, LOAD,
};
use symbolic_ir::{apply, int, str_node, sym, IRNode, ADD, MUL};

// ---------------------------------------------------------------------
// Fixtures and helpers
// ---------------------------------------------------------------------

fn fresh_session() -> MacsymaSession {
    MacsymaSession::new()
}

fn legendre_p(n: i64, x_name: &str) -> IRNode {
    apply(sym("LegendreP"), vec![int(n), sym(x_name)])
}

/// Run `load("name")` through the VM and return the result.
fn load_pkg(session: &mut MacsymaSession, name: &str) -> IRNode {
    let mut session_ref = session;
    eval_with_session(session_ref, apply(sym(LOAD), vec![str_node(name)]))
}

/// Helper that compiles `source` and returns the single resulting IR
/// node — convenience for tests that exercise the user-facing surface
/// rather than constructing IR by hand.
fn eval_with_session(session: &mut MacsymaSession, expr: IRNode) -> IRNode {
    let results = session.eval_statements(vec![expr]);
    results
        .into_iter()
        .next()
        .expect("expected one evaluation result")
        .output
}

/// Catch a `MacsymaUserError`-shaped panic and return its message.
fn catch_macsyma_user_error<F: FnOnce() + panic::UnwindSafe>(f: F) -> String {
    let prior_hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {})); // silence the default stderr noise
    let outcome = panic::catch_unwind(f);
    panic::set_hook(prior_hook);
    let payload = outcome.expect_err("expected the test body to panic");
    if let Some(msg) = payload.downcast_ref::<String>() {
        msg.clone()
    } else if let Some(msg) = payload.downcast_ref::<&'static str>() {
        msg.to_string()
    } else if let Some(err) = payload.downcast_ref::<MacsymaUserError>() {
        err.to_string()
    } else {
        panic!("unexpected panic payload type");
    }
}

// ---------------------------------------------------------------------
// 1. Without load, orthopoly heads stay unevaluated.
// ---------------------------------------------------------------------

#[test]
fn unloaded_legendre_p_returns_unevaluated() {
    let mut session = fresh_session();
    let result = eval_with_session(&mut session, legendre_p(3, "x"));
    match result {
        IRNode::Apply(node) => {
            assert_eq!(node.head, sym("LegendreP"));
            assert_eq!(node.args, vec![int(3), sym("x")]);
        }
        other => panic!("expected unevaluated Apply, got {other:?}"),
    }
}

#[test]
fn unloaded_chebyshev_t_returns_unevaluated() {
    let mut session = fresh_session();
    let result = eval_with_session(
        &mut session,
        apply(sym("ChebyshevT"), vec![int(4), sym("x")]),
    );
    match result {
        IRNode::Apply(node) => assert_eq!(node.head, sym("ChebyshevT")),
        other => panic!("expected unevaluated Apply, got {other:?}"),
    }
}

#[test]
fn unloaded_hermite_h_returns_unevaluated() {
    let mut session = fresh_session();
    let result = eval_with_session(
        &mut session,
        apply(sym("HermiteH"), vec![int(2), sym("x")]),
    );
    match result {
        IRNode::Apply(node) => assert_eq!(node.head, sym("HermiteH")),
        other => panic!("expected unevaluated Apply, got {other:?}"),
    }
}

// ---------------------------------------------------------------------
// 2. After load("orthopoly"), closed-form reductions kick in.
// ---------------------------------------------------------------------

#[test]
fn loaded_legendre_p_3_at_2_is_17() {
    // P_3(x) = (5x^3 − 3x)/2 → at x = 2: 17.  Pass the value directly
    // rather than through `Subst` (the existing Rust Subst handler
    // does structural substitution without re-eval, a pre-existing
    // implementation detail orthogonal to this test).
    let mut session = fresh_session();
    let _ = load_pkg(&mut session, "orthopoly");
    let result = eval_with_session(
        &mut session,
        apply(sym("LegendreP"), vec![int(3), int(2)]),
    );
    assert_eq!(result, int(17));
}

#[test]
fn loaded_legendre_p_0_and_1_are_seed_values() {
    let mut session = fresh_session();
    let _ = load_pkg(&mut session, "orthopoly");
    let p0 = eval_with_session(&mut session, legendre_p(0, "x"));
    assert_eq!(p0, int(1));
    let p1 = eval_with_session(&mut session, legendre_p(1, "x"));
    assert_eq!(p1, sym("x"));
}

#[test]
fn loaded_chebyshev_t_4_at_1_is_1() {
    let mut session = fresh_session();
    let _ = load_pkg(&mut session, "orthopoly");
    let result = eval_with_session(
        &mut session,
        apply(sym("ChebyshevT"), vec![int(4), int(1)]),
    );
    assert_eq!(result, int(1));
}

#[test]
fn loaded_chebyshev_u_3_at_1_is_4() {
    let mut session = fresh_session();
    let _ = load_pkg(&mut session, "orthopoly");
    let result = eval_with_session(
        &mut session,
        apply(sym("ChebyshevU"), vec![int(3), int(1)]),
    );
    assert_eq!(result, int(4));
}

#[test]
fn loaded_hermite_h_3_at_1_is_negative_four() {
    let mut session = fresh_session();
    let _ = load_pkg(&mut session, "orthopoly");
    let result = eval_with_session(
        &mut session,
        apply(sym("HermiteH"), vec![int(3), int(1)]),
    );
    assert_eq!(result, int(-4));
}

// ---------------------------------------------------------------------
// 3. Passthrough heads — symbols known after load, no reduction.
// ---------------------------------------------------------------------

#[test]
fn loaded_bessel_j_returns_unevaluated_with_known_head() {
    let mut session = fresh_session();
    let _ = load_pkg(&mut session, "orthopoly");
    let result = eval_with_session(
        &mut session,
        apply(sym("BesselJ"), vec![int(0), sym("x")]),
    );
    match result {
        IRNode::Apply(node) => assert_eq!(node.head, sym("BesselJ")),
        other => panic!("expected unevaluated Apply, got {other:?}"),
    }
}

#[test]
fn loaded_legendre_q_returns_unevaluated() {
    let mut session = fresh_session();
    let _ = load_pkg(&mut session, "orthopoly");
    let result = eval_with_session(
        &mut session,
        apply(sym("LegendreQ"), vec![int(2), sym("x")]),
    );
    match result {
        IRNode::Apply(node) => assert_eq!(node.head, sym("LegendreQ")),
        other => panic!("expected unevaluated Apply, got {other:?}"),
    }
}

// ---------------------------------------------------------------------
// 4. Allowlist enforcement.
// ---------------------------------------------------------------------

#[test]
fn load_unknown_package_raises_user_error() {
    let message = catch_macsyma_user_error(|| {
        let mut session = fresh_session();
        let _ = load_pkg(&mut session, "nonexistent");
    });
    assert!(message.contains("unknown package"));
    assert!(message.contains("'nonexistent'"));
    assert!(message.contains("orthopoly"));
}

#[test]
fn load_with_non_string_non_symbol_raises() {
    let message = catch_macsyma_user_error(|| {
        let mut session = fresh_session();
        let _ = eval_with_session(&mut session, apply(sym(LOAD), vec![int(42)]));
    });
    assert!(message.contains("string or symbol"));
}

#[test]
fn load_with_wrong_arity_raises() {
    let message = catch_macsyma_user_error(|| {
        let mut session = fresh_session();
        let _ = eval_with_session(&mut session, apply(sym(LOAD), vec![]));
    });
    assert!(message.contains("load takes 1 argument"));
}

#[test]
fn path_traversal_strings_are_rejected_as_unknown_names() {
    // The allowlist match is by string equality — no path resolution.
    // This test nails down the absence of an `Path::join`,
    // `libloading`, or `eval`-equivalent code path.
    for hostile in ["../etc/passwd", "/tmp/orthopoly", "orthopoly.rs", "os"] {
        let message = catch_macsyma_user_error(|| {
            let mut session = fresh_session();
            let _ = load_pkg(&mut session, hostile);
        });
        assert!(
            message.contains("unknown package"),
            "expected hostile name {hostile:?} to be rejected; got message {message:?}",
        );
    }
}

// ---------------------------------------------------------------------
// 5. Idempotence — re-loading is a no-op.
// ---------------------------------------------------------------------

#[test]
fn load_orthopoly_is_idempotent() {
    let mut session = fresh_session();
    let _ = load_pkg(&mut session, "orthopoly");
    let second = load_pkg(&mut session, "orthopoly");
    assert_eq!(second, str_node("orthopoly"));
    assert!(session.loaded_packages().contains("orthopoly"));
    // P_2(x) = (3x^2 − 1)/2 → at x = 3: 13.
    let p2_at_3 = eval_with_session(
        &mut session,
        apply(sym("LegendreP"), vec![int(2), int(3)]),
    );
    assert_eq!(p2_at_3, int(13));
}

// ---------------------------------------------------------------------
// 6. Per-session state — two backends are independent.
// ---------------------------------------------------------------------

#[test]
fn two_backends_have_independent_loaded_state() {
    let mut session_a = fresh_session();
    let mut session_b = fresh_session();

    let _ = load_pkg(&mut session_a, "orthopoly");

    assert!(session_a.loaded_packages().contains("orthopoly"));
    assert!(!session_b.loaded_packages().contains("orthopoly"));

    let a_result = eval_with_session(&mut session_a, legendre_p(0, "x"));
    assert_eq!(a_result, int(1));

    let b_result = eval_with_session(&mut session_b, legendre_p(0, "x"));
    match b_result {
        IRNode::Apply(node) => assert_eq!(node.head, sym("LegendreP")),
        other => panic!("expected unevaluated Apply, got {other:?}"),
    }
}

// ---------------------------------------------------------------------
// 7. Regression — non-orthopoly ops still work without a load.
// ---------------------------------------------------------------------

#[test]
fn basic_arithmetic_still_folds_without_load() {
    // The regression we care about: installing the M2 orthopoly
    // gates and the LOAD handler must not perturb the substrate
    // simplifier.  Run a 1+2 through the VM and confirm we get the
    // canonical integer back.
    let mut session = fresh_session();
    let result = eval_with_session(&mut session, apply(sym(ADD), vec![int(1), int(2)]));
    assert_eq!(result, int(3));
}

#[test]
fn mul_still_folds_without_load() {
    let mut session = fresh_session();
    let result = eval_with_session(&mut session, apply(sym(MUL), vec![int(2), int(7)]));
    assert_eq!(result, int(14));
}

// ---------------------------------------------------------------------
// 8. Surface-name routing — `load` is wired through the name table.
// ---------------------------------------------------------------------

#[test]
fn name_table_maps_load_and_orthopoly_surface_names() {
    let table = macsyma_name_table();
    assert_eq!(table.get("load"), Some(&"Load".to_string()));
    assert_eq!(table.get("legendre_p"), Some(&"LegendreP".to_string()));
    assert_eq!(table.get("legendre_q"), Some(&"LegendreQ".to_string()));
    assert_eq!(table.get("chebyshev_t"), Some(&"ChebyshevT".to_string()));
    assert_eq!(table.get("chebyshev_u"), Some(&"ChebyshevU".to_string()));
    assert_eq!(table.get("hermite"), Some(&"HermiteH".to_string()));
    assert_eq!(table.get("bessel_j"), Some(&"BesselJ".to_string()));
    assert_eq!(table.get("bessel_y"), Some(&"BesselY".to_string()));
}

// ---------------------------------------------------------------------
// 9. Symbol-form loading — `load(orthopoly)` (bare symbol) also works.
// ---------------------------------------------------------------------

#[test]
fn load_accepts_bare_symbol_argument() {
    let mut session = fresh_session();
    let result = eval_with_session(
        &mut session,
        apply(sym(LOAD), vec![sym("orthopoly")]),
    );
    assert_eq!(result, str_node("orthopoly"));
    assert!(session.loaded_packages().contains("orthopoly"));
}

// ---------------------------------------------------------------------
// 10. Non-integer first argument keeps the polynomial heads unevaluated.
// ---------------------------------------------------------------------

#[test]
fn loaded_legendre_p_symbolic_n_is_unevaluated() {
    let mut session = fresh_session();
    let _ = load_pkg(&mut session, "orthopoly");
    let result = eval_with_session(
        &mut session,
        apply(sym("LegendreP"), vec![sym("n"), sym("x")]),
    );
    match result {
        IRNode::Apply(node) => assert_eq!(node.head, sym("LegendreP")),
        other => panic!("expected unevaluated Apply, got {other:?}"),
    }
}

#[test]
fn loaded_legendre_p_negative_n_is_unevaluated() {
    let mut session = fresh_session();
    let _ = load_pkg(&mut session, "orthopoly");
    let result = eval_with_session(
        &mut session,
        apply(sym("LegendreP"), vec![int(-1), sym("x")]),
    );
    match result {
        IRNode::Apply(node) => assert_eq!(node.head, sym("LegendreP")),
        other => panic!("expected unevaluated Apply, got {other:?}"),
    }
}
