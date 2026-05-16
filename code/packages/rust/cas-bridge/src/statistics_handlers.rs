//! Handler factories that wrap `statistics-core` functions for the
//! symbolic VM.
//!
//! Each factory returns a `Handler` (`Arc<dyn Fn>`) that:
//!
//! 1. Tries to convert the `IRApply.args` into a `Double` vector.
//! 2. If all args are numeric, calls the underlying statistics-core
//!    function. On success, wraps the result back into an `IRNode`.
//!    On error, leaves the expression symbolic (matches MACSYMA's
//!    "preserve undefined" policy).
//! 3. If any arg is symbolic (a `Symbol`, an unevaluated `Apply`,
//!    etc.), returns the original `IRApply` untouched so the caller
//!    can hold it for later substitution.
//!
//! `na_rm` defaults to `true` for every reduction — the convention
//! Excel uses (`AVERAGE` ignores `#N/A`). R callers who want
//! `na.rm = FALSE` use the named-argument frontend handlers in
//! `r-runtime` (forthcoming) which dispatch with `na_rm = false`
//! explicitly before reaching this layer.

use std::sync::Arc;

use symbolic_ir::{IRApply, IRNode};
use symbolic_vm::{Handler, VM};

use crate::convert::{args_to_double, number_to_ir};

/// Build the "extract Double, call core fn, wrap result" template. If
/// the args aren't all numeric, returns the expression untouched
/// (symbolic pass-through).
fn make_reduction<F>(f: F) -> Handler
where
    F: Fn(&r_vector::Double) -> IRNode + Send + Sync + 'static,
{
    Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        match args_to_double(&expr.args) {
            Some(d) => f(&d),
            None => IRNode::Apply(Box::new(expr)),
        }
    })
}

/// Build a result-returning reduction handler.
fn reduction_to_number<R>(f: R) -> Handler
where
    R: Fn(&r_vector::Double) -> Result<numeric_tower::Number, statistics_core::StatsError>
        + Send
        + Sync
        + 'static,
{
    Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        match args_to_double(&expr.args) {
            Some(d) => match f(&d) {
                Ok(n) => number_to_ir(n),
                Err(_) => IRNode::Apply(Box::new(expr)),
            },
            None => IRNode::Apply(Box::new(expr)),
        }
    })
}

/// Build a usize-returning handler (count family).
fn reduction_to_count<C>(f: C) -> Handler
where
    C: Fn(&r_vector::Double) -> usize + Send + Sync + 'static,
{
    Arc::new(move |_vm: &mut VM, expr: IRApply| -> IRNode {
        match args_to_double(&expr.args) {
            Some(d) => IRNode::Integer(f(&d) as i64),
            None => IRNode::Apply(Box::new(expr)),
        }
    })
}

// ---------------------------------------------------------------------------
// Descriptive-stats handlers
// ---------------------------------------------------------------------------

/// `Sum(values...)` / `SUM(range)`.
pub fn sum_handler() -> Handler {
    reduction_to_number(|d| statistics_core::descriptive::sum(d, true))
}

/// `Prod(values...)` / `PRODUCT(range)`.
pub fn prod_handler() -> Handler {
    reduction_to_number(|d| statistics_core::descriptive::prod(d, true))
}

/// `Mean(values...)` / `AVERAGE(range)`.
pub fn mean_handler() -> Handler {
    reduction_to_number(|d| statistics_core::descriptive::mean(d, true))
}

/// `Median(values...)`.
pub fn median_handler() -> Handler {
    reduction_to_number(|d| statistics_core::descriptive::median(d, true))
}

/// `Var(values...)` / `VAR.S(range)`. Sample variance (Bessel's
/// correction).
pub fn var_handler() -> Handler {
    reduction_to_number(|d| statistics_core::descriptive::var(d, true))
}

/// `Sd(values...)` / `STDEV.S(range)`. Sample standard deviation.
pub fn sd_handler() -> Handler {
    reduction_to_number(|d| statistics_core::descriptive::sd(d, true))
}

/// `VarP(values...)` / `VAR.P(range)`. Population variance.
pub fn var_pop_handler() -> Handler {
    reduction_to_number(|d| statistics_core::descriptive::var_pop(d, true))
}

/// `SdP(values...)` / `STDEV.P(range)`. Population standard deviation.
pub fn sd_pop_handler() -> Handler {
    reduction_to_number(|d| statistics_core::descriptive::sd_pop(d, true))
}

/// `Min(values...)`.
pub fn min_handler() -> Handler {
    reduction_to_number(|d| statistics_core::descriptive::min(d, true))
}

/// `Max(values...)`.
pub fn max_handler() -> Handler {
    reduction_to_number(|d| statistics_core::descriptive::max(d, true))
}

// ---------------------------------------------------------------------------
// Counting handlers
// ---------------------------------------------------------------------------

