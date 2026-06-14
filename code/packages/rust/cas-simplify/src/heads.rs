//! Sentinel heads introduced by `cas-simplify`.

use symbolic_ir::{apply, sym, IRNode};

pub const SIMPLIFY: &str = "Simplify";
pub const CANONICAL: &str = "Canonical";

pub const ASSUME: &str = "Assume";
pub const FORGET: &str = "Forget";
pub const IS: &str = "Is";
pub const SIGN: &str = "Sign";

pub const RADCAN: &str = "Radcan";
pub const LOGCONTRACT: &str = "LogContract";
pub const LOGEXPAND: &str = "LogExpand";
pub const EXPONENTIALIZE: &str = "Exponentialize";
pub const DEMOIVRE: &str = "DeMoivre";

pub fn is_commutative_flat(head_name: &str) -> bool {
    matches!(head_name, symbolic_ir::ADD | symbolic_ir::MUL)
}

pub fn unary(head: &str, arg: IRNode) -> IRNode {
    apply(sym(head), vec![arg])
}

pub fn binary(head: &str, lhs: IRNode, rhs: IRNode) -> IRNode {
    apply(sym(head), vec![lhs, rhs])
}
