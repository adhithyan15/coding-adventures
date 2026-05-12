use std::collections::BTreeMap;
use std::fmt;

use symbolic_ir::{apply, int, rat, sym, IRNode, ADD, LIST, MUL, NEG, POW, RULE, SUB};

use crate::{buchberger, ideal_solve, make_var, reduce_poly, MPoly, Rational};
use crate::{GROEBNER, IDEAL_SOLVE, POLY_REDUCE};

/// VM-neutral multivariate handler signature.
///
/// The handler receives a raw IR node and either returns the evaluated IR or
/// the original node unchanged when conversion or solving cannot proceed.
pub type MultivariateHandler = fn(&IRNode) -> IRNode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversionError(pub String);

impl fmt::Display for ConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ConversionError: {}", self.0)
    }
}

impl std::error::Error for ConversionError {}

pub fn ir_to_mpoly(node: &IRNode, var_list: &[String]) -> Result<MPoly, ConversionError> {
    let nvars = var_list.len();
    match node {
        IRNode::Integer(n) => Ok(MPoly::constant(Rational::from_int(*n), nvars)),
        IRNode::Rational(n, d) => Ok(MPoly::constant(Rational::new(*n, *d), nvars)),
        IRNode::Symbol(name) => {
            let Some(index) = var_list.iter().position(|v| v == name) else {
                return Err(ConversionError(format!(
                    "unrecognized symbol in polynomial context: {name}"
                )));
            };
            Ok(make_var(index, nvars))
        }
        IRNode::Apply(app) => {
            let Some(head) = head_name(&app.head) else {
                return Err(ConversionError("non-symbol polynomial head".to_string()));
            };
            match head {
                ADD => app.args.iter().try_fold(MPoly::zero(nvars), |acc, arg| {
                    Ok(acc + ir_to_mpoly(arg, var_list)?)
                }),
                SUB => {
                    if app.args.len() != 2 {
                        return Err(ConversionError("Sub expects 2 arguments".to_string()));
                    }
                    Ok(ir_to_mpoly(&app.args[0], var_list)? - ir_to_mpoly(&app.args[1], var_list)?)
                }
                MUL => app
                    .args
                    .iter()
                    .try_fold(MPoly::constant(1, nvars), |acc, arg| {
                        Ok(acc * ir_to_mpoly(arg, var_list)?)
                    }),
                NEG => {
                    if app.args.len() != 1 {
                        return Err(ConversionError("Neg expects 1 argument".to_string()));
                    }
                    Ok(-ir_to_mpoly(&app.args[0], var_list)?)
                }
                POW => {
                    if app.args.len() != 2 {
                        return Err(ConversionError("Pow expects 2 arguments".to_string()));
                    }
                    let exponent = match app.args[1] {
                        IRNode::Integer(n) if n >= 0 => usize::try_from(n).map_err(|_| {
                            ConversionError("Pow exponent is too large".to_string())
                        })?,
                        _ => {
                            return Err(ConversionError(
                                "Pow exponent must be a non-negative integer".to_string(),
                            ));
                        }
                    };
                    let base = ir_to_mpoly(&app.args[0], var_list)?;
                    let mut result = MPoly::constant(1, nvars);
                    for _ in 0..exponent {
                        result = result * base.clone();
                    }
                    Ok(result)
                }
                _ => Err(ConversionError(format!(
                    "cannot convert head {head} to polynomial"
                ))),
            }
        }
        other => Err(ConversionError(format!(
            "cannot convert {other:?} to polynomial"
        ))),
    }
}

pub fn mpoly_to_ir(poly: &MPoly, var_symbols: &[IRNode]) -> IRNode {
    if poly.is_zero() {
        return int(0);
    }

    let mut terms = Vec::new();
    let monomials = poly.monomials_descending("grlex").unwrap_or_else(|_| {
        let mut monomials: Vec<_> = poly.coeffs.keys().cloned().collect();
        monomials.sort();
        monomials
    });

    for monomial in monomials {
        let coeff = poly.coeffs[&monomial];
        let mut parts = Vec::new();
        for (index, &exponent) in monomial.iter().enumerate() {
            if exponent == 1 {
                parts.push(var_symbols[index].clone());
            } else if exponent > 1 {
                parts.push(apply(
                    sym(POW),
                    vec![var_symbols[index].clone(), int(exponent as i64)],
                ));
            }
        }

        if parts.is_empty() {
            terms.push(rational_to_ir(coeff));
        } else if coeff == Rational::ONE {
            terms.push(product_ir(parts));
        } else if coeff == -Rational::ONE {
            terms.push(apply(sym(NEG), vec![product_ir(parts)]));
        } else {
            let mut factors = vec![rational_to_ir(coeff)];
            factors.extend(parts);
            terms.push(apply(sym(MUL), factors));
        }
    }

    if terms.len() == 1 {
        terms.pop().unwrap()
    } else {
        apply(sym(ADD), terms)
    }
}

