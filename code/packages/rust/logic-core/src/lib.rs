//! # logic-core — the semantic core of a logic VM.
//!
//! This crate is the data layer of a logic programming engine: terms,
//! variables, substitutions, and first-order unification.
//!
//! It is a Rust port of the Python `logic-core` package, which in turn
//! implements the language-agnostic specification in
//! [`code/specs/LP00-logic-core.md`](../../../specs/LP00-logic-core.md). The
//! Python version remains the canonical reference; this crate mirrors its
//! shape as closely as Rust's type system permits.
//!
//! ## The Term Universe
//!
//! Logic programming reasons over *terms*. A term is one of:
//!
//! - an **atom**, a zero-arity symbolic constant such as `homer` or `[]`;
//! - a **number**, integer or floating-point;
//! - a **string**, distinct from an atom because quoted Prolog literals
//!   carry case and whitespace;
//! - a **logic variable**, a placeholder that can be bound during search;
//! - a **compound** term, a functor applied to a list of argument terms,
//!   e.g. `father(homer, bart)`.
//!
//! Lists are represented in two equivalent ways: an ergonomic
//! [`logic_list`] constructor for human use, and the canonical Prolog
//! `'.'/2` / `[]` cons-cell encoding internally.
//!
//! ## Substitutions and Unification
//!
//! A [`Substitution`] is a partial map from variable id to term — the
//! result of binding variables during a query. **Unification** asks: given
//! two terms, is there a substitution that makes them syntactically equal?
//! [`unify`] is its implementation, with the **occurs-check** enabled
//! (i.e., `X = f(X)` fails rather than producing a cyclic term).
//!
//! Substitutions are persistent in spirit (cloning is cheap because the
//! underlying map is small for typical Prolog programs). Extensions are
//! returned as new values rather than mutating in place, which keeps
//! backtracking — to be added in a later crate — a matter of dropping
//! references rather than undoing writes.

use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

// ---------------------------------------------------------------------------
// Numbers — int and float kept distinct so equality is exact for integers
// ---------------------------------------------------------------------------

/// A numeric logic term.
///
/// Integers and floats are kept as separate variants so that equality on
/// integers is exact and so that downstream arithmetic can dispatch on
/// type without re-parsing. Mixing across variants in unification follows
/// Prolog tradition: `1 = 1.0` does **not** unify, because they are
/// distinct ground terms.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Number {
    Int(i64),
    Float(f64),
}

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Number::Int(i) => write!(f, "{}", i),
            Number::Float(x) => write!(f, "{}", x),
        }
    }
}

// ---------------------------------------------------------------------------
// Logic variables — identity is the numeric id, the display name is cosmetic
// ---------------------------------------------------------------------------

static NEXT_VAR_ID: AtomicU64 = AtomicU64::new(0);

/// A bindable variable whose identity is its numeric id.
///
/// Two variables with the same display name are still different variables
/// unless they share the same id. This is deliberate: it means that
/// renaming a variable for readability never affects program semantics, and
/// that fresh variables introduced during clause renaming (the "freshening"
/// step in SLD resolution, in a later crate) never collide with existing
/// ones.
#[derive(Debug, Clone)]
pub struct LogicVar {
    pub id: u64,
    pub display_name: Option<String>,
}

impl LogicVar {
    /// Allocate a brand-new variable with a fresh id.
    ///
    /// The id counter is process-wide. Tests that depend on specific id
    /// values are fragile by construction; they should compare bindings
    /// through [`Substitution::walk`] or [`Substitution::walk_var`]
    /// instead of inspecting ids directly.
    pub fn fresh(display_name: Option<&str>) -> Self {
        Self {
            id: NEXT_VAR_ID.fetch_add(1, Ordering::Relaxed),
            display_name: display_name.map(|s| s.to_string()),
        }
    }
}

impl PartialEq for LogicVar {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for LogicVar {}

impl std::hash::Hash for LogicVar {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl fmt::Display for LogicVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.display_name {
            Some(name) => write!(f, "{}", name),
            None => write!(f, "_G{}", self.id),
        }
    }
}

// ---------------------------------------------------------------------------
// The term universe
// ---------------------------------------------------------------------------

