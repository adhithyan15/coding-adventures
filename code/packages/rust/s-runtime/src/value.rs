//! The S value model — everything is a vector.
//!
//! S has no scalar type: what other languages call "a number" is a numeric
//! vector of length one. [`SValue`] captures the v1 value universe — numeric
//! (`double`), `logical`, and `character` vectors, the empty `NULL`, and
//! function values (user closures and built-ins). This module also owns the
//! cross-cutting vector mechanics that make S *feel* like S: **recycling**,
//! **NA propagation**, the **coercion lattice**, indexing, and the `[i]`-style
//! printing convention.

use crate::env::Env;
use crate::error::{SError, SResult};
use parser::grammar_parser::GrammarASTNode;
use r_vector::{is_na_real, na_real, Double};
use std::rc::Rc;

/// A built-in function: it receives the interpreter (so it can call back into
/// user functions — e.g. `sapply`, or S3 method dispatch) and the already-
/// evaluated arguments (v1 is eager — there are no lazy promises yet).
pub type Builtin = fn(&crate::eval::Interpreter, &[Arg]) -> SResult<SValue>;

/// A formal parameter of a user-defined function, with an optional default
/// expression (stored unevaluated, as in S).
#[derive(Clone)]
pub struct Param {
    pub name: String,
    pub default: Option<Rc<GrammarASTNode>>,
}

/// An actual argument at a call site: a value, optionally tagged with the name
/// it was passed under (`mean(x, na.rm = TRUE)` → `na.rm`).
#[derive(Clone)]
pub struct Arg {
    pub name: Option<String>,
    pub value: SValue,
}

/// A first-class S value.
///
/// Numeric vectors use `r_vector::Double` (the NA-aware double vector the
/// statistics substrate consumes directly). Logical and character vectors keep
/// their own `Vec<Option<_>>` payloads, where `None` is the typed `NA`.
#[derive(Clone)]
pub enum SValue {
    /// A numeric (`double`) vector. `NA` is encoded with the R/S NA bit pattern.
    Double(Double),
    /// A logical vector; `None` elements are `NA`.
    Logical(Vec<Option<bool>>),
    /// A character (string) vector; `None` elements are `NA`.
    Character(Vec<Option<String>>),
    /// The empty value, distinct from `NA`. `length(NULL)` is 0.
    Null,
    /// A user-defined function: parameters, body, and the environment it closed
    /// over (S has lexical scoping).
    Closure {
        params: Vec<Param>,
        body: Rc<GrammarASTNode>,
        env: Env,
    },
    /// A built-in function (`c`, `mean`, …).
    Builtin { name: String, func: Builtin },

    /// A factor: integer `codes` (1-based into `levels`, `None` = `NA`) plus the
    /// ordered `levels`. Implicit class `"factor"`.
    Factor {
        codes: Vec<Option<u32>>,
        levels: Vec<String>,
    },

    /// A data frame: equal-length `columns` with their `names`. Implicit class
    /// `"data.frame"`.
    DataFrame {
        names: Vec<String>,
        columns: Vec<SValue>,
    },

    /// A value carrying an explicit S3 `class` attribute. Transparent to most
    /// operations (they see through to `inner`); only `class()` and method
    /// dispatch observe the class.
    Classed {
        inner: Box<SValue>,
        class: Vec<String>,
    },
}

/// The S3 class vector of a value: the explicit class if one was set, otherwise
/// the implicit class derived from the value's type.
pub fn class_of(value: &SValue) -> Vec<String> {
    match value {
        SValue::Classed { class, .. } => class.clone(),
        SValue::Factor { .. } => vec!["factor".to_string()],
        SValue::DataFrame { .. } => vec!["data.frame".to_string()],
        SValue::Double(_) => vec!["numeric".to_string()],
        SValue::Logical(_) => vec!["logical".to_string()],
        SValue::Character(_) => vec!["character".to_string()],
        SValue::Null => vec!["NULL".to_string()],
        SValue::Closure { .. } | SValue::Builtin { .. } => vec!["function".to_string()],
    }
}

impl std::fmt::Debug for SValue {
    /// A compact debug view. Function values are summarized rather than printed
    /// in full, so we never recurse into a closure's captured environment.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SValue::Double(d) => write!(f, "Double({:?})", d.data()),
            SValue::Logical(v) => write!(f, "Logical({v:?})"),
            SValue::Character(v) => write!(f, "Character({v:?})"),
            SValue::Null => write!(f, "Null"),
            SValue::Closure { params, .. } => {
                write!(f, "Closure(/{} params/)", params.len())
            }
            SValue::Builtin { name, .. } => write!(f, "Builtin({name})"),
            SValue::Factor { codes, levels } => {
                write!(f, "Factor({} codes, {} levels)", codes.len(), levels.len())
            }
            SValue::DataFrame { names, columns } => {
                write!(f, "DataFrame({} cols: {:?})", columns.len(), names)
            }
            SValue::Classed { inner, class } => write!(f, "Classed({class:?}, {inner:?})"),
        }
    }
}

