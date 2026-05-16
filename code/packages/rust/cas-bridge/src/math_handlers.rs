//! Handler factories that wrap `math-core` functions for the
//! symbolic VM.
//!
//! Math functions are mostly unary or binary `f64 → f64` shapes,
//! which makes the handlers trivial. The pattern: extract f64 args
//! from the IRApply, call the math-core fn, wrap result.

use std::sync::Arc;

use symbolic_ir::{IRApply, IRNode};
use symbolic_vm::{Handler, VM};

use crate::convert::{f64_to_ir, ir_to_f64};

/// Wrap a `fn(f64) -> f64` math-core function as a Handler.
fn unary_f64(f: fn(f64) -> f64) -> Handler {
    Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        if expr.args.len() != 1 {
            return IRNode::Apply(Box::new(expr));
        }
        match ir_to_f64(&expr.args[0]) {
            Some(v) => f64_to_ir(f(v)),
            None => IRNode::Apply(Box::new(expr)),
        }
    })
}

/// Wrap a `fn(f64, f64) -> f64` math-core function as a Handler.
fn binary_f64(f: fn(f64, f64) -> f64) -> Handler {
    Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        if expr.args.len() != 2 {
            return IRNode::Apply(Box::new(expr));
        }
        match (ir_to_f64(&expr.args[0]), ir_to_f64(&expr.args[1])) {
            (Some(a), Some(b)) => f64_to_ir(f(a, b)),
            _ => IRNode::Apply(Box::new(expr)),
        }
    })
}

/// Wrap a `fn() -> f64` constant.
fn const_f64(value: f64) -> Handler {
    Arc::new(move |_vm: &mut VM, _expr: IRApply| -> IRNode { f64_to_ir(value) })
}

// ---------------------------------------------------------------------------
// Per-function handler factories
// ---------------------------------------------------------------------------