/// A logic term — the universe everything in this crate operates on.
#[derive(Debug, Clone, PartialEq)]
pub enum Term {
    Atom(String),
    Num(Number),
    Str(String),
    Var(LogicVar),
    Compound { functor: String, args: Vec<Term> },
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Term::Atom(s) => write!(f, "{}", s),
            Term::Num(n) => write!(f, "{}", n),
            Term::Str(s) => write!(f, "{:?}", s),
            Term::Var(v) => write!(f, "{}", v),
            Term::Compound { functor, args } => {
                write!(f, "{}(", functor)?;
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", a)?;
                }
                write!(f, ")")
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Constructors — small, ergonomic builders for use in tests and downstream crates
// ---------------------------------------------------------------------------

/// Build an atom from any string-like value.
pub fn atom(name: impl Into<String>) -> Term {
    Term::Atom(name.into())
}

/// Build an integer numeric term.
pub fn int(value: i64) -> Term {
    Term::Num(Number::Int(value))
}

/// Build a floating-point numeric term.
pub fn float(value: f64) -> Term {
    Term::Num(Number::Float(value))
}

/// Build a string term (distinct from atom).
pub fn string(value: impl Into<String>) -> Term {
    Term::Str(value.into())
}

/// Allocate a fresh logic variable and return it wrapped in a `Term`.
///
/// The `display_name` is for human-readable output only and does not
/// influence variable identity. See [`LogicVar::fresh`].
pub fn var(display_name: &str) -> LogicVar {
    LogicVar::fresh(Some(display_name))
}

/// Build a compound term `functor(args...)`.
pub fn compound(functor: impl Into<String>, args: Vec<Term>) -> Term {
    Term::Compound {
        functor: functor.into(),
        args,
    }
}

/// Build a list term using the canonical Prolog `'.'/2` cons-cell encoding.
///
/// `logic_list(vec![a, b, c])` becomes `.(a, .(b, .(c, [])))`, which is how
/// real Prolog represents lists internally. The pretty-printer for `Term`
/// shows the full cons form for now; later crates may add the standard
/// `[a, b, c]` syntactic sugar at the surface level.
pub fn logic_list(items: Vec<Term>) -> Term {
    let mut result = atom("[]");
    for item in items.into_iter().rev() {
        result = compound(".", vec![item, result]);
    }
    result
}

// ---------------------------------------------------------------------------
// Substitutions and unification
// ---------------------------------------------------------------------------

/// A substitution — a partial map from variable id to bound term.
///
/// Cloning a substitution copies the underlying map. For typical Prolog
/// programs this map has tens to hundreds of entries, so cloning is cheap.
/// Sharing-aware persistent representations are an optimization for a later
/// crate; the API is already shaped to allow them without breaking callers.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Substitution {
    bindings: HashMap<u64, Term>,
}

impl Substitution {
    /// The empty substitution.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Extend this substitution with a new binding, returning a new value.
    ///
    /// The original substitution is left unchanged. This is the operation
    /// every unification step uses internally; backtracking — in a later
    /// crate — simply drops the new substitution to recover the old one.
    pub fn extend(&self, var_id: u64, term: Term) -> Self {
        let mut new = self.bindings.clone();
        new.insert(var_id, term);
        Self { bindings: new }
    }

    /// Chase variable bindings until reaching a non-variable term or a
    /// variable that is not yet bound.
    ///
    /// This is the operation that turns a substitution into a *view* on
    /// the current state of a logic variable. It does **not** rewrite the
    /// substitution itself.
    pub fn walk(&self, term: &Term) -> Term {
        let mut current = term.clone();
        while let Term::Var(v) = &current {
            match self.bindings.get(&v.id) {
                Some(bound) => current = bound.clone(),
                None => break,
            }
        }
        current
    }

    /// Convenience: walk a variable by reference.
    pub fn walk_var(&self, v: &LogicVar) -> Term {
        self.walk(&Term::Var(v.clone()))
    }

    /// Return `true` if `var_id` occurs anywhere inside `term`, after
    /// walking through bindings. Used by [`unify`] to implement the
    /// occurs-check.
    fn occurs(&self, var_id: u64, term: &Term) -> bool {
        match self.walk(term) {
            Term::Var(v) => v.id == var_id,
            Term::Compound { args, .. } => args.iter().any(|a| self.occurs(var_id, a)),
            _ => false,
        }
    }
}

