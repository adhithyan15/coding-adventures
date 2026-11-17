//! # The types of the model IR
//!
//! The S2 checker publishes signatures but not the type of every sub-expression, and
//! three things in the lowering depend on expression types (the contextual type of a bare
//! decimal literal, the static widths of a `Bytes<N>` call, and whether an arithmetic node
//! wraps).  So this crate carries a small type of its own, [`Ty`], covering exactly what
//! `ir.core` can express (VER04r section 4.1):
//!
//! ```text
//!   Bool   Int   Nat   U8 U16 U32 U64 I8 I16 I32 I64
//!   Bytes  Bytes<4>  Bytes<N>           -- a literal width or a static width parameter
//!   Result<T, ErrorName>
//! ```
//!
//! `List`, `Option`, tuples, records and variants belong to `ir.data` and never reach a
//! [`Ty`]: the lowering refuses them first, naming the level.
//!
//! `Int` is the mathematical integer of VER04 section 3.  It exists only inside the IR
//! (the type of `Nat - Nat`, of a bare literal outside a `Nat` context); VER04r 2.1 allows
//! it in a `lit` node and nowhere else.
//!
//! [`Ty::name`] is the canonical spelling used in `ops`, binders and stream contexts
//! (`Result<Bytes,SelectError>` has **no space** after the comma, exactly as the v1 corpus
//! and the independent oracle write it).

use coding_adventures_warrant_grammar::fixed::FixedTy;

/// The width of a `Bytes<..>` type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Width {
    /// `Bytes<4>`.
    Lit(u64),
    /// `Bytes<N>`, where `N` is a static parameter of an operation (VER04i 2.5).
    Param(String),
}

/// A type of the model IR.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Ty {
    /// `Bool`.
    Bool,
    /// `Int`, the model-internal mathematical integer.
    Int,
    /// `Nat`, the natural numbers.
    Nat,
    /// One of the eight fixed-width integer types.
    Fixed(FixedTy),
    /// `Bytes` (no width) or `Bytes<N>` (a literal or named width).
    Bytes(Option<Width>),
    /// `Result<T, E>`: the payload type and the *name* of the declared error type.
    Result(Box<Ty>, String),
}

impl Ty {
    /// The unconstrained byte string type.
    pub const BYTES: Ty = Ty::Bytes(None);

    /// `Bytes<n>` for a literal width.
    pub fn bytes_n(n: u64) -> Ty {
        Ty::Bytes(Some(Width::Lit(n)))
    }

    /// The canonical spelling, e.g. `"U64"`, `"Bytes<N>"`, `"Result<Bytes,SelectError>"`.
    pub fn name(&self) -> String {
        match self {
            Ty::Bool => "Bool".into(),
            Ty::Int => "Int".into(),
            Ty::Nat => "Nat".into(),
            Ty::Fixed(t) => t.name().into(),
            Ty::Bytes(None) => "Bytes".into(),
            Ty::Bytes(Some(Width::Lit(n))) => format!("Bytes<{n}>"),
            Ty::Bytes(Some(Width::Param(p))) => format!("Bytes<{p}>"),
            Ty::Result(ok, err) => format!("Result<{},{err}>", ok.name()),
        }
    }

    /// The fixed-width type, if this is one.
    pub fn fixed(&self) -> Option<FixedTy> {
        match self {
            Ty::Fixed(t) => Some(*t),
            _ => None,
        }
    }

    /// Is this one of the eight fixed-width integer types?
    pub fn is_fixed(&self) -> bool {
        self.fixed().is_some()
    }

    /// `Int`, `Nat` or a fixed-width type: what `< + - *` accept.
    pub fn is_numeric(&self) -> bool {
        matches!(self, Ty::Int | Ty::Nat | Ty::Fixed(_))
    }

    /// Is this `Bytes` or `Bytes<..>`?
    pub fn is_bytes(&self) -> bool {
        matches!(self, Ty::Bytes(_))
    }

    /// The common type of two operand types, or `None` (VER04 section 3).
    ///
    /// ```text
    ///   Nat   with Int      -> Int          (Nat widens)
    ///   Bytes<N> with Bytes<M> / Bytes -> Bytes   (the width is forgotten)
    ///   Result<A,E> with Result<B,E>   -> Result<unify(A,B),E>
    ///   anything else must be equal; nothing else is implicit
    /// ```
    pub fn unify(a: &Ty, b: &Ty) -> Option<Ty> {
        if a == b {
            return Some(a.clone());
        }
        match (a, b) {
            (Ty::Nat, Ty::Int) | (Ty::Int, Ty::Nat) => Some(Ty::Int),
            (Ty::Bytes(_), Ty::Bytes(_)) => Some(Ty::BYTES),
            (Ty::Result(x, e1), Ty::Result(y, e2)) if e1 == e2 => {
                Ty::unify(x, y).map(|t| Ty::Result(Box::new(t), e1.clone()))
            }
            _ => None,
        }
    }

    /// Can a value of type `src` flow to a place of type `dst`?
    /// (`Nat` to `Int`; `Bytes<N>` to `Bytes` or to `Bytes<N>` of the same width.)
    pub fn assignable(src: &Ty, dst: &Ty) -> bool {
        if src == dst {
            return true;
        }
        match (src, dst) {
            (Ty::Nat, Ty::Int) => true,
            (Ty::Bytes(_), Ty::Bytes(None)) => true,
            (Ty::Result(x, e1), Ty::Result(y, e2)) if e1 == e2 => Ty::assignable(x, y),
            _ => false,
        }
    }