/// Position of a type in the coercion lattice `logical < double < character`.
fn type_rank(value: &SValue) -> u8 {
    match value {
        SValue::Logical(_) => 0,
        SValue::Double(_) => 1,
        SValue::Character(_) => 2,
        _ => 3, // NULL / functions — not part of the atomic lattice
    }
}

impl SValue {
    /// A numeric vector built from raw `f64`s.
    pub fn doubles(values: Vec<f64>) -> SValue {
        SValue::Double(Double::from_values(values))
    }

    /// A length-1 numeric vector.
    pub fn scalar(value: f64) -> SValue {
        SValue::Double(Double::singleton(value))
    }

    /// The human-facing S type name (used in error messages).
    pub fn type_name(&self) -> &'static str {
        match self {
            SValue::Double(_) => "double",
            SValue::Logical(_) => "logical",
            SValue::Character(_) => "character",
            SValue::Null => "NULL",
            SValue::Closure { .. } | SValue::Builtin { .. } => "closure",
            SValue::Factor { .. } => "factor",
            SValue::DataFrame { .. } => "data.frame",
            SValue::Classed { inner, .. } => inner.type_name(),
        }
    }

    /// `length(x)` — the element count. A factor's length is its code count; a
    /// data frame's length is its column count (matching R's `length(df)`).
    pub fn length(&self) -> usize {
        match self {
            SValue::Double(d) => d.len(),
            SValue::Logical(v) => v.len(),
            SValue::Character(v) => v.len(),
            SValue::Null => 0,
            SValue::Closure { .. } | SValue::Builtin { .. } => 1,
            SValue::Factor { codes, .. } => codes.len(),
            SValue::DataFrame { columns, .. } => columns.len(),
            SValue::Classed { inner, .. } => inner.length(),
        }
    }

    pub fn is_callable(&self) -> bool {
        matches!(self, SValue::Closure { .. } | SValue::Builtin { .. })
    }

    /// Coerce to a numeric `Double` for arithmetic and statistics. Logical
    /// becomes 0/1 (NA preserved); `NULL` is the empty numeric vector.
    pub fn as_double(&self) -> SResult<Double> {
        match self {
            SValue::Double(d) => Ok(d.clone()),
            SValue::Logical(v) => Ok(Double::from_values(
                v.iter()
                    .map(|o| match o {
                        Some(true) => 1.0,
                        Some(false) => 0.0,
                        None => na_real(),
                    })
                    .collect(),
            )),
            SValue::Null => Ok(Double::from_values(vec![])),
            SValue::Classed { inner, .. } => inner.as_double(),
            other => Err(SError::TypeError(format!(
                "non-numeric argument (got {})",
                other.type_name()
            ))),
        }
    }

    /// Coerce to a logical vector (`None` = `NA`). A numeric coerces by `x != 0`;
    /// a logical is taken as-is; `NULL` is empty. Used by `any`/`all`/`which`.
    pub fn as_logical(&self) -> SResult<Vec<Option<bool>>> {
        match self {
            SValue::Logical(v) => Ok(v.clone()),
            SValue::Double(d) => Ok(d
                .iter()
                .map(|x| if is_na_real(x) { None } else { Some(x != 0.0) })
                .collect()),
            SValue::Null => Ok(vec![]),
            SValue::Classed { inner, .. } => inner.as_logical(),
            other => Err(SError::TypeError(format!(
                "argument is not logical (got {})",
                other.type_name()
            ))),
        }
    }

    /// The character labels of a factor (`None` = `NA`), used by `as.character`
    /// and when a factor is combined into a character vector.
    pub fn factor_labels(codes: &[Option<u32>], levels: &[String]) -> Vec<Option<String>> {
        codes
            .iter()
            .map(|c| c.and_then(|k| levels.get((k as usize).wrapping_sub(1)).cloned()))
            .collect()
    }

    /// Coerce to a character vector (for `c()` mixing strings with other types).
    pub fn as_character(&self) -> Vec<Option<String>> {
        match self {
            SValue::Character(v) => v.clone(),
            SValue::Double(d) => d
                .iter()
                .map(|x| {
                    if is_na_real(x) {
                        None
                    } else {
                        Some(format_number(x))
                    }
                })
                .collect(),
            SValue::Logical(v) => v
                .iter()
                .map(|o| o.map(|b| if b { "TRUE".into() } else { "FALSE".into() }))
                .collect(),
            SValue::Null => vec![],
            SValue::Factor { codes, levels } => SValue::factor_labels(codes, levels),
            SValue::Classed { inner, .. } => inner.as_character(),
            other => vec![Some(other.type_name().to_string())],
        }
    }

    /// The single truth value used by `if` / `while`. Mirrors S: an empty value
    /// or an `NA` test is an error; a numeric uses `x != 0`.
    pub fn truthy(&self) -> SResult<bool> {
        match self {
            SValue::Logical(v) => match v.first() {
                Some(Some(b)) => Ok(*b),
                Some(None) => Err(SError::Missing(
                    "missing value where TRUE/FALSE needed".into(),
                )),
                None => Err(SError::Missing("argument is of length zero".into())),
            },
            SValue::Double(d) => match d.get_value(0) {
                Some(x) if is_na_real(x) => Err(SError::Missing(
                    "missing value where TRUE/FALSE needed".into(),
                )),
                Some(x) => Ok(x != 0.0),
                None => Err(SError::Missing("argument is of length zero".into())),
            },
            SValue::Classed { inner, .. } => inner.truthy(),
            other => Err(SError::TypeError(format!(
                "argument is not interpretable as logical (got {})",
                other.type_name()
            ))),
        }
    }
}

