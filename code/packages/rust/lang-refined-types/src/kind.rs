//! # Kind — the base type for a refined type.
//!
//! A `Kind` is what LANG22 already calls a "type hint": the coarse-grained
//! classification of a value before any predicate is applied.  LANG23 makes
//! the existing string type-hints explicit by enumerating the kinds the
//! type-checker and refinement-solver can reason about.
//!
//! ## Mapping from LANG22 type-hint strings
//!
//! | LANG22 string | `Kind` |
//! |---|---|
//! | `"i8"` | `Kind::I8` |
//! | `"i16"` | `Kind::I16` |
//! | `"i32"` | `Kind::I32` |
//! | `"i64"` | `Kind::Int` |
//! | `"u8"` | `Kind::U8` |
//! | `"u16"` | `Kind::U16` |
//! | `"u32"` | `Kind::U32` |
//! | `"u64"` | `Kind::U64` |
//! | `"f32"` | `Kind::F32` |
//! | `"f64"` | `Kind::Float` |
//! | `"bool"` | `Kind::Bool` |
//! | `"nil"` | `Kind::Nil` |
//! | `"str"` | `Kind::Str` |
//! | `"any"` | `Kind::Any` |
//! | anything else | `Kind::ClassId(name)` |
//!
//! The solver cares about `Int` (and `I8` / `I16` / `I32` / `U*` as
//! bounded integers) and `Bool`.  Float refinements are deferred to PR 23-N
//! (v1 is integer-only; float predicates produce `Opaque` → runtime check).

use std::fmt;

/// The base type of a refined variable.
///
/// Refinement predicates over an `Int` or bounded integer kind are
/// discharged by the LIA tactic in `constraint-engine`.  Predicates over
/// `Bool` are discharged by the SAT tactic.  Predicates over all other
/// kinds are `Opaque` in v1 (degrade to a runtime check).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Kind {
    /// 64-bit signed integer (the canonical integer kind for LANG22/23).
    Int,
    /// 8-bit signed integer (range −128 to 127).
    I8,
    /// 16-bit signed integer.
    I16,
    /// 32-bit signed integer.
    I32,
    /// 8-bit unsigned integer (range 0 to 255).
    U8,
    /// 16-bit unsigned integer.
    U16,
    /// 32-bit unsigned integer.
    U32,
    /// 64-bit unsigned integer.
    U64,
    /// 32-bit IEEE 754 float.  Float refinements degrade to `Opaque` in v1.
    F32,
    /// 64-bit IEEE 754 float.  Float refinements degrade to `Opaque` in v1.
    Float,
    /// Boolean (the canonical boolean kind).
    Bool,
    /// Nil / void (no value; no meaningful predicate).
    Nil,
    /// String (refinements degrade to `Opaque` in v1).
    Str,
    /// Any language value (gradual typing's top type).  No useful predicate.
    Any,
    /// A user-defined class or struct identified by its LANG22 class-id.
    /// Refinements degrade to `Opaque` in v1.
    ClassId(String),
}

impl Kind {
    /// Parse a LANG22 type-hint string into a `Kind`.
    ///
    /// Unknown strings become `Kind::ClassId(s)`.
    pub fn from_type_hint(s: &str) -> Self {
        match s {
            "i64" => Kind::Int,
            "i8" => Kind::I8,
            "i16" => Kind::I16,
            "i32" => Kind::I32,
            "u8" => Kind::U8,
            "u16" => Kind::U16,
            "u32" => Kind::U32,
            "u64" => Kind::U64,
            "f32" => Kind::F32,
            "f64" => Kind::Float,
            "bool" => Kind::Bool,
            "nil" | "null" | "void" => Kind::Nil,
            "str" | "string" => Kind::Str,
            "any" => Kind::Any,
            other => Kind::ClassId(other.to_owned()),
        }
    }

    /// Return the canonical LANG22 type-hint string for this kind.
    ///
    /// This is the inverse of [`Kind::from_type_hint`] for the built-in
    /// kinds.  `ClassId` returns the inner string directly.
    pub fn as_type_hint(&self) -> &str {
        match self {
            Kind::Int => "i64",
            Kind::I8 => "i8",
            Kind::I16 => "i16",
            Kind::I32 => "i32",
            Kind::U8 => "u8",
            Kind::U16 => "u16",
            Kind::U32 => "u32",
            Kind::U64 => "u64",
            Kind::F32 => "f32",
            Kind::Float => "f64",
            Kind::Bool => "bool",
            Kind::Nil => "nil",
            Kind::Str => "str",
            Kind::Any => "any",
            Kind::ClassId(s) => s.as_str(),
        }
    }

