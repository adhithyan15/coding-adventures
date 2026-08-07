//! # `AxiomValue` — an `IRNode` paired with its (possibly unknown) domain
//!
//! MA13 §2's central finding is that `symbolic_ir::IRNode` carries no
//! domain/type tag at all, so `axiom-runtime` adds its own thin wrapper
//! layer on top rather than changing the shared IR. [`AxiomValue`] is that
//! wrapper: the evaluated [`IRNode`] plus an [`Option<AxiomDomain>`] — `None`
//! when no domain classification is meaningful (an unbound free symbol with
//! no declared constraint, a registered function name, …), `Some(d)` once a
//! literal shape, a `:`/`::` declaration/coercion, or a category-query
//! result gives one.
//!
//! ## Domain inference is per-value, not propagated through computation
//!
//! Real Axiom threads domain information through every arithmetic step,
//! building "domain towers" so a fully-cancelled result like `x + 3 - x`
//! still reports `Polynomial(Integer)`, "so no information is lost" (MA13
//! §3, confirmed verbatim). That full propagation mechanism is part of the
//! *producer*-side domain-tower machinery MA13 §3 defers whole (it needs a
//! real per-operation domain-algebra, not a fixed lookup table). This cut
//! instead infers a value's domain **structurally, from its own final
//! shape** after evaluation (`infer_domain`, below) — a real, disclosed
//! narrowing: an expression that fully cancels to a plain literal is
//! domain-tagged from that literal's own shape (e.g. `PositiveInteger`),
//! not preserved as the richer domain the *original* expression shape
//! would have carried. An expression that does **not** fully cancel (e.g.
//! `x + y`, two distinct free symbols) still gets a meaningful answer,
//! because [`infer_domain`]'s `Apply` case reuses
//! [`crate::domains::is_polynomial_over_integers`]'s own `PolynomialInteger`
//! structural predicate as a catch-all: an unresolved arithmetic expression
//! over integer/symbol leaves is inferred as `Polynomial(Integer)`, which is
//! the same fixed-table domain the book's own example names.

use crate::domains::{is_polynomial_over_integers, AxiomDomain};
use symbolic_ir::IRNode;

/// An evaluated Axiom value: the underlying [`IRNode`] plus its domain, when
/// one is known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxiomValue {
    pub node: IRNode,
    pub domain: Option<AxiomDomain>,
}

impl AxiomValue {
    /// Wrap `node`, inferring its domain structurally (see the module doc
    /// comment).
    pub fn inferred(node: IRNode) -> Self {
        let domain = infer_domain(&node);
        AxiomValue { node, domain }
    }

    /// Wrap `node` with an explicit, already-known domain (used after a
    /// successful `:`/`::` check, and for `has`'s `Boolean` result).
    pub fn with_domain(node: IRNode, domain: AxiomDomain) -> Self {
        AxiomValue {
            node,
            domain: Some(domain),
        }
    }
}

