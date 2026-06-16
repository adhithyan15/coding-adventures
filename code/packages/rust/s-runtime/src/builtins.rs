//! Built-in functions installed into the global environment.
//!
//! These are the names an S user expects to be available without defining them:
//! the vector constructor `c`, `length`, `print`, `seq`, and the statistical
//! reductions. The reductions are thin glue over `statistics-core` — the same
//! crate that backs the spreadsheet and (eventually) R frontends — so the math
//! has a single authoritative home.

use crate::env::{define, Env};
use crate::error::{SError, SResult};
use crate::eval::{nth_element, Interpreter};
use crate::value::{bounded_sequence, class_of, combine, index, Arg, SValue};
use r_vector::{is_na_real, na_real, Double};
use statistics_core::{descriptive, Number, StatsError};
use std::collections::HashSet;

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

    // v2 — vectorized math.
    define(env, "abs", builtin("abs", |_, a| unary_math(a, f64::abs)));
    define(env, "sqrt", builtin("sqrt", |_, a| unary_math(a, f64::sqrt)));
    define(env, "exp", builtin("exp", |_, a| unary_math(a, f64::exp)));
    define(env, "log10", builtin("log10", |_, a| unary_math(a, f64::log10)));
    define(env, "floor", builtin("floor", |_, a| unary_math(a, f64::floor)));
    define(env, "ceiling", builtin("ceiling", |_, a| unary_math(a, f64::ceil)));
    define(env, "round", builtin("round", |_, a| unary_math(a, f64::round)));
    define(env, "sin", builtin("sin", |_, a| unary_math(a, f64::sin)));
    define(env, "cos", builtin("cos", |_, a| unary_math(a, f64::cos)));
    define(env, "tan", builtin("tan", |_, a| unary_math(a, f64::tan)));
    define(env, "log", builtin("log", b_log));

    // v2 — utilities. (No underscore-named helpers like R's seq_len/seq_along:
    // in historical S `_` is assignment, so such names are not expressible —
    // `seq()` and `1:n` cover the need.)
    define(env, "rev", builtin("rev", b_rev));
    define(env, "sort", builtin("sort", b_sort));
    define(env, "order", builtin("order", b_order));
    define(env, "rep", builtin("rep", b_rep));
    define(env, "unique", builtin("unique", b_unique));
    define(env, "which", builtin("which", b_which));
    define(env, "any", builtin("any", b_any));
    define(env, "all", builtin("all", b_all));
    define(env, "is.na", builtin("is.na", b_is_na));
    define(env, "cumsum", builtin("cumsum", b_cumsum));
    define(env, "cumprod", builtin("cumprod", b_cumprod));
    define(env, "paste", builtin("paste", b_paste));
    define(env, "paste0", builtin("paste0", b_paste0));

    // v2 — apply family.
    define(env, "sapply", builtin("sapply", b_sapply));

    // v2 — S3 dispatch and output.
    define(env, "cat", builtin("cat", b_cat));
    define(env, "class", builtin("class", b_class));
    define(env, "structure", builtin("structure", b_structure));
    define(env, "inherits", builtin("inherits", b_inherits));
    define(env, "unclass", builtin("unclass", b_unclass));

    // v2 — factors.
    define(env, "factor", builtin("factor", b_factor));
    define(env, "levels", builtin("levels", b_levels));
    define(env, "nlevels", builtin("nlevels", b_nlevels));
    define(env, "as.character", builtin("as.character", b_as_character));
    define(env, "as.integer", builtin("as.integer", b_as_integer));

    // v2 — data frames.
    define(env, "data.frame", builtin("data.frame", b_data_frame));
    define(env, "nrow", builtin("nrow", b_nrow));
    define(env, "ncol", builtin("ncol", b_ncol));
    define(env, "names", builtin("names", b_names));
    define(env, "colnames", builtin("colnames", b_names));
    define(env, "dim", builtin("dim", b_dim));
    define(env, "head", builtin("head", b_head));
}

// ===========================================================================
// v2 — data frames
// ===========================================================================

