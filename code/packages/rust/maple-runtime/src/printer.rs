//! # Pretty-printing — `symbolic-ir` → Maple surface notation
//!
//! After evaluation the result is a [`symbolic_ir::IRNode`] whose heads are
//! the IR's canonical names (`Add`, `Equal`, `List`, [`crate::lower::SET`],
//! …). A Maple user expects to read it back in Maple's own notation — infix
//! `+`/`*`/`^`, `and`/`or`/`not`, square-bracket `[a, b, c]` lists,
//! curly-brace `{a, b, c}` sets, `f(…)` application, `if ... then ... [elif
//! ... then ...] [else ...] end if`. This module is the inverse of
//! [`crate::lower`]: it walks the result tree and renders the surface
//! string, reversing the same head bridges `crate::lower` applies going in
//! (mirroring `reduce-runtime::printer`'s identical role and shape).
//!
//! It is deliberately a **presentation** layer, not a re-parse: the output
//! only needs to be readable and round-trippable for the heads this subset
//! actually produces. Precedence is handled by parenthesising a child
//! whenever its operator binds *looser* than the parent's.
//!
//! ## Precedence ladder (loosest → tightest, mirrors `maple.grammar`'s own
//! cascade — MA09 §3)
//!
//! | Level | Constructs |
//! |-------|------------|
//! | — | (`if`/`elif`/`else`/`end if` is self-delimited by its own keywords, |
//! |   | so it never needs surrounding parens — treated as atom-level; an |
//! |   | `Assign`/`Define` result never reaches the printer at all, see below) |
//! | 1 | `or` |
//! | 2 | `and` |
//! | 3 | `not` (prefix) / comparisons `=` `<>` `<` `<=` `>` `>=` |
//! | 4 | `Add` `Sub` |
//! | 5 | `Mul` `Div` |
//! | 6 | unary `Neg` |
//! | 7 | `Pow` |
//! | 8 | atoms, `f(…)`, `[…]`, `{…}` |
//!
//! `Assign`/`Define` never appear in a printed *result*: `assign_handler`
//! returns the bound *value* (not an `Assign(...)` node), and
//! `define_handler` returns `Symbol(name)` — see `symbolic-vm`'s own
//! `handlers.rs`. So, exactly like `reduce-runtime::printer`, this module
//! has no rendering case for either head.
//!
//! ## `If` CAN still reach the printer, even though it's a *statement*
//!
//! Maple's `if_expr` is never usable as a *nested* expression (MA09/
//! `maple.grammar`'s own "statements vs. expressions" design decision), but
//! it is still a legitimate **top-level statement** whose evaluated result
//! gets displayed — and `If` is a held head (`symbolic_vm::backend::
//! BaseBackend::new`), so when the condition doesn't resolve to `True`/
//! `False` (a free variable), `if_handler` rebuilds and returns the
//! unevaluated `If(...)` node itself. [`render_if`] reconstructs Maple's own
//! `if ... then ... [elif ... then ...] [else ...] end if` surface —
//! folding a nested `If` in the else-slot back into an `elif` continuation
//! (the exact reverse of [`crate::lower::lower_if`]'s own right-fold),
//! rather than printing a redundant, less-readable `else if ... end if end
//! if` — a deliberate presentation choice, not a re-parse requirement.

use symbolic_ir::{
    IRApply, IRNode, ADD, AND, DIV, D, EQUAL, GREATER, GREATER_EQUAL, IF, INTEGRATE, LESS,
    LESS_EQUAL, LIST, MUL, NEG, NOT, NOT_EQUAL, OR, POW, SUB,
};

use crate::lower::SET;

const PREC_LOWEST: u8 = 0;
const PREC_OR: u8 = 1;
const PREC_AND: u8 = 2;
const PREC_NOT_CMP: u8 = 3;
const PREC_ADD: u8 = 4;
const PREC_MUL: u8 = 5;
const PREC_NEG: u8 = 6;
const PREC_POW: u8 = 7;
const PREC_ATOM: u8 = 8;

