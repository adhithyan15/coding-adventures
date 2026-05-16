//! SIR type carriers.
//!
//! Per SIR10, the SIR carries optional type information but does
//! **not** infer or verify it.  A frontend either supplies a
//! `SirType` or leaves the slot `None`; the IR round-trips that
//! decision faithfully.  Backends may use the type for narrowing or
//! reject contradictions, but they may not synthesize missing types.
//!
//! The v0 type set is small — only what Twig needs.  Future
//! versions extend the enum (a versioned change to the SIR).

use std::fmt;

/// A SIR type — purely a carrier, never inferred by the IR itself.
///
/// Truth table for what currently lives in the enum:
///
/// | Variant   | Meaning                                              |
/// |-----------|------------------------------------------------------|
/// | `Any`     | top type; matches anything                            |
/// | `Int`     | 64-bit signed integer                                 |
/// | `Bool`    | boolean (true/false)                                  |
/// | `Nil`     | the singleton `nil` value                             |
/// | `Symbol`  | interned symbol (`'foo` in Twig)                      |
/// | `Str`     | string                                                |
/// | `Pair`    | cons cell (heap pair of two values)                   |
/// | `Closure` | any closure handle                                    |
/// | `Fn { params, ret }` | function type with typed params / return   |
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SirType {
    Any,
    Int,
    Bool,
    Nil,
    Symbol,
    Str,
    Pair,
    Closure,
    Fn { params: Vec<SirType>, ret: Box<SirType> },
}

impl SirType {
    /// Convenience constructor for `Fn`.
    pub fn function(params: Vec<SirType>, ret: SirType) -> Self {
        SirType::Fn { params, ret: Box::new(ret) }
    }

    /// `true` iff this is the top type `Any`.
    pub fn is_any(&self) -> bool {
        matches!(self, SirType::Any)
    }
}

impl fmt::Display for SirType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SirType::Any => write!(f, "any"),
            SirType::Int => write!(f, "int"),
            SirType::Bool => write!(f, "bool"),
            SirType::Nil => write!(f, "nil"),
            SirType::Symbol => write!(f, "symbol"),
            SirType::Str => write!(f, "str"),
            SirType::Pair => write!(f, "pair"),
            SirType::Closure => write!(f, "closure"),
            SirType::Fn { params, ret } => {
                write!(f, "(fn (")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, ") {})", ret)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_simple_types() {
        assert_eq!(format!("{}", SirType::Any), "any");
        assert_eq!(format!("{}", SirType::Int), "int");
        assert_eq!(format!("{}", SirType::Bool), "bool");
        assert_eq!(format!("{}", SirType::Nil), "nil");
        assert_eq!(format!("{}", SirType::Symbol), "symbol");
        assert_eq!(format!("{}", SirType::Str), "str");
        assert_eq!(format!("{}", SirType::Pair), "pair");
        assert_eq!(format!("{}", SirType::Closure), "closure");
    }

    #[test]
    fn display_fn_type() {
        let t = SirType::function(vec![SirType::Int, SirType::Int], SirType::Bool);
        assert_eq!(format!("{}", t), "(fn (int int) bool)");
    }

    #[test]
    fn display_fn_zero_args() {
        let t = SirType::function(vec![], SirType::Nil);
        assert_eq!(format!("{}", t), "(fn () nil)");
    }

    #[test]
    fn display_nested_fn() {
        // (fn (int) (fn () bool))
        let inner = SirType::function(vec![], SirType::Bool);
        let outer = SirType::function(vec![SirType::Int], inner);
        assert_eq!(format!("{}", outer), "(fn (int) (fn () bool))");
    }

    #[test]
    fn is_any_distinguishes() {
        assert!(SirType::Any.is_any());
        assert!(!SirType::Int.is_any());
    }

    #[test]
    fn equality_round_trip() {
        let a = SirType::function(vec![SirType::Int], SirType::Int);
        let b = SirType::function(vec![SirType::Int], SirType::Int);
        assert_eq!(a, b);
    }
}
