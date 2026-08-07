//! # Pretty-printing — `symbolic-ir` → Reduce surface notation
//!
//! After evaluation the result is a [`symbolic_ir::IRNode`] whose heads are
//! the IR's canonical names (`Add`, `Equal`, `List`, …). A Reduce user
//! expects to read it back in Reduce's own notation — infix `+`/`*`/`^`,
//! `and`/`or`/`not`, curly-brace `{a, b, c}` lists, `f(…)` application — not
//! the IR's default `Add(2, 3)` debug form. This module is the inverse of
//! [`crate::lower`]: it walks the result tree and renders the surface
//! string, reversing the same head bridges `crate::lower` applies going in
//! (mirroring `derive-runtime::printer`'s identical role and shape).
//!
//! It is deliberately a **presentation** layer, not a re-parse: the output
//! only needs to be readable and round-trippable for the heads this subset
//! actually produces. Precedence is handled by parenthesising a child
//! whenever its operator binds *looser* than the parent's.
//!
//! ## Precedence ladder (loosest → tightest, mirrors `reduce.grammar`'s own
//! cascade — MA08 §3 — one tier more than `derive-runtime::printer`'s table,
//! for the `cons` (`.`) tier Derive has no analogue of)
//!
//! | Level | Constructs |
//! |-------|------------|
//! | — | (`if`/`<< ... >>` are self-delimited by their own keywords/`<<`/`>>`,
//! |   | so they never need surrounding parens — treated as atom-level) |
//! | 0 | (unevaluated `Assign`/`Define` never reach the printer — see below) |
//! | 1 | `or` |
//! | 2 | `and` |
//! | 3 | `not` (prefix) / comparisons `=` `neq` `<` `<=` `>` `>=` |
//! | 4 | `Cons` (`.`, right-associative — only appears unresolved, MA08 §3) |
//! | 5 | `Add` `Sub` |
//! | 6 | `Mul` `Div` |
//! | 7 | unary `Neg` |
//! | 8 | `Pow` |
//! | 9 | atoms, `f(…)`, `{…}` |
//!
//! `Assign`/`Define` never appear in a printed *result*: `assign_handler`
//! returns the bound *value* (not an `Assign(...)` node), and
//! `define_handler` returns `Symbol(name)` — see `symbolic-vm`'s own
//! `handlers.rs`. So, unlike `derive-runtime::printer` (whose precedence
//! table reserves a level for them anyway, for documentation symmetry),
//! this module has no rendering case for either head at all.

use symbolic_ir::{
    IRApply, IRNode, ADD, AND, DIV, EQUAL, GREATER, GREATER_EQUAL, IF, LESS, LESS_EQUAL, LIST, MUL,
    NEG, NOT, NOT_EQUAL, OR, POW, SUB,
};

use crate::lower::{APPEND, COMPOUND_EXPRESSION, CONS, FIRST, PART, REST, REVERSE, SECOND, THIRD};

const PREC_LOWEST: u8 = 0;
const PREC_OR: u8 = 1;
const PREC_AND: u8 = 2;
const PREC_NOT_CMP: u8 = 3;
const PREC_CONS: u8 = 4;
const PREC_ADD: u8 = 5;
const PREC_MUL: u8 = 6;
const PREC_NEG: u8 = 7;
const PREC_POW: u8 = 8;
const PREC_ATOM: u8 = 9;

