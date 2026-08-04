//! # `AxiomDomain` / `AxiomCategory` — the fixed, non-extensible type table
//!
//! This is the one genuinely new piece of evaluator design MA13 §2/§3 asks
//! for: `symbolic_ir::IRNode` has **no domain, category, or per-value type
//! tag anywhere** (confirmed directly against the source, MA13 §2) — every
//! prior symbolic-family runtime in this repo (Wolfram/Macsyma/Derive/Reduce/
//! Maple) is, at the value-model level, a single flat universe of untyped
//! symbolic expressions. Axiom's own claim to fame is that *every* value
//! belongs to a domain, and reasoning about domains — declaring one (`:`),
//! coercing to one (`::`), and querying category membership (`has`) — is
//! constantly used even in the most basic session (MA13 §2).
//!
//! MA13 §3's scoping decision is deliberately narrow: this is **not** a
//! computed `Join`/conditional-export category algebra, and it is **not** a
//! generative domain-constructor mechanism a user could extend. It is a
//! small, fixed, hard-coded lookup table — the "consumer view" of Axiom's
//! type system (MA13 §3's own Chapters 0-2/5-6 vs. 11-13 split) — covering
//! exactly the built-in domains and categories MA13 §4 lists, no more:
//!
//! ```text
//! Domains  (fixed, non-extensible, MA13 §4):
//!   Boolean, Integer, PositiveInteger (x > 0), NonNegativeInteger (x >= 0),
//!   Float, String, Fraction(Integer), Polynomial(Integer), List(T)
//!   for T among the domains just listed (NOT List(List(...)))
//!
//! Categories (fixed, non-extensible, MA13 §4):
//!   Ring        -- Integer, Fraction(Integer), Polynomial(Integer)
//!   OrderedSet  -- Integer, Float, PositiveInteger, NonNegativeInteger
//! ```
//!
//! Deliberately **not** parameterized over each other beyond this (no
//! `Polynomial(Fraction(Integer))`, no `Complex`, no `Matrix` this cut) —
//! keeping the table genuinely small and finite is itself part of MA13 §3's
//! scoping decision, not an oversight: general recursive constructor
//! composition is exactly the "producer-side" generality MA13 defers whole.

use symbolic_ir::{apply, sym, IRNode};

/// A built-in Axiom domain — MA13 §4's fixed, non-extensible table.
///
/// `Fraction`/`Polynomial` are represented as their own dedicated variants
/// (`FractionInteger`/`PolynomialInteger`), not a generic `Fraction(Box<T>)`,
/// because MA13 §4 deliberately fixes their parameter to `Integer` only —
/// giving them a generic type parameter would silently invite exactly the
/// `Fraction(Polynomial(Integer))`/`Polynomial(Fraction(Integer))` nesting
/// MA13 §4 explicitly rules out for this cut.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AxiomDomain {
    Boolean,
    Integer,
    /// Subdomain of `Integer`: the membership predicate is `x > 0`.
    PositiveInteger,
    /// Subdomain of `Integer`: the membership predicate is `x >= 0`.
    NonNegativeInteger,
    Float,
    String,
    /// `Fraction(Integer)` — MA13 §2's own disclosed representation note:
    /// this is the only `Fraction(...)` shape with a packed native
    /// `IRNode::Rational` representation; a fraction of anything richer has
    /// no equivalent packed form and is out of scope this cut.
    FractionInteger,
    /// `Polynomial(Integer)`.
    PolynomialInteger,
    /// `List(T)` — `T` restricted to any of the other built-in domains
    /// (never `List(List(...))`, per MA13 §4's own "T among the domains
    /// just listed" wording, which does not re-include `List` itself).
    List(Box<AxiomDomain>),
}

/// A built-in Axiom category — MA13 §4's fixed, non-extensible table (just
/// `Ring` and `OrderedSet`, "enough to make this cut's own `has` queries
/// real and checkable ... without needing `Field`'s richer conditional-export
/// machinery").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AxiomCategory {
    Ring,
    OrderedSet,
}

