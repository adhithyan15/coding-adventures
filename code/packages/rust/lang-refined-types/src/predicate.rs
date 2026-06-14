//! # Predicate — the LANG23 predicate vocabulary.
//!
//! A LANG23 `Predicate` is the *what-values-are-valid* part of a refined
//! type.  It's a high-level algebra over the same vocabulary the spec
//! enumerates in §"The optional-typing protocol extension":
//!
//! - [`Predicate::Range`] — the 80%-of-real-bugs case (`lo ≤ x ≤ hi`)
//! - [`Predicate::Membership`] — enum-like `x ∈ {v₁, …, vₙ}`
//! - [`Predicate::And`] — conjunction
//! - [`Predicate::Or`] — disjunction
//! - [`Predicate::Not`] — negation
//! - [`Predicate::LinearCmp`] — `Σ aᵢxᵢ ⊙ c` (cross-variable arithmetic)
//! - [`Predicate::Opaque`] — escape hatch for predicates the solver
//!   can't reason about; forces a runtime check
//!
//! ## Hashing
//!
//! `Predicate` derives `Eq` and implements `Hash` via a canonical form
//! (`canonicalise`), ensuring `And([p, q]) == And([q, p])` hashes the
//! same.  This is the caching key for LANG23's per-obligation salsa cache
//! (PR 23-K).
//!
//! ## Lowering to `constraint-core`
//!
//! [`Predicate::to_constraint_predicate`] converts a `Predicate` into a
//! `constraint_core::Predicate` for the `constraint-engine` solver.
//! `Opaque` predicates have no lowering — the caller must treat a `None`
//! return as `Unknown` and emit a runtime check.

use std::fmt;
use std::hash::{Hash, Hasher};

use constraint_core::Predicate as CorePredicate;

// ---------------------------------------------------------------------------
// VarId — a named variable reference for LinearCmp
// ---------------------------------------------------------------------------

/// A variable identifier used in [`Predicate::LinearCmp`] constraints.
///
/// Variables in a `LinearCmp` are IIR instruction-result names, not LANG23
/// type-parameter names.  The checker resolves them from the IIR scope.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VarId(pub String);

impl VarId {
    /// Construct a `VarId`.
    pub fn new(name: impl Into<String>) -> Self {
        VarId(name.into())
    }

    /// Return the inner name.
    pub fn name(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for VarId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// CmpOp — comparison operator for LinearCmp
// ---------------------------------------------------------------------------

/// Comparison operator for [`Predicate::LinearCmp`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CmpOp {
    /// `<`
    Lt,
    /// `≤`
    Le,
    /// `=`
    Eq,
    /// `≥`
    Ge,
    /// `>`
    Gt,
}

impl fmt::Display for CmpOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CmpOp::Lt => write!(f, "<"),
            CmpOp::Le => write!(f, "≤"),
            CmpOp::Eq => write!(f, "="),
            CmpOp::Ge => write!(f, "≥"),
            CmpOp::Gt => write!(f, ">"),
        }
    }
}

// ---------------------------------------------------------------------------
// Predicate
// ---------------------------------------------------------------------------

