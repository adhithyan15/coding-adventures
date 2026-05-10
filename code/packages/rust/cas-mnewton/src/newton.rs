use std::error::Error;
use std::fmt;

use cas_substitution::subst;
use symbolic_ir::IRNode;

pub const MNEWTON: &str = "MNewton";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MNewtonOptions {
    pub tol: f64,
    pub max_iter: usize,
}

impl Default for MNewtonOptions {
    fn default() -> Self {
        Self {
            tol: 1e-10,
            max_iter: 50,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MNewtonError {
    ZeroDerivative { x: f64 },
}

impl fmt::Display for MNewtonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroDerivative { x } => {
                write!(f, "Newton's method: derivative is zero at x = {x:?}")
            }
        }
    }
}

impl Error for MNewtonError {}

pub fn ir_to_float(node: &IRNode) -> Option<f64> {
    match node {
        IRNode::Integer(value) => Some(*value as f64),
        IRNode::Rational(numer, denom) => Some(*numer as f64 / *denom as f64),
        IRNode::Float(value) => Some(*value),
        _ => None,
    }
}

pub fn mnewton_solve<E, D>(
    f_ir: &IRNode,
    x_sym: &IRNode,
    x0_ir: &IRNode,
    mut eval_fn: E,
    mut diff_fn: D,
    options: MNewtonOptions,
) -> Result<IRNode, MNewtonError>
where
    E: FnMut(IRNode) -> IRNode,
    D: FnMut(&IRNode, &IRNode) -> IRNode,
{
    let f_prime_ir = eval_fn(diff_fn(f_ir, x_sym));
    let Some(mut x_n) = ir_to_float(x0_ir) else {
        return Ok(f_ir.clone());
    };

    for _ in 0..options.max_iter {
        let x_n_ir = IRNode::Float(x_n);
        let f_xn_ir = eval_fn(subst(x_n_ir.clone(), x_sym, f_ir.clone()));
        let Some(f_xn) = ir_to_float(&f_xn_ir) else {
            return Ok(f_ir.clone());
        };

        if f_xn.abs() < options.tol {
            return Ok(IRNode::Float(x_n));
        }

        let f_prime_xn_ir = eval_fn(subst(x_n_ir, x_sym, f_prime_ir.clone()));
        let Some(f_prime_xn) = ir_to_float(&f_prime_xn_ir) else {
            return Ok(f_ir.clone());
        };

        if f_prime_xn.abs() < 1e-300 {
            return Err(MNewtonError::ZeroDerivative { x: x_n });
        }

        x_n -= f_xn / f_prime_xn;
    }

    Ok(IRNode::Float(x_n))
}
