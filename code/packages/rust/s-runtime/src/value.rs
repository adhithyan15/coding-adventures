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

    /// A generic list — an ordered, heterogeneous, optionally-named sequence of
    /// values (R's `list`). `names[i]` is `None` for an unnamed element. Access
    /// is `x[[i]]` / `x[["name"]]` / `x$name` (one element) and `x[i]`
    /// (a sub-list). Implicit class `"list"`.
    List {
        names: Vec<Option<String>>,
        items: Vec<SValue>,
    },

    /// A numeric matrix: an `nrow × ncol` rectangle of doubles stored
    /// **column-major** (R/Fortran order — element `(r, c)` is at `c*nrow + r`).
    /// Implicit class `"matrix"`; `length()` is `nrow*ncol`, `dim()` is
    /// `c(nrow, ncol)`.
    Matrix {
        data: Double,
        nrow: usize,
        ncol: usize,
    },

    /// An atomic vector carrying a **names attribute** (R's `names(x)`). The
    /// `names` vector runs in lockstep with the elements of `values` — one
    /// `Option<String>` per element, `None` for an unset name — and is *always*
    /// kept exactly as long as `values` (every constructor truncates or
    /// `NA`-pads it), so a name lookup can never index out of bounds.
    ///
    /// Like [`SValue::Classed`], this is a **transparent wrapper**: `length`,
    /// `type_name`, the coercions, arithmetic, comparison, and `class_of` all
    /// see straight through to `values`. Only the operations where R actually
    /// observes names — `names()`, character indexing, positional indexing that
    /// carries names along, and printing — look at the `names` field. `values`
    /// is itself never another `Named` (constructors flatten), and is always one
    /// of the atomic variants (`Double`/`Logical`/`Character`).
    Named {
        names: Vec<Option<String>>,
        values: Box<SValue>,
    },

    /// A value carrying **general attributes** — R's open key→value metadata map
    /// (R-16). `attrs` is an insertion-ordered association list of
    /// `(attribute name, attribute value)` pairs. The *special* attributes
    /// `names`, `class`, and `dim` are **never** stored here — they keep their
    /// dedicated representations ([`SValue::Named`], [`SValue::Classed`], and the
    /// matrix `dim`), so `attr(x, "names")` and `names(x)` can never disagree.
    ///
    /// Like [`SValue::Named`] and [`SValue::Classed`], this is a **transparent
    /// wrapper**: `length`, `type_name`, the coercions, arithmetic, comparison,
    /// `class_of`, indexing, and printing all see straight through to `inner`.
    /// Only the attribute builtins (`attr`/`attributes`/`structure`) observe the
    /// map. The map is bounded by [`MAX_ATTRIBUTES`]; an empty map is never
    /// constructed (the wrapper is dropped when its last entry is removed).
    Attributed {
        attrs: Vec<(String, SValue)>,
        inner: Box<SValue>,
    },
}

/// The largest number of *general* attributes a single value may carry. This
/// caps memory against a crafted `attributes(x) <- list(...)` (or a tight
/// `attr<-` loop) that would otherwise grow an unbounded association list. The
/// special attributes (`names`/`class`/`dim`) are not counted — they live in
/// their own wrappers — so this bounds only the open metadata map. 4096 is far
/// beyond any realistic object.
pub const MAX_ATTRIBUTES: usize = 4096;

impl SValue {
    /// Build a list from `(optional name, value)` pairs.
    pub fn list(pairs: Vec<(Option<String>, SValue)>) -> SValue {
        let (names, items) = pairs.into_iter().unzip();
        SValue::List { names, items }
    }

    /// Wrap an atomic `values` vector with a names attribute, normalizing the
    /// names to exactly the element count (truncating a too-long names vector,
    /// `NA`-padding a too-short one). If `values` is *already* `Named`, its old
    /// names are replaced (R's `names<-` semantics). A `values` that is not an
    /// atomic vector (a list, matrix, data frame, …) is returned unchanged
    /// rather than wrapped — names on those structures are out of scope here.
    pub fn with_names(values: SValue, mut names: Vec<Option<String>>) -> SValue {
        // Unwrap an existing names attribute so we never nest `Named`.
        let inner = match values {
            SValue::Named { values, .. } => *values,
            other => other,
        };
        if !matches!(
            inner,
            SValue::Double(_) | SValue::Logical(_) | SValue::Character(_)
        ) {
            return inner;
        }
        let n = inner.length();
        // Normalize to exactly `n` slots: pad short with NA, truncate long.
        if names.len() < n {
            names.resize(n, None);
        } else {
            names.truncate(n);
        }
        SValue::Named {
            names,
            values: Box::new(inner),
        }
    }

    /// The names attribute of a value, if it carries one (`None` otherwise).
    pub fn names_attr(&self) -> Option<&[Option<String>]> {
        match self {
            SValue::Named { names, .. } => Some(names),
            _ => None,
        }
    }

    /// The atomic value underneath a names wrapper (or the value itself).
    pub fn strip_names(&self) -> &SValue {
        match self {
            SValue::Named { values, .. } => values,
            other => other,
        }
    }

    /// The value underneath a general-attributes wrapper (or the value itself).
    /// Only peels one [`SValue::Attributed`] layer (constructors never nest one).
    pub fn strip_attrs(&self) -> &SValue {
        match self {
            SValue::Attributed { inner, .. } => inner,
            other => other,
        }
    }

    /// The general (non-special) attributes carried by this value, if any.
    pub fn general_attrs(&self) -> Option<&[(String, SValue)]> {
        match self {
            SValue::Attributed { attrs, .. } => Some(attrs),
            _ => None,
        }
    }

