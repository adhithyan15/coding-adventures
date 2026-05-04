//! [`Value`] — the dynamic value type stored in VM registers.
//!
//! Python's `Any` registry is replaced in Rust by a closed enum.  The five
//! variants cover every type the standard IIR opcode handlers produce:
//!
//! | Variant | IIR type strings | Notes |
//! |---------|-----------------|-------|
//! | `Int(i64)` | `"u8"` .. `"i64"` | Includes all unsigned widths |
//! | `Float(f64)` | `"f64"`, `"f32"` | Single 64-bit slot |
//! | `Bool(bool)` | `"bool"` | |
//! | `Str(String)` | `"str"` | Heap-allocated |
//! | `Null` | `"void"`, none | Default register value; ret_void result |
//!
//! Language frontends that need additional value kinds (cons cells, class
//! instances, heap refs) may extend this in a wrapper enum without modifying
//! vm-core itself.
//!
//! # Arithmetic helpers
//!
//! The `as_i64`, `as_f64`, `as_bool`, and `as_str` extractors return `None`
//! for a type mismatch so the dispatch loop can surface a clean
//! [`VMError::TypeError`] instead of a Rust panic.
//!
//! # Example
//!
//! ```
//! use vm_core::value::Value;
//!
//! let v = Value::Int(42);
//! assert_eq!(v.as_i64(), Some(42));
//! assert!(v.as_f64().is_none());
//!
//! let b = Value::Bool(true);
//! assert!(b.is_truthy());
//! ```

/// The dynamic value stored in a VM register.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// A signed integer covering all of u8 / u16 / u32 / u64 / i8..i64.
    ///
    /// The VM's `u8_wrap` flag masks the result with `& 0xFF` after every
    /// arithmetic operation when Tetrad 8-bit semantics are required.
    Int(i64),
    /// An IEEE 754 double-precision float.
    Float(f64),
    /// A boolean.
    Bool(bool),
    /// A heap-allocated string.
    Str(String),
    /// The absence of a value — default register contents and `ret_void` result.
    Null,
}

impl Value {
    // ------------------------------------------------------------------
    // Type extractors
    // ------------------------------------------------------------------

    /// Return the integer value if this is `Value::Int`, else `None`.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Int(n) => Some(*n),
            Value::Bool(b) => Some(*b as i64),
            _ => None,
        }
    }

    /// Return the float value if this is `Value::Float`, else `None`.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// Return the bool value if this is `Value::Bool`, else `None`.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Return the string slice if this is `Value::Str`, else `None`.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }

    // ------------------------------------------------------------------
    // Truth test (used by conditional branches)
    // ------------------------------------------------------------------

    /// Return whether the value is "truthy".
    ///
    /// - `Bool(false)` and `Int(0)` are falsy.
    /// - `Null` is falsy.
    /// - Everything else is truthy.
    ///
    /// This mirrors Python's `bool()` semantics and the behaviour of
    /// `jmp_if_true` / `jmp_if_false` in the Python vm-core.
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b)  => *b,
            Value::Int(n)   => *n != 0,
            Value::Float(f) => *f != 0.0,
            Value::Str(s)   => !s.is_empty(),
            Value::Null     => false,
        }
    }

    // ------------------------------------------------------------------
    // Type-name helper (for profiler)
    // ------------------------------------------------------------------

    /// Map this value to the IIR type string the profiler should record.
    ///
    /// Matches the `default_type_mapper` from the Python vm-core:
    ///
    /// | Python type | IIR type |
    /// |-------------|----------|
    /// | `bool`      | `"bool"` |
    /// | `int` 0..255 | `"u8"` |
    /// | `int` 0..65535 | `"u16"` |
    /// | `int` 0..2³²-1 | `"u32"` |
    /// | any other int | `"u64"` |
    /// | `float`     | `"f64"` |
    /// | `str`       | `"str"` |
    /// | other       | `"any"` |
    pub fn iir_type_name(&self) -> &'static str {
        match self {
            Value::Bool(_)  => "bool",
            Value::Int(n) => {
                let n = *n;
                if (0..=255).contains(&n) {
                    "u8"
                } else if (0..=65535).contains(&n) {
                    "u16"
                } else if (0..=0xFFFF_FFFF).contains(&n) {
                    "u32"
                } else {
                    "u64"
                }
            }
            Value::Float(_) => "f64",
            Value::Str(_)   => "str",
            Value::Null     => "any",
        }
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::Null
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(n)   => write!(f, "{n}"),
            Value::Float(v) => write!(f, "{v}"),
            Value::Bool(b)  => write!(f, "{b}"),
            Value::Str(s)   => write!(f, "{s:?}"),
            Value::Null     => write!(f, "null"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_i64_int() {
        assert_eq!(Value::Int(42).as_i64(), Some(42));
        assert_eq!(Value::Bool(true).as_i64(), Some(1));
        assert!(Value::Float(1.0).as_i64().is_none());
    }

    #[test]
    fn is_truthy_cases() {
        assert!(Value::Int(1).is_truthy());
        assert!(!Value::Int(0).is_truthy());
        assert!(Value::Bool(true).is_truthy());
        assert!(!Value::Bool(false).is_truthy());
        assert!(!Value::Null.is_truthy());
        assert!(Value::Str("hi".into()).is_truthy());
        assert!(!Value::Str("".into()).is_truthy());
    }

    #[test]
    fn iir_type_name_ranges() {
        assert_eq!(Value::Int(0).iir_type_name(), "u8");
        assert_eq!(Value::Int(255).iir_type_name(), "u8");
        assert_eq!(Value::Int(256).iir_type_name(), "u16");
        assert_eq!(Value::Int(65536).iir_type_name(), "u32");
        assert_eq!(Value::Int(-1).iir_type_name(), "u64");
        assert_eq!(Value::Float(1.0).iir_type_name(), "f64");
        assert_eq!(Value::Bool(true).iir_type_name(), "bool");
        assert_eq!(Value::Str("hi".into()).iir_type_name(), "str");
        assert_eq!(Value::Null.iir_type_name(), "any");
    }
}