/// `data.frame(name = column, …)` — build a data frame from named columns.
/// Positional columns are auto-named `V1`, `V2`, …; length-1 columns recycle to
/// the common row count, and any other length mismatch is an error.
fn b_data_frame(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let mut names = Vec::new();
    let mut columns = Vec::new();
    let mut auto = 1;
    for a in args {
        // `stringsAsFactors`-style options aren't modeled; skip non-column named
        // args that are scalars used as flags would be ambiguous, so we keep all.
        let name = a.name.clone().unwrap_or_else(|| {
            let n = format!("V{auto}");
            auto += 1;
            n
        });
        names.push(name);
        columns.push(a.value.clone());
    }

    let nrow = columns.iter().map(|c| c.length()).max().unwrap_or(0);
    for col in &mut columns {
        let len = col.length();
        if len == nrow {
            continue;
        }
        if len == 1 {
            // Recycle the single value to every row via repeated indexing.
            let idx = SValue::doubles(vec![1.0; nrow]);
            *col = index(col, &idx)?;
        } else {
            return Err(SError::BadArgs(format!(
                "arguments imply differing number of rows: {nrow}, {len}"
            )));
        }
    }

    Ok(SValue::DataFrame { names, columns })
}

/// `nrow(df)` — the row count (the common column length).
fn b_nrow(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    match first_positional(args)? {
        SValue::DataFrame { columns, .. } => {
            Ok(SValue::scalar(columns.first().map(|c| c.length()).unwrap_or(0) as f64))
        }
        _ => Ok(SValue::Null),
    }
}

/// `ncol(df)` — the column count.
fn b_ncol(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    match first_positional(args)? {
        SValue::DataFrame { columns, .. } => Ok(SValue::scalar(columns.len() as f64)),
        _ => Ok(SValue::Null),
    }
}

/// `names(df)` / `colnames(df)` — the column names.
fn b_names(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    match first_positional(args)? {
        SValue::DataFrame { names, .. } => {
            Ok(SValue::Character(names.iter().cloned().map(Some).collect()))
        }
        _ => Ok(SValue::Null),
    }
}

/// `dim(df)` — `c(nrow, ncol)`.
fn b_dim(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    match first_positional(args)? {
        SValue::DataFrame { columns, .. } => {
            let nrow = columns.first().map(|c| c.length()).unwrap_or(0) as f64;
            Ok(SValue::doubles(vec![nrow, columns.len() as f64]))
        }
        _ => Ok(SValue::Null),
    }
}

/// `head(x, n = 6)` — the first `n` elements of a vector, or rows of a data frame.
fn b_head(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let positional: Vec<&SValue> = args.iter().filter(|a| a.name.is_none()).map(|a| &a.value).collect();
    let x = *positional
        .first()
        .ok_or_else(|| SError::BadArgs("head: missing x".into()))?;
    let n = args
        .iter()
        .find(|a| a.name.as_deref() == Some("n"))
        .map(|a| &a.value)
        .or_else(|| positional.get(1).copied())
        .and_then(|v| v.as_double().ok())
        .and_then(|d| d.get_value(0))
        .map(|v| v.max(0.0) as usize)
        .unwrap_or(6);

    match x {
        SValue::DataFrame { names, columns } => {
            let nrow = columns.first().map(|c| c.length()).unwrap_or(0);
            let take = n.min(nrow);
            let rows = SValue::doubles((1..=take).map(|k| k as f64).collect());
            let new_cols: Vec<SValue> = columns
                .iter()
                .map(|c| index(c, &rows))
                .collect::<SResult<_>>()?;
            Ok(SValue::DataFrame {
                names: names.clone(),
                columns: new_cols,
            })
        }
        other => {
            let take = n.min(other.length());
            let idx = SValue::doubles((1..=take).map(|k| k as f64).collect());
            index(other, &idx)
        }
    }
}

// ===========================================================================
// v2 — factors
// ===========================================================================

