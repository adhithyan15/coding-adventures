//! Advanced limit handling with injectable simplification and differentiation.
//!
//! This module is a focused Rust port of Python's `limit_advanced.py`.  It
//! keeps the crate WASM-friendly and independent from the symbolic VM by taking
//! optional callbacks for differentiation and evaluation.  When a callback is
//! needed but absent, the result is returned as an unevaluated `Limit(...)`.

use cas_substitution::subst;
use symbolic_ir::{
    apply, flt, int, sym, IRNode, ADD, ATAN, COS, COSH, DIV, EXP, LOG, MUL, NEG, POW, SIN, SINH,
    SQRT, SUB, TAN, TANH,
};

use crate::LIMIT;

const EPS: f64 = 1e-300;
const INF_THRESHOLD: f64 = 1e100;
const DEFAULT_MAX_DEPTH: usize = 8;

/// Direction for one-sided limit classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitDirection {
    /// Approach from the right.
    Plus,
    /// Approach from the left.
    Minus,
}

/// Differentiation callback used by L'Hopital-style reductions.
pub type DiffFn<'a> = dyn Fn(&IRNode, &IRNode) -> IRNode + 'a;

/// Optional evaluation/simplification callback for intermediate IR.
pub type EvalFn<'a> = dyn Fn(IRNode) -> IRNode + 'a;

/// Options for [`limit_advanced`].
#[derive(Default)]
pub struct LimitAdvancedOptions<'a> {
    /// Optional one-sided direction.
    pub direction: Option<LimitDirection>,
    /// Optional symbolic derivative callback.
    pub diff_fn: Option<&'a DiffFn<'a>>,
    /// Optional VM/simplifier callback.
    pub eval_fn: Option<&'a EvalFn<'a>>,
    /// Optional recursion limit. Defaults to 8, matching the Python slice.
    pub max_depth: Option<usize>,
}

/// Compute an advanced limit for common indeterminate forms.
///
/// The implementation first classifies the expression numerically around the
/// limit point.  Direct finite cases return the exact substituted IR.  Common
/// indeterminate forms (`0/0`, `inf/inf`, `0*inf`, and selected power forms)
/// are reduced with injected callbacks when available; otherwise an unevaluated
/// `Limit(expr, var, point[, direction])` node is returned.
pub fn limit_advanced<'a>(
    expr: IRNode,
    var: &IRNode,
    point: IRNode,
    options: LimitAdvancedOptions<'a>,
) -> IRNode {
    let max_depth = options.max_depth.unwrap_or(DEFAULT_MAX_DEPTH);
    limit_advanced_inner(expr, var, point, &options, 0, max_depth)
}

fn limit_advanced_inner(
    expr: IRNode,
    var: &IRNode,
    point: IRNode,
    options: &LimitAdvancedOptions<'_>,
    depth: usize,
    max_depth: usize,
) -> IRNode {
    if depth > max_depth {
        return build_unevaluated(expr, var, point, options.direction);
    }

    let pt_f = num_eval(&point);
    if pt_f.is_nan() {
        return build_unevaluated(expr, var, point, options.direction);
    }

    let eps = if options.direction == Some(LimitDirection::Minus) {
        -EPS
    } else {
        EPS
    };
    let test_pt = if pt_f.is_infinite() { pt_f } else { pt_f + eps };
    let val = eval_at_float(&expr, var, test_pt);

    if is_effectively_inf(val) {
        return infinity_symbol(val.is_sign_positive());
    }

    if val.is_nan() {
        return handle_form(expr, var, point, options, depth, max_depth);
    }

    let mut subst_result = subst(point.clone(), var, expr.clone());
    if let Some(eval_fn) = options.eval_fn {
        subst_result = eval_fn(subst_result);
    }
    let exact_val = num_eval(&subst_result);

    if !exact_val.is_nan() {
        if exact_val.is_infinite() {
            return infinity_symbol(exact_val.is_sign_positive());
        }
        return subst_result;
    }

    handle_form(expr, var, point, options, depth, max_depth)
}

