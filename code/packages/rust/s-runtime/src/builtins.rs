//! Built-in functions installed into the global environment.
//!
//! These are the names an S user expects to be available without defining them:
//! the vector constructor `c`, `length`, `print`, `seq`, and the statistical
//! reductions. The reductions are thin glue over `statistics-core` — the same
//! crate that backs the spreadsheet and (eventually) R frontends — so the math
//! has a single authoritative home.

use crate::env::{define, lookup, Env};
use crate::error::{SError, SResult};
use crate::eval::{nth_element, Interpreter};
use crate::value::{
    bounded_sequence, class_of, combine, format_number, index, membership, Arg, SValue,
    MAX_ATTRIBUTES, MAX_SEQ_LEN,
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
    // R-29 — vector set operations & ordering.
    define(env, "union", builtin("union", b_union));
    define(env, "intersect", builtin("intersect", b_intersect));
    define(env, "setdiff", builtin("setdiff", b_setdiff));
    define(env, "is.element", builtin("is.element", b_is_element));
    define(env, "duplicated", builtin("duplicated", b_duplicated));
    // R-30 — first-duplicate index (ordering refinements).
    define(env, "anyDuplicated", builtin("anyDuplicated", b_any_duplicated));
    define(env, "rank", builtin("rank", b_rank));
    define(env, "which", builtin("which", b_which));
    define(env, "any", builtin("any", b_any));
    define(env, "all", builtin("all", b_all));
    define(env, "is.na", builtin("is.na", b_is_na));
    // R-23 — environment predicate + the `environment(f) <- e` replacement.
    define(env, "is.environment", builtin("is.environment", b_is_environment));
    define(
        env,
        "environment<-",
        builtin("environment<-", b_environment_replace),
    );
    define(env, "cumsum", builtin("cumsum", b_cumsum));
    define(env, "cumprod", builtin("cumprod", b_cumprod));
    define(env, "paste", builtin("paste", b_paste));
    define(env, "paste0", builtin("paste0", b_paste0));

    // Lists (R-6).
    define(env, "list", builtin("list", b_list));
    define(env, "lapply", builtin("lapply", b_lapply));
    define(env, "strsplit", builtin("strsplit", b_strsplit));

    // Reflective call + list overlay (R-17).
    define(env, "do.call", builtin("do.call", b_do_call));
    define(env, "modifyList", builtin("modifyList", b_modify_list));

    // Higher-order functionals (R-10) — pair with the R-9 `\(x)` lambdas.
    define(env, "Map", builtin("Map", b_map));
    define(env, "Reduce", builtin("Reduce", b_reduce));
    define(env, "Filter", builtin("Filter", b_filter));
    define(env, "mapply", builtin("mapply", b_mapply));
    define(env, "vapply", builtin("vapply", b_vapply));

    // More functional helpers (R-20) — build on the R-10 family above.
    define(env, "Find", builtin("Find", b_find));
    define(env, "Position", builtin("Position", b_position));
    define(env, "Negate", builtin("Negate", b_negate));
    define(env, "Recall", builtin("Recall", b_recall));

    // Apply-family & grouping (R-28) — pair the functional toolkit (R-10) with
    // matrices (R-11), lists (R-6), and factors (F4).
    define(env, "outer", builtin("outer", b_outer));
    define(env, "tapply", builtin("tapply", b_tapply));
    define(env, "split", builtin("split", b_split));
    define(env, "tabulate", builtin("tabulate", b_tabulate));

    // Binning & cross-product utilities (R-32) — build on the factor value (R-13)
    // and reuse `tabulate`'s allocation discipline.
    define(
        env,
        "findInterval",
        builtin("findInterval", b_find_interval),
    );
    define(env, "cut", builtin("cut", b_cut));

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

    // String utilities (R-34) — an independent string-utility family. All five
    // operate on Unicode `char`s (never raw byte indices) and reuse the existing
    // `as_character` coercion and `Option<String>`-NA convention.
    define(env, "startsWith", builtin("startsWith", b_starts_with));
    define(env, "endsWith", builtin("endsWith", b_ends_with));
    define(env, "trimws", builtin("trimws", b_trimws));
    define(env, "chartr", builtin("chartr", b_chartr));
    define(env, "strtoi", builtin("strtoi", b_strtoi));

    // Output formatting (R-27) — turn numbers and vectors into human-readable
    // text. Pure builtins, deterministic (locale-free) defaults.
    define(env, "format", builtin("format", b_format));
    define(env, "formatC", builtin("formatC", b_format_c));
    define(env, "prettyNum", builtin("prettyNum", b_pretty_num));
    define(env, "toString", builtin("toString", b_to_string));

    // v2 — apply family.
    define(env, "sapply", builtin("sapply", b_sapply));

    // v2 — S3 dispatch and output.
    define(env, "cat", builtin("cat", b_cat));

    // R-18 — error handling. `stop`/`warning` are ordinary (eager) builtins —
    // their arguments *are* evaluated and concatenated into the message. The
    // lazy `switch`/`tryCatch` are special forms handled in `eval.rs`, not here.
    // `conditionMessage` reads the message of a condition object.
    define(env, "stop", builtin("stop", b_stop));
    define(env, "warning", builtin("warning", b_warning));
    define(
        env,
        "conditionMessage",
        builtin("conditionMessage", b_condition_message),
    );

    define(env, "class", builtin("class", b_class));
    define(env, "structure", builtin("structure", b_structure));
    define(env, "inherits", builtin("inherits", b_inherits));
    define(env, "is", builtin("is", b_is));
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
    // R-35 — ordered factors.
    define(env, "ordered", builtin("ordered", b_ordered));
    define(env, "as.ordered", builtin("as.ordered", b_as_ordered));
    define(env, "is.ordered", builtin("is.ordered", b_is_ordered));
    define(env, "levels", builtin("levels", b_levels));
    define(env, "nlevels", builtin("nlevels", b_nlevels));
    define(env, "as.character", builtin("as.character", b_as_character));
    define(env, "as.integer", builtin("as.integer", b_as_integer));
    // R-44 — `as.numeric` (a base coercion; needed so `as.numeric(date)`
    // returns the raw days-since-epoch, and generally to drop a class to plain
    // numeric). `as.double` is R's exact synonym.
    define(env, "as.numeric", builtin("as.numeric", b_as_numeric));
    define(env, "as.double", builtin("as.double", b_as_numeric));

    // R-44 — base R Date support. A Date is days-since-epoch (1970-01-01)
    // carried by the transparent `SValue::Classed` wrapper with class "Date".
    define(env, "as.Date", builtin("as.Date", b_as_date));
    define(env, "Sys.Date", builtin("Sys.Date", b_sys_date));
    define(env, "format.Date", builtin("format.Date", b_format_date));
    define(env, "difftime", builtin("difftime", b_difftime));
    define(env, "weekdays", builtin("weekdays", b_weekdays));
    // R-45: month/quarter accessors. `seq.Date` is dispatched from within `seq`
    // when the first argument carries class "Date" (see `b_seq`).
    define(env, "months", builtin("months", b_months));
    define(env, "quarters", builtin("quarters", b_quarters));

    // R-46 — base R POSIXct date-times (UTC). A POSIXct is seconds-since-epoch
    // (1970-01-01 00:00:00 UTC) carried by the transparent `SValue::Classed`
    // wrapper with class c("POSIXct","POSIXt"); the date half reuses the R-44/R-45
    // civil kernel and field renderer. `as.numeric`/subtraction need no special
    // case (the wrapper is transparent → raw seconds).
    define(env, "as.POSIXct", builtin("as.POSIXct", b_as_posixct));
    define(env, "Sys.time", builtin("Sys.time", b_sys_time));
    define(
        env,
        "format.POSIXct",
        builtin("format.POSIXct", b_format_posixct),
    );

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
    // Matrix cross products (R-36): `t(x) %*% y` and `x %*% t(y)`. Both reuse
    // the `t()` transpose and the `%*%` product above — no new linear algebra.
    define(env, "crossprod", builtin("crossprod", b_crossprod));
    define(env, "tcrossprod", builtin("tcrossprod", b_tcrossprod));
    // Kronecker product (R-38): the (m*p)×(n*q) block-outer product. Reuses the
    // SValue::Matrix constructor + the MAX_SEQ_LEN cap (the `%x%` infix alias is
    // deferred to R-40, which needs grammar work).
    define(env, "kronecker", builtin("kronecker", b_kronecker));

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
    // Matrix norms (R-43): one-norm ("O"/"1"), infinity-norm ("I"), Frobenius/
    // Euclidean ("F"/"E"), and max-modulus ("M"). Reuses the shared matrix_parts
    // reader (rectangular matrices, not just square) and as_double for the
    // vector→1-column promotion. The spectral norm ("2", needs SVD) is deferred
    // to R-48 with a clear error.
    define(env, "norm", builtin("norm", b_norm));
    // Cholesky factorization (R-40): upper-triangular R with t(R) %*% R == X.
    // Reuses the `square_matrix` reader (shared with solve/det) and the
    // SValue::Matrix constructor. pivot=TRUE / chol2inv / complex deferred to R-41.
    define(env, "chol", builtin("chol", b_chol));
    // Triangular solves (R-41): backsolve solves an upper-triangular system,
    // forwardsolve a lower-triangular one, by back/forward substitution. Both
    // reuse `square_matrix` + the solve-style vector/matrix RHS handling; a zero
    // on the diagonal is a clean "singular" error. transpose=/k=/upper.tri=
    // deferred to R-42.
    define(env, "backsolve", builtin("backsolve", b_backsolve));
    define(env, "forwardsolve", builtin("forwardsolve", b_forwardsolve));
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
        // A named list (R-6 / R-28 `split`): report its element names. R renders an
        // *unset* element name as `""` rather than `NA`, and a list with *no* names
        // at all as `NULL`.
        SValue::List { names, .. } if names.iter().any(|n| n.is_some()) => Ok(SValue::Character(
            names
                .iter()
                .map(|n| Some(n.clone().unwrap_or_default()))
                .collect(),
        )),
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

/// Transpose a single value by reusing the public `t()` builtin (`b_t`). We wrap
/// `value` back into a one-element `Arg` slice — exactly the shape `b_t` expects
/// — so `crossprod`/`tcrossprod` get *identical* transpose semantics (matrix →
/// swapped dims; bare vector → `1×n` row matrix) with zero duplicated logic.
fn transpose_value(interp: &Interpreter, value: &SValue) -> SResult<SValue> {
    let one = [Arg {
        name: None,
        value: value.clone(),
    }];
    b_t(interp, &one)
}

/// `crossprod(x, y)` = `t(x) %*% y`; `crossprod(x)` (one argument) = `t(x) %*% x`.
///
/// This is the *Gram matrix* operation that shows up everywhere in statistics:
/// for a data matrix `X` whose columns are variables, `crossprod(X)` is the
/// (unscaled) `X'X` — the column-by-column dot products, the heart of a
/// least-squares normal equation `X'X b = X'y`.
///
/// ## Worked example (column-major, as R stores matrices)
///
/// `A = matrix(c(1, 2, 3, 4), nrow = 2)` is
///
/// ```text
///       col1 col2
/// row1   1    3
/// row2   2    4
/// ```
///
/// Its transpose `t(A)` is
///
/// ```text
///       col1 col2
/// row1   1    2
/// row2   3    4
/// ```
///
/// so `crossprod(A) = t(A) %*% A` =
///
/// ```text
/// [ 1*1+2*2  1*3+2*4 ]   [  5  11 ]
/// [ 3*1+4*2  3*3+4*4 ] = [ 11  25 ]
/// ```
///
/// ## Implementation
///
/// We do **not** reimplement multiply or transpose. We call the public `t()`
/// (`b_t`, via `transpose_value`) for the transpose and the evaluator's
/// `matrix_multiply` for the product. That means we inherit, for free:
///   * the `MAX_SEQ_LEN` allocation guard on `nrow * ncol` (no unchecked
///     multiply → OOM),
///   * the `"non-conformable arguments"` error when inner dims disagree,
///   * the column-major `array_runtime` fast path and NA propagation.
///
/// The second argument is optional and defaults to `x` (the one-argument form),
/// matching R. Inner dims always conform in the one-argument case because
/// `t(x)` has as many columns as `x` has rows.
fn b_crossprod(interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let x = first_positional(args)?;
    let y_owned;
    let y = match nth_positional(args, 1) {
        Some(y) => y,
        None => {
            // crossprod(x) ≡ crossprod(x, x): the second operand is x itself.
            y_owned = x.clone();
            &y_owned
        }
    };
    let xt = transpose_value(interp, x)?;
    interp.matrix_multiply(&xt, y)
}

/// `tcrossprod(x, y)` = `x %*% t(y)`; `tcrossprod(x)` (one argument) =
/// `x %*% t(x)`.
///
/// The "t" is for *transposed*: where `crossprod` transposes the **first**
/// operand, `tcrossprod` transposes the **second**. For a data matrix `X` whose
/// rows are observations, `tcrossprod(X)` is the `XX'` of pairwise row dot
/// products (a Gram matrix over observations rather than variables).
///
/// ## Worked example (same `A` as `crossprod`)
///
/// `tcrossprod(A) = A %*% t(A)` =
///
/// ```text
/// [ 1*1+3*3  1*2+3*4 ]   [ 10  14 ]
/// [ 2*1+4*3  2*2+4*4 ] = [ 14  20 ]
/// ```
///
/// As with `crossprod`, the multiply and transpose are *reused*, not rebuilt,
/// so the allocation guard, conformability error, and NA rules all carry over.
fn b_tcrossprod(interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let x = first_positional(args)?;
    let y_owned;
    let y = match nth_positional(args, 1) {
        Some(y) => y,
        None => {
            // tcrossprod(x) ≡ tcrossprod(x, x).
            y_owned = x.clone();
            &y_owned
        }
    };
    let yt = transpose_value(interp, y)?;
    interp.matrix_multiply(x, &yt)
}

/// Coerce a value to `(column-major data, nrow, ncol)` for the matrix builtins
/// that accept a bare vector. A `Matrix` keeps its shape; any other value is
/// read as a numeric vector and promoted to an `n × 1` **column** — the same
/// bare-column default `matrix(v)` itself uses (`matrix(c(1,2))` is 2×1). This
/// makes `kronecker(c(1,2), Y)` behave like `kronecker(matrix(c(1,2)), Y)`.
fn matrix_or_column(value: &SValue) -> SResult<(Double, usize, usize)> {
    match value {
        SValue::Matrix { data, nrow, ncol } => Ok((data.clone(), *nrow, *ncol)),
        other => {
            let d = other.as_double()?;
            let n = d.len();
            Ok((d, n, 1)) // bare vector → n×1 column, matching matrix(v)
        }
    }
}

/// `kronecker(X, Y)` — the **Kronecker product** (a.k.a. tensor/direct product).
///
/// For an `m × n` matrix `X` and a `p × q` matrix `Y`, the result is the
/// `(m·p) × (n·q)` matrix built from `m·n` blocks, where block `(i, j)` is the
/// scalar `X[i, j]` times the **whole** of `Y`:
///
/// ```text
///                 ┌                               ┐
///                 │  X[1,1]·Y   X[1,2]·Y  …  X[1,n]·Y │
///   X ⊗ Y   =     │  X[2,1]·Y   X[2,2]·Y  …  X[2,n]·Y │
///                 │     ⋮           ⋮     ⋱     ⋮     │
///                 │  X[m,1]·Y   X[m,2]·Y  …  X[m,n]·Y │
///                 └                               ┘
/// ```
///
/// ## Element formula (1-based, column-major to match `SValue::Matrix`)
///
/// `result[(i-1)·p + k, (j-1)·q + l] = X[i, j] · Y[k, l]`. Read the other way:
/// a result cell at row `r`, column `c` (both 0-based here) splits into
///   * the **outer** index into `X`:  `i = r / p`,  `j = c / q`  (integer div), and
///   * the **inner** index into `Y`:  `k = r % p`,  `l = c % q`.
///
/// We build the result column-by-column. The element at column-major offset
/// `c·(m·p) + r` is `X[i, j] · Y[k, l]` for those four decoded indices, where
/// `X[i, j]` lives at `X`'s offset `j·m + i` and `Y[k, l]` at `Y`'s offset
/// `l·p + k` (both stores are column-major).
///
/// ## Worked example
///
/// `X = matrix(c(1,2,3,4), nrow=2)` is `[[1,3],[2,4]]`; `Y = matrix(c(0,1,1,0),
/// nrow=2)` is `[[0,1],[1,0]]`. `kronecker(X, Y)` is the 4×4 matrix whose
/// top-left block is `1·Y`, top-right `3·Y`, bottom-left `2·Y`, bottom-right
/// `4·Y`. A 1×1 `X = matrix(5)` gives `5·Y`.
///
/// ## Vectors
///
/// A bare numeric vector is promoted to an `n × 1` column (the `matrix(v)`
/// default), via [`matrix_or_column`].
///
/// ## Security — the quadratic output-size guard
///
/// The result has `(m·p)·(n·q)` elements, **quadratic** in the inputs: two
/// 100×100 matrices Kronecker to a 10⁸-element matrix. So *before* allocating
/// we form the result row count `m·p`, column count `n·q`, **and** their product
/// with `checked_mul`, and reject anything exceeding `MAX_SEQ_LEN` — the same
/// cap `matrix()` and `matrix_multiply` enforce. An overflow or over-cap result
/// returns a "result too large" error and never allocates. Degenerate inputs
/// (`m=0`, `p=0`, …) yield a `0`-element result with a correct zero dimension;
/// the loops below simply do not execute, so there is no out-of-bounds risk.
///
/// The R infix alias `%x%` (i.e. `X %x% Y`) is **deferred to R-40** (it needs
/// lexer/grammar work for the special operator); this builtin is the function
/// form only.
fn b_kronecker(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let x = first_positional(args)?;
    let y = nth_positional(args, 1)
        .ok_or_else(|| SError::BadArgs("kronecker: needs two arguments (X, Y)".into()))?;
    let (xd, m, n) = matrix_or_column(x)?;
    let (yd, p, q) = matrix_or_column(y)?;

    // Result is (m*p) × (n*q). Guard each result dimension AND the total element
    // count with checked_mul against MAX_SEQ_LEN, before any allocation.
    let too_large =
        || SError::Index(format!("kronecker: result too large (limit {MAX_SEQ_LEN} elements)"));
    let rows = m.checked_mul(p).filter(|&t| t <= MAX_SEQ_LEN).ok_or_else(too_large)?;
    let cols = n.checked_mul(q).filter(|&t| t <= MAX_SEQ_LEN).ok_or_else(too_large)?;
    let total = rows
        .checked_mul(cols)
        .filter(|&t| t <= MAX_SEQ_LEN)
        .ok_or_else(too_large)?;

    let xs = xd.data();
    let ys = yd.data();
    let mut out = vec![0.0; total];
    // Walk the result column-major. For each output cell (r, c):
    //   outer X index (i, j) = (r / p, c / q); inner Y index (k, l) = (r % p, c % q).
    // X[i,j] is at j*m + i (column-major); Y[k,l] is at l*p + k.
    for c in 0..cols {
        let j = c / q; // outer column → X column
        let l = c % q; // inner column → Y column
        for r in 0..rows {
            let i = r / p; // outer row → X row
            let k = r % p; // inner row → Y row
            let xv = xs[j * m + i];
            let yv = ys[l * p + k];
            out[c * rows + r] = na_mul(xv, yv);
        }
    }
    Ok(SValue::Matrix {
        data: Double::from_values(out),
        nrow: rows,
        ncol: cols,
    })
}

/// Multiply two doubles, propagating R's `NA` (a specific NaN bit pattern): if
/// either operand is `NA_real_`, the product is `NA_real_`, exactly as R's
/// arithmetic does — so an `NA` anywhere in `X` or `Y` shows up in the product.
fn na_mul(a: f64, b: f64) -> f64 {
    if is_na_real(a) || is_na_real(b) {
        na_real()
    } else {
        a * b
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

/// `norm(x, type = "O")` — a **matrix norm**: one number measuring the "size" of
/// a numeric matrix `x`. `type` is a one-letter, case-insensitive string picking
/// *which* norm; each is a different way of collapsing all the `|x[i,j]|` into a
/// single non-negative scalar:
///
/// ```text
///   type            name              formula
///   ----            ----              -------
///   "O" / "1"       one-norm          max over columns j of  Σ_i |x[i,j]|
///   "I"             infinity-norm     max over rows    i of  Σ_j |x[i,j]|
///   "F" / "E"       Frobenius/        sqrt( Σ_{i,j} x[i,j]² )
///                   Euclidean
///   "M"             max-modulus       max_{i,j} |x[i,j]|
/// ```
///
/// Intuition: the one-norm asks "which **column** is biggest (in absolute sum)?",
/// the infinity-norm asks the same of **rows**, the Frobenius norm treats the
/// matrix as one long vector and takes its Euclidean length, and the max-modulus
/// is simply the largest single entry by magnitude.
///
/// `x` may also be a plain numeric **vector**, which R (and we) treat as an
/// `n × 1` (single-**column**) matrix. So `norm(c(3,4), "F")` is `sqrt(3²+4²) = 5`
/// (the 3-4-5 right triangle), `norm(c(3,4), "O")` is the lone column's absolute
/// sum `7`, and `norm(c(3,4), "I")` is the largest single-element row `4`.
///
/// **Reuse.** Dimensions + data come from the shared [`matrix_parts`] reader
/// (`(data, nrow, ncol)`, column-major), *not* `square_matrix` — norms apply to
/// rectangular matrices too. The vector case promotes through `as_double`, and
/// `type =` is read with the same named-or-positional string convention as other
/// builtins (`as_character` of the first non-`x` argument). The result is a
/// `SValue::scalar`.
///
/// **Safety / NA.** Any `NA` entry makes the result `NA` (base R propagates `NA`
/// through these reductions). An unknown `type` letter is a clean `BadArgs`
/// error — never a panic. An empty / 0-row / 0-column matrix does not panic: the
/// reductions start from `0`. The Frobenius sum-of-squares accumulates in `f64`,
/// so no `MAX_SEQ_LEN`-legal matrix of finite entries can overflow.
///
/// **Deferred.** `type = "2"` is the *spectral* norm (the largest singular value);
/// it needs an SVD and is deferred to **R-48**. For now it is a clear error
/// rather than a wrong number.
fn b_norm(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    // --- 1. Read x as (data, nrow, ncol), promoting a bare vector to n×1. ---
    let x = first_positional(args)?;
    let (data, nrow, ncol): (Vec<f64>, usize, usize) = match matrix_parts(x) {
        Some((d, nr, nc)) => (d.data().to_vec(), nr, nc),
        // A non-matrix numeric value → a single column (n rows, 1 column).
        None => {
            let d = x.as_double()?;
            let n = d.len();
            (d.data().to_vec(), n, 1)
        }
    };

    // --- 2. NA anywhere ⇒ NA (matches base R for all of these reductions). ---
    if data.iter().any(|v| is_na_real(*v)) {
        return Ok(SValue::scalar(na_real()));
    }

    // --- 3. Read the `type` letter. It may be positional (the 2nd positional
    // argument) or named `type =`; default "O" (R's default). Lower-case it so
    // matching is case-insensitive, then look at just the first character. ---
    let type_value = named_arg(args, "type").or_else(|| nth_positional(args, 1));
    let type_str: String = match type_value {
        Some(v) => match v.as_character().into_iter().next().flatten() {
            // An explicit NA / empty `type` falls back to the default, as in R.
            Some(s) if !s.is_empty() => s,
            _ => "O".to_string(),
        },
        None => "O".to_string(),
    };
    let kind = type_str.trim().to_ascii_uppercase();
    let first = kind.chars().next().unwrap_or('O');

    // --- 4. Dispatch on the (upper-cased) first letter. Column-major data: the
    // element at row i, column j lives at `j * nrow + i`. ---
    let result = match first {
        // One-norm: maximum absolute column sum. An empty matrix ⇒ 0.
        'O' | '1' => {
            let mut best = 0.0_f64;
            for j in 0..ncol {
                let mut col_sum = 0.0_f64;
                for i in 0..nrow {
                    col_sum += data[j * nrow + i].abs();
                }
                if col_sum > best {
                    best = col_sum;
                }
            }
            best
        }
        // Infinity-norm: maximum absolute row sum.
        'I' => {
            let mut best = 0.0_f64;
            for i in 0..nrow {
                let mut row_sum = 0.0_f64;
                for j in 0..ncol {
                    row_sum += data[j * nrow + i].abs();
                }
                if row_sum > best {
                    best = row_sum;
                }
            }
            best
        }
        // Frobenius / Euclidean: sqrt of the sum of squares of every entry.
        'F' | 'E' => {
            let mut ss = 0.0_f64;
            for &v in &data {
                ss += v * v;
            }
            ss.sqrt()
        }
        // Max-modulus: the largest absolute entry. Empty ⇒ 0.
        'M' => data.iter().fold(0.0_f64, |m, &v| m.max(v.abs())),
        // The spectral norm (largest singular value) needs an SVD; deferred.
        '2' => {
            return Err(SError::BadArgs(
                "norm: type '2' (spectral) not yet supported".into(),
            ));
        }
        // Anything else is an unrecognised norm type → a clean error.
        _ => {
            return Err(SError::BadArgs(format!(
                "norm: 'type' must be one of \"O\"/\"1\", \"I\", \"F\"/\"E\", \"M\" (got {type_str:?})"
            )));
        }
    };
    Ok(SValue::scalar(result))
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

/// `chol(x)` — the **Cholesky factorization** of a real symmetric
/// positive-definite matrix `x` (R-40).
///
/// ## What it returns
///
/// The **upper-triangular** matrix `R` such that `t(R) %*% R == x`. This is R's
/// convention: `chol` returns the upper factor, so `R'R = X` (some texts return
/// the *lower* factor `L` with `L L' = X` — note the difference). For
///
/// ```text
///       | 4  2 |                 | 2   1 |
///   X = |      |   →   chol(X) = |       |   because t(R) %*% R = X.
///       | 2  3 |                 | 0  √2 |
/// ```
///
/// ## Algorithm — Cholesky–Banachiewicz, upper form
///
/// Walking columns `i = 0 … n-1` (0-based here; the spec is 1-based):
///
/// ```text
///   R[i][i] = sqrt( X[i][i] − Σ_{k<i} R[k][i]² )
///   R[i][j] = ( X[i][j] − Σ_{k<i} R[k][i]·R[k][j] ) / R[i][i]     (j > i)
///   R[i][j] = 0                                                   (j < i)
/// ```
///
/// We read **only the upper triangle** of `X` (`X[i][j]` for `i ≤ j`), exactly as
/// R's default `chol` does — the strictly-lower triangle is never touched, so an
/// asymmetric lower triangle is silently ignored.
///
/// ## Column-major indexing
///
/// Like every `SValue::Matrix`, both `X` and the result are stored column-major:
/// element `(row, col)` lives at offset `col·n + row`. So `X[i][j]` is
/// `x[j*n + i]` and we write `R[i][j]` to `out[j*n + i]`.
///
/// ## Faithful error handling (no panic, no NaN)
///
/// * **Non-square / non-matrix / over-cap** `x` → the shared `square_matrix`
///   helper (used by `det`/`solve`) raises the error *before* we index anything.
/// * **`NA` in the upper triangle** → a clean error (`NA` cannot be factored).
/// * **Not positive-definite** → if the diagonal pivot
///   `X[i][i] − Σ_{k<i} R[k][i]²` is `≤ 0` (or non-finite), `X` is not SPD. We
///   test this **before** calling `sqrt`, so we never take the square root of a
///   negative number — the result is R's exact error *"the leading minor of order
///   i is not positive definite"*, never a propagated `NaN` and never a panic.
// The `!(pivot > 0.0)` form is a deliberate NaN-safe guard (see the inline
// comment): rewriting as `pivot <= 0.0` would let NaN through. Behavior-preserving allow.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
fn b_chol(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    // Reuse the det/solve square-matrix reader: it rejects non-matrix, non-square
    // and over-MAX_SOLVE_DIM inputs up front and returns column-major data + n.
    let (x, n) = square_matrix(first_positional(args)?, "chol")?;

    // The 0×0 matrix factors to the 0×0 matrix (vacuously, t(R) %*% R == X).
    // The allocation is the single n×n result, bounded by MAX_SOLVE_DIM (n ≤ 1000
    // ⇒ n² ≤ 10⁶), the same order cap square_matrix already enforces.
    let mut out = vec![0.0; n * n];
    for i in 0..n {
        // Diagonal: X[i][i] − Σ_{k<i} R[k][i]².  R[k][i] is out[i*n + k].
        if is_na_real(x[i * n + i]) {
            return Err(SError::BadArgs("chol: NA in 'a'".into()));
        }
        let mut pivot = x[i * n + i];
        for k in 0..i {
            let rki = out[i * n + k];
            pivot -= rki * rki;
        }
        // The ≤ 0 (or non-finite) check MUST precede sqrt — a non-SPD matrix is a
        // clean error here, never sqrt of a negative (NaN) and never a panic.
        if !(pivot > 0.0) || !pivot.is_finite() {
            return Err(SError::BadArgs(format!(
                "chol: the leading minor of order {} is not positive definite",
                i + 1
            )));
        }
        let diag = pivot.sqrt();
        out[i * n + i] = diag;

        // Off-diagonal, upper triangle only (j > i):
        //   R[i][j] = ( X[i][j] − Σ_{k<i} R[k][i]·R[k][j] ) / R[i][i].
        for j in (i + 1)..n {
            if is_na_real(x[j * n + i]) {
                return Err(SError::BadArgs("chol: NA in 'a'".into()));
            }
            let mut s = x[j * n + i]; // X[i][j], reading the UPPER triangle
            for k in 0..i {
                s -= out[i * n + k] * out[j * n + k]; // R[k][i]·R[k][j]
            }
            out[j * n + i] = s / diag;
        }
        // Sub-diagonal entries (j < i) stay 0 — `out` was zero-initialized.
    }

    Ok(SValue::Matrix {
        data: Double::from_values(out),
        nrow: n,
        ncol: n,
    })
}

/// `backsolve(r, x)` / `forwardsolve(l, x)` — solve a TRIANGULAR linear system
/// `R %*% y = x` (`backsolve`, `r` upper-triangular) or `L %*% y = x`
/// (`forwardsolve`, `l` lower-triangular) for `y`, by back/forward substitution.
///
/// Only the relevant triangle of the coefficient matrix is read. The right-hand
/// side `x` is either a length-`n` vector (→ a vector result) or an `n × m`
/// matrix (→ an `n × m` result, one solved column per right-hand side) — the
/// same shape contract as [`b_solve`]. A zero on the diagonal makes the system
/// singular: a clean error, never a divide-by-zero `NaN`/`Inf` or a panic.
///
/// Worked example (`backsolve`, upper-triangular, column-major):
///   R = [[2,1],[0,3]],  x = (5, 9)
///     y[2] = 9 / 3         = 3
///     y[1] = (5 − 1·3) / 2 = 1      ⇒  y = (1, 3),  and  R %*% y == x.
///
/// # R-42 — named options (`k` / `upper.tri` / `transpose`)
///
/// The bare two-argument form above is the R-41 default. R-42 threads three
/// base-R named options through this *same* helper; the resolved values arrive
/// as parameters so the per-builtin defaults live in the thin wrappers.
///
/// * **`upper_tri`** — which triangle of the first argument to read. Reading the
///   **upper** triangle ⇒ **back**-substitution (rows bottom-up); reading the
///   **lower** triangle ⇒ **forward**-substitution (rows top-down). So the
///   substitution direction *follows the triangle read*.
/// * **`transpose`** — when `true`, solve `t(R) %*% y = x`. Transposing a
///   triangular factor swaps which triangle is active, so it **flips** the
///   direction; and every coefficient read goes through the transposed
///   column-major index (`R[j,i]` at `i·n + j`) instead of `R[i,j]` at `j·n + i`.
///   The combined rule: the effective direction is **back**-substitution iff
///   `upper_tri != transpose`.
/// * **`k`** — solve only the **leading `k×k` block** of the (stride-`n`,
///   column-major) factor against the **first `k` rows** of the RHS; the result
///   has `k` rows. Indexing keeps the full stride `n`, so no data is copied — the
///   loops simply range over `0..k`. `k` must satisfy `0 ≤ k ≤ n`; an
///   out-of-range `k` is a clean error, never an out-of-bounds read.
///
/// The **diagonal** entry `a[i·n + i]` is the same with or without transpose, so
/// the zero-on-the-used-diagonal *singular* check is unchanged — a clean error,
/// never a propagated `NaN`/`Inf` and never a panic.
fn triangular_solve(
    args: &[Arg],
    who: &str,
    upper_tri_default: bool,
) -> SResult<SValue> {
    // The coefficient matrix: reuse the det/solve/chol square-matrix reader, which
    // rejects non-matrix, non-square and over-MAX_SOLVE_DIM inputs up front and
    // returns the data column-major (entry (row i, col j) is at index `j*n + i`).
    let (a, n) = square_matrix(first_positional(args)?, who)?;
    if a.iter().any(|v| is_na_real(*v)) {
        return Err(SError::BadArgs(format!(
            "{who}: NA in the coefficient matrix"
        )));
    }

    // --- R-42 named options -------------------------------------------------
    // `upper.tri =` (which triangle) and `transpose =` (solve t(R)) are logicals;
    // missing ⇒ the per-builtin default / `FALSE`. Reuse the shared `named_flag`.
    let upper_tri = named_flag(args, "upper.tri", upper_tri_default)?;
    let transpose = named_flag(args, "transpose", false)?;
    // The substitution runs back-substitution (rows bottom-up) iff the triangle
    // read and the transpose flag disagree (see the doc comment's truth table).
    let back = upper_tri != transpose;

    // `k =` selects the leading `k×k` block + first `k` RHS rows. Default = `n`
    // (the full matrix). Read it as a logical-free integer; a present-but-NA `k`
    // also falls back to `n`. Range-check `0 ≤ k ≤ n` BEFORE any indexing so a
    // malformed `k` is a clean error, never an out-of-bounds read.
    let k = match named_arg(args, "k") {
        Some(v) => {
            let raw = v.as_double()?;
            match raw.get_value(0) {
                Some(x) if !is_na_real(x) => {
                    // R truncates toward zero; reject anything outside `0..=n`.
                    if !(0.0..=(n as f64)).contains(&x.trunc()) {
                        return Err(SError::BadArgs(format!(
                            "{who}: 'k' must be between 0 and {n} (got {x})"
                        )));
                    }
                    x.trunc() as usize
                }
                _ => n, // empty / NA `k` ⇒ the full matrix, as in R.
            }
        }
        None => n,
    };

    // The right-hand side `x`: a matrix (→ matrix result, `m` columns) or a
    // vector (→ vector result, a single column). Same handling as `solve`. The
    // RHS must have `n` rows/length (the full factor); we then use the first `k`.
    let (b, m, b_is_vector) = match nth_positional(args, 1) {
        Some(x_val) => {
            if let Some((bd, bnr, bnc)) = matrix_parts(x_val) {
                if bnr != n {
                    return Err(SError::BadArgs(format!(
                        "{who}: the right-hand side must have {n} rows (got {bnr})"
                    )));
                }
                (bd.data().to_vec(), bnc, false)
            } else {
                let bd = x_val.as_double()?;
                if bd.len() != n {
                    return Err(SError::BadArgs(format!(
                        "{who}: the right-hand side must have length {n} (got {})",
                        bd.len()
                    )));
                }
                (bd.data().to_vec(), 1, true)
            }
        }
        None => {
            return Err(SError::BadArgs(format!(
                "{who}: missing the right-hand side 'x'"
            )))
        }
    };
    if b.iter().any(|v| is_na_real(*v)) {
        return Err(SError::BadArgs(format!(
            "{who}: NA in the right-hand side"
        )));
    }
    // The substitution is O(k²·m); cap the column count like `solve` does so a
    // wide right-hand side can't blow past the MAX_SOLVE_DIM work budget.
    if m > MAX_SOLVE_DIM {
        return Err(SError::Index(format!(
            "{who}: too many right-hand sides ({m}; limit {MAX_SOLVE_DIM})"
        )));
    }

    // Coefficient read for "equation i, unknown j" within the leading `k×k`
    // block. Without transpose this is `A[i][j]` at `j*n + i`; with transpose we
    // want `t(A)[i][j] = A[j][i]` at `i*n + j`. Both keep the full stride `n`, so
    // `i, j < k ≤ n` stays in bounds. The `n` stride is captured by the closure.
    let coef = |i: usize, j: usize| -> f64 {
        if transpose {
            a[i * n + j]
        } else {
            a[j * n + i]
        }
    };

    // The result has `k` rows (one solved entry per active unknown), `m` columns.
    // Solve each RHS column independently. Visit the rows in the order that makes
    // the already-solved entries available: bottom-up for back-substitution,
    // top-down for forward-substitution. We index the RHS row-wise within the
    // first `k` rows of each length-`n` column (`col * n + i`, `i < k`) and write
    // the packed `k`-row result (`col * k + i`).
    let mut y = vec![0.0; k * m];
    for col in 0..m {
        // `rows()` yields the active row indices `0..k` in substitution order.
        let order: Vec<usize> = if back {
            (0..k).rev().collect()
        } else {
            (0..k).collect()
        };
        for &i in &order {
            // Subtract the already-solved unknowns. For back-substitution those
            // are the rows below (`j > i`); for forward-substitution, above
            // (`j < i`). Both ranges stay within `0..k`.
            let mut s = b[col * n + i];
            if back {
                for j in (i + 1)..k {
                    s -= coef(i, j) * y[col * k + j];
                }
            } else {
                for j in 0..i {
                    s -= coef(i, j) * y[col * k + j];
                }
            }
            // The diagonal is transpose-invariant (`a[i*n + i]`); a zero here
            // makes the system singular — a clean error before the division.
            let diag = a[i * n + i];
            if diag == 0.0 {
                return Err(SError::BadArgs(format!(
                    "{who}: the matrix is singular (zero on the diagonal, position {})",
                    i + 1
                )));
            }
            y[col * k + i] = s / diag;
        }
    }

    if b_is_vector {
        Ok(SValue::doubles(y))
    } else {
        Ok(SValue::Matrix {
            data: Double::from_values(y),
            nrow: k,
            ncol: m,
        })
    }
}

/// `backsolve(r, x, k = ncol(r), upper.tri = TRUE, transpose = FALSE)` —
/// solve the UPPER-triangular system `r %*% y = x` (the default), honouring the
/// R-42 named options. See [`triangular_solve`].
fn b_backsolve(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    triangular_solve(args, "backsolve", true)
}

/// `forwardsolve(l, x, k = ncol(l), upper.tri = FALSE, transpose = FALSE)` —
/// solve the LOWER-triangular system `l %*% y = x` (the default), honouring the
/// R-42 named options. See [`triangular_solve`].
fn b_forwardsolve(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    triangular_solve(args, "forwardsolve", false)
}

// ===========================================================================
// v2 — factors
// ===========================================================================

/// `factor(x, levels =, labels =, ordered =)` — encode `x` as a factor. Levels
/// default to the sorted unique non-`NA` values of `x`; `labels` (if given) rename
/// them. **R-35:** `ordered = TRUE` makes the result an *ordered* factor (see
/// [`build_factor`]); the default is an unordered factor.
fn b_factor(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    // `ordered =` is a logical flag (default FALSE). A malformed value surfaces as
    // a clean error via `truthy` rather than a panic.
    let ordered = named_flag(args, "ordered", false)?;
    build_factor(args, ordered)
}

/// The shared factor builder used by `factor` (R-13) and `ordered` (R-35). Reads
/// the first positional argument as the data, the `levels =` / `labels =` named
/// arguments exactly as `factor` does, and stamps the supplied `ordered` flag onto
/// the result. Centralising this keeps `ordered()` from re-deriving the level
/// inference and code assignment.
fn build_factor(args: &[Arg], ordered: bool) -> SResult<SValue> {
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
        ordered,
    })
}

/// `ordered(x, levels =, labels =)` — build an **ordered** factor (R-35): a factor
/// whose levels carry a meaningful order, so its elements compare by level index.
/// Identical to `factor` (it reuses [`build_factor`]) but with `ordered = true`.
fn b_ordered(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    build_factor(args, true)
}

/// `as.ordered(x)` — coerce `x` to an ordered factor (R-35). An existing factor
/// (ordered or not) keeps its codes/levels and gains the ordered flag; any other
/// value is first encoded with [`build_factor`] (sorted-unique levels).
fn b_as_ordered(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    match peel_structural(first_positional(args)?) {
        SValue::Factor { codes, levels, .. } => Ok(SValue::Factor {
            codes: codes.clone(),
            levels: levels.clone(),
            ordered: true,
        }),
        // Not already a factor: encode it, then mark ordered. We reuse the same
        // single positional argument (the data) through `build_factor`.
        _ => build_factor(args, true),
    }
}

/// `is.ordered(x)` — `TRUE` iff `x` is an ordered factor (R-35); `FALSE` for an
/// unordered factor or any non-factor. Never errors.
fn b_is_ordered(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let ordered = matches!(
        peel_structural(first_positional(args)?),
        SValue::Factor { ordered: true, .. }
    );
    Ok(SValue::Logical(vec![Some(ordered)]))
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

/// `as.numeric(x)` / `as.double(x)` — coerce to a plain numeric vector, **dropping
/// any class** (R-44). For a `Date` this returns its raw days-since-epoch: the
/// `Classed` wrapper is transparent to `as_double`, so we simply re-wrap the
/// coerced doubles as a bare `Double` (the class is gone). A factor coerces by its
/// integer codes — matching `as.numeric(factor(...))` in R, which returns the
/// codes, not the labels.
fn b_as_numeric(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    match first_positional(args)? {
        // A factor's numeric form is its 1-based codes (NA-preserving), exactly
        // like `as.integer` — never the labels (those would be NA under as.double).
        SValue::Factor { codes, .. } => Ok(SValue::doubles(
            codes
                .iter()
                .map(|c| c.map(|k| k as f64).unwrap_or_else(na_real))
                .collect(),
        )),
        other => Ok(SValue::Double(other.as_double()?)),
    }
}

// ===========================================================================
// R-44 — base R Date support
// ===========================================================================
//
// In R a `Date` is *not* a distinct value kind: it is an ordinary numeric vector
// holding the count of **days since the Unix epoch 1970-01-01**, carrying the S3
// class attribute "Date". We model it with the existing transparent
// `SValue::Classed { inner: Double, class: ["Date"] }` wrapper — no new SValue
// variant — so every coercion (`as_double`/`as_character`) and the `arithmetic`
// kernel already see straight through to the day count. The only Date-aware code
// is parsing, rendering, and the small civil-date kernel below.
//
//   day 0  = 1970-01-01 (a Thursday)
//   day 1  = 1970-01-02
//   day -1 = 1969-12-31
//
// ---------------------------------------------------------------------------
// The civil-date kernel (Howard Hinnant's algorithms — no new dependency)
// ---------------------------------------------------------------------------
//
// These two pure functions convert between a (year, month, day) civil date and a
// signed day count relative to the epoch. They implement the **proleptic
// Gregorian calendar** (the Gregorian rules projected backward through all of
// history, including before 1582), and are exact inverses of one another. The
// trick is to shift the start of the year to **March**, so the leap day
// (February 29) lands at the *end* of the year-of-era and never disturbs the
// month-length pattern; an "era" is a 400-year cycle (exactly 146097 days), which
// is the calendar's repeat period.

/// Days from the civil date `(y, m, d)` to the Unix epoch 1970-01-01.
///
/// Hinnant's `days_from_civil`. `i64` throughout so distant or pre-epoch years can
/// never overflow within the (bounded) year range the parser admits. Examples:
/// `(1970,1,1) → 0`, `(1970,1,2) → 1`, `(1969,12,31) → -1`, `(2000,2,29) → 11016`.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    // Shift the year so it starts in March: Jan/Feb belong to the *previous* year.
    let y = if m <= 2 { y - 1 } else { y };
    // The era is the 400-year cycle this year falls in (floored toward -inf).
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400; // year of era, 0..=399
                             // Day of year counting from March 1 (mar=0 … feb=364/365).
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    // Day of era, 0..=146096.
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    // 719468 = days from 0000-03-01 to 1970-01-01 — shifts the result to the epoch.
    era * 146097 + doe - 719468
}

/// The civil date `(y, m, d)` for a day count `z` relative to 1970-01-01.
///
/// Hinnant's `civil_from_days` — the exact inverse of [`days_from_civil`]. Examples:
/// `0 → (1970,1,1)`, `-1 → (1969,12,31)`, `11016 → (2000,2,29)`.
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468; // re-base onto 0000-03-01
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097; // day of era, 0..=146096
                                // Year of era, 0..=399.
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // day of year (Mar-based), 0..=365
    let mp = (5 * doy + 2) / 153; // month, shifted (0 = March … 11 = February)
    let d = doy - (153 * mp + 2) / 5 + 1; // day of month, 1..=31
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // un-shift to 1..=12
    let y = if m <= 2 { y + 1 } else { y }; // Jan/Feb roll into the next civil year
    (y, m, d)
}

// ---------------------------------------------------------------------------
// R-45: English month / weekday name tables (hand-rolled — no new dependency).
// ---------------------------------------------------------------------------
//
// These back `%B`/`%b`/`%A`/`%a` in both rendering (`format.Date`) and parsing
// (`as.Date`), plus `months()`. All names are English; locale-specific names are
// deferred to R-46. The full and abbreviated forms are kept in parallel arrays so
// the abbreviation is always the first three letters — which it is for every
// English month and weekday — but we store both explicitly for clarity and so a
// future irregular abbreviation needs no special-casing.

/// Full month names, indexed by `month - 1` (so `MONTHS_FULL[0]` is January).
const MONTHS_FULL: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// Abbreviated month names, indexed by `month - 1` (`MONTHS_ABBR[0]` = "Jan").
const MONTHS_ABBR: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Full weekday names, **Sunday-based** (index 0 = Sunday) to match the weekday
/// index `(days + 4).rem_euclid(7)` (day 0 = Thursday = index 4).
const WEEKDAYS_FULL: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

/// Abbreviated weekday names, Sunday-based (index 0 = "Sun").
const WEEKDAYS_ABBR: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

/// The Sunday-based weekday index (0..7) of a day count `z`. The anchor is
/// `1970-01-01` (day 0) = Thursday = index 4; `rem_euclid` keeps the result in
/// `0..7` even for negative (pre-epoch) `z` (Rust's `%` can return a negative
/// remainder, which would panic on array indexing). Shared by `%A`/`%a` and the
/// `weekdays` builtin.
fn weekday_index(z: i64) -> usize {
    (z + 4).rem_euclid(7) as usize
}

/// Match `input` (starting at char index `idx`) against a fixed name `table`,
/// **case-insensitively** over ASCII, longest-name-first so e.g. "June" is not
/// shadowed by a hypothetical "Jun" prefix. On a match, advance `idx` past the
/// consumed name and return the table position (0-based); on no match, leave
/// `idx` and return `None` (→ the whole parse fails → NA). Never indexes out of
/// bounds: the comparison is length-checked against the remaining input, and
/// ASCII case-folding (`eq_ignore_ascii_case`) is byte-safe on the `char` slice.
fn match_name(input: &[char], idx: &mut usize, table: &[&str]) -> Option<usize> {
    // Try longer names before shorter ones so a short name that is a prefix of a
    // longer one (none in the English tables, but defensive) cannot win early.
    let mut order: Vec<usize> = (0..table.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(table[i].len()));
    for &pos in &order {
        let name: Vec<char> = table[pos].chars().collect();
        let end = idx.checked_add(name.len())?;
        if end > input.len() {
            continue; // not enough input left for this name
        }
        // Case-insensitive ASCII comparison, char by char (bounded by name.len()).
        let matches = input[*idx..end]
            .iter()
            .zip(name.iter())
            .all(|(a, b)| a.eq_ignore_ascii_case(b));
        if matches {
            *idx = end;
            return Some(pos);
        }
    }
    None
}

/// The largest number of decimal digits we will accumulate for any single date
/// field while parsing. A year is at most 4–6 digits in practice; capping the run
/// length means a crafted string of a million '9's can never overflow the `i64`
/// accumulation or hang — it simply fails to parse (→ NA). 9 digits keeps `i64`
/// far from overflow while still admitting any realistic year.
const MAX_DATE_DIGITS: usize = 9;

/// The largest magnitude (in days) we admit for a `Date`'s day count. Beyond this
/// the civil-date kernel's internal multiplications/additions (`z + 719468`,
/// `era * 146097`, `z - days_from_civil(y, 1, 1)`) could approach `i64` overflow,
/// and a `weekdays`/`format` call would panic (debug) or wrap to a nonsense date
/// (release). ±1e11 days is year ≈ ±270 million — astronomically beyond any real
/// use — yet keeps every kernel operation comfortably inside `i64`. This is the
/// numeric counterpart to [`MAX_DATE_DIGITS`]: the string parser caps the *year*,
/// this caps a directly-supplied *day count* (`as.Date(1e300)`), so **neither**
/// untrusted path can drive an out-of-range `z` into the kernel.
const MAX_DATE_DAYS: i64 = 100_000_000_000;

/// Clamp-or-reject a raw day count: `Some(z)` if it is finite and within
/// [`MAX_DATE_DAYS`], else `None` (→ NA). Used at every boundary where an
/// untrusted `f64` becomes a Date day count, so the civil kernel only ever sees
/// in-range values and can never overflow.
fn checked_date_days(v: f64) -> Option<i64> {
    if is_na_real(v) || !v.is_finite() {
        return None;
    }
    let z = v.trunc();
    if z.abs() > MAX_DATE_DAYS as f64 {
        None
    } else {
        Some(z as i64)
    }
}

/// Is `x` a Date — a value whose (explicit) class vector contains "Date"?
fn is_date(x: &SValue) -> bool {
    class_of(x).iter().any(|c| c == "Date")
}

/// Wrap a vector of day counts (NA-aware doubles) as a `Date` — class "Date" over
/// a plain `Double`. This is the single constructor every Date-producing builtin
/// funnels through.
fn make_date(days: Vec<f64>) -> SValue {
    SValue::Classed {
        inner: Box::new(SValue::doubles(days)),
        class: vec!["Date".to_string()],
    }
}

/// Parse one non-negative integer field of at most [`MAX_DATE_DIGITS`] digits from
/// `chars` starting at `idx`, advancing `idx` past it. Returns `None` (→ the whole
/// parse fails → NA) on no digits or an over-long run. Accumulates in `i64` so it
/// cannot overflow within the digit cap.
fn parse_uint_field(chars: &[char], idx: &mut usize) -> Option<i64> {
    let start = *idx;
    let mut val: i64 = 0;
    while *idx < chars.len() && chars[*idx].is_ascii_digit() {
        if *idx - start >= MAX_DATE_DIGITS {
            return None; // absurdly long run — refuse rather than risk overflow
        }
        val = val * 10 + (chars[*idx] as i64 - '0' as i64);
        *idx += 1;
    }
    if *idx == start {
        None // no digits where a number was required
    } else {
        Some(val)
    }
}

/// Parse one date `string` against a `format` pattern (supporting `%Y`, `%m`, `%d`
/// and literal characters), returning days-since-epoch — or `None` (→ NA) on any
/// mismatch, missing field, or out-of-range value. Never panics on crafted input.
fn parse_date_str(string: &str, format: &str) -> Option<i64> {
    let chars: Vec<char> = string.chars().collect();
    let fmt: Vec<char> = format.chars().collect();
    let (mut ci, mut fi) = (0usize, 0usize);
    // We require year + month + day to all appear; track each as we read them.
    let (mut year, mut month, mut day): (Option<i64>, Option<i64>, Option<i64>) =
        (None, None, None);

    while fi < fmt.len() {
        if fmt[fi] == '%' && fi + 1 < fmt.len() {
            match fmt[fi + 1] {
                'Y' => year = Some(parse_uint_field(&chars, &mut ci)?),
                'm' => month = Some(parse_uint_field(&chars, &mut ci)?),
                'd' => day = Some(parse_uint_field(&chars, &mut ci)?),
                // %e is a space-padded day-of-month: a single optional leading
                // space, then the digits. Skip the pad, then read the number.
                'e' => {
                    if ci < chars.len() && chars[ci] == ' ' {
                        ci += 1;
                    }
                    day = Some(parse_uint_field(&chars, &mut ci)?);
                }
                // %B / %b: a month NAME (full or abbreviated), case-insensitive.
                // Try the full table first, then the abbreviation. The matched
                // position + 1 is the month number. A bogus name → None → NA.
                'B' | 'b' => {
                    let pos = match_name(&chars, &mut ci, &MONTHS_FULL)
                        .or_else(|| match_name(&chars, &mut ci, &MONTHS_ABBR))?;
                    month = Some(pos as i64 + 1);
                }
                // %A / %a: a weekday NAME. Like base R's strptime, the weekday is
                // parsed (and must be a real name) but does NOT constrain the
                // resulting date — we consume and discard it.
                'A' | 'a' => {
                    match_name(&chars, &mut ci, &WEEKDAYS_FULL)
                        .or_else(|| match_name(&chars, &mut ci, &WEEKDAYS_ABBR))?;
                }
                // An unsupported conversion in the format → no parse (NA), rather
                // than silently misreading. (Sub-day %H/%M/%S land in R-46.)
                _ => return None,
            }
            fi += 2;
        } else {
            // A literal format character must match the input exactly.
            if ci >= chars.len() || chars[ci] != fmt[fi] {
                return None;
            }
            ci += 1;
            fi += 1;
        }
    }
    // Any unconsumed input is a mismatch (trailing garbage → NA).
    if ci != chars.len() {
        return None;
    }

    let (y, m, d) = (year?, month?, day?);
    // Range-check the calendar fields. `days_from_civil` is total over i64, but a
    // nonsense month/day would otherwise round-trip to a *different* date, so we
    // reject anything outside the real calendar. (Day-of-month is validated by a
    // round-trip below — the simplest correct check.)
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let z = days_from_civil(y, m, d);
    // Round-trip to reject impossible days like 2021-02-30 (which the raw formula
    // would happily fold into March): the day is valid iff it reconstructs exactly.
    if civil_from_days(z) != (y, m, d) {
        return None;
    }
    Some(z)
}

/// Render one day count `z` to a string under `format` (`%Y`, `%m`, `%d`, `%j`,
/// and literals). Total — never panics; pre-epoch counts work via the i64 kernel.
fn format_date_days(z: i64, format: &str) -> String {
    let (y, m, d) = civil_from_days(z);
    let fmt: Vec<char> = format.chars().collect();
    let mut out = String::new();
    let mut fi = 0;
    while fi < fmt.len() {
        if fmt[fi] == '%' && fi + 1 < fmt.len() {
            match fmt[fi + 1] {
                'Y' => out.push_str(&y.to_string()),
                'm' => out.push_str(&format!("{m:02}")),
                'd' => out.push_str(&format!("{d:02}")),
                // %e: day of month, **space-padded** to width 2 ("the 5th" → " 5").
                'e' => out.push_str(&format!("{d:2}")),
                // %B / %b: full / abbreviated English month name. `m` is 1..=12 by
                // construction (civil_from_days), so `m - 1` is a valid 0..11 index.
                'B' => out.push_str(MONTHS_FULL[(m - 1) as usize]),
                'b' => out.push_str(MONTHS_ABBR[(m - 1) as usize]),
                // %A / %a: full / abbreviated English weekday name.
                'A' => out.push_str(WEEKDAYS_FULL[weekday_index(z)]),
                'a' => out.push_str(WEEKDAYS_ABBR[weekday_index(z)]),
                'j' => {
                    // Day of year, 001..366 = (this day) − (Jan 1 of the same year) + 1.
                    let doy = z - days_from_civil(y, 1, 1) + 1;
                    out.push_str(&format!("{doy:03}"));
                }
                other => {
                    // Unknown conversion: emit it literally (forgiving), e.g. a
                    // stray "%q" renders as "%q". Full coverage lands in R-45.
                    out.push('%');
                    out.push(other);
                }
            }
            fi += 2;
        } else {
            out.push(fmt[fi]);
            fi += 1;
        }
    }
    out
}

/// `as.Date(x, format = "%Y-%m-%d")` — build a `Date` (R-44; R-45 extends fields).
///
/// - **Character `x`:** each element is parsed against `format` (default ISO
///   `"%Y-%m-%d"`; pass e.g. `format = "%Y/%m/%d"` or `"%B %d, %Y"`). The format
///   may be supplied either as the named `format =` or — matching base R and
///   `format.Date` — as the **second positional** argument
///   (`as.Date("15 Jan 2021", "%d %b %Y")`). Unparseable / out-of-range strings
///   become `NA` — never a panic.
/// - **Numeric `x`:** taken directly as days-since-epoch (`as.Date(0)` is
///   1970-01-01). An `origin =` other than the epoch is deferred (R-46); we use
///   1970-01-01.
///
/// Vectorised; the result always carries class "Date".
fn b_as_date(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let x = first_positional(args)?;
    // Already a Date? Return as-is (idempotent), seeing through the wrapper.
    if is_date(x) {
        return Ok(x.clone());
    }
    // The format is the named `format =`, or the second positional string (only
    // meaningful on the character path — the numeric path ignores it). Defaults to
    // ISO. Reading the positional only as a string means a numeric second arg
    // (which `as.Date` does not otherwise define) is harmlessly ignored.
    let format = named_str(args, "format")
        .or_else(|| {
            nth_positional(args, 1).and_then(|v| v.as_character().into_iter().next().flatten())
        })
        .unwrap_or_else(|| "%Y-%m-%d".to_string());

    // The peeled value decides character-parse vs numeric-wrap. A character vector
    // parses; anything coercible to double is taken as raw day counts.
    let days: Vec<f64> = match peel_structural(x) {
        SValue::Character(strs) => strs
            .iter()
            .map(|opt| match opt {
                Some(s) => parse_date_str(s, &format)
                    .map(|z| z as f64)
                    .unwrap_or_else(na_real),
                None => na_real(),
            })
            .collect(),
        other => {
            // Numeric (or coercible) input → days since epoch, truncated to whole
            // days (R stores Dates as doubles but they are integral here). An
            // out-of-range or non-finite count (e.g. `as.Date(1e300)`) becomes NA
            // rather than saturating to `i64::MAX` and overflowing the kernel — the
            // numeric counterpart to the string parser's digit cap.
            let d = other.as_double()?;
            d.iter()
                .map(|v| {
                    checked_date_days(v)
                        .map(|z| z as f64)
                        .unwrap_or_else(na_real)
                })
                .collect()
        }
    };
    Ok(make_date(days))
}

/// `Sys.Date()` — today's date as a length-1 `Date` (R-44).
///
/// The runtime has **no deterministic clock hook** (there is no `Sys.time`/`now`
/// abstraction to reuse), so we read the wall clock directly. The duration since
/// `UNIX_EPOCH` divided by 86400 seconds is the day count; a clock set *before*
/// the epoch yields a negative count, handled without panic. Because the value is
/// non-deterministic, tests assert only its structure (class "Date" + a single
/// finite numeric), never the exact day.
fn b_sys_date(_interp: &Interpreter, _args: &[Arg]) -> SResult<SValue> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now();
    // `duration_since` is Err if `now` precedes the epoch; handle both directions
    // so a misconfigured clock can never panic.
    let days = match now.duration_since(UNIX_EPOCH) {
        Ok(dur) => (dur.as_secs() / 86_400) as i64,
        Err(e) => -((e.duration().as_secs().div_ceil(86_400)) as i64),
    };
    Ok(make_date(vec![days as f64]))
}