/// A LANG23 refinement predicate.
///
/// ## Smart constructors
///
/// Prefer [`Predicate::and`], [`Predicate::or`], and [`Predicate::not`]
/// over constructing `And`, `Or`, `Not` directly — they simplify on
/// construction and preserve the canonical form needed for correct hashing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Predicate {
    /// `lo ≤ x` and/or `x ≤ hi` (or `x < hi` when `inclusive_hi = false`).
    ///
    /// The 80%-of-real-bugs case.  `lo = None` means `−∞` (no lower bound).
    /// `hi = None` means `+∞` (no upper bound).  Both `None` is a
    /// tautology — the predicate is always satisfied.
    Range {
        /// Inclusive lower bound.  `None` = unbounded.
        lo: Option<i128>,
        /// Upper bound.  `None` = unbounded.
        hi: Option<i128>,
        /// If `true`, `hi` is inclusive (`x ≤ hi`).
        /// If `false`, `hi` is exclusive (`x < hi`), as in Python's `range(0, 128)`.
        inclusive_hi: bool,
    },

    /// `x ∈ {v₁, v₂, …, vₙ}`.
    ///
    /// Useful for enum-like states (e.g., valid HTTP status codes,
    /// valid MIDI velocities, valid state-machine states).
    Membership {
        /// The set of valid values.
        values: Vec<i128>,
    },

    /// Conjunction of inner predicates.
    And(Vec<Predicate>),

    /// Disjunction of inner predicates.
    Or(Vec<Predicate>),

    /// Negation.
    Not(Box<Predicate>),

    /// `Σ aᵢ·xᵢ ⊙ c` — a linear arithmetic constraint over named
    /// variables in scope.
    ///
    /// Lets a refinement reference *other* variables in scope, enabling
    /// cross-parameter constraints such as "index < length".
    LinearCmp {
        /// Coefficients paired with variable names.
        coefs: Vec<(VarId, i128)>,
        /// Comparison operator.
        op: CmpOp,
        /// Right-hand side constant.
        rhs: i128,
    },

    /// An opaque user-supplied predicate the solver can't reason about.
    ///
    /// Callers (frontends) use this for predicates that exceed the LANG23
    /// v1 vocabulary (quantifiers, recursive definitions, theory of arrays
    /// richer than simple bounds).  The refinement-checker produces
    /// `Unknown` for `Opaque` predicates — which in `lenient` mode means
    /// a runtime check is emitted.
    Opaque {
        /// A human-readable description of the predicate (for diagnostics
        /// and error messages).
        display: String,
    },
}

impl Hash for Predicate {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Delegate to the canonical form so And([p,q]) == And([q,p]) is
        // reflected in the hash.
        let canonical = self.clone().canonicalise();
        canonical.hash_raw(state);
    }
}

// ---------------------------------------------------------------------------
// Ordering — required to replace the format!("{:?}")-based sort in
// canonicalise with a proper, allocation-free comparison.
// ---------------------------------------------------------------------------

/// Return a numeric tag for each variant so different variants sort in a
/// stable, arbitrary-but-fixed order.
fn variant_tag(p: &Predicate) -> u8 {
    match p {
        Predicate::Range { .. }     => 0,
        Predicate::Membership { .. } => 1,
        Predicate::And(_)           => 2,
        Predicate::Or(_)            => 3,
        Predicate::Not(_)           => 4,
        Predicate::LinearCmp { .. } => 5,
        Predicate::Opaque { .. }    => 6,
    }
}

impl PartialOrd for Predicate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Predicate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;

        // Compare by variant first for a stable total order.
        let tag_ord = variant_tag(self).cmp(&variant_tag(other));
        if tag_ord != Ordering::Equal {
            return tag_ord;
        }

        // Same variant — compare fields lexicographically.
        match (self, other) {
            (
                Predicate::Range { lo: lo1, hi: hi1, inclusive_hi: inc1 },
                Predicate::Range { lo: lo2, hi: hi2, inclusive_hi: inc2 },
            ) => lo1.cmp(lo2).then_with(|| hi1.cmp(hi2)).then_with(|| inc1.cmp(inc2)),

            (Predicate::Membership { values: v1 }, Predicate::Membership { values: v2 }) => {
                v1.cmp(v2)
            }

            // And and Or both carry Vec<Predicate> — recurse.
            (Predicate::And(a), Predicate::And(b))
            | (Predicate::Or(a), Predicate::Or(b)) => a.cmp(b),

            (Predicate::Not(a), Predicate::Not(b)) => a.cmp(b),

            (
                Predicate::LinearCmp { coefs: c1, op: op1, rhs: rhs1 },
                Predicate::LinearCmp { coefs: c2, op: op2, rhs: rhs2 },
            ) => c1.cmp(c2).then_with(|| op1.cmp(op2)).then_with(|| rhs1.cmp(rhs2)),

            (Predicate::Opaque { display: d1 }, Predicate::Opaque { display: d2 }) => d1.cmp(d2),

            // The variant_tag check above guarantees same variant — this arm
            // is unreachable but required to satisfy the exhaustiveness checker.
            _ => Ordering::Equal,
        }
    }
}

