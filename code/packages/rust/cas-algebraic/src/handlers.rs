use std::collections::BTreeMap;

use symbolic_ir::{IRApply, IRNode};

use crate::{
    algebraic::factor_over_extension,
    ir::{extract_radical_d, factors_to_ir, ir_to_cleared_integer_poly, ALG_FACTOR},
};

pub type AlgFactorHandler = fn(&IRApply) -> IRNode;
pub type AlgFactorHandlerTable = BTreeMap<&'static str, AlgFactorHandler>;

pub fn alg_factor_handler(expr: &IRApply) -> IRNode {
    if !is_alg_factor_application(expr) {
        return original_expr(expr);
    }

    let poly_ir = expr.args[0].clone();
    alg_factor_handler_from_poly(expr, &poly_ir)
}

pub fn alg_factor_handler_with_eval<F>(expr: &IRApply, eval: F) -> IRNode
where
    F: FnOnce(&IRNode) -> IRNode,
{
    if !is_alg_factor_application(expr) {
        return original_expr(expr);
    }

    let poly_ir = eval(&expr.args[0]);
    alg_factor_handler_from_poly(expr, &poly_ir)
}

pub fn build_alg_factor_handler_table() -> AlgFactorHandlerTable {
    let mut table = BTreeMap::new();
    table.insert(ALG_FACTOR, alg_factor_handler as AlgFactorHandler);
    table
}

fn alg_factor_handler_from_poly(expr: &IRApply, poly_ir: &IRNode) -> IRNode {
    let sqrt_ir = &expr.args[1];
    let Some(d) = extract_radical_d(sqrt_ir) else {
        return original_expr(expr);
    };
    let Some(variable) = find_variable(poly_ir) else {
        return original_expr(expr);
    };
    let Some(coeffs) = ir_to_cleared_integer_poly(poly_ir, &variable) else {
        return original_expr(expr);
    };
    let Some(factors) = factor_over_extension(&coeffs, d) else {
        return original_expr(expr);
    };

    factors_to_ir(&factors, &variable, sqrt_ir)
}

fn is_alg_factor_application(expr: &IRApply) -> bool {
    expr.head == IRNode::Symbol(ALG_FACTOR.to_string()) && expr.args.len() == 2
}

fn original_expr(expr: &IRApply) -> IRNode {
    IRNode::Apply(Box::new(expr.clone()))
}

fn find_variable(node: &IRNode) -> Option<IRNode> {
    match node {
        IRNode::Symbol(name) if !is_constant_name(name) => Some(node.clone()),
        IRNode::Apply(apply) => apply.args.iter().find_map(find_variable),
        _ => None,
    }
}

fn is_constant_name(name: &str) -> bool {
    matches!(name, "True" | "False" | "%pi" | "%e" | "%i")
}