impl AxiomDomain {
    /// The domains a `List(T)` element `T` may legally be (MA13 §4: "`T`
    /// among the domains just listed" — every built-in domain EXCEPT `List`
    /// itself, so `List(List(Integer))` is rejected).
    fn is_valid_list_element(&self) -> bool {
        !matches!(self, AxiomDomain::List(_))
    }

    /// Render the domain the way the book itself spells it (`Integer`,
    /// `Fraction(Integer)`, `List(Float)`, …) — used in coercion-failure
    /// error text and REPL output.
    pub fn display_name(&self) -> String {
        match self {
            AxiomDomain::Boolean => "Boolean".to_string(),
            AxiomDomain::Integer => "Integer".to_string(),
            AxiomDomain::PositiveInteger => "PositiveInteger".to_string(),
            AxiomDomain::NonNegativeInteger => "NonNegativeInteger".to_string(),
            AxiomDomain::Float => "Float".to_string(),
            AxiomDomain::String => "String".to_string(),
            AxiomDomain::FractionInteger => "Fraction(Integer)".to_string(),
            AxiomDomain::PolynomialInteger => "Polynomial(Integer)".to_string(),
            AxiomDomain::List(inner) => format!("List({})", inner.display_name()),
        }
    }
}

impl AxiomCategory {
    pub fn display_name(&self) -> &'static str {
        match self {
            AxiomCategory::Ring => "Ring",
            AxiomCategory::OrderedSet => "OrderedSet",
        }
    }
}

/// A resolved, generic `NAME [ (args...) ]` shape read off a parsed
/// `type_expr` node (see `crate::builtins::parse_type_spec`) — the common
/// intermediate this module's own domain/category resolvers both consume,
/// so `builtins.rs` only needs to know how to walk the *grammar*, and this
/// module only needs to know the fixed *table*.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeSpec {
    pub name: String,
    pub args: Vec<TypeSpec>,
}

/// A domain/category resolution failure — an unknown name, a wrong-arity
/// constructor call, or an argument that itself doesn't resolve to a domain
/// this constructor accepts. Mirrors the book's own confirmed
/// `Polynomial(String)`-is-invalid worked example (MA13 §3): this is a real,
/// checked rejection, not a silent fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainError(pub String);

impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for DomainError {}

/// Resolve a [`TypeSpec`] against the fixed domain table (MA13 §4).
///
/// Every constructor's arity and argument-domain restriction is checked
/// explicitly here — an unrecognised name, a wrong number of arguments, or
/// an argument that itself is not one of the domains a constructor accepts
/// (`Fraction`/`Polynomial` accept only `Integer`; `List` accepts any
/// built-in domain except `List` itself) is rejected with a [`DomainError`],
/// mirroring the book's own confirmed `Polynomial(String)`-is-invalid
/// example rather than silently accepting or guessing.
pub fn resolve_domain(spec: &TypeSpec) -> Result<AxiomDomain, DomainError> {
    match (spec.name.as_str(), spec.args.len()) {
        ("Boolean", 0) => Ok(AxiomDomain::Boolean),
        ("Integer", 0) => Ok(AxiomDomain::Integer),
        ("PositiveInteger", 0) => Ok(AxiomDomain::PositiveInteger),
        ("NonNegativeInteger", 0) => Ok(AxiomDomain::NonNegativeInteger),
        ("Float", 0) => Ok(AxiomDomain::Float),
        ("String", 0) => Ok(AxiomDomain::String),
        ("Fraction", 1) => {
            let inner = resolve_domain(&spec.args[0])?;
            if inner == AxiomDomain::Integer {
                Ok(AxiomDomain::FractionInteger)
            } else {
                Err(DomainError(format!(
                    "Fraction({}) is not a valid type -- this cut's `Fraction` is fixed to `Fraction(Integer)` only",
                    inner.display_name()
                )))
            }
        }
        ("Polynomial", 1) => {
            let inner = resolve_domain(&spec.args[0])?;
            if inner == AxiomDomain::Integer {
                Ok(AxiomDomain::PolynomialInteger)
            } else {
                Err(DomainError(format!(
                    "Polynomial({}) is not a valid type -- this cut's `Polynomial` is fixed to `Polynomial(Integer)` only",
                    inner.display_name()
                )))
            }
        }
        ("List", 1) => {
            let inner = resolve_domain(&spec.args[0])?;
            if inner.is_valid_list_element() {
                Ok(AxiomDomain::List(Box::new(inner)))
            } else {
                Err(DomainError(format!(
                    "List({}) is not a valid type -- `List`'s element type cannot itself be a `List`",
                    inner.display_name()
                )))
            }
        }
        ("Fraction" | "Polynomial" | "List", n) => Err(DomainError(format!(
            "`{}` takes exactly 1 type argument, got {n}",
            spec.name
        ))),
        (other, _) => Err(DomainError(format!(
            "`{other}` is not one of this cut's fixed built-in domains \
             (Boolean, Integer, PositiveInteger, NonNegativeInteger, Float, \
             String, Fraction(Integer), Polynomial(Integer), List(T))"
        ))),
    }
}

