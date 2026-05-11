//! MACSYMA / Maxima dialect.
//!
//! Surface syntax conventions:
//!
//! - Function calls use parentheses: `sin(x)`, `f(x, y)`.
//! - Lists use square brackets: `[1, 2, 3]`.
//! - Power is `^` (MACSYMA also accepts `**` on input; output uses `^`).
//! - Equality is `=`; not-equal is `#`.
//! - Function names are lowercase, with MACSYMA-specific aliases where the
//!   canonical spelling differs: `sin`, `diff`, `ratsimp`, `realpart`, etc.
//!
//! # Surface sugar
//!
//! The MACSYMA dialect applies three rewrite rules before the walker
//! dispatches:
//!
//! | Input IR                       | Rewrites to  | Displays as |
//! |-------------------------------|--------------|-------------|
//! | `Mul(-1, x)`                  | `Neg(x)`     | `-x`        |
//! | `Mul(-1, x, y, …)`            | `Mul(Neg(x), y, …)` | `-x*y*…` |
//! | `Add(a, Neg(b))` _(2-arg)_   | `Sub(a, b)`  | `a - b`     |
//! | `Add(a, Mul(b, Neg(c)))` _(2-arg)_ | `Sub(a, Mul(b, c))` | `a - b*c` |
//! | `Add(-n, y)`                 | `Sub(y, n)`  | `y - n`     |
//! | `Mul(a, Inv(b))` _(2-arg)_   | `Div(a, b)`  | `a/b`       |
//! | `Mul(a, Neg(b))` _(2-arg)_   | `Neg(Mul(a, b))` | `-(a*b)` |
//!
//! The walker applies sugar recursively, so these small local rewrites compose
//! with normal precedence handling.

use symbolic_ir::{apply, sym, IRApply, IRNode, DIV, INV, MUL, NEG, SUB};

use crate::dialect::{
    default_binary_op, default_function_name, default_precedence, default_unary_op, Dialect,
};

// ---------------------------------------------------------------------------
// MacsymaDialect
// ---------------------------------------------------------------------------

/// MACSYMA/Maxima dialect.
///
/// Use `&MacsymaDialect` when passing to [`pretty`](crate::pretty).
pub struct MacsymaDialect;

impl Dialect for MacsymaDialect {
    fn name(&self) -> &str {
        "macsyma"
    }

    // ---- numeric formatting ------------------------------------------------

    fn format_integer(&self, value: i64) -> String {
        value.to_string()
    }

    fn format_rational(&self, numer: i64, denom: i64) -> String {
        format!("{}/{}", numer, denom)
    }

    /// Uses Rust's `{:?}` which produces round-trip–safe output (e.g.
    /// `3.14` stays `3.14`, not `3.1400000000000001`).
    fn format_float(&self, value: f64) -> String {
        format!("{:?}", value)
    }

    fn format_string(&self, value: &str) -> String {
        format!("\"{}\"", value)
    }

    fn format_symbol(&self, name: &str) -> String {
        match name {
            "ImaginaryUnit" => "%i",
            other => other,
        }
        .to_string()
    }

    // ---- operators ---------------------------------------------------------

    fn binary_op(&self, head_name: &str) -> Option<String> {
        default_binary_op(head_name)
    }

    fn unary_op(&self, head_name: &str) -> Option<String> {
        default_unary_op(head_name)
    }

    fn function_name(&self, head_name: &str) -> String {
        match head_name {
            "Select" => "sublist",
            "MakeList" => "makelist",
            "Inverse" => "invert",
            "RatSimplify" => "ratsimp",
            "Apart" => "partfrac",
            "TrigSimplify" => "trigsimp",
            "TrigExpand" => "trigexpand",
            "TrigReduce" => "trigreduce",
            "Re" => "realpart",
            "Im" => "imagpart",
            "Arg" => "carg",
            "RectForm" => "rectform",
            "PolarForm" => "polarform",
            "IsPrime" => "primep",
            "NextPrime" => "next_prime",
            "PrevPrime" => "prev_prime",
            "FactorInteger" => "ifactor",
            "Divisors" => "divisors",
            "Totient" => "totient",
            "MoebiusMu" => "moebius",
            "JacobiSymbol" => "jacobi",
            "ChineseRemainder" => "chinese",
            "IntegerLength" => "numdigits",
            other => return default_function_name(other),
        }
        .to_string()
    }

    // ---- containers --------------------------------------------------------

