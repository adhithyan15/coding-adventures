//! Identity rule database used by [`crate::simplify`].
//!
//! Every rule is an `IRNode::Apply(Rule, [lhs, rhs])` created with
//! [`cas_pattern_matching::rule`].  Pattern variables use
//! `named("x", blank())` — **not** bare `sym("x")` — so that the rewriter's
//! substitution step can replace them with captured values.
//!
//! ## Why `named`, not `sym`?
//!
//! The `rewrite` engine substitutes only `Pattern(name, inner)` nodes in the
//! RHS.  A bare `Symbol("x")` in the RHS would be returned literally,
//! leaving the capture unused.  `named("x", blank())` wraps the name in a
//! `Pattern` sentinel that the substitution step recognises.
//!
//! ## Rule table
//!
//! | LHS | RHS | Identity |
//! |-----|-----|----------|
//! | `Add(x_, 0)` | `x_` | additive identity |
//! | `Add(0, x_)` | `x_` | additive identity (commuted) |
//! | `Mul(x_, 1)` | `x_` | multiplicative identity |
//! | `Mul(1, x_)` | `x_` | multiplicative identity (commuted) |
//! | `Mul(x_, 0)` | `0` | zero product |
//! | `Mul(0, x_)` | `0` | zero product (commuted) |
//! | `Pow(x_, 0)` | `1` | zeroth power |
//! | `Pow(x_, 1)` | `x_` | first power |
//! | `Pow(1, x_)` | `1` | one to any power |
//! | `Sub(x_, x_)` | `0` | self-cancellation |
//! | `Div(x_, x_)` | `1` | self-cancellation |
//! | `Log(Exp(x_))` | `x_` | log/exp inverse |
//! | `Exp(Log(x_))` | `x_` | exp/log inverse |
//! | `Sin(0)` | `0` | sin at zero |
//! | `Cos(0)` | `1` | cos at zero |

use cas_pattern_matching::{blank, named, rule};
use symbolic_ir::{apply, int, sym, IRNode, ADD, COS, DIV, EXP, LOG, MUL, POW, SIN, SUB};

/// Build and return the list of algebraic identity rules.
///
/// Each rule is an `IRNode::Apply(Rule, [lhs, rhs])` that the
/// `cas_pattern_matching::rewrite` engine can apply.
///
/// The returned `Vec` is fresh on every call.  For performance, callers
/// should build it once and store it.  [`crate::simplify`] builds it on each
/// call; that is acceptable because the call site is already in a loop and
/// the construction is O(1) allocations.
pub fn build_identity_rules() -> Vec<IRNode> {
    // The pattern variable `x_` — `named("x", blank())` matches anything and
    // binds the capture under the name `"x"`.  A closure is used so each rule
    // gets its own `IRNode` (they're not `Copy`).
    let x = || named("x", blank());
    let zero = int(0);
    let one = int(1);

    vec![
        // ----------------------------------------------------------------
        // Additive identity: x + 0 → x,  0 + x → x
        // ----------------------------------------------------------------
        rule(apply(sym(ADD), vec![x(), zero.clone()]), x()),
        rule(apply(sym(ADD), vec![zero.clone(), x()]), x()),
        // ----------------------------------------------------------------
        // Multiplicative identity: x*1 → x,  1*x → x
        // ----------------------------------------------------------------
        rule(apply(sym(MUL), vec![x(), one.clone()]), x()),
        rule(apply(sym(MUL), vec![one.clone(), x()]), x()),
        // ----------------------------------------------------------------
        // Zero product: x*0 → 0,  0*x → 0
        // ----------------------------------------------------------------
        rule(apply(sym(MUL), vec![x(), zero.clone()]), zero.clone()),
        rule(apply(sym(MUL), vec![zero.clone(), x()]), zero.clone()),
        // ----------------------------------------------------------------
        // Power identities
        //   x^0 → 1   (any non-zero base; 0^0 left unevaluated by convention)
        //   x^1 → x
        //   1^x → 1
        // ----------------------------------------------------------------
        rule(apply(sym(POW), vec![x(), zero.clone()]), one.clone()),
        rule(apply(sym(POW), vec![x(), one.clone()]), x()),
        rule(apply(sym(POW), vec![one.clone(), x()]), one.clone()),
        // ----------------------------------------------------------------
        // Self-cancellation: x - x → 0,  x / x → 1
        //
        // Using the same variable name `"x"` in both LHS slots forces the
        // matcher to check that both slots bind to the *same* expression.
        // Sub(a, b) matches only when a == b.
        // ----------------------------------------------------------------
        rule(apply(sym(SUB), vec![x(), x()]), zero.clone()),
        rule(apply(sym(DIV), vec![x(), x()]), one.clone()),
        // ----------------------------------------------------------------
        // Inverse-function identities
        //   Log(Exp(x)) → x
        //   Exp(Log(x)) → x
        // ----------------------------------------------------------------
        rule(
            apply(sym(LOG), vec![apply(sym(EXP), vec![x()])]),
            x(),
        ),
        rule(
            apply(sym(EXP), vec![apply(sym(LOG), vec![x()])]),
            x(),
        ),
        // ----------------------------------------------------------------
        // Trig at zero
        //   Sin(0) → 0
        //   Cos(0) → 1
        // ----------------------------------------------------------------
        rule(apply(sym(SIN), vec![zero.clone()]), zero.clone()),
        rule(apply(sym(COS), vec![zero.clone()]), one.clone()),
    ]
}