// ===========================================================================
// c() — the combine primitive, with the coercion lattice
// ===========================================================================

/// Combine values into a single vector, coercing to the highest type present.
/// `NULL` contributes nothing (`c(1, NULL, 2)` is `c(1, 2)`).
pub fn combine(args: &[Arg]) -> SValue {
    let present: Vec<&SValue> = args
        .iter()
        .map(|a| &a.value)
        .filter(|v| !matches!(v, SValue::Null))
        .collect();

    if present.is_empty() {
        return SValue::Null;
    }

    let rank = present.iter().map(|v| type_rank(v)).max().unwrap_or(0);

    match rank {
        2 => {
            // character wins: stringify everything.
            let mut out = Vec::new();
            for v in present {
                out.extend(v.as_character());
            }
            SValue::Character(out)
        }
        0 => {
            // all logical: concatenate logical payloads.
            let mut out = Vec::new();
            for v in present {
                if let SValue::Logical(l) = v {
                    out.extend(l.iter().cloned());
                }
            }
            SValue::Logical(out)
        }
        _ => {
            // double (with any logicals coerced to 0/1).
            let mut out = Vec::new();
            for v in present {
                // as_double cannot fail here: only logical/double remain.
                if let Ok(d) = v.as_double() {
                    out.extend(d.iter());
                }
            }
            SValue::Double(Double::from_values(out))
        }
    }
}

// ===========================================================================
// Bounded sequence construction (the `:` operator and seq())
// ===========================================================================

/// The largest number of elements a `:` sequence or `seq()` may materialize.
/// This caps memory use against crafted input — without it, a one-liner like
/// `1:1e18` would try to allocate an exabyte-scale vector and abort the
/// process. ~16.7M elements is far beyond any realistic interactive use.
pub const MAX_SEQ_LEN: usize = 1 << 24;

/// Build the inclusive numeric sequence from `from` to `to` stepping by ±1,
/// refusing non-finite bounds and any span that would exceed [`MAX_SEQ_LEN`].
/// Shared by the `:` operator and the `seq()` built-in so the bound cannot
/// drift between them.
pub fn bounded_sequence(from: f64, to: f64) -> SResult<Vec<f64>> {
    if !from.is_finite() || !to.is_finite() {
        return Err(SError::BadArgs("sequence bounds must be finite".into()));
    }
    let span = (to - from).abs();
    if span >= MAX_SEQ_LEN as f64 {
        return Err(SError::BadArgs(format!(
            "sequence of length {} exceeds the limit of {}",
            span.floor() as u64 + 1,
            MAX_SEQ_LEN
        )));
    }
    let n = span.floor() as usize + 1;
    let step = if to >= from { 1.0 } else { -1.0 };
    Ok((0..n).map(|k| from + step * k as f64).collect())
}

// ===========================================================================
// Element-wise arithmetic and comparison (recycling + NA propagation)
// ===========================================================================

/// Recycle two numeric slices element-wise under `f`, propagating `NA`. An
/// empty operand yields an empty result (S semantics).
fn recycle_double(a: &[f64], b: &[f64], f: impl Fn(f64, f64) -> f64) -> Vec<f64> {
    if a.is_empty() || b.is_empty() {
        return vec![];
    }
    let n = a.len().max(b.len());
    (0..n)
        .map(|i| {
            let x = a[i % a.len()];
            let y = b[i % b.len()];
            if is_na_real(x) || is_na_real(y) {
                na_real()
            } else {
                f(x, y)
            }
        })
        .collect()
}

/// Apply a binary arithmetic operator (`+ - * / ^`) to two values.
pub fn arithmetic(op: &str, lhs: &SValue, rhs: &SValue) -> SResult<SValue> {
    let a = lhs.as_double()?;
    let b = rhs.as_double()?;
    let f: fn(f64, f64) -> f64 = match op {
        "+" => |x, y| x + y,
        "-" => |x, y| x - y,
        "*" => |x, y| x * y,
        "/" => |x, y| x / y,
        "^" => |x, y| x.powf(y),
        // `%%` is R's modulo (result takes the divisor's sign); `%/%` is floor
        // division. Both reuse the same recycling/NA machinery.
        "%%" => |x, y| x - (x / y).floor() * y,
        "%/%" => |x, y| (x / y).floor(),
        other => {
            return Err(SError::TypeError(format!("unknown operator '{other}'")));
        }
    };
    Ok(SValue::Double(Double::from_values(recycle_double(
        a.data(),
        b.data(),
        f,
    ))))
}

