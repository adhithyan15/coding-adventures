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

/// A built-in function: it receives already-evaluated arguments (v1 is eager —
/// there are no lazy promises yet) and returns an [`SValue`].
pub type Builtin = fn(&[Arg]) -> SResult<SValue>;

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
        }
    }

    /// `length(x)` — the element count.
    pub fn length(&self) -> usize {
        match self {
            SValue::Double(d) => d.len(),
            SValue::Logical(v) => v.len(),
            SValue::Character(v) => v.len(),
            SValue::Null => 0,
            SValue::Closure { .. } | SValue::Builtin { .. } => 1,
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
            other => Err(SError::TypeError(format!(
                "non-numeric argument (got {})",
                other.type_name()
            ))),
        }
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
    };

    format_vector(&elems)
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