/// Render `node` as a Maple surface string.
pub fn print_maple(node: &IRNode) -> String {
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
        IRNode::Symbol(s) if s == "True" => ("true".to_string(), PREC_ATOM),
        IRNode::Symbol(s) if s == "False" => ("false".to_string(), PREC_ATOM),
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
            SET => return (render_set(args), PREC_ATOM),
            IF if args.len() == 2 || args.len() == 3 => return (render_if(args), PREC_ATOM),
            _ => {}
        }

        // Ordinary function application: `head(args…)`, bridging the
        // canonical IR head back to Maple's surface spelling (the reverse
        // of `crate::lower::canonical_head`); an unrecognised head (a
        // user-defined function, or one of MA09 §4's deferred `cas-*`
        // surface) is rendered as-typed/bridged-back but otherwise
        // unchanged.
        let surface = ir_head_to_surface(name).unwrap_or(name);
        let parts: Vec<String> = args.iter().map(print_maple).collect();
        return (format!("{surface}({})", parts.join(", ")), PREC_ATOM);
    }

    // A computed (non-symbol) head — render generically as `(head)(args…)`.
    let head_text = print_at(&app.head, PREC_ATOM);
    let parts: Vec<String> = args.iter().map(print_maple).collect();
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
        NOT_EQUAL => (" <> ", PREC_NOT_CMP),
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
        AND => (" and ", PREC_AND),
        OR => (" or ", PREC_OR),
        _ => return None,
    })
}

/// Render a `List(...)` node back to Maple's square-bracket syntax — the
/// exact reverse of `crate::lower::lower_list_literal`.
fn render_list(args: &[IRNode]) -> String {
    let parts: Vec<String> = args.iter().map(print_maple).collect();
    format!("[{}]", parts.join(", "))
}

/// Render a `Set(...)` node back to Maple's curly-brace syntax — the exact
/// reverse of `crate::lower::lower_set_literal`. Note this reflects
/// whatever arguments the (unresolved, since no shared handler exists yet —
/// see `crate::lower`'s own "Set" doc section) `Set` call was left holding:
/// duplicates are NOT removed and order is NOT normalised, since real
/// Maple's unordered/deduplicating set semantics aren't enforced at
/// evaluation time in this subset.
fn render_set(args: &[IRNode]) -> String {
    let parts: Vec<String> = args.iter().map(print_maple).collect();
    format!("{{{}}}", parts.join(", "))
}

/// Render an unresolved `If(cond, then[, else])` node back to Maple's
/// `if ... then ... [elif ... then ...] [else ...] end if` surface — see
/// this module's own doc comment's "`If` CAN still reach the printer"
/// section. When the else-slot is itself another `If(...)` application
/// (MA09 §3's own elif-desugaring, reversed), this folds it into an `elif`
/// continuation instead of nesting a second, redundant `if ... end if`.
fn render_if(args: &[IRNode]) -> String {
    let mut out = String::from("if ");
    out.push_str(&print_maple(&args[0]));
    out.push_str(" then ");
    out.push_str(&print_maple(&args[1]));

    let mut tail: Option<&IRNode> = args.get(2);
    while let Some(node) = tail {
        if let IRNode::Apply(app) = node {
            let is_if = matches!(&app.head, IRNode::Symbol(s) if s == IF);
            if is_if && (app.args.len() == 2 || app.args.len() == 3) {
                out.push_str(" elif ");
                out.push_str(&print_maple(&app.args[0]));
                out.push_str(" then ");
                out.push_str(&print_maple(&app.args[1]));
                tail = app.args.get(2);
                continue;
            }
        }
        out.push_str(" else ");
        out.push_str(&print_maple(node));
        break;
    }
    out.push_str(" end if");
    out
}