/// Negate a numeric value (unary minus).
pub fn negate(value: &SValue) -> SResult<SValue> {
    let d = value.as_double()?;
    Ok(SValue::Double(Double::from_values(
        d.iter()
            .map(|x| if is_na_real(x) { na_real() } else { -x })
            .collect(),
    )))
}

/// `x %in% table` — for each element of `x`, whether it appears in `table`.
/// Returns a logical vector the length of `x`. Membership is tested on the
/// coerced character form, so it works uniformly for numeric, logical, and
/// character values. Unlike comparison, `%in%` never yields `NA` (an `NA` in
/// `x` is `TRUE` iff `table` also contains `NA`, matching R).
pub fn membership(lhs: &SValue, rhs: &SValue) -> SValue {
    let haystack: std::collections::HashSet<Option<String>> =
        rhs.as_character().into_iter().collect();
    SValue::Logical(
        lhs.as_character()
            .into_iter()
            .map(|n| Some(haystack.contains(&n)))
            .collect(),
    )
}

/// Apply a comparison operator, producing a logical vector. Numeric operands
/// compare numerically; if either side is character, both are compared as
/// strings (S's coercion for relational operators).
pub fn compare(op: &str, lhs: &SValue, rhs: &SValue) -> SResult<SValue> {
    let either_char = matches!(lhs, SValue::Character(_)) || matches!(rhs, SValue::Character(_));

    if either_char {
        let a = lhs.as_character();
        let b = rhs.as_character();
        if a.is_empty() || b.is_empty() {
            return Ok(SValue::Logical(vec![]));
        }
        let n = a.len().max(b.len());
        let out = (0..n)
            .map(|i| {
                let x = &a[i % a.len()];
                let y = &b[i % b.len()];
                match (x, y) {
                    (Some(x), Some(y)) => Some(compare_ord(op, x.cmp(y))),
                    _ => None,
                }
            })
            .collect();
        return Ok(SValue::Logical(out));
    }

    let a = lhs.as_double()?;
    let b = rhs.as_double()?;
    if a.is_empty() || b.is_empty() {
        return Ok(SValue::Logical(vec![]));
    }
    let n = a.len().max(b.len());
    let out = (0..n)
        .map(|i| {
            let x = a.data()[i % a.len()];
            let y = b.data()[i % b.len()];
            if is_na_real(x) || is_na_real(y) {
                None
            } else {
                Some(compare_f64(op, x, y))
            }
        })
        .collect();
    Ok(SValue::Logical(out))
}

fn compare_f64(op: &str, x: f64, y: f64) -> bool {
    match op {
        "==" => x == y,
        "!=" => x != y,
        "<" => x < y,
        ">" => x > y,
        "<=" => x <= y,
        ">=" => x >= y,
        _ => false,
    }
}

fn compare_ord(op: &str, ord: std::cmp::Ordering) -> bool {
    use std::cmp::Ordering::*;
    match op {
        "==" => ord == Equal,
        "!=" => ord != Equal,
        "<" => ord == Less,
        ">" => ord == Greater,
        "<=" => ord != Greater,
        ">=" => ord != Less,
        _ => false,
    }
}

// ===========================================================================
// Indexing — x[i] with a positive-integer index vector (v1)
// ===========================================================================

/// Index `base` with the numeric index vector `idx` (1-based). Index `0` is
/// dropped (as in S); an out-of-range or `NA` index yields `NA`. Negative and
/// logical indices are not supported in v1.
pub fn index(base: &SValue, idx: &SValue) -> SResult<SValue> {
    let positions = idx.as_double()?;
    let len = base.length();

    // Resolve each requested 1-based position into either Some(0-based) or None
    // (meaning NA / drop). 0 is dropped entirely.
    let mut picks: Vec<Option<usize>> = Vec::new();
    for p in positions.iter() {
        if is_na_real(p) {
            picks.push(None);
            continue;
        }
        if p < 0.0 {
            return Err(SError::Index(
                "negative subscripts are not supported in v1".into(),
            ));
        }
        let one_based = p as usize; // truncates toward zero, like S
        if one_based == 0 {
            continue; // 0 selects nothing
        }
        if one_based > len {
            picks.push(None); // out of range → NA
        } else {
            picks.push(Some(one_based - 1));
        }
    }

    Ok(match base {
        SValue::Double(d) => SValue::Double(Double::from_values(
            picks
                .iter()
                .map(|p| p.and_then(|i| d.get_value(i)).unwrap_or_else(na_real))
                .collect(),
        )),
        SValue::Logical(v) => SValue::Logical(picks.iter().map(|p| p.and_then(|i| v[i])).collect()),
        SValue::Character(v) => {
            SValue::Character(picks.iter().map(|p| p.and_then(|i| v[i].clone())).collect())
        }
        SValue::Null => SValue::Null,
        SValue::Factor { codes, levels } => SValue::Factor {
            codes: picks.iter().map(|p| p.and_then(|i| codes[i])).collect(),
            levels: levels.clone(),
        },
        SValue::Classed { inner, .. } => index(inner, idx)?,
        other => {
            return Err(SError::Index(format!(
                "object of type '{}' is not subsettable",
                other.type_name()
            )))
        }
    })
}

