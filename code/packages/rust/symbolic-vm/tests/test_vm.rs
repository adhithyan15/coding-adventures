//! Integration tests for the symbolic VM.
//!
//! These tests exercise the full eval loop — constructing IR expressions
//! directly and evaluating them under each backend.

use symbolic_ir::{apply, flt, int, rat, sym, ADD, AND, ASSIGN, COS, DEFINE, DIV, EXP, IF, LIST,
    LOG, MUL, NEG, NOT, OR, POW, SIN, SQRT, SUB};
use symbolic_vm::{StrictBackend, SymbolicBackend, VM};

// ---------------------------------------------------------------------------
// Helper: build a VM
// ---------------------------------------------------------------------------

fn strict() -> VM {
    VM::new(Box::new(StrictBackend::new()))
}

fn symbolic() -> VM {
    VM::new(Box::new(SymbolicBackend::new()))
}

// ---------------------------------------------------------------------------
// Numeric literals pass through unchanged
// ---------------------------------------------------------------------------

#[test]
fn integer_literal_unchanged() {
    assert_eq!(symbolic().eval(int(42)), int(42));
}

#[test]
fn float_literal_unchanged() {
    assert_eq!(symbolic().eval(flt(3.14)), flt(3.14));
}

#[test]
fn rational_literal_unchanged() {
    assert_eq!(symbolic().eval(rat(1, 2)), rat(1, 2));
}

// ---------------------------------------------------------------------------
// Arithmetic — strict backend (fully numeric)
// ---------------------------------------------------------------------------

#[test]
fn strict_add_integers() {
    assert_eq!(strict().eval(apply(sym(ADD), vec![int(2), int(3)])), int(5));
}

#[test]
fn strict_sub_integers() {
    assert_eq!(strict().eval(apply(sym(SUB), vec![int(10), int(3)])), int(7));
}

#[test]
fn strict_mul_integers() {
    assert_eq!(strict().eval(apply(sym(MUL), vec![int(4), int(5)])), int(20));
}

#[test]
fn strict_div_integers_exact() {
    // 6 / 3 = 2 (exact integer)
    assert_eq!(strict().eval(apply(sym(DIV), vec![int(6), int(3)])), int(2));
}

#[test]
fn strict_div_integers_rational() {
    // 1 / 3 = 1/3 (exact rational)
    assert_eq!(strict().eval(apply(sym(DIV), vec![int(1), int(3)])), rat(1, 3));
}

#[test]
fn strict_pow_integers() {
    assert_eq!(strict().eval(apply(sym(POW), vec![int(2), int(10)])), int(1024));
}

#[test]
fn strict_neg() {
    assert_eq!(strict().eval(apply(sym(NEG), vec![int(5)])), int(-5));
}

#[test]
fn strict_neg_rational() {
    assert_eq!(strict().eval(apply(sym(NEG), vec![rat(1, 3)])), rat(-1, 3));
}

#[test]
fn strict_nested_arithmetic() {
    // (2 + 3) * 4 = 20
    let inner = apply(sym(ADD), vec![int(2), int(3)]);
    let outer = apply(sym(MUL), vec![inner, int(4)]);
    assert_eq!(strict().eval(outer), int(20));
}

// ---------------------------------------------------------------------------
// Arithmetic — symbolic backend (identity folding)
// ---------------------------------------------------------------------------

#[test]
fn symbolic_add_fold() {
    // Add(2, 3) → 5
    assert_eq!(symbolic().eval(apply(sym(ADD), vec![int(2), int(3)])), int(5));
}

#[test]
fn symbolic_add_identity_right() {
    // Add(x, 0) → x
    let expr = apply(sym(ADD), vec![sym("x"), int(0)]);
    assert_eq!(symbolic().eval(expr), sym("x"));
}

#[test]
fn symbolic_add_identity_left() {
    // Add(0, x) → x
    let expr = apply(sym(ADD), vec![int(0), sym("x")]);
    assert_eq!(symbolic().eval(expr), sym("x"));
}

#[test]
fn symbolic_mul_absorbing_zero() {
    // Mul(0, x) → 0
    assert_eq!(
        symbolic().eval(apply(sym(MUL), vec![int(0), sym("x")])),
        int(0)
    );
}

#[test]
fn symbolic_mul_identity() {
    // Mul(1, x) → x
    assert_eq!(
        symbolic().eval(apply(sym(MUL), vec![int(1), sym("x")])),
        sym("x")
    );
}

