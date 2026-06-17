//! # Pretty-printing — `symbolic-ir` → Wolfram surface notation
//!
//! After evaluation the result is a [`symbolic_ir::IRNode`] whose heads are the
//! IR's canonical names (`Add`, `Mul`, `Pow`, `List`, …). A Wolfram user expects
//! to read it back in *Mathematica* notation — infix `+`/`*`/`^`, `f[…]`
//! application, `{…}` lists — not the IR's default `Add(2, 3)` debug form. This
//! module is the inverse of [`crate::lower`]: it walks the result tree and
//! renders the surface string.
//!
//! It is deliberately a **presentation** layer, not a re-parse: we only need the
//! output to be readable and round-trippable for the common heads. Precedence is
//! handled by parenthesising a child whenever its operator binds *looser* than
//! the parent's, so `Mul(Add(a, b), c)` prints as `(a + b)*c` — never the
//! ambiguous `a + b*c`.
//!
//! ## Precedence ladder (loosest → tightest)
//!
//! | Level | Constructs |
//! |-------|------------|
//! | 0 | `Set` `:=` `Rule` `ReplaceAll` (we print these flat) |
//! | 1 | `Or` |
//! | 2 | `And` |
//! | 3 | comparisons `==` `<` `>` … |
//! | 4 | `Add` `Sub` |
//! | 5 | `Mul` `Div` |
//! | 6 | unary `Neg` |
//! | 7 | `Pow` |
//! | 8 | atoms, `f[…]`, `{…}` |

use crate::lower::REPLACE_ALL;
use cas_pattern_matching::nodes::{
    BLANK, PATTERN, RULE as PM_RULE, RULE_DELAYED as PM_RULE_DELAYED,
};
use symbolic_ir::{
    IRApply, IRNode, ADD, AND, ASSIGN, DEFINE, DIV, EQUAL, GREATER, GREATER_EQUAL, LESS,
    LESS_EQUAL, LIST, MUL, NEG, NOT, NOT_EQUAL, OR, POW, SUB,
};

/// Binding-strength levels used to decide when a child needs parentheses.
const PREC_LOWEST: u8 = 0; // statement-level (Set, Rule, ReplaceAll)
const PREC_OR: u8 = 1;
const PREC_AND: u8 = 2;
const PREC_CMP: u8 = 3;
const PREC_ADD: u8 = 4;
const PREC_MUL: u8 = 5;
const PREC_NEG: u8 = 6;
const PREC_POW: u8 = 7;
const PREC_ATOM: u8 = 8;

/// Render `node` as a Wolfram surface string.
pub fn print_wolfram(node: &IRNode) -> String {
    print_at(node, PREC_LOWEST)
}

