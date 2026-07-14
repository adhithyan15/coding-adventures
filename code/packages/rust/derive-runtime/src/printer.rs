//! # Pretty-printing — `symbolic-ir` → Derive surface notation
//!
//! After evaluation the result is a [`symbolic_ir::IRNode`] whose heads are
//! the IR's canonical names (`Add`, `Sin`, `D`, …). A Derive user expects to
//! read it back in Derive's own notation — infix `+`/`*`/`^`, `AND`/`OR`,
//! `F(…)` application — not the IR's default `Add(2, 3)` debug form. This
//! module is the inverse of [`crate::lower`]: it walks the result tree and
//! renders the surface string, reversing the same uppercase head-name bridge
//! [`crate::lower`] applies going in.
//!
//! It is deliberately a **presentation** layer, not a re-parse: the output
//! only needs to be readable and round-trippable for the heads this subset
//! actually produces. Precedence is handled by parenthesising a child
//! whenever its operator binds *looser* than the parent's.
//!
//! ## Precedence ladder (loosest → tightest, mirrors MA07 §3 exactly)
//!
//! | Level | Constructs |
//! |-------|------------|
//! | 0 | (unevaluated `Assign`/`Define` — printed via the generic call form) |
//! | 1 | `Or` |
//! | 2 | `And` |
//! | 3 | `Not` (prefix) / comparisons `=` `<=` `<` `>` `>=` |
//! | 4 | `Add` `Sub` |
//! | 5 | `Mul` `Div` |
//! | 6 | unary `Neg` |
//! | 7 | `Pow` |
//! | 8 | atoms, `F(…)` |

use symbolic_ir::{
    IRApply, IRNode, ACOS, ACOSH, ADD, AND, ASIN, ASINH, ATAN, ATANH, COS, COSH, COTH, CSCH, D,
    DIV, EQUAL, EXP, GREATER, GREATER_EQUAL, IF, INTEGRATE, LESS, LESS_EQUAL, LIST, LOG, MUL, NEG,
    NOT, OR, POW, SECH, SIN, SINH, SQRT, SUB, TAN, TANH,
};

const PREC_LOWEST: u8 = 0;
const PREC_OR: u8 = 1;
const PREC_AND: u8 = 2;
const PREC_NOT_CMP: u8 = 3;
const PREC_ADD: u8 = 4;
const PREC_MUL: u8 = 5;
const PREC_NEG: u8 = 6;
const PREC_POW: u8 = 7;
const PREC_ATOM: u8 = 8;

/// Render `node` as a Derive surface string.
pub fn print_derive(node: &IRNode) -> String {
    print_at(node, PREC_LOWEST)
}

/// Render `node`, wrapping it in parentheses if its own precedence is looser
/// than `parent_prec` (so the surface string re-parses to the same tree).
fn print_at(node: &IRNode, parent_prec: u8) -> String {
    let (text, prec) = render(node);
    if prec < parent_prec {
        format!("({text})")
    } else {
        text
    }
}

/// Render a node, returning its surface text and its own precedence level.
fn render(node: &IRNode) -> (String, u8) {
    match node {
        IRNode::Integer(n) => (n.to_string(), PREC_ATOM),
        IRNode::Float(v) => (format!("{v:?}"), PREC_ATOM),
        IRNode::Rational(n, d) => (format!("{n}/{d}"), PREC_MUL),
        IRNode::Str(s) => (format!("\"{s}\""), PREC_ATOM),
        IRNode::Symbol(s) => (s.clone(), PREC_ATOM),
        IRNode::Apply(app) => render_apply(app),
    }
}