/// `factor(x, levels =, labels =)` — encode `x` as a factor. Levels default to
/// the sorted unique non-`NA` values of `x`; `labels` (if given) rename them.
fn b_factor(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let values = first_positional(args)?.as_character();

    // Levels: explicit, else the sorted distinct non-NA values.
    let levels: Vec<String> = match args.iter().find(|a| a.name.as_deref() == Some("levels")) {
        Some(arg) => arg.value.as_character().into_iter().flatten().collect(),
        None => {
            let mut seen: HashSet<String> = HashSet::new();
            let mut uniq: Vec<String> = values
                .iter()
                .flatten()
                .filter(|s| seen.insert((*s).clone()))
                .cloned()
                .collect();
            uniq.sort();
            uniq
        }
    };

    // Encode each element as a 1-based code into `levels` (None = NA / unmatched).
    let position: std::collections::HashMap<&str, u32> = levels
        .iter()
        .enumerate()
        .map(|(i, s)| (s.as_str(), (i + 1) as u32))
        .collect();
    let codes: Vec<Option<u32>> = values
        .iter()
        .map(|o| o.as_ref().and_then(|s| position.get(s.as_str()).copied()))
        .collect();

    // `labels` rename the displayed levels (must match the level count).
    let display = match args.iter().find(|a| a.name.as_deref() == Some("labels")) {
        Some(arg) => arg.value.as_character().into_iter().flatten().collect(),
        None => levels,
    };

    Ok(SValue::Factor {
        codes,
        levels: display,
    })
}

/// `levels(f)` — the level labels of a factor (`NULL` otherwise).
fn b_levels(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    match first_positional(args)? {
        SValue::Factor { levels, .. } => {
            Ok(SValue::Character(levels.iter().cloned().map(Some).collect()))
        }
        _ => Ok(SValue::Null),
    }
}

/// `nlevels(f)` — the number of levels.
fn b_nlevels(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let n = match first_positional(args)? {
        SValue::Factor { levels, .. } => levels.len(),
        _ => 0,
    };
    Ok(SValue::scalar(n as f64))
}

/// `as.character(x)` — the character form (factor → its labels).
fn b_as_character(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    Ok(SValue::Character(first_positional(args)?.as_character()))
}

/// `as.integer(x)` — factor codes, or numerics truncated toward zero.
fn b_as_integer(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    match first_positional(args)? {
        SValue::Factor { codes, .. } => Ok(SValue::doubles(
            codes
                .iter()
                .map(|c| c.map(|k| k as f64).unwrap_or_else(na_real))
                .collect(),
        )),
        other => {
            let d = other.as_double()?;
            Ok(SValue::doubles(
                d.iter()
                    .map(|x| if is_na_real(x) { na_real() } else { x.trunc() })
                    .collect(),
            ))
        }
    }
}

// ===========================================================================
// v2 — S3 dispatch helpers
// ===========================================================================

/// `class(x)` — the class vector (explicit if set, else the implicit type).
fn b_class(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let classes = class_of(first_positional(args)?);
    Ok(SValue::Character(classes.into_iter().map(Some).collect()))
}

/// `structure(x, class = …)` — attach an explicit S3 class to a value. v2
/// supports the `class` attribute; other attributes are accepted but ignored.
fn b_structure(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let inner = first_positional(args)?.clone();
    match args.iter().find(|a| a.name.as_deref() == Some("class")) {
        Some(arg) => {
            let class: Vec<String> = arg
                .value
                .as_character()
                .into_iter()
                .flatten()
                .collect();
            Ok(SValue::Classed {
                inner: Box::new(inner),
                class,
            })
        }
        None => Ok(inner),
    }
}

/// `inherits(x, what)` — whether any class of `x` matches `what`.
fn b_inherits(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let positional: Vec<&SValue> = args.iter().filter(|a| a.name.is_none()).map(|a| &a.value).collect();
    let classes: HashSet<String> = class_of(
        positional
            .first()
            .ok_or_else(|| SError::BadArgs("inherits: missing x".into()))?,
    )
    .into_iter()
    .collect();
    let what: Vec<Option<String>> = positional
        .get(1)
        .map(|v| v.as_character())
        .unwrap_or_default();
    let hit = what
        .into_iter()
        .flatten()
        .any(|w| classes.contains(&w));
    Ok(SValue::Logical(vec![Some(hit)]))
}