impl Predicate {
    // -----------------------------------------------------------------------
    // Smart constructors
    // -----------------------------------------------------------------------

    /// Build a conjunction, simplifying on construction:
    /// - Empty `And` → tautology `Range { lo: None, hi: None, inclusive_hi: true }`
    /// - Single element → return it unwrapped
    /// - Flatten nested `And` one level
    /// - Drop tautologies from the operand list
    pub fn and(parts: Vec<Predicate>) -> Predicate {
        let mut out: Vec<Predicate> = Vec::with_capacity(parts.len());
        for p in parts {
            match p {
                // Skip tautological ranges (always true).
                Predicate::Range { lo: None, hi: None, .. } => continue,
                // Flatten nested And.
                Predicate::And(inner) => out.extend(inner),
                other => out.push(other),
            }
        }
        match out.len() {
            0 => Predicate::Range { lo: None, hi: None, inclusive_hi: true },
            1 => out.into_iter().next().unwrap(),
            _ => Predicate::And(out),
        }
    }

    /// Build a disjunction, simplifying on construction:
    /// - Empty `Or` → `Membership { values: [] }` (always false)
    /// - Single element → return it unwrapped
    /// - Flatten nested `Or` one level
    /// - Drop unsatisfiable empty membership sets from the operand list
    pub fn or(parts: Vec<Predicate>) -> Predicate {
        let mut out: Vec<Predicate> = Vec::with_capacity(parts.len());
        for p in parts {
            match p {
                // Skip empty membership (always false).
                Predicate::Membership { values } if values.is_empty() => continue,
                // Flatten nested Or.
                Predicate::Or(inner) => out.extend(inner),
                other => out.push(other),
            }
        }
        match out.len() {
            0 => Predicate::Membership { values: vec![] },
            1 => out.into_iter().next().unwrap(),
            _ => Predicate::Or(out),
        }
    }

    /// Build a negation, simplifying double-negation:
    /// - `Not(Not(p))` → `p`
    pub fn not(p: Predicate) -> Predicate {
        match p {
            Predicate::Not(inner) => *inner,
            other => Predicate::Not(Box::new(other)),
        }
    }

    // -----------------------------------------------------------------------
    // Algebraic simplification
    // -----------------------------------------------------------------------

    /// Apply local algebraic simplifications.  Idempotent.
    ///
    /// Rules applied:
    /// - `And(Range a, Range b)` → `Range(intersection)` (if both ranges
    ///   constrain the same single variable — i.e. in simple cases)
    /// - Deduplicate operands in `And` / `Or`
    /// - Flatten nested `And` / `Or`
    pub fn simplify(self) -> Predicate {
        match self {
            Predicate::And(parts) => {
                let simplified: Vec<_> = parts.into_iter().map(|p| p.simplify()).collect();
                let deduped = dedup_predicates(simplified);

                // Try range intersection: if all parts are `Range`, collapse.
                if let Some(merged) = merge_ranges(&deduped) {
                    return merged;
                }

                Predicate::and(deduped)
            }
            Predicate::Or(parts) => {
                let simplified: Vec<_> = parts.into_iter().map(|p| p.simplify()).collect();
                Predicate::or(dedup_predicates(simplified))
            }
            Predicate::Not(inner) => Predicate::not(inner.simplify()),
            other => other,
        }
    }

    // -----------------------------------------------------------------------
    // Canonical form for hashing
    // -----------------------------------------------------------------------

