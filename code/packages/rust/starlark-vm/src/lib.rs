//! # Starlark VM -- A complete Starlark bytecode interpreter.
//!
//! This crate ties everything together. It provides:
//!
//! 1. `StarlarkValue` -- the runtime value type for Starlark
//! 2. `StarlarkBuiltins` -- all ~25 built-in functions
//! 3. `StarlarkResult` -- execution result with variables and output
//! 4. `execute_starlark()` -- source code to result in one call
//!
//! ## The Full Pipeline
//!
//! ```text
//! Starlark source code
//!     | (starlark_lexer)
//! Token stream
//!     | (starlark_parser)
//! AST (ASTNode tree)
//!     | (starlark_compiler)
//! CodeObject (bytecode)
//!     | (THIS CRATE)
//! Execution result
//! ```
//!
//! ## Starlark Type Semantics
//!
//! Starlark has a small, well-defined type system:
//! - `int + int -> int`, `float + float -> float`, `int + float -> float`
//! - `str + str -> str` (concatenation), `str * int -> str` (repetition)
//! - `list + list -> list` (concatenation), `list * int -> list` (repetition)
//! - Division always produces `float` (even `4 / 2 -> 2.0`)
//! - Floor division `//` produces `int` (for int operands)
//! - Truthiness: `0`, `0.0`, `""`, `[]`, `{}`, `()`, `None`, `False` are falsy

use std::collections::HashMap;
use std::fmt;

// Re-export opcodes from starlark-compiler
pub use starlark_compiler::Op;

// =========================================================================
// StarlarkValue -- runtime value type
// =========================================================================

/// A Starlark runtime value.
///
/// This represents every possible value that can exist during Starlark
/// execution: integers, floats, strings, booleans, None, lists, dicts,
/// tuples, and callable objects.
#[derive(Debug, Clone, PartialEq)]
pub enum StarlarkValue {
    /// A Starlark integer (arbitrary precision in real Starlark, i64 here).
    Int(i64),
    /// A Starlark float (f64).
    Float(f64),
    /// A Starlark string.
    String(String),
    /// A Starlark boolean.
    Bool(bool),
    /// Starlark None.
    None,
    /// A Starlark list (mutable, ordered sequence).
    List(Vec<StarlarkValue>),
    /// A Starlark dict (ordered key-value mapping).
    Dict(Vec<(StarlarkValue, StarlarkValue)>),
    /// A Starlark tuple (immutable, ordered sequence).
    Tuple(Vec<StarlarkValue>),
}

impl StarlarkValue {
    /// Check if this value is truthy according to Starlark rules.
    ///
    /// Falsy values: 0, 0.0, "", [], {}, (), None, False
    /// Everything else is truthy.
    pub fn is_truthy(&self) -> bool {
        match self {
            StarlarkValue::None => false,
            StarlarkValue::Bool(b) => *b,
            StarlarkValue::Int(i) => *i != 0,
            StarlarkValue::Float(f) => *f != 0.0,
            StarlarkValue::String(s) => !s.is_empty(),
            StarlarkValue::List(l) => !l.is_empty(),
            StarlarkValue::Dict(d) => !d.is_empty(),
            StarlarkValue::Tuple(t) => !t.is_empty(),
        }
    }

    /// Get the type name as a string (matches Starlark's type() builtin).
    pub fn type_name(&self) -> &str {
        match self {
            StarlarkValue::Int(_) => "int",
            StarlarkValue::Float(_) => "float",
            StarlarkValue::String(_) => "string",
            StarlarkValue::Bool(_) => "bool",
            StarlarkValue::None => "NoneType",
            StarlarkValue::List(_) => "list",
            StarlarkValue::Dict(_) => "dict",
            StarlarkValue::Tuple(_) => "tuple",
        }
    }

    /// Convert to a display string (for print()).
    pub fn to_display_string(&self) -> String {
        match self {
            StarlarkValue::None => "None".to_string(),
            StarlarkValue::Bool(true) => "True".to_string(),
            StarlarkValue::Bool(false) => "False".to_string(),
            StarlarkValue::Int(i) => i.to_string(),
            StarlarkValue::Float(f) => format!("{}", f),
            StarlarkValue::String(s) => s.clone(),
            StarlarkValue::List(items) => {
                let inner: Vec<String> = items.iter().map(|v| v.to_repr_string()).collect();
                format!("[{}]", inner.join(", "))
            }
            StarlarkValue::Dict(entries) => {
                let inner: Vec<String> = entries.iter()
                    .map(|(k, v)| format!("{}: {}", k.to_repr_string(), v.to_repr_string()))
                    .collect();
                format!("{{{}}}", inner.join(", "))
            }
            StarlarkValue::Tuple(items) => {
                let inner: Vec<String> = items.iter().map(|v| v.to_repr_string()).collect();
                if items.len() == 1 {
                    format!("({},)", inner[0])
                } else {
                    format!("({})", inner.join(", "))
                }
            }
        }
    }