/// Infer a value's domain purely from its own evaluated shape.
///
/// - `Integer(n)`: `PositiveInteger` if `n > 0`, else `Integer` — MA13 §4's
///   own literal-inference row lists only `PositiveInteger`/`Integer`/
///   `Float`, not `NonNegativeInteger`, so a zero or negative integer
///   literal is conservatively inferred as the broader `Integer`, not
///   `NonNegativeInteger` (a real, disclosed, narrower reading of that row
///   rather than an invented extension of it).
/// - `Rational(_, _)`: `Fraction(Integer)`.
/// - `Float(_)`: `Float`.
/// - `Str(_)`: `String`.
/// - `Symbol("True"|"False")`: `Boolean`.
/// - any other bare `Symbol`: unknown (`None`) — a free variable with no
///   declared constraint has no domain of its own in this cut.
/// - `Apply(List, elems)`: `List(T)`, `T` inferred from the first element if
///   every element shares that same inferred domain; `None` for an empty or
///   a domain-heterogeneous list (a real, disclosed narrowing rather than
///   fabricating a placeholder element domain).
/// - any other `Apply(...)`: `Polynomial(Integer)` if it structurally fits
///   that shape (see `crate::domains::is_polynomial_over_integers`'s own predicate —
///   the book's own "`x + 3 - x` stays `Polynomial(Integer)`" example, for
///   the un-cancelled case), else `None` (a user-function call, or an
///   expression mixing a `Float`/`String` leaf into otherwise-arithmetic
///   shape).
pub fn infer_domain(node: &IRNode) -> Option<AxiomDomain> {
    match node {
        IRNode::Integer(n) if *n > 0 => Some(AxiomDomain::PositiveInteger),
        IRNode::Integer(_) => Some(AxiomDomain::Integer),
        IRNode::Rational(_, _) => Some(AxiomDomain::FractionInteger),
        IRNode::Float(_) => Some(AxiomDomain::Float),
        IRNode::Str(_) => Some(AxiomDomain::String),
        IRNode::Symbol(s) if s == "True" || s == "False" => Some(AxiomDomain::Boolean),
        IRNode::Symbol(_) => None,
        IRNode::Apply(app) if is_list_head(&app.head) => infer_list_domain(&app.args),
        IRNode::Apply(_) if is_polynomial_over_integers(node) => {
            Some(AxiomDomain::PolynomialInteger)
        }
        IRNode::Apply(_) => None,
    }
}

fn is_list_head(head: &IRNode) -> bool {
    matches!(head, IRNode::Symbol(s) if s == symbolic_ir::LIST)
}

fn infer_list_domain(elems: &[IRNode]) -> Option<AxiomDomain> {
    let (first, rest) = elems.split_first()?;
    let first_domain = infer_domain(first)?;
    for elem in rest {
        if infer_domain(elem).as_ref() != Some(&first_domain) {
            return None;
        }
    }
    Some(AxiomDomain::List(Box::new(first_domain)))
}

/// Render an [`IRNode`] the way an Axiom session echoes it: infix
/// arithmetic (`+ - * / ^`), Axiom's own `~=` not-equal spelling, `[a, b,
/// c]` lists, and lowercase `true`/`false` for the `True`/`False` symbols
/// (a disclosed presentation judgment call, matching this repo's existing
/// convention of choosing a clean, readable rendering when the exact
/// interactive byte-for-byte console format is not itself independently
/// re-verified here — see `idl-runtime::value::display`'s own identical
/// disclosure for its own numeric formatting). Everything else (a bare
/// symbol, a user-defined function call, an `Integer`/`Float`/`Rational`/
/// `Str` literal) prints exactly the way `symbolic_ir::IRNode`'s own
/// `Display` impl already renders it, reused unchanged, since this cut has
/// no surface convention that diverges from it (Axiom's arithmetic/list
/// surface is the same family of infix notation Reduce/Derive/Maple already
/// share, MA13 §5).
pub fn print_axiom(node: &IRNode) -> String {
    print_at(node, PREC_LOWEST)
}

const PREC_LOWEST: u8 = 0;
const PREC_CMP: u8 = 1;
const PREC_ADD: u8 = 2;
const PREC_MUL: u8 = 3;
const PREC_NEG: u8 = 4;
const PREC_POW: u8 = 5;
const PREC_ATOM: u8 = 6;

fn print_at(node: &IRNode, parent_prec: u8) -> String {
    let (text, prec) = render(node);
    if prec < parent_prec {
        format!("({text})")
    } else {
        text
    }
}

fn render(node: &IRNode) -> (String, u8) {
    match node {
        IRNode::Integer(n) => (n.to_string(), PREC_ATOM),
        IRNode::Float(v) => (format!("{v:?}"), PREC_ATOM),
        IRNode::Rational(n, d) => (format!("{n}/{d}"), PREC_MUL),
        IRNode::Str(s) => (format!("\"{s}\""), PREC_ATOM),
        IRNode::Symbol(s) if s == "True" => ("true".to_string(), PREC_ATOM),
        IRNode::Symbol(s) if s == "False" => ("false".to_string(), PREC_ATOM),
        IRNode::Symbol(s) => (s.clone(), PREC_ATOM),
        IRNode::Apply(app) => render_apply(app),
    }
}