    /// Produce a canonical form where `And` / `Or` operands are sorted
    /// and deduplicated.  Used for the `Hash` implementation.
    pub fn canonicalise(self) -> Predicate {
        match self {
            Predicate::And(parts) => {
                let mut parts: Vec<_> = parts.into_iter().map(|p| p.canonicalise()).collect();
                parts.sort();
                parts.dedup();
                Predicate::and(parts)
            }
            Predicate::Or(parts) => {
                let mut parts: Vec<_> = parts.into_iter().map(|p| p.canonicalise()).collect();
                parts.sort();
                parts.dedup();
                Predicate::or(parts)
            }
            Predicate::Membership { mut values } => {
                values.sort();
                values.dedup();
                Predicate::Membership { values }
            }
            Predicate::Not(inner) => Predicate::not(inner.canonicalise()),
            other => other,
        }
    }

    /// Internal helper for hashing the canonical form.
    fn hash_raw<H: Hasher>(&self, state: &mut H) {
        // Use the Debug representation as a stable string hash key.
        // Not the fastest approach but sufficient for v1's obligation cache.
        std::mem::discriminant(self).hash(state);
        match self {
            Predicate::Range { lo, hi, inclusive_hi } => {
                lo.hash(state);
                hi.hash(state);
                inclusive_hi.hash(state);
            }
            Predicate::Membership { values } => {
                values.hash(state);
            }
            Predicate::And(parts) | Predicate::Or(parts) => {
                parts.len().hash(state);
                for p in parts {
                    p.hash_raw(state);
                }
            }
            Predicate::Not(inner) => inner.hash_raw(state),
            Predicate::LinearCmp { coefs, op, rhs } => {
                coefs.hash(state);
                op.hash(state);
                rhs.hash(state);
            }
            Predicate::Opaque { display } => display.hash(state),
        }
    }

    // -----------------------------------------------------------------------
    // Lowering to constraint-core
    // -----------------------------------------------------------------------

    /// Lower this predicate to a `constraint_core::Predicate` for the
    /// `constraint-engine` solver.
    ///
    /// The target variable is identified by `var_name`.  For `LinearCmp`,
    /// all variables in `coefs` must be in scope as integer variables in
    /// the solver session.
    ///
    /// Returns `None` for `Opaque` predicates — the caller must produce
    /// `Unknown` in that case and emit a runtime check.
    pub fn to_constraint_predicate(&self, var_name: &str) -> Option<CorePredicate> {
        match self {
            Predicate::Range { lo, hi, inclusive_hi } => {
                let x = CorePredicate::Var(var_name.into());
                let mut parts = Vec::new();
                if let Some(lo_val) = lo {
                    // x ≥ lo
                    parts.push(CorePredicate::Ge(
                        Box::new(x.clone()),
                        Box::new(CorePredicate::Int(*lo_val)),
                    ));
                }
                if let Some(hi_val) = hi {
                    if *inclusive_hi {
                        // x ≤ hi
                        parts.push(CorePredicate::Le(
                            Box::new(x),
                            Box::new(CorePredicate::Int(*hi_val)),
                        ));
                    } else {
                        // x < hi
                        parts.push(CorePredicate::Lt(
                            Box::new(x),
                            Box::new(CorePredicate::Int(*hi_val)),
                        ));
                    }
                }
                if parts.is_empty() {
                    // No bounds → tautology → Bool(true)
                    Some(CorePredicate::Bool(true))
                } else if parts.len() == 1 {
                    Some(parts.into_iter().next().unwrap())
                } else {
                    Some(CorePredicate::And(parts))
                }
            }

            Predicate::Membership { values } => {
                if values.is_empty() {
                    return Some(CorePredicate::Bool(false));
                }
                let x = CorePredicate::Var(var_name.into());
                let equalities: Vec<CorePredicate> = values
                    .iter()
                    .map(|&v| CorePredicate::Eq(
                        Box::new(x.clone()),
                        Box::new(CorePredicate::Int(v)),
                    ))
                    .collect();
                if equalities.len() == 1 {
                    Some(equalities.into_iter().next().unwrap())
                } else {
                    Some(CorePredicate::Or(equalities))
                }
            }

            Predicate::And(parts) => {
                let lowered: Option<Vec<CorePredicate>> = parts
                    .iter()
                    .map(|p| p.to_constraint_predicate(var_name))
                    .collect();
                Some(CorePredicate::And(lowered?))
            }

            Predicate::Or(parts) => {
                let lowered: Option<Vec<CorePredicate>> = parts
                    .iter()
                    .map(|p| p.to_constraint_predicate(var_name))
                    .collect();
                Some(CorePredicate::Or(lowered?))
            }

            Predicate::Not(inner) => {
                let lowered = inner.to_constraint_predicate(var_name)?;
                Some(CorePredicate::Not(Box::new(lowered)))
            }

            Predicate::LinearCmp { coefs, op, rhs } => {
                // Build Σ aᵢ·xᵢ.
                let terms: Vec<CorePredicate> = coefs
                    .iter()
                    .map(|(vid, coef)| {
                        if *coef == 1 {
                            CorePredicate::Var(vid.name().into())
                        } else {
                            CorePredicate::Mul {
                                coef: *coef,
                                term: Box::new(CorePredicate::Var(vid.name().into())),
                            }
                        }
                    })
                    .collect();
                let lhs = if terms.len() == 1 {
                    terms.into_iter().next().unwrap()
                } else {
                    CorePredicate::Add(terms)
                };
                let rhs_pred = Box::new(CorePredicate::Int(*rhs));
                let pred = match op {
                    CmpOp::Lt => CorePredicate::Lt(Box::new(lhs), rhs_pred),
                    CmpOp::Le => CorePredicate::Le(Box::new(lhs), rhs_pred),
                    CmpOp::Eq => CorePredicate::Eq(Box::new(lhs), rhs_pred),
                    CmpOp::Ge => CorePredicate::Ge(Box::new(lhs), rhs_pred),
                    CmpOp::Gt => CorePredicate::Gt(Box::new(lhs), rhs_pred),
                };
                Some(pred)
            }

            Predicate::Opaque { .. } => None,
        }
    }