/// The IR→surface head dictionary — the exact reverse of
/// `crate::lower::surface_head_to_ir`. Kept as its own table (rather than
/// derived at runtime) so each direction reads as a simple, directly
/// checkable list. `List`/`Set` are handled specially above (bracket
/// syntax, never the generic call form), so neither has an entry here.
fn ir_head_to_surface(name: &str) -> Option<&'static str> {
    Some(match name {
        D => "diff",
        INTEGRATE => "int",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use symbolic_ir::{apply, flt, int, rat, sym};

    #[test]
    fn atoms_print_bare() {
        assert_eq!(print_maple(&int(42)), "42");
        assert_eq!(print_maple(&flt(1.5)), "1.5");
        assert_eq!(print_maple(&rat(1, 3)), "1/3");
        assert_eq!(print_maple(&sym("x")), "x");
    }

    #[test]
    fn true_false_symbols_print_lowercase() {
        assert_eq!(print_maple(&sym("True")), "true");
        assert_eq!(print_maple(&sym("False")), "false");
    }

    #[test]
    fn arithmetic_prints_infix() {
        assert_eq!(
            print_maple(&apply(sym(ADD), vec![sym("x"), int(1)])),
            "x + 1"
        );
        assert_eq!(
            print_maple(&apply(sym(MUL), vec![sym("x"), int(2)])),
            "x*2"
        );
        assert_eq!(
            print_maple(&apply(sym(DIV), vec![sym("x"), int(2)])),
            "x/2"
        );
    }

    #[test]
    fn precedence_forces_parens_on_the_looser_child() {
        let e = apply(
            sym(MUL),
            vec![apply(sym(ADD), vec![sym("a"), sym("b")]), sym("c")],
        );
        assert_eq!(print_maple(&e), "(a + b)*c");
    }

    #[test]
    fn power_prints_caret_right_associative() {
        let e = apply(
            sym(POW),
            vec![sym("a"), apply(sym(POW), vec![sym("b"), sym("c")])],
        );
        assert_eq!(print_maple(&e), "a^b^c");
    }

    #[test]
    fn negation_and_not_print_prefix() {
        assert_eq!(print_maple(&apply(sym(NEG), vec![sym("x")])), "-x");
        assert_eq!(print_maple(&apply(sym(NOT), vec![sym("a")])), "not a");
    }

    #[test]
    fn logic_chain_prints_flat() {
        let e = apply(sym(OR), vec![sym("a"), sym("b"), sym("c")]);
        assert_eq!(print_maple(&e), "a or b or c");
    }

    #[test]
    fn comparisons_print_maple_spelling() {
        assert_eq!(
            print_maple(&apply(sym(EQUAL), vec![sym("x"), int(4)])),
            "x = 4"
        );
        assert_eq!(
            print_maple(&apply(sym(NOT_EQUAL), vec![sym("x"), int(4)])),
            "x <> 4"
        );
        assert_eq!(
            print_maple(&apply(sym(LESS_EQUAL), vec![sym("a"), sym("b")])),
            "a <= b"
        );
    }

    #[test]
    fn user_function_call_prints_as_typed() {
        assert_eq!(
            print_maple(&apply(sym("f"), vec![sym("x"), sym("y")])),
            "f(x, y)"
        );
    }

    #[test]
    fn list_prints_with_square_brackets() {
        assert_eq!(
            print_maple(&apply(sym(LIST), vec![int(1), int(2), int(3)])),
            "[1, 2, 3]"
        );
    }

    #[test]
    fn empty_list_prints_as_empty_brackets() {
        assert_eq!(print_maple(&apply(sym(LIST), vec![])), "[]");
    }

    #[test]
    fn set_prints_with_curly_braces() {
        assert_eq!(
            print_maple(&apply(sym(SET), vec![int(1), int(2), int(3)])),
            "{1, 2, 3}"
        );
    }

    #[test]
    fn empty_set_prints_as_empty_braces() {
        assert_eq!(print_maple(&apply(sym(SET), vec![])), "{}");
    }

    #[test]
    fn list_and_set_print_differently_for_the_same_elements() {
        let list = apply(sym(LIST), vec![sym("a"), sym("b")]);
        let set = apply(sym(SET), vec![sym("a"), sym("b")]);
        assert_eq!(print_maple(&list), "[a, b]");
        assert_eq!(print_maple(&set), "{a, b}");
    }

    #[test]
    fn diff_and_int_bridge_back_to_lowercase_surface_names() {
        assert_eq!(
            print_maple(&apply(sym(D), vec![sym("f"), sym("x")])),
            "diff(f, x)"
        );
        assert_eq!(
            print_maple(&apply(sym(INTEGRATE), vec![sym("f"), sym("x")])),
            "int(f, x)"
        );
    }

    #[test]
    fn unresolved_two_arg_if_prints_if_then_end_if() {
        assert_eq!(
            print_maple(&apply(sym(IF), vec![sym("a"), int(1)])),
            "if a then 1 end if"
        );
    }

    #[test]
    fn unresolved_three_arg_if_prints_if_then_else_end_if() {
        assert_eq!(
            print_maple(&apply(sym(IF), vec![sym("a"), int(1), int(2)])),
            "if a then 1 else 2 end if"
        );
    }

    #[test]
    fn nested_if_in_the_else_slot_folds_back_into_elif() {
        // If(a, 1, If(b, 2, 3)) -> "if a then 1 elif b then 2 else 3 end if"
        let e = apply(
            sym(IF),
            vec![
                sym("a"),
                int(1),
                apply(sym(IF), vec![sym("b"), int(2), int(3)]),
            ],
        );
        assert_eq!(print_maple(&e), "if a then 1 elif b then 2 else 3 end if");
    }

    #[test]
    fn nested_if_with_no_final_else_folds_into_elif_with_no_trailing_else() {
        // If(a, 1, If(b, 2)) -> "if a then 1 elif b then 2 end if"
        let e = apply(
            sym(IF),
            vec![sym("a"), int(1), apply(sym(IF), vec![sym("b"), int(2)])],
        );
        assert_eq!(print_maple(&e), "if a then 1 elif b then 2 end if");
    }
}