/// `unclass(x)` — drop an explicit S3 class, returning the underlying value.
fn b_unclass(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    match first_positional(args)? {
        SValue::Classed { inner, .. } => Ok((**inner).clone()),
        other => Ok(other.clone()),
    }
}

// ===========================================================================
// v2 — vectorized math
// ===========================================================================

/// Map a scalar function elementwise over the first positional argument,
/// preserving `NA`.
fn unary_math(args: &[Arg], f: impl Fn(f64) -> f64) -> SResult<SValue> {
    let d = first_positional(args)?.as_double()?;
    Ok(SValue::doubles(
        d.iter()
            .map(|x| if is_na_real(x) { na_real() } else { f(x) })
            .collect(),
    ))
}

/// `log(x)` is the natural log; `log(x, base)` (positional or `base =`) uses the
/// given base.
fn b_log(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let positional: Vec<&SValue> = args.iter().filter(|a| a.name.is_none()).map(|a| &a.value).collect();
    let x = positional
        .first()
        .ok_or_else(|| SError::BadArgs("log: missing argument".into()))?
        .as_double()?;
    let base = args
        .iter()
        .find(|a| a.name.as_deref() == Some("base"))
        .map(|a| &a.value)
        .or_else(|| positional.get(1).copied());
    let mapper: Box<dyn Fn(f64) -> f64> = match base {
        Some(b) => {
            let bv = b.as_double()?.get_value(0).unwrap_or(std::f64::consts::E);
            Box::new(move |x: f64| x.log(bv))
        }
        None => Box::new(|x: f64| x.ln()),
    };
    Ok(SValue::doubles(
        x.iter()
            .map(|v| if is_na_real(v) { na_real() } else { mapper(v) })
            .collect(),
    ))
}

// ===========================================================================
// v2 — utilities
// ===========================================================================

/// `rev(x)` reverses any vector (reuses indexing, so the element type survives).
fn b_rev(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let v = first_positional(args)?;
    let n = v.length();
    let idx = SValue::doubles((0..n).rev().map(|k| (k + 1) as f64).collect());
    index(v, &idx)
}

/// `sort(x)` — ascending order, dropping `NA`. Numeric and character supported.
fn b_sort(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    match first_positional(args)? {
        SValue::Character(v) => {
            let mut items: Vec<String> = v.iter().flatten().cloned().collect();
            items.sort();
            Ok(SValue::Character(items.into_iter().map(Some).collect()))
        }
        other => {
            let d = other.as_double()?;
            let mut items: Vec<f64> = d.iter().filter(|x| !is_na_real(*x)).collect();
            items.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            Ok(SValue::doubles(items))
        }
    }
}

/// `order(x)` — the 1-based permutation that sorts `x` ascending.
fn b_order(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let d = first_positional(args)?.as_double()?;
    let data = d.data();
    let mut idx: Vec<usize> = (0..data.len()).collect();
    idx.sort_by(|&a, &b| data[a].partial_cmp(&data[b]).unwrap_or(std::cmp::Ordering::Equal));
    Ok(SValue::doubles(idx.iter().map(|i| (i + 1) as f64).collect()))
}

/// `rep(x, times)` — concatenate `times` copies of `x`.
fn b_rep(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let x = first_positional(args)?.clone();
    let times = args
        .iter()
        .find(|a| a.name.as_deref() == Some("times"))
        .map(|a| &a.value)
        .or_else(|| args.iter().filter(|a| a.name.is_none()).nth(1).map(|a| &a.value))
        .and_then(|v| v.as_double().ok())
        .and_then(|d| d.get_value(0))
        .map(|n| n.max(0.0) as usize)
        .unwrap_or(1);
    let copies: Vec<Arg> = (0..times)
        .map(|_| Arg { name: None, value: x.clone() })
        .collect();
    Ok(combine(&copies))
}