    /// Replace static width names by their values (`Bytes<N>` with `N = 4` is
    /// `Bytes<4>`).  Names with no value are kept.
    pub fn substitute(&self, statics: &[(String, u64)]) -> Ty {
        match self {
            Ty::Bytes(Some(Width::Param(p))) => statics
                .iter()
                .find(|(name, _)| name == p)
                .map(|(_, v)| Ty::bytes_n(*v))
                .unwrap_or_else(|| self.clone()),
            Ty::Result(ok, e) => Ty::Result(Box::new(ok.substitute(statics)), e.clone()),
            other => other.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result_bytes() -> Ty {
        Ty::Result(Box::new(Ty::BYTES), "SelectError".into())
    }

    #[test]
    fn names_are_the_canonical_spelling() {
        assert_eq!(Ty::Bool.name(), "Bool");
        assert_eq!(Ty::Int.name(), "Int");
        assert_eq!(Ty::Nat.name(), "Nat");
        assert_eq!(Ty::Fixed(FixedTy::U64).name(), "U64");
        assert_eq!(Ty::Fixed(FixedTy::I8).name(), "I8");
        assert_eq!(Ty::BYTES.name(), "Bytes");
        assert_eq!(Ty::bytes_n(4).name(), "Bytes<4>");
        assert_eq!(Ty::Bytes(Some(Width::Param("N".into()))).name(), "Bytes<N>");
        assert_eq!(result_bytes().name(), "Result<Bytes,SelectError>");
    }

    #[test]
    fn classification_helpers() {
        assert!(Ty::Fixed(FixedTy::U8).is_fixed() && Ty::Fixed(FixedTy::U8).is_numeric());
        assert_eq!(Ty::Fixed(FixedTy::I16).fixed(), Some(FixedTy::I16));
        assert_eq!(Ty::Nat.fixed(), None);
        assert!(Ty::Nat.is_numeric() && Ty::Int.is_numeric() && !Ty::Bool.is_numeric());
        assert!(Ty::bytes_n(2).is_bytes() && !Ty::Nat.is_bytes());
        assert!(!result_bytes().is_numeric());
    }

    #[test]
    fn unify_widens_nat_forgets_widths_and_nothing_else_is_implicit() {
        assert_eq!(Ty::unify(&Ty::Nat, &Ty::Int), Some(Ty::Int));
        assert_eq!(Ty::unify(&Ty::Int, &Ty::Nat), Some(Ty::Int));
        assert_eq!(Ty::unify(&Ty::bytes_n(2), &Ty::bytes_n(3)), Some(Ty::BYTES));
        assert_eq!(
            Ty::unify(&Ty::bytes_n(2), &Ty::bytes_n(2)),
            Some(Ty::bytes_n(2))
        );
        assert_eq!(Ty::unify(&Ty::BYTES, &Ty::bytes_n(2)), Some(Ty::BYTES));
        assert_eq!(
            Ty::unify(&Ty::Fixed(FixedTy::U8), &Ty::Fixed(FixedTy::U64)),
            None
        );
        assert_eq!(Ty::unify(&Ty::Fixed(FixedTy::U8), &Ty::Nat), None);
        assert_eq!(Ty::unify(&Ty::Bool, &Ty::Nat), None);
        let narrow = Ty::Result(Box::new(Ty::bytes_n(1)), "E".into());
        let wide = Ty::Result(Box::new(Ty::bytes_n(2)), "E".into());
        assert_eq!(
            Ty::unify(&narrow, &wide),
            Some(Ty::Result(Box::new(Ty::BYTES), "E".into()))
        );
        let other_err = Ty::Result(Box::new(Ty::bytes_n(1)), "F".into());
        assert_eq!(Ty::unify(&narrow, &other_err), None);
        let bad_inner = Ty::Result(Box::new(Ty::Bool), "E".into());
        assert_eq!(Ty::unify(&narrow, &bad_inner), None);
    }

    #[test]
    fn assignable_is_directional() {
        assert!(Ty::assignable(&Ty::Nat, &Ty::Int));
        assert!(!Ty::assignable(&Ty::Int, &Ty::Nat));
        assert!(Ty::assignable(&Ty::bytes_n(4), &Ty::BYTES));
        assert!(!Ty::assignable(&Ty::BYTES, &Ty::bytes_n(4)));
        assert!(!Ty::assignable(&Ty::bytes_n(4), &Ty::bytes_n(5)));
        assert!(Ty::assignable(&Ty::bytes_n(4), &Ty::bytes_n(4)));
        let narrow = Ty::Result(Box::new(Ty::bytes_n(1)), "E".into());
        assert!(Ty::assignable(
            &narrow,
            &Ty::Result(Box::new(Ty::BYTES), "E".into())
        ));
        assert!(!Ty::assignable(
            &narrow,
            &Ty::Result(Box::new(Ty::BYTES), "F".into())
        ));
        assert!(!Ty::assignable(&Ty::Bool, &Ty::Nat));
    }

    #[test]
    fn substitute_replaces_known_static_widths_only() {
        let n = Ty::Bytes(Some(Width::Param("N".into())));
        assert_eq!(n.substitute(&[("N".into(), 4)]), Ty::bytes_n(4));
        assert_eq!(n.substitute(&[("M".into(), 4)]), n);
        let r = Ty::Result(Box::new(n.clone()), "E".into());
        assert_eq!(
            r.substitute(&[("N".into(), 16)]),
            Ty::Result(Box::new(Ty::bytes_n(16)), "E".into())
        );
        assert_eq!(Ty::Nat.substitute(&[("N".into(), 1)]), Ty::Nat);
    }
}
