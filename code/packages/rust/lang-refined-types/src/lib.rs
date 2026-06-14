//! # `lang-refined-types` — LANG23 refinement type data model.
//!
//! **LANG23 PR 23-A**.  Pure data + algebra — no solver, no IIR dependency.
//! Everything the refinement-checker and frontend lowering need to *represent*
//! a refined type lives here.
//!
//! ## What is a refinement type?
//!
//! A refinement type extends a *base kind* with a *predicate* that restricts
//! which values are valid.  Examples from the spec:
//!
//! ```text
//! (Int 0 128)        → kind = Int, predicate = Range { lo: 0, hi: 128, inclusive_hi: false }
//! (Member int 1 2 3) → kind = Int, predicate = Membership { values: [1, 2, 3] }
//! (Int 0 _)          → kind = Int, predicate = None  (any non-negative integer)
//! ```
//!
//! When `predicate` is `None` the type is *unrefined* — exactly the same
//! semantics as LANG22 Tier A/B/C types.  The two representations are
//! unified in [`RefinedType`] so the compiler can treat both uniformly.
//!
//! ## Predicate vocabulary
//!
//! [`Predicate`] carries five variants:
//!
//! | Variant | Meaning | LIA decidable? |
//! |---------|---------|---------------|
//! | [`Predicate::Range`] | `lo ≤ x` and/or `x ≤ hi` | Yes |
//! | [`Predicate::Membership`] | `x ∈ {v₁, v₂, …}` | Yes |
//! | [`Predicate::And`] | conjunction | Yes |
//! | [`Predicate::Or`] | disjunction | Yes |
//! | [`Predicate::Not`] | negation | Yes |
//! | [`Predicate::LinearCmp`] | `Σ aᵢxᵢ ⊙ c` | Yes |
//! | [`Predicate::Opaque`] | user-supplied, solver can't reason about | No → runtime check |
//!
//! ## Algebra
//!
//! Smart constructors ([`Predicate::and`], [`Predicate::or`],
//! [`Predicate::not`]) simplify on construction:
//!
//! - Empty `And` → `Range { lo: None, hi: None }` (any value — always true)
//! - `And([p])` → `p`
//! - `Not(Not(p))` → `p`
//! - `And` with a `Range` that already covers everything → drop it
//! - `And`/`Or` of nested `And`/`Or` → flatten one level
//!
//! [`Predicate::simplify`] performs further algebraic reductions:
//! - Range intersection for `And(Range, Range)` (tightest bounds win)
//! - Duplicate elimination
//!
//! ## Lowering to `constraint-core`
//!
//! [`Predicate::to_constraint_predicate`] converts a LANG23 `Predicate`
//! into a `constraint_core::Predicate` so `constraint-engine` can
//! discharge proof obligations.  The mapping is:
//!
//! | LANG23 Predicate | constraint-core Predicate |
//! |------------------|--------------------------|
//! | `Range { lo, hi, inclusive_hi }` | `Ge(x, lo)` ∧ `Le/Lt(x, hi)` |
//! | `Membership { values }` | `Or(Eq(x, v₁), Eq(x, v₂), …)` |
//! | `And(preds)` | `And(lowered_preds)` |
//! | `Or(preds)` | `Or(lowered_preds)` |
//! | `Not(pred)` | `Not(lowered_pred)` |
//! | `LinearCmp { coefs, op, rhs }` | `op(Σ coef·x, rhs)` |
//! | `Opaque { .. }` | *(returns None — solver gives Unknown)* |

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use std::fmt;

pub mod kind;
pub mod predicate;

pub use kind::Kind;
pub use predicate::{CmpOp, Predicate, VarId};

// ---------------------------------------------------------------------------
// RefinedType — the unified type representation
// ---------------------------------------------------------------------------

/// A LANG23 refined type: a base [`Kind`] plus an optional [`Predicate`].
///
/// When `predicate` is `None`, the type is *unrefined* and behaves exactly
/// as the corresponding LANG22 Tier A type hint string.  The type-checker
/// treats both the same: `RefinedType { kind: Kind::Int, predicate: None }`
/// carries the same meaning as the LANG22 type hint `"i64"`.
///
/// When `predicate` is `Some(p)`, the refinement checker runs the solver on
/// proof obligations of the form `∀x: kind. (call-site constraint) ⇒ p(x)`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RefinedType {
    /// The base kind (i64, f64, bool, class-id, "any").
    pub kind: Kind,
    /// Optional predicate restricting the value set within `kind`.
    /// `None` means "any value of `kind`" — unchanged from LANG22.
    pub predicate: Option<Predicate>,
}

impl RefinedType {
    /// Construct an unrefined type with the given kind.
    pub fn unrefined(kind: Kind) -> Self {
        RefinedType { kind, predicate: None }
    }

    /// Construct a refined type.
    pub fn refined(kind: Kind, predicate: Predicate) -> Self {
        RefinedType { kind, predicate: Some(predicate) }
    }

    /// Return `true` if this type carries no refinement predicate.
    pub fn is_unrefined(&self) -> bool {
        self.predicate.is_none()
    }

    /// Return `true` if this type carries a refinement predicate.
    pub fn is_refined(&self) -> bool {
        self.predicate.is_some()
    }

    /// A human-readable summary of the type.
    ///
    /// ```text
    /// i64               (unrefined)
    /// i64 where (x ∈ [0, 128))  (refined)
    /// ```
    pub fn display_str(&self) -> String {
        match &self.predicate {
            None => self.kind.to_string(),
            Some(p) => format!("{} where {p}", self.kind),
        }
    }
}

impl fmt::Display for RefinedType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_str())
    }
}

// ---------------------------------------------------------------------------
// Tests for RefinedType
// ---------------------------------------------------------------------------

#[cfg(test)]
mod refined_type_tests {
    use super::*;

    #[test]
    fn unrefined_type_has_no_predicate() {
        let t = RefinedType::unrefined(Kind::Int);
        assert!(t.is_unrefined());
        assert!(!t.is_refined());
        assert_eq!(t.to_string(), "i64");
    }

    #[test]
    fn refined_type_has_predicate() {
        let p = Predicate::Range {
            lo: Some(0),
            hi: Some(128),
            inclusive_hi: false,
        };
        let t = RefinedType::refined(Kind::Int, p);
        assert!(t.is_refined());
        assert!(!t.is_unrefined());
        assert!(t.to_string().contains("i64"));
        assert!(t.to_string().contains("where"));
    }

    #[test]
    fn display_unrefined_int() {
        let t = RefinedType::unrefined(Kind::Int);
        assert_eq!(format!("{t}"), "i64");
    }

    #[test]
    fn display_refined_bool() {
        let t = RefinedType::refined(Kind::Bool, Predicate::Range { lo: None, hi: None, inclusive_hi: true });
        assert!(t.to_string().contains("bool"));
    }

    #[test]
    fn eq_and_hash() {
        use std::collections::HashMap;
        let t1 = RefinedType::unrefined(Kind::Int);
        let t2 = RefinedType::unrefined(Kind::Int);
        assert_eq!(t1, t2);
        let mut map = HashMap::new();
        map.insert(t1.clone(), 1);
        assert_eq!(map[&t2], 1);
    }
}