/// `format.Date(d, format = "%Y-%m-%d")` — render a `Date` to a character vector
/// (R-44). Supported fields: `%Y`, `%m`, `%d`, `%j` (day-of-year). `NA` days stay
/// `NA`. Vectorised. Reached directly and via the `format()` generic's dispatch.
fn b_format_date(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let x = first_positional(args)?;
    // The format may be the named `format =` or the second positional argument
    // (`format(d, "%Y/%m/%d")`), defaulting to ISO.
    let format = named_str(args, "format")
        .or_else(|| {
            nth_positional(args, 1).and_then(|v| v.as_character().into_iter().next().flatten())
        })
        .unwrap_or_else(|| "%Y-%m-%d".to_string());

    let days = x.as_double()?;
    let out: Vec<Option<String>> = days
        .iter()
        // `checked_date_days` rejects NA / non-finite / out-of-range counts → NA,
        // so an out-of-range day (e.g. a hand-built `structure(1e300, class="Date")`)
        // can never overflow the civil kernel in `format_date_days`.
        .map(|v| checked_date_days(v).map(|z| format_date_days(z, &format)))
        .collect();
    Ok(SValue::Character(out))
}

/// `difftime(time1, time2)` — the difference `time1 − time2` in **days**, as a
/// numeric vector (R-44). In base R this returns a `"difftime"` object; here the
/// only supported unit is days, so we return the plain numeric day difference
/// (units other than days are deferred to R-45). Recycles and propagates `NA`
/// through the shared `arithmetic` kernel — which already sees through the Date
/// wrappers — so this is a thin, faithful wrapper.
fn b_difftime(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let t1 = nth_positional(args, 0)
        .ok_or_else(|| SError::BadArgs("difftime: missing time1".into()))?;
    let t2 = nth_positional(args, 1)
        .ok_or_else(|| SError::BadArgs("difftime: missing time2".into()))?;
    crate::value::arithmetic("-", t1, t2)
}

