use std::collections::HashMap;

use symbolic_ir::IRNode;

use crate::newton::{ir_to_float, mnewton_solve, MNewtonOptions, MNEWTON};

pub type EvalFn = dyn FnMut(IRNode) -> IRNode;
pub type DiffFn = dyn FnMut(&IRNode, &IRNode) -> IRNode;
pub type MNewtonHandler = fn(&IRNode, &mut EvalFn, &mut DiffFn) -> IRNode;
pub type MNewtonHandlerTable = HashMap<&'static str, MNewtonHandler>;

pub fn mnewton_handler(expr: &IRNode, eval_fn: &mut EvalFn, diff_fn: &mut DiffFn) -> IRNode {
    let IRNode::Apply(apply_node) = expr else {
        return expr.clone();
    };
    if !matches!(&apply_node.head, IRNode::Symbol(name) if name == MNEWTON) {
        return expr.clone();
    }
    if apply_node.args.len() != 3 && apply_node.args.len() != 4 {
        return expr.clone();
    }

    let f_ir = &apply_node.args[0];
    let x_sym = &apply_node.args[1];
    let x0_ir = &apply_node.args[2];
    if !matches!(x_sym, IRNode::Symbol(_)) || ir_to_float(x0_ir).is_none() {
        return expr.clone();
    }

    let mut options = MNewtonOptions::default();
    if let Some(tol_ir) = apply_node.args.get(3) {
        let Some(tol) = ir_to_float(tol_ir) else {
            return expr.clone();
        };
        options.tol = tol;
    }

    match mnewton_solve(f_ir, x_sym, x0_ir, eval_fn, diff_fn, options) {
        Ok(result) => result,
        Err(_) => expr.clone(),
    }
}

pub fn build_mnewton_handler_table() -> MNewtonHandlerTable {
    HashMap::from([(MNEWTON, mnewton_handler as MNewtonHandler)])
}