/// First-order unification with occurs-check.
///
/// Returns `Some(new_substitution)` if the two terms can be made
/// syntactically equal under some extension of `subst`, or `None` if they
/// cannot.
///
/// ## Cases
///
/// - **Two variables**: bind one to the other (the lower id is kept as
///   the representative purely for determinism).
/// - **Variable and non-variable**: bind the variable to the term,
///   provided it does not occur inside the term (occurs-check).
/// - **Two atoms / numbers / strings**: succeed iff equal.
/// - **Two compounds**: same functor, same arity, recursively unify
///   each argument pair.
/// - **Anything else**: fail.
///
/// Numbers do *not* cross variants: `1` and `1.0` do not unify. This
/// matches Prolog's `=/2` behaviour.
pub fn unify(a: &Term, b: &Term, subst: &Substitution) -> Option<Substitution> {
    let a = subst.walk(a);
    let b = subst.walk(b);

    match (a, b) {
        // Same variable on both sides — already equal under this substitution.
        (Term::Var(va), Term::Var(vb)) if va.id == vb.id => Some(subst.clone()),

        // Variable on the left — bind it (after occurs-check) to the right.
        (Term::Var(va), other) => {
            if subst.occurs(va.id, &other) {
                None
            } else {
                Some(subst.extend(va.id, other))
            }
        }

        // Variable on the right — symmetric to the previous case.
        (other, Term::Var(vb)) => {
            if subst.occurs(vb.id, &other) {
                None
            } else {
                Some(subst.extend(vb.id, other))
            }
        }

        // Ground equalities.
        (Term::Atom(x), Term::Atom(y)) if x == y => Some(subst.clone()),
        (Term::Num(x), Term::Num(y)) if x == y => Some(subst.clone()),
        (Term::Str(x), Term::Str(y)) if x == y => Some(subst.clone()),

        // Compound terms: same functor and arity, recurse.
        (
            Term::Compound {
                functor: fx,
                args: ax,
            },
            Term::Compound {
                functor: fy,
                args: ay,
            },
        ) if fx == fy && ax.len() == ay.len() => {
            let mut s = subst.clone();
            for (xi, yi) in ax.iter().zip(ay.iter()) {
                match unify(xi, yi, &s) {
                    Some(next) => s = next,
                    None => return None,
                }
            }
            Some(s)
        }

        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Inline unit tests — focused on each public surface in isolation
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atom_displays_as_its_name() {
        assert_eq!(atom("homer").to_string(), "homer");
    }

    #[test]
    fn int_and_float_are_distinct_terms() {
        let a = int(1);
        let b = float(1.0);
        assert_ne!(a, b);
        assert_eq!(a.to_string(), "1");
        assert_eq!(b.to_string(), "1");
    }

    #[test]
    fn strings_display_with_quotes() {
        assert_eq!(string("hello world").to_string(), "\"hello world\"");
    }

    #[test]
    fn fresh_variables_have_distinct_ids() {
        let x = var("X");
        let y = var("X"); // same display name, different identity
        assert_ne!(x.id, y.id);
        assert_ne!(x, y);
    }

    #[test]
    fn compound_displays_in_functional_form() {
        let t = compound("father", vec![atom("homer"), atom("bart")]);
        assert_eq!(t.to_string(), "father(homer, bart)");
    }

    #[test]
    fn logic_list_uses_cons_cell_encoding() {
        let l = logic_list(vec![atom("a"), atom("b")]);
        // .(a, .(b, []))
        assert_eq!(l.to_string(), ".(a, .(b, []))");
    }

    #[test]
    fn unify_two_identical_atoms_succeeds_without_new_bindings() {
        let s = unify(&atom("a"), &atom("a"), &Substitution::empty()).unwrap();
        assert_eq!(s, Substitution::empty());
    }

    #[test]
    fn unify_two_different_atoms_fails() {
        assert!(unify(&atom("a"), &atom("b"), &Substitution::empty()).is_none());
    }

    #[test]
    fn unify_variable_with_atom_binds_it() {
        let x = var("X");
        let s = unify(
            &Term::Var(x.clone()),
            &atom("homer"),
            &Substitution::empty(),
        )
        .unwrap();
        assert_eq!(s.walk_var(&x), atom("homer"));
    }

    #[test]
    fn unify_compound_unifies_argument_pairs() {
        // father(homer, X) ?= father(homer, bart) -> X = bart
        let x = var("X");
        let query = compound("father", vec![atom("homer"), Term::Var(x.clone())]);
        let fact = compound("father", vec![atom("homer"), atom("bart")]);
        let s = unify(&query, &fact, &Substitution::empty()).unwrap();
        assert_eq!(s.walk_var(&x), atom("bart"));
    }

    #[test]
    fn unify_fails_on_mismatched_functor() {
        let a = compound("p", vec![atom("x")]);
        let b = compound("q", vec![atom("x")]);
        assert!(unify(&a, &b, &Substitution::empty()).is_none());
    }

    #[test]
    fn unify_fails_on_mismatched_arity() {
        let a = compound("p", vec![atom("x")]);
        let b = compound("p", vec![atom("x"), atom("y")]);
        assert!(unify(&a, &b, &Substitution::empty()).is_none());
    }

    #[test]
    fn unify_int_and_float_does_not_succeed() {
        assert!(unify(&int(1), &float(1.0), &Substitution::empty()).is_none());
    }

    #[test]
    fn occurs_check_prevents_cyclic_binding() {
        // X = f(X) must fail under the occurs-check
        let x = var("X");
        let cyclic = compound("f", vec![Term::Var(x.clone())]);
        assert!(unify(&Term::Var(x.clone()), &cyclic, &Substitution::empty()).is_none());
    }

    #[test]
    fn unify_two_variables_makes_them_equal() {
        let x = var("X");
        let y = var("Y");
        let s = unify(
            &Term::Var(x.clone()),
            &Term::Var(y.clone()),
            &Substitution::empty(),
        )
        .unwrap();
        // Whatever the representative is, the two variables now walk to
        // the same term.
        assert_eq!(s.walk_var(&x), s.walk_var(&y));
    }

    #[test]
    fn substitution_extend_does_not_mutate_original() {
        let s0 = Substitution::empty();
        let s1 = s0.extend(0, atom("a"));
        assert!(s0.bindings.is_empty());
        assert_eq!(s1.bindings.len(), 1);
    }

    #[test]
    fn walk_through_chained_bindings_reaches_root() {
        // X -> Y -> homer
        let x = var("X");
        let y = var("Y");
        let s = Substitution::empty()
            .extend(x.id, Term::Var(y.clone()))
            .extend(y.id, atom("homer"));
        assert_eq!(s.walk_var(&x), atom("homer"));
    }
}