/// `weekdays(d)` — the English weekday name of each `Date` (R-44).
///
/// The anchor is the historical fact that **1970-01-01 (day 0) was a Thursday**.
/// Indexing the names from Sunday=0, day 0 (Thursday) is index 4, so the weekday
/// index is `(days + 4).rem_euclid(7)`. We use `rem_euclid` — *not* `%` — because
/// Rust's `%` returns a **negative** remainder for negative (pre-epoch) day
/// counts, which would panic on `Vec` indexing; `rem_euclid` always lands in
/// `0..7`. `NA` days yield `NA`.
fn b_weekdays(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let days = first_positional(args)?.as_double()?;
    let out: Vec<Option<String>> = days
        .iter()
        .map(|v| {
            // `checked_date_days` rejects NA / non-finite / out-of-range counts → NA,
            // so `z + 4` in `weekday_index` can never overflow. The shared
            // `WEEKDAYS_FULL` table (Sunday-based) is the same one `%A` uses.
            checked_date_days(v).map(|z| WEEKDAYS_FULL[weekday_index(z)].to_string())
        })
        .collect();
    Ok(SValue::Character(out))
}

/// `months(d)` — the full English month name of each `Date` (R-45). Equivalent to
/// `format(d, "%B")`. Vectorised; `NA` days yield `NA`. `civil_from_days` always
/// returns a month in `1..=12`, so the `m - 1` index is always in range.
fn b_months(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let days = first_positional(args)?.as_double()?;
    let out: Vec<Option<String>> = days
        .iter()
        .map(|v| {
            checked_date_days(v).map(|z| {
                let (_, m, _) = civil_from_days(z);
                MONTHS_FULL[(m - 1) as usize].to_string()
            })
        })
        .collect();
    Ok(SValue::Character(out))
}

/// `quarters(d)` — the calendar quarter of each `Date` as `"Q1"`..`"Q4"` (R-45).
/// The quarter is `(month - 1) / 3 + 1` (Jan–Mar = Q1, …, Oct–Dec = Q4).
/// Vectorised; `NA` days yield `NA`.
fn b_quarters(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let days = first_positional(args)?.as_double()?;
    let out: Vec<Option<String>> = days
        .iter()
        .map(|v| {
            checked_date_days(v).map(|z| {
                let (_, m, _) = civil_from_days(z);
                let q = (m - 1) / 3 + 1; // m in 1..=12 → q in 1..=4
                format!("Q{q}")
            })
        })
        .collect();
    Ok(SValue::Character(out))
}

// ===========================================================================
// R-46 — POSIXct date-times (UTC). A *date-time* layered on the R-44/R-45
// calendar machinery.
// ===========================================================================
//
// A `POSIXct` is — exactly like `Date` — an ordinary numeric vector, but the
// number counts **seconds since the epoch 1970-01-01 00:00:00 UTC** instead of
// whole days. It carries the two-element class `c("POSIXct", "POSIXt")` via the
// same transparent `SValue::Classed` wrapper that backs `Date`, so it is just as
// transparent to `as.numeric`, arithmetic, recycling, and indexing.
//
//   seconds = days * 86400 + intraday_seconds
//
// The whole point of the design is **reuse**: split a seconds count into a day
// count and an intraday remainder, and *everything else is the existing Date
// code*. The date half feeds straight into the R-44 civil kernel
// (`civil_from_days` / `days_from_civil`) and the R-45 `format_date_days`
// `%`-field renderer; only the intraday H:M:S split and the seconds bound are new.
//
//   ┌──────────── seconds (i64, may be negative for pre-epoch) ────────────┐
//   │  div_euclid(86400) → day count z  ──▶ civil_from_days / format_date  │
//   │  rem_euclid(86400) → intraday s   ──▶ H = s/3600, M = s/60%60, S=s%60 │
//   └──────────────────────────────────────────────────────────────────────┘
//
// `div_euclid` / `rem_euclid` (NOT `/` and `%`) are essential: for a pre-epoch
// instant like −1 second (= 1969-12-31 23:59:59) we need day −1 with intraday
// 86399, which is exactly what the *Euclidean* (floored) operations give; plain
// truncating division would give day 0 with intraday −1 and a negative array
// index. This mirrors the `rem_euclid` reasoning already used for `weekday_index`.

/// Seconds in one day — the conversion factor between the `Date` day count and a
/// `POSIXct` seconds count.
const SECONDS_PER_DAY: i64 = 86_400;

/// The largest magnitude (in seconds) we admit for a `POSIXct`. It is the
/// [`MAX_DATE_DAYS`] day bound scaled to seconds, so a POSIXct's *date half* can
/// never exceed the range the civil kernel safely handles, and the multiply
/// `MAX_DATE_DAYS * 86400` (≈ 8.64e15) stays well inside `i64` (max ≈ 9.2e18).
/// This is the numeric counterpart to [`MAX_DATE_DIGITS`] for the seconds path:
/// `as.POSIXct(1e300)` becomes `NA` rather than overflowing the kernel.
const MAX_POSIXCT_SECONDS: i64 = MAX_DATE_DAYS * SECONDS_PER_DAY;

/// Clamp-or-reject a raw seconds count: `Some(s)` if it is finite and within
/// [`MAX_POSIXCT_SECONDS`], else `None` (→ NA). The seconds analogue of
/// [`checked_date_days`]; every untrusted `f64`→POSIXct boundary funnels through
/// here so the civil kernel only ever sees an in-range day count.
fn checked_posixct_seconds(v: f64) -> Option<i64> {
    if is_na_real(v) || !v.is_finite() {
        return None;
    }
    let s = v.trunc();
    if s.abs() > MAX_POSIXCT_SECONDS as f64 {
        None
    } else {
        Some(s as i64)
    }
}

/// Is `x` a POSIXct — a value whose (explicit) class vector contains "POSIXct"?
fn is_posixct(x: &SValue) -> bool {
    class_of(x).iter().any(|c| c == "POSIXct")
}

/// Wrap a vector of seconds-since-epoch (NA-aware doubles) as a `POSIXct` — class
/// `c("POSIXct", "POSIXt")` over a plain `Double`. The single constructor every
/// POSIXct-producing builtin funnels through (mirrors [`make_date`]).
fn make_posixct(seconds: Vec<f64>) -> SValue {
    SValue::Classed {
        inner: Box::new(SValue::doubles(seconds)),
        class: vec!["POSIXct".to_string(), "POSIXt".to_string()],
    }
}

/// Parse a `"YYYY-MM-DD HH:MM:SS"` (or bare `"YYYY-MM-DD"` → midnight) datetime
/// string to **seconds since the epoch**, or `None` (→ NA) on any malformed or
/// out-of-range input. Never panics on crafted input.
///
/// The date half reuses R-44's [`parse_date_str`] with the ISO `"%Y-%m-%d"`
/// pattern, so it inherits all of that function's safety (digit cap, calendar
/// round-trip validation, `MAX_DATE_DIGITS`). The optional time half is then read
/// as `HH:MM:SS` with the fields range-checked — **hour** 0–23, **minute** 0–59,
/// **second** 0–60 (the trailing 60 is the POSIX leap-second slot, which base R
/// also accepts). The resulting seconds are bounded by [`MAX_POSIXCT_SECONDS`]
/// before return.
fn parse_posixct_str(string: &str) -> Option<i64> {
    // Split the date and (optional) time on the first ASCII space. A datetime is
    // "<date> <time>"; a bare date has no space and is treated as midnight.
    let (date_part, time_part) = match string.split_once(' ') {
        Some((d, t)) => (d, Some(t)),
        None => (string, None),
    };

    // Reuse the R-44 calendar parser for the date half (ISO only — other date
    // layouts are out of scope for `as.POSIXct` here).
    let z = parse_date_str(date_part, "%Y-%m-%d")?;

    // The intraday offset in seconds. Absent time half ⇒ midnight (0).
    let intraday = match time_part {
        None => 0,
        Some(t) => {
            // Parse exactly "HH:MM:SS" via the same length-bounded uint reader the
            // date parser uses. Any trailing garbage, missing colon, or extra
            // field fails the whole parse (→ NA).
            let chars: Vec<char> = t.chars().collect();
            let mut i = 0usize;
            let h = parse_uint_field(&chars, &mut i)?;
            expect_char(&chars, &mut i, ':')?;
            let m = parse_uint_field(&chars, &mut i)?;
            expect_char(&chars, &mut i, ':')?;
            let s = parse_uint_field(&chars, &mut i)?;
            // No trailing characters allowed (e.g. fractional seconds → R-47).
            if i != chars.len() {
                return None;
            }
            // Range-check each field. Second admits 60 (leap-second slot).
            if !(0..=23).contains(&h) || !(0..=59).contains(&m) || !(0..=60).contains(&s) {
                return None;
            }
            h * 3600 + m * 60 + s
        }
    };

    // days * 86400 + intraday. `z` is already bounded by MAX_DATE_DAYS (from
    // parse_date_str), so this multiply stays inside i64; the final bound check
    // is belt-and-suspenders against the additive intraday term.
    let seconds = z.checked_mul(SECONDS_PER_DAY)?.checked_add(intraday)?;
    if seconds.abs() > MAX_POSIXCT_SECONDS {
        return None;
    }
    Some(seconds)
}

/// Match a single literal character at `chars[*idx]`, advancing past it; `None`
/// (→ the whole parse fails → NA) if the input is exhausted or the char differs.
/// A tiny helper so the time parser reads literally like its grammar `HH:MM:SS`.
fn expect_char(chars: &[char], idx: &mut usize, want: char) -> Option<()> {
    if *idx < chars.len() && chars[*idx] == want {
        *idx += 1;
        Some(())
    } else {
        None
    }
}

/// Render one seconds count to a string under `format`. Supports the new sub-day
/// fields `%H` (00–23), `%M` (00–59), `%S` (00–60) and **every R-44/R-45 date
/// field** (`%Y %m %d %B %b %A %a %j %e` and literals), since the date half is
/// just the day count fed into [`format_date_days`]. Total — never panics; the
/// `div_euclid`/`rem_euclid` split keeps pre-epoch instants correct.
///
/// Implementation note: rather than re-implement the whole `%`-field scanner, we
/// pre-substitute *only* the three time fields into the format string and let the
/// reused date renderer handle the rest. The time fields render to fixed two-digit
/// numbers with no `%`, so they cannot collide with the date renderer's scan, and
/// a `%%` escape is preserved (the date renderer passes unknown `%x` through
/// literally, and we never touch `%%`).
fn format_posixct_seconds(seconds: i64, format: &str) -> String {
    // Floored split: day count for the date half, intraday seconds for the clock.
    let z = seconds.div_euclid(SECONDS_PER_DAY);
    let intraday = seconds.rem_euclid(SECONDS_PER_DAY); // 0..86400
    let hh = intraday / 3600;
    let mm = (intraday % 3600) / 60;
    let ss = intraday % 60;

    // Walk the format, emitting %H/%M/%S ourselves and delegating each maximal run
    // of date-only text to `format_date_days`. We accumulate non-time format
    // characters into `date_run` and flush them (rendered against `z`) whenever we
    // hit a time field, so reused fields like %B/%A still resolve correctly.
    let fmt: Vec<char> = format.chars().collect();
    let mut out = String::new();
    let mut date_run = String::new();
    let mut fi = 0;
    while fi < fmt.len() {
        if fmt[fi] == '%' && fi + 1 < fmt.len() {
            match fmt[fi + 1] {
                'H' | 'M' | 'S' => {
                    // Flush any pending date-only run first (preserves order).
                    if !date_run.is_empty() {
                        out.push_str(&format_date_days(z, &date_run));
                        date_run.clear();
                    }
                    let val = match fmt[fi + 1] {
                        'H' => hh,
                        'M' => mm,
                        _ => ss,
                    };
                    out.push_str(&format!("{val:02}"));
                }
                // Any other conversion belongs to the date renderer — buffer it
                // verbatim (both the '%' and the letter) for the next flush.
                other => {
                    date_run.push('%');
                    date_run.push(other);
                }
            }
            fi += 2;
        } else {
            date_run.push(fmt[fi]);
            fi += 1;
        }
    }
    if !date_run.is_empty() {
        out.push_str(&format_date_days(z, &date_run));
    }
    out
}

/// `as.POSIXct(x, tz = "UTC")` — build a `POSIXct` (R-46).
///
/// - **Character `x`:** each element is parsed as `"YYYY-MM-DD HH:MM:SS"` (or
///   `"YYYY-MM-DD"` → midnight) to seconds since the epoch. Unparseable or
///   out-of-range strings become `NA` — never a panic.
/// - **Numeric `x`:** taken directly as raw seconds-since-epoch (bounded by
///   [`MAX_POSIXCT_SECONDS`]; out-of-range / non-finite → `NA`).
///
/// Only `tz = "UTC"` is honoured; a different `tz` is currently ignored (R-47).
/// Vectorised; the result carries class `c("POSIXct", "POSIXt")`.
fn b_as_posixct(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let x = first_positional(args)?;
    // Already a POSIXct? Return as-is (idempotent), seeing through the wrapper.
    if is_posixct(x) {
        return Ok(x.clone());
    }
    // A `Date` converts by scaling its day count to seconds (midnight UTC).
    if is_date(x) {
        let days = x.as_double()?;
        let seconds: Vec<f64> = days
            .iter()
            .map(|v| {
                checked_date_days(v)
                    .map(|z| (z * SECONDS_PER_DAY) as f64)
                    .unwrap_or_else(na_real)
            })
            .collect();
        return Ok(make_posixct(seconds));
    }

    let seconds: Vec<f64> = match peel_structural(x) {
        SValue::Character(strs) => strs
            .iter()
            .map(|opt| match opt {
                Some(s) => parse_posixct_str(s)
                    .map(|secs| secs as f64)
                    .unwrap_or_else(na_real),
                None => na_real(),
            })
            .collect(),
        other => {
            // Numeric (or coercible) input → raw seconds, bounded so an
            // out-of-range value (`as.POSIXct(1e300)`) becomes NA rather than
            // overflowing the kernel downstream.
            let d = other.as_double()?;
            d.iter()
                .map(|v| {
                    checked_posixct_seconds(v)
                        .map(|s| s as f64)
                        .unwrap_or_else(na_real)
                })
                .collect()
        }
    };
    Ok(make_posixct(seconds))
}

/// `Sys.time()` — the current time as a length-1 `POSIXct` (R-46).
///
/// Like [`b_sys_date`], the runtime has no deterministic clock hook, so we read
/// the wall clock directly: seconds since `UNIX_EPOCH`. A clock set *before* the
/// epoch yields a negative count, handled without panic. Non-deterministic, so
/// tests assert only its structure (class `c("POSIXct","POSIXt")` + a single
/// finite numeric), never the exact instant.
fn b_sys_time(_interp: &Interpreter, _args: &[Arg]) -> SResult<SValue> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now();
    let seconds = match now.duration_since(UNIX_EPOCH) {
        Ok(dur) => dur.as_secs() as i64,
        Err(e) => -(e.duration().as_secs() as i64),
    };
    Ok(make_posixct(vec![seconds as f64]))
}

/// `format.POSIXct(x, format = "%Y-%m-%d %H:%M:%S")` — render a `POSIXct` to a
/// character vector (R-46). Supports `%H`/`%M`/`%S` plus every reused R-44/R-45
/// date field; `NA` seconds stay `NA`. Reached directly and via the `format()`
/// generic's dispatch (which checks `"POSIXct"` before `"Date"`).
fn b_format_posixct(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let x = first_positional(args)?;
    let format = named_str(args, "format")
        .or_else(|| {
            nth_positional(args, 1).and_then(|v| v.as_character().into_iter().next().flatten())
        })
        .unwrap_or_else(|| "%Y-%m-%d %H:%M:%S".to_string());

    let secs = x.as_double()?;
    let out: Vec<Option<String>> = secs
        .iter()
        // `checked_posixct_seconds` rejects NA / non-finite / out-of-range → NA,
        // so an out-of-range seconds count can never overflow the civil kernel.
        .map(|v| checked_posixct_seconds(v).map(|s| format_posixct_seconds(s, &format)))
        .collect();
    Ok(SValue::Character(out))
}

// ===========================================================================
// v2 — S3 dispatch helpers
// ===========================================================================

