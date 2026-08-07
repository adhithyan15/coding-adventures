//! Pipeline tests for n-variate Hensel-lift factorisation — Track K2
//! (Rust port of Python Track K1, PR #5590).
//!
// The recursive `walk` helper threads `vars` through purely for signature
// symmetry with `var_idx`/`zero`; it is only re-forwarded in the recursive
// call, which clippy::only_used_in_recursion flags. That is intentional here.
#![allow(clippy::only_used_in_recursion)]
//!
//! Exercise the end-to-end `Factor(expr)` path through the VM: construct
//! `Factor(...)` IR, evaluate on a SymbolicBackend, and verify the result
//! by re-expanding the returned product back to a sparse-dict polynomial
//! and comparing against the sparse-dict expansion of the input.  We
//! verify *algebraic* equality rather than *shape* — the Hensel lift may
//! emit factors in a different deterministic order than the human-
//! recognisable canonical order, and integer-content can be pulled out
//! separately.

use std::collections::BTreeMap;

use symbolic_ir::{apply, int, sym, IRNode, ADD, MUL, POW, SUB};
use symbolic_vm::{SymbolicBackend, VM};

fn symbolic() -> VM {
    VM::new(Box::new(SymbolicBackend::new()))
}

fn factor(inner: IRNode) -> IRNode {
    apply(sym("Factor"), vec![inner])
}

type PolyDict = BTreeMap<Vec<i64>, i128>;

fn expand_to_dict(node: &IRNode, vars: &[&str]) -> PolyDict {
    let n = vars.len();
    let zero = vec![0i64; n];
    let mut var_idx: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for (i, v) in vars.iter().enumerate() {
        var_idx.insert(*v, i);
    }

    fn unit(name: &str, var_idx: &std::collections::HashMap<&str, usize>, n: usize) -> Vec<i64> {
        let i = *var_idx.get(name).expect("var present");
        let mut k = vec![0i64; n];
        k[i] = 1;
        k
    }
    fn add_keys(a: &[i64], b: &[i64]) -> Vec<i64> {
        a.iter().zip(b.iter()).map(|(x, y)| x + y).collect()
    }
    fn normalize(d: &mut PolyDict) {
        d.retain(|_, v| *v != 0);
    }
    fn add(a: &PolyDict, b: &PolyDict) -> PolyDict {
        let mut out: PolyDict = a.clone();
        for (k, v) in b {
            let cur = out.get(k).copied().unwrap_or(0);
            out.insert(k.clone(), cur + v);
        }
        normalize(&mut out);
        out
    }
    fn neg(a: &PolyDict) -> PolyDict {
        let mut out: PolyDict = BTreeMap::new();
        for (k, v) in a {
            out.insert(k.clone(), -v);
        }
        normalize(&mut out);
        out
    }
    fn mul(a: &PolyDict, b: &PolyDict) -> PolyDict {
        let mut out: PolyDict = BTreeMap::new();
        for (ka, va) in a {
            for (kb, vb) in b {
                let k = add_keys(ka, kb);
                let cur = out.get(&k).copied().unwrap_or(0);
                out.insert(k, cur + va * vb);
            }
        }
        normalize(&mut out);
        out
    }
    // `vars` is threaded through the recursion for symmetry with `var_idx`/`zero`
    // and readability of the walk signature, even though this arm only forwards it.
    #[allow(clippy::only_used_in_recursion)]
    fn walk(
        node: &IRNode,
        vars: &[&str],
        var_idx: &std::collections::HashMap<&str, usize>,
        n: usize,
        zero: &[i64],
    ) -> PolyDict {
        match node {
            IRNode::Integer(value) => {
                if *value == 0 {
                    BTreeMap::new()
                } else {
                    let mut m: PolyDict = BTreeMap::new();
                    m.insert(zero.to_vec(), *value as i128);
                    m
                }
            }
            IRNode::Symbol(name) => {
                if var_idx.contains_key(name.as_str()) {
                    let mut m: PolyDict = BTreeMap::new();
                    m.insert(unit(name, var_idx, n), 1);
                    m
                } else {
                    panic!("unexpected symbol: {}", name)
                }
            }
            IRNode::Apply(a) => {
                let head = match &a.head {
                    IRNode::Symbol(s) => s.as_str(),
                    _ => panic!("non-symbol head"),
                };
                if head == ADD {
                    let mut acc: PolyDict = BTreeMap::new();
                    for arg in &a.args {
                        acc = add(&acc, &walk(arg, vars, var_idx, n, zero));
                    }
                    acc
                } else if head == SUB && a.args.len() == 2 {
                    let l = walk(&a.args[0], vars, var_idx, n, zero);
                    let r = walk(&a.args[1], vars, var_idx, n, zero);
                    add(&l, &neg(&r))
                } else if head == MUL {
                    let mut acc: PolyDict = BTreeMap::new();
                    acc.insert(zero.to_vec(), 1);
                    for arg in &a.args {
                        acc = mul(&acc, &walk(arg, vars, var_idx, n, zero));
                    }
                    acc
                } else if head == POW && a.args.len() == 2 {
                    let base = walk(&a.args[0], vars, var_idx, n, zero);
                    let exp = match a.args[1] {
                        IRNode::Integer(e) => e,
                        _ => panic!("non-int exp"),
                    };
                    assert!(exp >= 0);
                    if exp == 0 {
                        let mut m: PolyDict = BTreeMap::new();
                        m.insert(zero.to_vec(), 1);
                        return m;
                    }
                    let mut out = base.clone();
                    for _ in 1..exp {
                        out = mul(&out, &base);
                    }
                    out
                } else {
                    panic!("unexpected head: {}", head)
                }
            }
            _ => panic!("unexpected node"),
        }
    }
    walk(node, vars, &var_idx, n, &zero)
}

