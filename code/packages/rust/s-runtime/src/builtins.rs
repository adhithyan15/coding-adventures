//! Built-in functions installed into the global environment.
//!
//! These are the names an S user expects to be available without defining them:
//! the vector constructor `c`, `length`, `print`, `seq`, and the statistical
//! reductions. The reductions are thin glue over `statistics-core` — the same
//! crate that backs the spreadsheet and (eventually) R frontends — so the math
//! has a single authoritative home.

use crate::env::{define, Env};
use crate::error::{SError, SResult};
use crate::value::{bounded_sequence, combine, Arg, SValue};
use r_vector::Double;
use statistics_core::{descriptive, Number, StatsError};

/// Install every built-in into `env` (the global scope).
pub fn install(env: &Env) {
    define(env, "c", builtin("c", b_c));
    define(env, "length", builtin("length", b_length));
    define(env, "print", builtin("print", b_print));
    define(env, "seq", builtin("seq", b_seq));

    // Single-argument reductions: x is the first positional argument.
    define(env, "mean", builtin("mean", b_mean));
    define(env, "median", builtin("median", b_median));
    define(env, "var", builtin("var", b_var));
    define(env, "sd", builtin("sd", b_sd));

    // Variadic reductions: all positional arguments are combined first
    // (`sum(1, 2, 3)` is `sum(c(1, 2, 3))`).
    define(env, "sum", builtin("sum", b_sum));
    define(env, "prod", builtin("prod", b_prod));
    define(env, "min", builtin("min", b_min));
    define(env, "max", builtin("max", b_max));
}

fn builtin(name: &str, func: fn(&[Arg]) -> SResult<SValue>) -> SValue {
    SValue::Builtin {
        name: name.to_string(),
        func,
    }
}

// ===========================================================================
// Core built-ins
// ===========================================================================

/// `c(...)` — combine arguments into one vector (coercing to the highest type).
fn b_c(args: &[Arg]) -> SResult<SValue> {
    Ok(combine(args))
}

/// `length(x)` — the element count, as a length-1 numeric vector.
fn b_length(args: &[Arg]) -> SResult<SValue> {
    let v = first_positional(args)?;
    Ok(SValue::scalar(v.length() as f64))
}

/// `print(x)` — the side effect (writing output) is handled by the evaluator,
/// which formats whatever this returns. Here we simply pass the value through.
fn b_print(args: &[Arg]) -> SResult<SValue> {
    Ok(args
        .iter()
        .find(|a| a.name.is_none())
        .map(|a| a.value.clone())
        .unwrap_or(SValue::Null))
}

/// `seq(to)` is `1:to`; `seq(from, to)` is `from:to` (step 1). A minimal subset
/// of R's `seq` sufficient for v1.
fn b_seq(args: &[Arg]) -> SResult<SValue> {
    let positionals: Vec<f64> = args
        .iter()
        .filter(|a| a.name.is_none())
        .map(|a| a.value.as_double())
        .collect::<SResult<Vec<_>>>()?
        .iter()
        .map(|d| d.get_value(0).unwrap_or(f64::NAN))
        .collect();

    let (from, to) = match positionals.as_slice() {
        [to] => (1.0, *to),
        [from, to, ..] => (*from, *to),
        [] => return Err(SError::BadArgs("seq requires at least one argument".into())),
    };
    Ok(SValue::doubles(bounded_sequence(from, to)?))
}

// ===========================================================================
// Statistical reductions (glue over statistics-core)
// ===========================================================================

type Reducer = fn(&Double, bool) -> Result<Number, StatsError>;

fn b_mean(args: &[Arg]) -> SResult<SValue> {
    reduce_single(args, "mean", descriptive::mean)
}
fn b_median(args: &[Arg]) -> SResult<SValue> {
    reduce_single(args, "median", descriptive::median)
}
fn b_var(args: &[Arg]) -> SResult<SValue> {
    reduce_single(args, "var", descriptive::var)
}
fn b_sd(args: &[Arg]) -> SResult<SValue> {
    reduce_single(args, "sd", descriptive::sd)
}
fn b_sum(args: &[Arg]) -> SResult<SValue> {
    reduce_variadic(args, "sum", descriptive::sum)
}
fn b_prod(args: &[Arg]) -> SResult<SValue> {
    reduce_variadic(args, "prod", descriptive::prod)
}
fn b_min(args: &[Arg]) -> SResult<SValue> {
    reduce_variadic(args, "min", descriptive::min)
}
fn b_max(args: &[Arg]) -> SResult<SValue> {
    reduce_variadic(args, "max", descriptive::max)
}

/// Reduce the first positional argument (`mean`, `median`, `var`, `sd`).
fn reduce_single(args: &[Arg], name: &str, f: Reducer) -> SResult<SValue> {
    let data = first_positional(args)?.as_double()?;
    apply_reducer(name, f, &data, na_rm_flag(args)?)
}

/// Reduce *all* positional arguments combined (`sum`, `prod`, `min`, `max`).
fn reduce_variadic(args: &[Arg], name: &str, f: Reducer) -> SResult<SValue> {
    let positional: Vec<Arg> = args.iter().filter(|a| a.name.is_none()).cloned().collect();
    let data = combine(&positional).as_double()?;
    apply_reducer(name, f, &data, na_rm_flag(args)?)
}

fn apply_reducer(name: &str, f: Reducer, data: &Double, na_rm: bool) -> SResult<SValue> {
    match f(data, na_rm) {
        Ok(num) => Ok(SValue::scalar(num.to_f64_lossy())),
        Err(err) => Err(SError::Domain(format!("{name}: {}", describe(err)))),
    }
}

// ===========================================================================
// Argument helpers
// ===========================================================================

/// The first positional (unnamed) argument's value.
fn first_positional(args: &[Arg]) -> SResult<&SValue> {
    args.iter()
        .find(|a| a.name.is_none())
        .map(|a| &a.value)
        .ok_or_else(|| SError::BadArgs("argument \"x\" is missing".into()))
}

/// The value of the `na.rm` named argument (default `FALSE`).
fn na_rm_flag(args: &[Arg]) -> SResult<bool> {
    match args.iter().find(|a| a.name.as_deref() == Some("na.rm")) {
        Some(arg) => arg.value.truthy(),
        None => Ok(false),
    }
}

/// A short, S-flavored description of a statistics-core error.
fn describe(err: StatsError) -> String {
    match err {
        StatsError::EmptyInput { .. } => "argument has length zero".to_string(),
        StatsError::DomainError { what, .. } => what,
        other => format!("{other:?}"),
    }
}
