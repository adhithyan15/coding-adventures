//! Track B1 — Apart simple-roots partial-fraction decomposition.
//!
//! Mirrors the six TypeScript test cases (and the Python reference)
//! exercised by ``code/specs/macsyma-finish-plan.md`` Track B1.
//! Apart is registered under the symbol name ``"Apart"`` in the
//! symbolic backend's handler table.

use symbolic_ir::{apply, int, rat, sym, ADD, DIV, MUL, NEG, POW, SUB};
use symbolic_vm::{SymbolicBackend, VM};

fn vm() -> VM {
    VM::new(Box::new(SymbolicBackend::new()))
}

fn apart(inner: symbolic_ir::IRNode) -> symbolic_ir::IRNode {
    apply(sym("Apart"), vec![inner, sym("x")])
}

#[test]
fn apart_one_over_x_squared_minus_one_acceptance() {
    let mut v = vm();
    // 1 / (x^2 - 1)
    let inner = apply(
        sym(DIV),
        vec![
            int(1),
            apply(sym(SUB), vec![apply(sym(POW), vec![sym("x"), int(2)]), int(1)]),
        ],
    );
    let result = v.eval(apart(inner));
    // Roots sort ascending: -1 first, then 1.  Matches Python's output:
    // Add(Div(-1/2, (1+x)), Div(1/2, (-1+x))).
    let expected = apply(
        sym(ADD),
        vec![
            apply(sym(DIV), vec![rat(-1, 2), apply(sym(ADD), vec![int(1), sym("x")])]),
            apply(sym(DIV), vec![rat(1, 2), apply(sym(ADD), vec![int(-1), sym("x")])]),
        ],
    );
    assert_eq!(result, expected);
}

#[test]
fn apart_three_distinct_simple_roots() {
    let mut v = vm();
    let f1 = apply(sym(SUB), vec![sym("x"), int(1)]);
    let f2 = apply(sym(SUB), vec![sym("x"), int(2)]);
    let f3 = apply(sym(SUB), vec![sym("x"), int(3)]);
    let inner = apply(
        sym(DIV),
        vec![int(1), apply(sym(MUL), vec![apply(sym(MUL), vec![f1, f2]), f3])],
    );
    let result = v.eval(apart(inner));
    // Residues: A_1 = 1/2, A_2 = -1, A_3 = 1/2.
    // ``A = -1`` renders as Neg(Div(1, ...)).
    let expected = apply(
        sym(ADD),
        vec![
            apply(
                sym(ADD),
                vec![
                    apply(sym(DIV), vec![rat(1, 2), apply(sym(ADD), vec![int(-1), sym("x")])]),
                    apply(
                        sym(NEG),
                        vec![apply(sym(DIV), vec![int(1), apply(sym(ADD), vec![int(-2), sym("x")])])],
                    ),
                ],
            ),
            apply(sym(DIV), vec![rat(1, 2), apply(sym(ADD), vec![int(-3), sym("x")])]),
        ],
    );
    assert_eq!(result, expected);
}

#[test]
fn apart_improper_fraction_x_cubed_over_x_squared_minus_one() {
    let mut v = vm();
    let inner = apply(
        sym(DIV),
        vec![
            apply(sym(POW), vec![sym("x"), int(3)]),
            apply(sym(SUB), vec![apply(sym(POW), vec![sym("x"), int(2)]), int(1)]),
        ],
    );
    let result = v.eval(apart(inner));
    // x^3/(x^2-1) = x + 1/(2(x-1)) + 1/(2(x+1)).
    // ``from_polynomial([0, 1], x)`` collapses to bare x.
    let expected = apply(
        sym(ADD),
        vec![
            sym("x"),
            apply(
                sym(ADD),
                vec![
                    apply(sym(DIV), vec![rat(1, 2), apply(sym(ADD), vec![int(1), sym("x")])]),
                    apply(sym(DIV), vec![rat(1, 2), apply(sym(ADD), vec![int(-1), sym("x")])]),
                ],
            ),
        ],
    );
    assert_eq!(result, expected);
}

#[test]
fn apart_irreducible_quadratic_stays_unevaluated() {
    let mut v = vm();
    let inner = apply(
        sym(DIV),
        vec![int(1), apply(sym(ADD), vec![apply(sym(POW), vec![sym("x"), int(2)]), int(1)])],
    );
    let result = v.eval(apart(inner.clone()));
    // No rational roots — Apart stays wrapped.
    assert_eq!(result, apply(sym("Apart"), vec![inner, sym("x")]));
}

#[test]
fn apart_repeated_root_stays_unevaluated() {
    let mut v = vm();
    // 1 / (x - 1)^2
    let inner = apply(
        sym(DIV),
        vec![int(1), apply(sym(POW), vec![apply(sym(SUB), vec![sym("x"), int(1)]), int(2)])],
    );
    let result = v.eval(apart(inner.clone()));
    // Phase 48 explicitly out of scope; handler bails to unevaluated form.
    assert_eq!(result, apply(sym("Apart"), vec![inner, sym("x")]));
}

#[test]
fn apart_already_polynomial_passes_through() {
    let mut v = vm();
    // Apart(x + 1, x) → x + 1
    let inner = apply(sym(ADD), vec![sym("x"), int(1)]);
    let result = v.eval(apart(inner));
    // ``to_rational(x+1, x)`` → num = [1, 1], den = [1]; the early-return
    // path emits ``from_polynomial(num, x)`` = Add(1, x).
    let expected = apply(sym(ADD), vec![int(1), sym("x")]);
    assert_eq!(result, expected);
}