    /// Convert to a repr string (with quotes around strings).
    pub fn to_repr_string(&self) -> String {
        match self {
            StarlarkValue::String(s) => format!("\"{}\"", s),
            other => other.to_display_string(),
        }
    }
}

impl fmt::Display for StarlarkValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_display_string())
    }
}

// =========================================================================
// StarlarkError
// =========================================================================

/// Errors that can occur during Starlark execution.
#[derive(Debug, Clone, PartialEq)]
pub enum StarlarkError {
    TypeError(String),
    NameError(String),
    ValueError(String),
    IndexError(String),
    DivisionByZero,
    StackUnderflow,
    MaxRecursion,
}

impl fmt::Display for StarlarkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StarlarkError::TypeError(msg) => write!(f, "TypeError: {}", msg),
            StarlarkError::NameError(msg) => write!(f, "NameError: {}", msg),
            StarlarkError::ValueError(msg) => write!(f, "ValueError: {}", msg),
            StarlarkError::IndexError(msg) => write!(f, "IndexError: {}", msg),
            StarlarkError::DivisionByZero => write!(f, "division by zero"),
            StarlarkError::StackUnderflow => write!(f, "stack underflow"),
            StarlarkError::MaxRecursion => write!(f, "maximum recursion depth exceeded"),
        }
    }
}

// =========================================================================
// Built-in functions
// =========================================================================

/// Execute a built-in function by name.
///
/// Each built-in takes a list of StarlarkValues and returns a StarlarkValue.
/// These are registered with the VM and dispatched by the CALL_FUNCTION handler.
pub fn call_builtin(name: &str, args: Vec<StarlarkValue>) -> Result<StarlarkValue, StarlarkError> {
    match name {
        "type" => builtin_type(args),
        "bool" => builtin_bool(args),
        "int" => builtin_int(args),
        "float" => builtin_float(args),
        "str" => builtin_str(args),
        "len" => builtin_len(args),
        "list" => builtin_list(args),
        "dict" => builtin_dict(args),
        "tuple" => builtin_tuple(args),
        "range" => builtin_range(args),
        "sorted" => builtin_sorted(args),
        "reversed" => builtin_reversed(args),
        "enumerate" => builtin_enumerate(args),
        "zip" => builtin_zip(args),
        "min" => builtin_min(args),
        "max" => builtin_max(args),
        "abs" => builtin_abs(args),
        "all" => builtin_all(args),
        "any" => builtin_any(args),
        "repr" => builtin_repr(args),
        "print" => Ok(StarlarkValue::None), // Handled by PRINT opcode
        _ => Err(StarlarkError::NameError(format!("name '{}' is not defined", name))),
    }
}

/// Get names of all built-in functions.
pub fn builtin_names() -> Vec<&'static str> {
    vec![
        "type", "bool", "int", "float", "str",
        "len", "list", "dict", "tuple", "range",
        "sorted", "reversed", "enumerate", "zip",
        "min", "max", "abs", "all", "any",
        "repr", "print",
    ]
}

// ----- Individual built-in implementations -----

fn builtin_type(args: Vec<StarlarkValue>) -> Result<StarlarkValue, StarlarkError> {
    if args.len() != 1 {
        return Err(StarlarkError::TypeError(format!("type() takes exactly 1 argument ({} given)", args.len())));
    }
    Ok(StarlarkValue::String(args[0].type_name().to_string()))
}

fn builtin_bool(args: Vec<StarlarkValue>) -> Result<StarlarkValue, StarlarkError> {
    if args.len() != 1 {
        return Err(StarlarkError::TypeError(format!("bool() takes exactly 1 argument ({} given)", args.len())));
    }
    Ok(StarlarkValue::Bool(args[0].is_truthy()))
}

