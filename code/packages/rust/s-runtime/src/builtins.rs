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
use crate::value::{
    bounded_sequence, class_of, combine, index, Arg, SValue, MAX_ATTRIBUTES, MAX_SEQ_LEN,
};
use r_vector::{is_na_real, na_real, Double};
use statistics_core::distributions::{dnorm, pnorm, qnorm, rnorm};
use statistics_core::distributions_more::{
    dbinom, dexp, dpois, dunif, pbinom, pexp, ppois, punif, qbinom, qexp, qpois, qunif, rbinom,
    rexp, rpois, runif,
};
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
    define(
        env,
        "sqrt",
        builtin("sqrt", |_, a| unary_math(a, f64::sqrt)),
    );
    define(env, "exp", builtin("exp", |_, a| unary_math(a, f64::exp)));
    define(
        env,
        "log10",
        builtin("log10", |_, a| unary_math(a, f64::log10)),
    );
    define(
        env,
        "floor",
        builtin("floor", |_, a| unary_math(a, f64::floor)),
    );
    define(
        env,
        "ceiling",
        builtin("ceiling", |_, a| unary_math(a, f64::ceil)),
    );
    define(
        env,
        "round",
        builtin("round", |_, a| unary_math(a, f64::round)),
    );
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

    // Lists (R-6).
    define(env, "list", builtin("list", b_list));
    define(env, "lapply", builtin("lapply", b_lapply));
    define(env, "strsplit", builtin("strsplit", b_strsplit));

    // Higher-order functionals (R-10) — pair with the R-9 `\(x)` lambdas.
    define(env, "Map", builtin("Map", b_map));
    define(env, "Reduce", builtin("Reduce", b_reduce));
    define(env, "Filter", builtin("Filter", b_filter));
    define(env, "mapply", builtin("mapply", b_mapply));
    define(env, "vapply", builtin("vapply", b_vapply));

    // Regular expressions (R-7).
    define(env, "grepl", builtin("grepl", b_grepl));
    define(env, "grep", builtin("grep", b_grep));
    define(env, "gsub", builtin("gsub", b_gsub));
    define(env, "sub", builtin("sub", b_sub));

    // Distribution family (R-8): density (d*), distribution/CDF (p*),
    // quantile (q*), and random sampling (r*), wired to statistics-core.
    define(env, "set.seed", builtin("set.seed", b_set_seed));
    define(env, "dnorm", builtin("dnorm", b_dnorm));
    define(env, "pnorm", builtin("pnorm", b_pnorm));
    define(env, "qnorm", builtin("qnorm", b_qnorm));
    define(env, "rnorm", builtin("rnorm", b_rnorm));
    define(env, "dunif", builtin("dunif", b_dunif));
    define(env, "punif", builtin("punif", b_punif));
    define(env, "qunif", builtin("qunif", b_qunif));
    define(env, "runif", builtin("runif", b_runif));
    define(env, "dexp", builtin("dexp", b_dexp));
    define(env, "pexp", builtin("pexp", b_pexp));
    define(env, "qexp", builtin("qexp", b_qexp));
    define(env, "rexp", builtin("rexp", b_rexp));

    // Discrete distribution family (R-8b): binomial and Poisson.
    define(env, "dbinom", builtin("dbinom", b_dbinom));
    define(env, "pbinom", builtin("pbinom", b_pbinom));
    define(env, "qbinom", builtin("qbinom", b_qbinom));
    define(env, "rbinom", builtin("rbinom", b_rbinom));
    define(env, "dpois", builtin("dpois", b_dpois));
    define(env, "ppois", builtin("ppois", b_ppois));
    define(env, "qpois", builtin("qpois", b_qpois));
    define(env, "rpois", builtin("rpois", b_rpois));

    // String manipulation (vectorized over a character vector).
    define(env, "nchar", builtin("nchar", b_nchar));
    define(env, "toupper", builtin("toupper", b_toupper));
    define(env, "tolower", builtin("tolower", b_tolower));
    define(env, "substr", builtin("substr", b_substr));
    define(env, "sprintf", builtin("sprintf", b_sprintf));

    // v2 — apply family.
    define(env, "sapply", builtin("sapply", b_sapply));

    // v2 — S3 dispatch and output.
    define(env, "cat", builtin("cat", b_cat));
    define(env, "class", builtin("class", b_class));
    define(env, "structure", builtin("structure", b_structure));
    define(env, "inherits", builtin("inherits", b_inherits));
    define(env, "unclass", builtin("unclass", b_unclass));

    // R-16 — general attributes. `attr<-` / `attributes<-` slot into the
    // replacement-function lvalue path R-15 added (`f(x) <- v` ≡ `x <- \`f<-\`(x, v)`).
    define(env, "attr", builtin("attr", b_attr));
    define(env, "attr<-", builtin("attr<-", b_attr_replace));
    define(env, "attributes", builtin("attributes", b_attributes));
    define(
        env,
        "attributes<-",
        builtin("attributes<-", b_attributes_replace),
    );

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
    // The `names(x) <- value` replacement function and its functional form.
    define(env, "names<-", builtin("names<-", b_set_names_replace));
    define(env, "setNames", builtin("setNames", b_set_names));
    define(env, "dim", builtin("dim", b_dim));
    define(env, "head", builtin("head", b_head));

    // Matrices (R-11). `%*%` lives in the evaluator's infix dispatch.
    define(env, "matrix", builtin("matrix", b_matrix));
    define(env, "t", builtin("t", b_t));
    define(env, "apply", builtin("apply", b_apply));

    // Matrix linear algebra (R-12).
    define(env, "diag", builtin("diag", b_diag));
    define(env, "rowSums", builtin("rowSums", b_row_sums));
    define(env, "colSums", builtin("colSums", b_col_sums));
    define(env, "rowMeans", builtin("rowMeans", b_row_means));
    define(env, "colMeans", builtin("colMeans", b_col_means));
    define(env, "cbind", builtin("cbind", b_cbind));
    define(env, "rbind", builtin("rbind", b_rbind));
    define(env, "solve", builtin("solve", b_solve));
    define(env, "det", builtin("det", b_det));
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
    match peel_structural(first_positional(args)?) {
        SValue::DataFrame { columns, .. } => Ok(SValue::scalar(
            columns.first().map(|c| c.length()).unwrap_or(0) as f64,
        )),
        SValue::Matrix { nrow, .. } => Ok(SValue::scalar(*nrow as f64)),
        _ => Ok(SValue::Null),
    }
}

/// `ncol(df)` — the column count.
fn b_ncol(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    match peel_structural(first_positional(args)?) {
        SValue::DataFrame { columns, .. } => Ok(SValue::scalar(columns.len() as f64)),
        SValue::Matrix { ncol, .. } => Ok(SValue::scalar(*ncol as f64)),
        _ => Ok(SValue::Null),
    }
}

/// `names(x)` / `colnames(df)` — the names of `x`. For a data frame these are the
/// column names; for a **named vector** (R-15) they are the element names (an
/// unset name → `NA`). Anything without names is `NULL`.
fn b_names(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    // Peel class/general wrappers (but not the names wrapper itself) so a classed
    // or generally-attributed named vector still reports its names.
    match peel_to_named(first_positional(args)?) {
        SValue::DataFrame { names, .. } => {
            Ok(SValue::Character(names.iter().cloned().map(Some).collect()))
        }
        SValue::Named { names, .. } => Ok(SValue::Character(names.clone())),
        _ => Ok(SValue::Null),
    }
}

/// `names<-`(x, value)` — the replacement form behind `names(x) <- value`
/// (R-15). Coerces `value` to character and attaches it as `x`'s names, R-style:
/// a too-short names vector pads the tail with `NA`, a too-long one is an error,
/// and `value = NULL` drops the names entirely. Names attach only to atomic
/// vectors; on any other value the names are silently ignored (R errors, but
/// this subset keeps it lenient and returns the value unchanged).
fn b_set_names_replace(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    // The replacement convention passes (x, value): x is positional[0], the new
    // names are the `value =` named arg (or positional[1]).
    let x = first_positional(args)?.clone();
    let value = args
        .iter()
        .find(|a| a.name.as_deref() == Some("value"))
        .map(|a| &a.value)
        .or_else(|| nth_positional(args, 1));

    match value {
        // `names(x) <- NULL` clears the names.
        None | Some(SValue::Null) => Ok(x.strip_names().clone()),
        Some(v) => {
            let new_names = v.as_character();
            // Reject a names vector longer than the value (R's
            // "'names' attribute must be the same length as the vector" — except
            // R actually allows shorter with NA-pad; longer is the error).
            if new_names.len() > x.length() {
                return Err(SError::BadArgs(format!(
                    "'names' attribute [{}] must be no longer than the vector [{}]",
                    new_names.len(),
                    x.length()
                )));
            }
            Ok(SValue::with_names(x, new_names))
        }
    }
}

/// `setNames(x, nm)` — the functional form of `names(x) <- nm`: return `x` with
/// its names set to `nm` (NA-padded / cleared by `NULL`, as `names<-`).
fn b_set_names(interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    // Reuse the replacement engine, mapping the second positional to `value`.
    let x = first_positional(args)?.clone();
    let nm = nth_positional(args, 1).cloned().unwrap_or(SValue::Null);
    b_set_names_replace(
        interp,
        &[
            Arg {
                name: None,
                value: x,
            },
            Arg {
                name: Some("value".to_string()),
                value: nm,
            },
        ],
    )
}

/// `dim(df)` — `c(nrow, ncol)`.
fn b_dim(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    match peel_structural(first_positional(args)?) {
        SValue::DataFrame { columns, .. } => {
            let nrow = columns.first().map(|c| c.length()).unwrap_or(0) as f64;
            Ok(SValue::doubles(vec![nrow, columns.len() as f64]))
        }
        SValue::Matrix { nrow, ncol, .. } => Ok(SValue::doubles(vec![*nrow as f64, *ncol as f64])),
        _ => Ok(SValue::Null),
    }
}