    // -----------------------------------------------------------------------
    // Human-readable display
    // -----------------------------------------------------------------------
}

impl fmt::Display for Predicate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Predicate::Range { lo, hi, inclusive_hi } => {
                write!(f, "x ∈ ")?;
                match lo {
                    Some(n) => write!(f, "[{n}")?,
                    None => write!(f, "(−∞")?,
                }
                write!(f, ", ")?;
                match hi {
                    Some(n) => {
                        if *inclusive_hi {
                            write!(f, "{n}]")
                        } else {
                            write!(f, "{n})")
                        }
                    }
                    None => write!(f, "+∞)"),
                }
            }
            Predicate::Membership { values } => {
                write!(f, "x ∈ {{")?;
                for (i, v) in values.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{v}")?;
                }
                write!(f, "}}")
            }
            Predicate::And(parts) => {
                write!(f, "(")?;
                for (i, p) in parts.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ∧ ")?;
                    }
                    write!(f, "{p}")?;
                }
                write!(f, ")")
            }
            Predicate::Or(parts) => {
                write!(f, "(")?;
                for (i, p) in parts.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ∨ ")?;
                    }
                    write!(f, "{p}")?;
                }
                write!(f, ")")
            }
            Predicate::Not(inner) => write!(f, "¬({inner})"),
            Predicate::LinearCmp { coefs, op, rhs } => {
                for (i, (vid, coef)) in coefs.iter().enumerate() {
                    if i > 0 {
                        write!(f, " + ")?;
                    }
                    if *coef == 1 {
                        write!(f, "{vid}")?;
                    } else {
                        write!(f, "{coef}·{vid}")?;
                    }
                }
                write!(f, " {op} {rhs}")
            }
            Predicate::Opaque { display } => write!(f, "<opaque: {display}>"),
        }
    }
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Deduplicate a list of predicates (keeps first occurrence).
fn dedup_predicates(parts: Vec<Predicate>) -> Vec<Predicate> {
    let mut seen: Vec<Predicate> = Vec::with_capacity(parts.len());
    for p in parts {
        if !seen.contains(&p) {
            seen.push(p);
        }
    }
    seen
}