fn render_apply(app: &symbolic_ir::IRApply) -> (String, u8) {
    let head_name = match &app.head {
        IRNode::Symbol(s) => Some(s.as_str()),
        _ => None,
    };
    let args = &app.args;

    if let Some(name) = head_name {
        if let Some((op, prec)) = infix_binary(name) {
            if args.len() == 2 {
                let l = print_at(&args[0], prec);
                let r = print_at(&args[1], prec);
                return (format!("{l}{op}{r}"), prec);
            }
        }
        match name {
            symbolic_ir::POW if args.len() == 2 => {
                let base = print_at(&args[0], PREC_POW + 1);
                let exp = print_at(&args[1], PREC_NEG);
                return (format!("{base}^{exp}"), PREC_POW);
            }
            symbolic_ir::NEG if args.len() == 1 => {
                let inner = print_at(&args[0], PREC_NEG);
                return (format!("-{inner}"), PREC_NEG);
            }
            symbolic_ir::LIST => {
                let parts: Vec<String> = args.iter().map(print_axiom).collect();
                return (format!("[{}]", parts.join(", ")), PREC_ATOM);
            }
            _ => {}
        }
        let parts: Vec<String> = args.iter().map(print_axiom).collect();
        return (format!("{name}({})", parts.join(", ")), PREC_ATOM);
    }

    let head_text = print_at(&app.head, PREC_ATOM);
    let parts: Vec<String> = args.iter().map(print_axiom).collect();
    (format!("{head_text}({})", parts.join(", ")), PREC_ATOM)
}