#[test]
fn symbolic_pow_zero_exponent() {
    // Pow(x, 0) → 1
    assert_eq!(
        symbolic().eval(apply(sym(POW), vec![sym("x"), int(0)])),
        int(1)
    );
}

#[test]
fn symbolic_pow_unit_exponent() {
    // Pow(x, 1) → x
    assert_eq!(
        symbolic().eval(apply(sym(POW), vec![sym("x"), int(1)])),
        sym("x")
    );
}

#[test]
fn symbolic_neg_double_negation() {
    // Neg(Neg(x)) → x
    let inner = apply(sym(NEG), vec![sym("x")]);
    let outer = apply(sym(NEG), vec![inner]);
    assert_eq!(symbolic().eval(outer), sym("x"));
}

#[test]
fn symbolic_unknown_head_passes_through() {
    // UnknownFunc(x) → UnknownFunc(x) unchanged in symbolic mode
    let expr = apply(sym("UnknownFunc"), vec![sym("x")]);
    assert_eq!(symbolic().eval(expr.clone()), expr);
}

// ---------------------------------------------------------------------------
// Symbol resolution
// ---------------------------------------------------------------------------

#[test]
fn symbolic_unbound_symbol_returns_self() {
    assert_eq!(symbolic().eval(sym("x")), sym("x"));
}

#[test]
fn symbolic_bound_symbol_resolves() {
    let mut vm = symbolic();
    vm.backend.bind("x", int(42));
    assert_eq!(vm.eval(sym("x")), int(42));
}

#[test]
fn symbolic_transitive_binding() {
    // a := b; b := 5  →  eval(a) == 5
    let mut vm = symbolic();
    vm.backend.bind("b", int(5));
    vm.backend.bind("a", sym("b"));
    assert_eq!(vm.eval(sym("a")), int(5));
}

#[test]
fn symbolic_self_loop_guard() {
    // x := x → x (no infinite recursion)
    let mut vm = symbolic();
    vm.backend.bind("x", sym("x"));
    assert_eq!(vm.eval(sym("x")), sym("x"));
}

// ---------------------------------------------------------------------------
// Elementary functions
// ---------------------------------------------------------------------------

#[test]
fn sin_zero() {
    assert_eq!(symbolic().eval(apply(sym(SIN), vec![int(0)])), int(0));
}

#[test]
fn cos_zero() {
    assert_eq!(symbolic().eval(apply(sym(COS), vec![int(0)])), int(1));
}

#[test]
fn exp_zero() {
    assert_eq!(symbolic().eval(apply(sym(EXP), vec![int(0)])), int(1));
}

#[test]
fn log_one() {
    assert_eq!(symbolic().eval(apply(sym(LOG), vec![int(1)])), int(0));
}

#[test]
fn sqrt_zero() {
    assert_eq!(symbolic().eval(apply(sym(SQRT), vec![int(0)])), int(0));
}

#[test]
fn sqrt_one() {
    assert_eq!(symbolic().eval(apply(sym(SQRT), vec![int(1)])), int(1));
}

#[test]
fn sin_symbolic_stays() {
    // Sin(x) stays when x is unbound (symbolic mode)
    let expr = apply(sym(SIN), vec![sym("x")]);
    assert_eq!(symbolic().eval(expr.clone()), expr);
}

// ---------------------------------------------------------------------------
// Logic
// ---------------------------------------------------------------------------

#[test]
fn and_true_true() {
    let expr = apply(sym(AND), vec![sym("True"), sym("True")]);
    assert_eq!(symbolic().eval(expr), sym("True"));
}

#[test]
fn and_short_circuit_false() {
    // And(False, x) → False regardless of x
    let expr = apply(sym(AND), vec![sym("False"), sym("x")]);
    assert_eq!(symbolic().eval(expr), sym("False"));
}

#[test]
fn or_true_short_circuits() {
    let expr = apply(sym(OR), vec![sym("True"), sym("x")]);
    assert_eq!(symbolic().eval(expr), sym("True"));
}

#[test]
fn not_true() {
    assert_eq!(
        symbolic().eval(apply(sym(NOT), vec![sym("True")])),
        sym("False")
    );
}

#[test]
fn not_false() {
    assert_eq!(
        symbolic().eval(apply(sym(NOT), vec![sym("False")])),
        sym("True")
    );
}

// ---------------------------------------------------------------------------
// If — held head
// ---------------------------------------------------------------------------