/// Render `node` as a Reduce surface string.
pub fn print_reduce(node: &IRNode) -> String {
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
                return (format!("not {inner}"), PREC_NOT_CMP);
            }
            LIST => return (render_list(args), PREC_ATOM),
            IF if args.len() == 2 => {
                return (
                    format!(
                        "if {} then {}",
                        print_reduce(&args[0]),
                        print_reduce(&args[1])
                    ),
                    PREC_ATOM,
                )
            }
            IF if args.len() == 3 => {
                return (
                    format!(
                        "if {} then {} else {}",
                        print_reduce(&args[0]),
                        print_reduce(&args[1]),
                        print_reduce(&args[2])
                    ),
                    PREC_ATOM,
                )
            }
            // A group statement left unevaluated (MA08 §3's "last statement's
            // value" collapse has no handler in the shared table yet — see
            // `crate::lower`'s module doc comment). Rendered back with its
            // real `<< s1; s2; ... >>` surface syntax, an honest reflection
            // of what actually survived evaluation (each `s_i` already fully
            // evaluated, just not collapsed to the last one).
            COMPOUND_EXPRESSION if !args.is_empty() => {
                let parts: Vec<String> = args.iter().map(print_reduce).collect();
                return (format!("<< {} >>", parts.join("; ")), PREC_ATOM);
            }
            _ => {}
        }

        // Ordinary function application: `head(args…)`, bridging the
        // canonical IR head back to Reduce's lowercase surface spelling
        // (the reverse of `crate::lower::canonical_head`); an unrecognised
        // head (a user-defined operator/procedure, or one of the
        // no-handler-yet heads like `Cons`/`First`/… left structurally
        // unevaluated) is rendered as-typed/bridged-back but otherwise
        // unchanged.
        let surface = ir_head_to_surface(name).unwrap_or(name);
        let parts: Vec<String> = args.iter().map(print_reduce).collect();
        return (format!("{surface}({})", parts.join(", ")), PREC_ATOM);
    }

    // A computed (non-symbol) head — render generically as `(head)(args…)`.
    let head_text = print_at(&app.head, PREC_ATOM);
    let parts: Vec<String> = args.iter().map(print_reduce).collect();
    (format!("{head_text}({})", parts.join(", ")), PREC_ATOM)
}

/// Binary infix arithmetic/comparison/cons operators — `(surface,
/// precedence)`. `Cons` only ever reaches here when its right-hand side
/// wasn't structurally a literal `List` at lowering time (see
/// `crate::lower::fold_cons`) — the one case MA08 §3 leaves as a bare,
/// unevaluated `Cons[a, b]` application (no handler in the shared table).
fn infix_binary(name: &str) -> Option<(&'static str, u8)> {
    Some(match name {
        ADD => (" + ", PREC_ADD),
        SUB => (" - ", PREC_ADD),
        MUL => ("*", PREC_MUL),
        DIV => ("/", PREC_MUL),
        EQUAL => (" = ", PREC_NOT_CMP),
        NOT_EQUAL => (" neq ", PREC_NOT_CMP),
        LESS => (" < ", PREC_NOT_CMP),
        GREATER => (" > ", PREC_NOT_CMP),
        LESS_EQUAL => (" <= ", PREC_NOT_CMP),
        GREATER_EQUAL => (" >= ", PREC_NOT_CMP),
        CONS => (" . ", PREC_CONS),
        _ => return None,
    })
}

/// n-ary logic operators — `(join-text, precedence)`.
fn nary_logic(name: &str) -> Option<(&'static str, u8)> {
    Some(match name {
        AND => (" and ", PREC_AND),
        OR => (" or ", PREC_OR),
        _ => return None,
    })
}

/// Render a `List(...)` node back to Reduce's curly-brace syntax — the
/// exact reverse of `crate::lower::lower_list_literal`. Unlike
/// `derive-runtime::printer::render_list`, there is no row/matrix
/// distinction to make: Reduce's list is always flat (matrices are out of
/// scope, MA08 §4), so every element just prints comma-separated.
fn render_list(args: &[IRNode]) -> String {
    let parts: Vec<String> = args.iter().map(print_reduce).collect();
    format!("{{{}}}", parts.join(", "))
}