    /// Return `true` if the refinement solver can reason about this kind
    /// in v1.  Only integer kinds and `Bool` are supported.
    pub fn is_solver_supported(&self) -> bool {
        matches!(
            self,
            Kind::Int
                | Kind::I8
                | Kind::I16
                | Kind::I32
                | Kind::U8
                | Kind::U16
                | Kind::U32
                | Kind::U64
                | Kind::Bool
        )
    }

    /// Return the integer range `(min, max)` that defines the valid
    /// domain of this kind, if it is a bounded integer kind.
    ///
    /// Returns `None` for `Kind::Int` (unbounded), `Kind::Bool`, and all
    /// non-integer kinds.
    pub fn integer_bounds(&self) -> Option<(i128, i128)> {
        match self {
            Kind::I8 => Some((i8::MIN as i128, i8::MAX as i128)),
            Kind::I16 => Some((i16::MIN as i128, i16::MAX as i128)),
            Kind::I32 => Some((i32::MIN as i128, i32::MAX as i128)),
            Kind::U8 => Some((0, u8::MAX as i128)),
            Kind::U16 => Some((0, u16::MAX as i128)),
            Kind::U32 => Some((0, u32::MAX as i128)),
            Kind::U64 => Some((0, u64::MAX as i128)),
            _ => None,
        }
    }

    /// Return `true` if this is any integer kind (signed, unsigned, or
    /// `Kind::Int`).
    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            Kind::Int
                | Kind::I8
                | Kind::I16
                | Kind::I32
                | Kind::U8
                | Kind::U16
                | Kind::U32
                | Kind::U64
        )
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_type_hint())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_type_hint_round_trips() {
        for s in &["i64", "i8", "i16", "i32", "u8", "u16", "u32", "u64",
                   "f32", "f64", "bool", "nil", "str", "any"] {
            let k = Kind::from_type_hint(s);
            assert_eq!(k.as_type_hint(), *s, "round-trip failed for {s}");
        }
    }

    #[test]
    fn unknown_type_hint_becomes_class_id() {
        let k = Kind::from_type_hint("MyClass");
        assert_eq!(k, Kind::ClassId("MyClass".into()));
        assert_eq!(k.as_type_hint(), "MyClass");
    }

    #[test]
    fn is_solver_supported() {
        assert!(Kind::Int.is_solver_supported());
        assert!(Kind::I8.is_solver_supported());
        assert!(Kind::Bool.is_solver_supported());
        assert!(!Kind::Float.is_solver_supported());
        assert!(!Kind::Str.is_solver_supported());
        assert!(!Kind::Any.is_solver_supported());
    }

    #[test]
    fn integer_bounds_for_bounded_kinds() {
        assert_eq!(Kind::I8.integer_bounds(), Some((-128, 127)));
        assert_eq!(Kind::U8.integer_bounds(), Some((0, 255)));
        assert_eq!(Kind::U64.integer_bounds(), Some((0, u64::MAX as i128)));
        // Unbounded int has no bounds.
        assert_eq!(Kind::Int.integer_bounds(), None);
        // Bool is not a bounded integer.
        assert_eq!(Kind::Bool.integer_bounds(), None);
    }

    #[test]
    fn is_integer() {
        assert!(Kind::Int.is_integer());
        assert!(Kind::U8.is_integer());
        assert!(!Kind::Bool.is_integer());
        assert!(!Kind::Float.is_integer());
    }

    #[test]
    fn display() {
        assert_eq!(Kind::Int.to_string(), "i64");
        assert_eq!(Kind::Bool.to_string(), "bool");
        assert_eq!(Kind::ClassId("Foo".into()).to_string(), "Foo");
    }

    #[test]
    fn kind_eq_and_hash_works_in_hashset() {
        use std::collections::HashSet;
        let mut s: HashSet<Kind> = HashSet::new();
        s.insert(Kind::Int);
        s.insert(Kind::Bool);
        s.insert(Kind::Int); // duplicate
        assert_eq!(s.len(), 2);
    }
}