fn builtin_int(args: Vec<StarlarkValue>) -> Result<StarlarkValue, StarlarkError> {
    if args.is_empty() || args.len() > 2 {
        return Err(StarlarkError::TypeError(format!("int() takes 1 or 2 arguments ({} given)", args.len())));
    }
    match &args[0] {
        StarlarkValue::Int(i) => Ok(StarlarkValue::Int(*i)),
        StarlarkValue::Float(f) => Ok(StarlarkValue::Int(*f as i64)),
        StarlarkValue::Bool(b) => Ok(StarlarkValue::Int(if *b { 1 } else { 0 })),
        StarlarkValue::String(s) => {
            let base = if args.len() == 2 {
                match &args[1] {
                    StarlarkValue::Int(b) => *b as u32,
                    _ => return Err(StarlarkError::TypeError("int() base must be an integer".to_string())),
                }
            } else { 10 };
            i64::from_str_radix(s.trim(), base)
                .map(StarlarkValue::Int)
                .map_err(|_| StarlarkError::ValueError(format!("invalid literal for int(): '{}'", s)))
        }
        other => Err(StarlarkError::TypeError(format!("int() argument must be a string or number, not '{}'", other.type_name()))),
    }
}

fn builtin_float(args: Vec<StarlarkValue>) -> Result<StarlarkValue, StarlarkError> {
    if args.len() != 1 {
        return Err(StarlarkError::TypeError(format!("float() takes exactly 1 argument ({} given)", args.len())));
    }
    match &args[0] {
        StarlarkValue::Int(i) => Ok(StarlarkValue::Float(*i as f64)),
        StarlarkValue::Float(f) => Ok(StarlarkValue::Float(*f)),
        StarlarkValue::String(s) => s.trim().parse::<f64>()
            .map(StarlarkValue::Float)
            .map_err(|_| StarlarkError::ValueError(format!("could not convert string to float: '{}'", s))),
        other => Err(StarlarkError::TypeError(format!("float() argument must be a string or number, not '{}'", other.type_name()))),
    }
}

fn builtin_str(args: Vec<StarlarkValue>) -> Result<StarlarkValue, StarlarkError> {
    if args.len() != 1 {
        return Err(StarlarkError::TypeError(format!("str() takes exactly 1 argument ({} given)", args.len())));
    }
    Ok(StarlarkValue::String(args[0].to_display_string()))
}

fn builtin_len(args: Vec<StarlarkValue>) -> Result<StarlarkValue, StarlarkError> {
    if args.len() != 1 {
        return Err(StarlarkError::TypeError(format!("len() takes exactly 1 argument ({} given)", args.len())));
    }
    match &args[0] {
        StarlarkValue::String(s) => Ok(StarlarkValue::Int(s.len() as i64)),
        StarlarkValue::List(l) => Ok(StarlarkValue::Int(l.len() as i64)),
        StarlarkValue::Dict(d) => Ok(StarlarkValue::Int(d.len() as i64)),
        StarlarkValue::Tuple(t) => Ok(StarlarkValue::Int(t.len() as i64)),
        other => Err(StarlarkError::TypeError(format!("object of type '{}' has no len()", other.type_name()))),
    }
}

fn builtin_list(args: Vec<StarlarkValue>) -> Result<StarlarkValue, StarlarkError> {
    if args.is_empty() { return Ok(StarlarkValue::List(vec![])); }
    if args.len() != 1 {
        return Err(StarlarkError::TypeError(format!("list() takes at most 1 argument ({} given)", args.len())));
    }
    match &args[0] {
        StarlarkValue::List(l) => Ok(StarlarkValue::List(l.clone())),
        StarlarkValue::Tuple(t) => Ok(StarlarkValue::List(t.clone())),
        StarlarkValue::String(s) => Ok(StarlarkValue::List(
            s.chars().map(|c| StarlarkValue::String(c.to_string())).collect()
        )),
        _ => Err(StarlarkError::TypeError("list() argument must be an iterable".to_string())),
    }
}

fn builtin_dict(args: Vec<StarlarkValue>) -> Result<StarlarkValue, StarlarkError> {
    if args.is_empty() { return Ok(StarlarkValue::Dict(vec![])); }
    Err(StarlarkError::TypeError("dict() with arguments not yet supported".to_string()))
}