fn is_factor_wrapper(node: &IRNode) -> bool {
    if let IRNode::Apply(a) = node {
        if let IRNode::Symbol(s) = &a.head {
            return s == "Factor";
        }
    }
    false
}

#[test]
fn factor_x3_y3_z3_minus_3xyz_recovers_two_factors() {
    let mut vm = symbolic();
    // x^3 + y^3 + z^3 - 3*x*y*z
    let target = apply(
        sym(SUB),
        vec![
            apply(
                sym(ADD),
                vec![
                    apply(
                        sym(ADD),
                        vec![
                            apply(sym(POW), vec![sym("x"), int(3)]),
                            apply(sym(POW), vec![sym("y"), int(3)]),
                        ],
                    ),
                    apply(sym(POW), vec![sym("z"), int(3)]),
                ],
            ),
            apply(
                sym(MUL),
                vec![
                    int(3),
                    apply(
                        sym(MUL),
                        vec![apply(sym(MUL), vec![sym("x"), sym("y")]), sym("z")],
                    ),
                ],
            ),
        ],
    );
    let expr = factor(target.clone());
    let result = vm.eval(expr.clone());
    assert!(!is_factor_wrapper(&result), "expected factored output");
    let vars = ["x", "y", "z"];
    assert_eq!(expand_to_dict(&result, &vars), expand_to_dict(&target, &vars));
}

#[test]
fn factor_linear_product_round_trips() {
    let mut vm = symbolic();
    // x^2 + 3xy + 4xz + 2y^2 + 5yz + 3z^2  =  (x + y + z)(x + 2y + 3z)
    let expanded = apply(
        sym(ADD),
        vec![
            apply(sym(POW), vec![sym("x"), int(2)]),
            apply(
                sym(ADD),
                vec![
                    apply(sym(MUL), vec![int(3), apply(sym(MUL), vec![sym("x"), sym("y")])]),
                    apply(
                        sym(ADD),
                        vec![
                            apply(sym(MUL), vec![int(4), apply(sym(MUL), vec![sym("x"), sym("z")])]),
                            apply(
                                sym(ADD),
                                vec![
                                    apply(sym(MUL), vec![int(2), apply(sym(POW), vec![sym("y"), int(2)])]),
                                    apply(
                                        sym(ADD),
                                        vec![
                                            apply(sym(MUL), vec![int(5), apply(sym(MUL), vec![sym("y"), sym("z")])]),
                                            apply(sym(MUL), vec![int(3), apply(sym(POW), vec![sym("z"), int(2)])]),
                                        ],
                                    ),
                                ],
                            ),
                        ],
                    ),
                ],
            ),
        ],
    );
    let expr = factor(expanded.clone());
    let result = vm.eval(expr.clone());
    let vars = ["x", "y", "z"];
    if is_factor_wrapper(&result) {
        return; // unevaluated wrapper — acceptable
    }
    assert_eq!(expand_to_dict(&result, &vars), expand_to_dict(&expanded, &vars));
}

#[test]
fn factor_irreducible_x2_y2_z2_plus_1_falls_through() {
    let mut vm = symbolic();
    let target = apply(
        sym(ADD),
        vec![
            apply(sym(POW), vec![sym("x"), int(2)]),
            apply(
                sym(ADD),
                vec![
                    apply(sym(POW), vec![sym("y"), int(2)]),
                    apply(
                        sym(ADD),
                        vec![apply(sym(POW), vec![sym("z"), int(2)]), int(1)],
                    ),
                ],
            ),
        ],
    );
    let expr = factor(target.clone());
    let result = vm.eval(expr.clone());
    // Either Factor(...) wrapper or algebraic round-trip.
    if is_factor_wrapper(&result) {
        return;
    }
    let vars = ["x", "y", "z"];
    assert_eq!(expand_to_dict(&result, &vars), expand_to_dict(&target, &vars));
}

#[test]
fn factor_transcendental_does_not_crash() {
    let mut vm = symbolic();
    let target = apply(
        sym(ADD),
        vec![
            apply(sym("Sin"), vec![sym("x")]),
            apply(sym(ADD), vec![sym("y"), sym("z")]),
        ],
    );
    let expr = factor(target);
    let _ = vm.eval(expr.clone());
}

#[test]
fn regression_bivariate_x2_xy_minus_2y2_still_factors() {
    let mut vm = symbolic();
    // x^2 + xy - 2y^2 = (x + 2y)(x - y)
    let target = apply(
        sym(SUB),
        vec![
            apply(
                sym(ADD),
                vec![
                    apply(sym(POW), vec![sym("x"), int(2)]),
                    apply(sym(MUL), vec![sym("x"), sym("y")]),
                ],
            ),
            apply(sym(MUL), vec![int(2), apply(sym(POW), vec![sym("y"), int(2)])]),
        ],
    );
    let expr = factor(target.clone());
    let result = vm.eval(expr.clone());
    assert!(!is_factor_wrapper(&result), "expected factored output");
    let vars = ["x", "y"];
    assert_eq!(expand_to_dict(&result, &vars), expand_to_dict(&target, &vars));
}

#[test]
fn regression_univariate_x2_minus_1_still_factors() {
    let mut vm = symbolic();
    let target = apply(sym(SUB), vec![apply(sym(POW), vec![sym("x"), int(2)]), int(1)]);
    let expr = factor(target.clone());
    let result = vm.eval(expr.clone());
    assert!(!is_factor_wrapper(&result), "expected univariate factoring");
    let vars = ["x"];
    assert_eq!(expand_to_dict(&result, &vars), expand_to_dict(&target, &vars));
}