    fn list_brackets(&self) -> (&'static str, &'static str) {
        ("[", "]")
    }

    fn call_brackets(&self) -> (&'static str, &'static str) {
        ("(", ")")
    }

    // ---- precedence --------------------------------------------------------

    fn precedence(&self, head_name: &str) -> u32 {
        default_precedence(head_name)
    }

    fn is_right_associative(&self, head_name: &str) -> bool {
        // Only `^` is right-associative: `a^b^c` = `a^(b^c)`.
        head_name == "Pow"
    }

    // ---- sugar -------------------------------------------------------------

    fn try_sugar(&self, node: &IRApply) -> Option<IRNode> {
        macsyma_sugar(node)
    }
}

// ---------------------------------------------------------------------------
// Sugar logic (also used by MathematicaDialect and MapleDialect)
// ---------------------------------------------------------------------------

/// Apply MACSYMA surface-sugar rules to `node`.
///
/// Called by both [`MacsymaDialect`] and the other dialects that share the
/// same arithmetic surface syntax.
pub(crate) fn macsyma_sugar(node: &IRApply) -> Option<IRNode> {
    let head_name = match &node.head {
        IRNode::Symbol(s) => s.as_str(),
        _ => return None,
    };

    // Rule 1: Mul(-1, x) → Neg(x)
    //         Mul(-1, x, y, …) → Mul(Neg(x), y, …)
    //
    // This turns `Mul(-1, x)` into `-x` and keeps longer products as
    // `-x*y*...` instead of wrapping the whole product.
    if head_name == MUL && node.args.len() >= 2 {
        if let IRNode::Integer(-1) = &node.args[0] {
            let rest = node.args[1..].to_vec();
            let inner = if rest.len() == 1 {
                rest.into_iter().next().unwrap()
            } else {
                let mut negated_product = Vec::with_capacity(rest.len());
                let mut rest_iter = rest.into_iter();
                negated_product.push(apply(sym(NEG), vec![rest_iter.next().unwrap()]));
                negated_product.extend(rest_iter);
                return Some(apply(sym(MUL), negated_product));
            };
            return Some(apply(sym(NEG), vec![inner]));
        }
    }

    // Rule 2: Add(a, Neg(b)) → Sub(a, b)  [2-arg case only]
    //         Add(a, Mul(b, Neg(c))) → Sub(a, Mul(b, c))
    //         Add(-n, y) → Sub(y, n)
    //
    // Peek one sugar level into the second argument so a negated product
    // becomes a clean subtraction, matching the Python reference behavior.
    if head_name == "Add" && node.args.len() == 2 {
        let (a, b) = (&node.args[0], &node.args[1]);

        let mut sugared_b: Option<IRNode> = None;
        if let IRNode::Apply(b_apply) = b {
            sugared_b = macsyma_sugar(b_apply);
        }
        let b_effective = sugared_b.as_ref().unwrap_or(b);

        if let IRNode::Apply(b_apply) = b_effective {
            if matches!(&b_apply.head, IRNode::Symbol(b_head) if b_head.as_str() == NEG)
                && b_apply.args.len() == 1
            {
                return Some(apply(sym(SUB), vec![a.clone(), b_apply.args[0].clone()]));
            }
        }

        if let IRNode::Integer(value) = a {
            if *value < 0 {
                return Some(apply(sym(SUB), vec![b.clone(), IRNode::Integer(-*value)]));
            }
        }
    }

    // Rule 3: Mul(a, Inv(b)) → Div(a, b)  [2-arg case only]
    //         Mul(a, Neg(b)) → Neg(Mul(a, b))
    //
    // Same caution as Rule 2: only the simple 2-arg multiplication by an
    // inverse or negated factor is sugar'd.
    if head_name == MUL && node.args.len() == 2 {
        let (a, b) = (&node.args[0], &node.args[1]);
        if let IRNode::Apply(b_apply) = b {
            if let IRNode::Symbol(b_head) = &b_apply.head {
                if b_apply.args.len() == 1 {
                    if b_head.as_str() == INV {
                        return Some(apply(sym(DIV), vec![a.clone(), b_apply.args[0].clone()]));
                    }
                    if b_head.as_str() == NEG {
                        return Some(apply(
                            sym(NEG),
                            vec![apply(sym(MUL), vec![a.clone(), b_apply.args[0].clone()])],
                        ));
                    }
                }
            }
        }
    }

    None
}