/// Render a compound `head(args)` node.
fn render_apply(app: &IRApply) -> (String, u8) {
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
        // n-ary associative logic: And/Or flatten to a chain (mirrors how
        // `lower_logical_chain` folds a homogeneous run flat rather than
        // pairwise).
        if let Some((op, prec)) = nary_logic(name) {
            if args.len() >= 2 {
                let parts: Vec<String> = args.iter().map(|a| print_at(a, prec)).collect();
                return (parts.join(op), prec);
            }
            if args.len() == 1 {
                return render(&args[0]);
            }
        }
        match name {
            POW if args.len() == 2 => {
                // Right-associative and tighter than unary minus.
                let base = print_at(&args[0], PREC_POW + 1);
                let exp = print_at(&args[1], PREC_NEG);
                return (format!("{base}^{exp}"), PREC_POW);
            }
            NEG if args.len() == 1 => {
                let inner = print_at(&args[0], PREC_NEG);
                return (format!("-{inner}"), PREC_NEG);
            }
            NOT if args.len() == 1 => {
                let inner = print_at(&args[0], PREC_NOT_CMP);
                return (format!("NOT {inner}"), PREC_NOT_CMP);
            }
            LIST => return (render_list(args), PREC_ATOM),
            _ => {}
        }

        // Ordinary function application: `head(args…)`, bridging the
        // canonical IR head back to Derive's uppercase surface spelling
        // (the reverse of `crate::lower::canonical_head`); an unrecognised
        // head (a user-defined function) is rendered as-typed.
        let surface = ir_head_to_surface(name).unwrap_or(name);
        let parts: Vec<String> = args.iter().map(print_derive).collect();
        return (format!("{surface}({})", parts.join(", ")), PREC_ATOM);
    }

    // A computed (non-symbol) head — render generically as `(head)(args…)`.
    let head_text = print_at(&app.head, PREC_ATOM);
    let parts: Vec<String> = args.iter().map(print_derive).collect();
    (format!("{head_text}({})", parts.join(", ")), PREC_ATOM)
}

/// Binary infix arithmetic/comparison operators — `(surface, precedence)`.
fn infix_binary(name: &str) -> Option<(&'static str, u8)> {
    Some(match name {
        ADD => (" + ", PREC_ADD),
        SUB => (" - ", PREC_ADD),
        MUL => ("*", PREC_MUL),
        DIV => ("/", PREC_MUL),
        EQUAL => (" = ", PREC_NOT_CMP),
        LESS => (" < ", PREC_NOT_CMP),
        GREATER => (" > ", PREC_NOT_CMP),
        LESS_EQUAL => (" <= ", PREC_NOT_CMP),
        GREATER_EQUAL => (" >= ", PREC_NOT_CMP),
        _ => return None,
    })
}

/// n-ary logic operators — `(join-text, precedence)`.
fn nary_logic(name: &str) -> Option<(&'static str, u8)> {
    Some(match name {
        AND => (" AND ", PREC_AND),
        OR => (" OR ", PREC_OR),
        _ => return None,
    })
}

/// Render a `List(...)` node back to Derive's bracket syntax (D-5) — the
/// exact reverse of [`crate::lower::lower_vector`]. A matrix
/// (`List(List(row1…), List(row2…), …)`, every element itself a `List`)
/// prints with `;`-separated rows; a flat vector (`List(elem…)`) prints
/// with `,`. `lower_vector` never produces a *mixed* shape (some elements
/// `List`, some not), so this "all-or-nothing" check is unambiguous for
/// anything this crate's own lowering can produce.
fn render_list(args: &[IRNode]) -> String {
    if !args.is_empty() && args.iter().all(is_list_node) {
        let rows: Vec<String> = args
            .iter()
            .map(|row| match row {
                IRNode::Apply(row_app) => row_app
                    .args
                    .iter()
                    .map(print_derive)
                    .collect::<Vec<_>>()
                    .join(", "),
                _ => unreachable!("is_list_node guarantees an Apply"),
            })
            .collect();
        format!("[{}]", rows.join("; "))
    } else {
        let parts: Vec<String> = args.iter().map(print_derive).collect();
        format!("[{}]", parts.join(", "))
    }
}

/// True if `node` is itself a `List(...)` apply.
fn is_list_node(node: &IRNode) -> bool {
    matches!(node, IRNode::Apply(app) if matches!(&app.head, IRNode::Symbol(s) if s == LIST))
}