// ===========================================================================
// Formatting — the printed representation, with the [i] index prefix
// ===========================================================================

/// Format one `f64` the way S prints it: integer-valued doubles drop the
/// decimal point (`2`, not `2.0`); specials are `NA`/`Inf`/`-Inf`/`NaN`.
pub fn format_number(x: f64) -> String {
    if is_na_real(x) {
        "NA".to_string()
    } else if x.is_nan() {
        "NaN".to_string()
    } else if x.is_infinite() {
        if x > 0.0 {
            "Inf".into()
        } else {
            "-Inf".into()
        }
    } else if x.fract() == 0.0 && x.abs() < 1e15 {
        format!("{}", x as i64)
    } else {
        // ~7 significant digits, trailing zeros trimmed (close to S's default).
        let s = format!("{:.6}", x);
        let trimmed = s.trim_end_matches('0').trim_end_matches('.');
        trimmed.to_string()
    }
}

/// Render a value as the lines S would print, including the `[i]` index prefix
/// that precedes each line of a vector. `NULL` prints as `NULL`; an empty
/// atomic vector prints its type tag (`character(0)`, `numeric(0)`, …).
pub fn format_value(value: &SValue) -> Vec<String> {
    let elems: Vec<String> = match value {
        SValue::Null => return vec!["NULL".to_string()],
        SValue::Double(d) => {
            if d.is_empty() {
                return vec!["numeric(0)".to_string()];
            }
            d.iter().map(format_number).collect()
        }
        SValue::Logical(v) => {
            if v.is_empty() {
                return vec!["logical(0)".to_string()];
            }
            v.iter()
                .map(|o| match o {
                    Some(true) => "TRUE".to_string(),
                    Some(false) => "FALSE".to_string(),
                    None => "NA".to_string(),
                })
                .collect()
        }
        SValue::Character(v) => {
            if v.is_empty() {
                return vec!["character(0)".to_string()];
            }
            v.iter()
                .map(|o| match o {
                    Some(s) => format!("\"{s}\""),
                    None => "NA".to_string(),
                })
                .collect()
        }
        SValue::Closure { params, .. } => {
            let names: Vec<&str> = params.iter().map(|p| p.name.as_str()).collect();
            return vec![format!("function ({})", names.join(", "))];
        }
        SValue::Builtin { name, .. } => {
            return vec![format!("function ({}) .Primitive", name)];
        }
        SValue::Factor { codes, levels } => {
            if codes.is_empty() {
                return vec![
                    "factor(0)".to_string(),
                    format!("Levels: {}", levels.join(" ")),
                ];
            }
            let labels: Vec<String> = factor_labels(codes, levels)
                .into_iter()
                .map(|o| o.unwrap_or_else(|| "<NA>".to_string()))
                .collect();
            let mut lines = format_vector(&labels);
            lines.push(format!("Levels: {}", levels.join(" ")));
            return lines;
        }
        SValue::DataFrame { names, columns } => return format_data_frame(names, columns),
        SValue::Classed { inner, .. } => return format_value(inner),
    };

    format_vector(&elems)
}

/// Free-function form of [`SValue::factor_labels`] for use inside formatting.
fn factor_labels(codes: &[Option<u32>], levels: &[String]) -> Vec<Option<String>> {
    SValue::factor_labels(codes, levels)
}

/// The unquoted printed form of element `i` of a value (used by data-frame
/// table rendering). Out-of-range or unsupported cells render as `NA`.
pub fn element_string(value: &SValue, i: usize) -> String {
    match value {
        SValue::Double(d) => d
            .get_value(i)
            .map(format_number)
            .unwrap_or_else(|| "NA".into()),
        SValue::Logical(v) => match v.get(i) {
            Some(Some(true)) => "TRUE".into(),
            Some(Some(false)) => "FALSE".into(),
            _ => "NA".into(),
        },
        SValue::Character(v) => v
            .get(i)
            .and_then(|o| o.clone())
            .unwrap_or_else(|| "NA".into()),
        SValue::Factor { codes, levels } => codes
            .get(i)
            .and_then(|c| *c)
            .and_then(|k| levels.get((k as usize).wrapping_sub(1)).cloned())
            .unwrap_or_else(|| "NA".into()),
        SValue::Classed { inner, .. } => element_string(inner, i),
        _ => "NA".into(),
    }
}

/// Render a data frame as a simple left-aligned table with a leading row-number
/// column, the way R's `print.data.frame` does (without the fancier formatting).
fn format_data_frame(names: &[String], columns: &[SValue]) -> Vec<String> {
    let nrow = columns.first().map(|c| c.length()).unwrap_or(0);
    if columns.is_empty() {
        return vec!["data frame with 0 columns and 0 rows".to_string()];
    }

    // Build the grid of cell strings: header row + one row per observation. The
    // first column is the 1-based row number.
    let mut header: Vec<String> = vec![String::new()];
    header.extend(names.iter().cloned());
    let mut rows: Vec<Vec<String>> = vec![header];
    for r in 0..nrow {
        let mut row = vec![(r + 1).to_string()];
        for col in columns {
            row.push(element_string(col, r));
        }
        rows.push(row);
    }

    // Column widths, then left-align each cell.
    let ncol = columns.len() + 1;
    let widths: Vec<usize> = (0..ncol)
        .map(|c| rows.iter().map(|row| row[c].len()).max().unwrap_or(0))
        .collect();
    rows.iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .map(|(c, cell)| format!("{cell:<width$}", width = widths[c]))
                .collect::<Vec<_>>()
                .join(" ")
                .trim_end()
                .to_string()
        })
        .collect()
}