    /// Wrap `inner` with the general-attribute association list `attrs`, dropping
    /// the wrapper entirely when `attrs` is empty (an empty `Attributed` is never
    /// constructed). If `inner` is *already* an `Attributed`, the new `attrs`
    /// replace the old ones (the wrapper never nests). The caller is responsible
    /// for keeping `attrs` free of the special names (`names`/`class`/`dim`) and
    /// within [`MAX_ATTRIBUTES`].
    pub fn with_general_attrs(inner: SValue, attrs: Vec<(String, SValue)>) -> SValue {
        let inner = match inner {
            SValue::Attributed { inner, .. } => *inner,
            other => other,
        };
        if attrs.is_empty() {
            inner
        } else {
            SValue::Attributed {
                attrs,
                inner: Box::new(inner),
            }
        }
    }
}

/// The S3 class vector of a value: the explicit class if one was set, otherwise
/// the implicit class derived from the value's type.
pub fn class_of(value: &SValue) -> Vec<String> {
    match value {
        SValue::Classed { class, .. } => class.clone(),
        SValue::Factor { .. } => vec!["factor".to_string()],
        SValue::DataFrame { .. } => vec!["data.frame".to_string()],
        SValue::List { .. } => vec!["list".to_string()],
        SValue::Double(_) => vec!["numeric".to_string()],
        SValue::Logical(_) => vec!["logical".to_string()],
        SValue::Character(_) => vec!["character".to_string()],
        SValue::Null => vec!["NULL".to_string()],
        SValue::Closure { .. } | SValue::Builtin { .. } => vec!["function".to_string()],
        SValue::Matrix { .. } => vec!["matrix".to_string(), "array".to_string()],
        // A names attribute is transparent to `class()` — see through to the
        // underlying value's class (a named numeric is still `"numeric"`).
        SValue::Named { values, .. } => class_of(values),
        // General attributes are transparent to `class()` too — the class lives
        // in `Classed`, not the general map (R-16).
        SValue::Attributed { inner, .. } => class_of(inner),
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
            SValue::List { items, .. } => write!(f, "List({} items)", items.len()),
            SValue::Matrix { nrow, ncol, .. } => write!(f, "Matrix({nrow}x{ncol})"),
            SValue::Named { names, values } => {
                write!(f, "Named({:?}, {values:?})", names)
            }
            SValue::Attributed { attrs, inner } => {
                let keys: Vec<&str> = attrs.iter().map(|(k, _)| k.as_str()).collect();
                write!(f, "Attributed({keys:?}, {inner:?})")
            }
        }
    }
}