pub fn extract_var_list(node: &IRNode) -> Option<Vec<String>> {
    let args = list_args(node)?;
    let mut names = Vec::with_capacity(args.len());
    for arg in args {
        match arg {
            IRNode::Symbol(name) => names.push(name.clone()),
            _ => return None,
        }
    }
    Some(names)
}

pub fn extract_poly_list(node: &IRNode, var_list: &[String]) -> Option<Vec<MPoly>> {
    let args = list_args(node)?;
    let mut polys = Vec::with_capacity(args.len());
    for arg in args {
        polys.push(ir_to_mpoly(arg, var_list).ok()?);
    }
    Some(polys)
}

pub fn groebner_handler(expr: &IRNode) -> IRNode {
    let Some(args) = handler_args(expr, GROEBNER) else {
        return expr.clone();
    };
    if args.len() != 2 {
        return expr.clone();
    }
    let Some(var_list) = extract_var_list(&args[1]).filter(|vars| !vars.is_empty()) else {
        return expr.clone();
    };
    let Some(polys) = extract_poly_list(&args[0], &var_list) else {
        return expr.clone();
    };
    let Ok(basis) = buchberger(&polys, "grlex") else {
        return expr.clone();
    };
    let var_symbols: Vec<_> = var_list.iter().map(sym).collect();
    apply(
        sym(LIST),
        basis
            .iter()
            .map(|poly| mpoly_to_ir(poly, &var_symbols))
            .collect(),
    )
}

pub fn poly_reduce_handler(expr: &IRNode) -> IRNode {
    let Some(args) = handler_args(expr, POLY_REDUCE) else {
        return expr.clone();
    };
    if args.len() != 3 {
        return expr.clone();
    }
    let Some(var_list) = extract_var_list(&args[2]).filter(|vars| !vars.is_empty()) else {
        return expr.clone();
    };
    let Ok(f_poly) = ir_to_mpoly(&args[0], &var_list) else {
        return expr.clone();
    };
    let Some(polys) = extract_poly_list(&args[1], &var_list) else {
        return expr.clone();
    };
    let Ok(remainder) = reduce_poly(&f_poly, &polys, "grlex") else {
        return expr.clone();
    };
    let var_symbols: Vec<_> = var_list.iter().map(sym).collect();
    mpoly_to_ir(&remainder, &var_symbols)
}

pub fn ideal_solve_handler(expr: &IRNode) -> IRNode {
    let Some(args) = handler_args(expr, IDEAL_SOLVE) else {
        return expr.clone();
    };
    if args.len() != 2 {
        return expr.clone();
    }
    let Some(var_list) = extract_var_list(&args[1]).filter(|vars| !vars.is_empty()) else {
        return expr.clone();
    };
    let Some(polys) = extract_poly_list(&args[0], &var_list) else {
        return expr.clone();
    };
    let Some(solutions) = ideal_solve(&polys) else {
        return expr.clone();
    };

    let var_symbols: Vec<_> = var_list.iter().map(sym).collect();
    let solution_nodes: Vec<_> = solutions
        .iter()
        .filter(|solution| solution.len() == var_symbols.len())
        .map(|solution| {
            apply(
                sym(LIST),
                solution
                    .iter()
                    .enumerate()
                    .map(|(index, value)| {
                        apply(
                            sym(RULE),
                            vec![var_symbols[index].clone(), rational_to_ir(*value)],
                        )
                    })
                    .collect(),
            )
        })
        .collect();

    if solution_nodes.is_empty() {
        expr.clone()
    } else {
        apply(sym(LIST), solution_nodes)
    }
}

pub fn build_multivariate_handler_table() -> BTreeMap<&'static str, MultivariateHandler> {
    BTreeMap::from([
        (GROEBNER, groebner_handler as MultivariateHandler),
        (POLY_REDUCE, poly_reduce_handler as MultivariateHandler),
        (IDEAL_SOLVE, ideal_solve_handler as MultivariateHandler),
    ])
}

fn head_name(node: &IRNode) -> Option<&str> {
    match node {
        IRNode::Symbol(name) => Some(name),
        _ => None,
    }
}

fn handler_args<'a>(node: &'a IRNode, head: &str) -> Option<&'a [IRNode]> {
    match node {
        IRNode::Apply(app) if app.head == sym(head) => Some(&app.args),
        _ => None,
    }
}

fn list_args(node: &IRNode) -> Option<&[IRNode]> {
    handler_args(node, LIST)
}

fn product_ir(parts: Vec<IRNode>) -> IRNode {
    if parts.len() == 1 {
        parts.into_iter().next().unwrap()
    } else {
        apply(sym(MUL), parts)
    }
}

fn rational_to_ir(value: Rational) -> IRNode {
    if value.denom == 1 {
        int(value.numer)
    } else {
        rat(value.numer, value.denom)
    }
}