/// Lay out element strings into `[i]`-prefixed lines, wrapping near 80 columns,
/// right-aligning every element to a common width — exactly S's console style.
fn format_vector(elems: &[String]) -> Vec<String> {
    let width = elems.iter().map(|s| s.len()).max().unwrap_or(1);
    let n = elems.len();
    let index_width = format!("[{n}]").len();

    // How many elements fit per line after the index prefix.
    let avail = 80usize.saturating_sub(index_width + 1).max(width + 1);
    let per_line = (avail / (width + 1)).max(1);

    let mut lines = Vec::new();
    let mut i = 0;
    while i < n {
        let prefix = format!("[{}]", i + 1);
        let prefix = format!("{prefix:>index_width$}");
        let mut line = prefix;
        for e in elems.iter().skip(i).take(per_line) {
            line.push(' ');
            line.push_str(&format!("{e:>width$}"));
        }
        lines.push(line);
        i += per_line;
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arg(value: SValue) -> Arg {
        Arg { name: None, value }
    }

    fn dbl(v: &SValue) -> Vec<f64> {
        match v {
            SValue::Double(d) => d.data().to_vec(),
            other => panic!("not a double: {:?}", other),
        }
    }

    // --- scalars, lengths, type names -----------------------------------

    #[test]
    fn scalars_and_lengths() {
        assert_eq!(SValue::scalar(3.0).length(), 1);
        assert_eq!(SValue::doubles(vec![1.0, 2.0]).type_name(), "double");
        assert_eq!(SValue::Logical(vec![Some(true)]).type_name(), "logical");
        assert_eq!(
            SValue::Character(vec![Some("a".into())]).type_name(),
            "character"
        );
        assert_eq!(SValue::Null.type_name(), "NULL");
        assert_eq!(SValue::Null.length(), 0);
        assert!(!SValue::scalar(1.0).is_callable());
    }

    // --- coercion -------------------------------------------------------

    #[test]
    fn logical_coerces_to_double() {
        let d = SValue::Logical(vec![Some(true), Some(false), None])
            .as_double()
            .unwrap();
        assert_eq!(d.get_value(0), Some(1.0));
        assert_eq!(d.get_value(1), Some(0.0));
        assert!(is_na_real(d.get_value(2).unwrap()));
    }

    #[test]
    fn null_coerces_to_empty_and_character_is_non_numeric() {
        assert_eq!(SValue::Null.as_double().unwrap().len(), 0);
        assert!(SValue::Character(vec![Some("a".into())])
            .as_double()
            .is_err());
    }

    #[test]
    fn as_character_renders_each_type() {
        assert_eq!(
            SValue::doubles(vec![1.0, 2.5]).as_character(),
            vec![Some("1".to_string()), Some("2.5".to_string())]
        );
        assert_eq!(
            SValue::Logical(vec![Some(true), None]).as_character(),
            vec![Some("TRUE".to_string()), None]
        );
        assert!(SValue::Null.as_character().is_empty());
    }

    // --- truthiness -----------------------------------------------------

    #[test]
    fn truthiness_rules() {
        assert!(SValue::Logical(vec![Some(true)]).truthy().unwrap());
        assert!(!SValue::scalar(0.0).truthy().unwrap());
        assert!(SValue::scalar(2.0).truthy().unwrap());
        assert!(SValue::Logical(vec![None]).truthy().is_err());
        assert!(SValue::doubles(vec![]).truthy().is_err());
        assert!(SValue::Character(vec![Some("x".into())]).truthy().is_err());
    }

    // --- combine and the coercion lattice -------------------------------

    #[test]
    fn combine_drops_null_and_picks_highest_type() {
        // logical + double -> double
        let v = combine(&[
            arg(SValue::Logical(vec![Some(true)])),
            arg(SValue::scalar(2.0)),
        ]);
        assert_eq!(dbl(&v), vec![1.0, 2.0]);
        // anything + character -> character
        let v = combine(&[
            arg(SValue::scalar(1.0)),
            arg(SValue::Character(vec![Some("a".into())])),
        ]);
        assert!(matches!(v, SValue::Character(_)));
        // all-logical stays logical
        let v = combine(&[
            arg(SValue::Logical(vec![Some(true)])),
            arg(SValue::Logical(vec![None])),
        ]);
        assert!(matches!(v, SValue::Logical(_)));
        // NULL contributes nothing; all-null -> NULL
        assert!(matches!(combine(&[arg(SValue::Null)]), SValue::Null));
        let v = combine(&[
            arg(SValue::scalar(1.0)),
            arg(SValue::Null),
            arg(SValue::scalar(2.0)),
        ]);
        assert_eq!(dbl(&v), vec![1.0, 2.0]);
    }

    // --- arithmetic, recycling, NA --------------------------------------

    #[test]
    fn arithmetic_operators_and_recycling() {
        let a = SValue::doubles(vec![1.0, 2.0, 3.0, 4.0]);
        let b = SValue::doubles(vec![10.0, 20.0]);
        assert_eq!(
            dbl(&arithmetic("+", &a, &b).unwrap()),
            vec![11.0, 22.0, 13.0, 24.0]
        );
        assert_eq!(
            dbl(&arithmetic("-", &SValue::scalar(5.0), &SValue::scalar(2.0)).unwrap()),
            vec![3.0]
        );
        assert_eq!(
            dbl(&arithmetic("*", &SValue::scalar(3.0), &SValue::scalar(4.0)).unwrap()),
            vec![12.0]
        );
        assert_eq!(
            dbl(&arithmetic("/", &SValue::scalar(8.0), &SValue::scalar(2.0)).unwrap()),
            vec![4.0]
        );
        assert_eq!(
            dbl(&arithmetic("^", &SValue::scalar(2.0), &SValue::scalar(10.0)).unwrap()),
            vec![1024.0]
        );
        assert!(arithmetic("?", &a, &b).is_err());
    }

    #[test]
    fn empty_operand_yields_empty_and_na_propagates() {
        let empty = arithmetic("+", &SValue::doubles(vec![]), &SValue::scalar(1.0)).unwrap();
        assert_eq!(dbl(&empty).len(), 0);
        let na = SValue::Logical(vec![None]); // NA
        let r = arithmetic("+", &na, &SValue::scalar(1.0)).unwrap();
        assert!(is_na_real(dbl(&r)[0]));
    }

    #[test]
    fn negate_handles_na() {
        let r = negate(&SValue::doubles(vec![1.0, -2.0])).unwrap();
        assert_eq!(dbl(&r), vec![-1.0, 2.0]);
    }

    // --- comparison -----------------------------------------------------

    #[test]
    fn numeric_and_character_comparison() {
        let r = compare(
            ">",
            &SValue::doubles(vec![1.0, 2.0, 3.0]),
            &SValue::scalar(2.0),
        )
        .unwrap();
        assert!(
            matches!(&r, SValue::Logical(v) if *v == vec![Some(false), Some(false), Some(true)])
        );
        let r = compare(
            "==",
            &SValue::Character(vec![Some("a".into())]),
            &SValue::Character(vec![Some("a".into())]),
        )
        .unwrap();
        assert!(matches!(&r, SValue::Logical(v) if v[0] == Some(true)));
        let r = compare(
            "<",
            &SValue::Character(vec![Some("a".into())]),
            &SValue::Character(vec![Some("b".into())]),
        )
        .unwrap();
        assert!(matches!(&r, SValue::Logical(v) if v[0] == Some(true)));
        // empty operand -> empty logical
        let r = compare("<", &SValue::doubles(vec![]), &SValue::scalar(1.0)).unwrap();
        assert!(matches!(&r, SValue::Logical(v) if v.is_empty()));
        // NA in comparison -> NA
        let r = compare("==", &SValue::Logical(vec![None]), &SValue::scalar(1.0)).unwrap();
        assert!(matches!(&r, SValue::Logical(v) if v[0].is_none()));
    }

    // --- indexing -------------------------------------------------------

    #[test]
    fn indexing_variants() {
        let base = SValue::doubles(vec![10.0, 20.0, 30.0]);
        assert_eq!(
            dbl(&index(&base, &SValue::scalar(2.0)).unwrap()),
            vec![20.0]
        );
        // 0 selects nothing; out-of-range -> NA
        assert_eq!(
            dbl(&index(&base, &SValue::doubles(vec![0.0, 1.0])).unwrap()),
            vec![10.0]
        );
        let oob = index(&base, &SValue::scalar(9.0)).unwrap();
        assert!(is_na_real(dbl(&oob)[0]));
        // negative subscript is rejected in v1
        assert!(index(&base, &SValue::scalar(-1.0)).is_err());
        // logical and character vectors are subsettable too
        let lg = index(
            &SValue::Logical(vec![Some(true), Some(false)]),
            &SValue::scalar(2.0),
        )
        .unwrap();
        assert!(matches!(&lg, SValue::Logical(v) if v[0] == Some(false)));
        let ch = index(
            &SValue::Character(vec![Some("a".into()), Some("b".into())]),
            &SValue::scalar(1.0),
        )
        .unwrap();
        assert!(matches!(&ch, SValue::Character(v) if v[0].as_deref() == Some("a")));
        // a function is not subsettable
        assert!(index(
            &SValue::Builtin {
                name: "c".into(),
                func: |_, _| Ok(SValue::Null)
            },
            &SValue::scalar(1.0)
        )
        .is_err());
    }

    // --- number and value formatting ------------------------------------

    #[test]
    fn format_number_specials() {
        assert_eq!(format_number(2.0), "2");
        assert_eq!(format_number(2.5), "2.5");
        assert_eq!(format_number(na_real()), "NA");
        assert_eq!(format_number(f64::NAN), "NaN");
        assert_eq!(format_number(f64::INFINITY), "Inf");
        assert_eq!(format_number(f64::NEG_INFINITY), "-Inf");
    }

    #[test]
    fn format_value_atomic_and_empty() {
        assert_eq!(format_value(&SValue::Null), vec!["NULL"]);
        assert_eq!(format_value(&SValue::doubles(vec![])), vec!["numeric(0)"]);
        assert_eq!(format_value(&SValue::Logical(vec![])), vec!["logical(0)"]);
        assert_eq!(
            format_value(&SValue::Character(vec![])),
            vec!["character(0)"]
        );
        assert_eq!(
            format_value(&SValue::Logical(vec![Some(true), None, Some(false)])),
            vec!["[1]  TRUE    NA FALSE"]
        );
        assert_eq!(
            format_value(&SValue::Character(vec![Some("hi".into()), None])),
            vec!["[1] \"hi\"   NA"]
        );
    }

    #[test]
    fn format_value_wraps_long_vectors() {
        let v = SValue::doubles((1..=40).map(|n| n as f64).collect());
        let lines = format_value(&v);
        assert!(lines.len() > 1, "long vector should wrap across lines");
        // Index labels are right-aligned to a common width (as in R), so the
        // first line's label may carry leading padding.
        assert!(lines[0].trim_start().starts_with("[1]"));
        assert!(lines[1].trim_start().starts_with('['));
    }

    #[test]
    fn format_value_for_callables() {
        let b = SValue::Builtin {
            name: "c".into(),
            func: |_, _| Ok(SValue::Null),
        };
        assert_eq!(format_value(&b), vec!["function (c) .Primitive"]);
    }

    // --- bounded sequence ------------------------------------------------

    #[test]
    fn bounded_sequence_ok_and_limits() {
        assert_eq!(
            bounded_sequence(1.0, 5.0).unwrap(),
            vec![1.0, 2.0, 3.0, 4.0, 5.0]
        );
        assert_eq!(bounded_sequence(3.0, 1.0).unwrap(), vec![3.0, 2.0, 1.0]);
        assert!(bounded_sequence(1.0, f64::INFINITY).is_err());
        assert!(bounded_sequence(1.0, 1e18).is_err());
    }

    // --- v2: membership, logical coercion, class, new formatting --------

    #[test]
    fn membership_returns_logical() {
        let r = membership(
            &SValue::doubles(vec![1.0, 5.0]),
            &SValue::doubles(vec![1.0, 2.0, 3.0]),
        );
        assert!(matches!(&r, SValue::Logical(v) if *v == vec![Some(true), Some(false)]));
    }

    #[test]
    fn as_logical_coercion() {
        assert_eq!(SValue::scalar(0.0).as_logical().unwrap(), vec![Some(false)]);
        assert_eq!(SValue::scalar(2.0).as_logical().unwrap(), vec![Some(true)]);
        assert!(SValue::Character(vec![Some("x".into())])
            .as_logical()
            .is_err());
    }

    #[test]
    fn class_of_implicit_and_explicit() {
        assert_eq!(class_of(&SValue::scalar(1.0)), vec!["numeric"]);
        assert_eq!(class_of(&SValue::Character(vec![])), vec!["character"]);
        assert_eq!(class_of(&SValue::Null), vec!["NULL"]);
        let f = SValue::Factor {
            codes: vec![],
            levels: vec![],
        };
        assert_eq!(class_of(&f), vec!["factor"]);
        let c = SValue::Classed {
            inner: Box::new(SValue::scalar(1.0)),
            class: vec!["myc".into()],
        };
        assert_eq!(class_of(&c), vec!["myc"]);
    }

    #[test]
    fn format_factor_and_classed_and_data_frame() {
        let f = SValue::Factor {
            codes: vec![Some(2), Some(1)],
            levels: vec!["a".into(), "b".into()],
        };
        assert_eq!(format_value(&f), vec!["[1] b a", "Levels: a b"]);

        // Classed delegates to its inner value's formatting.
        let c = SValue::Classed {
            inner: Box::new(SValue::scalar(5.0)),
            class: vec!["myc".into()],
        };
        assert_eq!(format_value(&c), vec!["[1] 5"]);

        let df = SValue::DataFrame {
            names: vec!["x".into()],
            columns: vec![SValue::doubles(vec![1.0, 2.0])],
        };
        assert!(format_value(&df).len() >= 3); // header + two rows
    }

    #[test]
    fn factor_labels_and_element_string() {
        let codes = vec![Some(2u32), None, Some(1u32)];
        let levels = vec!["a".to_string(), "b".to_string()];
        assert_eq!(
            SValue::factor_labels(&codes, &levels),
            vec![Some("b".to_string()), None, Some("a".to_string())]
        );
        let d = SValue::doubles(vec![1.0, 2.5]);
        assert_eq!(element_string(&d, 1), "2.5");
    }
}