/// The IR→surface head dictionary — the exact reverse of
/// [`crate::lower`]'s `surface_head_to_ir`, kept as its own table (rather
/// than derived at runtime) so each direction reads as a simple, directly
/// checkable list. `List` is handled specially above (curly-brace syntax,
/// never the generic call form), so it has no entry here.
fn ir_head_to_surface(name: &str) -> Option<&'static str> {
    Some(match name {
        FIRST => "first",
        SECOND => "second",
        THIRD => "third",
        REST => "rest",
        PART => "part",
        APPEND => "append",
        REVERSE => "reverse",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use symbolic_ir::{apply, flt, int, rat, sym};

    #[test]
    fn atoms_print_bare() {
        assert_eq!(print_reduce(&int(42)), "42");
        assert_eq!(print_reduce(&flt(1.5)), "1.5");
        assert_eq!(print_reduce(&rat(1, 3)), "1/3");
        assert_eq!(print_reduce(&sym("x")), "x");
    }

    #[test]
    fn arithmetic_prints_infix() {
        assert_eq!(
            print_reduce(&apply(sym(ADD), vec![sym("x"), int(1)])),
            "x + 1"
        );
        assert_eq!(
            print_reduce(&apply(sym(MUL), vec![sym("x"), int(2)])),
            "x*2"
        );
        assert_eq!(
            print_reduce(&apply(sym(DIV), vec![sym("x"), int(2)])),
            "x/2"
        );
    }

    #[test]
    fn precedence_forces_parens_on_the_looser_child() {
        // Mul(Add(a, b), c)  ->  "(a + b)*c" — never the ambiguous "a + b*c".
        let e = apply(
            sym(MUL),
            vec![apply(sym(ADD), vec![sym("a"), sym("b")]), sym("c")],
        );
        assert_eq!(print_reduce(&e), "(a + b)*c");
    }

    #[test]
    fn power_prints_caret_right_associative() {
        let e = apply(
            sym(POW),
            vec![sym("a"), apply(sym(POW), vec![sym("b"), sym("c")])],
        );
        assert_eq!(print_reduce(&e), "a^b^c");
    }

    #[test]
    fn negation_and_not_print_prefix() {
        assert_eq!(print_reduce(&apply(sym(NEG), vec![sym("x")])), "-x");
        assert_eq!(print_reduce(&apply(sym(NOT), vec![sym("a")])), "not a");
    }

    #[test]
    fn logic_chain_prints_flat() {
        let e = apply(sym(OR), vec![sym("a"), sym("b"), sym("c")]);
        assert_eq!(print_reduce(&e), "a or b or c");
    }

    #[test]
    fn comparisons_print_reduce_spelling() {
        assert_eq!(
            print_reduce(&apply(sym(EQUAL), vec![sym("x"), int(4)])),
            "x = 4"
        );
        assert_eq!(
            print_reduce(&apply(sym(NOT_EQUAL), vec![sym("x"), int(4)])),
            "x neq 4"
        );
        assert_eq!(
            print_reduce(&apply(sym(LESS_EQUAL), vec![sym("a"), sym("b")])),
            "a <= b"
        );
    }

    #[test]
    fn user_function_call_prints_as_typed() {
        assert_eq!(
            print_reduce(&apply(sym("f"), vec![sym("x"), sym("y")])),
            "f(x, y)"
        );
    }

    #[test]
    fn flat_list_prints_with_curly_braces() {
        assert_eq!(
            print_reduce(&apply(sym(LIST), vec![int(1), int(2), int(3)])),
            "{1, 2, 3}"
        );
    }

    #[test]
    fn empty_list_prints_as_empty_braces() {
        assert_eq!(print_reduce(&apply(sym(LIST), vec![])), "{}");
    }

    #[test]
    fn list_of_expressions_prints_each_element() {
        let e = apply(
            sym(LIST),
            vec![apply(sym(ADD), vec![sym("x"), int(1)]), sym("y")],
        );
        assert_eq!(print_reduce(&e), "{x + 1, y}");
    }

    #[test]
    fn list_accessor_calls_print_bridged_back_to_lowercase() {
        assert_eq!(print_reduce(&apply(sym(FIRST), vec![sym("l")])), "first(l)");
        assert_eq!(print_reduce(&apply(sym(REST), vec![sym("l")])), "rest(l)");
        assert_eq!(
            print_reduce(&apply(sym(APPEND), vec![sym("l1"), sym("l2")])),
            "append(l1, l2)"
        );
        assert_eq!(
            print_reduce(&apply(sym(REVERSE), vec![sym("l")])),
            "reverse(l)"
        );
        assert_eq!(
            print_reduce(&apply(sym(PART), vec![sym("l"), int(2)])),
            "part(l, 2)"
        );
    }

    #[test]
    fn unresolved_cons_prints_the_dot_operator() {
        assert_eq!(
            print_reduce(&apply(sym(CONS), vec![sym("a"), sym("b")])),
            "a . b"
        );
    }

    #[test]
    fn unresolved_if_prints_if_then_else() {
        assert_eq!(
            print_reduce(&apply(sym(IF), vec![sym("a"), int(1), int(2)])),
            "if a then 1 else 2"
        );
        assert_eq!(
            print_reduce(&apply(sym(IF), vec![sym("a"), int(1)])),
            "if a then 1"
        );
    }

    #[test]
    fn unresolved_compound_expression_prints_group_syntax() {
        assert_eq!(
            print_reduce(&apply(sym(COMPOUND_EXPRESSION), vec![int(1), int(2)])),
            "<< 1; 2 >>"
        );
    }
}