#[test]
fn if_true_branch() {
    // If(True, 1, 2) → 1
    let expr = apply(sym(IF), vec![sym("True"), int(1), int(2)]);
    assert_eq!(symbolic().eval(expr), int(1));
}

#[test]
fn if_false_branch() {
    // If(False, 1, 2) → 2
    let expr = apply(sym(IF), vec![sym("False"), int(1), int(2)]);
    assert_eq!(symbolic().eval(expr), int(2));
}

#[test]
fn if_false_no_else() {
    // If(False, 1) → False
    let expr = apply(sym(IF), vec![sym("False"), int(1)]);
    assert_eq!(symbolic().eval(expr), sym("False"));
}

// ---------------------------------------------------------------------------
// Assign
// ---------------------------------------------------------------------------

#[test]
fn assign_binds_and_returns_value() {
    let mut vm = symbolic();
    let expr = apply(sym(ASSIGN), vec![sym("x"), int(42)]);
    let result = vm.eval(expr);
    assert_eq!(result, int(42));
    assert_eq!(vm.backend.lookup("x"), Some(int(42)));
}

// ---------------------------------------------------------------------------
// Define / user-defined functions
// ---------------------------------------------------------------------------

#[test]
fn define_and_call_function() {
    // f(x) := x * 2; f(5) → 10
    let mut vm = symbolic();

    // Define(f, List(x), Mul(x, 2))
    let def = apply(
        sym(DEFINE),
        vec![
            sym("f"),
            apply(sym(LIST), vec![sym("x")]),
            apply(sym(MUL), vec![sym("x"), int(2)]),
        ],
    );
    vm.eval(def);

    // Call f(5)
    let call = apply(sym("f"), vec![int(5)]);
    assert_eq!(vm.eval(call), int(10));
}

// ---------------------------------------------------------------------------
// List
// ---------------------------------------------------------------------------

#[test]
fn list_passthrough() {
    let expr = apply(sym(LIST), vec![int(1), int(2), int(3)]);
    assert_eq!(symbolic().eval(expr.clone()), expr);
}

// ---------------------------------------------------------------------------
// eval_program
// ---------------------------------------------------------------------------

#[test]
fn eval_program_returns_last_value() {
    let mut vm = symbolic();
    let result = vm.eval_program(vec![int(1), int(2), int(3)]);
    assert_eq!(result, Some(int(3)));
}

#[test]
fn eval_program_empty_returns_none() {
    let mut vm = symbolic();
    assert_eq!(vm.eval_program(vec![]), None);
}

#[test]
fn eval_program_bindings_persist() {
    let mut vm = symbolic();
    // x := 5; x + 1
    let assign = apply(sym(ASSIGN), vec![sym("x"), int(5)]);
    let expr = apply(sym(ADD), vec![sym("x"), int(1)]);
    let result = vm.eval_program(vec![assign, expr]);
    assert_eq!(result, Some(int(6)));
}

// ---------------------------------------------------------------------------
// Exact rational arithmetic
// ---------------------------------------------------------------------------

#[test]
fn rational_add_exact() {
    // 1/2 + 1/3 = 5/6
    let expr = apply(sym(ADD), vec![rat(1, 2), rat(1, 3)]);
    assert_eq!(strict().eval(expr), rat(5, 6));
}

#[test]
fn rational_mul_exact() {
    // 2/3 * 3/4 = 1/2
    let expr = apply(sym(MUL), vec![rat(2, 3), rat(3, 4)]);
    assert_eq!(strict().eval(expr), rat(1, 2));
}

#[test]
fn integer_div_exact_rational() {
    // 1 / 4 = 1/4
    assert_eq!(strict().eval(apply(sym(DIV), vec![int(1), int(4)])), rat(1, 4));
}

// ---------------------------------------------------------------------------
// Strict backend panics on unknowns
// ---------------------------------------------------------------------------

#[test]
#[should_panic(expected = "undefined symbol")]
fn strict_panics_on_unbound_symbol() {
    strict().eval(sym("unbound_x_yz"));
}

#[test]
#[should_panic(expected = "no handler for head")]
fn strict_panics_on_unknown_head() {
    strict().eval(apply(sym("UnknownFunc"), vec![int(1)]));
}

#[test]
#[should_panic(expected = "undefined symbol")]
fn strict_panics_on_symbolic_add() {
    // In strict mode, evaluating args first means `x` triggers on_unresolved
    // before Add even sees the arguments.
    strict().eval(apply(sym(ADD), vec![sym("x"), int(1)]));
}