/// `unique(x)` — distinct elements, first occurrence order preserved.
fn b_unique(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let v = first_positional(args)?;
    let keys = v.as_character();
    let mut seen: HashSet<Option<String>> = HashSet::new();
    let keep: Vec<f64> = keys
        .iter()
        .enumerate()
        .filter_map(|(i, k)| seen.insert(k.clone()).then_some((i + 1) as f64))
        .collect();
    index(v, &SValue::doubles(keep))
}

/// `which(x)` — the 1-based indices where logical `x` is `TRUE`.
fn b_which(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let bits = first_positional(args)?.as_logical()?;
    Ok(SValue::doubles(
        bits.iter()
            .enumerate()
            .filter_map(|(i, b)| (*b == Some(true)).then_some((i + 1) as f64))
            .collect(),
    ))
}

/// `any(...)` — `TRUE` if any element is `TRUE`, tri-valued with `NA`.
fn b_any(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let bits = combined_logical(args)?;
    let result = if bits.iter().any(|b| *b == Some(true)) {
        Some(true)
    } else if bits.iter().any(|b| b.is_none()) {
        None
    } else {
        Some(false)
    };
    Ok(SValue::Logical(vec![result]))
}

/// `all(...)` — `TRUE` if every element is `TRUE`, tri-valued with `NA`.
fn b_all(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let bits = combined_logical(args)?;
    let result = if bits.iter().any(|b| *b == Some(false)) {
        Some(false)
    } else if bits.iter().any(|b| b.is_none()) {
        None
    } else {
        Some(true)
    };
    Ok(SValue::Logical(vec![result]))
}

/// `is.na(x)` — a logical vector, `TRUE` where `x` is `NA`.
fn b_is_na(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let v = first_positional(args)?;
    let flags: Vec<Option<bool>> = match v {
        SValue::Double(d) => d.iter().map(|x| Some(is_na_real(x))).collect(),
        SValue::Logical(l) => l.iter().map(|o| Some(o.is_none())).collect(),
        SValue::Character(c) => c.iter().map(|o| Some(o.is_none())).collect(),
        SValue::Factor { codes, .. } => codes.iter().map(|c| Some(c.is_none())).collect(),
        other => other.as_character().iter().map(|o| Some(o.is_none())).collect(),
    };
    Ok(SValue::Logical(flags))
}

/// `cumsum(x)` — running totals (delegates to statistics-core).
fn b_cumsum(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    Ok(SValue::Double(descriptive::cumsum(
        &first_positional(args)?.as_double()?,
    )))
}

/// `cumprod(x)` — running products.
fn b_cumprod(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    Ok(SValue::Double(descriptive::cumprod(
        &first_positional(args)?.as_double()?,
    )))
}

/// `paste(..., sep = " ")` — element-wise string join across arguments, with
/// recycling. `paste0` is `paste` with no separator.
fn b_paste(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    paste_impl(args, " ")
}
fn b_paste0(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    paste_impl(args, "")
}

fn paste_impl(args: &[Arg], default_sep: &str) -> SResult<SValue> {
    let sep = args
        .iter()
        .find(|a| a.name.as_deref() == Some("sep"))
        .and_then(|a| a.value.as_character().into_iter().next().flatten())
        .unwrap_or_else(|| default_sep.to_string());

    // Each positional argument becomes a character column; empty ones are
    // ignored (matching R).
    let cols: Vec<Vec<Option<String>>> = args
        .iter()
        .filter(|a| a.name.is_none())
        .map(|a| a.value.as_character())
        .filter(|c| !c.is_empty())
        .collect();
    if cols.is_empty() {
        return Ok(SValue::Character(vec![]));
    }
    let n = cols.iter().map(|c| c.len()).max().unwrap_or(0);
    let out: Vec<Option<String>> = (0..n)
        .map(|i| {
            let parts: Vec<String> = cols
                .iter()
                .map(|c| c[i % c.len()].clone().unwrap_or_else(|| "NA".to_string()))
                .collect();
            Some(parts.join(&sep))
        })
        .collect();
    Ok(SValue::Character(out))
}