/// `class(x)` — the class vector (explicit if set, else the implicit type). For an
/// R5 reference-class **instance** the vector is its inheritance chain
/// `c("Sub", "Base", …, "envRefClass", "environment")` (R-25), so `class(obj)`
/// reveals the hierarchy; every other value keeps the ordinary `class_of`.
fn b_class(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let x = first_positional(args)?;
    let classes = crate::refclass::instance_class_vector(x).unwrap_or_else(|| class_of(x));
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

/// The class vector used by `inherits`/`is`: an R5 instance's inheritance chain
/// (R-25) when `x` is one, otherwise the ordinary [`class_of`]. Centralised so the
/// two predicates agree on what "the classes of `x`" means.
fn query_classes(x: &SValue) -> Vec<String> {
    crate::refclass::instance_class_vector(x).unwrap_or_else(|| class_of(x))
}

/// `inherits(x, what)` — whether any class of `x` matches `what`. For an R5
/// instance the classes are its inheritance chain, so
/// `inherits(sub_obj, "Base")` is `TRUE` (R-25).
fn b_inherits(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let positional: Vec<&SValue> = args
        .iter()
        .filter(|a| a.name.is_none())
        .map(|a| &a.value)
        .collect();
    let classes: HashSet<String> = query_classes(
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

/// `is(object, class2)` — whether `object` is (a member of, or inherits from)
/// `class2`. In this subset `is` is the R5/S4-flavoured cousin of `inherits`: it is
/// `TRUE` when `class2` appears anywhere in `object`'s class vector — which, for an
/// R5 reference-class instance, is its full inheritance chain (R-25). So
/// `is(sub_obj, "Sub")` and `is(sub_obj, "Base")` are both `TRUE`, while
/// `is(sub_obj, "Other")` is `FALSE`. A missing `class2` is a clean error.
fn b_is(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let positional: Vec<&SValue> = args
        .iter()
        .filter(|a| a.name.is_none())
        .map(|a| &a.value)
        .collect();
    let object = positional
        .first()
        .ok_or_else(|| SError::BadArgs("is: missing object".into()))?;
    let classes: HashSet<String> = query_classes(object).into_iter().collect();
    let class2 = positional
        .get(1)
        .map(|v| v.as_character())
        .unwrap_or_default();
    // `is(x)` with no class2 in R returns the class vector; we require class2 here
    // (the one-arg form is out of scope) and treat its absence as FALSE rather than
    // erroring, so `is(x, missing)` never panics. A supplied class2 matches if any
    // of its elements is in the class set.
    let hit = class2.into_iter().flatten().any(|c| classes.contains(&c));
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
/// `order(x, y, ...)` — the permutation of 1-based indices that sorts `x`
/// ascending, **breaking ties by `y`, then the next key, …**, lexicographically.
/// Indices still tied after every key are kept in their **original order** (a
/// *stable* sort), exactly as R does.
///
/// ```text
/// order(c(2, 1, 2), c(1, 2, 1))
///   element (idx : key1, key2):  1:(2,1)  2:(1,2)  3:(2,1)
///   sort by key1:                idx2 (1) first; idx1 & idx3 (both 2) tie
///   break by key2:               idx1 (1) and idx3 (1) STILL tie
///   stable fallback:             original order → idx1 before idx3
///   result:                      c(2, 1, 3)
/// ```
///
/// Each key is coerced to a comparison form **independently**: a pure-numeric key
/// compares numerically (with `NA`/`NaN` sorting **last**, mirroring R's default
/// `na.last = TRUE`); any other key compares on its character rendering
/// (lexicographically). That lets numeric and character keys be mixed across
/// positions. The single-key R-13 form is simply the one-key case.
///
/// Every key must have the **same length** as the first; a mismatched length is a
/// graceful error, never an out-of-bounds index. The only allocation is one
/// `usize` per element of the first key, sorted in `O(n log n)` — no
/// user-controlled multiplier, so no extra cap beyond the inputs' own
/// `MAX_SEQ_LEN` bound.
fn b_order(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    // Collect the sort keys, in order, from the positional arguments. Each key is
    // pre-coerced once into a totally-ordered comparison form (numeric or string),
    // so the comparator below is a cheap lookup rather than a re-coercion.
    enum Key {
        Num(Vec<f64>),
        Str(Vec<Option<String>>),
    }
    let positional: Vec<&SValue> = args.iter().filter(|a| a.name.is_none()).map(|a| &a.value).collect();
    let first = *positional
        .first()
        .ok_or_else(|| SError::BadArgs("argument \"x\" is missing".into()))?;
    let n = first.length();

    let mut keys: Vec<Key> = Vec::with_capacity(positional.len());
    for v in &positional {
        // A length mismatch between keys is an error in R ("argument lengths
        // differ"); we reject it before any indexing can go out of bounds.
        if v.length() != n {
            return Err(SError::BadArgs(
                "argument lengths differ in order()".into(),
            ));
        }
        let key = match v.strip_names().strip_attrs() {
            SValue::Character(_) | SValue::Factor { .. } => Key::Str(v.as_character()),
            other => Key::Num(other.as_double()?.iter().collect()),
        };
        keys.push(key);
    }

    // Compare two original indices `a`, `b` by walking the keys left-to-right; the
    // first key that distinguishes them decides. NA sorts last within a key.
    let cmp = |a: usize, b: usize| -> std::cmp::Ordering {
        for key in &keys {
            let ord = match key {
                Key::Num(v) => match (is_na_real(v[a]), is_na_real(v[b])) {
                    (true, true) => std::cmp::Ordering::Equal,
                    (true, false) => std::cmp::Ordering::Greater,
                    (false, true) => std::cmp::Ordering::Less,
                    (false, false) => v[a].partial_cmp(&v[b]).unwrap_or(std::cmp::Ordering::Equal),
                },
                Key::Str(v) => match (&v[a], &v[b]) {
                    (None, None) => std::cmp::Ordering::Equal,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (Some(x), Some(y)) => x.cmp(y),
                },
            };
            if ord != std::cmp::Ordering::Equal {
                return ord;
            }
        }
        std::cmp::Ordering::Equal
    };

    let mut idx: Vec<usize> = (0..n).collect();
    // `sort_by` is stable, so indices equal under every key keep original order.
    idx.sort_by(|&a, &b| cmp(a, b));
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

/// Parse the **`incomparables =`** keyword argument that `unique`, `duplicated`,
/// and `anyDuplicated` share (R-31) into a set of *incomparable character keys*.
///
/// R's contract: the default is `incomparables = FALSE`, meaning "there are no
/// incomparable values". Any other value is a **vector listing the elements** to
/// treat as incomparable — a value listed there is **never considered equal to
/// anything** (not even another copy of itself), so it is never flagged as a
/// duplicate and never removed as one. We model "incomparable" on the same coerced
/// **character** key the dedup builtins already use (`as_character`), so a numeric
/// `incomparables = 1` and a character `incomparables = "1"` agree, exactly like the
/// rest of the set-op family.
///
/// The single special case is the literal default `FALSE` (a length-1 logical that
/// is `FALSE`): R spells "no incomparables" that way, so we must NOT treat the
/// string `"FALSE"` as an incomparable element. Every other value — including a
/// `TRUE`, a number, a string, or a longer vector — contributes its character keys.
/// An absent argument yields the empty set. This never errors and never panics:
/// `as_character` is total, and the result is just a `HashSet` whose size is bounded
/// by the (already-`MAX_SEQ_LEN`-capped) `incomparables` vector.
fn incomparables_keys(args: &[Arg]) -> HashSet<Option<String>> {
    let Some(arg) = args.iter().find(|a| a.name.as_deref() == Some("incomparables")) else {
        return HashSet::new();
    };
    // The default `FALSE` means "no incomparables" — treat it as the empty set.
    if matches!(&arg.value, SValue::Logical(v) if v.as_slice() == [Some(false)]) {
        return HashSet::new();
    }
    arg.value.as_character().into_iter().collect()
}

/// `unique(x, incomparables = FALSE, fromLast = FALSE)` — distinct elements,
/// first-occurrence order preserved.
///
/// The base case is the same `as_character`-key + `seen`-set scan as `duplicated`:
/// keep the 1-based position of each key the first time we meet it.
///
/// **`fromLast = TRUE`** (R-31) keeps the **last** occurrence of each distinct value
/// instead of the first. We scan right-to-left to decide which positions survive,
/// then gather them in **ascending index order** so the kept elements stay in input
/// order — R's behaviour (`unique(c(1,2,1), fromLast=TRUE)` is `c(2, 1)`).
///
/// **`incomparables =`** (R-31, via [`incomparables_keys`]) lists values that are
/// never equal to anything: a position whose key is incomparable is **always kept**
/// and is **never recorded in `seen`** (so it can neither be suppressed nor suppress
/// a later genuine duplicate). `unique(c(1,1,2,2), incomparables=1)` is `c(1, 1, 2)`.
fn b_unique(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let v = first_positional(args)?;
    let from_last = match args.iter().find(|a| a.name.as_deref() == Some("fromLast")) {
        Some(arg) => arg.value.truthy()?,
        None => false,
    };
    let incomparable = incomparables_keys(args);
    let keys = v.as_character();
    let n = keys.len();
    let mut seen: HashSet<Option<String>> = HashSet::new();
    // Decide which positions to keep. A key is kept if it is incomparable, or if
    // it is the first sighting of a comparable key in the scan direction.
    let mut keep_flag = vec![false; n];
    let mut decide = |i: usize| {
        // Kept if incomparable, or the first sighting of a comparable key. The `||`
        // short-circuits exactly like the original if/else-if: `seen.insert` runs
        // only when the key is comparable.
        if incomparable.contains(&keys[i]) || seen.insert(keys[i].clone()) {
            keep_flag[i] = true;
        }
    };
    if from_last {
        for i in (0..n).rev() {
            decide(i);
        }
    } else {
        for i in 0..n {
            decide(i);
        }
    }
    // Gather kept positions in ascending index order (input order), regardless of
    // scan direction, so `fromLast` only changes *which* copy survives, not order.
    let keep: Vec<f64> = (0..n)
        .filter(|&i| keep_flag[i])
        .map(|i| (i + 1) as f64)
        .collect();
    index(v, &SValue::doubles(keep))
}

// ─── R-29 — vector set operations & ordering ─────────────────────────────────
//
// R treats vectors as *multisets* when you ask for set operations: `union`,
// `intersect`, and `setdiff` all deduplicate, and — crucially — they preserve
// **first-occurrence order** rather than sorting (R's `union(c(3,1), c(1,2))`
// is `c(3, 1, 2)`, not `c(1, 2, 3)`). The comparison key is the same coerced
// *character* form that `unique` and `%in%` already use (`as_character`), so a
// single code path handles numeric and character vectors uniformly: two values
// are "the same element" iff their character renderings match. We never build a
// new value type — we compute which 1-based positions to keep and hand them to
// `value::index`, which gathers them while preserving the original element type.
//
//     union(x, y)     = unique(c(x, y))                  first-occurrence order
//     intersect(x, y) = keep x[i] once, if key(x[i]) ∈ keys(y)
//     setdiff(x, y)   = keep x[i] once, if key(x[i]) ∉ keys(y)
//
// Output size is bounded by the inputs (union ≤ |x|+|y|, the others ≤ |x|), and
// every operand is already `MAX_SEQ_LEN`-bounded, so no fresh cap is needed.

/// Read the **two** positional arguments (`x`, `y`) a binary set op expects.
/// Missing or surplus named arguments are ignored; the *second positional* (not
/// merely the second argument) is `y`, matching R's argument matching for these
/// purely positional builtins.
fn two_positional(args: &[Arg]) -> SResult<(&SValue, &SValue)> {
    let mut pos = args.iter().filter(|a| a.name.is_none()).map(|a| &a.value);
    let x = pos
        .next()
        .ok_or_else(|| SError::BadArgs("argument \"x\" is missing".into()))?;
    let y = pos
        .next()
        .ok_or_else(|| SError::BadArgs("argument \"y\" is missing".into()))?;
    Ok((x, y))
}

/// `union(x, y)` — the distinct elements of `c(x, y)`, in first-occurrence
/// order. Implemented as `unique(combine(x, y))`: concatenate (reusing `combine`,
/// the engine behind `c(...)`), then keep the first sighting of each character
/// key. The result's element type is whatever `combine` produced (numeric if
/// both sides are numeric, character if either side is character) — identical to
/// what `c(x, y)` would yield, then deduplicated.
fn b_union(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let (x, y) = two_positional(args)?;
    let joined = combine(&[
        Arg {
            name: None,
            value: x.clone(),
        },
        Arg {
            name: None,
            value: y.clone(),
        },
    ]);
    let keys = joined.as_character();
    let mut seen: HashSet<Option<String>> = HashSet::new();
    let keep: Vec<f64> = keys
        .iter()
        .enumerate()
        .filter_map(|(i, k)| seen.insert(k.clone()).then_some((i + 1) as f64))
        .collect();
    index(&joined, &SValue::doubles(keep))
}

/// `intersect(x, y)` — the elements present in **both** `x` and `y`, in `x`'s
/// order, deduplicated. We build `y`'s key-set once, then walk `x` keeping each
/// position whose key is in `y` *and* has not already been kept (the `seen` set
/// enforces the dedup).
fn b_intersect(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let (x, y) = two_positional(args)?;
    let y_keys: HashSet<Option<String>> = y.as_character().into_iter().collect();
    let mut seen: HashSet<Option<String>> = HashSet::new();
    let keep: Vec<f64> = x
        .as_character()
        .iter()
        .enumerate()
        .filter_map(|(i, k)| {
            (y_keys.contains(k) && seen.insert(k.clone())).then_some((i + 1) as f64)
        })
        .collect();
    index(x, &SValue::doubles(keep))
}

/// `setdiff(x, y)` — the elements of `x` **not** in `y`, deduplicated, in `x`'s
/// order. The mirror of `intersect`: keep a position whose key is *absent* from
/// `y` and not yet seen.
fn b_setdiff(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let (x, y) = two_positional(args)?;
    let y_keys: HashSet<Option<String>> = y.as_character().into_iter().collect();
    let mut seen: HashSet<Option<String>> = HashSet::new();
    let keep: Vec<f64> = x
        .as_character()
        .iter()
        .enumerate()
        .filter_map(|(i, k)| {
            (!y_keys.contains(k) && seen.insert(k.clone())).then_some((i + 1) as f64)
        })
        .collect();
    index(x, &SValue::doubles(keep))
}

/// `is.element(el, set)` — the function spelling of `el %in% set`: a logical
/// vector, one entry per element of `el`, `TRUE` where it appears in `set`. A
/// thin alias over `value::membership` so the two stay bug-for-bug identical
/// (same coercion, same `NA`-matches-`NA` rule).
fn b_is_element(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let (el, set) = two_positional(args)?;
    Ok(membership(el, set))
}

/// `duplicated(x)` — a logical vector, `TRUE` for an element that equals one
/// seen **earlier** (so the first occurrence of every distinct value is
/// `FALSE`). Same `as_character`-key + `seen`-set scan as `unique`, but it emits
/// the flag instead of gathering: `seen.insert(k)` returns `true` the first time
/// we meet a key (→ not a duplicate) and `false` thereafter (→ a duplicate).
///
/// With **`fromLast = TRUE`** the scan runs **right-to-left** instead, so the
/// *last* occurrence of each value is the keeper (`FALSE`) and the *earlier*
/// copies are flagged. We scan in reverse, recording the flag at the element's
/// real position, so the output stays aligned with the input:
///
/// ```text
/// duplicated(c(1, 2, 1))                 = c(FALSE, FALSE, TRUE)   (keep first 1)
/// duplicated(c(1, 2, 1), fromLast=TRUE)  = c(TRUE,  FALSE, FALSE)  (keep last  1)
/// ```
///
/// **`incomparables =`** (R-31, via [`incomparables_keys`]) lists values that are
/// never equal to anything: an element whose key is incomparable is **always**
/// `FALSE` (never a duplicate) and is **never recorded in `seen`**, so it cannot
/// suppress a later genuine duplicate either:
///
/// ```text
/// duplicated(c(1, 1, 2, 2), incomparables=1) = c(FALSE, FALSE, FALSE, TRUE)
/// ```
fn b_duplicated(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let x = first_positional(args)?;
    let from_last = match args.iter().find(|a| a.name.as_deref() == Some("fromLast")) {
        Some(arg) => arg.value.truthy()?,
        None => false,
    };
    let incomparable = incomparables_keys(args);
    let keys = x.as_character();
    let n = keys.len();
    let mut seen: HashSet<Option<String>> = HashSet::new();
    let mut flags: Vec<Option<bool>> = vec![Some(false); n];
    // One element's flag: incomparable keys are never a duplicate (and are never
    // recorded, so they never suppress a later one); otherwise the first sighting
    // (in the scan direction) is FALSE and the rest are TRUE.
    let mut flag = |i: usize, key: &Option<String>| {
        flags[i] = if incomparable.contains(key) {
            Some(false)
        } else {
            Some(!seen.insert(key.clone()))
        };
    };
    if from_last {
        // Right-to-left: the first comparable key seen (from the right) is the
        // last occurrence and is NOT a duplicate; earlier ones are.
        for i in (0..n).rev() {
            let key = keys[i].clone();
            flag(i, &key);
        }
    } else {
        for (i, k) in keys.iter().enumerate() {
            flag(i, k);
        }
    }
    Ok(SValue::Logical(flags))
}

/// `anyDuplicated(x)` — the **1-based index of the first duplicated element**
/// (the first position whose value already appeared earlier), or `0` when `x` has
/// no duplicates. Defined to agree with `which(duplicated(x))[1]` (and `0` when
/// that is empty). One forward pass over the shared character keys; numeric and
/// character vectors alike. The result is a scalar numeric, matching the other
/// index-returning builtins.
///
/// ```text
/// anyDuplicated(c(1, 2, 1)) = 3   (the second 1, at position 3, is the first dup)
/// anyDuplicated(c(1, 2, 3)) = 0   (no repeats)
/// ```
///
/// **`incomparables =`** (R-31, via [`incomparables_keys`]) lists values that are
/// never equal to anything, so a repeat of an incomparable value is **not** counted
/// as a duplicate (and the value is never recorded in `seen`):
///
/// ```text
/// anyDuplicated(c(1, 2, 1), incomparables=1) = 0   (the only repeat is incomparable)
/// anyDuplicated(c(1, 2, 2), incomparables=1) = 3   (the repeated 2 is comparable)
/// ```
fn b_any_duplicated(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let x = first_positional(args)?;
    let incomparable = incomparables_keys(args);
    let mut seen: HashSet<Option<String>> = HashSet::new();
    for (i, k) in x.as_character().into_iter().enumerate() {
        if incomparable.contains(&k) {
            continue;
        }
        if !seen.insert(k) {
            return Ok(SValue::scalar((i + 1) as f64));
        }
    }
    Ok(SValue::scalar(0.0))
}

/// `rank(x)` — **sample ranks** with **average** tie handling (R's default
/// `ties.method = "average"`). Conceptually: sort the values ascending; an
/// element's rank is its 1-based position in that order, and a run of equal
/// values shares the **mean** of the positions it spans.
///
/// ```text
/// x          = c(1, 1, 2)
/// sorted     =   1   1   2     positions 1   2   3
/// the two 1s span positions 1,2  -> average (1+2)/2 = 1.5
/// rank(x)    = c(1.5, 1.5, 3)
/// ```
///
/// Implementation: sort an index permutation `order` (so `order[p]` is the
/// original index of the value at sorted-position `p`), then walk `order` in
/// runs of equal keys, assigning every member of a run a rank determined by the
/// `ties.method`. Numeric input compares numerically; character input
/// lexicographically. The result is always numeric, matching R. `O(n log n)`
/// time, one `f64` per element — no user-controlled multiplier, so no extra
/// allocation cap is required.
///
/// **`ties.method`** (R-30) selects how a run of `m` equal values spanning the
/// 1-based positions `lo ..= hi` is scored:
///
/// ```text
/// values   c(1, 1, 2)       run of two 1s spans positions 1,2
/// "average"  (lo+hi)/2       → 1.5, 1.5, 3   (the default — R-29 behaviour)
/// "min"      lo              → 1,   1,   3
/// "max"      hi              → 2,   2,   3
/// "first"    lo, lo+1, …     → 1,   2,   3   (consecutive, in original order)
/// ```
///
/// For `"first"` the run members are scored in **original-index order** (the
/// sort is stable, so `order` already lists tied indices smallest-first), giving
/// distinct consecutive ranks. An unrecognised method is a graceful error.
///
/// **`"random"`** (R-31) scores a run like `"first"` — consecutive ranks
/// `lo, lo+1, …, hi` — but assigns those ranks to the tied positions in a **uniform
/// random order** drawn from the **session RNG** (the same generator seeded by
/// `set.seed` that `runif`/`rnorm` use, reached via `Interpreter::sample_with`). The
/// permutation is a Fisher–Yates shuffle driven by `RngState::next_u32`, so the
/// result is **fully reproducible** under `set.seed`. Each swap consumes at most one
/// `u32`, and there are at most `n` swaps total, so the RNG draw is bounded — no
/// unbounded work. Because non-tied elements form length-1 runs, `"random"` only
/// perturbs genuine ties; with no ties it is identical to the plain ranks.
fn b_rank(interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let x = first_positional(args)?;

    // The tie rule. R's default is "average"; we additionally support "min",
    // "max", "first", and "random" (R-31, RNG-backed). The keyword is read as a
    // character scalar, mirroring how `factor(levels=, labels=)` reads its strings.
    enum Ties {
        Average,
        Min,
        Max,
        First,
        Random,
    }
    let ties = match args.iter().find(|a| a.name.as_deref() == Some("ties.method")) {
        Some(arg) => match arg
            .value
            .as_character()
            .into_iter()
            .next()
            .flatten()
            .as_deref()
        {
            Some("average") | None => Ties::Average,
            Some("min") => Ties::Min,
            Some("max") => Ties::Max,
            Some("first") => Ties::First,
            Some("random") => Ties::Random,
            Some(other) => {
                return Err(SError::BadArgs(format!(
                    "unknown ties.method {other:?} (expected \"average\", \"min\", \"max\", \"first\", or \"random\")"
                )));
            }
        },
        None => Ties::Average,
    };
    // Compute a totally-ordered comparison and a "same key" test from one coerced
    // form. For numeric vectors we rank on the f64 values (NA sorts last, like R's
    // default na.last); for character (or anything else) we rank on the string keys.
    let n = x.length();
    if n == 0 {
        return Ok(SValue::doubles(vec![]));
    }

    // `keys[i]` is the order-and-equality key for the original element `i`.
    // We use the character rendering for everything except a pure numeric vector,
    // where numeric comparison is what R uses (and "10" must rank above "9").
    enum Keys {
        Num(Vec<f64>),
        Str(Vec<Option<String>>),
    }
    let keys = match x.strip_names().strip_attrs() {
        SValue::Character(_) | SValue::Factor { .. } => Keys::Str(x.as_character()),
        other => Keys::Num(other.as_double()?.iter().collect()),
    };

    let mut order: Vec<usize> = (0..n).collect();
    match &keys {
        Keys::Num(v) => order.sort_by(|&a, &b| {
            // NA (NaN) sorts last, mirroring R's default `na.last = TRUE`.
            match (is_na_real(v[a]), is_na_real(v[b])) {
                (true, true) => std::cmp::Ordering::Equal,
                (true, false) => std::cmp::Ordering::Greater,
                (false, true) => std::cmp::Ordering::Less,
                (false, false) => v[a].partial_cmp(&v[b]).unwrap_or(std::cmp::Ordering::Equal),
            }
        }),
        Keys::Str(v) => order.sort_by(|&a, &b| match (&v[a], &v[b]) {
            (None, None) => std::cmp::Ordering::Equal,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (Some(_), None) => std::cmp::Ordering::Less,
            (Some(a), Some(b)) => a.cmp(b),
        }),
    }

    // Two sorted neighbours are tied iff their keys are equal.
    let tied = |a: usize, b: usize| -> bool {
        match &keys {
            Keys::Num(v) => v[a] == v[b] || (is_na_real(v[a]) && is_na_real(v[b])),
            Keys::Str(v) => v[a] == v[b],
        }
    };

    // Walk the sorted permutation in runs of equal keys; each run is scored per
    // the chosen `ties.method`. A run occupies the 1-based positions `lo ..= hi`.
    let mut ranks = vec![0.0_f64; n];
    let mut p = 0usize;
    while p < n {
        let mut q = p + 1;
        while q < n && tied(order[p], order[q]) {
            q += 1;
        }
        let lo = (p + 1) as f64; // first 1-based position in the run
        let hi = q as f64; // last 1-based position in the run
        match ties {
            Ties::Average => {
                let avg = (lo + hi) / 2.0;
                for k in p..q {
                    ranks[order[k]] = avg;
                }
            }
            Ties::Min => {
                for k in p..q {
                    ranks[order[k]] = lo;
                }
            }
            Ties::Max => {
                for k in p..q {
                    ranks[order[k]] = hi;
                }
            }
            Ties::First => {
                // `order` lists tied indices in original order (stable sort), so
                // walking the run assigns consecutive ranks lo, lo+1, … in that
                // order — distinct ranks, no ties.
                for (offset, k) in (p..q).enumerate() {
                    ranks[order[k]] = lo + offset as f64;
                }
            }
            Ties::Random => {
                // Like "first" (consecutive ranks lo..=hi), but the tied positions
                // receive those ranks in a uniformly random order. We shuffle the
                // run's slice of `order` in place with Fisher–Yates, drawing each
                // swap target from the session RNG so the whole thing is reproducible
                // under `set.seed`. The shuffle is a no-op for a length-1 run, so
                // non-tied elements keep their natural rank.
                let m = q - p;
                let mut slot: Vec<usize> = order[p..q].to_vec();
                interp.sample_with(|rng| {
                    // Fisher–Yates: for j from m-1 down to 1, swap slot[j] with a
                    // uniformly chosen slot[0..=j]. `next_u32() % (j+1)` is bounded
                    // and consumes exactly one u32 per iteration (≤ n draws total).
                    for j in (1..m).rev() {
                        let k = (rng.next_u32() as usize) % (j + 1);
                        slot.swap(j, k);
                    }
                });
                for (offset, &orig) in slot.iter().enumerate() {
                    ranks[orig] = lo + offset as f64;
                }
            }
        }
        p = q;
    }
    Ok(SValue::doubles(ranks))
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

/// `is.environment(x)` (R-23) — `TRUE` iff `x` is a first-class environment
/// value, `FALSE` otherwise. A scalar predicate matching R: it inspects the
/// single value's *type*, so `is.environment(new.env())` is `TRUE` and
/// `is.environment(1)` / `is.environment("e")` are `FALSE`. The tests assert a
/// closure's captured env through this (`is.environment(environment(f))`).
fn b_is_environment(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let v = first_positional(args)?;
    Ok(SValue::Logical(vec![Some(matches!(
        v,
        SValue::Environment(_)
    ))]))
}

/// `environment(f) <- e` (R-23), reached through the R-15/R-16 replacement-
/// function lvalue path (`f(x) <- v` ≡ ``x <- `f<-`(x, v)``). The replacement
/// convention passes `(x, value)`: `x` is the closure whose captured environment
/// is being set (positional[0]); `value` is the new environment (the `value =`
/// named arg, or positional[1]).
///
/// Closures are **immutable values** here, so we return a *fresh* `Closure` with
/// its `env` field swapped to `e` and the same `params`/`body` (the caller — the
/// replacement-assignment desugaring — rebinds the variable to this result, as R
/// does). The new closure now closes over `e`, so free variables in its body
/// resolve from `e`'s chain. Errors are clean (never panics): a non-closure `x`
/// is a `TypeError`, a missing or non-environment `value` is a `BadArgs`.
fn b_environment_replace(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let x = first_positional(args)?.clone();
    let value = args
        .iter()
        .find(|a| a.name.as_deref() == Some("value"))
        .map(|a| a.value.clone())
        .or_else(|| nth_positional(args, 1).cloned())
        .ok_or_else(|| {
            SError::BadArgs("environment(f) <- value: the replacement value is missing".into())
        })?;
    let new_env = match value {
        SValue::Environment(e) => e,
        other => {
            return Err(SError::BadArgs(format!(
                "environment(f) <- value: value must be an environment, got {}",
                other.type_name()
            )))
        }
    };
    match x {
        SValue::Closure { params, body, .. } => Ok(SValue::Closure {
            params,
            body,
            env: new_env,
        }),
        other => Err(SError::TypeError(format!(
            "environment(f) <- value: f must be a closure, got {}",
            other.type_name()
        ))),
    }
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
// Reflective call + list overlay (R-17)
// ===========================================================================

/// The largest number of arguments `do.call` will spread out of its `args`
/// list, and the largest list `modifyList` will build. Without this cap a
/// crafted `do.call(f, as.list(1:1e9))` (or a `modifyList` overlaying millions
/// of new names) would build an unbounded call frame / list and exhaust the
/// heap. A real R program never reaches anywhere near this — the most argument-
/// hungry builtins (`paste`, `c`) handle dozens, not tens of thousands — so a
/// generous 100k ceiling is invisible to legitimate use while fail-closing on
/// abuse. We reuse the same ceiling for `modifyList`'s result for the same
/// reason.
const MAX_DOCALL_ARGS: usize = 100_000;

/// View a value as a `(names, items)` list, seeing through the transparent
/// wrappers (`Classed`/`Attributed`/`Named`) so a classed or attribute-carrying
/// list still counts as a list. Returns `None` for anything that is not a list.
///
/// `NULL` is *not* a list here — callers that want to accept `NULL` as "the
/// empty list" (R's `do.call(f, NULL)`) handle that case explicitly before
/// calling this, so that a stray `NULL` argument elsewhere still errors.
fn as_list(value: &SValue) -> Option<(&[Option<String>], &[SValue])> {
    match value {
        SValue::List { names, items } => Some((names, items)),
        SValue::Classed { inner, .. } => as_list(inner),
        SValue::Attributed { inner, .. } => as_list(inner),
        SValue::Named { values, .. } => as_list(values),
        _ => None,
    }
}

/// `do.call(what, args)` — build and evaluate a call to `what` with the
/// elements of the list `args` as its arguments. `what` is a callable value or
/// a length-one string naming one (resolved in the global environment); each
/// element of `args` becomes one argument — unnamed elements are positional and
/// named elements are passed by name, in order. This reuses the interpreter's
/// `call_value` machinery (the same path `lapply`/`Reduce` use), so default
/// arguments, named/positional matching, and recycling all behave exactly as in
/// a direct call: `do.call(paste, list("a", "b", sep = "-"))` is `paste("a",
/// "b", sep = "-")` → `"a-b"`.
fn b_do_call(interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let positional: Vec<&SValue> = args.iter().filter(|a| a.name.is_none()).map(|a| &a.value).collect();
    let what = positional
        .first()
        .ok_or_else(|| SError::BadArgs("do.call: missing 'what' argument".into()))?;
    let arglist = positional.get(1).copied();

    // Resolve `what` to a callable: a callable value passes through; a length-one
    // string is looked up by name in the global environment. Anything else is an
    // error (never a panic).
    let callee = resolve_callable(interp, what)?;

    // `args` defaults to / accepts NULL as the empty argument list; otherwise it
    // must be a list. A non-list, non-NULL `args` is an error, not a panic.
    let call_args: Vec<Arg> = match arglist {
        None | Some(SValue::Null) => Vec::new(),
        Some(value) => {
            let (names, items) = as_list(value).ok_or_else(|| {
                SError::BadArgs(format!(
                    "do.call: second argument must be a list, got {}",
                    value.type_name()
                ))
            })?;
            if items.len() > MAX_DOCALL_ARGS {
                return Err(SError::BadArgs(format!(
                    "do.call: too many arguments (limit {MAX_DOCALL_ARGS})"
                )));
            }
            items
                .iter()
                .enumerate()
                .map(|(i, value)| {
                    // An empty-string name is treated as no name, matching R's
                    // positional-element handling.
                    let name = names
                        .get(i)
                        .and_then(|n| n.clone())
                        .filter(|s| !s.is_empty());
                    Arg {
                        name,
                        value: value.clone(),
                    }
                })
                .collect()
        }
    };

    interp.call_value(callee, &call_args)
}

/// Resolve `what` (for `do.call`) to a callable value: a callable passes
/// through; a length-one character string is looked up by name in the global
/// environment (an unknown or non-callable name is a clean error).
fn resolve_callable(interp: &Interpreter, what: &SValue) -> SResult<SValue> {
    let bare = what.strip_attrs().strip_names();
    if bare.is_callable() {
        return Ok(bare.clone());
    }
    if let SValue::Character(v) = bare {
        let name = v
            .first()
            .and_then(|o| o.clone())
            .ok_or_else(|| SError::BadArgs("do.call: 'what' name is NA".into()))?;
        let found = lookup(interp.global(), &name)
            .ok_or_else(|| SError::Undefined(name.clone()))?;
        if !found.is_callable() {
            return Err(SError::NotCallable(found.type_name().to_string()));
        }
        return Ok(found);
    }
    Err(SError::NotCallable(what.type_name().to_string()))
}

/// `modifyList(x, val)` — `x` with the elements of the list `val` overlaid by
/// name. A name in both `x` and `val` is replaced (in place); a name only in
/// `val` is appended; and a `val` element whose value is `NULL` removes that
/// name from the result (R's documented deletion semantics). Element order
/// follows `x` (with removals dropped) and then `val`'s new names in `val`
/// order. Both arguments must be lists.
fn b_modify_list(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let positional: Vec<&SValue> =
        args.iter().filter(|a| a.name.is_none()).map(|a| &a.value).collect();
    let x = positional
        .first()
        .ok_or_else(|| SError::BadArgs("modifyList: missing 'x' argument".into()))?;
    let val = positional
        .get(1)
        .ok_or_else(|| SError::BadArgs("modifyList: missing 'val' argument".into()))?;

    let (x_names, x_items) = as_list(x)
        .ok_or_else(|| SError::BadArgs(format!("modifyList: 'x' must be a list, got {}", x.type_name())))?;
    let (v_names, v_items) = as_list(val)
        .ok_or_else(|| SError::BadArgs(format!("modifyList: 'val' must be a list, got {}", val.type_name())))?;

    // Every element of `val` must be named (R errors on an unnamed overlay
    // element — there is no positional notion of "modify"). A names vector
    // shorter than the values, or a `None`/empty name, both count as unnamed.
    let all_named = v_names.len() == v_items.len()
        && v_names
            .iter()
            .all(|n| n.as_deref().is_some_and(|s| !s.is_empty()));
    if !all_named {
        return Err(SError::BadArgs(
            "modifyList: elements of 'val' must all be named".into(),
        ));
    }

    // Start from `x`. Walk `val` in order: replace an existing name in place,
    // mark a name whose value is NULL for removal, and queue a genuinely new
    // name for appending.
    let mut names: Vec<Option<String>> = x_names.to_vec();
    let mut items: Vec<SValue> = x_items.to_vec();
    let mut to_remove: Vec<usize> = Vec::new();

    // Iterate over the *values* so the index can never run past `v_items`
    // (the `List` fields are public, so we do not assume `v_names` and
    // `v_items` are the same length — a missing name is treated as unnamed,
    // which the all-named check below already rejects).
    for (i, new_value) in v_items.iter().enumerate() {
        let name = v_names.get(i).and_then(|n| n.as_deref()).unwrap_or_default();
        let pos = names.iter().position(|n| n.as_deref() == Some(name));
        match (pos, matches!(new_value, SValue::Null)) {
            // Existing name, NULL value → remove (record the index).
            (Some(p), true) => {
                if !to_remove.contains(&p) {
                    to_remove.push(p);
                }
            }
            // Existing name, non-NULL value → replace in place.
            (Some(p), false) => {
                items[p] = new_value.clone();
            }
            // New name, NULL value → nothing to remove, nothing to add.
            (None, true) => {}
            // New name, non-NULL value → append.
            (None, false) => {
                if names.len() + 1 > MAX_DOCALL_ARGS {
                    return Err(SError::BadArgs(format!(
                        "modifyList: result too large (limit {MAX_DOCALL_ARGS})"
                    )));
                }
                names.push(Some(name.to_string()));
                items.push(new_value.clone());
            }
        }
    }

    // Drop removed indices (highest-first so earlier indices stay valid).
    to_remove.sort_unstable();
    to_remove.dedup();
    for &idx in to_remove.iter().rev() {
        names.remove(idx);
        items.remove(idx);
    }

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

// ---------------------------------------------------------------------------
// R-34 — string utilities (startsWith / endsWith / trimws / chartr / strtoi)
// ---------------------------------------------------------------------------
//
// Design notes shared by all five:
//
//   * NA convention. A character element is `Option<String>`; `None` means `NA`.
//     We never invent text for an `NA`; an `NA` input yields an `NA` output.
//   * UTF-8 safety. Every operation works on Unicode scalar values, never on raw
//     byte offsets. `startsWith`/`endsWith` lean on `str::starts_with`/`ends_with`
//     (which compare whole code points), and `trimws`/`chartr`/`strtoi` iterate
//     `char`s. A multibyte string like "café" can therefore never be split mid
//     code point, so none of these can panic on real-world text.
//   * Recycling. `startsWith`/`endsWith` are binary and vectorized over *both*
//     arguments, recycled to the longer length (R's rule). The length is the max
//     of the two (already bounded by the input vectors), so the loop count cannot
//     overflow.

/// The recycled length of two vectors: the longer one, or `0` if either is empty
/// (R short-circuits a zero-length operand to a zero-length result).
fn recycle_len(a: usize, b: usize) -> usize {
    if a == 0 || b == 0 {
        0
    } else {
        a.max(b)
    }
}

/// Shared body for `startsWith` / `endsWith`. `test(haystack, needle)` decides a
/// single pair; both arguments are coerced to character and recycled to the
/// longer length, with `NA` in either position producing an `NA` result.
fn affix_test(
    args: &[Arg],
    needle_name: &str,
    test: impl Fn(&str, &str) -> bool,
) -> SResult<SValue> {
    let x = nth_positional(args, 0)
        .ok_or_else(|| SError::BadArgs("argument \"x\" is missing".into()))?
        .as_character();
    let affix = nth_positional(args, 1)
        .ok_or_else(|| SError::BadArgs(format!("argument {needle_name:?} is missing")))?
        .as_character();

    let n = recycle_len(x.len(), affix.len());
    let out: Vec<Option<bool>> = (0..n)
        .map(|i| {
            // The recycle is modular; both operands are non-empty here (n == 0
            // when either is, so this closure never runs in that case).
            match (&x[i % x.len()], &affix[i % affix.len()]) {
                (Some(s), Some(p)) => Some(test(s, p)),
                _ => None, // NA in either operand → NA.
            }
        })
        .collect();
    Ok(SValue::Logical(out))
}

/// `startsWith(x, prefix)` — `TRUE` where `x[i]` begins with `prefix[i]`.
/// Vectorized and recycled over both arguments; `NA` → `NA`.
fn b_starts_with(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    affix_test(args, "prefix", |s, p| s.starts_with(p))
}

/// `endsWith(x, suffix)` — the trailing-edge analogue of `startsWith`.
fn b_ends_with(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    affix_test(args, "suffix", |s, p| s.ends_with(p))
}

/// Base R's default `trimws` whitespace class, `[ \t\r\n]` — a space, tab, CR,
/// and LF. Used verbatim when no `whitespace =` argument is supplied (R-37). We
/// keep it as the literal regex source (rather than `char::is_whitespace`) so the
/// default and the custom path share one code route and stay locale-free.
const TRIMWS_DEFAULT_WS: &str = "[ \t\r\n]";

/// `trimws(x, which = "both", whitespace = "[ \t\r\n]")` — strip leading and/or
/// trailing whitespace from each element. `which` is the second positional or the
/// `which =` named arg and must be one of `"both"`, `"left"`, `"right"`; anything
/// else is a clean error. `NA` elements pass through unchanged.
///
/// **`whitespace =` (R-37).** The set of characters to strip is a *regular
/// expression* (faithful to base R ≥ 3.6), defaulting to [`TRIMWS_DEFAULT_WS`].
/// We compile it once with the same RE2-based `regex` engine `grepl`/`gsub` use,
/// wrapped as a non-capturing group repeated one-or-more times and anchored to the
/// edge being trimmed: `^(?:p)+` for the left, `(?:p)+$` for the right. Only a run
/// of the class at the very start/end is removed; interior matches are untouched.
/// Because RE2 matches in guaranteed linear time with no backtracking, no
/// `whitespace =` pattern can cause catastrophic backtracking (no ReDoS). An
/// invalid pattern is a clean `Err`. All slicing uses the byte offset that the
/// regex itself reports for a whole-`char` match, so multibyte input is UTF-8 safe.
fn b_trimws(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let x = first_positional(args)?.as_character();
    // `which =` named arg wins; else the second positional; else "both".
    // (`whitespace` is keyword-only, so it never collides with the positional.)
    let which = args
        .iter()
        .find(|a| a.name.as_deref() == Some("which"))
        .map(|a| &a.value)
        .or_else(|| {
            args.iter()
                .filter(|a| a.name.is_none())
                .nth(1)
                .map(|a| &a.value)
        })
        .map(|v| v.as_character())
        .and_then(|c| c.into_iter().next().flatten())
        .unwrap_or_else(|| "both".to_string());

    let (trim_left, trim_right) = match which.as_str() {
        "both" => (true, true),
        "left" => (true, false),
        "right" => (false, true),
        other => {
            return Err(SError::BadArgs(format!(
                "trimws: 'which' must be one of \"both\", \"left\", \"right\" (got {other:?})"
            )));
        }
    };

    // The `whitespace =` pattern (keyword-only); default is R's `[ \t\r\n]`.
    let ws = args
        .iter()
        .find(|a| a.name.as_deref() == Some("whitespace"))
        .map(|a| a.value.as_character())
        .and_then(|c| c.into_iter().next().flatten())
        .unwrap_or_else(|| TRIMWS_DEFAULT_WS.to_string());

    // Compile each edge's anchored matcher *once* (not per element). A bad pattern
    // surfaces here as a clean error before we touch any data.
    let left_re = if trim_left {
        Some(compile(&format!("^(?:{ws})+"), false)?)
    } else {
        None
    };
    let right_re = if trim_right {
        Some(compile(&format!("(?:{ws})+$"), false)?)
    } else {
        None
    };

    let out = x
        .into_iter()
        .map(|o| {
            o.map(|s| {
                let mut t: &str = &s;
                // Strip a leading run: the match (if any) starts at byte 0, so we
                // keep the tail after `m.end()` — a regex-reported byte boundary,
                // hence always a valid UTF-8 char boundary.
                if let Some(re) = &left_re {
                    if let Some(m) = re.find(t) {
                        t = &t[m.end()..];
                    }
                }
                // Strip a trailing run: the `$`-anchored match ends at the string
                // end, so we keep the head before `m.start()` (also a char boundary).
                if let Some(re) = &right_re {
                    if let Some(m) = re.find(t) {
                        t = &t[..m.start()];
                    }
                }
                t.to_string()
            })
        })
        .collect();
    Ok(SValue::Character(out))
}

/// `chartr(old, new, x)` — translate each character of `x` found at position *i*
/// of `old` into the character at position *i* of `new`. `old` and `new` are
/// single strings of **equal `nchar`** (else an error); the mapping is built by
/// zipping their `char`s, so multibyte code points map as whole units (UTF-8
/// safe). Vectorized over `x`; an `NA` element stays `NA`. When `old` repeats a
/// character, R uses the *first* mapping; `HashMap::entry(...).or_insert` keeps
/// that first-wins behavior.
fn b_chartr(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let old = nth_positional(args, 0)
        .map(|v| v.as_character())
        .and_then(|c| c.into_iter().next().flatten())
        .ok_or_else(|| SError::BadArgs("chartr: 'old' is missing".into()))?;
    let new = nth_positional(args, 1)
        .map(|v| v.as_character())
        .and_then(|c| c.into_iter().next().flatten())
        .ok_or_else(|| SError::BadArgs("chartr: 'new' is missing".into()))?;
    let x = nth_positional(args, 2)
        .ok_or_else(|| SError::BadArgs("chartr: 'x' is missing".into()))?
        .as_character();

    // Compare by code-point count, not byte length: "é" (one char, two bytes)
    // must match a one-char replacement.
    let old_chars: Vec<char> = old.chars().collect();
    let new_chars: Vec<char> = new.chars().collect();
    if old_chars.len() != new_chars.len() {
        return Err(SError::BadArgs(
            "chartr: 'old' and 'new' must be the same length".into(),
        ));
    }

    let mut table: std::collections::HashMap<char, char> =
        std::collections::HashMap::with_capacity(old_chars.len());
    for (o, n) in old_chars.into_iter().zip(new_chars) {
        table.entry(o).or_insert(n); // first mapping wins, matching R.
    }

    let out = x
        .into_iter()
        .map(|o| {
            o.map(|s| {
                s.chars()
                    .map(|c| *table.get(&c).unwrap_or(&c))
                    .collect::<String>()
            })
        })
        .collect();
    Ok(SValue::Character(out))
}

/// `strtoi(x, base = 10L)` — parse each string as an integer in the given base
/// (2..36), returning a `Double` vector with `NA` for anything unparseable.
///
/// Semantics follow C `strtol`, as R does:
///   * leading ASCII whitespace is skipped; an optional `+`/`-` sign is honored;
///   * for base 16 an optional `0x`/`0X` prefix is accepted;
///   * the **whole remaining string must be consumed** — trailing garbage
///     (including trailing whitespace) makes the element `NA`;
///   * an empty (or sign-only / prefix-only) string is `NA`;
///   * a digit outside the base's range makes the element `NA`;
///   * a `base` outside `{0} ∪ 2..36` makes **every** element `NA`.
///
/// `base = 0L` auto-detects each string's radix from its prefix (R-37; see
/// [`parse_strtoi`]). Accumulation is `i64`-checked, so a long all-digits string
/// overflows to `NA` rather than panicking.
fn b_strtoi(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let x = first_positional(args)?.as_character();
    let base = args
        .iter()
        .find(|a| a.name.as_deref() == Some("base"))
        .map(|a| &a.value)
        .or_else(|| nth_positional(args, 1));
    // `scalar_int` truncates toward zero and defaults to 10 when missing/NA.
    let base = scalar_int(base, 10);

    let out: Vec<f64> = x
        .into_iter()
        .map(|o| match o {
            None => na_real(),
            Some(s) => match parse_strtoi(&s, base) {
                Some(v) => v as f64,
                None => na_real(),
            },
        })
        .collect();
    Ok(SValue::doubles(out))
}

/// Parse a single string for [`b_strtoi`]. Returns `None` (→ `NA`) for any base
/// outside `{0} ∪ 2..=36`, an empty/garbage string, an out-of-range digit, or an
/// `i64` overflow. Never panics: every step is a `checked_*` arithmetic op or a
/// bounded `char` scan.
///
/// **`base == 0` (R-37).** The radix is inferred from the post-sign prefix, exactly
/// as C `strtol(…, 0)`:
///
/// | post-sign prefix        | inferred base | example         |
/// |-------------------------|---------------|-----------------|
/// | `0x` / `0X` + hex digits | 16           | `"0x1F"` → 31   |
/// | `0` + another digit      | 8            | `"010"` → 8     |
/// | exactly `"0"`            | 10 (value 0) | `"0"` → 0       |
/// | anything else            | 10           | `"12"` → 12     |
///
/// Because octal is then parsed in base 8, a stray `8`/`9` after a leading `0`
/// (e.g. `"08"`) is an out-of-range digit and yields `None`. A bare `0x`/`0X`
/// with no following digits also yields `None` (empty digit run).
fn parse_strtoi(s: &str, base: i64) -> Option<i64> {
    // Base must be representable and in {0} ∪ 2..=36; anything else is NA.
    if base != 0 && !(2..=36).contains(&base) {
        return None;
    }

    // Skip leading ASCII whitespace (strtol's `isspace`), then read an optional
    // sign. We work on the char iterator so multibyte tails can't desync indices.
    let trimmed = s.trim_start_matches(|c: char| c.is_ascii_whitespace());
    let (negative, rest) = match trimmed.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, trimmed.strip_prefix('+').unwrap_or(trimmed)),
    };

    // Resolve the effective base and strip any radix prefix from the digit run.
    //   * base 0  → auto-detect from the prefix (R-37);
    //   * base 16 → tolerate an optional `0x`/`0X` prefix (strtol convention);
    //   * otherwise → the digits are taken literally.
    let (base, digits) = if base == 0 {
        detect_base0_prefix(rest)
    } else if base == 16 {
        let d = rest
            .strip_prefix("0x")
            .or_else(|| rest.strip_prefix("0X"))
            .unwrap_or(rest);
        (16u32, d)
    } else {
        (base as u32, rest)
    };

    // Require at least one digit, and the *entire* remainder to be valid digits
    // in this base (no trailing garbage, no embedded whitespace).
    if digits.is_empty() {
        return None;
    }
    let mut acc: i64 = 0;
    for c in digits.chars() {
        let d = c.to_digit(base)?; // out-of-base char or non-digit → None.
        acc = acc.checked_mul(base as i64)?.checked_add(d as i64)?;
    }
    if negative {
        acc = acc.checked_neg()?;
    }
    Some(acc)
}

/// `strtoi(x, base = 0L)` prefix detection (R-37). Given the post-sign remainder
/// `rest`, return `(effective_base, digit_run)`:
///
///   * a `0x`/`0X` prefix → base 16, with the prefix stripped;
///   * a leading `0` *followed by at least one more character* → base 8, with the
///     leading `0` stripped (so `"010"` parses the run `"10"` in base 8 → 8, and
///     `"08"` parses `"8"` in base 8 → an out-of-range digit → `NA`);
///   * a lone `"0"` → base 10, digit run `"0"` (the number zero);
///   * anything else → base 10, digits taken as-is.
///
/// Pure prefix inspection (no allocation); the caller does the actual digit scan
/// and so still rejects an empty digit run (e.g. a bare `"0x"`) as `NA`.
fn detect_base0_prefix(rest: &str) -> (u32, &str) {
    if let Some(d) = rest.strip_prefix("0x").or_else(|| rest.strip_prefix("0X")) {
        (16, d)
    } else if let Some(d) = rest.strip_prefix('0') {
        // A leading `0` with more text behind it is octal; a lone "0" is decimal
        // zero (octal "" would be an empty run → NA, which is wrong for "0").
        if d.is_empty() {
            (10, rest) // exactly "0": decimal, value 0.
        } else {
            (8, d) // octal: parse the digits after the leading 0.
        }
    } else {
        (10, rest)
    }
}

/// Hard cap on any single field width or precision (1 MiB). A user-supplied
/// `fmt`, `width`, `nsmall`, or `digits` is *data*; this bound is what stops a
/// crafted spec like `%999999999d` or `formatC(x, width = 1e9)` from triggering
/// a giant allocation (a width-DoS). Shared by `sprintf`, `format`, and
/// `formatC`. There is no `%n`-style conversion, so there is no
/// write-what-where primitive to worry about — only allocation size.
const MAX_FIELD: usize = 1 << 20;

/// Hard cap on the **total** padded output of a single `format`/`formatC` call
/// (256 MiB). `MAX_FIELD` bounds each *element*, but a long vector formatted to
/// a wide common field multiplies: `format(rep(0, 1e7), width = 1e6)` would be
/// ~10 TB. This budget bounds the *product* (`common_width × element_count`) so
/// no short expression can demand an unbounded allocation. Oversize is a clean
/// `BadArgs` error, never an OOM abort.
const MAX_TOTAL_OUTPUT: usize = 256 << 20;

/// Reject a `format`/`formatC` request whose padded output (`per_element`
/// columns × `count` elements) would exceed [`MAX_TOTAL_OUTPUT`]. Uses
/// saturating arithmetic so the multiplication itself can never overflow.
fn check_output_budget(per_element: usize, count: usize) -> SResult<()> {
    if per_element.saturating_mul(count) > MAX_TOTAL_OUTPUT {
        return Err(SError::BadArgs(
            "format: total output size is too large".into(),
        ));
    }
    Ok(())
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

// ===========================================================================
// Output formatting (R-27)
// ===========================================================================
//
// A small family of pure builtins for turning numbers and vectors into
// human-readable text. The design goal is **determinism**: nothing here reads
// the clock or the host locale. The default thousands separator is `","` and
// the decimal point is `"."`, fixed everywhere — so a CI run in a `de_DE`
// locale (where R's real `format()` would use `.` for thousands and `,` for the
// decimal) produces exactly the same bytes as a run in `C`/`en_US`.
//
//   ┌───────────────┬──────────────────────────────────────────────────────┐
//   │ format(x,…)   │ general formatter; numeric vectors pad to a COMMON     │
//   │               │ width (R right-justifies all to the widest element)    │
//   │ formatC(x,…)  │ C-style printf wrapper: format="d"/"f"/"e"/"g"/"s"/"x" │
//   │ prettyNum(x)  │ insert a thousands separator into the integer part     │
//   │ toString(x)   │ collapse a whole vector into ONE comma-joined string   │
//   └───────────────┴──────────────────────────────────────────────────────┘

/// Read a scalar named argument as a non-negative field count (`width`,
/// `nsmall`, `digits`). Absent → `None`. Present → the value coerced to a
/// double, floored, and **clamped to `MAX_FIELD`**; a negative or non-finite
/// value is treated as `0`. The clamp is the width-DoS guard: a caller asking
/// for `width = 1e9` gets `MAX_FIELD`, not a 1-GB allocation.
fn named_count(args: &[Arg], name: &str) -> Option<usize> {
    let raw = args
        .iter()
        .find(|a| a.name.as_deref() == Some(name))?
        .value
        .as_double()
        .ok()?
        .get_value(0)?;
    if !raw.is_finite() || raw < 0.0 {
        return Some(0);
    }
    Some((raw as usize).min(MAX_FIELD))
}

/// Read a scalar named *string* argument (`justify`, `big.mark`, `format`,
/// `flag`, `sep`). Absent or `NA` → `None`.
fn named_str(args: &[Arg], name: &str) -> Option<String> {
    args.iter()
        .find(|a| a.name.as_deref() == Some(name))?
        .value
        .as_character()
        .into_iter()
        .next()
        .flatten()
}

/// Insert `mark` between every group of three digits of an integer-part string,
/// counting from the right. `"1234567"` → `"1,234,567"`. A leading sign is
/// preserved and never grouped. Pure string surgery, no locale.
fn group_thousands(int_part: &str, mark: &str) -> String {
    if mark.is_empty() {
        return int_part.to_string();
    }
    // Split off an optional leading sign so we group only the digits.
    let (sign, digits) = match int_part.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", int_part),
    };
    // Group from the RIGHT. The leading group is the remainder (1, 2, or 3
    // digits); every group after it is exactly 3. `"1234567"` → first = 1 →
    // "1" | "234" | "567" → "1,234,567".
    let bytes = digits.as_bytes();
    let len = bytes.len();
    let first = if len % 3 == 0 { 3 } else { len % 3 };
    let mut grouped = String::with_capacity(len + len / 3 * mark.len());
    let mut idx = 0usize;
    while idx < len {
        let take = if idx == 0 { first } else { 3 };
        if idx != 0 {
            grouped.push_str(mark);
        }
        grouped.push_str(std::str::from_utf8(&bytes[idx..idx + take]).unwrap_or(""));
        idx += take;
    }
    format!("{sign}{grouped}")
}

/// Format one finite number for `format`/`prettyNum`: when `nsmall > 0`, render
/// with **exactly** `nsmall` decimal places (so `format(3, nsmall = 2)` is
/// `"3.00"` and `format(3.14159, nsmall = 2)` is `"3.14"`); when `nsmall == 0`,
/// use R's default rendering (`format_number`). Then optionally group the
/// integer part with `big.mark`. Non-finite / NA delegate to `format_number`.
///
/// (R's real `nsmall` is a *minimum* applied on top of `getOption("digits")`
/// significant-digit rounding; this subset treats a supplied `nsmall` as the
/// decimal count directly — simpler, fully deterministic, and matching the
/// documented R-27 examples. The `scientific=`/significant-digit corner is
/// deferred to R-28.)
fn format_number_nsmall(x: f64, nsmall: usize, big_mark: &str) -> String {
    if !x.is_finite() {
        return format_number(x);
    }
    let rendered = if nsmall == 0 {
        format_number(x)
    } else {
        format!("{x:.nsmall$}")
    };
    if big_mark.is_empty() {
        return rendered;
    }
    // Group only the integer part; leave any fractional part untouched.
    match rendered.split_once('.') {
        Some((int, frac)) => format!("{}.{}", group_thousands(int, big_mark), frac),
        None => group_thousands(&rendered, big_mark),
    }
}

/// `format(x, nsmall=, width=, justify=, big.mark=)` — the general formatter.
///
/// * **Numeric `x`** — each element is rendered with `nsmall` decimal places and
///   an optional `big.mark` thousands separator, then **the whole vector is
///   padded to a common width**: R right-justifies every element to the width of
///   the widest (so columns line up). `width` raises that common width to at
///   least its value.
/// * **Character `x`** — each element is padded to the common width (the max of
///   `width` and the widest element) honouring `justify`: `"left"` (default),
///   `"right"`, or `"centre"`.
/// * **Logical `x`** — coerced to `"TRUE"`/`"FALSE"` strings, then treated as a
///   character vector.
///
/// Returns a character vector the same length as `x` (length-0 in, length-0
/// out). `NA` renders as the string `"NA"`.
fn b_format(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let x = first_positional(args)?;
    // R-44: `format()` is an S3 generic. A value carrying class "Date" routes to
    // the Date renderer (`format.Date`) — so `format(as.Date("2021-03-14"))`
    // yields "2021-03-14" rather than the bare day count.
    //
    // R-46: check "POSIXct" *first* — a POSIXct's class is c("POSIXct","POSIXt")
    // and does NOT contain "Date", but ordering the date-time renderer ahead keeps
    // the intent explicit and is robust to any future shared class.
    if is_posixct(x) {
        return b_format_posixct(_interp, args);
    }
    if is_date(x) {
        return b_format_date(_interp, args);
    }
    let nsmall = named_count(args, "nsmall").unwrap_or(0);
    let min_width = named_count(args, "width").unwrap_or(0);
    let big_mark = named_str(args, "big.mark").unwrap_or_default();
    let justify = named_str(args, "justify").unwrap_or_else(|| "left".to_string());

    // Render each element to its natural string first; common-width padding is
    // a second pass so we can measure the widest.
    let is_numeric = matches!(peel_structural(x), SValue::Double(_) | SValue::Matrix { .. });
    let rendered: Vec<String> = if is_numeric {
        let d = x.as_double()?;
        d.iter()
            .map(|v| {
                if is_na_real(v) {
                    "NA".to_string()
                } else {
                    format_number_nsmall(v, nsmall, &big_mark)
                }
            })
            .collect()
    } else {
        x.as_character()
            .into_iter()
            .map(|o| o.unwrap_or_else(|| "NA".to_string()))
            .collect()
    };
    if rendered.is_empty() {
        return Ok(SValue::Character(vec![]));
    }

    let widest = rendered.iter().map(|s| s.chars().count()).max().unwrap_or(0);
    let target = widest.max(min_width).min(MAX_FIELD);
    // Bound the *product* (common width × element count), not just each field.
    check_output_budget(target, rendered.len())?;

    // Numeric vectors always right-justify (R lines decimals up on the right);
    // character vectors honour `justify`.
    let out: Vec<Option<String>> = rendered
        .into_iter()
        .map(|s| {
            let padded = if is_numeric {
                pad(&s, target, false, false)
            } else {
                pad_justify(&s, target, &justify)
            };
            Some(padded)
        })
        .collect();
    Ok(SValue::Character(out))
}

/// Pad `body` to `width` columns honouring an R `justify` keyword. `"right"`
/// pads on the left; `"centre"`/`"center"` splits the slack (extra on the
/// right); anything else (including `"left"`) pads on the right.
fn pad_justify(body: &str, width: usize, justify: &str) -> String {
    let len = body.chars().count();
    if len >= width {
        return body.to_string();
    }
    let slack = width - len;
    match justify {
        "right" => format!("{}{}", " ".repeat(slack), body),
        "centre" | "center" => {
            let left = slack / 2;
            let right = slack - left;
            format!("{}{}{}", " ".repeat(left), body, " ".repeat(right))
        }
        _ => format!("{}{}", body, " ".repeat(slack)),
    }
}

/// `formatC(x, format=, digits=, width=, flag=)` — the C-style formatter, a
/// thin wrapper over the shared `printf` engine (`render_conversion` + `pad`).
///
/// * `format` — `"d"` integer, `"f"` fixed, `"e"` scientific, `"g"` shortest,
///   `"s"` string, `"x"` hex (integers). Default: `"d"` for numerics, `"s"` for
///   character `x`.
/// * `digits` — precision passed to the conversion.
/// * `width` — minimum field width (clamped to `MAX_FIELD`).
/// * `flag` — a string of `printf` flags: `"-"` left-justify, `"0"` zero-pad,
///   `"+"` force a leading sign on numbers.
///
/// Vectorized over `x`; returns a character vector of the same length.
fn b_format_c(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let x = first_positional(args)?;
    let width = named_count(args, "width").unwrap_or(0);
    let digits = named_count(args, "digits");
    let flag = named_str(args, "flag").unwrap_or_default();
    let left = flag.contains('-');
    let zero = flag.contains('0');
    let plus = flag.contains('+');

    let is_char = matches!(
        peel_structural(x),
        SValue::Character(_) | SValue::Factor { .. }
    );
    let format =
        named_str(args, "format").unwrap_or_else(|| if is_char { "s".into() } else { "d".into() });
    let conv = format.chars().next().unwrap_or('s');

    let n = x.length();
    if n == 0 {
        return Ok(SValue::Character(vec![]));
    }
    // Bound the *product*: each element is at most `width` columns from padding
    // plus roughly `digits` columns from a high-precision `%f`/`%e` body, so the
    // per-element estimate is `max(width, digits)`. Reject before rendering.
    check_output_budget(width.max(digits.unwrap_or(0)), n)?;
    let chars = x.as_character();
    let doubles = x.as_double().ok();

    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        // NA renders as the literal "NA" in every conversion.
        let is_na = match &doubles {
            Some(d) => d.get_value(i).map(is_na_real).unwrap_or(true),
            None => chars.get(i).map(|o| o.is_none()).unwrap_or(true),
        };
        let body = if is_na && conv != 's' {
            "NA".to_string()
        } else {
            format_c_one(conv, &doubles, &chars, i, digits)?
        };
        // A leading `+` forces a sign on non-negative numbers (the printf flag).
        let signed = if plus && conv != 's' && !body.starts_with('-') && body != "NA" {
            format!("+{body}")
        } else {
            body
        };
        out.push(Some(pad(&signed, width, left, zero && !left)));
    }
    Ok(SValue::Character(out))
}

/// Render one element for `formatC` under conversion `conv`. Reuses the
/// `sprintf` engine for `d`/`f`/`e`/`g`/`s`; adds `x` (hex of an integer).
fn format_c_one(
    conv: char,
    doubles: &Option<Double>,
    chars: &[Option<String>],
    i: usize,
    digits: Option<usize>,
) -> SResult<String> {
    if conv == 'x' {
        // Hex of the integer value; NA/non-finite → "NA".
        let v = doubles.as_ref().and_then(|d| d.get_value(i));
        return Ok(match v {
            Some(x) if x.is_finite() => format!("{:x}", x as i64),
            _ => "NA".to_string(),
        });
    }
    if conv == 's' {
        return Ok(chars
            .get(i)
            .cloned()
            .flatten()
            .unwrap_or_else(|| "NA".to_string()));
    }
    // d/f/e/g go through the shared sprintf conversion renderer. Wrap the
    // element as a one-row SValue so `render_conversion`'s recycling indexes it.
    let sval = match doubles {
        Some(d) => SValue::Double(Double::from_values(vec![d.get_value(i).unwrap_or(f64::NAN)])),
        None => SValue::Character(vec![chars.get(i).cloned().flatten()]),
    };
    render_conversion(conv, Some(&sval), 0, digits)
}

/// `prettyNum(x, big.mark = ",")` — insert a thousands separator into each
/// number's integer part. A convenience wrapper: `prettyNum(x, big.mark = m)`
/// is `format(x)` with grouping but no common-width padding and no `nsmall`.
fn b_pretty_num(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let x = first_positional(args)?;
    let big_mark = named_str(args, "big.mark").unwrap_or_else(|| ",".to_string());
    // Each number's integer part is ≤ ~309 digits, so a long `big.mark` grouped
    // across a long vector can still amplify (~100 groups × mark.len × count).
    // Bound the product the same way `format`/`formatC` do.
    check_output_budget(big_mark.len().saturating_mul(110).max(1), x.length())?;
    let out: Vec<Option<String>> = match peel_structural(x) {
        SValue::Double(_) | SValue::Matrix { .. } => x
            .as_double()?
            .iter()
            .map(|v| {
                if is_na_real(v) {
                    Some("NA".to_string())
                } else {
                    Some(format_number_nsmall(v, 0, &big_mark))
                }
            })
            .collect(),
        // Non-numeric prettyNum just coerces to character (matching R, which
        // applies big.mark only to the numeric-looking integer part).
        _ => x
            .as_character()
            .into_iter()
            .map(|o| match o {
                Some(s) => Some(group_thousands_if_numeric(&s, &big_mark)),
                None => Some("NA".to_string()),
            })
            .collect(),
    };
    Ok(SValue::Character(out))
}

/// For a character `prettyNum`: if the whole string parses as a number, group
/// it; otherwise pass it through unchanged.
fn group_thousands_if_numeric(s: &str, mark: &str) -> String {
    match s.split_once('.') {
        Some((int, frac))
            if int.parse::<i64>().is_ok() && frac.chars().all(|c| c.is_ascii_digit()) =>
        {
            format!("{}.{}", group_thousands(int, mark), frac)
        }
        None if s.parse::<i64>().is_ok() => group_thousands(s, mark),
        _ => s.to_string(),
    }
}

/// `toString(x, sep = ", ")` — collapse a whole vector into a **single** string,
/// joining the character-coerced elements with `sep`. `toString(1:3)` is
/// `"1, 2, 3"`. Always length-1 (an empty vector gives `""`).
fn b_to_string(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let x = first_positional(args)?;
    let sep = named_str(args, "sep").unwrap_or_else(|| ", ".to_string());
    let parts: Vec<String> = x
        .as_character()
        .into_iter()
        .map(|o| o.unwrap_or_else(|| "NA".to_string()))
        .collect();
    Ok(SValue::Character(vec![Some(parts.join(&sep))]))
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

/// `Reduce(f, x[, init][, accumulate])` — left fold. Without `init`, `f` is
/// first applied to the first two elements; an empty `x` with no `init` is
/// `NULL`.
///
/// **`accumulate` (R-20).** When `accumulate = TRUE`, the result is not the
/// single final fold but the sequence of *running* folds, combined with `c()`
/// (so atomic folds simplify to a vector, list folds stay a list):
///
/// ```text
/// Reduce(\(a, b) a + b, 1:4)                     -> 10           (final only)
/// Reduce(\(a, b) a + b, 1:4, accumulate = TRUE)  -> c(1, 3, 6, 10)
/// Reduce(\(a, b) a + b, 1:3, 10, accumulate=TRUE)-> c(10, 11, 13, 16)
/// ```
///
/// With an `init`, the init is the **first** accumulated element. The number of
/// accumulated elements is `length(x)` (or `length(x) + 1` with an init), which
/// is itself bounded by [`MAX_SEQ_LEN`]: `x` cannot be longer than that, so the
/// accumulator vector is bounded too — no extra cap is needed.
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
    // `accumulate =` (named only; defaults to FALSE) selects running-fold output.
    let accumulate = match args.iter().find(|a| a.name.as_deref() == Some("accumulate")) {
        Some(a) => a.value.truthy()?,
        None => false,
    };

    let n = x.length();
    let (mut acc, start) = match init {
        Some(v) => (v, 0),
        None if n == 0 => return Ok(SValue::Null),
        None => (nth_element(&x, 0), 1),
    };

    // The running folds (only collected when `accumulate`), starting from `acc`.
    let mut running: Vec<Arg> = Vec::new();
    if accumulate {
        running.push(Arg {
            name: None,
            value: acc.clone(),
        });
    }

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
        if accumulate {
            running.push(Arg {
                name: None,
                value: acc.clone(),
            });
        }
    }
    if accumulate {
        Ok(combine(&running))
    } else {
        Ok(acc)
    }
}

/// `Find(f, x)` — the **first** element of `x` for which `f(element)` is `TRUE`,
/// or an invisible `NULL` if none matches. Short-circuits on the first hit
/// (unlike `Filter`, which scans the whole vector). `f` is taken by `f =` / the
/// first callable positional (the R-10 `split_fun` helper), so it composes with
/// the pipe: `1:5 |> Find(f = \(x) x > 2)` is `3`.
fn b_find(interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let (f, data) = split_fun(args, "Find")?;
    let x = data
        .into_iter()
        .next()
        .ok_or_else(|| SError::BadArgs("Find: missing x".into()))?;
    for i in 0..x.length() {
        let element = nth_element(&x, i);
        let verdict = interp.call_value(
            f.clone(),
            &[Arg {
                name: None,
                value: element.clone(),
            }],
        )?;
        if verdict.truthy()? {
            return Ok(element);
        }
    }
    Ok(SValue::Null)
}

/// `Position(f, x)` — the **1-based index** of the first element for which
/// `f(element)` is `TRUE`, or `NULL` if none. The index counterpart to `Find`
/// (which returns the value).
fn b_position(interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let (f, data) = split_fun(args, "Position")?;
    let x = data
        .into_iter()
        .next()
        .ok_or_else(|| SError::BadArgs("Position: missing x".into()))?;
    for i in 0..x.length() {
        let verdict = interp.call_value(
            f.clone(),
            &[Arg {
                name: None,
                value: nth_element(&x, i),
            }],
        )?;
        if verdict.truthy()? {
            // R indexes from 1.
            return Ok(SValue::scalar((i + 1) as f64));
        }
    }
    Ok(SValue::Null)
}

/// `Negate(f)` — return a **new function** computing `!f(...)`. The wrapped `f`
/// must be callable (else a clean error). The returned value is an
/// [`SValue::Negated`], recognized by the call dispatcher: calling it invokes
/// `f` through the normal depth-bounded path and logically negates the result.
/// So `Negate(is.na)(NA)` → `FALSE` and `Negate(\(x) x > 0)(5)` → `FALSE`.
fn b_negate(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    // The function is the sole argument — by `f =`/`FUN =` or the first callable
    // positional. There is no data, so `split_fun` returning leftover positionals
    // would be an error; instead pick the function directly.
    let f = args
        .iter()
        .find(|a| matches!(a.name.as_deref(), Some("f") | Some("FUN")))
        .map(|a| a.value.clone())
        .or_else(|| {
            args.iter()
                .find(|a| a.name.is_none() && a.value.is_callable())
                .map(|a| a.value.clone())
        })
        .ok_or_else(|| SError::BadArgs("Negate: missing function argument".into()))?;
    if !f.is_callable() {
        return Err(SError::NotCallable(f.type_name().to_string()));
    }
    Ok(SValue::Negated(Box::new(f)))
}

/// `Recall(...)` — re-invoke the **enclosing** function (anonymous recursion).
/// Reads the current function off the interpreter's call stack and calls it with
/// the supplied arguments. Outside any function it is an error. Recursion is
/// bounded by `MAX_EVAL_DEPTH` (every `Recall` goes through `call_value` →
/// `eval_node`), so runaway anonymous recursion fails cleanly rather than
/// overflowing the native stack. This makes the classic anonymous factorial
/// `(\(n) if (n <= 1) 1 else n * Recall(n - 1))(5)` evaluate to `120`.
fn b_recall(interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let current = interp
        .current_function()
        .ok_or_else(|| SError::BadArgs("Recall called from outside a closure".into()))?;
    interp.call_value(current, args)
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

// ===========================================================================
// Apply-family & grouping (R-28)
// ===========================================================================
//
// These pair the R-10 functional toolkit (`split_fun`, `interp.call_value`) with
// the R-11 matrix, the R-6 list, and the factor machinery (F4). They are pure
// builtins — no grammar change. The watchword is *bounded allocation*: `outer`
// is O(len(X)·len(Y)), so the product is guarded with `checked_mul` against
// `MAX_SEQ_LEN` *before* anything is allocated; `tabulate` caps `nbins`.

/// `outer(X, Y, FUN = "*")` — the **outer product** generalised to any binary
/// operation. The result is the `length(X) × length(Y)` matrix whose `(i, j)`
/// entry is `FUN(X[i], Y[j])`, stored **column-major** (R-11 `SValue::Matrix`,
/// element `(i, j)` at `j*nrow + i`).
///
/// ```text
///   X = c(1, 2, 3), Y = c(1, 2), FUN = "*"
///
///        Y[0]=1  Y[1]=2          column-major data:
///   X0=1   1       2               col 0 (Y=1): 1, 2, 3
///   X1=2   2       4               col 1 (Y=2): 2, 4, 6
///   X2=3   3       6             → c(1, 2, 3, 2, 4, 6), nrow=3, ncol=2
/// ```
///
/// `FUN` defaults to `"*"` and may be:
///   - the string `"*"` or `"+"` — taken on a **fast numeric path** (no per-cell
///     function call), or
///   - **any callable** (a closure, builtin, or R-9 lambda), invoked once per
///     `(i, j)` pair through `interp.call_value` — so
///     `outer(1:2, 1:2, \(a, b) a*10 + b)` works.
///
/// **Output-size cap (security).** The element count `nrow*ncol` is computed with
/// [`usize::checked_mul`] and rejected with a clean `Index` error when it
/// overflows or exceeds [`MAX_SEQ_LEN`] — *before* the result `Vec` is allocated,
/// so a crafted `outer(1:1e6, 1:1e6)` cannot OOM. A non-callable `FUN` is a clean
/// `NotCallable` error.
fn b_outer(interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let positional: Vec<&Arg> = args.iter().filter(|a| a.name.is_none()).collect();
    let x = positional
        .first()
        .map(|a| a.value.clone())
        .ok_or_else(|| SError::BadArgs("outer: missing X".into()))?;
    let y = positional
        .get(1)
        .map(|a| a.value.clone())
        .ok_or_else(|| SError::BadArgs("outer: missing Y".into()))?;
    // `FUN` is the third positional or the `FUN =` named argument; default "*".
    let fun = args
        .iter()
        .find(|a| a.name.as_deref() == Some("FUN"))
        .map(|a| a.value.clone())
        .or_else(|| positional.get(2).map(|a| a.value.clone()))
        .unwrap_or_else(|| SValue::Character(vec![Some("*".to_string())]));

    let nrow = x.length();
    let ncol = y.length();
    // Guard the product BEFORE allocating: refuse overflow or > MAX_SEQ_LEN.
    let total = nrow.checked_mul(ncol).filter(|&t| t <= MAX_SEQ_LEN).ok_or_else(|| {
        SError::Index(format!("outer: result too large (limit {MAX_SEQ_LEN} elements)"))
    })?;

    // Fast numeric paths for the two arithmetic primitives, identified by the
    // string forms "*" / "+". Anything else (including a function value) goes
    // through the general per-cell `call_value` path.
    let op: Option<fn(f64, f64) -> f64> = match fun.strip_names() {
        SValue::Character(v) if v.len() == 1 => match v[0].as_deref() {
            Some("*") => Some(|a, b| a * b),
            Some("+") => Some(|a, b| a + b),
            _ => None,
        },
        _ => None,
    };

    if let Some(op) = op {
        // Numeric fast path: read both inputs as doubles once, then fill
        // column-major. NA in either operand propagates through f64 NA.
        let xs = x.as_double()?;
        let ys = y.as_double()?;
        let mut out = vec![0.0; total];
        for j in 0..ncol {
            let yj = ys.get_value(j).unwrap_or_else(na_real);
            for i in 0..nrow {
                let xi = xs.get_value(i).unwrap_or_else(na_real);
                out[j * nrow + i] = if is_na_real(xi) || is_na_real(yj) {
                    na_real()
                } else {
                    op(xi, yj)
                };
            }
        }
        return Ok(SValue::Matrix {
            data: Double::from_values(out),
            nrow,
            ncol,
        });
    }

    // General path: `FUN` must be callable. Invoke it once per (i, j), reading a
    // single double back from each result (R simplifies length-1 atomics).
    if !fun.is_callable() {
        return Err(SError::NotCallable(fun.type_name().to_string()));
    }
    let mut out = vec![0.0; total];
    for j in 0..ncol {
        let yj = nth_element(&y, j);
        for i in 0..nrow {
            let xi = nth_element(&x, i);
            let r = interp.call_value(
                fun.clone(),
                &[
                    Arg { name: None, value: xi },
                    Arg {
                        name: None,
                        value: yj.clone(),
                    },
                ],
            )?;
            // Take the first element as a double (NA if absent/empty).
            out[j * nrow + i] = r.as_double()?.get_value(0).unwrap_or_else(na_real);
        }
    }
    Ok(SValue::Matrix {
        data: Double::from_values(out),
        nrow,
        ncol,
    })
}

/// Compute the **sorted, distinct, non-NA** group labels of an `INDEX`/`f`
/// argument, *plus* the per-element label (one `Option<String>` per element of
/// `index`, `None` where the element is `NA`). A `Factor` keeps its declared
/// `levels` order (R semantics: factor levels define the group order even if some
/// are empty); any other vector is coerced to character labels and the distinct
/// labels are sorted.
///
/// Returns `(levels, labels)` where `labels[k]` is the group of element `k`.
fn group_labels(index: &SValue) -> (Vec<String>, Vec<Option<String>>) {
    match index.strip_names() {
        SValue::Factor { codes, levels, .. } => {
            let labels: Vec<Option<String>> = codes
                .iter()
                .map(|c| c.and_then(|k| levels.get((k as usize).wrapping_sub(1)).cloned()))
                .collect();
            (levels.clone(), labels)
        }
        other => {
            let labels = other.as_character();
            let mut seen: HashSet<String> = HashSet::new();
            let mut levels: Vec<String> = labels
                .iter()
                .flatten()
                .filter(|s| seen.insert((*s).clone()))
                .cloned()
                .collect();
            levels.sort();
            (levels, labels)
        }
    }
}

/// Partition the elements of `x` into groups keyed by `labels`, one bucket per
/// entry of `levels` (in `levels` order). Each bucket holds the `nth_element`
/// values of `x` whose label matches; elements whose label is `NA` or not a level
/// are dropped (R's `split`/`tapply` semantics). The data length governs the
/// iteration, so a shorter/longer `INDEX` simply truncates/ignores extra labels —
/// never an index panic.
fn partition_by_group(x: &SValue, levels: &[String], labels: &[Option<String>]) -> Vec<SValue> {
    let mut buckets: Vec<Vec<Arg>> = vec![Vec::new(); levels.len()];
    let n = x.length();
    for k in 0..n {
        if let Some(Some(label)) = labels.get(k) {
            if let Some(g) = levels.iter().position(|lv| lv == label) {
                buckets[g].push(Arg {
                    name: None,
                    value: nth_element(x, k),
                });
            }
        }
    }
    buckets.iter().map(|b| combine(b)).collect()
}

/// `tapply(X, INDEX, FUN)` — **table apply**: split `X` into groups by `INDEX`,
/// apply `FUN` to each group, and return a **named** vector (names = the sorted
/// unique levels, in lockstep with the per-group results).
///
/// ```text
///   tapply(c(1, 2, 3, 4), c("a", "b", "a", "b"), sum)
///     group "a" = c(1, 3) → sum = 4
///     group "b" = c(2, 4) → sum = 6
///   → c(a = 4, b = 6)
/// ```
///
/// Reuses `split_fun` (to locate the callable `FUN`) and `group_labels` /
/// `partition_by_group`. A non-callable `FUN` is a clean `NotCallable` error.
fn b_tapply(interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    // The function is the callable argument (FUN =, or the callable positional);
    // the remaining positionals are X then INDEX.
    let (fun, data) = split_fun(args, "tapply")?;
    let x = data
        .first()
        .cloned()
        .ok_or_else(|| SError::BadArgs("tapply: missing X".into()))?;
    let index = data
        .get(1)
        .cloned()
        .ok_or_else(|| SError::BadArgs("tapply: missing INDEX".into()))?;

    let (levels, labels) = group_labels(&index);
    let groups = partition_by_group(&x, &levels, &labels);

    // Apply FUN to each group, taking the first element of the (length-1) result.
    let mut values: Vec<f64> = Vec::with_capacity(groups.len());
    for group in &groups {
        let r = interp.call_value(
            fun.clone(),
            &[Arg {
                name: None,
                value: group.clone(),
            }],
        )?;
        values.push(r.as_double()?.get_value(0).unwrap_or_else(na_real));
    }
    let names: Vec<Option<String>> = levels.into_iter().map(Some).collect();
    Ok(SValue::with_names(SValue::doubles(values), names))
}

/// `split(x, f)` — partition `x` by the factor (or coerced vector) `f`, returning
/// a **named list**: one element per level (in sorted-unique-level order), names
/// = the levels.
///
/// ```text
///   split(1:4, c("a", "b", "a", "b"))
///     → list(a = c(1, 3), b = c(2, 4))
/// ```
///
/// Reuses the R-6 `SValue::List` construction via `group_labels` /
/// `partition_by_group`.
fn b_split(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let positional: Vec<&Arg> = args.iter().filter(|a| a.name.is_none()).collect();
    let x = positional
        .first()
        .map(|a| a.value.clone())
        .ok_or_else(|| SError::BadArgs("split: missing x".into()))?;
    let f = args
        .iter()
        .find(|a| a.name.as_deref() == Some("f"))
        .map(|a| a.value.clone())
        .or_else(|| positional.get(1).map(|a| a.value.clone()))
        .ok_or_else(|| SError::BadArgs("split: missing f".into()))?;

    let (levels, labels) = group_labels(&f);
    let groups = partition_by_group(&x, &levels, &labels);
    let names: Vec<Option<String>> = levels.into_iter().map(Some).collect();
    Ok(SValue::List {
        names,
        items: groups,
    })
}

/// `tabulate(bin, nbins = max(bin))` — count how many times each of `1..nbins`
/// appears in the integer vector `bin`. Returns an integer (double) vector of
/// length `nbins`. Values `< 1`, `> nbins`, and `NA` are silently ignored
/// (matching R).
///
/// ```text
///   tabulate(c(1, 2, 2, 3, 3, 3))   → c(1, 2, 3)   (nbins defaults to max = 3)
///   tabulate(c(2, 3, 5), nbins = 5) → c(0, 1, 1, 0, 1)
/// ```
///
/// **Output-size cap (security).** `nbins` is **data** — a crafted `nbins = 1e18`
/// (or a `bin` containing a huge value, which feeds the default) must not allocate
/// terabytes. `nbins` is clamped to `[0, MAX_SEQ_LEN]`: a non-finite or negative
/// request becomes `0`, and an over-large one is a clean `Index` error.
fn b_tabulate(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let bin = first_positional(args)?.as_double()?;

    // Default nbins = max(bin) (0 when bin is empty or has no finite element).
    let default_nbins = bin
        .iter()
        .filter(|x| !is_na_real(*x) && x.is_finite())
        .fold(0.0f64, |acc, x| acc.max(x))
        .floor();
    let nbins_f = match args.iter().find(|a| a.name.as_deref() == Some("nbins")) {
        Some(a) => a.value.as_double()?.get_value(0).unwrap_or(default_nbins),
        None => default_nbins,
    };

    // Clamp to a sane, allocation-safe range. Non-finite / negative → 0.
    let nbins: usize = if !nbins_f.is_finite() || nbins_f < 1.0 {
        0
    } else if nbins_f > MAX_SEQ_LEN as f64 {
        return Err(SError::Index(format!(
            "tabulate: nbins too large (limit {MAX_SEQ_LEN})"
        )));
    } else {
        nbins_f.floor() as usize
    };

    let mut counts = vec![0.0f64; nbins];
    for x in bin.iter() {
        if is_na_real(x) || !x.is_finite() {
            continue;
        }
        // R floors toward the integer bin; values in [1, nbins] count.
        let b = x.floor();
        if b >= 1.0 && b <= nbins as f64 {
            counts[(b as usize) - 1] += 1.0;
        }
    }
    Ok(SValue::doubles(counts))
}

/// The second positional argument (used by `findInterval` and `cut` for their
/// `vec` / `breaks` operand), or the value of the given `name`d argument if
/// present. Returns a `BadArgs` error (never panics) when neither is supplied.
fn second_arg<'a>(args: &'a [Arg], name: &str, what: &str) -> SResult<&'a SValue> {
    args.iter()
        .find(|a| a.name.as_deref() == Some(name))
        .map(|a| &a.value)
        .or_else(|| {
            args.iter()
                .filter(|a| a.name.is_none())
                .nth(1)
                .map(|a| &a.value)
        })
        .ok_or_else(|| SError::BadArgs(format!("{what}: argument \"{name}\" is missing")))
}

/// `find_interval_index(x, vec)` — the shared kernel behind `findInterval` (and,
/// transitively, `cut`).
///
/// `vec` is a **non-decreasing** vector of breakpoints. For a single value `x`
/// we return the 1-based count of breakpoints that do **not exceed** `x` — i.e.
/// the largest `i` with `vec[i] <= x`:
///
/// ```text
///   vec = [1, 2, 3]
///
///   x        : -inf .. 1   1 .. 2   2 .. 3   3 .. +inf
///   result   :     0         1        2         3
///                  |         |        |         |
///   meaning  :  below      in       in       at/above
///              first      [1,2)    [2,3)     last break
/// ```
///
/// A right-continuous (left-closed) step: an `x` exactly equal to `vec[i]` lands
/// in the bucket that *starts* at `vec[i]`. `NA`/non-finite `x` returns `None`
/// (the caller maps it to `NA`).
///
/// `prefix` is the **leading non-NA run** of the breakpoint vector (everything up
/// to, but not including, the first `NA` element) — computed once per call by
/// [`break_prefix_len`]. Within that run the breaks are assumed sorted, so we use
/// `partition_point` (a binary search) rather than a linear scan: this turns the
/// whole `findInterval(x, vec)` / `cut` cost from `O(len(x) · len(vec))` into
/// `O(len(x) · log(len(vec)))`, which matters because both lengths can reach
/// `MAX_SEQ_LEN` (≈ 16.7M) — a quadratic scan there would be a CPU-amplification
/// hazard for untrusted programs. Trimming to the non-NA prefix preserves the
/// "first `NA` breakpoint stops the count" behaviour of the original linear form.
fn find_interval_index(x: f64, prefix: &[f64]) -> Option<usize> {
    if is_na_real(x) || !x.is_finite() {
        return None;
    }
    // The number of breaks `<= x` — equivalently the first index whose break is
    // strictly greater than `x`. `partition_point` requires the predicate to be
    // partitioned (all-true then all-false), which holds for a sorted prefix.
    Some(prefix.partition_point(|&b| b <= x))
}

/// The length of the leading non-`NA` run of a breakpoint vector. `find_interval`
/// and `cut` only ever consider breaks before the first `NA` (an `NA` break acts
/// as a hard stop, matching the original linear scan), so the binary search runs
/// over `&breaks[..break_prefix_len(breaks)]`.
fn break_prefix_len(breaks: &[f64]) -> usize {
    breaks
        .iter()
        .position(|&b| is_na_real(b))
        .unwrap_or(breaks.len())
}

/// `findInterval(x, vec)` — for each element of `x`, the index of the last
/// breakpoint in the non-decreasing `vec` that does not exceed it (see
/// [`find_interval_index`]). `0` below the first break, `length(vec)` at or above
/// the last; `NA`/non-finite `x` → `NA`.
///
/// ```text
///   findInterval(c(0.5, 1.5, 2.5), c(1, 2, 3))  ->  c(0, 1, 2)
///   findInterval(5,                c(1, 2, 3))  ->  3
/// ```
fn b_find_interval(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let x = first_positional(args)?.as_double()?;
    let vec: Vec<f64> = second_arg(args, "vec", "findInterval")?
        .as_double()?
        .iter()
        .collect();
    // The binary search runs over the leading non-NA run only (an NA break stops
    // the count). Computed once, reused for every element of `x`.
    let prefix = &vec[..break_prefix_len(&vec)];

    let out: Vec<f64> = x
        .iter()
        .map(|xi| match find_interval_index(xi, prefix) {
            Some(i) => i as f64,
            None => na_real(),
        })
        .collect();
    Ok(SValue::doubles(out))
}

/// Format a numeric breakpoint for an interval label to `dig_lab` **significant
/// digits** (R-35), the way R's `cut` formats break numbers.
///
/// * A "nice" integer break (`3.0`, `10`) prints without a decimal point
///   (`"3"`, `"10"`) regardless of `dig_lab`, keeping labels like `"(0,3]"`.
/// * A fractional break is rounded to `dig_lab` significant figures with trailing
///   zeros trimmed, so `3.14159` at `dig_lab = 2` → `"3.1"` and at the default
///   `dig_lab = 3` → `"3.14"`.
///
/// `dig_lab` is already clamped to `1..=22` by [`dig_lab_value`], so the fixed
/// precision below is bounded — no caller-controlled value can force a huge width.
fn format_break(b: f64, dig_lab: usize) -> String {
    // Non-finite (shouldn't reach here for real breaks) → plain fallback.
    if !b.is_finite() {
        return format!("{b}");
    }
    // A whole-number break keeps its integer form (no spurious ".0"), matching the
    // R-32/R-33 behaviour, as long as it is representable as an i64.
    if b.fract() == 0.0 && b.abs() < 1e15 {
        return format!("{}", b as i64);
    }
    format_sig(b, dig_lab)
}

/// Round `x` to `sig` significant digits and render it without an exponent,
/// trimming trailing zeros. `sig` is bounded (`1..=22`) by the caller.
///
/// **Bound on the format width (security).** The number of decimal places needed
/// for `sig` significant figures is `sig - 1 - floor(log10|x|)`. For a *tiny* break
/// (e.g. `1e-300`) `floor(log10|x|)` is very negative, so the naive count would be
/// several hundred — a 340-character fixed-precision string per break. The clamp to
/// `0..=22` below caps that: we never emit more than 22 fractional digits, so a
/// caller-controlled (subnormal) break can neither force a huge allocation nor a
/// long label. `sig <= 22` keeps the *high* end small too. (R itself switches to
/// scientific notation for such values; capping the digit count tracks that.)
fn format_sig(x: f64, sig: usize) -> String {
    if x == 0.0 {
        return "0".to_string();
    }
    let exp = x.abs().log10().floor() as i32;
    // Clamp the decimal count to `0..=22`: `.max(0)` alone is NOT enough, because a
    // tiny `x` makes `exp` very negative and blows the count up — `clamp` bounds it.
    let decimals = (sig as i32 - 1 - exp).clamp(0, 22) as usize;
    let s = format!("{x:.decimals$}");
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

/// Build the auto-generated interval label for the `i`-th interval (0-based) of
/// `breaks`, respecting `right`: right-closed intervals print `"(lo,hi]"`,
/// left-closed `[lo,hi)`. (R-33 — extends the R-32 right-closed-only formatter;
/// R-35 — break numbers are formatted to `dig_lab` significant digits.)
fn cut_interval_label(breaks: &[f64], i: usize, right: bool, dig_lab: usize) -> String {
    let lo = format_break(breaks[i], dig_lab);
    let hi = format_break(breaks[i + 1], dig_lab);
    if right {
        format!("({lo},{hi}]")
    } else {
        format!("[{lo},{hi})")
    }
}

/// Derive the `breaks` vector when `cut` is called with a **single number** `n`
/// (the number of equal-width bins). Mirrors R's `cut.default`: take the range of
/// the finite values of `x`, extend it by `dx/1000` on each side so the extreme
/// data points sit strictly inside the outer bins, then lay down `n + 1` equally
/// spaced breakpoints. Returns a `BadArgs`/`Index` error (never panics or
/// allocates a giant vector) when `n` is non-finite, `< 1`, or would exceed
/// `MAX_SEQ_LEN`, or when `x` has no finite values.
///
/// ```text
///   rx = (min, max) over the finite x          dx = max - min
///   if dx == 0: dx = |min|; if still 0: dx = 1   (degenerate all-equal x)
///   lo = min - dx/1000      hi = max + dx/1000
///   breaks[j] = lo + j * (hi - lo)/n   for j in 0..=n
/// ```
fn equal_width_breaks(x: &Double, n_f: f64) -> SResult<Vec<f64>> {
    // `n` must be a finite, positive whole number. R rounds the requested bin
    // count toward the nearest integer; we require it to be at least 1.
    if !n_f.is_finite() || n_f < 1.0 {
        return Err(SError::BadArgs(
            "cut: invalid number of intervals".to_string(),
        ));
    }
    // Guard the bin count BEFORE building any vector: a huge `n` would otherwise
    // allocate `n + 1` breaks and `n` level strings. `MAX_SEQ_LEN` is the same cap
    // every other length-amplifying builtin honours.
    if n_f > MAX_SEQ_LEN as f64 {
        return Err(SError::Index(format!(
            "cut: number of intervals too large (limit {MAX_SEQ_LEN})"
        )));
    }
    let n = n_f as usize;

    // The range over the finite (non-NA, non-infinite) values of `x`.
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for v in x.iter() {
        if v.is_finite() {
            lo = lo.min(v);
            hi = hi.max(v);
        }
    }
    if !lo.is_finite() || !hi.is_finite() {
        return Err(SError::BadArgs(
            "cut: 'x' has no finite values to bin".to_string(),
        ));
    }

    // Extend the range by 0.1% on each side. A degenerate (all-equal) range has
    // `dx == 0`; R falls back to `abs(min)`, then to `1`, so the bins stay finite
    // and we never divide by zero when computing the step.
    let mut dx = hi - lo;
    if dx == 0.0 {
        dx = lo.abs();
        if dx == 0.0 {
            dx = 1.0;
        }
    }
    let pad = dx / 1000.0;
    let lo = lo - pad;
    let hi = hi + pad;

    // `n + 1` equally spaced breakpoints. `n >= 1` so the step denominator is
    // non-zero and finite. Extreme-magnitude `x` can still overflow the extended
    // range to `±inf` (e.g. `dx = hi - lo` overflowing), which would make the
    // breaks `NaN`/`inf`; reject that up front so every emitted break is finite
    // (no garbage `"(NaN,NaN]"` levels, and the downstream scan only ever sees
    // sorted finite breaks).
    let step = (hi - lo) / n as f64;
    if !lo.is_finite() || !hi.is_finite() || !step.is_finite() {
        return Err(SError::BadArgs(
            "cut: range of 'x' is too large to bin".to_string(),
        ));
    }
    let mut breaks = Vec::with_capacity(n + 1);
    for j in 0..=n {
        breaks.push(lo + j as f64 * step);
    }
    Ok(breaks)
}

/// `cut(x, breaks)` — bin the numeric vector `x` into the intervals delimited by
/// the **sorted** breakpoint vector `breaks`, returning a **factor**.
///
/// With `k = length(breaks)` breakpoints there are `k - 1` intervals. The default
/// intervals are **right-closed** `(lo, hi]`, and the auto-generated level labels
/// are exactly `"(lo,hi]"`:
///
/// ```text
///   breaks = [0, 3, 6, 11]            levels = ["(0,3]", "(3,6]", "(6,11]"]
///
///   x = 1   -> findInterval = 1 -> code 1 -> "(0,3]"
///   x = 5   -> findInterval = 2 -> code 2 -> "(3,6]"
///   x = 10  -> findInterval = 3 -> code 3 -> "(6,11]"
///   x = -1  -> findInterval = 0 -> out of range -> NA
///   x = 20  -> findInterval = 4 (= k) -> out of range -> NA
/// ```
///
/// The whole job reduces to `findInterval`: the interval index `i` is the 1-based
/// factor code precisely when `1 <= i <= k-1`; the boundary indices `0` (below the
/// first break) and `k` (at/above the last) — and any `NA` `x` — map to a `NA`
/// code.
///
/// **R-33 options** (all layered onto the same scan, none changing the default):
///
/// - **`right = FALSE`** — left-closed intervals `[lo, hi)`. The default
///   right-closed `(lo, hi]` lookup counts breaks strictly `< x` (so `x == break`
///   lands in the bin it *closes*); the left-closed lookup counts breaks `<= x`
///   (so `x == break` lands in the bin it *opens*). See [`cut_code`].
/// - **`include.lowest = TRUE`** — fold the extreme break into the adjacent
///   interval. Right-closed: an `x` equal to `breaks[0]` (which would otherwise be
///   "below the first interval") is pulled into interval 1. Left-closed: an `x`
///   equal to `breaks[k-1]` (otherwise "at/above the last") is pulled into the
///   last interval.
/// - **`labels = FALSE`** — return the **integer bin codes** as a plain numeric
///   vector (not a factor). `labels = <character>` — use those strings as the
///   factor levels (length must equal the number of intervals).
/// - **integer `breaks`** — a single number `N` requests `N` equal-width bins over
///   the (slightly extended) range of `x` (see [`equal_width_breaks`]).
fn b_cut(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let x = first_positional(args)?.as_double()?;

    // `right` (default TRUE) and `include.lowest` (default FALSE) are read as
    // logical flags; a malformed value is a clean error via `truthy`.
    let right = named_flag(args, "right", true)?;
    let include_lowest = named_flag(args, "include.lowest", false)?;

    // `breaks` may be the usual breakpoint vector OR a single number `N` (number
    // of equal-width bins). R treats `length(breaks) == 1` as the bin-count form.
    let breaks_val = second_arg(args, "breaks", "cut")?;
    let breaks_double = breaks_val.as_double()?;
    let breaks: Vec<f64> = if breaks_double.len() == 1 {
        // Single number → derive equal-width breakpoints over the range of `x`.
        // (Honours the MAX_SEQ_LEN cap and degenerate-range fallback internally.)
        let n = breaks_double.get_value(0).unwrap_or(na_real());
        equal_width_breaks(&x, n)?
    } else {
        breaks_double.iter().collect()
    };

    // `k - 1` intervals; with fewer than two breaks there are none, so every
    // value is unbinned (NA). `saturating_sub` keeps this from underflowing.
    let n_intervals = breaks.len().saturating_sub(1);

    // Each value's 1-based interval code (or `None` for out-of-range / NA).
    let prefix = &breaks[..break_prefix_len(&breaks)];
    let codes: Vec<Option<u32>> = x
        .iter()
        .map(|xi| cut_code(xi, prefix, &breaks, n_intervals, right, include_lowest))
        .collect();

    // `labels = FALSE` short-circuits to the bare integer codes — no factor.
    if let Some(arg) = args.iter().find(|a| a.name.as_deref() == Some("labels")) {
        if let SValue::Logical(v) = strip_wrappers(&arg.value) {
            if matches!(v.first(), Some(Some(false))) {
                let out: Vec<f64> = codes
                    .iter()
                    .map(|c| c.map(|k| k as f64).unwrap_or_else(na_real))
                    .collect();
                return Ok(SValue::doubles(out));
            }
        }
    }

    // R-35: `dig.lab` (default 3) controls the number of significant digits used
    // when formatting break numbers in the auto-generated labels. It is clamped to
    // a safe range inside `dig_lab_value` so an extreme value cannot drive a huge
    // allocation or a formatter panic.
    let dig_lab = dig_lab_value(args)?;

    // Otherwise build the factor levels: custom `labels` (validated length) or the
    // auto-generated interval strings (respecting `right` and `dig.lab`).
    let levels = cut_levels(args, &breaks, n_intervals, right, dig_lab)?;

    // R-35: `ordered_result = TRUE` makes the binned factor an *ordered* factor —
    // its intervals are naturally ordered low→high, so the bins compare by order.
    let ordered = named_flag(args, "ordered_result", false)?;

    Ok(SValue::Factor {
        codes,
        levels,
        ordered,
    })
}

/// R-35 — read `cut`'s `dig.lab` argument: the number of **significant digits**
/// used when auto-formatting break numbers in interval labels. Defaults to **3**
/// (base R's default). The value is **clamped to `1..=22`** (R's representable-
/// digit ceiling) so a caller-supplied extreme (e.g. `dig.lab = 1e9`) can never
/// drive an unbounded format width or a panic; a non-finite or non-positive value
/// falls back to the default rather than erroring, matching R's lenient handling.
fn dig_lab_value(args: &[Arg]) -> SResult<usize> {
    const DEFAULT: usize = 3;
    const MAX: usize = 22;
    match args.iter().find(|a| a.name.as_deref() == Some("dig.lab")) {
        None => Ok(DEFAULT),
        Some(arg) => {
            let d = arg.value.as_double()?;
            match d.get_value(0) {
                Some(x) if x.is_finite() && x >= 1.0 => Ok((x.trunc() as usize).clamp(1, MAX)),
                // NA / non-finite / < 1 → fall back to the default (no panic).
                _ => Ok(DEFAULT),
            }
        }
    }
}

/// The 1-based factor code for a single value under `cut`'s interval rules, or
/// `None` (→ `<NA>`) when the value falls in no interval. Centralises the
/// `right` / `include.lowest` logic shared by the factor and `labels=FALSE`
/// paths.
fn cut_code(
    xi: f64,
    prefix: &[f64],
    breaks: &[f64],
    n_intervals: usize,
    right: bool,
    include_lowest: bool,
) -> Option<u32> {
    if n_intervals == 0 || is_na_real(xi) || !xi.is_finite() {
        return None;
    }
    // The 1-based interval index is a count of breakpoints below `xi`, with the
    // comparison decided by which end is closed:
    //
    //   right = TRUE   (lo, hi]  →  interval i contains `breaks[i-1] < x <= breaks[i]`
    //                              →  index = #{breaks strictly < x}
    //   right = FALSE  [lo, hi)  →  interval i contains `breaks[i-1] <= x < breaks[i]`
    //                              →  index = #{breaks <= x}
    //
    // (Both are `partition_point` binary searches over the sorted non-NA prefix.)
    // For an `x` exactly on an *interior* break the two rules disagree by one,
    // which is precisely the `(lo,hi]` vs `[lo,hi)` boundary convention.
    let idx = if right {
        prefix.partition_point(|&b| b < xi)
    } else {
        prefix.partition_point(|&b| b <= xi)
    };

    if idx >= 1 && idx <= n_intervals {
        return Some(idx as u32);
    }

    // `include.lowest`: fold the single extreme boundary value into the adjacent
    // interval (the only point that the strict end-convention leaves unbinned).
    if include_lowest {
        if right {
            // Right-closed: `x == breaks[0]` gives index 0 (below the first
            // interval) — pull it into interval 1, making the first bin `[lo,hi]`.
            if xi == breaks[0] {
                return Some(1);
            }
        } else {
            // Left-closed: `x == breaks[k]` gives index k = n_intervals + 1 (at/above
            // the last) — pull it into the last interval, making it `[lo,hi]`.
            if xi == breaks[n_intervals] {
                return Some(n_intervals as u32);
            }
        }
    }
    None
}

/// Build the factor levels for `cut`: a custom `labels = <character>` vector
/// (whose length must equal `n_intervals`) when supplied, otherwise the
/// auto-generated `"(lo,hi]"` / `"[lo,hi)"` interval strings.
fn cut_levels(
    args: &[Arg],
    breaks: &[f64],
    n_intervals: usize,
    right: bool,
    dig_lab: usize,
) -> SResult<Vec<String>> {
    if let Some(arg) = args.iter().find(|a| a.name.as_deref() == Some("labels")) {
        let stripped = strip_wrappers(&arg.value);
        // `labels = TRUE` (or absent) means "use the auto labels"; only a non-
        // logical value is taken as a custom label vector. `labels = FALSE` is
        // handled by the caller before we ever get here.
        let is_logical_flag = matches!(stripped, SValue::Logical(_));
        if !is_logical_flag && !matches!(stripped, SValue::Null) {
            let labels: Vec<String> = arg
                .value
                .as_character()
                .into_iter()
                .map(|o| o.unwrap_or_else(|| "NA".to_string()))
                .collect();
            if labels.len() != n_intervals {
                return Err(SError::BadArgs(
                    "lengths of 'breaks' and 'labels' differ".to_string(),
                ));
            }
            return Ok(labels);
        }
    }
    Ok((0..n_intervals)
        .map(|i| cut_interval_label(breaks, i, right, dig_lab))
        .collect())
}

/// Read a named logical flag (`right`, `include.lowest`) with a default. A
/// malformed value surfaces as a clean error via `truthy` rather than a panic.
fn named_flag(args: &[Arg], name: &str, default: bool) -> SResult<bool> {
    match args.iter().find(|a| a.name.as_deref() == Some(name)) {
        Some(arg) => arg.value.truthy(),
        None => Ok(default),
    }
}

/// Borrow the value of a named argument (the first match), or `None` if absent.
/// Used by callers that need to inspect the value themselves (e.g. read an
/// integer `k =` with their own range check) rather than coerce to a flag.
fn named_arg<'a>(args: &'a [Arg], name: &str) -> Option<&'a SValue> {
    args.iter()
        .find(|a| a.name.as_deref() == Some(name))
        .map(|a| &a.value)
}

/// Peel `Classed` / `Named` / `Attributed` wrappers off a value so we can inspect
/// its underlying variant (used to tell `labels = FALSE`/`TRUE` from a character
/// label vector).
fn strip_wrappers(v: &SValue) -> &SValue {
    match v {
        SValue::Classed { inner, .. }
        | SValue::Named { values: inner, .. }
        | SValue::Attributed { inner, .. } => strip_wrappers(inner),
        other => other,
    }
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

/// Concatenate every positional argument into a single message string, the way
/// `stop`/`warning` build their text (coerce to character, drop names, join with
/// no separator — R's `.makeMessage` behaviour). NA elements render as the
/// literal `"NA"`.
fn concat_message(args: &[Arg]) -> String {
    args.iter()
        .filter(|a| a.name.is_none())
        .flat_map(|a| a.value.as_character())
        .map(|o| o.unwrap_or_else(|| "NA".to_string()))
        .collect::<Vec<_>>()
        .join("")
}

/// `stop(...)` — raise an error whose message is the concatenation of the
/// arguments. Surfaces as [`SError::User`], which `tryCatch(error = ...)` can
/// catch. A bare `stop()` (no message) raises an empty-message error, as in R.
fn b_stop(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    Err(SError::User(concat_message(args)))
}

/// `warning(...)` — emit a warning (concatenated message) without aborting, and
/// return invisibly. The message is recorded in the session warning buffer and
/// printed immediately. The evaluator marks builtins invisible by name, but a
/// `warning()` value is `NULL` anyway.
fn b_warning(interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    interp.warn(&concat_message(args));
    Ok(SValue::Null)
}

/// `conditionMessage(e)` — the `message` element of a condition object (the
/// character string a `tryCatch` handler was handed). Equivalent to `e$message`.
/// On a value with no `message` element this yields `NULL` (R would error, but
/// the lenient list `$` access is harmless and keeps us panic-free).
fn b_condition_message(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    let cond = first_positional(args)?;
    crate::dataframe::column_by_name(cond, "message")
}

/// `seq(to)` is `1:to`; `seq(from, to)` is `from:to` (step 1). A minimal subset
/// of R's `seq` sufficient for v1, plus the R-45 `seq.Date` dispatch: when the
/// first positional argument carries class "Date", we delegate to [`seq_date`],
/// which understands a numeric / `"day"` / `"week"` / `"month"` / `"year"` `by =`
/// and a `length.out =` alternative to `to`.
fn b_seq(_interp: &Interpreter, args: &[Arg]) -> SResult<SValue> {
    // R-45: Date dispatch. `seq(as.Date(...), ...)` is S3 `seq.Date`. We branch on
    // the first positional's class before the numeric fast-path so plain numeric
    // `seq` is entirely unaffected.
    if let Some(first) = args.iter().find(|a| a.name.is_none()) {
        if is_date(&first.value) {
            return seq_date(args);
        }
    }

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

/// A `seq.Date` `by =` step, parsed from either a number (of days) or a unit
/// string. `Days(n)` covers numeric `by`, `"day"` (n=1×mult), and `"week"`
/// (n=7×mult); `Months(n)` and `Years(n)` step the civil Y/M/D with day clamping.
enum DateStep {
    Days(i64),
    Months(i64),
    Years(i64),
}

/// Parse a `seq.Date` `by =` argument into a [`DateStep`]. A numeric `by` is a
/// (possibly negative) whole number of days. A string `by` is `"day"`, `"week"`,
/// `"month"`, or `"year"` with an optional leading **integer multiplier**
/// (`"2 weeks"`, `"3 months"`) — anything else is rejected. The multiplier is
/// parsed with a bounded `i64` and the resulting day step for day/week units is
/// later checked against the sequence-length cap, so no `by =` can drive an
/// unbounded allocation.
fn parse_date_by(args: &[Arg]) -> SResult<DateStep> {
    let by = args
        .iter()
        .find(|a| a.name.as_deref() == Some("by"))
        .map(|a| &a.value);
    let Some(by) = by else {
        // Default step is one day (R's default for Date `from:to`).
        return Ok(DateStep::Days(1));
    };
    // Numeric `by` → that many days. (`by = 7` ≡ `by = "week"`.)
    if let SValue::Double(d) = peel_structural(by) {
        let v = d.get_value(0).unwrap_or(f64::NAN);
        if !v.is_finite() {
            return Err(SError::BadArgs("seq.Date: 'by' must be finite".into()));
        }
        return Ok(DateStep::Days(v.trunc() as i64));
    }
    // String `by` → "[mult ]unit". Split on the first space; the optional left
    // part is an integer multiplier, the right (or whole) part is the unit.
    let s = by
        .as_character()
        .into_iter()
        .next()
        .flatten()
        .ok_or_else(|| SError::BadArgs("seq.Date: invalid 'by'".into()))?;
    let s = s.trim();
    let (mult, unit) = match s.split_once(char::is_whitespace) {
        Some((n, u)) => {
            let m: i64 = n
                .trim()
                .parse()
                .map_err(|_| SError::BadArgs(format!("seq.Date: invalid 'by' = {s:?}")))?;
            (m, u.trim())
        }
        None => (1, s),
    };
    // Accept both singular and plural unit spellings ("week"/"weeks").
    let unit = unit.strip_suffix('s').unwrap_or(unit);
    match unit {
        "day" => Ok(DateStep::Days(mult)),
        "week" => {
            let n = mult
                .checked_mul(7)
                .ok_or_else(|| SError::BadArgs("seq.Date: 'by' overflow".into()))?;
            Ok(DateStep::Days(n))
        }
        "month" => Ok(DateStep::Months(mult)),
        "year" => Ok(DateStep::Years(mult)),
        other => Err(SError::BadArgs(format!("seq.Date: invalid 'by' unit {other:?}"))),
    }
}

/// The widest absolute civil-month index `add_months_clamped` will ever feed to
/// the kernel. A representable Date is at most `MAX_DATE_DAYS` ≈ 1e11 days from the
/// epoch (~270 million years); a month is at least 28 days, so any month index
/// beyond `MAX_DATE_DAYS / 28` (plus a small slack) provably lands outside the
/// Date range. Clamping `total` to this bound keeps `days_from_civil`'s internal
/// `era * 146097` multiplication comfortably inside `i64` — so an absurd `by =
/// "9e18 months"` can never overflow/panic the kernel; the clamped (still
/// out-of-range) day count is then rejected by the caller's `MAX_DATE_DAYS`
/// `push` guard, exactly as a directly out-of-range numeric Date would be.
const MAX_DATE_MONTHS: i64 = MAX_DATE_DAYS / 28 + 12;

/// Add `n` civil months to `(y, m)` (keeping a separate day), clamping the
/// day-of-month to the target month's length. `n` may be negative. Pure i64
/// arithmetic with `rem_euclid`/`div_euclid` so negative totals never panic, and
/// the absolute month index is clamped to [`MAX_DATE_MONTHS`] so the kernel call
/// below can never overflow even for an absurd `n`.
fn add_months_clamped(y: i64, m: i64, d: i64, n: i64) -> (i64, i64, i64) {
    // Convert to a 0-based absolute month index, shift, decompose back. Saturating
    // arithmetic prevents an overflow *here*, and the explicit clamp keeps the
    // index small enough that `days_from_civil` (called via `days_in_month`)
    // cannot overflow either. A clamped, out-of-range result is re-validated
    // against MAX_DATE_DAYS by the caller's `push`, which turns it into an error.
    let total = y
        .saturating_mul(12)
        .saturating_add(m - 1)
        .saturating_add(n)
        .clamp(-MAX_DATE_MONTHS, MAX_DATE_MONTHS);
    let ny = total.div_euclid(12);
    let nm = total.rem_euclid(12) + 1; // 1..=12
                                       // Clamp the day to the new month's length.
    let last = days_in_month(ny, nm);
    (ny, nm, d.min(last))
}

/// The number of days in civil month `m` of year `y` (Gregorian, leap-aware).
/// Computed from the kernel itself — the day before the 1st of the *next* month —
/// so the leap-year rule lives in exactly one place (`days_from_civil`).
fn days_in_month(y: i64, m: i64) -> i64 {
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    days_from_civil(ny, nm, 1) - days_from_civil(y, m, 1)
}

/// `seq.Date(from, to = , by = , length.out = )` — generate a `Date` sequence
/// (R-45). `from` is the first (positional) Date; `to` is the second positional
/// **or** the named `to =`. The step comes from [`parse_date_by`]. `length.out =`
/// is an alternative to `to` (and wins if both are given). The output length is
/// bounded by [`MAX_SEQ_LEN`] with checked arithmetic *before* any allocation, so
/// a span/step implying billions of dates errors rather than exhausting memory.
fn seq_date(args: &[Arg]) -> SResult<SValue> {
    // `from` — the first positional Date, taken as a single day count. We route it
    // through `checked_date_days` (the same ±MAX_DATE_DAYS guard `as.Date` uses) so
    // a hand-built out-of-range Date (e.g. `structure(1e300, class="Date")`) is
    // rejected up front and the span/step arithmetic below can never overflow i64.
    let from = first_positional(args)?
        .as_double()?
        .get_value(0)
        .and_then(checked_date_days)
        .ok_or_else(|| SError::BadArgs("seq.Date: 'from' must be a finite, in-range Date".into()))?;

    let step = parse_date_by(args)?;

    // `length.out =` (alias `length_out`) takes priority over `to`.
    let length_out = named_count(args, "length.out").or_else(|| named_count(args, "length_out"));

    // `to` is the second positional argument or the named `to =`. Same in-range
    // guard as `from`, so a crafted out-of-range `to` cannot overflow `to - from`.
    let to: Option<i64> = nth_positional(args, 1)
        .or_else(|| {
            args.iter()
                .find(|a| a.name.as_deref() == Some("to"))
                .map(|a| &a.value)
        })
        .and_then(|v| v.as_double().ok())
        .and_then(|d| d.get_value(0))
        .and_then(checked_date_days);

    if length_out.is_none() && to.is_none() {
        return Err(SError::BadArgs(
            "seq.Date: need either 'to' or 'length.out'".into(),
        ));
    }

    // Build the day-count vector, capping length at MAX_SEQ_LEN throughout.
    let mut days: Vec<f64> = Vec::new();
    let push = |days: &mut Vec<f64>, z: i64| -> SResult<()> {
        if days.len() >= MAX_SEQ_LEN {
            return Err(SError::BadArgs(format!(
                "seq.Date: result too large (limit {MAX_SEQ_LEN} elements)"
            )));
        }
        if z.abs() > MAX_DATE_DAYS {
            return Err(SError::BadArgs(
                "seq.Date: generated date out of range".into(),
            ));
        }
        days.push(z as f64);
        Ok(())
    };

    match (length_out, to, &step) {
        // length.out given: emit exactly that many dates from `from`. Each date is
        // computed directly as `from + k·step` (k = 0..n), so there is no carried
        // mutable state to get wrong and month/year clamping always references the
        // *original* anchor day (R's behaviour: Jan 31, Feb 28, Mar 31 — not Feb
        // 28, Feb 28, …).
        (Some(n), _, _) => {
            if n > MAX_SEQ_LEN {
                return Err(SError::BadArgs(format!(
                    "seq.Date: length.out {n} exceeds the limit of {MAX_SEQ_LEN}"
                )));
            }
            let (oy, om, od) = civil_from_days(from);
            for k in 0..n as i64 {
                let z = match &step {
                    DateStep::Days(s) => {
                        let off = s
                            .checked_mul(k)
                            .ok_or_else(|| SError::BadArgs("seq.Date: step overflow".into()))?;
                        from.checked_add(off)
                            .ok_or_else(|| SError::BadArgs("seq.Date: step overflow".into()))?
                    }
                    DateStep::Months(s) => {
                        let months = s
                            .checked_mul(k)
                            .ok_or_else(|| SError::BadArgs("seq.Date: step overflow".into()))?;
                        let (ny, nm, nd) = add_months_clamped(oy, om, od, months);
                        days_from_civil(ny, nm, nd)
                    }
                    DateStep::Years(s) => {
                        let months = s
                            .checked_mul(12)
                            .and_then(|sm| sm.checked_mul(k))
                            .ok_or_else(|| SError::BadArgs("seq.Date: step overflow".into()))?;
                        let (ny, nm, nd) = add_months_clamped(oy, om, od, months);
                        days_from_civil(ny, nm, nd)
                    }
                };
                push(&mut days, z)?;
            }
        }
        // `to` given (no length.out): step until we pass `to`.
        (None, Some(to), _) => match &step {
            DateStep::Days(s) => {
                if *s == 0 {
                    return Err(SError::BadArgs("seq.Date: 'by' must be non-zero".into()));
                }
                // Pre-compute the length and cap it BEFORE allocating, so a huge
                // span can never OOM. n = floor((to - from) / s) + 1 when the sign
                // of (to - from) matches s; else the sequence is empty.
                let span = to - from;
                if (span >= 0) == (*s > 0) || span == 0 {
                    let steps = (span / s).unsigned_abs();
                    let n = steps
                        .checked_add(1)
                        .filter(|&t| t <= MAX_SEQ_LEN as u64)
                        .ok_or_else(|| {
                            SError::BadArgs(format!(
                                "seq.Date: result too large (limit {MAX_SEQ_LEN} elements)"
                            ))
                        })?;
                    for k in 0..n as i64 {
                        // Checked: `from + s·k`. By construction k·s stays within
                        // the (bounded) span, but compute it defensively so even a
                        // crafted `by` cannot overflow — it errors instead.
                        let off = s
                            .checked_mul(k)
                            .ok_or_else(|| SError::BadArgs("seq.Date: step overflow".into()))?;
                        let z = from
                            .checked_add(off)
                            .ok_or_else(|| SError::BadArgs("seq.Date: step overflow".into()))?;
                        push(&mut days, z)?;
                    }
                }
                // else: step points away from `to` → empty sequence (R returns
                // `from` only when to==from; the equality case is handled above).
            }
            DateStep::Months(s) | DateStep::Years(s) => {
                let step_months = if matches!(step, DateStep::Years(_)) {
                    s.checked_mul(12)
                        .ok_or_else(|| SError::BadArgs("seq.Date: 'by' overflow".into()))?
                } else {
                    *s
                };
                if step_months == 0 {
                    return Err(SError::BadArgs("seq.Date: 'by' must be non-zero".into()));
                }
                let (oy, om, od) = civil_from_days(from);
                let ascending = step_months > 0;
                let mut k: i64 = 0;
                loop {
                    // Saturating: a runaway k cannot overflow here, and the
                    // resulting out-of-range day is rejected by `push` below.
                    let (ny, nm, nd) =
                        add_months_clamped(oy, om, od, step_months.saturating_mul(k));
                    let z = days_from_civil(ny, nm, nd);
                    // Stop once we pass `to` in the step's direction.
                    if (ascending && z > to) || (!ascending && z < to) {
                        break;
                    }
                    push(&mut days, z)?;
                    k = k
                        .checked_add(1)
                        .ok_or_else(|| SError::BadArgs("seq.Date: step overflow".into()))?;
                }
            }
        },
        (None, None, _) => unreachable!("guarded above"),
    }

    Ok(make_date(days))
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

// ===========================================================================
// R-44 — unit tests for the civil-date kernel and the Date parse/format helpers
// ===========================================================================
//
// The end-to-end Date builtins (`as.Date`, `format`, `weekdays`, `difftime`,
// `Sys.Date`) are exercised through R/S syntax in the `lib.rs` test module and in
// `r-runtime`. Here we test the *pure* kernel and parse/format helpers directly,
// since they are private to this module — especially the round-trip invariant
// `civil_from_days(days_from_civil(y,m,d)) == (y,m,d)` over leap and pre-epoch
// dates, and the parse-safety guards (malformed → None, never a panic).
#[cfg(test)]
mod date_tests {
    use super::*;

    /// Known fixed points: the epoch and its neighbours, anchoring the convention
    /// that day 0 is 1970-01-01 and day -1 is the day before.
    #[test]
    fn epoch_anchor_points() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(1970, 1, 2), 1);
        assert_eq!(days_from_civil(1969, 12, 31), -1);
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(1), (1970, 1, 2));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
    }

    /// The exact-inverse invariant across leap days, century boundaries, and
    /// deep pre-epoch / post-epoch dates — the property that makes the kernel
    /// trustworthy.
    #[test]
    fn civil_days_round_trip() {
        let cases = [
            (1970, 1, 1),
            (1969, 12, 31),
            (2000, 2, 29), // leap day (divisible by 400)
            (2020, 2, 29), // leap day (divisible by 4, not 100)
            (1900, 3, 1),  // 1900 is NOT a leap year (divisible by 100, not 400)
            (2021, 3, 14),
            (1, 1, 1),
            (1899, 12, 31),
            (2400, 12, 31),
            (-1, 12, 31), // proleptic, year before year 0
        ];
        for (y, m, d) in cases {
            let z = days_from_civil(y, m, d);
            assert_eq!(
                civil_from_days(z),
                (y, m, d),
                "round-trip failed for {y}-{m}-{d} (z={z})"
            );
        }
    }

    /// 1900 was not a leap year (the divisible-by-100-but-not-400 rule), so
    /// 1900-02-28 + 1 day is March 1, not February 29.
    #[test]
    fn non_leap_century() {
        let feb28 = days_from_civil(1900, 2, 28);
        assert_eq!(civil_from_days(feb28 + 1), (1900, 3, 1));
    }

    #[test]
    fn parse_iso_default() {
        assert_eq!(parse_date_str("1970-01-01", "%Y-%m-%d"), Some(0));
        assert_eq!(parse_date_str("1970-01-02", "%Y-%m-%d"), Some(1));
        assert_eq!(parse_date_str("1969-12-31", "%Y-%m-%d"), Some(-1));
        assert_eq!(parse_date_str("2021-03-14", "%Y-%m-%d"), Some(18700));
    }

    #[test]
    fn parse_slash_format() {
        assert_eq!(
            parse_date_str("2021/03/14", "%Y/%m/%d"),
            parse_date_str("2021-03-14", "%Y-%m-%d")
        );
    }

    /// Malformed, out-of-range, and adversarial inputs must all yield `None`
    /// (→ NA at the builtin level) and never panic or overflow.
    #[test]
    fn parse_rejects_garbage() {
        assert_eq!(parse_date_str("not-a-date", "%Y-%m-%d"), None);
        assert_eq!(parse_date_str("2021-13-01", "%Y-%m-%d"), None); // month 13
        assert_eq!(parse_date_str("2021-02-30", "%Y-%m-%d"), None); // impossible day
        assert_eq!(parse_date_str("2021-00-10", "%Y-%m-%d"), None); // month 0
        assert_eq!(parse_date_str("2021-03-14 ", "%Y-%m-%d"), None); // trailing space
        assert_eq!(parse_date_str("2021-03", "%Y-%m-%d"), None); // missing day
        assert_eq!(parse_date_str("", "%Y-%m-%d"), None); // empty
                                                          // A million digits cannot overflow i64 — the digit cap refuses it.
        let huge = "9".repeat(1_000_000);
        assert_eq!(parse_date_str(&format!("{huge}-01-01"), "%Y-%m-%d"), None);
    }

    #[test]
    fn format_iso_and_fields() {
        assert_eq!(format_date_days(0, "%Y-%m-%d"), "1970-01-01");
        assert_eq!(format_date_days(18700, "%Y-%m-%d"), "2021-03-14");
        // Leap-day round-trip through the formatter.
        let leap = days_from_civil(2000, 2, 29);
        assert_eq!(format_date_days(leap, "%Y-%m-%d"), "2000-02-29");
        // %j day-of-year: Jan 1 is 001; Mar 14 2021 (non-leap) is day 73.
        assert_eq!(format_date_days(days_from_civil(2021, 1, 1), "%j"), "001");
        assert_eq!(format_date_days(18700, "%j"), "073");
    }

    /// Negative (pre-epoch) day counts must format without panicking.
    #[test]
    fn format_pre_epoch() {
        assert_eq!(format_date_days(-1, "%Y-%m-%d"), "1969-12-31");
    }

    // -----------------------------------------------------------------------
    // R-46 — POSIXct: the seconds↔(days, h, m, s) split, parse, and render.
    // -----------------------------------------------------------------------

    /// The intraday split is `div_euclid`/`rem_euclid` by 86400, so it stays
    /// correct (and never negatively-indexes) on pre-epoch (negative) seconds.
    #[test]
    fn posixct_intraday_split_handles_pre_epoch() {
        // +1 second past the epoch: 0 days, 1 intraday second.
        assert_eq!(1i64.div_euclid(86_400), 0);
        assert_eq!(1i64.rem_euclid(86_400), 1);
        // -1 second (1969-12-31 23:59:59): -1 day, 86399 intraday seconds.
        assert_eq!((-1i64).div_euclid(86_400), -1);
        assert_eq!((-1i64).rem_euclid(86_400), 86_399);
    }

    /// `parse_posixct_str` reads "YYYY-MM-DD HH:MM:SS" to seconds since epoch.
    #[test]
    fn parse_posixct_full_datetime() {
        assert_eq!(parse_posixct_str("1970-01-01 00:00:00"), Some(0));
        assert_eq!(parse_posixct_str("1970-01-01 00:01:00"), Some(60));
        assert_eq!(parse_posixct_str("1970-01-02 00:00:00"), Some(86_400));
        // 2021-03-14 09:30:05.
        let z = days_from_civil(2021, 3, 14);
        assert_eq!(
            parse_posixct_str("2021-03-14 09:30:05"),
            Some(z * 86_400 + 9 * 3600 + 30 * 60 + 5)
        );
    }

    /// A bare date with no time half is taken as midnight (days * 86400).
    #[test]
    fn parse_posixct_date_only_is_midnight() {
        let z = days_from_civil(2021, 3, 14);
        assert_eq!(parse_posixct_str("2021-03-14"), Some(z * 86_400));
    }

    /// Malformed input and out-of-range H/M/S are rejected (None → NA), never a
    /// panic. The leap-second slot (S = 60) is accepted.
    #[test]
    fn parse_posixct_malformed_and_ranges() {
        assert_eq!(parse_posixct_str("garbage"), None);
        assert_eq!(parse_posixct_str("2021-03-14 25:00:00"), None); // hour > 23
        assert_eq!(parse_posixct_str("2021-03-14 09:60:00"), None); // minute > 59
        assert_eq!(parse_posixct_str("2021-03-14 09:30:61"), None); // second > 60
        assert!(parse_posixct_str("2021-03-14 09:30:60").is_some()); // leap second OK
        assert_eq!(parse_posixct_str("2021-13-01 00:00:00"), None); // bad month
    }

    /// `format_posixct_seconds` renders the default and a custom format, reusing
    /// the R-45 date fields on the date half.
    #[test]
    fn format_posixct_default_and_fields() {
        let secs = parse_posixct_str("2021-03-14 09:30:05").unwrap();
        assert_eq!(
            format_posixct_seconds(secs, "%Y-%m-%d %H:%M:%S"),
            "2021-03-14 09:30:05"
        );
        assert_eq!(format_posixct_seconds(secs, "%H:%M"), "09:30");
        // Reused %B from the date half.
        let jan = parse_posixct_str("2021-01-15 06:07:08").unwrap();
        assert_eq!(format_posixct_seconds(jan, "%B"), "January");
    }

    /// Pre-epoch seconds render without panic and pick the correct clock time.
    #[test]
    fn format_posixct_pre_epoch() {
        // -1 second = 1969-12-31 23:59:59.
        assert_eq!(
            format_posixct_seconds(-1, "%Y-%m-%d %H:%M:%S"),
            "1969-12-31 23:59:59"
        );
    }

    /// The seconds bound rejects absurd magnitudes before the civil kernel sees
    /// them (→ NA), the numeric counterpart to the digit cap.
    #[test]
    fn posixct_seconds_bound() {
        assert!(checked_posixct_seconds(0.0).is_some());
        assert!(checked_posixct_seconds(MAX_POSIXCT_SECONDS as f64).is_some());
        assert!(checked_posixct_seconds(1e300).is_none());
        assert!(checked_posixct_seconds(f64::NAN).is_none());
        assert!(checked_posixct_seconds(f64::INFINITY).is_none());
    }
}