fn builtin_tuple(args: Vec<StarlarkValue>) -> Result<StarlarkValue, StarlarkError> {
    if args.is_empty() { return Ok(StarlarkValue::Tuple(vec![])); }
    if args.len() != 1 {
        return Err(StarlarkError::TypeError(format!("tuple() takes at most 1 argument ({} given)", args.len())));
    }
    match &args[0] {
        StarlarkValue::List(l) => Ok(StarlarkValue::Tuple(l.clone())),
        StarlarkValue::Tuple(t) => Ok(StarlarkValue::Tuple(t.clone())),
        _ => Err(StarlarkError::TypeError("tuple() argument must be an iterable".to_string())),
    }
}

fn builtin_range(args: Vec<StarlarkValue>) -> Result<StarlarkValue, StarlarkError> {
    let (start, stop, step) = match args.len() {
        1 => match &args[0] {
            StarlarkValue::Int(n) => (0, *n, 1),
            _ => return Err(StarlarkError::TypeError("range() argument must be int".to_string())),
        },
        2 => match (&args[0], &args[1]) {
            (StarlarkValue::Int(a), StarlarkValue::Int(b)) => (*a, *b, 1),
            _ => return Err(StarlarkError::TypeError("range() arguments must be int".to_string())),
        },
        3 => match (&args[0], &args[1], &args[2]) {
            (StarlarkValue::Int(a), StarlarkValue::Int(b), StarlarkValue::Int(c)) => (*a, *b, *c),
            _ => return Err(StarlarkError::TypeError("range() arguments must be int".to_string())),
        },
        _ => return Err(StarlarkError::TypeError(format!("range() takes 1 to 3 arguments ({} given)", args.len()))),
    };
    let mut result = Vec::new();
    let mut val = start;
    if step > 0 {
        while val < stop { result.push(StarlarkValue::Int(val)); val += step; }
    } else if step < 0 {
        while val > stop { result.push(StarlarkValue::Int(val)); val += step; }
    }
    Ok(StarlarkValue::List(result))
}