/// Resolve a [`TypeSpec`] against the fixed category table (MA13 §4: just
/// `Ring` and `OrderedSet`, both zero-argument names).
pub fn resolve_category(spec: &TypeSpec) -> Result<AxiomCategory, DomainError> {
    match (spec.name.as_str(), spec.args.len()) {
        ("Ring", 0) => Ok(AxiomCategory::Ring),
        ("OrderedSet", 0) => Ok(AxiomCategory::OrderedSet),
        ("Ring" | "OrderedSet", n) => Err(DomainError(format!(
            "`{}` takes no type arguments, got {n}",
            spec.name
        ))),
        (other, _) => Err(DomainError(format!(
            "`{other}` is not one of this cut's fixed built-in categories (Ring, OrderedSet)"
        ))),
    }
}

/// The fixed `AxiomDomain x AxiomCategory -> bool` membership table (MA13
/// §3/§4) — a hard-coded lookup, not a computed `Join`/conditional-export
/// algebra. Confirmed examples from the book: `Polynomial(Integer) has Ring`
/// is `true`; `List(Integer) has Ring` is `false` (MA13 §4).
pub fn domain_has_category(domain: &AxiomDomain, category: AxiomCategory) -> bool {
    match category {
        AxiomCategory::Ring => matches!(
            domain,
            AxiomDomain::Integer | AxiomDomain::FractionInteger | AxiomDomain::PolynomialInteger
        ),
        AxiomCategory::OrderedSet => matches!(
            domain,
            AxiomDomain::Integer
                | AxiomDomain::Float
                | AxiomDomain::PositiveInteger
                | AxiomDomain::NonNegativeInteger
        ),
    }
}

/// Attempt to coerce the evaluated value `node` into `domain`, returning the
/// (possibly representation-converted) value on success or `None` on
/// failure.
///
/// Used by both `::` (coercion) and `:`-declared assignment (MA13 §3/§4) to
/// decide success/failure *and* to produce the value actually stored/
/// displayed. Subdomain membership (`PositiveInteger`/`NonNegativeInteger`)
/// is implemented exactly the way the book describes it conceptually — a
/// predicate function checked at coercion time — rather than a generative
/// subdomain-definition mechanism (MA13 §3). Most domains are a pure
/// membership check (the value's own `IRNode` shape already IS the target
/// representation, so a successful coercion returns it unchanged) — `Float`
/// is the one built-in domain this cut actually *converts* representation
/// for (`Integer`/`Rational` -> `Float`, real Axiom's own `3 :: Float`
/// coercion genuinely produces `3.0`, not merely re-tagging an `Integer`
/// node with a `Float` label, which would leave the printed value and its
/// domain visibly inconsistent).
pub fn coerce_value(node: &IRNode, domain: &AxiomDomain) -> Option<IRNode> {
    match domain {
        AxiomDomain::Boolean => is_boolean(node).then(|| node.clone()),
        AxiomDomain::Integer => matches!(node, IRNode::Integer(_)).then(|| node.clone()),
        AxiomDomain::PositiveInteger => {
            matches!(node, IRNode::Integer(n) if *n > 0).then(|| node.clone())
        }
        AxiomDomain::NonNegativeInteger => {
            matches!(node, IRNode::Integer(n) if *n >= 0).then(|| node.clone())
        }
        AxiomDomain::Float => match node {
            IRNode::Float(_) => Some(node.clone()),
            IRNode::Integer(n) => Some(IRNode::Float(*n as f64)),
            IRNode::Rational(n, d) => Some(IRNode::Float(*n as f64 / *d as f64)),
            _ => None,
        },
        AxiomDomain::String => matches!(node, IRNode::Str(_)).then(|| node.clone()),
        // A whole number is trivially also a fraction over the integers
        // (real Axiom's own `3 :: Fraction Integer` coerces cleanly) --
        // `symbolic-ir`'s own `rational()` constructor already reduces any
        // denominator-1 rational down to a packed `Integer`, so an
        // `IRNode::Integer` is the ONLY other shape a `Fraction(Integer)`
        // value could ever arrive in besides `Rational` itself. Neither
        // needs converting -- both are already the packed representation
        // this domain uses.
        AxiomDomain::FractionInteger => {
            matches!(node, IRNode::Rational(_, _) | IRNode::Integer(_)).then(|| node.clone())
        }
        AxiomDomain::PolynomialInteger => {
            is_polynomial_over_integers(node).then(|| node.clone())
        }
        AxiomDomain::List(inner) => match node {
            IRNode::Apply(app) if is_list_head(&app.head) => {
                let mut coerced = Vec::with_capacity(app.args.len());
                for elem in &app.args {
                    coerced.push(coerce_value(elem, inner)?);
                }
                Some(apply(sym(symbolic_ir::LIST), coerced))
            }
            _ => None,
        },
    }
}