/// Render `node`, wrapping it in parentheses if its own precedence is looser than
/// `parent_prec` (so the surface string re-parses to the same tree).
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
        // Use Rust's round-trip float repr, but trim a trailing `.0` so `3.0`
        // reads as the Wolfram `3.` is avoided — keep `3.0` which is clearer.
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
        // Binary infix arithmetic / comparison / logic.
        if let Some((op, prec)) = infix_binary(name) {
            if args.len() == 2 {
                let l = print_at(&args[0], prec);
                // The right operand of a left-assoc op needs the next-tighter
                // level to force `a - (b - c)` to parenthesise; for our printing
                // we keep it simple and use `prec` (correct for the round-trip of
                // the trees our lowering produces, which are already left-folded).
                let r = print_at(&args[1], prec);
                return (format!("{l}{op}{r}"), prec);
            }
        }
        // n-ary associative operators: Add / Mul / And / Or flatten to a chain.
        if let Some((op, prec)) = nary_operator(name) {
            if args.len() >= 2 {
                let parts: Vec<String> = args.iter().map(|a| print_at(a, prec)).collect();
                return (parts.join(op), prec);
            }
            if args.len() == 1 {
                // A degenerate single-arg associative op: just its operand.
                return render(&args[0]);
            }
        }

        match name {
            POW if args.len() == 2 => {
                // Power is right-associative and tighter than unary minus.
                let base = print_at(&args[0], PREC_POW + 1);
                let exp = print_at(&args[1], PREC_NEG);
                return (format!("{base}^{exp}"), PREC_POW);
            }
            NEG if args.len() == 1 => {
                let inner = print_at(&args[0], PREC_NEG);
                return (format!("-{inner}"), PREC_NEG);
            }
            NOT if args.len() == 1 => {
                let inner = print_at(&args[0], PREC_ATOM);
                return (format!("!{inner}"), PREC_NEG);
            }
            LIST => {
                let parts: Vec<String> = args.iter().map(print_wolfram).collect();
                return (format!("{{{}}}", parts.join(", ")), PREC_ATOM);
            }
            ASSIGN if args.len() == 2 => {
                let l = print_wolfram(&args[0]);
                let r = print_wolfram(&args[1]);
                return (format!("{l} = {r}"), PREC_LOWEST);
            }
            DEFINE if args.len() == 3 => {
                // Define(head, List(params), body)  ->  head[params] := body
                let params = if let IRNode::Apply(list) = &args[1] {
                    list.args.iter().map(print_wolfram).collect::<Vec<_>>()
                } else {
                    vec![]
                };
                let body = print_wolfram(&args[2]);
                let head = print_wolfram(&args[0]);
                return (
                    format!("{head}[{}] := {body}", params.join(", ")),
                    PREC_LOWEST,
                );
            }
            PM_RULE if args.len() == 2 => {
                let l = print_wolfram(&args[0]);
                let r = print_wolfram(&args[1]);
                return (format!("{l} -> {r}"), PREC_LOWEST);
            }
            PM_RULE_DELAYED if args.len() == 2 => {
                let l = print_wolfram(&args[0]);
                let r = print_wolfram(&args[1]);
                return (format!("{l} :> {r}"), PREC_LOWEST);
            }
            REPLACE_ALL if args.len() == 2 => {
                let l = print_wolfram(&args[0]);
                let r = print_wolfram(&args[1]);
                return (format!("{l} /. {r}"), PREC_LOWEST);
            }
            BLANK => {
                // Blank()  ->  _   ;  Blank(h)  ->  _h
                let inner = args.first().map(print_wolfram).unwrap_or_default();
                return (format!("_{inner}"), PREC_ATOM);
            }
            PATTERN if args.len() == 2 => {
                // Pattern(x, Blank())  ->  x_   ;  Pattern(x, Blank(h)) -> x_h
                let name = print_wolfram(&args[0]);
                let (inner, _) = render(&args[1]);
                // `inner` already starts with `_` from the Blank rendering.
                return (format!("{name}{inner}"), PREC_ATOM);
            }
            _ => {}
        }
    }

    // Default: a function application `head[arg, …]` (the Wolfram surface uses
    // square brackets). The head itself may be compound (e.g. `(f[x])[y]`).
    let head = print_at(&app.head, PREC_ATOM);
    let parts: Vec<String> = args.iter().map(print_wolfram).collect();
    (format!("{head}[{}]", parts.join(", ")), PREC_ATOM)
}

/// Binary infix operators rendered as `lhs OP rhs` (with surrounding text).
/// Returns the operator string (including spacing) and its precedence level.
fn infix_binary(name: &str) -> Option<(&'static str, u8)> {
    Some(match name {
        SUB => (" - ", PREC_ADD),
        DIV => ("/", PREC_MUL),
        EQUAL => (" == ", PREC_CMP),
        NOT_EQUAL => (" != ", PREC_CMP),
        LESS => (" < ", PREC_CMP),
        GREATER => (" > ", PREC_CMP),
        LESS_EQUAL => (" <= ", PREC_CMP),
        GREATER_EQUAL => (" >= ", PREC_CMP),
        _ => return None,
    })
}