/// `Count(values...)` / `COUNT(range)`. Counts non-NA numeric values.
pub fn count_handler() -> Handler {
    reduction_to_count(|d| statistics_core::counting::count_non_na(d))
}

/// `CountA(values...)` / `COUNTA(range)`. Counts non-blank values.
/// Operates on Double — same as `Count` until r-vector grows mixed-
/// type vectors. Documented in the PR; future Phase 2 will widen.
pub fn count_a_handler() -> Handler {
    reduction_to_count(|d| statistics_core::counting::count_non_na(d))
}

/// `Length(values...)`. Structural length — includes NAs (matches R's
/// `length()` not `COUNT` semantics).
pub fn length_handler() -> Handler {
    Arc::new(|_vm: &mut VM, expr: IRApply| -> IRNode {
        match args_to_double(&expr.args) {
            Some(d) => {
                use r_vector::Vector as _;
                IRNode::Integer(d.len() as i64)
            }
            None => IRNode::Apply(Box::new(expr)),
        }
    })
}

// Silence unused-helper warning when no caller uses the generic
// `make_reduction` factory directly (every public function above goes
// through a Result-returning specialisation). Keeping `make_reduction`
// exported lets downstream cores (math-core, financial-core) build
// handlers without re-implementing the IR-extraction boilerplate.
#[allow(dead_code)]
fn _keep_make_reduction_alive() {
    let _: Handler = make_reduction(|d| {
        use r_vector::Vector as _;
        IRNode::Integer(d.len() as i64)
    });
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
        use crate::registry::{register_statistics_handlers, HandlerRegistry};
        let mut registry = HandlerRegistry::new();
        register_statistics_handlers(&mut registry);
        let handler = registry.get(name).expect("registered").clone();
        let mut vm = VM::new(Box::new(SymbolicBackend::new()));
        let expr = IRApply {
            head: sym(name),
            args,
        };
        handler(&mut vm, expr)
    }

    #[test]
    fn mean_of_integers_returns_rational_or_float() {
        let r = eval_with("Mean", vec![IRNode::Integer(1), IRNode::Integer(2), IRNode::Integer(3)]);
        // Either Integer(2), Rational(2,1) (which collapses to Integer(2)), or Float(2.0).
        match r {
            IRNode::Integer(2) => {}
            IRNode::Float(v) if (v - 2.0).abs() < 1e-9 => {}
            other => panic!("unexpected mean result: {other:?}"),
        }
    }

    #[test]
    fn sum_aliases_excel_name() {
        // SUM and Sum and AVERAGE/Mean all dispatch through the same handlers.
        let s = eval_with("SUM", vec![IRNode::Integer(1), IRNode::Integer(2), IRNode::Integer(3)]);
        let m = eval_with("AVERAGE", vec![IRNode::Integer(1), IRNode::Integer(2), IRNode::Integer(3)]);
        assert!(matches!(s, IRNode::Integer(6) | IRNode::Float(_)));
        match m {
            IRNode::Integer(2) | IRNode::Float(_) => {}
            other => panic!("AVERAGE returned: {other:?}"),
        }
    }

    #[test]
    fn handler_passes_through_symbolic_args() {
        // Mean(x, 1) — `x` is unbound. Handler should pass through.
        let r = eval_with("Mean", vec![sym("x"), IRNode::Integer(1)]);
        // The result is the un-evaluated Apply.
        match r {
            IRNode::Apply(boxed) => {
                assert_eq!(boxed.head, sym("Mean"));
                assert_eq!(boxed.args.len(), 2);
            }
            other => panic!("expected symbolic pass-through, got {other:?}"),
        }
    }

    #[test]
    fn count_returns_integer_count() {
        let r = eval_with(
            "Count",
            vec![IRNode::Integer(1), IRNode::Integer(2), IRNode::Integer(3)],
        );
        assert_eq!(r, IRNode::Integer(3));
    }

    #[test]
    fn length_includes_na_unlike_count() {
        let r_len = eval_with(
            "Length",
            vec![IRNode::Integer(1), apply(sym("NA"), vec![]), IRNode::Integer(3)],
        );
        let r_count = eval_with(
            "Count",
            vec![IRNode::Integer(1), apply(sym("NA"), vec![]), IRNode::Integer(3)],
        );
        assert_eq!(r_len, IRNode::Integer(3));
        assert_eq!(r_count, IRNode::Integer(2));
    }

    #[test]
    fn flattens_list_wrapper() {
        // Sum(List(1, 2, 3)) == Sum(1, 2, 3) — frontends pass either.
        let list = apply(
            sym("List"),
            vec![IRNode::Integer(1), IRNode::Integer(2), IRNode::Integer(3)],
        );
        let r = eval_with("Sum", vec![list]);
        match r {
            IRNode::Integer(6) | IRNode::Float(_) => {}
            other => panic!("Sum(List(...)) returned {other:?}"),
        }
    }
}