/// Binary infix arithmetic/comparison operators — `(surface, precedence)`.
/// Axiom's own not-equal spelling is `~=` (MA13 §4, confirmed directly --
/// NOT Maple's `<>`, NOT Wolfram's `!=`).
fn infix_binary(name: &str) -> Option<(&'static str, u8)> {
    Some(match name {
        symbolic_ir::ADD => (" + ", PREC_ADD),
        symbolic_ir::SUB => (" - ", PREC_ADD),
        symbolic_ir::MUL => ("*", PREC_MUL),
        symbolic_ir::DIV => ("/", PREC_MUL),
        symbolic_ir::EQUAL => (" = ", PREC_CMP),
        symbolic_ir::NOT_EQUAL => (" ~= ", PREC_CMP),
        symbolic_ir::LESS => (" < ", PREC_CMP),
        symbolic_ir::GREATER => (" > ", PREC_CMP),
        symbolic_ir::LESS_EQUAL => (" <= ", PREC_CMP),
        symbolic_ir::GREATER_EQUAL => (" >= ", PREC_CMP),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use symbolic_ir::{apply, flt, int, rat, str_node, sym};

    // --- infer_domain --------------------------------------------------

    #[test]
    fn positive_integer_literal_infers_positive_integer() {
        assert_eq!(infer_domain(&int(5)), Some(AxiomDomain::PositiveInteger));
    }

    #[test]
    fn zero_and_negative_integer_literals_infer_plain_integer() {
        assert_eq!(infer_domain(&int(0)), Some(AxiomDomain::Integer));
        assert_eq!(infer_domain(&int(-3)), Some(AxiomDomain::Integer));
    }

    #[test]
    fn float_literal_infers_float() {
        assert_eq!(infer_domain(&flt(1.5)), Some(AxiomDomain::Float));
    }

    #[test]
    fn rational_infers_fraction_of_integer() {
        assert_eq!(infer_domain(&rat(1, 3)), Some(AxiomDomain::FractionInteger));
    }

    #[test]
    fn string_literal_infers_string() {
        assert_eq!(infer_domain(&str_node("hi")), Some(AxiomDomain::String));
    }

    #[test]
    fn boolean_symbols_infer_boolean() {
        assert_eq!(infer_domain(&sym("True")), Some(AxiomDomain::Boolean));
        assert_eq!(infer_domain(&sym("False")), Some(AxiomDomain::Boolean));
    }

    #[test]
    fn a_free_symbol_has_no_inferred_domain() {
        assert_eq!(infer_domain(&sym("x")), None);
    }

    #[test]
    fn unresolved_arithmetic_over_symbols_infers_polynomial_integer() {
        // MA13 §3's own confirmed example (the un-cancelled case): a sum of
        // free symbols/integers is Polynomial(Integer).
        let expr = apply(sym(symbolic_ir::ADD), vec![sym("x"), sym("y")]);
        assert_eq!(infer_domain(&expr), Some(AxiomDomain::PolynomialInteger));
    }

    #[test]
    fn a_user_function_call_has_no_inferred_domain() {
        let expr = apply(sym("f"), vec![sym("x")]);
        assert_eq!(infer_domain(&expr), None);
    }

    #[test]
    fn homogeneous_list_infers_list_of_element_domain() {
        let list = apply(sym(symbolic_ir::LIST), vec![int(1), int(2), int(3)]);
        assert_eq!(
            infer_domain(&list),
            Some(AxiomDomain::List(Box::new(AxiomDomain::PositiveInteger)))
        );
    }

    #[test]
    fn heterogeneous_list_has_no_inferred_domain() {
        let list = apply(sym(symbolic_ir::LIST), vec![int(1), flt(1.0)]);
        assert_eq!(infer_domain(&list), None);
    }

    #[test]
    fn empty_list_has_no_inferred_domain() {
        let list = apply(sym(symbolic_ir::LIST), vec![]);
        assert_eq!(infer_domain(&list), None);
    }

    // --- print_axiom -----------------------------------------------------

    #[test]
    fn atoms_print_bare() {
        assert_eq!(print_axiom(&int(42)), "42");
        assert_eq!(print_axiom(&flt(1.5)), "1.5");
        assert_eq!(print_axiom(&rat(1, 3)), "1/3");
        assert_eq!(print_axiom(&sym("x")), "x");
        assert_eq!(print_axiom(&str_node("hi")), "\"hi\"");
    }

    #[test]
    fn booleans_print_lowercase() {
        assert_eq!(print_axiom(&sym("True")), "true");
        assert_eq!(print_axiom(&sym("False")), "false");
    }

    #[test]
    fn arithmetic_prints_infix() {
        assert_eq!(
            print_axiom(&apply(sym(symbolic_ir::ADD), vec![sym("x"), int(1)])),
            "x + 1"
        );
        assert_eq!(
            print_axiom(&apply(sym(symbolic_ir::MUL), vec![sym("x"), int(2)])),
            "x*2"
        );
    }

    #[test]
    fn precedence_forces_parens_on_the_looser_child() {
        let e = apply(
            sym(symbolic_ir::MUL),
            vec![
                apply(sym(symbolic_ir::ADD), vec![sym("a"), sym("b")]),
                sym("c"),
            ],
        );
        assert_eq!(print_axiom(&e), "(a + b)*c");
    }

    #[test]
    fn power_prints_caret_right_associative() {
        let e = apply(
            sym(symbolic_ir::POW),
            vec![
                sym("a"),
                apply(sym(symbolic_ir::POW), vec![sym("b"), sym("c")]),
            ],
        );
        assert_eq!(print_axiom(&e), "a^b^c");
    }

    #[test]
    fn negation_prints_prefix() {
        assert_eq!(
            print_axiom(&apply(sym(symbolic_ir::NEG), vec![sym("x")])),
            "-x"
        );
    }

    #[test]
    fn not_equal_prints_the_axiom_spelling() {
        assert_eq!(
            print_axiom(&apply(sym(symbolic_ir::NOT_EQUAL), vec![sym("a"), sym("b")])),
            "a ~= b"
        );
    }

    #[test]
    fn list_prints_bracketed() {
        assert_eq!(
            print_axiom(&apply(sym(symbolic_ir::LIST), vec![int(1), int(2), int(3)])),
            "[1, 2, 3]"
        );
        assert_eq!(print_axiom(&apply(sym(symbolic_ir::LIST), vec![])), "[]");
    }

    #[test]
    fn user_function_call_prints_as_typed() {
        assert_eq!(
            print_axiom(&apply(sym("f"), vec![sym("x"), sym("y")])),
            "f(x, y)"
        );
    }
}