/// Associative operators rendered as `a OP b OP c …`.
fn nary_operator(name: &str) -> Option<(&'static str, u8)> {
    Some(match name {
        ADD => (" + ", PREC_ADD),
        MUL => ("*", PREC_MUL),
        AND => (" && ", PREC_AND),
        OR => (" || ", PREC_OR),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use symbolic_ir::{apply, flt, int, str_node, sym};

    #[test]
    fn atoms() {
        assert_eq!(print_wolfram(&int(42)), "42");
        assert_eq!(print_wolfram(&sym("x")), "x");
        assert_eq!(print_wolfram(&str_node("hi")), "\"hi\"");
        assert_eq!(print_wolfram(&flt(1.5)), "1.5");
    }

    #[test]
    fn addition_and_multiplication() {
        assert_eq!(
            print_wolfram(&apply(sym(ADD), vec![sym("x"), int(1)])),
            "x + 1"
        );
        assert_eq!(
            print_wolfram(&apply(sym(MUL), vec![int(2), sym("y")])),
            "2*y"
        );
    }

    #[test]
    fn precedence_parenthesises_looser_children() {
        // Mul(Add(a, b), c)  ->  (a + b)*c
        let expr = apply(
            sym(MUL),
            vec![apply(sym(ADD), vec![sym("a"), sym("b")]), sym("c")],
        );
        assert_eq!(print_wolfram(&expr), "(a + b)*c");
    }

    #[test]
    fn no_spurious_parens_when_child_binds_tighter() {
        // Add(a, Mul(b, c))  ->  a + b*c   (no parens needed)
        let expr = apply(
            sym(ADD),
            vec![sym("a"), apply(sym(MUL), vec![sym("b"), sym("c")])],
        );
        assert_eq!(print_wolfram(&expr), "a + b*c");
    }

    #[test]
    fn power_and_neg() {
        assert_eq!(
            print_wolfram(&apply(sym(POW), vec![sym("x"), int(2)])),
            "x^2"
        );
        assert_eq!(print_wolfram(&apply(sym(NEG), vec![sym("x")])), "-x");
    }

    #[test]
    fn list_uses_braces() {
        assert_eq!(
            print_wolfram(&apply(sym(LIST), vec![int(1), int(2), int(3)])),
            "{1, 2, 3}"
        );
        assert_eq!(print_wolfram(&apply(sym(LIST), vec![])), "{}");
    }

    #[test]
    fn application_uses_square_brackets() {
        assert_eq!(print_wolfram(&apply(sym("Sin"), vec![sym("x")])), "Sin[x]");
        assert_eq!(
            print_wolfram(&apply(sym("f"), vec![sym("a"), sym("b")])),
            "f[a, b]"
        );
    }

    #[test]
    fn comparison_and_logic() {
        assert_eq!(
            print_wolfram(&apply(sym(EQUAL), vec![sym("a"), sym("b")])),
            "a == b"
        );
        assert_eq!(
            print_wolfram(&apply(sym(AND), vec![sym("a"), sym("b")])),
            "a && b"
        );
        assert_eq!(print_wolfram(&apply(sym(NOT), vec![sym("p")])), "!p");
    }

    #[test]
    fn blank_and_pattern_render_back() {
        assert_eq!(print_wolfram(&apply(sym(BLANK), vec![])), "_");
        assert_eq!(
            print_wolfram(&apply(sym(BLANK), vec![sym("Integer")])),
            "_Integer"
        );
        assert_eq!(
            print_wolfram(&apply(
                sym(PATTERN),
                vec![sym("x"), apply(sym(BLANK), vec![])]
            )),
            "x_"
        );
    }

    #[test]
    fn rule_and_replaceall() {
        assert_eq!(
            print_wolfram(&apply(sym(PM_RULE), vec![sym("a"), sym("b")])),
            "a -> b"
        );
        assert_eq!(
            print_wolfram(&apply(sym(PM_RULE_DELAYED), vec![sym("a"), sym("b")])),
            "a :> b"
        );
    }

    #[test]
    fn assignment_renders() {
        assert_eq!(
            print_wolfram(&apply(sym(ASSIGN), vec![sym("x"), int(5)])),
            "x = 5"
        );
    }

    #[test]
    fn unknown_compound_head_falls_back_to_application() {
        // An unknown 2-arg head with no infix form prints as application.
        assert_eq!(
            print_wolfram(&apply(sym("Mystery"), vec![int(1), int(2)])),
            "Mystery[1, 2]"
        );
    }

    #[test]
    fn nested_application_head() {
        // (f[x])[y]
        let expr = apply(apply(sym("f"), vec![sym("x")]), vec![sym("y")]);
        assert_eq!(print_wolfram(&expr), "f[x][y]");
    }
}