fn handle_form(
    expr: IRNode,
    var: &IRNode,
    point: IRNode,
    options: &LimitAdvancedOptions<'_>,
    depth: usize,
    max_depth: usize,
) -> IRNode {
    let exact_pt = num_eval(&point);

    if let IRNode::Apply(a) = &expr {
        if a.head == sym(DIV) && a.args.len() == 2 {
            let numer = &a.args[0];
            let denom = &a.args[1];
            let n_val = eval_at_float(numer, var, exact_pt);
            let d_val = eval_at_float(denom, var, exact_pt);
            let zero_zero = n_val == 0.0 && d_val == 0.0;
            let inf_inf = is_effectively_inf(n_val) && is_effectively_inf(d_val);
            if zero_zero || inf_inf {
                if let Some(diff_fn) = options.diff_fn {
                    return lhopital_step(
                        numer, denom, var, point, options, diff_fn, depth, max_depth,
                    );
                }
            }
        }

        if a.head == sym(MUL) && a.args.len() == 2 {
            if let Some(result) = zero_inf_rewrite(
                &a.args[0], &a.args[1], var, &point, exact_pt, options, depth, max_depth,
            ) {
                return result;
            }
        }

        if a.head == sym(POW) && a.args.len() == 2 {
            if let Some(result) = pow_exp_log(
                &a.args[0], &a.args[1], var, &point, exact_pt, options, depth, max_depth,
            ) {
                return result;
            }
        }
    }

    build_unevaluated(expr, var, point, options.direction)
}

fn lhopital_step(
    numer: &IRNode,
    denom: &IRNode,
    var: &IRNode,
    point: IRNode,
    options: &LimitAdvancedOptions<'_>,
    diff_fn: &DiffFn<'_>,
    depth: usize,
    max_depth: usize,
) -> IRNode {
    let mut d_numer = diff_fn(numer, var);
    let mut d_denom = diff_fn(denom, var);
    if let Some(eval_fn) = options.eval_fn {
        d_numer = eval_fn(d_numer);
        d_denom = eval_fn(d_denom);
    }
    let mut new_ratio = apply(sym(DIV), vec![d_numer, d_denom]);
    if let Some(eval_fn) = options.eval_fn {
        new_ratio = eval_fn(new_ratio);
    }
    limit_advanced_inner(new_ratio, var, point, options, depth + 1, max_depth)
}

fn zero_inf_rewrite(
    a: &IRNode,
    b: &IRNode,
    var: &IRNode,
    point: &IRNode,
    exact_pt: f64,
    options: &LimitAdvancedOptions<'_>,
    depth: usize,
    max_depth: usize,
) -> Option<IRNode> {
    let a_val = eval_at_float(a, var, exact_pt);
    let b_val = eval_at_float(b, var, exact_pt);

    if a_val == 0.0 && is_effectively_inf(b_val) {
        let one_over_a = apply(sym(DIV), vec![int(1), a.clone()]);
        let mut new_expr = apply(sym(DIV), vec![b.clone(), one_over_a]);
        if let Some(eval_fn) = options.eval_fn {
            new_expr = eval_fn(new_expr);
        }
        let rewritten_options = without_direction(options);
        return Some(limit_advanced_inner(
            new_expr,
            var,
            point.clone(),
            &rewritten_options,
            depth + 1,
            max_depth,
        ));
    }

    if b_val == 0.0 && is_effectively_inf(a_val) {
        let one_over_b = apply(sym(DIV), vec![int(1), b.clone()]);
        let mut new_expr = apply(sym(DIV), vec![a.clone(), one_over_b]);
        if let Some(eval_fn) = options.eval_fn {
            new_expr = eval_fn(new_expr);
        }
        let rewritten_options = without_direction(options);
        return Some(limit_advanced_inner(
            new_expr,
            var,
            point.clone(),
            &rewritten_options,
            depth + 1,
            max_depth,
        ));
    }

    None
}

fn pow_exp_log(
    base: &IRNode,
    exponent: &IRNode,
    var: &IRNode,
    point: &IRNode,
    exact_pt: f64,
    options: &LimitAdvancedOptions<'_>,
    depth: usize,
    max_depth: usize,
) -> Option<IRNode> {
    let b_val = eval_at_float(base, var, exact_pt);
    let e_val = eval_at_float(exponent, var, exact_pt);

    let is_1_inf = (b_val - 1.0).abs() < 1e-10 && is_effectively_inf(e_val);
    let is_0_0 = b_val == 0.0 && e_val == 0.0;
    let is_inf_0 = is_effectively_inf(b_val) && e_val == 0.0;

    if !(is_1_inf || is_0_0 || is_inf_0) {
        return None;
    }

    let log_base = apply(sym(LOG), vec![base.clone()]);
    let mut product = apply(sym(MUL), vec![exponent.clone(), log_base]);
    if let Some(eval_fn) = options.eval_fn {
        product = eval_fn(product);
    }
    let rewritten_options = without_direction(options);
    let exponent_limit = limit_advanced_inner(
        product,
        var,
        point.clone(),
        &rewritten_options,
        depth + 1,
        max_depth,
    );
    let mut result = apply(sym(EXP), vec![exponent_limit]);
    if let Some(eval_fn) = options.eval_fn {
        result = eval_fn(result);
    }
    Some(result)
}