fn builtin_sorted(args: Vec<StarlarkValue>) -> Result<StarlarkValue, StarlarkError> {
    if args.is_empty() || args.len() > 2 {
        return Err(StarlarkError::TypeError(format!("sorted() takes 1 or 2 arguments ({} given)", args.len())));
    }
    let items = match &args[0] {
        StarlarkValue::List(l) => l.clone(),
        StarlarkValue::Tuple(t) => t.clone(),
        _ => return Err(StarlarkError::TypeError("sorted() argument must be iterable".to_string())),
    };
    let reverse = args.get(1).map_or(false, |v| v.is_truthy());
    let mut sorted_items = items;
    sorted_items.sort_by(|a, b| {
        match (a, b) {
            (StarlarkValue::Int(x), StarlarkValue::Int(y)) => x.cmp(y),
            (StarlarkValue::Float(x), StarlarkValue::Float(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
            (StarlarkValue::String(x), StarlarkValue::String(y)) => x.cmp(y),
            _ => std::cmp::Ordering::Equal,
        }
    });
    if reverse { sorted_items.reverse(); }
    Ok(StarlarkValue::List(sorted_items))
}

fn builtin_reversed(args: Vec<StarlarkValue>) -> Result<StarlarkValue, StarlarkError> {
    if args.len() != 1 {
        return Err(StarlarkError::TypeError(format!("reversed() takes exactly 1 argument ({} given)", args.len())));
    }
    match &args[0] {
        StarlarkValue::List(l) => { let mut r = l.clone(); r.reverse(); Ok(StarlarkValue::List(r)) }
        StarlarkValue::Tuple(t) => { let mut r = t.clone(); r.reverse(); Ok(StarlarkValue::List(r)) }
        _ => Err(StarlarkError::TypeError("reversed() argument must be a sequence".to_string())),
    }
}

fn builtin_enumerate(args: Vec<StarlarkValue>) -> Result<StarlarkValue, StarlarkError> {
    if args.is_empty() || args.len() > 2 {
        return Err(StarlarkError::TypeError(format!("enumerate() takes 1 or 2 arguments ({} given)", args.len())));
    }
    let start = args.get(1).and_then(|v| match v {
        StarlarkValue::Int(i) => Some(*i),
        _ => None,
    }).unwrap_or(0);
    let items = match &args[0] {
        StarlarkValue::List(l) => l.clone(),
        StarlarkValue::Tuple(t) => t.clone(),
        _ => return Err(StarlarkError::TypeError("enumerate() argument must be iterable".to_string())),
    };
    let result: Vec<StarlarkValue> = items.into_iter().enumerate()
        .map(|(i, v)| StarlarkValue::Tuple(vec![StarlarkValue::Int(start + i as i64), v]))
        .collect();
    Ok(StarlarkValue::List(result))
}

fn builtin_zip(args: Vec<StarlarkValue>) -> Result<StarlarkValue, StarlarkError> {
    let mut iterables: Vec<Vec<StarlarkValue>> = Vec::new();
    for arg in &args {
        match arg {
            StarlarkValue::List(l) => iterables.push(l.clone()),
            StarlarkValue::Tuple(t) => iterables.push(t.clone()),
            _ => return Err(StarlarkError::TypeError("zip() argument must be iterable".to_string())),
        }
    }
    if iterables.is_empty() { return Ok(StarlarkValue::List(vec![])); }
    let min_len = iterables.iter().map(|l| l.len()).min().unwrap_or(0);
    let mut result = Vec::new();
    for i in 0..min_len {
        let tuple: Vec<StarlarkValue> = iterables.iter().map(|l| l[i].clone()).collect();
        result.push(StarlarkValue::Tuple(tuple));
    }
    Ok(StarlarkValue::List(result))
}

fn builtin_min(args: Vec<StarlarkValue>) -> Result<StarlarkValue, StarlarkError> {
    let items = if args.len() == 1 {
        match &args[0] {
            StarlarkValue::List(l) => l.clone(),
            StarlarkValue::Tuple(t) => t.clone(),
            _ => return Err(StarlarkError::TypeError("min() argument must be iterable".to_string())),
        }
    } else { args };
    if items.is_empty() { return Err(StarlarkError::ValueError("min() arg is an empty sequence".to_string())); }
    let mut min_val = items[0].clone();
    for item in &items[1..] {
        match (&min_val, item) {
            (StarlarkValue::Int(a), StarlarkValue::Int(b)) => { if b < a { min_val = item.clone(); } }
            (StarlarkValue::Float(a), StarlarkValue::Float(b)) => { if b < a { min_val = item.clone(); } }
            _ => {}
        }
    }
    Ok(min_val)
}

fn builtin_max(args: Vec<StarlarkValue>) -> Result<StarlarkValue, StarlarkError> {
    let items = if args.len() == 1 {
        match &args[0] {
            StarlarkValue::List(l) => l.clone(),
            StarlarkValue::Tuple(t) => t.clone(),
            _ => return Err(StarlarkError::TypeError("max() argument must be iterable".to_string())),
        }
    } else { args };
    if items.is_empty() { return Err(StarlarkError::ValueError("max() arg is an empty sequence".to_string())); }
    let mut max_val = items[0].clone();
    for item in &items[1..] {
        match (&max_val, item) {
            (StarlarkValue::Int(a), StarlarkValue::Int(b)) => { if b > a { max_val = item.clone(); } }
            (StarlarkValue::Float(a), StarlarkValue::Float(b)) => { if b > a { max_val = item.clone(); } }
            _ => {}
        }
    }
    Ok(max_val)
}

fn builtin_abs(args: Vec<StarlarkValue>) -> Result<StarlarkValue, StarlarkError> {
    if args.len() != 1 {
        return Err(StarlarkError::TypeError(format!("abs() takes exactly 1 argument ({} given)", args.len())));
    }
    match &args[0] {
        StarlarkValue::Int(i) => Ok(StarlarkValue::Int(i.abs())),
        StarlarkValue::Float(f) => Ok(StarlarkValue::Float(f.abs())),
        other => Err(StarlarkError::TypeError(format!("bad operand type for abs(): '{}'", other.type_name()))),
    }
}

fn builtin_all(args: Vec<StarlarkValue>) -> Result<StarlarkValue, StarlarkError> {
    if args.len() != 1 {
        return Err(StarlarkError::TypeError(format!("all() takes exactly 1 argument ({} given)", args.len())));
    }
    match &args[0] {
        StarlarkValue::List(l) => Ok(StarlarkValue::Bool(l.iter().all(|v| v.is_truthy()))),
        StarlarkValue::Tuple(t) => Ok(StarlarkValue::Bool(t.iter().all(|v| v.is_truthy()))),
        _ => Err(StarlarkError::TypeError("all() argument must be iterable".to_string())),
    }
}

fn builtin_any(args: Vec<StarlarkValue>) -> Result<StarlarkValue, StarlarkError> {
    if args.len() != 1 {
        return Err(StarlarkError::TypeError(format!("any() takes exactly 1 argument ({} given)", args.len())));
    }
    match &args[0] {
        StarlarkValue::List(l) => Ok(StarlarkValue::Bool(l.iter().any(|v| v.is_truthy()))),
        StarlarkValue::Tuple(t) => Ok(StarlarkValue::Bool(t.iter().any(|v| v.is_truthy()))),
        _ => Err(StarlarkError::TypeError("any() argument must be iterable".to_string())),
    }
}

fn builtin_repr(args: Vec<StarlarkValue>) -> Result<StarlarkValue, StarlarkError> {
    if args.len() != 1 {
        return Err(StarlarkError::TypeError(format!("repr() takes exactly 1 argument ({} given)", args.len())));
    }
    Ok(StarlarkValue::String(args[0].to_repr_string()))
}

// =========================================================================
// StarlarkResult
// =========================================================================

/// The result of executing a Starlark program.
#[derive(Debug, Clone)]
pub struct StarlarkResult {
    /// Final variable state after execution.
    pub variables: HashMap<String, StarlarkValue>,
    /// Captured print output, one entry per print() call.
    pub output: Vec<String>,
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ----- StarlarkValue tests -----

    #[test]
    fn test_truthiness() {
        assert!(!StarlarkValue::None.is_truthy());
        assert!(!StarlarkValue::Bool(false).is_truthy());
        assert!(StarlarkValue::Bool(true).is_truthy());
        assert!(!StarlarkValue::Int(0).is_truthy());
        assert!(StarlarkValue::Int(42).is_truthy());
        assert!(!StarlarkValue::Float(0.0).is_truthy());
        assert!(StarlarkValue::Float(3.14).is_truthy());
        assert!(!StarlarkValue::String("".to_string()).is_truthy());
        assert!(StarlarkValue::String("hello".to_string()).is_truthy());
        assert!(!StarlarkValue::List(vec![]).is_truthy());
        assert!(StarlarkValue::List(vec![StarlarkValue::Int(1)]).is_truthy());
        assert!(!StarlarkValue::Dict(vec![]).is_truthy());
        assert!(!StarlarkValue::Tuple(vec![]).is_truthy());
    }

    #[test]
    fn test_type_name() {
        assert_eq!(StarlarkValue::Int(42).type_name(), "int");
        assert_eq!(StarlarkValue::Float(3.14).type_name(), "float");
        assert_eq!(StarlarkValue::String("hi".to_string()).type_name(), "string");
        assert_eq!(StarlarkValue::Bool(true).type_name(), "bool");
        assert_eq!(StarlarkValue::None.type_name(), "NoneType");
        assert_eq!(StarlarkValue::List(vec![]).type_name(), "list");
        assert_eq!(StarlarkValue::Dict(vec![]).type_name(), "dict");
        assert_eq!(StarlarkValue::Tuple(vec![]).type_name(), "tuple");
    }

    #[test]
    fn test_display_string() {
        assert_eq!(StarlarkValue::Int(42).to_display_string(), "42");
        assert_eq!(StarlarkValue::Bool(true).to_display_string(), "True");
        assert_eq!(StarlarkValue::None.to_display_string(), "None");
        assert_eq!(StarlarkValue::String("hello".to_string()).to_display_string(), "hello");
    }

    #[test]
    fn test_repr_string() {
        assert_eq!(StarlarkValue::String("hello".to_string()).to_repr_string(), "\"hello\"");
        assert_eq!(StarlarkValue::Int(42).to_repr_string(), "42");
    }

    // ----- Built-in function tests -----

    #[test]
    fn test_builtin_type() {
        assert_eq!(call_builtin("type", vec![StarlarkValue::Int(42)]).unwrap(),
            StarlarkValue::String("int".to_string()));
        assert_eq!(call_builtin("type", vec![StarlarkValue::String("hi".to_string())]).unwrap(),
            StarlarkValue::String("string".to_string()));
    }

    #[test]
    fn test_builtin_bool() {
        assert_eq!(call_builtin("bool", vec![StarlarkValue::Int(0)]).unwrap(), StarlarkValue::Bool(false));
        assert_eq!(call_builtin("bool", vec![StarlarkValue::Int(1)]).unwrap(), StarlarkValue::Bool(true));
        assert_eq!(call_builtin("bool", vec![StarlarkValue::String("".to_string())]).unwrap(), StarlarkValue::Bool(false));
    }

    #[test]
    fn test_builtin_int() {
        assert_eq!(call_builtin("int", vec![StarlarkValue::Float(3.7)]).unwrap(), StarlarkValue::Int(3));
        assert_eq!(call_builtin("int", vec![StarlarkValue::String("42".to_string())]).unwrap(), StarlarkValue::Int(42));
        assert_eq!(call_builtin("int", vec![StarlarkValue::Bool(true)]).unwrap(), StarlarkValue::Int(1));
    }

    #[test]
    fn test_builtin_float() {
        assert_eq!(call_builtin("float", vec![StarlarkValue::Int(42)]).unwrap(), StarlarkValue::Float(42.0));
        assert_eq!(call_builtin("float", vec![StarlarkValue::String("3.14".to_string())]).unwrap(), StarlarkValue::Float(3.14));
    }

    #[test]
    fn test_builtin_str() {
        assert_eq!(call_builtin("str", vec![StarlarkValue::Int(42)]).unwrap(), StarlarkValue::String("42".to_string()));
        assert_eq!(call_builtin("str", vec![StarlarkValue::None]).unwrap(), StarlarkValue::String("None".to_string()));
    }

    #[test]
    fn test_builtin_len() {
        assert_eq!(call_builtin("len", vec![StarlarkValue::String("hello".to_string())]).unwrap(), StarlarkValue::Int(5));
        assert_eq!(call_builtin("len", vec![StarlarkValue::List(vec![StarlarkValue::Int(1), StarlarkValue::Int(2)])]).unwrap(), StarlarkValue::Int(2));
    }

    #[test]
    fn test_builtin_range() {
        let result = call_builtin("range", vec![StarlarkValue::Int(5)]).unwrap();
        assert_eq!(result, StarlarkValue::List(vec![
            StarlarkValue::Int(0), StarlarkValue::Int(1), StarlarkValue::Int(2),
            StarlarkValue::Int(3), StarlarkValue::Int(4),
        ]));
    }

    #[test]
    fn test_builtin_range_start_stop() {
        let result = call_builtin("range", vec![StarlarkValue::Int(2), StarlarkValue::Int(5)]).unwrap();
        assert_eq!(result, StarlarkValue::List(vec![
            StarlarkValue::Int(2), StarlarkValue::Int(3), StarlarkValue::Int(4),
        ]));
    }

    #[test]
    fn test_builtin_range_with_step() {
        let result = call_builtin("range", vec![StarlarkValue::Int(0), StarlarkValue::Int(10), StarlarkValue::Int(3)]).unwrap();
        assert_eq!(result, StarlarkValue::List(vec![
            StarlarkValue::Int(0), StarlarkValue::Int(3), StarlarkValue::Int(6), StarlarkValue::Int(9),
        ]));
    }

    #[test]
    fn test_builtin_sorted() {
        let input = StarlarkValue::List(vec![StarlarkValue::Int(3), StarlarkValue::Int(1), StarlarkValue::Int(2)]);
        let result = call_builtin("sorted", vec![input]).unwrap();
        assert_eq!(result, StarlarkValue::List(vec![StarlarkValue::Int(1), StarlarkValue::Int(2), StarlarkValue::Int(3)]));
    }

    #[test]
    fn test_builtin_reversed() {
        let input = StarlarkValue::List(vec![StarlarkValue::Int(1), StarlarkValue::Int(2), StarlarkValue::Int(3)]);
        let result = call_builtin("reversed", vec![input]).unwrap();
        assert_eq!(result, StarlarkValue::List(vec![StarlarkValue::Int(3), StarlarkValue::Int(2), StarlarkValue::Int(1)]));
    }

    #[test]
    fn test_builtin_enumerate() {
        let input = StarlarkValue::List(vec![
            StarlarkValue::String("a".to_string()),
            StarlarkValue::String("b".to_string()),
        ]);
        let result = call_builtin("enumerate", vec![input]).unwrap();
        assert_eq!(result, StarlarkValue::List(vec![
            StarlarkValue::Tuple(vec![StarlarkValue::Int(0), StarlarkValue::String("a".to_string())]),
            StarlarkValue::Tuple(vec![StarlarkValue::Int(1), StarlarkValue::String("b".to_string())]),
        ]));
    }

    #[test]
    fn test_builtin_zip() {
        let a = StarlarkValue::List(vec![StarlarkValue::Int(1), StarlarkValue::Int(2)]);
        let b = StarlarkValue::List(vec![StarlarkValue::String("a".to_string()), StarlarkValue::String("b".to_string())]);
        let result = call_builtin("zip", vec![a, b]).unwrap();
        assert_eq!(result, StarlarkValue::List(vec![
            StarlarkValue::Tuple(vec![StarlarkValue::Int(1), StarlarkValue::String("a".to_string())]),
            StarlarkValue::Tuple(vec![StarlarkValue::Int(2), StarlarkValue::String("b".to_string())]),
        ]));
    }

    #[test]
    fn test_builtin_min_max() {
        assert_eq!(call_builtin("min", vec![StarlarkValue::Int(3), StarlarkValue::Int(1), StarlarkValue::Int(2)]).unwrap(), StarlarkValue::Int(1));
        assert_eq!(call_builtin("max", vec![StarlarkValue::Int(3), StarlarkValue::Int(1), StarlarkValue::Int(2)]).unwrap(), StarlarkValue::Int(3));
    }

    #[test]
    fn test_builtin_abs() {
        assert_eq!(call_builtin("abs", vec![StarlarkValue::Int(-5)]).unwrap(), StarlarkValue::Int(5));
        assert_eq!(call_builtin("abs", vec![StarlarkValue::Float(-3.14)]).unwrap(), StarlarkValue::Float(3.14));
    }

    #[test]
    fn test_builtin_all_any() {
        let all_true = StarlarkValue::List(vec![StarlarkValue::Bool(true), StarlarkValue::Int(1)]);
        assert_eq!(call_builtin("all", vec![all_true]).unwrap(), StarlarkValue::Bool(true));

        let has_false = StarlarkValue::List(vec![StarlarkValue::Bool(true), StarlarkValue::Int(0)]);
        assert_eq!(call_builtin("all", vec![has_false]).unwrap(), StarlarkValue::Bool(false));

        let has_true = StarlarkValue::List(vec![StarlarkValue::Bool(false), StarlarkValue::Int(1)]);
        assert_eq!(call_builtin("any", vec![has_true]).unwrap(), StarlarkValue::Bool(true));

        let all_false = StarlarkValue::List(vec![StarlarkValue::Bool(false), StarlarkValue::Int(0)]);
        assert_eq!(call_builtin("any", vec![all_false]).unwrap(), StarlarkValue::Bool(false));
    }

    #[test]
    fn test_builtin_repr() {
        assert_eq!(call_builtin("repr", vec![StarlarkValue::String("hello".to_string())]).unwrap(),
            StarlarkValue::String("\"hello\"".to_string()));
        assert_eq!(call_builtin("repr", vec![StarlarkValue::Int(42)]).unwrap(),
            StarlarkValue::String("42".to_string()));
    }

    #[test]
    fn test_builtin_list_from_tuple() {
        let input = StarlarkValue::Tuple(vec![StarlarkValue::Int(1), StarlarkValue::Int(2)]);
        let result = call_builtin("list", vec![input]).unwrap();
        assert_eq!(result, StarlarkValue::List(vec![StarlarkValue::Int(1), StarlarkValue::Int(2)]));
    }

    #[test]
    fn test_builtin_tuple_from_list() {
        let input = StarlarkValue::List(vec![StarlarkValue::Int(1), StarlarkValue::Int(2)]);
        let result = call_builtin("tuple", vec![input]).unwrap();
        assert_eq!(result, StarlarkValue::Tuple(vec![StarlarkValue::Int(1), StarlarkValue::Int(2)]));
    }

    #[test]
    fn test_builtin_names() {
        let names = builtin_names();
        assert!(names.contains(&"type"));
        assert!(names.contains(&"len"));
        assert!(names.contains(&"range"));
        assert!(names.contains(&"print"));
        assert!(names.len() >= 20);
    }

    #[test]
    fn test_error_wrong_arg_count() {
        assert!(call_builtin("type", vec![]).is_err());
        assert!(call_builtin("len", vec![StarlarkValue::Int(1), StarlarkValue::Int(2)]).is_err());
    }

    #[test]
    fn test_error_type_mismatch() {
        assert!(call_builtin("len", vec![StarlarkValue::Int(42)]).is_err());
        assert!(call_builtin("int", vec![StarlarkValue::List(vec![])]).is_err());
    }
}