/// `head(x, n = 6)` — the first `n` elements of a vector, or rows of a data frame.
fn b_head(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let positional: Vec<&SValue> = args
        .iter()
        .filter(|a| a.name.is_none())
        .map(|a| &a.value)
        .collect();
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
// Matrices (R-11)
// ===========================================================================

/// Read a positive-integer `matrix`/`apply` dimension argument, by name or
/// position.
fn dim_arg(args: &[Arg], name: &str, pos: usize) -> Option<usize> {
    args.iter()
        .find(|a| a.name.as_deref() == Some(name))
        .map(|a| &a.value)
        .or_else(|| nth_positional(args, pos))
        .and_then(|v| v.as_double().ok())
        .and_then(|d| d.get_value(0))
        .filter(|x| x.is_finite() && *x >= 1.0)
        .map(|x| x as usize)
}

/// `matrix(data, nrow =, ncol =, byrow = FALSE)` — lay `data` (recycled) into a
/// matrix. Column-major by default; `byrow = TRUE` fills row by row. With only
/// one of `nrow`/`ncol`, the other is derived from the data length.
fn b_matrix(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let data = first_positional(args)?.as_double()?;
    let n = data.len();
    let nrow_a = dim_arg(args, "nrow", 1);
    let ncol_a = dim_arg(args, "ncol", 2);
    let byrow = args
        .iter()
        .find(|a| a.name.as_deref() == Some("byrow"))
        .map(|a| a.value.truthy().unwrap_or(false))
        .unwrap_or(false);

    let ceil_div = |a: usize, b: usize| a.div_ceil(b.max(1));
    let (nrow, ncol) = match (nrow_a, ncol_a) {
        (Some(r), Some(c)) => (r, c),
        (Some(r), None) => (r, ceil_div(n, r).max(1)),
        (None, Some(c)) => (ceil_div(n, c).max(1), c),
        (None, None) => (n.max(1), 1), // a bare column
    };
    let total = nrow
        .checked_mul(ncol)
        .filter(|&t| t <= MAX_SEQ_LEN)
        .ok_or_else(|| SError::Index(format!("matrix too large (limit {MAX_SEQ_LEN} elements)")))?;

    let src = data.data();
    let at = |i: usize| if n == 0 { na_real() } else { src[i % n] };
    let mut out = vec![0.0; total];
    if byrow {
        for r in 0..nrow {
            for c in 0..ncol {
                out[c * nrow + r] = at(r * ncol + c); // read row-major, store column-major
            }
        }
    } else {
        for (i, slot) in out.iter_mut().enumerate() {
            *slot = at(i);
        }
    }
    Ok(SValue::Matrix {
        data: Double::from_values(out),
        nrow,
        ncol,
    })
}

/// `t(x)` — the transpose of a matrix; a bare vector becomes a `1×n` row.
fn b_t(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    match first_positional(args)? {
        SValue::Matrix { data, nrow, ncol } => {
            let (nr, nc) = (*nrow, *ncol);
            let s = data.data();
            let mut out = vec![0.0; nr * nc];
            for r in 0..nr {
                for c in 0..nc {
                    out[r * nc + c] = s[c * nr + r]; // result (c, r) ← original (r, c)
                }
            }
            Ok(SValue::Matrix {
                data: Double::from_values(out),
                nrow: nc,
                ncol: nr,
            })
        }
        other => {
            let d = other.as_double()?;
            let n = d.len();
            Ok(SValue::Matrix {
                data: d,
                nrow: 1,
                ncol: n,
            })
        }
    }
}

/// `apply(X, MARGIN, FUN, …)` — apply `FUN` to each row (`MARGIN = 1`) or column
/// (`MARGIN = 2`) of a matrix `X`. Simplifies to a vector when every result is a
/// scalar, to a matrix (one column per margin) when they share a length, else a
/// list. Trailing arguments are passed to `FUN`.
fn b_apply(interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let positional: Vec<&Arg> = args.iter().filter(|a| a.name.is_none()).collect();
    let x = positional
        .first()
        .ok_or_else(|| SError::BadArgs("apply: missing X".into()))?
        .value
        .clone();
    let margin = positional
        .get(1)
        .and_then(|a| a.value.as_double().ok())
        .and_then(|d| d.get_value(0))
        .map(|m| m as i64)
        .filter(|&m| m == 1 || m == 2)
        .ok_or_else(|| SError::BadArgs("apply: MARGIN must be 1 (rows) or 2 (columns)".into()))?;
    let f = positional
        .get(2)
        .ok_or_else(|| SError::BadArgs("apply: missing FUN".into()))?
        .value
        .clone();
    if !f.is_callable() {
        return Err(SError::NotCallable(f.type_name().to_string()));
    }
    let extra: Vec<Arg> = positional
        .iter()
        .skip(3)
        .map(|a| Arg {
            name: None,
            value: a.value.clone(),
        })
        .collect();

    let (data, nrow, ncol) = match &x {
        SValue::Matrix { data, nrow, ncol } => (data.clone(), *nrow, *ncol),
        other => {
            return Err(SError::BadArgs(format!(
                "apply: X must be a matrix (got {})",
                other.type_name()
            )))
        }
    };
    let s = data.data();
    let count = if margin == 1 { nrow } else { ncol };
    let mut results = Vec::with_capacity(count);
    for k in 0..count {
        let slice: Vec<f64> = if margin == 1 {
            (0..ncol).map(|c| s[c * nrow + k]).collect() // row k
        } else {
            (0..nrow).map(|r| s[k * nrow + r]).collect() // column k
        };
        let mut call_args = Vec::with_capacity(1 + extra.len());
        call_args.push(Arg {
            name: None,
            value: SValue::doubles(slice),
        });
        call_args.extend(extra.iter().cloned());
        results.push(interp.call_value(f.clone(), &call_args)?);
    }

    if results.iter().all(|r| r.length() == 1) {
        let wrapped: Vec<Arg> = results
            .into_iter()
            .map(|value| Arg { name: None, value })
            .collect();
        Ok(combine(&wrapped))
    } else if !results.is_empty() && results.iter().all(|r| r.length() == results[0].length()) {
        let rlen = results[0].length();
        let mut out = Vec::with_capacity(rlen * count);
        for r in &results {
            out.extend_from_slice(r.as_double()?.data());
        }
        Ok(SValue::Matrix {
            data: Double::from_values(out),
            nrow: rlen,
            ncol: count,
        })
    } else {
        Ok(SValue::List {
            names: vec![None; results.len()],
            items: results,
        })
    }
}

// ===========================================================================
// Matrix linear algebra (R-12)
// ===========================================================================

/// Largest square dimension `solve`/`det` will factor. Their Gaussian
/// elimination is `O(n³)`, so without a cap a (still `MAX_SEQ_LEN`-legal)
/// 4000×4000 matrix would be ~10¹¹ flops — a denial-of-service. 1000 keeps the
/// work near a billion flops (sub-second) while covering any realistic teaching
/// or interactive use.
const MAX_SOLVE_DIM: usize = 1000;

/// Pull the `(data, nrow, ncol)` out of a `SValue::Matrix`, or `None` for any
/// other value. Borrows, so callers clone only when they must.
fn matrix_parts(value: &SValue) -> Option<(&Double, usize, usize)> {
    match value {
        SValue::Matrix { data, nrow, ncol } => Some((data, *nrow, *ncol)),
        _ => None,
    }
}

/// `diag(x)` — R's three-way overload:
/// * `x` a **matrix** → its diagonal, as a vector of length `min(nrow, ncol)`.
/// * `x` a length-`> 1` **vector** → the square matrix with `x` on the diagonal.
/// * `x` a single **number** `n` → the `n × n` identity matrix.
///
/// For the vector / identity forms, `nrow`/`ncol` (by name or position) override
/// the derived shape, with the diagonal value(s) recycled.
fn b_diag(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let x = first_positional(args)?;

    // Matrix → extract the diagonal.
    if let Some((data, nrow, ncol)) = matrix_parts(x) {
        let k = nrow.min(ncol);
        let mut out = Vec::with_capacity(k);
        for i in 0..k {
            out.push(data.get_value(i * nrow + i).unwrap_or_else(na_real));
        }
        return Ok(SValue::doubles(out));
    }

    let d = x.as_double()?;
    let nrow_a = dim_arg(args, "nrow", 1);
    let ncol_a = dim_arg(args, "ncol", 2);

    // A single number with no explicit shape → identity of that order.
    if d.len() == 1 && nrow_a.is_none() && ncol_a.is_none() {
        let v = d.get_value(0).unwrap_or_else(na_real);
        if !v.is_finite() || v < 0.0 {
            return Err(SError::BadArgs(
                "diag: the dimension must be a finite, non-negative number".into(),
            ));
        }
        let n = v as usize;
        return identity_matrix(n);
    }

    // A vector → a diagonal matrix. The shape is `nrow × ncol`, defaulting to a
    // square of the vector's length; diagonal entries are recycled from `d`.
    let len = d.len();
    let nrow = nrow_a.unwrap_or(len);
    let ncol = ncol_a.unwrap_or(nrow);
    let total = nrow
        .checked_mul(ncol)
        .filter(|&t| t <= MAX_SEQ_LEN)
        .ok_or_else(|| SError::Index(format!("diag: matrix too large (limit {MAX_SEQ_LEN})")))?;
    let src = d.data();
    let mut out = vec![0.0; total];
    let k = nrow.min(ncol);
    for i in 0..k {
        // Recycle the diagonal values; an empty `d` leaves zeros (R uses NA, but
        // `diag(numeric(0))` is a degenerate case — keep it simple and safe).
        out[i * nrow + i] = if len == 0 { na_real() } else { src[i % len] };
    }
    Ok(SValue::Matrix {
        data: Double::from_values(out),
        nrow,
        ncol,
    })
}

/// Build the `n × n` identity matrix, bounded by `MAX_SEQ_LEN`.
fn identity_matrix(n: usize) -> SResult<SValue> {
    let total = n
        .checked_mul(n)
        .filter(|&t| t <= MAX_SEQ_LEN)
        .ok_or_else(|| SError::Index(format!("diag: matrix too large (limit {MAX_SEQ_LEN})")))?;
    let mut out = vec![0.0; total];
    for i in 0..n {
        out[i * n + i] = 1.0;
    }
    Ok(SValue::Matrix {
        data: Double::from_values(out),
        nrow: n,
        ncol: n,
    })
}

/// Shared engine for the four margin reductions. `by_row` selects rows vs
/// columns; `mean` divides by the (non-`NA`, when `na.rm`) count. Reads an
/// `na.rm` named argument (default `FALSE`).
fn margin_reduce(args: &[Arg], by_row: bool, mean: bool) -> SResult<SValue> {
    let x = first_positional(args)?;
    let (data, nrow, ncol) = matrix_parts(x).ok_or_else(|| {
        SError::TypeError(format!("'x' must be a matrix (got {})", x.type_name()))
    })?;
    let na_rm = args
        .iter()
        .find(|a| a.name.as_deref() == Some("na.rm"))
        .map(|a| a.value.truthy().unwrap_or(false))
        .unwrap_or(false);

    let s = data.data();
    let count = if by_row { nrow } else { ncol };
    let span = if by_row { ncol } else { nrow };
    let mut out = vec![0.0; count];
    for (k, slot) in out.iter_mut().enumerate() {
        let mut acc = 0.0;
        let mut n_used = 0usize;
        let mut saw_na = false;
        for j in 0..span {
            // Row k: element (k, j) at j*nrow + k. Column k: (j, k) at k*nrow + j.
            let v = if by_row {
                s[j * nrow + k]
            } else {
                s[k * nrow + j]
            };
            if is_na_real(v) {
                if na_rm {
                    continue;
                }
                saw_na = true;
                break;
            }
            acc += v;
            n_used += 1;
        }
        *slot = if saw_na {
            na_real()
        } else if mean {
            if n_used == 0 {
                f64::NAN // mean of nothing — matches R's NaN
            } else {
                acc / n_used as f64
            }
        } else {
            acc
        };
    }
    Ok(SValue::doubles(out))
}

/// `rowSums(x)` — the sum of each row of matrix `x` (with `na.rm`).
fn b_row_sums(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    margin_reduce(args, true, false)
}

/// `colSums(x)` — the sum of each column.
fn b_col_sums(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    margin_reduce(args, false, false)
}

/// `rowMeans(x)` — the mean of each row.
fn b_row_means(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    margin_reduce(args, true, true)
}

/// `colMeans(x)` — the mean of each column.
fn b_col_means(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    margin_reduce(args, false, true)
}

/// One column-bind/row-bind source: its column-major data and shape. A bare
/// vector is carried as an `n × 1` (cbind) or `1 × n` (rbind) block by the
/// caller's interpretation of `is_matrix`.
struct BindSource {
    data: Double,
    nrow: usize,
    ncol: usize,
    is_matrix: bool,
}

/// Collect the positional arguments of `cbind`/`rbind` as bind sources: each is
/// either a real matrix or a vector (carried as length × 1, flagged
/// `is_matrix = false` so the binder knows it may be recycled).
fn bind_sources(args: &[Arg]) -> SResult<Vec<BindSource>> {
    let mut sources = Vec::new();
    for arg in args.iter().filter(|a| a.name.is_none()) {
        if let Some((data, nrow, ncol)) = matrix_parts(&arg.value) {
            sources.push(BindSource {
                data: data.clone(),
                nrow,
                ncol,
                is_matrix: true,
            });
        } else if matches!(arg.value, SValue::Null) {
            continue; // NULL contributes nothing, as in R
        } else {
            let d = arg.value.as_double()?;
            let n = d.len();
            sources.push(BindSource {
                data: d,
                nrow: n,
                ncol: 1,
                is_matrix: false,
            });
        }
    }
    Ok(sources)
}

/// `cbind(…)` — bind vectors and matrices as columns. The common row count is
/// the largest source height; shorter **vectors** are recycled, but a **matrix**
/// whose row count differs is an error. The all-empty call is `NULL`.
fn b_cbind(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let sources = bind_sources(args)?;
    if sources.is_empty() {
        return Ok(SValue::Null);
    }
    let nrow = sources.iter().map(|s| s.nrow).max().unwrap_or(0);
    let ncol: usize = sources.iter().map(|s| s.ncol).sum();
    let total = nrow
        .checked_mul(ncol)
        .filter(|&t| t <= MAX_SEQ_LEN)
        .ok_or_else(|| SError::Index(format!("cbind: result too large (limit {MAX_SEQ_LEN})")))?;
    let mut out = vec![0.0; total];
    let mut col = 0usize;
    for src in &sources {
        if src.is_matrix && src.nrow != nrow {
            return Err(SError::BadArgs(format!(
                "cbind: number of rows of matrices must match ({} != {nrow})",
                src.nrow
            )));
        }
        let s = src.data.data();
        let src_rows = src.nrow.max(1); // guard a 0-length vector (recycle base)
        for c in 0..src.ncol {
            for r in 0..nrow {
                // Recycle short vectors down their rows; matrices index directly.
                let value = if src.nrow == 0 {
                    na_real()
                } else {
                    s[c * src.nrow + (r % src_rows)]
                };
                out[col * nrow + r] = value;
            }
            col += 1;
        }
    }
    Ok(SValue::Matrix {
        data: Double::from_values(out),
        nrow,
        ncol,
    })
}

/// `rbind(…)` — bind vectors and matrices as rows. The common column count is the
/// largest source width; shorter **vectors** are recycled across columns, but a
/// **matrix** whose column count differs is an error. The all-empty call is
/// `NULL`.
fn b_rbind(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let mut sources = bind_sources(args)?;
    if sources.is_empty() {
        return Ok(SValue::Null);
    }
    // A bare vector is one ROW here, so reinterpret its `n × 1` as `1 × n`.
    for src in &mut sources {
        if !src.is_matrix {
            src.ncol = src.nrow;
            src.nrow = 1;
        }
    }
    let ncol = sources.iter().map(|s| s.ncol).max().unwrap_or(0);
    let nrow: usize = sources.iter().map(|s| s.nrow).sum();
    let total = nrow
        .checked_mul(ncol)
        .filter(|&t| t <= MAX_SEQ_LEN)
        .ok_or_else(|| SError::Index(format!("rbind: result too large (limit {MAX_SEQ_LEN})")))?;
    let mut out = vec![0.0; total];
    let mut row = 0usize;
    for src in &sources {
        if src.is_matrix && src.ncol != ncol {
            return Err(SError::BadArgs(format!(
                "rbind: number of columns of matrices must match ({} != {ncol})",
                src.ncol
            )));
        }
        let s = src.data.data();
        let src_cols = src.ncol.max(1);
        for r in 0..src.nrow {
            for c in 0..ncol {
                // Matrix element (r, c) at c*src.nrow + r; a recycled vector row
                // reads column c modulo its length.
                let value = if src.ncol == 0 {
                    na_real()
                } else if src.is_matrix {
                    s[c * src.nrow + r]
                } else {
                    s[c % src_cols]
                };
                out[c * nrow + (row + r)] = value;
            }
        }
        row += src.nrow;
    }
    Ok(SValue::Matrix {
        data: Double::from_values(out),
        nrow,
        ncol,
    })
}

/// Read a square matrix argument, rejecting NA, non-square, and over-`MAX_SOLVE_DIM`
/// cases up front. Returns the column-major data and the order `n`.
fn square_matrix(value: &SValue, who: &str) -> SResult<(Vec<f64>, usize)> {
    let (data, nrow, ncol) = matrix_parts(value)
        .ok_or_else(|| SError::TypeError(format!("{who}: 'a' must be a matrix")))?;
    if nrow != ncol {
        return Err(SError::BadArgs(format!(
            "{who}: 'a' must be square ({nrow}x{ncol})"
        )));
    }
    if nrow > MAX_SOLVE_DIM {
        return Err(SError::Index(format!(
            "{who}: matrix too large ({nrow}x{ncol}; limit {MAX_SOLVE_DIM})"
        )));
    }
    Ok((data.data().to_vec(), nrow))
}

/// `det(a)` — the determinant of a square matrix, via LU (Gaussian elimination
/// with partial pivoting). `NA` anywhere makes the result `NA`; a singular matrix
/// has determinant `0`.
fn b_det(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let (mut a, n) = square_matrix(first_positional(args)?, "det")?;
    if a.iter().any(|x| is_na_real(*x)) {
        return Ok(SValue::scalar(na_real()));
    }
    if n == 0 {
        return Ok(SValue::scalar(1.0)); // det of the 0×0 matrix is 1, as in R
    }
    let mut det = 1.0;
    for k in 0..n {
        // Partial pivot: the largest-magnitude entry in column k, rows k..n.
        let mut pivot = k;
        let mut best = a[k * n + k].abs();
        for i in (k + 1)..n {
            let v = a[k * n + i].abs();
            if v > best {
                best = v;
                pivot = i;
            }
        }
        if a[k * n + pivot] == 0.0 {
            return Ok(SValue::scalar(0.0)); // singular
        }
        if pivot != k {
            for c in 0..n {
                a.swap(c * n + k, c * n + pivot);
            }
            det = -det;
        }
        let diag = a[k * n + k];
        det *= diag;
        for i in (k + 1)..n {
            let f = a[k * n + i] / diag;
            if f != 0.0 {
                for c in k..n {
                    a[c * n + i] -= f * a[c * n + k];
                }
            }
        }
    }
    Ok(SValue::scalar(det))
}

/// `solve(a)` → the inverse of `a`; `solve(a, b)` → the `x` solving `a %*% x = b`
/// (`b` a vector → an `n`-vector result, or a matrix → an `n × m` result), via
/// Gauss–Jordan elimination with partial pivoting. A singular `a` is an error.
fn b_solve(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let (a, n) = square_matrix(first_positional(args)?, "solve")?;
    if a.iter().any(|x| is_na_real(*x)) {
        return Err(SError::BadArgs("solve: NA in 'a'".into()));
    }

    // The right-hand side: the second positional `b`, else the identity (inverse).
    // `b_is_vector` decides whether to return a vector or a matrix.
    let (b, m, b_is_vector) = match nth_positional(args, 1) {
        Some(b_val) => {
            if let Some((bd, bnr, bnc)) = matrix_parts(b_val) {
                if bnr != n {
                    return Err(SError::BadArgs(format!(
                        "solve: 'b' must have {n} rows (got {bnr})"
                    )));
                }
                (bd.data().to_vec(), bnc, false)
            } else {
                let bd = b_val.as_double()?;
                if bd.len() != n {
                    return Err(SError::BadArgs(format!(
                        "solve: 'b' must have length {n} (got {})",
                        bd.len()
                    )));
                }
                (bd.data().to_vec(), 1, true)
            }
        }
        None => {
            let mut id = vec![0.0; n * n];
            for i in 0..n {
                id[i * n + i] = 1.0;
            }
            (id, n, false)
        }
    };
    if b.iter().any(|x| is_na_real(*x)) {
        return Err(SError::BadArgs("solve: NA in 'b'".into()));
    }
    // The RHS elimination is O(n²·m), so a wide `b` (legal up to MAX_SEQ_LEN
    // elements ≈ 16k columns at n = 1000) would blow past the MAX_SOLVE_DIM work
    // budget the order cap alone enforces. Cap the column count too.
    if m > MAX_SOLVE_DIM {
        return Err(SError::Index(format!(
            "solve: too many right-hand sides ({m}; limit {MAX_SOLVE_DIM})"
        )));
    }

    let x = gauss_jordan(a, n, b, m)?;
    if b_is_vector {
        Ok(SValue::doubles(x))
    } else {
        Ok(SValue::Matrix {
            data: Double::from_values(x),
            nrow: n,
            ncol: m,
        })
    }
}

/// Solve `a x = b` for `x` by Gauss–Jordan elimination with partial pivoting.
/// `a` is `n × n` and `b` is `n × m`, both column-major; the returned `x` is
/// `n × m` column-major. A singular `a` is a clean error, never a panic.
fn gauss_jordan(mut a: Vec<f64>, n: usize, mut b: Vec<f64>, m: usize) -> SResult<Vec<f64>> {
    for k in 0..n {
        // Partial pivot for numerical stability.
        let mut pivot = k;
        let mut best = a[k * n + k].abs();
        for i in (k + 1)..n {
            let v = a[k * n + i].abs();
            if v > best {
                best = v;
                pivot = i;
            }
        }
        if best == 0.0 {
            return Err(SError::BadArgs("solve: matrix is exactly singular".into()));
        }
        if pivot != k {
            for c in 0..n {
                a.swap(c * n + k, c * n + pivot);
            }
            for c in 0..m {
                b.swap(c * n + k, c * n + pivot);
            }
        }
        // Eliminate column k from every other row.
        let diag = a[k * n + k];
        for i in 0..n {
            if i == k {
                continue;
            }
            let f = a[k * n + i] / diag;
            if f != 0.0 {
                for c in k..n {
                    a[c * n + i] -= f * a[c * n + k];
                }
                for c in 0..m {
                    b[c * n + i] -= f * b[c * n + k];
                }
            }
        }
    }
    // Normalize each pivot row to 1.
    for k in 0..n {
        let diag = a[k * n + k];
        for c in 0..m {
            b[c * n + k] /= diag;
        }
    }
    Ok(b)
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
    match peel_structural(first_positional(args)?) {
        SValue::Factor { levels, .. } => Ok(SValue::Character(
            levels.iter().cloned().map(Some).collect(),
        )),
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

/// `structure(x, ...)` — return `x` with each named `...` argument attached as an
/// attribute (R-16). `structure(1:3, class = "myc", foo = "bar")` attaches both
/// the special `class` and the general `foo`. Each named argument is routed
/// through the same per-name logic as `attr<-` (special names — `names`/`.Names`,
/// `class`, `dim`/`.Dim` — go to their dedicated wrappers; the rest into the
/// general attribute map), so a single call can set `dim`, `names`, and arbitrary
/// attributes consistently. The first positional argument is `x`; any further
/// positional arguments are ignored (R has no positional `...` attributes here).
fn b_structure(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let mut value = first_positional(args)?.clone();
    for a in args {
        if let Some(name) = &a.name {
            // `.Data` is R's positional alias for the object itself; skip it.
            if name == ".Data" {
                continue;
            }
            value = set_attr(value, name, &a.value)?;
        }
    }
    Ok(value)
}

/// `inherits(x, what)` — whether any class of `x` matches `what`.
fn b_inherits(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let positional: Vec<&SValue> = args
        .iter()
        .filter(|a| a.name.is_none())
        .map(|a| &a.value)
        .collect();
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
    let hit = what.into_iter().flatten().any(|w| classes.contains(&w));
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
// R-16 — general attributes (attr / attributes / structure)
// ===========================================================================
//
// R's attribute system is an open key→value metadata map on every object. Three
// attributes are *special* — `names`, `class`, `dim` — and we keep them in their
// dedicated representations so they can never disagree with the wrappers built
// for them:
//
//   | attribute | where it actually lives           | get reads                | set routes to        |
//   |-----------|-----------------------------------|--------------------------|----------------------|
//   | "names"   | `SValue::Named.names` (R-15)      | `names(x)`               | `with_names`         |
//   | "class"   | `SValue::Classed.class` (S v2)    | `class_of` (explicit)    | `Classed` / `unclass`|
//   | "dim"     | `SValue::Matrix{nrow,ncol}` (R-11)| `c(nrow, ncol)`          | reshape into Matrix  |
//
// Every *other* attribute is stored generally in `SValue::Attributed.attrs`.
// Because the special ones are never duplicated into that map, `attr(x,"names")`
// *is* `names(x)` — both read the same field — and likewise for class/dim. R
// also accepts `.Names`/`.Dim` as aliases for `names`/`dim`, which we honour.

/// Peel the transparent wrappers — `Attributed` (general attrs, R-16), `Classed`
/// (S v2 class), and `Named` (names, R-15) — to reach the *structural* value
/// underneath (a `Matrix`/`DataFrame`/`Factor`/atomic). Used by the
/// structural-query builtins (`dim`/`nrow`/`ncol`) so a value that has had a
/// class or general attribute layered on top still reports its shape — keeping
/// `attr(x,"dim")` and `dim(x)` in agreement even after `attr(x,"class") <- …`.
fn peel_structural(x: &SValue) -> &SValue {
    match x {
        SValue::Attributed { inner, .. } => peel_structural(inner),
        SValue::Classed { inner, .. } => peel_structural(inner),
        SValue::Named { values, .. } => peel_structural(values),
        other => other,
    }
}

/// Peel only the non-`Named` transparent wrappers (`Attributed`, `Classed`),
/// stopping at a `Named` so a names lookup still finds it.
fn peel_to_named(x: &SValue) -> &SValue {
    match x {
        SValue::Attributed { inner, .. } => peel_to_named(inner),
        SValue::Classed { inner, .. } => peel_to_named(inner),
        other => other,
    }
}

/// `attr(x, which)` — the named attribute, or `NULL` if absent. Special names are
/// synthesized from the dedicated wrappers; everything else is looked up in the
/// general attribute map.
fn get_attr(x: &SValue, which: &str) -> SValue {
    match which {
        "names" | ".Names" => match x {
            SValue::Named { names, .. } => SValue::Character(names.clone()),
            SValue::DataFrame { names, .. } => {
                SValue::Character(names.iter().cloned().map(Some).collect())
            }
            // Names live *inside* the class/general wrappers — see through them.
            SValue::Attributed { inner, .. } => get_attr(inner, "names"),
            SValue::Classed { inner, .. } => get_attr(inner, "names"),
            _ => SValue::Null,
        },
        "class" => match x {
            // Only an *explicitly* set class is an attribute; the implicit class
            // of a bare vector is not (matching R's `attr(1, "class")` → NULL).
            SValue::Classed { class, .. } => {
                SValue::Character(class.iter().cloned().map(Some).collect())
            }
            // See through a general-attribute wrapper to find an inner class.
            SValue::Attributed { inner, .. } => get_attr(inner, "class"),
            SValue::Named { values, .. } => get_attr(values, "class"),
            _ => SValue::Null,
        },
        "dim" | ".Dim" => match x {
            SValue::Matrix { nrow, ncol, .. } => SValue::doubles(vec![*nrow as f64, *ncol as f64]),
            SValue::Attributed { inner, .. } => get_attr(inner, "dim"),
            SValue::Classed { inner, .. } => get_attr(inner, "dim"),
            SValue::Named { values, .. } => get_attr(values, "dim"),
            _ => SValue::Null,
        },
        // A general attribute: look it up in the map (seeing through Named/Classed
        // so `attr(setNames(structure(...)), "foo")` still finds `foo`).
        _ => match x {
            SValue::Attributed { attrs, .. } => attrs
                .iter()
                .find(|(k, _)| k == which)
                .map(|(_, v)| v.clone())
                .unwrap_or(SValue::Null),
            SValue::Named { values, .. } => get_attr(values, which),
            SValue::Classed { inner, .. } => get_attr(inner, which),
            _ => SValue::Null,
        },
    }
}

/// `attr(x, which) <- value` — set/replace/remove a single attribute, returning
/// the modified value. Assigning `NULL` *removes* the attribute. Special names
/// route to their dedicated wrappers; the rest go into the general map (bounded
/// by [`MAX_ATTRIBUTES`]). Never panics on malformed input — a bad `dim` or
/// over-long `names` returns a clean `SError`.
fn set_attr(x: SValue, which: &str, value: &SValue) -> SResult<SValue> {
    let removing = matches!(value, SValue::Null);
    match which {
        "names" | ".Names" => set_names_attr(x, value),
        "class" => Ok(set_class_attr(x, value)),
        "dim" | ".Dim" => set_dim_attr(x, value),
        _ => set_general_attr(x, which, value, removing),
    }
}

/// Route a `names`/`class`/`dim` set into the existing wrapper machinery, leaving
/// the general-attribute layer (if any) intact around the result.
///
/// Set the `names` attribute, reusing R-15's `with_names` (NA-pad / truncate /
/// `NULL`-clear), preserving any general attributes around it.
fn set_names_attr(x: SValue, value: &SValue) -> SResult<SValue> {
    // Peel a general-attribute wrapper so names attach to the bare value, then
    // re-wrap. (`names<-` semantics are unchanged from R-15.)
    let (attrs, bare) = split_general(x);
    let renamed = match value {
        SValue::Null => bare.strip_names().clone(),
        v => {
            let new_names = v.as_character();
            if new_names.len() > bare.length() {
                return Err(SError::BadArgs(format!(
                    "'names' attribute [{}] must be no longer than the vector [{}]",
                    new_names.len(),
                    bare.length()
                )));
            }
            SValue::with_names(bare, new_names)
        }
    };
    Ok(SValue::with_general_attrs(renamed, attrs))
}

/// Set (or, with `NULL`, clear) the explicit S3 `class`, preserving general attrs.
fn set_class_attr(x: SValue, value: &SValue) -> SValue {
    let (attrs, bare) = split_general(x);
    // `unclass` first so re-setting replaces rather than nests.
    let unclassed = match bare {
        SValue::Classed { inner, .. } => *inner,
        other => other,
    };
    let classed = match value {
        SValue::Null => unclassed,
        v => {
            let class: Vec<String> = v.as_character().into_iter().flatten().collect();
            if class.is_empty() {
                unclassed
            } else {
                SValue::Classed {
                    inner: Box::new(unclassed),
                    class,
                }
            }
        }
    };
    SValue::with_general_attrs(classed, attrs)
}

/// Set (or, with `NULL`, clear) the `dim` attribute. Setting `dim <- c(nr, nc)`
/// reshapes a length-`nr*nc` numeric vector into a column-major matrix; the
/// product must equal the element count (as in R). Clearing turns a matrix back
/// into its flat vector. General attributes are preserved around the result.
fn set_dim_attr(x: SValue, value: &SValue) -> SResult<SValue> {
    let (attrs, bare) = split_general(x);
    let reshaped = match value {
        // Clearing dim: a matrix collapses to its flat column-major vector.
        SValue::Null => match bare {
            SValue::Matrix { data, .. } => SValue::Double(data),
            other => other,
        },
        v => {
            let dims = v.as_double()?;
            if dims.len() != 2 {
                return Err(SError::BadArgs(
                    "dim<-: only 2-D dims (c(nrow, ncol)) are supported".into(),
                ));
            }
            let nr = dim_component(dims.get_value(0))?;
            let nc = dim_component(dims.get_value(1))?;
            // Checked product, bounded — never allocate an oversize matrix.
            let total = nr
                .checked_mul(nc)
                .filter(|&t| t <= MAX_SEQ_LEN)
                .ok_or_else(|| {
                    SError::BadArgs(format!("dim<-: dimensions too large (limit {MAX_SEQ_LEN})"))
                })?;
            let data = bare.as_double()?;
            if data.len() != total {
                return Err(SError::BadArgs(format!(
                    "dim<-: dims [product {total}] do not match the length of object [{}]",
                    data.len()
                )));
            }
            SValue::Matrix {
                data,
                nrow: nr,
                ncol: nc,
            }
        }
    };
    Ok(SValue::with_general_attrs(reshaped, attrs))
}

/// Validate one `dim` component: a finite non-negative integer.
fn dim_component(x: Option<f64>) -> SResult<usize> {
    match x {
        Some(v) if v.is_finite() && v >= 0.0 && v.fract() == 0.0 => Ok(v as usize),
        _ => Err(SError::BadArgs(
            "dim<-: each dimension must be a non-negative integer".into(),
        )),
    }
}

/// Set/replace/remove a *general* (non-special) attribute in the `Attributed`
/// map, bounded by [`MAX_ATTRIBUTES`].
fn set_general_attr(x: SValue, which: &str, value: &SValue, removing: bool) -> SResult<SValue> {
    // Pull the current general attrs off (if any), keeping the special wrappers.
    let (mut attrs, bare) = split_general(x);
    if let Some(pos) = attrs.iter().position(|(k, _)| k == which) {
        if removing {
            attrs.remove(pos);
        } else {
            attrs[pos].1 = value.clone();
        }
    } else if !removing {
        if attrs.len() >= MAX_ATTRIBUTES {
            return Err(SError::BadArgs(format!(
                "too many attributes (limit {MAX_ATTRIBUTES})"
            )));
        }
        attrs.push((which.to_string(), value.clone()));
    }
    Ok(SValue::with_general_attrs(bare, attrs))
}

/// Split a value into its general-attribute list (empty if none) and the bare
/// value underneath the `Attributed` wrapper. Special wrappers (`Named`,
/// `Classed`, `Matrix`) stay intact inside `bare`.
fn split_general(x: SValue) -> (Vec<(String, SValue)>, SValue) {
    match x {
        SValue::Attributed { attrs, inner } => (attrs, *inner),
        other => (Vec::new(), other),
    }
}

/// `attr(x, which)` — get a single attribute (or `NULL`). `which` is the second
/// positional (or `which =`) argument, coerced to its first character element.
fn b_attr(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let x = first_positional(args)?;
    let which = attr_which(args)?;
    Ok(get_attr(x, &which))
}

/// `attr<-`(x, which, value)` — the replacement form behind `attr(x, which) <- v`.
/// The replacement convention passes `(x, which, value = v)`.
fn b_attr_replace(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let x = first_positional(args)?.clone();
    let which = attr_which(args)?;
    let value = args
        .iter()
        .find(|a| a.name.as_deref() == Some("value"))
        .map(|a| &a.value)
        .or_else(|| nth_positional(args, 2))
        .cloned()
        .unwrap_or(SValue::Null);
    set_attr(x, &which, &value)
}

/// Read the `which` argument of `attr` / `attr<-`: the `which =` named arg, or
/// the second positional. Must be a non-`NA` character scalar.
fn attr_which(args: &[Arg]) -> SResult<String> {
    let v = args
        .iter()
        .find(|a| a.name.as_deref() == Some("which"))
        .map(|a| &a.value)
        .or_else(|| nth_positional(args, 1))
        .ok_or_else(|| SError::BadArgs("attr: 'which' is missing".into()))?;
    v.as_character()
        .into_iter()
        .next()
        .flatten()
        .ok_or_else(|| SError::BadArgs("attr: 'which' must be a non-NA character string".into()))
}

/// `attributes(x)` — *all* attributes as a named list (or `NULL` if none). The
/// special attributes come first in R's canonical order (`names`, then `dim`,
/// then the general ones in insertion order), with `class` last.
fn b_attributes(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let x = first_positional(args)?;
    let mut pairs: Vec<(Option<String>, SValue)> = Vec::new();

    // names, then dim (special, canonical order).
    let names = get_attr(x, "names");
    if !matches!(names, SValue::Null) {
        pairs.push((Some("names".to_string()), names));
    }
    let dim = get_attr(x, "dim");
    if !matches!(dim, SValue::Null) {
        pairs.push((Some("dim".to_string()), dim));
    }
    // General attributes, in insertion order. The general-attribute wrapper is
    // always the *outermost* layer (every `set_attr`/`structure` re-wraps it
    // outside the special `Named`/`Classed`/`Matrix` wrappers), so a single
    // `general_attrs()` on `x` sees them all.
    if let Some(attrs) = x.general_attrs() {
        for (k, v) in attrs {
            pairs.push((Some(k.clone()), v.clone()));
        }
    }
    // class last.
    let class = get_attr(x, "class");
    if !matches!(class, SValue::Null) {
        pairs.push((Some("class".to_string()), class));
    }

    if pairs.is_empty() {
        Ok(SValue::Null)
    } else {
        Ok(SValue::list(pairs))
    }
}

/// `attributes<-`(x, value)` — replace the *whole* attribute set. `value` is a
/// named list (each element applied as the matching attribute) or `NULL` (clear
/// every attribute). An unnamed element, or a non-list / non-NULL `value`, is an
/// error. Bounded by [`MAX_ATTRIBUTES`] via the per-attribute `set_attr` path.
fn b_attributes_replace(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let x = first_positional(args)?.clone();
    let value = args
        .iter()
        .find(|a| a.name.as_deref() == Some("value"))
        .map(|a| &a.value)
        .or_else(|| nth_positional(args, 1))
        .cloned()
        .unwrap_or(SValue::Null);

    // Start from a fully bare value: clearing every attribute is the baseline.
    let bare = strip_all_attrs(x);
    match value {
        SValue::Null => Ok(bare),
        SValue::List { names, items } => {
            if items.len() > MAX_ATTRIBUTES {
                return Err(SError::BadArgs(format!(
                    "attributes<-: too many attributes (limit {MAX_ATTRIBUTES})"
                )));
            }
            let mut out = bare;
            for (name, item) in names.iter().zip(items.iter()) {
                let key = name.as_deref().filter(|s| !s.is_empty()).ok_or_else(|| {
                    SError::BadArgs("attributes<-: all attributes in the list must be named".into())
                })?;
                out = set_attr(out, key, item)?;
            }
            Ok(out)
        }
        other => Err(SError::TypeError(format!(
            "attributes<-: value must be a named list or NULL (got {})",
            other.type_name()
        ))),
    }
}

/// Strip *every* attribute (special and general) from a value, returning the bare
/// underlying vector — the baseline for `attributes(x) <- list(...)`.
fn strip_all_attrs(x: SValue) -> SValue {
    match x {
        SValue::Attributed { inner, .. } => strip_all_attrs(*inner),
        SValue::Named { values, .. } => strip_all_attrs(*values),
        SValue::Classed { inner, .. } => strip_all_attrs(*inner),
        SValue::Matrix { data, .. } => SValue::Double(data),
        other => other,
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
    let positional: Vec<&SValue> = args
        .iter()
        .filter(|a| a.name.is_none())
        .map(|a| &a.value)
        .collect();
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
    idx.sort_by(|&a, &b| {
        data[a]
            .partial_cmp(&data[b])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(SValue::doubles(
        idx.iter().map(|i| (i + 1) as f64).collect(),
    ))
}

/// `rep(x, times)` — concatenate `times` copies of `x`.
fn b_rep(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let x = first_positional(args)?.clone();
    let times = args
        .iter()
        .find(|a| a.name.as_deref() == Some("times"))
        .map(|a| &a.value)
        .or_else(|| {
            args.iter()
                .filter(|a| a.name.is_none())
                .nth(1)
                .map(|a| &a.value)
        })
        .and_then(|v| v.as_double().ok())
        .and_then(|d| d.get_value(0))
        .map(|n| n.max(0.0) as usize)
        .unwrap_or(1);
    // Bound the result so `rep(x, 1e12)` can't force a huge allocation.
    if x.length().saturating_mul(times) > MAX_SEQ_LEN {
        return Err(SError::BadArgs(format!(
            "rep result too large (limit {MAX_SEQ_LEN} elements)"
        )));
    }
    let copies: Vec<Arg> = (0..times)
        .map(|_| Arg {
            name: None,
            value: x.clone(),
        })
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
    let result = if bits.contains(&Some(true)) {
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
    let result = if bits.contains(&Some(false)) {
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
        other => other
            .as_character()
            .iter()
            .map(|o| Some(o.is_none()))
            .collect(),
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

// ===========================================================================
// Regular expressions
// ===========================================================================

/// Read a boolean named argument (e.g. `fixed = TRUE`), defaulting to `false`.
fn flag(args: &[Arg], name: &str) -> bool {
    args.iter()
        .find(|a| a.name.as_deref() == Some(name))
        .and_then(|a| a.value.truthy().ok())
        .unwrap_or(false)
}

/// Compile a pattern into a `regex::Regex`. With `fixed`, the pattern is matched
/// literally (escaped). An invalid pattern returns a clean error, never a panic.
fn compile(pattern: &str, fixed: bool) -> SResult<regex::Regex> {
    let source = if fixed {
        regex::escape(pattern)
    } else {
        pattern.to_string()
    };
    regex::Regex::new(&source)
        .map_err(|e| SError::BadArgs(format!("invalid regular expression '{pattern}': {e}")))
}

/// Translate an R replacement string to the `regex` crate's syntax: R back-
/// references `\1` become `${1}`, `\\` becomes a literal backslash, and a literal
/// `$` is escaped to `$$` (the regex crate's literal-dollar). With `fixed`, the
/// replacement is taken literally (only `$` needs escaping).
fn translate_replacement(repl: &str, fixed: bool) -> String {
    let mut out = String::with_capacity(repl.len());
    let mut chars = repl.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '$' => out.push_str("$$"),
            '\\' if !fixed => match chars.next() {
                Some(d) if d.is_ascii_digit() => {
                    out.push_str("${");
                    out.push(d);
                    out.push('}');
                }
                Some(other) => out.push(other), // `\\` -> `\`, `\x` -> `x`
                None => {}
            },
            other => out.push(other),
        }
    }
    out
}

/// The `(pattern, x, fixed)` of a `grepl`/`grep` call (pattern + first vector).
fn regex_unary(args: &[Arg]) -> SResult<(regex::Regex, Vec<Option<String>>)> {
    let pattern = nth_positional(args, 0)
        .map(|v| v.as_character())
        .and_then(|c| c.into_iter().next().flatten())
        .ok_or_else(|| SError::BadArgs("missing 'pattern'".into()))?;
    let x = nth_positional(args, 1)
        .ok_or_else(|| SError::BadArgs("missing 'x'".into()))?
        .as_character();
    Ok((compile(&pattern, flag(args, "fixed"))?, x))
}

/// `grepl(pattern, x)` — a logical vector: does each element match? `NA` → `NA`.
fn b_grepl(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let (re, x) = regex_unary(args)?;
    Ok(SValue::Logical(
        x.iter()
            .map(|o| o.as_ref().map(|s| re.is_match(s)))
            .collect(),
    ))
}

/// `grep(pattern, x, value = FALSE)` — the indices (1-based) of matching
/// elements, or the matching strings themselves when `value = TRUE`.
fn b_grep(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let (re, x) = regex_unary(args)?;
    let hits: Vec<(usize, &Option<String>)> = x
        .iter()
        .enumerate()
        .filter(|(_, o)| o.as_ref().is_some_and(|s| re.is_match(s)))
        .collect();
    if flag(args, "value") {
        Ok(SValue::Character(
            hits.iter().map(|(_, o)| (*o).clone()).collect(),
        ))
    } else {
        Ok(SValue::doubles(
            hits.iter().map(|(i, _)| (i + 1) as f64).collect(),
        ))
    }
}

/// Shared `gsub`/`sub` body. `all` replaces every match; otherwise the first.
fn replace_impl(args: &[Arg], all: bool) -> SResult<SValue> {
    let fixed = flag(args, "fixed");
    let pattern = nth_positional(args, 0)
        .map(|v| v.as_character())
        .and_then(|c| c.into_iter().next().flatten())
        .ok_or_else(|| SError::BadArgs("missing 'pattern'".into()))?;
    let replacement = nth_positional(args, 1)
        .map(|v| v.as_character())
        .and_then(|c| c.into_iter().next().flatten())
        .ok_or_else(|| SError::BadArgs("missing 'replacement'".into()))?;
    let x = nth_positional(args, 2)
        .ok_or_else(|| SError::BadArgs("missing 'x'".into()))?
        .as_character();

    let re = compile(&pattern, fixed)?;
    let rep = translate_replacement(&replacement, fixed);
    let out = x
        .into_iter()
        .map(|o| {
            o.map(|s| {
                if all {
                    re.replace_all(&s, rep.as_str()).into_owned()
                } else {
                    re.replace(&s, rep.as_str()).into_owned()
                }
            })
        })
        .collect();
    Ok(SValue::Character(out))
}

/// `gsub(pattern, replacement, x)` — replace every match in each element.
fn b_gsub(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    replace_impl(args, true)
}

/// `sub(pattern, replacement, x)` — replace only the first match in each element.
fn b_sub(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    replace_impl(args, false)
}

// ===========================================================================
// Lists
// ===========================================================================

/// `list(...)` — build a generic list from positional and named arguments,
/// preserving order and element names.
fn b_list(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let pairs = args
        .iter()
        .map(|a| (a.name.clone(), a.value.clone()))
        .collect();
    Ok(SValue::list(pairs))
}

/// `lapply(x, f)` — apply `f` to each element of `x`, returning a list of the
/// results (with `x`'s names, if any, carried over).
fn b_lapply(interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let positional: Vec<&Arg> = args.iter().filter(|a| a.name.is_none()).collect();
    let x = positional
        .first()
        .ok_or_else(|| SError::BadArgs("lapply: missing X".into()))?
        .value
        .clone();
    let f = positional
        .get(1)
        .ok_or_else(|| SError::BadArgs("lapply: missing FUN".into()))?
        .value
        .clone();
    if !f.is_callable() {
        return Err(SError::NotCallable(f.type_name().to_string()));
    }
    let names = list_names(&x);
    let mut items = Vec::with_capacity(x.length());
    for i in 0..x.length() {
        let elem = nth_element(&x, i);
        items.push(interp.call_value(
            f.clone(),
            &[Arg {
                name: None,
                value: elem,
            }],
        )?);
    }
    Ok(SValue::List { names, items })
}

/// The element names of `x` if it is a list, else all-unnamed.
fn list_names(x: &SValue) -> Vec<Option<String>> {
    match x {
        SValue::List { names, .. } => names.clone(),
        other => vec![None; other.length()],
    }
}

/// `strsplit(x, split)` — split each element of `x` by the fixed substring
/// `split`, returning a *list* of character vectors (one per element of `x`).
/// An empty `split` splits into individual characters (as in R).
fn b_strsplit(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let x = first_positional(args)?.as_character();
    let split = nth_positional(args, 1)
        .map(|v| v.as_character())
        .and_then(|c| c.into_iter().next().flatten())
        .unwrap_or_default();

    let items: Vec<SValue> = x
        .into_iter()
        .map(|o| match o {
            None => SValue::Character(vec![None]),
            Some(s) => {
                let parts: Vec<Option<String>> = if split.is_empty() {
                    s.chars().map(|c| Some(c.to_string())).collect()
                } else {
                    s.split(split.as_str())
                        .map(|p| Some(p.to_string()))
                        .collect()
                };
                SValue::Character(parts)
            }
        })
        .collect();
    let names = vec![None; items.len()];
    Ok(SValue::List { names, items })
}

// ===========================================================================
// String manipulation
// ===========================================================================

/// The `n`-th positional (unnamed) argument's value, if present.
fn nth_positional(args: &[Arg], n: usize) -> Option<&SValue> {
    args.iter()
        .filter(|a| a.name.is_none())
        .map(|a| &a.value)
        .nth(n)
}

/// Read a scalar integer (the first element, truncated toward zero) from an
/// optional value; missing/empty/NA becomes `default`.
fn scalar_int(value: Option<&SValue>, default: i64) -> i64 {
    value
        .and_then(|v| v.as_double().ok())
        .and_then(|d| d.get_value(0))
        .filter(|x| !is_na_real(*x))
        .map(|x| x as i64)
        .unwrap_or(default)
}

/// `nchar(x)` — the character count of each element. `NA` elements yield `NA`.
fn b_nchar(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let chars = first_positional(args)?.as_character();
    Ok(SValue::doubles(
        chars
            .iter()
            .map(|o| match o {
                Some(s) => s.chars().count() as f64,
                None => na_real(),
            })
            .collect(),
    ))
}

/// Map a character vector element-wise through `f`, preserving `NA`.
fn map_chars(args: &[Arg], f: impl Fn(&str) -> String) -> SResult<SValue> {
    let chars = first_positional(args)?.as_character();
    Ok(SValue::Character(
        chars.into_iter().map(|o| o.map(|s| f(&s))).collect(),
    ))
}

/// `toupper(x)` — upper-case each element.
fn b_toupper(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    map_chars(args, |s| s.to_uppercase())
}

/// `tolower(x)` — lower-case each element.
fn b_tolower(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    map_chars(args, |s| s.to_lowercase())
}

/// `substr(x, start, stop)` — the 1-based inclusive character substring of each
/// element. `start`/`stop` are taken as scalars and clamped to the string; an
/// out-of-order or out-of-range range yields the empty string. Operates on
/// `char`s, so it is always UTF-8 boundary safe.
fn b_substr(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let x = nth_positional(args, 0)
        .ok_or_else(|| SError::BadArgs("substr: argument \"x\" is missing".into()))?
        .as_character();
    let start = scalar_int(nth_positional(args, 1), 1);
    let stop = scalar_int(nth_positional(args, 2), i64::MAX);
    let out = x
        .into_iter()
        .map(|o| o.map(|s| substring(&s, start, stop)))
        .collect();
    Ok(SValue::Character(out))
}

/// The 1-based inclusive `[start, stop]` character slice of `s`.
fn substring(s: &str, start: i64, stop: i64) -> String {
    if stop < start || stop < 1 {
        return String::new();
    }
    let from = (start - 1).max(0) as usize;
    let count = (stop - start.max(1) + 1).max(0) as usize;
    s.chars().skip(from).take(count).collect()
}

/// `sprintf(fmt, ...)` — a minimal C-style formatter supporting `%d`/`%i`,
/// `%s`, `%f`/`%e`/`%g`, and `%%`, with optional width, `.precision`, and the
/// `-` (left-justify) and `0` (zero-pad) flags. Vectorized: the result has the
/// length of the longest argument, with shorter arguments recycled.
fn b_sprintf(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let fmt = first_positional(args)?
        .as_character()
        .into_iter()
        .next()
        .flatten()
        .ok_or_else(|| SError::BadArgs("sprintf: 'fmt' is missing".into()))?;

    // The substitution arguments are every positional after the format string.
    let rest: Vec<&SValue> = args
        .iter()
        .filter(|a| a.name.is_none())
        .skip(1)
        .map(|a| &a.value)
        .collect();
    let n = rest.iter().map(|v| v.length().max(1)).max().unwrap_or(1);

    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        out.push(Some(format_one(&fmt, &rest, i)?));
    }
    Ok(SValue::Character(out))
}

/// Render the format string once, pulling argument index `row` from each
/// consumed conversion's value (recycled).
fn format_one(fmt: &str, args: &[&SValue], row: usize) -> SResult<String> {
    let mut out = String::new();
    let mut chars = fmt.chars().peekable();
    let mut arg_idx = 0usize;

    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        // `%%` is a literal percent.
        if chars.peek() == Some(&'%') {
            chars.next();
            out.push('%');
            continue;
        }
        // flags
        let mut left = false;
        let mut zero = false;
        while let Some(&f) = chars.peek() {
            match f {
                '-' => left = true,
                '0' => zero = true,
                '+' | ' ' | '#' => {}
                _ => break,
            }
            chars.next();
        }
        // width
        let mut width = 0usize;
        while let Some(&d) = chars.peek() {
            if d.is_ascii_digit() {
                width = width
                    .saturating_mul(10)
                    .saturating_add((d as u8 - b'0') as usize);
                chars.next();
            } else {
                break;
            }
        }
        // precision
        let mut precision: Option<usize> = None;
        if chars.peek() == Some(&'.') {
            chars.next();
            let mut p = 0usize;
            while let Some(&d) = chars.peek() {
                if d.is_ascii_digit() {
                    p = p
                        .saturating_mul(10)
                        .saturating_add((d as u8 - b'0') as usize);
                    chars.next();
                } else {
                    break;
                }
            }
            precision = Some(p);
        }
        // Cap the field width and precision so a crafted format like
        // `%999999999999d` can't trigger an unbounded allocation in `pad`.
        const MAX_FIELD: usize = 1 << 20; // 1 MiB per field
        if width > MAX_FIELD || precision.is_some_and(|p| p > MAX_FIELD) {
            return Err(SError::BadArgs(
                "sprintf: field width or precision is too large".into(),
            ));
        }
        let conv = chars
            .next()
            .ok_or_else(|| SError::BadArgs("sprintf: truncated format".into()))?;

        let value = args.get(arg_idx).copied();
        arg_idx += 1;
        let body = render_conversion(conv, value, row, precision)?;
        out.push_str(&pad(&body, width, left, zero && !left));
    }
    Ok(out)
}

/// Format one conversion's value (recycling row index) into its unpadded body.
fn render_conversion(
    conv: char,
    value: Option<&SValue>,
    row: usize,
    precision: Option<usize>,
) -> SResult<String> {
    let nth_string = |v: &SValue| {
        let c = v.as_character();
        c.get(row % c.len().max(1))
            .cloned()
            .flatten()
            .unwrap_or_else(|| "NA".to_string())
    };
    let nth_double = |v: &SValue| -> f64 {
        v.as_double()
            .ok()
            .and_then(|d| d.get_value(row % d.len().max(1)))
            .unwrap_or(f64::NAN)
    };
    Ok(match conv {
        's' => value.map(nth_string).unwrap_or_default(),
        'd' | 'i' => {
            let x = value.map(nth_double).unwrap_or(0.0);
            format!("{}", x as i64)
        }
        'f' => {
            let x = value.map(nth_double).unwrap_or(0.0);
            format!("{:.*}", precision.unwrap_or(6), x)
        }
        'e' => {
            let x = value.map(nth_double).unwrap_or(0.0);
            format!("{:.*e}", precision.unwrap_or(6), x)
        }
        'g' => {
            let x = value.map(nth_double).unwrap_or(0.0);
            // %g: trim trailing zeros from a fixed rendering.
            let s = format!("{:.*}", precision.unwrap_or(6), x);
            if s.contains('.') {
                s.trim_end_matches('0').trim_end_matches('.').to_string()
            } else {
                s
            }
        }
        other => return Err(SError::BadArgs(format!("sprintf: unsupported %{other}"))),
    })
}

/// Pad `body` to `width` columns. `left` left-justifies; otherwise right-justify,
/// using zeros when `zero` is set (only meaningful for right-justified numbers).
fn pad(body: &str, width: usize, left: bool, zero: bool) -> String {
    let len = body.chars().count();
    if len >= width {
        return body.to_string();
    }
    let fill = if zero { "0" } else { " " }.repeat(width - len);
    if left {
        format!("{body}{fill}")
    } else {
        format!("{fill}{body}")
    }
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
        let r = interp.call_value(
            f.clone(),
            &[Arg {
                name: None,
                value: elem,
            }],
        )?;
        results.push(Arg {
            name: None,
            value: r,
        });
    }
    Ok(combine(&results))
}

// ===========================================================================
// Higher-order functionals (R-10)
// ===========================================================================
//
// The classic functional-programming toolkit, pairing with the R-9 `\(x)`
// lambdas. Like `sapply`/`lapply` they take a callable and invoke it through
// `interp.call_value`. `Map`/`mapply` zip several sequences element-wise (Map
// returns a list, mapply simplifies to a vector); `Reduce` folds; `Filter`
// keeps elements; `vapply` is `sapply` with a result-shape template.

/// Split a functional's arguments into `(function, data…)`. The function is the
/// one passed by name (`f =` / `FUN =`) or, failing that, the first *callable*
/// positional argument; the remaining positionals are the data, in order. This
/// matches R's argument matching closely enough that both `Map(f, x, y)` and the
/// piped `x |> Map(f = …)` work (`|>` makes the data the first positional, so the
/// function must be named).
fn split_fun(args: &[Arg], name: &str) -> SResult<(SValue, Vec<SValue>)> {
    let positional: Vec<SValue> = args
        .iter()
        .filter(|a| a.name.is_none())
        .map(|a| a.value.clone())
        .collect();
    let named_f = args
        .iter()
        .find(|a| matches!(a.name.as_deref(), Some("f") | Some("FUN")))
        .map(|a| a.value.clone());

    let (f, data) = match named_f {
        Some(f) => (f, positional),
        None => {
            let idx = positional
                .iter()
                .position(|v| v.is_callable())
                .ok_or_else(|| SError::BadArgs(format!("{name}: missing function argument")))?;
            let mut data = positional;
            let f = data.remove(idx);
            (f, data)
        }
    };
    if !f.is_callable() {
        return Err(SError::NotCallable(f.type_name().to_string()));
    }
    Ok((f, data))
}

/// `Map(f, ...)` — apply `f` element-wise across one or more sequences,
/// recycling shorter ones to the longest. Returns a list (one entry per
/// element); use `mapply` for a simplified vector.
fn b_map(interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let (items, _) = zip_apply(interp, args, "Map")?;
    Ok(SValue::List {
        names: vec![None; items.len()],
        items,
    })
}

/// `mapply(f, ...)` — like `Map`, but simplifies the results to a vector.
fn b_mapply(interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let (items, _) = zip_apply(interp, args, "mapply")?;
    let results: Vec<Arg> = items
        .into_iter()
        .map(|value| Arg { name: None, value })
        .collect();
    Ok(combine(&results))
}

/// Shared engine for `Map`/`mapply`: call `f(seq1[i], seq2[i], …)` for each
/// position `i`, recycling each sequence to the longest length.
fn zip_apply(interp: &Interpreter, args: &[Arg], name: &str) -> SResult<(Vec<SValue>, usize)> {
    let (f, seqs) = split_fun(args, name)?;
    if seqs.is_empty() {
        return Err(SError::BadArgs(format!("{name}: nothing to iterate over")));
    }
    // The result length is the longest input; an empty input yields nothing.
    let lengths: Vec<usize> = seqs.iter().map(|s| s.length()).collect();
    let len = if lengths.contains(&0) {
        0
    } else {
        lengths.iter().copied().max().unwrap_or(0)
    };
    let mut items = Vec::with_capacity(len);
    for i in 0..len {
        let call_args: Vec<Arg> = seqs
            .iter()
            .zip(&lengths)
            .map(|(s, &l)| Arg {
                name: None,
                value: nth_element(s, i % l), // recycle
            })
            .collect();
        items.push(interp.call_value(f.clone(), &call_args)?);
    }
    Ok((items, len))
}

/// `Reduce(f, x[, init])` — left fold. Without `init`, `f` is first applied to
/// the first two elements; an empty `x` with no `init` is `NULL`.
fn b_reduce(interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let (f, data) = split_fun(args, "Reduce")?;
    let x = data
        .first()
        .cloned()
        .ok_or_else(|| SError::BadArgs("Reduce: missing x".into()))?;
    // The initial value comes from `init =` or the next positional after x.
    let init = args
        .iter()
        .find(|a| a.name.as_deref() == Some("init"))
        .map(|a| a.value.clone())
        .or_else(|| data.get(1).cloned());

    let n = x.length();
    let (mut acc, start) = match init {
        Some(v) => (v, 0),
        None if n == 0 => return Ok(SValue::Null),
        None => (nth_element(&x, 0), 1),
    };
    for i in start..n {
        acc = interp.call_value(
            f.clone(),
            &[
                Arg {
                    name: None,
                    value: acc,
                },
                Arg {
                    name: None,
                    value: nth_element(&x, i),
                },
            ],
        )?;
    }
    Ok(acc)
}

/// `Filter(f, x)` — keep the elements of `x` for which `f(element)` is true.
/// Returns the same kind of structure (a list stays a list, a vector stays a
/// vector).
fn b_filter(interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let (f, data) = split_fun(args, "Filter")?;
    let x = data
        .into_iter()
        .next()
        .ok_or_else(|| SError::BadArgs("Filter: missing x".into()))?;
    let mut keep = Vec::new();
    for i in 0..x.length() {
        let verdict = interp.call_value(
            f.clone(),
            &[Arg {
                name: None,
                value: nth_element(&x, i),
            }],
        )?;
        if verdict.truthy()? {
            keep.push(i);
        }
    }
    match &x {
        SValue::List { names, items } => Ok(SValue::List {
            names: keep.iter().map(|&i| names[i].clone()).collect(),
            items: keep.iter().map(|&i| items[i].clone()).collect(),
        }),
        _ => {
            let kept: Vec<Arg> = keep
                .iter()
                .map(|&i| Arg {
                    name: None,
                    value: nth_element(&x, i),
                })
                .collect();
            Ok(combine(&kept))
        }
    }
}

/// `vapply(x, f, template)` — like `sapply`, but every result must match the
/// length of `template` (its `FUN.VALUE`); a mismatch is an error. This makes
/// the result shape predictable, unlike `sapply`.
fn b_vapply(interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    // `vapply(X, FUN, FUN.VALUE)`: the function is the callable argument, the
    // data and template are the other positionals (X first, template next).
    let (f, data) = split_fun(args, "vapply")?;
    let x = data
        .first()
        .cloned()
        .ok_or_else(|| SError::BadArgs("vapply: missing X".into()))?;
    let tlen = args
        .iter()
        .find(|a| a.name.as_deref() == Some("FUN.VALUE"))
        .map(|a| a.value.clone())
        .or_else(|| data.get(1).cloned())
        .ok_or_else(|| SError::BadArgs("vapply: missing FUN.VALUE template".into()))?
        .length();

    let mut results: Vec<Arg> = Vec::with_capacity(x.length());
    for i in 0..x.length() {
        let r = interp.call_value(
            f.clone(),
            &[Arg {
                name: None,
                value: nth_element(&x, i),
            }],
        )?;
        if r.length() != tlen {
            return Err(SError::BadArgs(format!(
                "vapply: values must be length {tlen}, but element {} is length {}",
                i + 1,
                r.length()
            )));
        }
        results.push(Arg {
            name: None,
            value: r,
        });
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
// Distribution family (R-8) — d/p/q/r over statistics-core
// ===========================================================================
//
// R names probability functions with a one-letter prefix on the distribution:
//
//   prefix   meaning                  example         maps to
//   ------   ----------------------   -------------   ----------------------
//   d*       density / mass           dnorm(x)        statistics_core::…::dnorm
//   p*       cumulative probability   pnorm(q)        …::pnorm  (CDF, P[X ≤ q])
//   q*       quantile (inverse CDF)   qnorm(p)        …::qnorm  (the x with that p)
//   r*       random sample of size n  rnorm(n)        …::rnorm  (draws from the RNG)
//
// `d*`/`p*`/`q*` are pure and **vectorized over their first argument** (the
// quantile/probability vector); the distribution parameters (`mean`, `sd`,
// `min`, `max`, `rate`) are read as scalars, by name or position, with R's
// defaults. `r*` draws from the session generator on the `Interpreter`, so a
// run of `set.seed(s); rnorm(3)` is reproducible. NA in the input propagates to
// NA in the output, exactly as in R.
//
// Scope: the closed-form continuous families (normal, uniform, exponential).
// Their density/CDF/quantile are O(1) and sampling is O(n), so there are no
// input-driven unbounded loops — the only resource knob is `n`, which
// [`sample_count`] caps at `MAX_SEQ_LEN`. Discrete families (binomial, Poisson),
// whose CDF/sampling loop over a user-supplied count, are a separate follow-up.

/// `set.seed(n)` — reseed the session RNG so subsequent `r*` draws are
/// reproducible. Returns invisibly (`NULL`), like R.
fn b_set_seed(interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let seed = first_positional(args)?
        .as_double()?
        .get_value(0)
        .filter(|v| v.is_finite())
        .ok_or_else(|| SError::BadArgs("set.seed: seed must be a finite number".into()))?;
    // R coerces the seed to a 32-bit integer; mirror that so `set.seed(1)`
    // behaves the same here as there.
    interp.reseed(seed.trunc() as i64 as u32 as u64);
    Ok(SValue::Null)
}

/// Read distribution parameter `name` (or positional index `pos`) as a scalar,
/// falling back to `default` when absent. Position 0 is the quantile/probability
/// vector, so parameters start at position 1.
fn dist_param(args: &[Arg], pos: usize, name: &str, default: f64) -> SResult<f64> {
    let v = args
        .iter()
        .find(|a| a.name.as_deref() == Some(name))
        .map(|a| &a.value)
        .or_else(|| nth_positional(args, pos));
    match v {
        Some(sv) => Ok(sv.as_double()?.get_value(0).unwrap_or(default)),
        None => Ok(default),
    }
}

/// Interpret the first positional argument as a sample count for an `r*`
/// function. Following R, a vector of length > 1 means "draw `length(n)`
/// samples". The result is capped at [`MAX_SEQ_LEN`] and rejects non-finite or
/// negative counts, so `rnorm(1e18)` is a clean error rather than an
/// out-of-memory abort.
fn sample_count(args: &[Arg]) -> SResult<usize> {
    let nv = first_positional(args)?.as_double()?;
    let n = if nv.len() > 1 {
        nv.len()
    } else {
        let v = nv
            .get_value(0)
            .ok_or_else(|| SError::BadArgs("invalid arguments (n is missing)".into()))?;
        if !v.is_finite() || v < 0.0 {
            return Err(SError::BadArgs(
                "invalid arguments (n must be a non-negative count)".into(),
            ));
        }
        v.trunc() as usize // saturates for huge v; the cap below rejects it
    };
    if n > MAX_SEQ_LEN {
        return Err(SError::BadArgs(format!(
            "cannot allocate a sample of length {n} (limit {MAX_SEQ_LEN})"
        )));
    }
    Ok(n)
}

// --- normal --------------------------------------------------------------

fn b_dnorm(_i: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let (mean, sd) = (
        dist_param(args, 1, "mean", 0.0)?,
        dist_param(args, 2, "sd", 1.0)?,
    );
    unary_math(args, move |x| dnorm(x, mean, sd))
}
fn b_pnorm(_i: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let (mean, sd) = (
        dist_param(args, 1, "mean", 0.0)?,
        dist_param(args, 2, "sd", 1.0)?,
    );
    unary_math(args, move |x| pnorm(x, mean, sd))
}
fn b_qnorm(_i: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let (mean, sd) = (
        dist_param(args, 1, "mean", 0.0)?,
        dist_param(args, 2, "sd", 1.0)?,
    );
    unary_math(args, move |p| qnorm(p, mean, sd))
}
fn b_rnorm(interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let n = sample_count(args)?;
    let (mean, sd) = (
        dist_param(args, 1, "mean", 0.0)?,
        dist_param(args, 2, "sd", 1.0)?,
    );
    Ok(SValue::doubles(
        interp.sample_with(|rng| rnorm(n, mean, sd, rng)),
    ))
}

// --- uniform -------------------------------------------------------------

fn b_dunif(_i: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let (min, max) = (
        dist_param(args, 1, "min", 0.0)?,
        dist_param(args, 2, "max", 1.0)?,
    );
    unary_math(args, move |x| dunif(x, min, max))
}
fn b_punif(_i: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let (min, max) = (
        dist_param(args, 1, "min", 0.0)?,
        dist_param(args, 2, "max", 1.0)?,
    );
    unary_math(args, move |x| punif(x, min, max))
}
fn b_qunif(_i: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let (min, max) = (
        dist_param(args, 1, "min", 0.0)?,
        dist_param(args, 2, "max", 1.0)?,
    );
    unary_math(args, move |p| qunif(p, min, max))
}
fn b_runif(interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let n = sample_count(args)?;
    let (min, max) = (
        dist_param(args, 1, "min", 0.0)?,
        dist_param(args, 2, "max", 1.0)?,
    );
    Ok(SValue::doubles(
        interp.sample_with(|rng| runif(n, min, max, rng)),
    ))
}

// --- exponential ---------------------------------------------------------

fn b_dexp(_i: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let rate = dist_param(args, 1, "rate", 1.0)?;
    unary_math(args, move |x| dexp(x, rate))
}
fn b_pexp(_i: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let rate = dist_param(args, 1, "rate", 1.0)?;
    unary_math(args, move |x| pexp(x, rate))
}
fn b_qexp(_i: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let rate = dist_param(args, 1, "rate", 1.0)?;
    unary_math(args, move |p| qexp(p, rate))
}
fn b_rexp(interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let n = sample_count(args)?;
    let rate = dist_param(args, 1, "rate", 1.0)?;
    Ok(SValue::doubles(
        interp.sample_with(|rng| rexp(n, rate, rng)),
    ))
}

// ===========================================================================
// Discrete distribution family (R-8b) — binomial and Poisson
// ===========================================================================
//
// Unlike the continuous families (R-8), the discrete CDFs and samplers loop
// over an integer *count*: `pbinom`/`qbinom` sum/scan O(size) terms,
// `ppois` sums O(x) terms (Poisson has unbounded support), and the samplers
// draw via inverse-CDF, so `rbinom` is O(n·size). Left unbounded, a crafted
// `pbinom(0, size = 1e18)` or `rbinom(1e6, size = 1e9)` would hang the process.
//
// Two guards bound every loop:
//   * MAX_DISCRETE_SUPPORT caps the per-element driver — `size` (binomial) and
//     the `x` quantile fed to `ppois` — so no single term-sum runs away.
//   * MAX_DISCRETE_WORK caps the *total* iterations of a call: `len · driver`
//     for the vectorized `d`/`p`/`q` functions and `n · per_sample` for the
//     samplers. Anything larger is a clean error, never an unbounded loop.
// statistics-core's own `qpois`/Poisson sampler already cap their internal
// scan at ~10_000, which we reuse as the Poisson per-element/per-sample driver.

/// Largest per-element loop driver (`size`, or the `ppois` quantile) we accept.
const MAX_DISCRETE_SUPPORT: u64 = 1 << 20; // ~1.05M
/// Largest total inner-loop iteration count a single discrete call may incur.
const MAX_DISCRETE_WORK: u128 = 1 << 27; // ~134M (sub-second)
/// statistics-core caps the Poisson quantile/sampler scan at this many terms.
const POISSON_SCAN_CAP: u128 = 10_000;

/// Reject a discrete call whose estimated total iteration count exceeds the
/// budget, before it runs.
fn check_discrete_work(count: u128, driver: u128, what: &str) -> SResult<()> {
    if count.saturating_mul(driver) > MAX_DISCRETE_WORK {
        return Err(SError::BadArgs(format!(
            "{what}: requested work exceeds the safety limit \
             (count {count} × {driver} > {MAX_DISCRETE_WORK})"
        )));
    }
    Ok(())
}

/// Read a **required** distribution parameter (by name or position) as a finite
/// scalar — `dbinom`/`dpois` have no defaults for `size`/`prob`/`lambda`.
fn required_param(args: &[Arg], pos: usize, name: &str) -> SResult<f64> {
    let v = args
        .iter()
        .find(|a| a.name.as_deref() == Some(name))
        .map(|a| &a.value)
        .or_else(|| nth_positional(args, pos))
        .ok_or_else(|| {
            SError::BadArgs(format!("argument \"{name}\" is missing, with no default"))
        })?;
    v.as_double()?
        .get_value(0)
        .filter(|x| x.is_finite())
        .ok_or_else(|| SError::BadArgs(format!("invalid \"{name}\" (must be a finite number)")))
}

/// Read and validate the binomial parameters `(size, prob)`. `size` is a
/// non-negative count capped at [`MAX_DISCRETE_SUPPORT`]; `prob` is in `[0, 1]`.
fn binom_params(args: &[Arg]) -> SResult<(u64, f64)> {
    let size_f = required_param(args, 1, "size")?;
    if size_f < 0.0 || size_f > MAX_DISCRETE_SUPPORT as f64 {
        return Err(SError::BadArgs(format!(
            "size must be in 0..={MAX_DISCRETE_SUPPORT} (got {size_f})"
        )));
    }
    let prob = required_param(args, 2, "prob")?;
    if !(0.0..=1.0).contains(&prob) {
        return Err(SError::BadArgs(format!(
            "prob must be in [0, 1] (got {prob})"
        )));
    }
    Ok((size_f.trunc() as u64, prob))
}

/// Read and validate the Poisson `lambda` (a non-negative rate).
fn pois_lambda(args: &[Arg]) -> SResult<f64> {
    let lambda = required_param(args, 1, "lambda")?;
    if lambda < 0.0 {
        return Err(SError::BadArgs(format!(
            "lambda must be ≥ 0 (got {lambda})"
        )));
    }
    Ok(lambda)
}

/// Map an integer-valued density/CDF/quantile over the first positional vector,
/// truncating each element toward zero and propagating `NA`.
fn map_discrete(args: &[Arg], f: impl Fn(f64) -> f64) -> SResult<SValue> {
    let x = first_positional(args)?.as_double()?;
    Ok(SValue::doubles(
        x.iter()
            .map(|v| if is_na_real(v) { na_real() } else { f(v) })
            .collect(),
    ))
}

// --- binomial ------------------------------------------------------------

fn b_dbinom(_i: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let (size, prob) = binom_params(args)?; // pmf is O(1) per element
    map_discrete(args, move |x| dbinom(x.trunc() as i64, size, prob))
}
fn b_pbinom(_i: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let (size, prob) = binom_params(args)?;
    let x = first_positional(args)?.as_double()?;
    check_discrete_work(x.len() as u128, size as u128, "pbinom")?;
    map_discrete(args, move |q| pbinom(q.trunc() as i64, size, prob))
}
fn b_qbinom(_i: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let (size, prob) = binom_params(args)?;
    let p = first_positional(args)?.as_double()?;
    check_discrete_work(p.len() as u128, size as u128, "qbinom")?;
    map_discrete(args, move |p| qbinom(p, size, prob) as f64)
}
fn b_rbinom(interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let n = sample_count(args)?;
    let (size, prob) = binom_params(args)?;
    check_discrete_work(n as u128, size as u128, "rbinom")?; // each draw is O(size)
    Ok(SValue::doubles(
        interp
            .sample_with(|rng| rbinom(n, size, prob, rng))
            .iter()
            .map(|&k| k as f64)
            .collect(),
    ))
}

// --- Poisson -------------------------------------------------------------

fn b_dpois(_i: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let lambda = pois_lambda(args)?; // pmf is O(1) per element
    map_discrete(args, move |x| dpois(x.trunc() as i64, lambda))
}
fn b_ppois(_i: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let lambda = pois_lambda(args)?;
    let x = first_positional(args)?.as_double()?;
    // ppois sums O(x) terms (unbounded support) — bound the largest x and the
    // total work before evaluating.
    let max_x = x
        .iter()
        .filter(|v| v.is_finite() && *v >= 0.0)
        .map(|v| v.trunc() as i128)
        .max()
        .unwrap_or(0);
    if max_x > MAX_DISCRETE_SUPPORT as i128 {
        return Err(SError::BadArgs(format!(
            "ppois: x = {max_x} exceeds the safety limit of {MAX_DISCRETE_SUPPORT}"
        )));
    }
    check_discrete_work(x.len() as u128, max_x.max(1) as u128, "ppois")?;
    map_discrete(args, move |q| ppois(q.trunc() as i64, lambda))
}
fn b_qpois(_i: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let lambda = pois_lambda(args)?;
    let p = first_positional(args)?.as_double()?;
    // statistics-core caps the quantile scan at ~10_000 terms per element.
    check_discrete_work(p.len() as u128, POISSON_SCAN_CAP, "qpois")?;
    map_discrete(args, move |p| qpois(p, lambda) as f64)
}
fn b_rpois(interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let n = sample_count(args)?;
    let lambda = pois_lambda(args)?;
    // Knuth sampling is ~O(lambda) for small lambda; the large-lambda path uses
    // the ~10_000-capped quantile. Bound the per-sample cost accordingly.
    let per_sample = if lambda < 30.0 { 64 } else { POISSON_SCAN_CAP };
    check_discrete_work(n as u128, per_sample, "rpois")?;
    Ok(SValue::doubles(
        interp
            .sample_with(|rng| rpois(n, lambda, rng))
            .iter()
            .map(|&k| k as f64)
            .collect(),
    ))
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