/// `Abs(x)` / `ABS(x)`.
pub fn abs_handler() -> Handler {
    unary_f64(math_core::arithmetic::abs)
}
/// `Sign(x)` / `SIGN(x)`.
pub fn sign_handler() -> Handler {
    unary_f64(math_core::arithmetic::sign)
}
/// `Int(x)` / `INT(x)`.
pub fn int_handler() -> Handler {
    unary_f64(math_core::arithmetic::int)
}
/// `Sqrt(x)` / `SQRT(x)`. Domain check: returns NaN (`Apply(NA)`) for x < 0.
pub fn sqrt_handler() -> Handler {
    unary_f64(|x| if x < 0.0 { f64::NAN } else { x.sqrt() })
}
/// `Exp(x)`.
pub fn exp_handler() -> Handler {
    unary_f64(f64::exp)
}
/// `Ln(x)`.
pub fn ln_handler() -> Handler {
    unary_f64(|x| if x <= 0.0 { f64::NAN } else { x.ln() })
}
/// `Log10(x)`.
pub fn log10_handler() -> Handler {
    unary_f64(|x| if x <= 0.0 { f64::NAN } else { x.log10() })
}
/// `Log2(x)`.
pub fn log2_handler() -> Handler {
    unary_f64(|x| if x <= 0.0 { f64::NAN } else { x.log2() })
}
/// `Sin(x)`.
pub fn sin_handler() -> Handler {
    unary_f64(f64::sin)
}
/// `Cos(x)`.
pub fn cos_handler() -> Handler {
    unary_f64(f64::cos)
}
/// `Tan(x)`.
pub fn tan_handler() -> Handler {
    unary_f64(f64::tan)
}
/// `Asin(x)`. Domain: [-1, 1].
pub fn asin_handler() -> Handler {
    unary_f64(|x| if !(-1.0..=1.0).contains(&x) { f64::NAN } else { x.asin() })
}
/// `Acos(x)`. Domain: [-1, 1].
pub fn acos_handler() -> Handler {
    unary_f64(|x| if !(-1.0..=1.0).contains(&x) { f64::NAN } else { x.acos() })
}
/// `Atan(x)`.
pub fn atan_handler() -> Handler {
    unary_f64(f64::atan)
}
/// `Sinh(x)`.
pub fn sinh_handler() -> Handler {
    unary_f64(f64::sinh)
}
/// `Cosh(x)`.
pub fn cosh_handler() -> Handler {
    unary_f64(f64::cosh)
}
/// `Tanh(x)`.
pub fn tanh_handler() -> Handler {
    unary_f64(f64::tanh)
}
/// `Power(x, n)` / `POWER(x, n)`. Two-arg.
pub fn power_handler() -> Handler {
    binary_f64(|x, n| x.powf(n))
}
/// `Atan2(y, x)`.
pub fn atan2_handler() -> Handler {
    binary_f64(f64::atan2)
}
/// `Degrees(rad)`.
pub fn degrees_handler() -> Handler {
    unary_f64(math_core::conversion::degrees)
}
/// `Radians(deg)`.
pub fn radians_handler() -> Handler {
    unary_f64(math_core::conversion::radians)
}
/// `Mod(a, b)` / `MOD(a, b)`. Excel-style mod.
pub fn mod_handler() -> Handler {
    binary_f64(|a, b| {
        if b == 0.0 {
            f64::NAN
        } else {
            a - (a / b).floor() * b
        }
    })
}
/// `Pi` constant.
pub fn pi_handler() -> Handler {
    const_f64(core::f64::consts::PI)
}
/// `E` constant.
pub fn e_handler() -> Handler {
    const_f64(core::f64::consts::E)
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

/// Populate `registry` with the math-core handlers.
pub fn register_math_handlers(registry: &mut crate::registry::HandlerRegistry) {
    registry.register_aliases(["Abs", "ABS"], abs_handler());
    registry.register_aliases(["Sign", "SIGN"], sign_handler());
    registry.register_aliases(["Int", "INT"], int_handler());
    registry.register_aliases(["Sqrt", "SQRT"], sqrt_handler());
    registry.register_aliases(["Exp", "EXP"], exp_handler());
    registry.register_aliases(["Ln", "LN"], ln_handler());
    registry.register_aliases(["Log10", "LOG10"], log10_handler());
    registry.register_aliases(["Log2", "LOG2"], log2_handler());
    registry.register_aliases(["Sin", "SIN"], sin_handler());
    registry.register_aliases(["Cos", "COS"], cos_handler());
    registry.register_aliases(["Tan", "TAN"], tan_handler());
    registry.register_aliases(["Asin", "ASIN"], asin_handler());
    registry.register_aliases(["Acos", "ACOS"], acos_handler());
    registry.register_aliases(["Atan", "ATAN"], atan_handler());
    registry.register_aliases(["Sinh", "SINH"], sinh_handler());
    registry.register_aliases(["Cosh", "COSH"], cosh_handler());
    registry.register_aliases(["Tanh", "TANH"], tanh_handler());
    registry.register_aliases(["Power", "POWER", "Pow", "POW"], power_handler());
    registry.register_aliases(["Atan2", "ATAN2"], atan2_handler());
    registry.register_aliases(["Degrees", "DEGREES"], degrees_handler());
    registry.register_aliases(["Radians", "RADIANS"], radians_handler());
    registry.register_aliases(["Mod", "MOD"], mod_handler());
    registry.register_aliases(["Pi", "PI"], pi_handler());
    registry.register_aliases(["E", "Euler"], e_handler());
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use symbolic_ir::{apply, sym, IRNode};
    use symbolic_vm::SymbolicBackend;

    fn eval_with(name: &str, args: Vec<IRNode>) -> IRNode {
        use crate::registry::HandlerRegistry;
        let mut registry = HandlerRegistry::new();
        register_math_handlers(&mut registry);
        let handler = registry.get(name).expect("registered").clone();
        let mut vm = VM::new(Box::new(SymbolicBackend::new()));
        let expr = IRApply {
            head: sym(name),
            args,
        };
        handler(&mut vm, expr)
    }

    fn close_f64(n: &IRNode, expected: f64) -> bool {
        match n {
            IRNode::Float(v) => (v - expected).abs() < 1e-9,
            IRNode::Integer(v) => ((*v as f64) - expected).abs() < 1e-9,
            _ => false,
        }
    }

    #[test]
    fn sqrt_known_values() {
        assert!(close_f64(&eval_with("Sqrt", vec![IRNode::Integer(4)]), 2.0));
        assert!(close_f64(&eval_with("SQRT", vec![IRNode::Integer(9)]), 3.0));
    }

    #[test]
    fn sqrt_negative_returns_nan_float() {
        let r = eval_with("Sqrt", vec![IRNode::Integer(-1)]);
        // f64::NAN is a real NaN, not the r-vector NA bit pattern;
        // it surfaces as IRNode::Float(NaN). (R-vector NA is a
        // specific NaN payload — a true domain-undefined NaN should
        // *not* be conflated with it.)
        match r {
            IRNode::Float(v) => assert!(v.is_nan(), "expected NaN, got {v}"),
            other => panic!("expected Float(NaN), got {other:?}"),
        }
    }

    #[test]
    fn power_two_args() {
        let r = eval_with("Power", vec![IRNode::Integer(2), IRNode::Integer(10)]);
        assert!(close_f64(&r, 1024.0));
    }

    #[test]
    fn pi_constant() {
        let r = eval_with("PI", vec![]);
        assert!(close_f64(&r, core::f64::consts::PI));
    }

    #[test]
    fn handler_passes_through_symbolic() {
        let r = eval_with("Sin", vec![sym("x")]);
        match r {
            IRNode::Apply(boxed) => assert_eq!(boxed.head, sym("Sin")),
            _ => panic!("expected symbolic pass-through"),
        }
    }

    #[test]
    fn aliases_share_handler() {
        let abs_int = eval_with("Abs", vec![IRNode::Integer(-5)]);
        let abs_excel = eval_with("ABS", vec![IRNode::Integer(-5)]);
        assert_eq!(abs_int, abs_excel);
        assert!(close_f64(&abs_int, 5.0));
    }
}