fn build_unevaluated(
    expr: IRNode,
    var: &IRNode,
    point: IRNode,
    direction: Option<LimitDirection>,
) -> IRNode {
    let mut args = vec![expr, var.clone(), point];
    if let Some(direction) = direction {
        args.push(match direction {
            LimitDirection::Plus => sym("plus"),
            LimitDirection::Minus => sym("minus"),
        });
    }
    apply(sym(LIMIT), args)
}

fn infinity_symbol(positive: bool) -> IRNode {
    if positive {
        sym("inf")
    } else {
        sym("minf")
    }
}

fn eval_at_float(expr: &IRNode, var: &IRNode, pt: f64) -> f64 {
    let substituted = subst(flt(pt), var, expr.clone());
    num_eval(&substituted)
}

fn is_effectively_inf(v: f64) -> bool {
    v.is_infinite() || v.abs() > INF_THRESHOLD
}

fn num_eval(node: &IRNode) -> f64 {
    ev(node)
}

fn ev(node: &IRNode) -> f64 {
    match node {
        IRNode::Integer(v) => *v as f64,
        IRNode::Rational(n, d) => *n as f64 / *d as f64,
        IRNode::Float(v) => *v,
        IRNode::Symbol(name) => match name.as_str() {
            "inf" => f64::INFINITY,
            "minf" => f64::NEG_INFINITY,
            "%pi" => std::f64::consts::PI,
            "%e" => std::f64::consts::E,
            _ => f64::NAN,
        },
        IRNode::Str(_) => f64::NAN,
        IRNode::Apply(a) => ev_apply(&a.head, &a.args),
    }
}

fn ev_apply(head: &IRNode, args: &[IRNode]) -> f64 {
    let Some(head) = symbol_name(head) else {
        return f64::NAN;
    };

    match head {
        ADD => args.iter().map(ev).sum(),
        SUB if args.len() == 2 => ev(&args[0]) - ev(&args[1]),
        MUL => args.iter().map(ev).product(),
        DIV if args.len() == 2 => {
            let numer = ev(&args[0]);
            let denom = ev(&args[1]);
            if denom == 0.0 {
                if numer == 0.0 {
                    f64::NAN
                } else {
                    numer.signum() * f64::INFINITY
                }
            } else {
                numer / denom
            }
        }
        NEG if args.len() == 1 => -ev(&args[0]),
        POW if args.len() == 2 => ev_pow(&args[0], &args[1]),
        SQRT if args.len() == 1 => {
            let val = ev(&args[0]);
            if val < 0.0 {
                f64::NAN
            } else {
                val.sqrt()
            }
        }
        EXP if args.len() == 1 => ev(&args[0]).exp(),
        LOG if args.len() == 1 => {
            let val = ev(&args[0]);
            if val == 0.0 {
                f64::NEG_INFINITY
            } else if val < 0.0 {
                f64::NAN
            } else {
                val.ln()
            }
        }
        SIN if args.len() == 1 => ev_trig(&args[0], f64::sin),
        COS if args.len() == 1 => ev_trig(&args[0], f64::cos),
        TAN if args.len() == 1 => ev_trig(&args[0], f64::tan),
        ATAN if args.len() == 1 => ev(&args[0]).atan(),
        SINH if args.len() == 1 => ev(&args[0]).sinh(),
        COSH if args.len() == 1 => ev(&args[0]).cosh(),
        TANH if args.len() == 1 => ev(&args[0]).tanh(),
        _ => f64::NAN,
    }
}

fn ev_pow(base: &IRNode, exponent: &IRNode) -> f64 {
    let base = ev(base);
    let exp_val = ev(exponent);
    if (base - 1.0).abs() < 1e-10 && is_effectively_inf(exp_val) {
        return f64::NAN;
    }
    if base == 0.0 && exp_val == 0.0 {
        return f64::NAN;
    }
    if is_effectively_inf(base) && exp_val == 0.0 {
        return f64::NAN;
    }
    if base < 0.0 && exp_val.fract() != 0.0 {
        return f64::NAN;
    }
    base.powf(exp_val)
}

fn ev_trig(arg: &IRNode, f: fn(f64) -> f64) -> f64 {
    let val = ev(arg);
    if val.is_infinite() {
        f64::NAN
    } else {
        f(val)
    }
}

fn symbol_name(node: &IRNode) -> Option<&str> {
    match node {
        IRNode::Symbol(name) => Some(name.as_str()),
        _ => None,
    }
}

fn without_direction<'a>(options: &LimitAdvancedOptions<'a>) -> LimitAdvancedOptions<'a> {
    LimitAdvancedOptions {
        direction: None,
        diff_fn: options.diff_fn,
        eval_fn: options.eval_fn,
        max_depth: options.max_depth,
    }
}