/// Position of a type in the coercion lattice `logical < double < character`.
fn type_rank(value: &SValue) -> u8 {
    match value {
        SValue::Logical(_) => 0,
        SValue::Double(_) => 1,
        SValue::Character(_) => 2,
        // A names attribute is transparent: rank by the underlying value.
        SValue::Named { values, .. } => type_rank(values),
        // General attributes are transparent: rank by the underlying value.
        SValue::Attributed { inner, .. } => type_rank(inner),
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
            SValue::List { .. } => "list",
            SValue::Matrix { .. } => "double",
            SValue::Named { values, .. } => values.type_name(),
            SValue::Attributed { inner, .. } => inner.type_name(),
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
            SValue::List { items, .. } => items.len(),
            SValue::Matrix { data, .. } => data.len(),
            SValue::Named { values, .. } => values.length(),
            SValue::Attributed { inner, .. } => inner.length(),
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
            SValue::Named { values, .. } => values.as_double(),
            SValue::Attributed { inner, .. } => inner.as_double(),
            SValue::Matrix { data, .. } => Ok(data.clone()),
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
            SValue::Named { values, .. } => values.as_logical(),
            SValue::Attributed { inner, .. } => inner.as_logical(),
            SValue::Matrix { data, .. } => Ok(data
                .iter()
                .map(|x| if is_na_real(x) { None } else { Some(x != 0.0) })
                .collect()),
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
            SValue::Named { values, .. } => values.as_character(),
            SValue::Attributed { inner, .. } => inner.as_character(),
            SValue::Matrix { data, .. } => data
                .iter()
                .map(|x| {
                    if is_na_real(x) {
                        None
                    } else {
                        Some(format_number(x))
                    }
                })
                .collect(),
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
            SValue::Named { values, .. } => values.truthy(),
            SValue::Attributed { inner, .. } => inner.truthy(),
            SValue::Matrix { data, .. } => SValue::Double(data.clone()).truthy(),
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
///
/// **Names (R-15).** `c()` builds a names attribute iff any contributing
/// argument is *tagged* (`c(a = 1)`) or already *carries* names
/// (`c(x = c(a = 1))`). The R combination rule for each contributing element is:
///
/// | argument tag | element name in piece | resulting name |
/// |--------------|-----------------------|----------------|
/// | `tag`        | `inner`               | `tag.inner`    |
/// | `tag`        | (none)                | `tag` (length 1) / `tag1`, `tag2`, … (longer) |
/// | (none)       | `inner`               | `inner`        |
/// | (none)       | (none)                | `""` (empty)   |
///
/// If no name appears anywhere, the result is a plain unnamed vector.
pub fn combine(args: &[Arg]) -> SValue {
    // Drop NULL arguments; carry each surviving argument's tag alongside its
    // value so we can build names in lockstep with the concatenation.
    let present: Vec<(&Option<String>, &SValue)> = args
        .iter()
        .filter(|a| !matches!(a.value, SValue::Null))
        .map(|a| (&a.name, &a.value))
        .collect();

    if present.is_empty() {
        return SValue::Null;
    }

    let any_named = present
        .iter()
        .any(|(tag, v)| tag.is_some() || v.names_attr().is_some());

    // Build the names vector (only used if `any_named`), one entry per output
    // element, in the same order as the value concatenation below.
    let names: Vec<Option<String>> = if any_named {
        let mut names = Vec::new();
        for (tag, v) in &present {
            let inner_names = v.names_attr();
            let count = v.length();
            for i in 0..count {
                let inner = inner_names.and_then(|ns| ns.get(i).and_then(|o| o.clone()));
                names.push(combined_name(tag.as_deref(), inner.as_deref(), count, i));
            }
        }
        names
    } else {
        Vec::new()
    };

    let rank = present.iter().map(|(_, v)| type_rank(v)).max().unwrap_or(0);

    let combined = match rank {
        2 => {
            // character wins: stringify everything.
            let mut out = Vec::new();
            for (_, v) in &present {
                out.extend(v.as_character());
            }
            SValue::Character(out)
        }
        0 => {
            // all logical: concatenate logical payloads (seeing through any
            // names or general-attribute wrapper).
            let mut out = Vec::new();
            for (_, v) in &present {
                if let SValue::Logical(l) = v.strip_names().strip_attrs() {
                    out.extend(l.iter().cloned());
                }
            }
            SValue::Logical(out)
        }
        _ => {
            // double (with any logicals coerced to 0/1).
            let mut out = Vec::new();
            for (_, v) in &present {
                // as_double cannot fail here: only logical/double remain.
                if let Ok(d) = v.as_double() {
                    out.extend(d.iter());
                }
            }
            SValue::Double(Double::from_values(out))
        }
    };

    if any_named {
        SValue::with_names(combined, names)
    } else {
        combined
    }
}

/// Compute the R-combination name for one output element. `tag` is the
/// argument's name at the call site (`c(tag = …)`); `inner` is the element's own
/// name (if the argument was itself a named vector); `count` is the argument's
/// length and `i` the 0-based position within it. An empty string (not `NA`)
/// marks a positionally-unnamed slot, matching R.
fn combined_name(tag: Option<&str>, inner: Option<&str>, count: usize, i: usize) -> Option<String> {
    let name = match (tag, inner) {
        (Some(t), Some(inner)) => format!("{t}.{inner}"),
        (Some(t), None) => {
            // A scalar tagged argument takes the tag verbatim; a longer one
            // suffixes the 1-based position (`c(p = c(1, 2))` → `p1`, `p2`).
            if count == 1 {
                t.to_string()
            } else {
                format!("{t}{}", i + 1)
            }
        }
        (None, Some(inner)) => inner.to_string(),
        (None, None) => String::new(),
    };
    Some(name)
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
    // See through any names / general-attribute wrapper before deciding numeric
    // vs string compare.
    let (lhs, rhs) = (
        lhs.strip_names().strip_attrs(),
        rhs.strip_names().strip_attrs(),
    );
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

/// Resolve an index vector against a dimension of length `len` into a list of
/// selected 0-based positions (`Some(i)`), where `None` marks a slot that
/// should become `NA` (an out-of-range or `NA` index). Supports R's three index
/// styles:
///
/// * **logical** — a mask recycled to `len`; `TRUE` selects, `FALSE` skips, a
///   `TRUE` past the end (a longer mask) and an `NA` both yield an `NA` slot;
/// * **negative** — `-k` *excludes* position `k` (cannot be mixed with positive
///   subscripts, and `NA` is not allowed);
/// * **positive** — 1-based selection; `0` selects nothing, out-of-range/`NA`
///   yield an `NA` slot.
fn resolve_picks(len: usize, idx: &SValue) -> SResult<Vec<Option<usize>>> {
    resolve_picks_named(len, None, idx)
}

/// As [`resolve_picks`], but also handling **character** index vectors by name
/// (R-15): `x["b"]` / `x[c("a","c")]`. `names` is the base's names attribute (if
/// any); each lookup matches the *first* occurrence of the name, and an
/// unmatched (or `NA`) name yields a `None` slot (→ an `NA` element). A
/// character index against a base with no names selects all-`NA`, as in R.
fn resolve_picks_named(
    len: usize,
    names: Option<&[Option<String>]>,
    idx: &SValue,
) -> SResult<Vec<Option<usize>>> {
    // Character index → match by name.
    if let SValue::Character(keys) = idx.strip_names() {
        let mut picks = Vec::with_capacity(keys.len());
        for key in keys {
            match (key, names) {
                (Some(k), Some(ns)) => {
                    // First matching name; miss → NA slot.
                    picks.push(ns.iter().position(|n| n.as_deref() == Some(k.as_str())));
                }
                // No names on the base, or an NA key → no match.
                _ => picks.push(None),
            }
        }
        return Ok(picks);
    }

    // Logical mask (recycled to the longer of len / mask length).
    if let SValue::Logical(mask) = idx.strip_names() {
        if mask.is_empty() {
            return Ok(Vec::new());
        }
        let span = len.max(mask.len());
        if span > MAX_SEQ_LEN {
            return Err(SError::Index(format!(
                "logical index too long (limit {MAX_SEQ_LEN})"
            )));
        }
        let mut picks = Vec::new();
        for i in 0..span {
            match mask[i % mask.len()] {
                Some(true) => picks.push(if i < len { Some(i) } else { None }),
                Some(false) => {}
                None => picks.push(None),
            }
        }
        return Ok(picks);
    }

    let positions = idx.as_double()?;
    let any_neg = positions.iter().any(|p| !is_na_real(p) && p < 0.0);
    let any_pos = positions.iter().any(|p| !is_na_real(p) && p >= 1.0);
    if any_neg && any_pos {
        return Err(SError::Index(
            "can't mix positive and negative subscripts".into(),
        ));
    }

    if any_neg {
        // Negative subscripts EXCLUDE; NA is not allowed, out-of-range is ignored.
        if positions.iter().any(is_na_real) {
            return Err(SError::Index(
                "NAs are not allowed in negative subscripts".into(),
            ));
        }
        let mut excluded = vec![false; len];
        for p in positions.iter() {
            if p == 0.0 {
                continue;
            }
            let drop = (-p) as usize; // 1-based magnitude
            if drop >= 1 && drop <= len {
                excluded[drop - 1] = true;
            }
        }
        return Ok((0..len).filter(|i| !excluded[*i]).map(Some).collect());
    }

    // Positive (the common case): 1-based, 0 drops, out-of-range/NA → NA slot.
    let mut picks: Vec<Option<usize>> = Vec::new();
    for p in positions.iter() {
        if is_na_real(p) {
            picks.push(None);
            continue;
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
    Ok(picks)
}

/// Apply already-resolved `picks` to an atomic `base` value, materializing the
/// selected elements (a `None` pick → an `NA`/`NULL` slot). Used by the names
/// wrapper in [`index`] to subset the underlying value without recomputing the
/// picks. `base` must be one of the atomic variants; other types are an error.
fn pick_into(base: &SValue, picks: &[Option<usize>]) -> SResult<SValue> {
    Ok(match base {
        SValue::Double(d) => SValue::Double(Double::from_values(
            picks
                .iter()
                .map(|p| p.and_then(|i| d.get_value(i)).unwrap_or_else(na_real))
                .collect(),
        )),
        SValue::Logical(v) => SValue::Logical(
            picks
                .iter()
                .map(|p| p.and_then(|i| v.get(i).copied().flatten()))
                .collect(),
        ),
        SValue::Character(v) => SValue::Character(
            picks
                .iter()
                .map(|p| p.and_then(|i| v.get(i).cloned().flatten()))
                .collect(),
        ),
        other => {
            return Err(SError::Index(format!(
                "object of type '{}' is not subsettable",
                other.type_name()
            )))
        }
    })
}

/// Index `base` with the index vector `idx` (1-based; positive, negative, or
/// logical — see [`resolve_picks`]). A `Matrix` is indexed *linearly* over its
/// flat column-major data (dropping its matrix structure, as R does for `m[i]`).
pub fn index(base: &SValue, idx: &SValue) -> SResult<SValue> {
    // `m[i]` — single-subscript indexing of a matrix is over the flat vector.
    if let SValue::Matrix { data, .. } = base {
        return index(&SValue::Double(data.clone()), idx);
    }

    // A value carrying **general attributes**: `[` drops them in R, so index the
    // underlying value directly (names/dim, living in their own wrappers, are
    // handled by the arms above/below).
    if let SValue::Attributed { inner, .. } = base {
        return index(inner, idx);
    }

    // A **named** vector: index the underlying value, then carry the selected
    // names along (R keeps names through `[`). Character indices resolve by name.
    if let SValue::Named { names, values } = base {
        let len = values.length();
        let picks = resolve_picks_named(len, Some(names), idx)?;
        let selected_values = pick_into(values, &picks)?;
        let selected_names: Vec<Option<String>> = picks
            .iter()
            .map(|p| p.and_then(|i| names.get(i).and_then(|o| o.clone())))
            .collect();
        return Ok(SValue::with_names(selected_values, selected_names));
    }

    let len = base.length();
    let picks = resolve_picks(len, idx)?;

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
        // Single-bracket on a list returns a *sub-list* (NA index → a NULL slot).
        SValue::List { names, items } => {
            let mut out_names = Vec::new();
            let mut out_items = Vec::new();
            for p in &picks {
                match p {
                    Some(i) => {
                        out_names.push(names[*i].clone());
                        out_items.push(items[*i].clone());
                    }
                    None => {
                        out_names.push(None);
                        out_items.push(SValue::Null);
                    }
                }
            }
            SValue::List {
                names: out_names,
                items: out_items,
            }
        }
        other => {
            return Err(SError::Index(format!(
                "object of type '{}' is not subsettable",
                other.type_name()
            )))
        }
    })
}

// ===========================================================================
// 2-D indexing — `x[rows, cols]` (R-13)
// ===========================================================================

/// `x[rows, cols]` — two-subscript indexing, where each subscript is `None`
/// (an empty subscript: select the whole dimension) or `Some(idx)`. Dispatches
/// to matrix or data-frame 2-D subsetting.
pub fn index2d(value: &SValue, rows: Option<&SValue>, cols: Option<&SValue>) -> SResult<SValue> {
    match value {
        SValue::Matrix { data, nrow, ncol } => index_matrix_2d(data, *nrow, *ncol, rows, cols),
        SValue::DataFrame { .. } => crate::dataframe::index2d(value, rows, cols),
        SValue::Classed { inner, .. } => index2d(inner, rows, cols),
        SValue::Attributed { inner, .. } => index2d(inner, rows, cols),
        SValue::Named { values, .. } => index2d(values, rows, cols),
        other => Err(SError::Index(format!(
            "incorrect number of dimensions for {}",
            other.type_name()
        ))),
    }
}

/// Resolve one matrix subscript into concrete 0-based positions along a
/// dimension of length `dim`. An empty subscript (`None`) is the whole
/// dimension; otherwise NA / out-of-range positions are a hard error (R rejects
/// them for matrix subscripts rather than producing NA rows).
fn resolve_dim(slot: Option<&SValue>, dim: usize) -> SResult<Vec<usize>> {
    match slot {
        None => Ok((0..dim).collect()),
        Some(idx) => resolve_picks(dim, idx)?
            .into_iter()
            .map(|p| p.ok_or_else(|| SError::Index("subscript out of bounds".into())))
            .collect(),
    }
}

/// `m[rows, cols]` over a column-major matrix. The result is the `rows × cols`
/// sub-rectangle; following R's default `drop = TRUE`, a single-row or
/// single-column result collapses to a plain vector.
fn index_matrix_2d(
    data: &Double,
    nrow: usize,
    ncol: usize,
    rows: Option<&SValue>,
    cols: Option<&SValue>,
) -> SResult<SValue> {
    let rsel = resolve_dim(rows, nrow)?;
    let csel = resolve_dim(cols, ncol)?;
    let (out_nrow, out_ncol) = (rsel.len(), csel.len());
    let total = out_nrow
        .checked_mul(out_ncol)
        .filter(|&t| t <= MAX_SEQ_LEN)
        .ok_or_else(|| SError::Index(format!("matrix subset too large (limit {MAX_SEQ_LEN})")))?;

    let src = data.data();
    let mut out = vec![0.0; total];
    for (oc, &c) in csel.iter().enumerate() {
        for (or, &r) in rsel.iter().enumerate() {
            // Both are column-major: (r, c) at c*nrow + r.
            out[oc * out_nrow + or] = src[c * nrow + r];
        }
    }

    // drop = TRUE: a single row or column becomes a vector.
    if out_nrow == 1 || out_ncol == 1 {
        Ok(SValue::Double(Double::from_values(out)))
    } else {
        Ok(SValue::Matrix {
            data: Double::from_values(out),
            nrow: out_nrow,
            ncol: out_ncol,
        })
    }
}

// ===========================================================================
// Sub-assignment — `x[i] <- v`, `m[i, j] <- v` (R-14)
// ===========================================================================

/// Resolve a subscript into concrete write positions, rejecting `NA` /
/// out-of-range (R forbids assigning *to* an `NA` or beyond-the-end position —
/// the vector-extending case is deferred). `None` means the whole length.
fn assign_positions(len: usize, slot: Option<&SValue>) -> SResult<Vec<usize>> {
    match slot {
        None => Ok((0..len).collect()),
        Some(idx) => resolve_picks(len, idx)?
            .into_iter()
            .map(|p| {
                p.ok_or_else(|| {
                    SError::Index("NAs/out-of-range are not allowed in index assignment".into())
                })
            })
            .collect(),
    }
}

/// Write the recycled numeric `rhs` into `slots` of `out` (R recycles the
/// replacement to fill the selected cells; an empty replacement is an error).
fn write_recycled(out: &mut [f64], slots: &[usize], rhs: &Double) -> SResult<()> {
    if rhs.is_empty() {
        return Err(SError::BadArgs("replacement has length zero".into()));
    }
    let src = rhs.data();
    for (k, &pos) in slots.iter().enumerate() {
        out[pos] = src[k % src.len()];
    }
    Ok(())
}

/// `base[idx] <- rhs` — single-subscript assignment. Numeric only; a matrix
/// keeps its shape (linear write over the flat column-major data). Returns the
/// modified value (the caller rebinds it), so the original binding is never
/// mutated in place — no aliasing.
pub fn assign_index(base: &SValue, idx: Option<&SValue>, rhs: &SValue) -> SResult<SValue> {
    let rhs_d = rhs.as_double()?;
    match base {
        SValue::Matrix { data, nrow, ncol } => {
            let slots = assign_positions(data.len(), idx)?;
            let mut out = data.data().to_vec();
            write_recycled(&mut out, &slots, &rhs_d)?;
            Ok(SValue::Matrix {
                data: Double::from_values(out),
                nrow: *nrow,
                ncol: *ncol,
            })
        }
        SValue::Double(d) => {
            let slots = assign_positions(d.len(), idx)?;
            let mut out = d.data().to_vec();
            write_recycled(&mut out, &slots, &rhs_d)?;
            Ok(SValue::Double(Double::from_values(out)))
        }
        SValue::Classed { inner, class } => Ok(SValue::Classed {
            inner: Box::new(assign_index(inner, idx, rhs)?),
            class: class.clone(),
        }),
        // A value with general attributes: write through to the inner value and
        // keep the attributes (R preserves them across `x[i] <- v`).
        SValue::Attributed { attrs, inner } => Ok(SValue::Attributed {
            attrs: attrs.clone(),
            inner: Box::new(assign_index(inner, idx, rhs)?),
        }),
        // A named vector: write into the underlying value (resolving a character
        // subscript by name) and keep the names attribute. The selection length
        // is bounded, so this is the same write count as the unnamed path.
        SValue::Named { names, values } => {
            let slots = match idx {
                None => (0..values.length()).collect(),
                Some(i) => resolve_picks_named(values.length(), Some(names), i)?
                    .into_iter()
                    .map(|p| {
                        p.ok_or_else(|| {
                            SError::Index(
                                "NAs/out-of-range are not allowed in index assignment".into(),
                            )
                        })
                    })
                    .collect::<SResult<Vec<usize>>>()?,
            };
            let mut out = values.as_double()?.data().to_vec();
            write_recycled(&mut out, &slots, &rhs_d)?;
            Ok(SValue::Named {
                names: names.clone(),
                values: Box::new(SValue::Double(Double::from_values(out))),
            })
        }
        other => Err(SError::TypeError(format!(
            "cannot index-assign into a value of type '{}'",
            other.type_name()
        ))),
    }
}

/// `m[rows, cols] <- rhs` — two-subscript matrix assignment (column-major; the
/// recycled `rhs` fills the selected cells in column order, matching R).
pub fn assign_index2d(
    base: &SValue,
    rows: Option<&SValue>,
    cols: Option<&SValue>,
    rhs: &SValue,
) -> SResult<SValue> {
    match base {
        SValue::Matrix { data, nrow, ncol } => {
            let (nrow, ncol) = (*nrow, *ncol);
            let rsel = resolve_dim(rows, nrow)?;
            let csel = resolve_dim(cols, ncol)?;
            let rhs_d = rhs.as_double()?;
            if rhs_d.is_empty() {
                return Err(SError::BadArgs("replacement has length zero".into()));
            }
            let src = rhs_d.data();
            let mut out = data.data().to_vec();
            let mut k = 0usize;
            for &c in &csel {
                for &r in &rsel {
                    out[c * nrow + r] = src[k % src.len()];
                    k += 1;
                }
            }
            Ok(SValue::Matrix {
                data: Double::from_values(out),
                nrow,
                ncol,
            })
        }
        SValue::Classed { inner, class } => Ok(SValue::Classed {
            inner: Box::new(assign_index2d(inner, rows, cols, rhs)?),
            class: class.clone(),
        }),
        other => Err(SError::Index(format!(
            "incorrect number of dimensions for {}",
            other.type_name()
        ))),
    }
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
        SValue::List { names, items } => return format_list(names, items),
        SValue::Matrix { data, nrow, ncol } => return format_matrix(data, *nrow, *ncol),
        SValue::Named { names, values } => return format_named(names, values),
        // General attributes are transparent to printing — show the inner value.
        SValue::Attributed { inner, .. } => return format_value(inner),
    };

    format_vector(&elems)
}

/// Render a **named** vector R-style: a row of right-aligned names above a row
/// of right-aligned values, each column as wide as the wider of its name and its
/// value. An unset name prints as `<NA>` (matching R). An empty named vector
/// falls back to the underlying empty-vector form. Long vectors wrap at ~80
/// columns, repeating the name header for each wrapped block.
fn format_named(names: &[Option<String>], values: &SValue) -> Vec<String> {
    // The displayed value cells (same convention as `format_vector`: strings are
    // quoted, doubles use `format_number`, NA logical → "NA", etc.).
    let value_cells: Vec<String> = match value_strings(values) {
        Some(cells) => cells,
        // Empty or non-atomic underlying value: defer to its own formatting.
        None => return format_value(values),
    };
    if value_cells.is_empty() {
        return format_value(values);
    }
    let name_cells: Vec<String> = (0..value_cells.len())
        .map(|i| match names.get(i).and_then(|o| o.as_deref()) {
            Some(s) => s.to_string(),
            None => "<NA>".to_string(),
        })
        .collect();

    // Per-column width = max(name width, value width).
    let widths: Vec<usize> = value_cells
        .iter()
        .zip(&name_cells)
        .map(|(v, n)| v.len().max(n.len()))
        .collect();

    // Wrap so each block's total width stays within ~80 columns.
    let n = value_cells.len();
    let mut lines = Vec::new();
    let mut i = 0;
    while i < n {
        let mut used = 0usize;
        let mut j = i;
        while j < n {
            let add = widths[j] + if j > i { 1 } else { 0 };
            if j > i && used + add > 80 {
                break;
            }
            used += add;
            j += 1;
        }
        let mut name_line = String::new();
        let mut value_line = String::new();
        for k in i..j {
            if k > i {
                name_line.push(' ');
                value_line.push(' ');
            }
            let w = widths[k];
            name_line.push_str(&format!("{:>w$}", name_cells[k]));
            value_line.push_str(&format!("{:>w$}", value_cells[k]));
        }
        lines.push(name_line);
        lines.push(value_line);
        i = j;
    }
    lines
}

/// The display strings for an atomic value's elements (quoted for character),
/// or `None` if the value is empty or not an atomic vector.
fn value_strings(values: &SValue) -> Option<Vec<String>> {
    match values {
        SValue::Double(d) if !d.is_empty() => Some(d.iter().map(format_number).collect()),
        SValue::Logical(v) if !v.is_empty() => Some(
            v.iter()
                .map(|o| match o {
                    Some(true) => "TRUE".to_string(),
                    Some(false) => "FALSE".to_string(),
                    None => "NA".to_string(),
                })
                .collect(),
        ),
        SValue::Character(v) if !v.is_empty() => Some(
            v.iter()
                .map(|o| match o {
                    Some(s) => format!("\"{s}\""),
                    None => "NA".to_string(),
                })
                .collect(),
        ),
        _ => None,
    }
}

/// Render a matrix the way R's console does: a `[,j]` column-header row, then
/// `[i,]`-labelled data rows, every cell right-aligned to a common width.
fn format_matrix(data: &Double, nrow: usize, ncol: usize) -> Vec<String> {
    if nrow == 0 || ncol == 0 {
        return vec![format!("<{nrow} x {ncol} matrix>")];
    }
    // Column-major access: element (r, c) is at c*nrow + r.
    let cell = |r: usize, c: usize| {
        data.get_value(c * nrow + r)
            .map(format_number)
            .unwrap_or_else(|| "NA".to_string())
    };
    let row_label_w = format!("[{nrow},]").len();
    // Each column is as wide as the widest of its header and its cells.
    let col_w: Vec<usize> = (0..ncol)
        .map(|c| {
            let header = format!("[,{}]", c + 1).len();
            (0..nrow)
                .map(|r| cell(r, c).len())
                .max()
                .unwrap_or(1)
                .max(header)
        })
        .collect();

    let mut lines = Vec::with_capacity(nrow + 1);
    let mut header = format!("{:>row_label_w$}", "");
    for (c, &w) in col_w.iter().enumerate() {
        header.push(' ');
        header.push_str(&format!("{:>w$}", format!("[,{}]", c + 1)));
    }
    lines.push(header);
    for r in 0..nrow {
        let mut line = format!("{:>row_label_w$}", format!("[{},]", r + 1));
        for (c, &w) in col_w.iter().enumerate() {
            line.push(' ');
            line.push_str(&format!("{:>w$}", cell(r, c)));
        }
        lines.push(line);
    }
    lines
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
        SValue::Named { values, .. } => element_string(values, i),
        SValue::Attributed { inner, .. } => element_string(inner, i),
        _ => "NA".into(),
    }
}

/// Render a list the way R does: each element under a `$name` (named) or `[[i]]`
/// (unnamed) header, the element's own formatting indented below, blank line
/// between. An empty list prints `list()`.
fn format_list(names: &[Option<String>], items: &[SValue]) -> Vec<String> {
    if items.is_empty() {
        return vec!["list()".to_string()];
    }
    let mut lines = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let header = match names.get(i).and_then(|n| n.clone()) {
            Some(name) if !name.is_empty() => format!("${name}"),
            _ => format!("[[{}]]", i + 1),
        };
        lines.push(header);
        lines.extend(format_value(item));
        lines.push(String::new()); // blank separator
    }
    lines
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
        // negative subscript EXCLUDES that position (R-13)
        assert_eq!(
            dbl(&index(&base, &SValue::scalar(-1.0)).unwrap()),
            vec![20.0, 30.0]
        );
        // mixing positive and negative is an error
        assert!(index(&base, &SValue::doubles(vec![-1.0, 2.0])).is_err());
        // a logical mask selects by position (recycled)
        assert_eq!(
            dbl(&index(
                &base,
                &SValue::Logical(vec![Some(true), Some(false), Some(true)])
            )
            .unwrap()),
            vec![10.0, 30.0]
        );
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

    // --- R-15: named vectors --------------------------------------------

    fn name(s: &str) -> Option<String> {
        Some(s.to_string())
    }

    /// Build an Arg with an optional tag (call-site name).
    fn tagged(tag: Option<&str>, value: SValue) -> Arg {
        Arg {
            name: tag.map(|s| s.to_string()),
            value,
        }
    }

    #[test]
    fn with_names_normalizes_length() {
        // Exactly-right names attach verbatim.
        let v = SValue::with_names(SValue::doubles(vec![1.0, 2.0]), vec![name("a"), name("b")]);
        assert_eq!(v.names_attr(), Some(&[name("a"), name("b")][..]));
        // Too-short pads with NA (None).
        let v = SValue::with_names(SValue::doubles(vec![1.0, 2.0, 3.0]), vec![name("a")]);
        assert_eq!(v.names_attr(), Some(&[name("a"), None, None][..]));
        // Too-long truncates.
        let v = SValue::with_names(
            SValue::doubles(vec![1.0]),
            vec![name("a"), name("b"), name("c")],
        );
        assert_eq!(v.names_attr(), Some(&[name("a")][..]));
        // Re-wrapping a Named replaces its old names (never nests).
        let v = SValue::with_names(v, vec![name("z")]);
        assert!(
            matches!(&v, SValue::Named { values, .. } if !matches!(**values, SValue::Named { .. }))
        );
        assert_eq!(v.names_attr(), Some(&[name("z")][..]));
        // A non-atomic value is returned unwrapped.
        let lst = SValue::list(vec![(None, SValue::scalar(1.0))]);
        assert!(SValue::with_names(lst, vec![name("a")])
            .names_attr()
            .is_none());
    }

    #[test]
    fn combine_attaches_and_combines_names() {
        // c(a = 1, b = 2) attaches the tags.
        let v = combine(&[
            tagged(Some("a"), SValue::scalar(1.0)),
            tagged(Some("b"), SValue::scalar(2.0)),
        ]);
        assert_eq!(v.names_attr(), Some(&[name("a"), name("b")][..]));
        // Nested: c(x = c(a = 1), 2) → "x.a", "".
        let inner = combine(&[tagged(Some("a"), SValue::scalar(1.0))]);
        let v = combine(&[tagged(Some("x"), inner), tagged(None, SValue::scalar(2.0))]);
        assert_eq!(v.names_attr(), Some(&[name("x.a"), name("")][..]));
        // A tagged multi-element argument suffixes its position.
        let v = combine(&[tagged(Some("p"), SValue::doubles(vec![1.0, 2.0]))]);
        assert_eq!(v.names_attr(), Some(&[name("p1"), name("p2")][..]));
        // No names anywhere → a plain unnamed vector.
        let v = combine(&[
            tagged(None, SValue::scalar(1.0)),
            tagged(None, SValue::scalar(2.0)),
        ]);
        assert!(v.names_attr().is_none());
        assert!(matches!(v, SValue::Double(_)));
    }

    #[test]
    fn character_index_resolves_by_name() {
        let names = [name("a"), name("b"), name("c")];
        // Hit → the matching 0-based position; miss → None (NA slot).
        let picks = resolve_picks_named(
            3,
            Some(&names),
            &SValue::Character(vec![name("b"), name("z"), name("a")]),
        )
        .unwrap();
        assert_eq!(picks, vec![Some(1), None, Some(0)]);
        // No names on the base → all-None.
        let picks = resolve_picks_named(3, None, &SValue::Character(vec![name("a")])).unwrap();
        assert_eq!(picks, vec![None]);
    }

    #[test]
    fn index_on_named_carries_names_and_resolves_characters() {
        let v = SValue::with_names(
            SValue::doubles(vec![1.0, 2.0, 3.0]),
            vec![name("a"), name("b"), name("c")],
        );
        // x["b"] → value 2, name "b".
        let r = index(&v, &SValue::Character(vec![name("b")])).unwrap();
        assert_eq!(r.strip_names().as_double().unwrap().data(), &[2.0]);
        assert_eq!(r.names_attr(), Some(&[name("b")][..]));
        // x[c(1, 3)] keeps names a, c.
        let r = index(&v, &SValue::doubles(vec![1.0, 3.0])).unwrap();
        assert_eq!(r.names_attr(), Some(&[name("a"), name("c")][..]));
        // A miss → NA value and NA name.
        let r = index(&v, &SValue::Character(vec![name("z")])).unwrap();
        assert!(is_na_real(r.strip_names().as_double().unwrap().data()[0]));
        assert_eq!(r.names_attr(), Some(&[None][..]));
    }

    #[test]
    fn assign_index_keeps_names_and_resolves_character_subscript() {
        let v = SValue::with_names(
            SValue::doubles(vec![1.0, 2.0, 3.0]),
            vec![name("a"), name("b"), name("c")],
        );
        // Positional assignment keeps names.
        let r = assign_index(&v, Some(&SValue::scalar(2.0)), &SValue::scalar(9.0)).unwrap();
        assert_eq!(
            r.strip_names().as_double().unwrap().data(),
            &[1.0, 9.0, 3.0]
        );
        assert_eq!(r.names_attr(), Some(&[name("a"), name("b"), name("c")][..]));
        // Character subscript assignment writes the named element.
        let r = assign_index(
            &v,
            Some(&SValue::Character(vec![name("a")])),
            &SValue::scalar(5.0),
        )
        .unwrap();
        assert_eq!(
            r.strip_names().as_double().unwrap().data(),
            &[5.0, 2.0, 3.0]
        );
    }

    #[test]
    fn named_value_is_transparent_to_core_ops() {
        let v = SValue::with_names(SValue::doubles(vec![1.0, 2.0]), vec![name("a"), name("b")]);
        assert_eq!(v.length(), 2);
        assert_eq!(v.type_name(), "double");
        assert_eq!(class_of(&v), vec!["numeric"]);
        assert_eq!(v.as_double().unwrap().data(), &[1.0, 2.0]);
        // Arithmetic sees through (and the result has no names).
        let r = arithmetic("+", &v, &SValue::scalar(1.0)).unwrap();
        assert_eq!(dbl(&r), vec![2.0, 3.0]);
        assert!(r.names_attr().is_none());
        // Comparison on a named character vector still compares as strings.
        let cv = SValue::with_names(SValue::Character(vec![name("x")]), vec![name("k")]);
        let r = compare("==", &cv, &SValue::Character(vec![name("x")])).unwrap();
        assert!(matches!(&r, SValue::Logical(v) if v[0] == Some(true)));
    }

    #[test]
    fn format_named_lays_out_two_aligned_rows() {
        let v = SValue::with_names(
            SValue::doubles(vec![1.0, 2.0, 3.0]),
            vec![name("a"), name("b"), name("c")],
        );
        assert_eq!(format_value(&v), vec!["a b c", "1 2 3"]);
        // A wider value widens the name column too; an unset name prints <NA>.
        let v = SValue::with_names(SValue::doubles(vec![100.0, 2.0]), vec![name("x"), None]);
        assert_eq!(format_value(&v), vec!["  x <NA>", "100    2"]);
    }

    // --- R-16: general-attribute wrapper mechanics ----------------------

    fn attributed(attrs: Vec<(&str, SValue)>, inner: SValue) -> SValue {
        SValue::with_general_attrs(
            inner,
            attrs.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
        )
    }

    #[test]
    fn with_general_attrs_drops_empty_and_never_nests() {
        // An empty attr list returns the bare value (no wrapper).
        let bare = SValue::with_general_attrs(SValue::scalar(1.0), vec![]);
        assert!(matches!(bare, SValue::Double(_)));
        assert!(bare.general_attrs().is_none());
        // A non-empty list wraps.
        let v = attributed(vec![("foo", SValue::scalar(9.0))], SValue::scalar(1.0));
        assert_eq!(v.general_attrs().map(|a| a.len()), Some(1));
        // Re-wrapping replaces (never nests an Attributed in an Attributed).
        let v2 = attributed(vec![("bar", SValue::scalar(2.0))], v);
        if let SValue::Attributed { attrs, inner } = &v2 {
            assert_eq!(attrs.len(), 1);
            assert_eq!(attrs[0].0, "bar");
            assert!(!matches!(**inner, SValue::Attributed { .. }));
        } else {
            panic!("expected Attributed");
        }
    }

    #[test]
    fn attributed_is_transparent_to_core_ops() {
        let v = attributed(
            vec![("foo", SValue::Character(vec![name("bar")]))],
            SValue::doubles(vec![1.0, 2.0, 3.0]),
        );
        // length / type / class / coercions / truthy all see through.
        assert_eq!(v.length(), 3);
        assert_eq!(v.type_name(), "double");
        assert_eq!(class_of(&v), vec!["numeric"]);
        assert_eq!(v.as_double().unwrap().data(), &[1.0, 2.0, 3.0]);
        assert_eq!(
            v.strip_attrs().as_double().unwrap().data(),
            &[1.0, 2.0, 3.0]
        );
        // Arithmetic sees through and drops the attribute.
        let r = arithmetic("+", &v, &SValue::scalar(1.0)).unwrap();
        assert_eq!(dbl(&r), vec![2.0, 3.0, 4.0]);
        assert!(r.general_attrs().is_none());
    }

    #[test]
    fn attributed_index_drops_attrs_but_assign_keeps_them() {
        let v = attributed(
            vec![("foo", SValue::scalar(7.0))],
            SValue::doubles(vec![10.0, 20.0, 30.0]),
        );
        // `[` drops general attributes (as in R).
        let got = index(&v, &SValue::scalar(2.0)).unwrap();
        assert_eq!(dbl(&got), vec![20.0]);
        assert!(got.general_attrs().is_none());
        // `x[i] <- v` keeps them.
        let assigned = assign_index(&v, Some(&SValue::scalar(1.0)), &SValue::scalar(99.0)).unwrap();
        assert_eq!(
            assigned.strip_attrs().as_double().unwrap().data(),
            &[99.0, 20.0, 30.0]
        );
        assert_eq!(assigned.general_attrs().map(|a| a.len()), Some(1));
    }

    #[test]
    fn attributed_format_and_compare_see_through() {
        let v = attributed(
            vec![("foo", SValue::scalar(1.0))],
            SValue::Character(vec![name("a"), name("b")]),
        );
        // Printing shows the inner value, not the attribute.
        assert_eq!(format_value(&v), vec!["[1] \"a\" \"b\""]);
        // String comparison still works through the wrapper.
        let r = compare("==", &v, &SValue::Character(vec![name("a")])).unwrap();
        assert!(matches!(&r, SValue::Logical(l) if l[0] == Some(true)));
    }
}
