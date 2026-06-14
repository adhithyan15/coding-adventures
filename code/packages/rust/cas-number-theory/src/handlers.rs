//! VM-neutral handlers for number-theory IR heads.
//!
//! The handlers intentionally avoid depending on `symbolic-vm`. They accept an
//! already-built `IRApply`, fold fully numeric calls, and return the original
//! expression unchanged when the call is symbolic or malformed.

use std::collections::BTreeMap;

use symbolic_ir::{apply, int, sym, IRApply, IRNode};

use crate::{
    crt, divisors, factor_integer, integer_length, is_prime, jacobi_symbol, moebius_mu, next_prime,
    prev_prime, totient,
};

pub type Handler = fn(&IRApply) -> IRNode;

const LIST: &str = "List";
const TRUE: &str = "True";
const FALSE: &str = "False";

pub fn is_prime_handler(expr: &IRApply) -> IRNode {
    let Some([n]) = unary_int_args(expr) else {
        return unevaluated(expr);
    };
    if is_prime(n) {
        sym(TRUE)
    } else {
        sym(FALSE)
    }
}

pub fn next_prime_handler(expr: &IRApply) -> IRNode {
    let Some([n]) = unary_int_args(expr) else {
        return unevaluated(expr);
    };
    int(next_prime(n))
}

pub fn prev_prime_handler(expr: &IRApply) -> IRNode {
    let Some([n]) = unary_int_args(expr) else {
        return unevaluated(expr);
    };
    prev_prime(n).map(int).unwrap_or_else(|| unevaluated(expr))
}

pub fn factor_integer_handler(expr: &IRApply) -> IRNode {
    let Some([n]) = unary_int_args(expr) else {
        return unevaluated(expr);
    };
    if n <= 0 {
        return unevaluated(expr);
    }
    let pairs = factor_integer(n)
        .into_iter()
        .map(|(prime, exponent)| apply(sym(LIST), vec![int(prime), int(exponent as i64)]))
        .collect();
    apply(sym(LIST), pairs)
}

pub fn divisors_handler(expr: &IRApply) -> IRNode {
    let Some([n]) = unary_int_args(expr) else {
        return unevaluated(expr);
    };
    if n <= 0 {
        return unevaluated(expr);
    }
    apply(sym(LIST), divisors(n).into_iter().map(int).collect())
}

pub fn totient_handler(expr: &IRApply) -> IRNode {
    let Some([n]) = unary_int_args(expr) else {
        return unevaluated(expr);
    };
    if n <= 0 {
        return unevaluated(expr);
    }
    int(totient(n))
}

pub fn moebius_mu_handler(expr: &IRApply) -> IRNode {
    let Some([n]) = unary_int_args(expr) else {
        return unevaluated(expr);
    };
    if n <= 0 {
        return unevaluated(expr);
    }
    int(moebius_mu(n))
}

pub fn jacobi_symbol_handler(expr: &IRApply) -> IRNode {
    let Some([a, n]) = binary_int_args(expr) else {
        return unevaluated(expr);
    };
    if n <= 0 || n % 2 == 0 {
        return unevaluated(expr);
    }
    int(jacobi_symbol(a, n))
}

pub fn chinese_remainder_handler(expr: &IRApply) -> IRNode {
    if expr.args.len() != 2 {
        return unevaluated(expr);
    }
    let Some(remainders) = list_ints(&expr.args[0]) else {
        return unevaluated(expr);
    };
    let Some(moduli) = list_ints(&expr.args[1]) else {
        return unevaluated(expr);
    };
    crt(&remainders, &moduli)
        .map(int)
        .unwrap_or_else(|| unevaluated(expr))
}

pub fn integer_length_handler(expr: &IRApply) -> IRNode {
    if expr.args.len() != 1 && expr.args.len() != 2 {
        return unevaluated(expr);
    }
    let Some(n) = as_int(&expr.args[0]) else {
        return unevaluated(expr);
    };
    let base = match expr.args.get(1) {
        Some(node) => match as_int(node) {
            Some(base) if base >= 2 => base,
            _ => return unevaluated(expr),
        },
        None => 10,
    };
    int(integer_length(n, base))
}

pub fn build_number_theory_handler_table() -> BTreeMap<&'static str, Handler> {
    BTreeMap::from([
        ("IsPrime", is_prime_handler as Handler),
        ("NextPrime", next_prime_handler as Handler),
        ("PrevPrime", prev_prime_handler as Handler),
        ("FactorInteger", factor_integer_handler as Handler),
        ("Divisors", divisors_handler as Handler),
        ("Totient", totient_handler as Handler),
        ("MoebiusMu", moebius_mu_handler as Handler),
        ("JacobiSymbol", jacobi_symbol_handler as Handler),
        ("ChineseRemainder", chinese_remainder_handler as Handler),
        ("IntegerLength", integer_length_handler as Handler),
    ])
}

fn unary_int_args(expr: &IRApply) -> Option<[i64; 1]> {
    if expr.args.len() == 1 {
        as_int(&expr.args[0]).map(|n| [n])
    } else {
        None
    }
}

fn binary_int_args(expr: &IRApply) -> Option<[i64; 2]> {
    if expr.args.len() == 2 {
        Some([as_int(&expr.args[0])?, as_int(&expr.args[1])?])
    } else {
        None
    }
}

fn as_int(node: &IRNode) -> Option<i64> {
    match node {
        IRNode::Integer(n) => Some(*n),
        _ => None,
    }
}

fn list_ints(node: &IRNode) -> Option<Vec<i64>> {
    let IRNode::Apply(apply_node) = node else {
        return None;
    };
    match &apply_node.head {
        IRNode::Symbol(name) if name == LIST => apply_node.args.iter().map(as_int).collect(),
        _ => None,
    }
}

fn unevaluated(expr: &IRApply) -> IRNode {
    IRNode::Apply(Box::new(expr.clone()))
}