/// Combine the logical coercions of all positional arguments into one vector.
fn combined_logical(args: &[Arg]) -> SResult<Vec<Option<bool>>> {
    let mut bits = Vec::new();
    for a in args.iter().filter(|a| a.name.is_none()) {
        bits.extend(a.value.as_logical()?);
    }
    Ok(bits)
}

// ===========================================================================
// v2 — apply family
// ===========================================================================

/// `sapply(x, f)` — apply `f` to each element of `x` and simplify the results
/// into a vector (length-1 atomic results combine into one vector). `lapply`
/// (which must return a list) is deferred until S grows a list type.
fn b_sapply(interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let positional: Vec<&Arg> = args.iter().filter(|a| a.name.is_none()).collect();
    let x = positional
        .first()
        .ok_or_else(|| SError::BadArgs("sapply: missing X".into()))?
        .value
        .clone();
    let f = positional
        .get(1)
        .ok_or_else(|| SError::BadArgs("sapply: missing FUN".into()))?
        .value
        .clone();
    if !f.is_callable() {
        return Err(SError::NotCallable(f.type_name().to_string()));
    }
    let mut results: Vec<Arg> = Vec::with_capacity(x.length());
    for i in 0..x.length() {
        let elem = nth_element(&x, i);
        let r = interp.call_value(f.clone(), &[Arg { name: None, value: elem }])?;
        results.push(Arg { name: None, value: r });
    }
    Ok(combine(&results))
}

fn builtin(name: &str, func: fn(&Interpreter, &[Arg]) -> SResult<SValue>) -> SValue {
    SValue::Builtin {
        name: name.to_string(),
        func,
    }
}

// ===========================================================================
// Core built-ins
// ===========================================================================

/// `c(...)` — combine arguments into one vector (coercing to the highest type).
fn b_c(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    Ok(combine(args))
}

/// `length(x)` — the element count, as a length-1 numeric vector.
fn b_length(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let v = first_positional(args)?;
    Ok(SValue::scalar(v.length() as f64))
}

/// `print(x)` — the S3 generic. Dispatch (to a `print.<class>` method, else the
/// default formatting) is handled by the evaluator; this just forwards the
/// argument to it.
fn b_print(interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let value = args
        .iter()
        .find(|a| a.name.is_none())
        .map(|a| a.value.clone())
        .unwrap_or(SValue::Null);
    interp.dispatch_print(&value)
}

/// `cat(..., sep = " ")` — write the arguments to the console with no quoting
/// and no trailing newline (the caller includes `\n`). Returns `NULL`.
fn b_cat(interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let sep = args
        .iter()
        .find(|a| a.name.as_deref() == Some("sep"))
        .and_then(|a| a.value.as_character().into_iter().next().flatten())
        .unwrap_or_else(|| " ".to_string());
    let parts: Vec<String> = args
        .iter()
        .filter(|a| a.name.is_none())
        .flat_map(|a| a.value.as_character())
        .map(|o| o.unwrap_or_else(|| "NA".to_string()))
        .collect();
    interp.emit_raw(&parts.join(&sep));
    Ok(SValue::Null)
}

/// `seq(to)` is `1:to`; `seq(from, to)` is `from:to` (step 1). A minimal subset
/// of R's `seq` sufficient for v1.
fn b_seq(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
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

fn b_mean(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    reduce_single(args, "mean", descriptive::mean)
}
fn b_median(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    reduce_single(args, "median", descriptive::median)
}
fn b_var(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    reduce_single(args, "var", descriptive::var)
}
fn b_sd(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    reduce_single(args, "sd", descriptive::sd)
}
fn b_sum(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    reduce_variadic(args, "sum", descriptive::sum)
}
fn b_prod(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    reduce_variadic(args, "prod", descriptive::prod)
}
fn b_min(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    reduce_variadic(args, "min", descriptive::min)
}
fn b_max(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
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