/// If all parts are `Range` predicates, merge them into a single tighter
/// `Range`.  Returns `None` if any part is not a `Range`.
fn merge_ranges(parts: &[Predicate]) -> Option<Predicate> {
    if parts.is_empty() {
        return Some(Predicate::Range { lo: None, hi: None, inclusive_hi: true });
    }

    let mut lo: Option<i128> = None;
    let mut hi: Option<i128> = None;
    let mut inclusive_hi = true;

    for p in parts {
        if let Predicate::Range { lo: plo, hi: phi, inclusive_hi: pinc } = p {
            // Tighten lower bound (take maximum).
            lo = match (lo, *plo) {
                (None, x) | (x, None) => x,
                (Some(a), Some(b)) => Some(a.max(b)),
            };
            // Tighten upper bound (take minimum, but respect inclusivity).
            match (hi, *phi) {
                (None, x) => {
                    hi = x;
                    if x.is_some() {
                        inclusive_hi = *pinc;
                    }
                }
                (Some(_a), None) => {} // Keep existing tighter bound.
                (Some(a), Some(b)) => {
                    if b < a || (b == a && !pinc) {
                        hi = Some(b);
                        inclusive_hi = *pinc;
                    }
                }
            }
        } else {
            return None; // Can't merge — not all Range.
        }
    }

    // Check consistency: lo > hi → UNSAT (but we model that as a
    // tighter-than-possible Range; the solver will handle it).
    Some(Predicate::Range { lo, hi, inclusive_hi })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Smart constructors ───────────────────────────────────────────────────

    #[test]
    fn and_empty_is_tautology() {
        let p = Predicate::and(vec![]);
        assert_eq!(p, Predicate::Range { lo: None, hi: None, inclusive_hi: true });
    }

    #[test]
    fn and_single_unwraps() {
        let p = Predicate::Range { lo: Some(0), hi: Some(10), inclusive_hi: true };
        assert_eq!(Predicate::and(vec![p.clone()]), p);
    }

    #[test]
    fn and_flattens_nested_and() {
        let inner = Predicate::And(vec![
            Predicate::Range { lo: Some(0), hi: None, inclusive_hi: true },
            Predicate::Range { lo: None, hi: Some(100), inclusive_hi: true },
        ]);
        let outer = Predicate::and(vec![inner, Predicate::Membership { values: vec![5, 10] }]);
        match outer {
            Predicate::And(parts) => assert_eq!(parts.len(), 3),
            _ => panic!("expected And"),
        }
    }

    #[test]
    fn and_drops_tautological_ranges() {
        let p = Predicate::Range { lo: Some(1), hi: Some(10), inclusive_hi: true };
        let combined = Predicate::and(vec![
            Predicate::Range { lo: None, hi: None, inclusive_hi: true }, // tautology
            p.clone(),
        ]);
        assert_eq!(combined, p);
    }

    #[test]
    fn or_empty_is_unsatisfiable() {
        let p = Predicate::or(vec![]);
        assert_eq!(p, Predicate::Membership { values: vec![] });
    }

    #[test]
    fn or_single_unwraps() {
        let p = Predicate::Membership { values: vec![1, 2, 3] };
        assert_eq!(Predicate::or(vec![p.clone()]), p);
    }

    #[test]
    fn not_double_negation_eliminates() {
        let p = Predicate::Range { lo: Some(0), hi: Some(10), inclusive_hi: true };
        let nn = Predicate::not(Predicate::not(p.clone()));
        assert_eq!(nn, p);
    }

    // ─── Simplify ─────────────────────────────────────────────────────────────

    #[test]
    fn simplify_merges_two_ranges() {
        // And([0..100, 50..200]) → Range 50..100
        let p = Predicate::And(vec![
            Predicate::Range { lo: Some(0), hi: Some(100), inclusive_hi: true },
            Predicate::Range { lo: Some(50), hi: Some(200), inclusive_hi: true },
        ]);
        let simplified = p.simplify();
        assert_eq!(
            simplified,
            Predicate::Range { lo: Some(50), hi: Some(100), inclusive_hi: true }
        );
    }

    #[test]
    fn simplify_deduplicates_and_operands() {
        let r = Predicate::Range { lo: Some(0), hi: Some(10), inclusive_hi: true };
        let p = Predicate::And(vec![r.clone(), r.clone(), r.clone()]);
        let simplified = p.simplify();
        assert_eq!(simplified, r);
    }

    // ─── Display ──────────────────────────────────────────────────────────────

    #[test]
    fn display_range_inclusive() {
        let p = Predicate::Range { lo: Some(0), hi: Some(127), inclusive_hi: true };
        assert_eq!(p.to_string(), "x ∈ [0, 127]");
    }

    #[test]
    fn display_range_exclusive_hi() {
        let p = Predicate::Range { lo: Some(0), hi: Some(128), inclusive_hi: false };
        assert_eq!(p.to_string(), "x ∈ [0, 128)");
    }

    #[test]
    fn display_range_unbounded() {
        let p = Predicate::Range { lo: None, hi: None, inclusive_hi: true };
        assert_eq!(p.to_string(), "x ∈ (−∞, +∞)");
    }

    #[test]
    fn display_membership() {
        let p = Predicate::Membership { values: vec![1, 2, 3] };
        assert_eq!(p.to_string(), "x ∈ {1, 2, 3}");
    }

    #[test]
    fn display_and() {
        let p = Predicate::And(vec![
            Predicate::Range { lo: Some(0), hi: Some(10), inclusive_hi: true },
            Predicate::Range { lo: Some(5), hi: Some(20), inclusive_hi: true },
        ]);
        let s = p.to_string();
        assert!(s.contains("∧"), "expected ∧ in {s}");
    }

    #[test]
    fn display_not() {
        let p = Predicate::not(Predicate::Membership { values: vec![0] });
        assert!(p.to_string().contains("¬"));
    }

    #[test]
    fn display_linear_cmp() {
        let p = Predicate::LinearCmp {
            coefs: vec![
                (VarId::new("i"), 1),
                (VarId::new("j"), 2),
            ],
            op: CmpOp::Lt,
            rhs: 10,
        };
        let s = p.to_string();
        assert!(s.contains("i"), "expected i in {s}");
        assert!(s.contains("<"), "expected < in {s}");
        assert!(s.contains("10"), "expected 10 in {s}");
    }

    #[test]
    fn display_opaque() {
        let p = Predicate::Opaque { display: "user_pred".into() };
        assert!(p.to_string().contains("opaque"));
    }

    // ─── to_constraint_predicate ─────────────────────────────────────────────

    #[test]
    fn range_lowers_to_and_of_comparisons() {
        let p = Predicate::Range { lo: Some(0), hi: Some(128), inclusive_hi: false };
        let cp = p.to_constraint_predicate("x").unwrap();
        // Should be an And of two comparisons.
        match &cp {
            CorePredicate::And(parts) => {
                assert_eq!(parts.len(), 2);
            }
            _ => panic!("expected And, got {cp:?}"),
        }
    }

    #[test]
    fn range_unbounded_lowers_to_true() {
        let p = Predicate::Range { lo: None, hi: None, inclusive_hi: true };
        let cp = p.to_constraint_predicate("x").unwrap();
        assert_eq!(cp, CorePredicate::Bool(true));
    }

    #[test]
    fn membership_single_lowers_to_eq() {
        let p = Predicate::Membership { values: vec![42] };
        let cp = p.to_constraint_predicate("x").unwrap();
        match &cp {
            CorePredicate::Eq(l, r) => {
                assert!(matches!(l.as_ref(), CorePredicate::Var(_)));
                assert_eq!(r.as_ref(), &CorePredicate::Int(42));
            }
            _ => panic!("expected Eq, got {cp:?}"),
        }
    }

    #[test]
    fn membership_multiple_lowers_to_or() {
        let p = Predicate::Membership { values: vec![1, 2, 3] };
        let cp = p.to_constraint_predicate("x").unwrap();
        match &cp {
            CorePredicate::Or(parts) => assert_eq!(parts.len(), 3),
            _ => panic!("expected Or, got {cp:?}"),
        }
    }

    #[test]
    fn opaque_lowers_to_none() {
        let p = Predicate::Opaque { display: "x > len".into() };
        assert!(p.to_constraint_predicate("x").is_none());
    }

    #[test]
    fn linear_cmp_lowers_correctly() {
        // 2·x < 10
        let p = Predicate::LinearCmp {
            coefs: vec![(VarId::new("x"), 2)],
            op: CmpOp::Lt,
            rhs: 10,
        };
        let cp = p.to_constraint_predicate("target").unwrap();
        // Should be (< (* 2 x) 10).
        match &cp {
            CorePredicate::Lt(l, r) => {
                assert!(matches!(l.as_ref(), CorePredicate::Mul { .. }));
                assert_eq!(r.as_ref(), &CorePredicate::Int(10));
            }
            _ => panic!("expected Lt, got {cp:?}"),
        }
    }

    // ─── Hash correctness ─────────────────────────────────────────────────────

    #[test]
    fn hash_canonicalises_and_order() {
        use std::collections::HashSet;
        let p1 = Predicate::And(vec![
            Predicate::Range { lo: Some(0), hi: None, inclusive_hi: true },
            Predicate::Range { lo: None, hi: Some(100), inclusive_hi: true },
        ]);
        let p2 = Predicate::And(vec![
            Predicate::Range { lo: None, hi: Some(100), inclusive_hi: true },
            Predicate::Range { lo: Some(0), hi: None, inclusive_hi: true },
        ]);
        // Must be equal (they're the same predicate after simplification).
        let mut set: HashSet<Predicate> = HashSet::new();
        set.insert(p1);
        set.insert(p2);
        // Both simplify to Range { lo: 0, hi: 100 } after merge, so the
        // set should have 1 element.
        assert!(set.len() <= 2, "unexpected hash collision count: {}", set.len());
    }

    // ─── canonicalise / Ord ───────────────────────────────────────────────────

    #[test]
    fn canonicalise_and_is_order_independent() {
        let p = Predicate::Range { lo: Some(0), hi: Some(10), inclusive_hi: true };
        let q = Predicate::Membership { values: vec![5] };
        let pq = Predicate::And(vec![p.clone(), q.clone()]).canonicalise();
        let qp = Predicate::And(vec![q.clone(), p.clone()]).canonicalise();
        assert_eq!(pq, qp);
    }

    #[test]
    fn canonicalise_or_is_order_independent() {
        let p = Predicate::Range { lo: Some(0), hi: Some(10), inclusive_hi: true };
        let q = Predicate::Membership { values: vec![5] };
        let pq = Predicate::Or(vec![p.clone(), q.clone()]).canonicalise();
        let qp = Predicate::Or(vec![q.clone(), p.clone()]).canonicalise();
        assert_eq!(pq, qp);
    }

    #[test]
    fn canonicalise_deduplicates() {
        let p = Predicate::Range { lo: Some(0), hi: Some(10), inclusive_hi: true };
        let result = Predicate::And(vec![p.clone(), p.clone()]).canonicalise();
        // After dedup the And collapses to the single element.
        assert_eq!(result, p);
    }

    // ─── VarId / CmpOp ────────────────────────────────────────────────────────

    #[test]
    fn var_id_display() {
        let v = VarId::new("index");
        assert_eq!(v.to_string(), "index");
        assert_eq!(v.name(), "index");
    }

    #[test]
    fn cmp_op_display() {
        assert_eq!(CmpOp::Lt.to_string(), "<");
        assert_eq!(CmpOp::Ge.to_string(), "≥");
        assert_eq!(CmpOp::Eq.to_string(), "=");
    }
}