/// True for the `True`/`False` symbols the shared `symbolic-vm` handler
/// table already produces for every comparison/logic result (see
/// `symbolic_vm::handlers::true_sym`/`false_sym`, not re-exported, so this
/// checks the same fixed spelling directly).
pub(crate) fn is_boolean(node: &IRNode) -> bool {
    matches!(node, IRNode::Symbol(s) if s == "True" || s == "False")
}

fn is_list_head(head: &IRNode) -> bool {
    matches!(head, IRNode::Symbol(s) if s == symbolic_ir::LIST)
}

/// Structural "is this a polynomial over the integers" predicate — a
/// variable, an integer constant, or `Add`/`Sub`/`Mul`/`Neg` combining
/// values that are themselves `Polynomial(Integer)`-shaped, or `Pow` raising
/// one to a non-negative integer constant exponent. There is no dedicated
/// `Polynomial` *value* representation in this cut (MA13 §2: Axiom's
/// polynomial arithmetic reuses the ordinary `Apply(Times, ...)`/
/// `Apply(Plus, ...)` tree `cas-simplify` already normalizes for every other
/// CAS-family language here) -- this predicate is `axiom-runtime`'s own
/// structural stand-in for category/domain membership, checked entirely
/// within this crate (MA13 §2/§5: "evaluated entirely within
/// `axiom-runtime`'s own dispatcher, never inside `symbolic-vm` itself"),
/// never a change to `symbolic-ir`/`symbolic-vm` themselves.
///
/// This also gives the book's own confirmed "no information is lost"
/// example real teeth for the un-cancelled case: an unresolved symbolic sum
/// like `x + y` (MA13 §3's own `x + 3 - x` illustration, before/if it fully
/// cancels) structurally fits this predicate and is domain-inferred as
/// `Polynomial(Integer)` by `crate::value::infer_domain`, which calls this
/// same function.
///
/// No separate recursion-depth cap is added here: this walks an already-
/// *evaluated* `IRNode`, whose depth is bounded by the same
/// `MAX_STATEMENT_TOKENS` input gate `crate::check_statement_token_counts`
/// applies before evaluation ever starts (see that constant's own doc
/// comment), and the whole evaluation-plus-domain-check pipeline already
/// runs on `crate::EVAL_STACK_SIZE`'s large worker-thread stack.
pub(crate) fn is_polynomial_over_integers(node: &IRNode) -> bool {
    match node {
        IRNode::Integer(_) | IRNode::Symbol(_) => true,
        IRNode::Apply(app) => {
            let head = match &app.head {
                IRNode::Symbol(s) => s.as_str(),
                _ => return false,
            };
            match head {
                symbolic_ir::ADD | symbolic_ir::SUB | symbolic_ir::MUL => {
                    app.args.iter().all(is_polynomial_over_integers)
                }
                symbolic_ir::NEG => {
                    app.args.len() == 1 && is_polynomial_over_integers(&app.args[0])
                }
                symbolic_ir::POW => {
                    app.args.len() == 2
                        && is_polynomial_over_integers(&app.args[0])
                        && matches!(&app.args[1], IRNode::Integer(n) if *n >= 0)
                }
                _ => false,
            }
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symbolic_ir::{apply, int, rat, str_node, sym};

    fn spec(name: &str, args: Vec<TypeSpec>) -> TypeSpec {
        TypeSpec {
            name: name.to_string(),
            args,
        }
    }
    fn leaf(name: &str) -> TypeSpec {
        spec(name, vec![])
    }

    // --- resolve_domain -------------------------------------------------

    #[test]
    fn every_zero_arg_builtin_domain_resolves() {
        for name in [
            "Boolean",
            "Integer",
            "PositiveInteger",
            "NonNegativeInteger",
            "Float",
            "String",
        ] {
            assert!(resolve_domain(&leaf(name)).is_ok(), "{name} should resolve");
        }
    }

    #[test]
    fn fraction_of_integer_resolves() {
        assert_eq!(
            resolve_domain(&spec("Fraction", vec![leaf("Integer")])),
            Ok(AxiomDomain::FractionInteger)
        );
    }

    #[test]
    fn polynomial_of_integer_resolves() {
        assert_eq!(
            resolve_domain(&spec("Polynomial", vec![leaf("Integer")])),
            Ok(AxiomDomain::PolynomialInteger)
        );
    }

    #[test]
    fn polynomial_of_string_is_rejected() {
        // The book's own confirmed worked example (MA13 §3): `Polynomial(String)`
        // is "not a valid type."
        assert!(resolve_domain(&spec("Polynomial", vec![leaf("String")])).is_err());
    }

    #[test]
    fn fraction_of_polynomial_is_rejected() {
        // MA13 §4: this cut's `Fraction`/`Polynomial` are fixed to `Integer`
        // only, not parameterized over each other.
        assert!(resolve_domain(&spec(
            "Fraction",
            vec![spec("Polynomial", vec![leaf("Integer")])]
        ))
        .is_err());
    }

    #[test]
    fn list_of_a_builtin_domain_resolves() {
        assert_eq!(
            resolve_domain(&spec("List", vec![leaf("Integer")])),
            Ok(AxiomDomain::List(Box::new(AxiomDomain::Integer)))
        );
        assert!(resolve_domain(&spec("List", vec![leaf("Float")])).is_ok());
        assert!(resolve_domain(&spec("List", vec![spec("Fraction", vec![leaf("Integer")])]))
            .is_ok());
    }

    #[test]
    fn list_of_list_is_rejected() {
        assert!(resolve_domain(&spec("List", vec![spec("List", vec![leaf("Integer")])])).is_err());
    }

    #[test]
    fn unknown_domain_name_is_rejected() {
        assert!(resolve_domain(&leaf("Matrix")).is_err());
        assert!(resolve_domain(&leaf("Complex")).is_err());
        assert!(resolve_domain(&leaf("Ring")).is_err()); // a category, not a domain
    }

    #[test]
    fn wrong_arity_is_rejected() {
        assert!(resolve_domain(&spec("Integer", vec![leaf("Integer")])).is_err());
        assert!(resolve_domain(&spec("Fraction", vec![])).is_err());
        assert!(resolve_domain(&spec(
            "Fraction",
            vec![leaf("Integer"), leaf("Integer")]
        ))
        .is_err());
    }

    // --- resolve_category -------------------------------------------------

    #[test]
    fn ring_and_ordered_set_resolve() {
        assert_eq!(resolve_category(&leaf("Ring")), Ok(AxiomCategory::Ring));
        assert_eq!(
            resolve_category(&leaf("OrderedSet")),
            Ok(AxiomCategory::OrderedSet)
        );
    }

    #[test]
    fn unknown_category_name_is_rejected() {
        assert!(resolve_category(&leaf("Field")).is_err()); // real Axiom has Field; this cut doesn't
        assert!(resolve_category(&leaf("Integer")).is_err()); // a domain, not a category
    }

    // --- domain_has_category (the fixed table) -----------------------------

    #[test]
    fn polynomial_integer_has_ring_the_books_own_confirmed_true_example() {
        assert!(domain_has_category(
            &AxiomDomain::PolynomialInteger,
            AxiomCategory::Ring
        ));
    }

    #[test]
    fn list_integer_does_not_have_ring_the_books_own_confirmed_false_example() {
        assert!(!domain_has_category(
            &AxiomDomain::List(Box::new(AxiomDomain::Integer)),
            AxiomCategory::Ring
        ));
    }

    #[test]
    fn fraction_integer_has_ring() {
        assert!(domain_has_category(
            &AxiomDomain::FractionInteger,
            AxiomCategory::Ring
        ));
    }

    #[test]
    fn boolean_does_not_have_ring() {
        // Real Axiom's own worked counter-example (MA13 §3): under
        // APL-style 0/1 encoding Boolean would syntactically export every
        // Ring operation, yet is correctly NOT asserted a Ring (the
        // additive-inverse axiom fails). This cut's fixed table encodes
        // that assertion directly rather than computing it.
        assert!(!domain_has_category(&AxiomDomain::Boolean, AxiomCategory::Ring));
    }

    #[test]
    fn every_ring_member_is_exactly_the_confirmed_set() {
        for d in [
            AxiomDomain::Integer,
            AxiomDomain::FractionInteger,
            AxiomDomain::PolynomialInteger,
        ] {
            assert!(domain_has_category(&d, AxiomCategory::Ring), "{d:?}");
        }
        for d in [
            AxiomDomain::Boolean,
            AxiomDomain::Float,
            AxiomDomain::String,
            AxiomDomain::PositiveInteger,
            AxiomDomain::NonNegativeInteger,
            AxiomDomain::List(Box::new(AxiomDomain::Integer)),
        ] {
            assert!(!domain_has_category(&d, AxiomCategory::Ring), "{d:?}");
        }
    }

    #[test]
    fn every_ordered_set_member_is_exactly_the_confirmed_set() {
        for d in [
            AxiomDomain::Integer,
            AxiomDomain::Float,
            AxiomDomain::PositiveInteger,
            AxiomDomain::NonNegativeInteger,
        ] {
            assert!(domain_has_category(&d, AxiomCategory::OrderedSet), "{d:?}");
        }
        for d in [
            AxiomDomain::Boolean,
            AxiomDomain::String,
            AxiomDomain::FractionInteger,
            AxiomDomain::PolynomialInteger,
        ] {
            assert!(!domain_has_category(&d, AxiomCategory::OrderedSet), "{d:?}");
        }
    }

    // --- coerce_value (subdomain predicates + coercion checks) -------------

    #[test]
    fn positive_integer_predicate() {
        assert!(coerce_value(&int(1), &AxiomDomain::PositiveInteger).is_some());
        assert!(coerce_value(&int(0), &AxiomDomain::PositiveInteger).is_none());
        assert!(coerce_value(&int(-1), &AxiomDomain::PositiveInteger).is_none());
    }

    #[test]
    fn non_negative_integer_predicate() {
        assert!(coerce_value(&int(0), &AxiomDomain::NonNegativeInteger).is_some());
        assert!(coerce_value(&int(5), &AxiomDomain::NonNegativeInteger).is_some());
        assert!(coerce_value(&int(-1), &AxiomDomain::NonNegativeInteger).is_none());
    }

    #[test]
    fn integer_domain_rejects_float() {
        assert!(coerce_value(&symbolic_ir::flt(1.0), &AxiomDomain::Integer).is_none());
    }

    #[test]
    fn float_domain_converts_integer_and_rational() {
        // Real Axiom's own `3 :: Float` genuinely produces `3.0`, not merely
        // a re-tagged Integer -- Float is the one built-in domain this cut
        // actually converts representation for.
        assert_eq!(
            coerce_value(&int(3), &AxiomDomain::Float),
            Some(symbolic_ir::flt(3.0))
        );
        assert_eq!(
            coerce_value(&rat(1, 4), &AxiomDomain::Float),
            Some(symbolic_ir::flt(0.25))
        );
        assert_eq!(
            coerce_value(&symbolic_ir::flt(2.5), &AxiomDomain::Float),
            Some(symbolic_ir::flt(2.5))
        );
    }

    #[test]
    fn fraction_integer_accepts_rational_and_plain_integer_unchanged() {
        assert_eq!(
            coerce_value(&rat(1, 3), &AxiomDomain::FractionInteger),
            Some(rat(1, 3))
        );
        assert_eq!(
            coerce_value(&int(3), &AxiomDomain::FractionInteger),
            Some(int(3))
        );
    }

    #[test]
    fn string_domain_predicate() {
        assert!(coerce_value(&str_node("hi"), &AxiomDomain::String).is_some());
        assert!(coerce_value(&int(1), &AxiomDomain::String).is_none());
    }

    #[test]
    fn boolean_domain_predicate() {
        assert!(coerce_value(&sym("True"), &AxiomDomain::Boolean).is_some());
        assert!(coerce_value(&sym("False"), &AxiomDomain::Boolean).is_some());
        assert!(coerce_value(&sym("x"), &AxiomDomain::Boolean).is_none());
    }

    #[test]
    fn polynomial_integer_accepts_symbols_and_arithmetic_over_them() {
        assert!(coerce_value(&sym("x"), &AxiomDomain::PolynomialInteger).is_some());
        assert!(coerce_value(&int(5), &AxiomDomain::PolynomialInteger).is_some());
        let expr = apply(sym(symbolic_ir::ADD), vec![sym("x"), int(3)]);
        assert!(coerce_value(&expr, &AxiomDomain::PolynomialInteger).is_some());
    }

    #[test]
    fn polynomial_integer_rejects_float_and_string_leaves() {
        let expr = apply(sym(symbolic_ir::ADD), vec![sym("x"), symbolic_ir::flt(1.5)]);
        assert!(coerce_value(&expr, &AxiomDomain::PolynomialInteger).is_none());
    }

    #[test]
    fn list_of_integer_checks_every_element() {
        let good = apply(sym(symbolic_ir::LIST), vec![int(1), int(2), int(3)]);
        assert!(coerce_value(&good, &AxiomDomain::List(Box::new(AxiomDomain::Integer))).is_some());
        let bad = apply(sym(symbolic_ir::LIST), vec![int(1), symbolic_ir::flt(2.0)]);
        assert!(coerce_value(&bad, &AxiomDomain::List(Box::new(AxiomDomain::Integer))).is_none());
    }

    #[test]
    fn list_of_float_converts_every_integer_element() {
        let list = apply(sym(symbolic_ir::LIST), vec![int(1), int(2)]);
        assert_eq!(
            coerce_value(&list, &AxiomDomain::List(Box::new(AxiomDomain::Float))),
            Some(apply(
                sym(symbolic_ir::LIST),
                vec![symbolic_ir::flt(1.0), symbolic_ir::flt(2.0)]
            ))
        );
    }

    #[test]
    fn empty_list_fits_any_list_domain_vacuously() {
        let empty = apply(sym(symbolic_ir::LIST), vec![]);
        assert!(coerce_value(&empty, &AxiomDomain::List(Box::new(AxiomDomain::Integer))).is_some());
    }

    #[test]
    fn display_names_match_the_books_own_spelling() {
        assert_eq!(AxiomDomain::Integer.display_name(), "Integer");
        assert_eq!(
            AxiomDomain::FractionInteger.display_name(),
            "Fraction(Integer)"
        );
        assert_eq!(
            AxiomDomain::PolynomialInteger.display_name(),
            "Polynomial(Integer)"
        );
        assert_eq!(
            AxiomDomain::List(Box::new(AxiomDomain::Float)).display_name(),
            "List(Float)"
        );
        assert_eq!(AxiomCategory::Ring.display_name(), "Ring");
        assert_eq!(AxiomCategory::OrderedSet.display_name(), "OrderedSet");
    }
}