/// The IR→surface head dictionary — the exact reverse of
/// [`crate::lower`]'s `surface_head_to_ir`, kept as its own table (rather
/// than derived at runtime) so each direction reads as a simple, directly
/// checkable list.
fn ir_head_to_surface(name: &str) -> Option<&'static str> {
    Some(match name {
        D => "DIF",
        INTEGRATE => "INT",
        IF => "IF",
        SIN => "SIN",
        COS => "COS",
        TAN => "TAN",
        SQRT => "SQRT",
        EXP => "EXP",
        LOG => "LOG",
        ATAN => "ATAN",
        ASIN => "ASIN",
        ACOS => "ACOS",
        SINH => "SINH",
        COSH => "COSH",
        TANH => "TANH",
        ASINH => "ASINH",
        ACOSH => "ACOSH",
        ATANH => "ATANH",
        COTH => "COTH",
        SECH => "SECH",
        CSCH => "CSCH",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use symbolic_ir::{apply, flt, int, rat, sym};

    #[test]
    fn atoms_print_bare() {
        assert_eq!(print_derive(&int(42)), "42");
        assert_eq!(print_derive(&flt(1.5)), "1.5");
        assert_eq!(print_derive(&rat(1, 3)), "1/3");
        assert_eq!(print_derive(&sym("x")), "x");
    }

    #[test]
    fn arithmetic_prints_infix() {
        assert_eq!(
            print_derive(&apply(sym(ADD), vec![sym("x"), int(1)])),
            "x + 1"
        );
        assert_eq!(
            print_derive(&apply(sym(MUL), vec![sym("x"), int(2)])),
            "x*2"
        );
    }

    #[test]
    fn precedence_forces_parens_on_the_looser_child() {
        // Mul(Add(a, b), c)  ->  "(a + b)*c" — never the ambiguous "a + b*c".
        let e = apply(
            sym(MUL),
            vec![apply(sym(ADD), vec![sym("a"), sym("b")]), sym("c")],
        );
        assert_eq!(print_derive(&e), "(a + b)*c");
    }

    #[test]
    fn power_prints_caret_right_associative() {
        let e = apply(
            sym(POW),
            vec![sym("a"), apply(sym(POW), vec![sym("b"), sym("c")])],
        );
        assert_eq!(print_derive(&e), "a^b^c");
    }

    #[test]
    fn negation_and_not_print_prefix() {
        assert_eq!(print_derive(&apply(sym(NEG), vec![sym("x")])), "-x");
        assert_eq!(print_derive(&apply(sym(NOT), vec![sym("a")])), "NOT a");
    }

    #[test]
    fn logic_chain_prints_flat() {
        let e = apply(sym(OR), vec![sym("a"), sym("b"), sym("c")]);
        assert_eq!(print_derive(&e), "a OR b OR c");
    }

    #[test]
    fn comparisons_print_derive_spelling() {
        assert_eq!(
            print_derive(&apply(sym(EQUAL), vec![sym("x"), int(4)])),
            "x = 4"
        );
        assert_eq!(
            print_derive(&apply(sym(LESS_EQUAL), vec![sym("a"), sym("b")])),
            "a <= b"
        );
    }

    #[test]
    fn builtin_calls_print_bridged_back_to_uppercase() {
        assert_eq!(
            print_derive(&apply(sym(D), vec![sym("u"), sym("x")])),
            "DIF(u, x)"
        );
        assert_eq!(print_derive(&apply(sym(SIN), vec![sym("x")])), "SIN(x)");
    }

    #[test]
    fn user_function_call_prints_as_typed() {
        assert_eq!(
            print_derive(&apply(sym("F"), vec![sym("x"), sym("y")])),
            "F(x, y)"
        );
    }

    #[test]
    fn flat_list_prints_as_a_vector() {
        assert_eq!(
            print_derive(&apply(sym(LIST), vec![int(1), int(2), int(3)])),
            "[1, 2, 3]"
        );
    }

    #[test]
    fn list_of_lists_prints_as_a_matrix() {
        let e = apply(
            sym(LIST),
            vec![
                apply(sym(LIST), vec![int(1), int(2)]),
                apply(sym(LIST), vec![int(3), int(4)]),
            ],
        );
        assert_eq!(print_derive(&e), "[1, 2; 3, 4]");
    }

    #[test]
    fn empty_list_prints_as_empty_brackets() {
        assert_eq!(print_derive(&apply(sym(LIST), vec![])), "[]");
    }

    #[test]
    fn list_of_expressions_prints_each_element() {
        let e = apply(
            sym(LIST),
            vec![apply(sym(ADD), vec![sym("x"), int(1)]), sym("y")],
        );
        assert_eq!(print_derive(&e), "[x + 1, y]");
    }
}
