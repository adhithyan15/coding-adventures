//! The `math-frontend` adapter (L6) — the capstone that makes LaTeX a **pluggable frontend**.
//!
//! Layers L0–L5 built a standalone LaTeX parser with its own AST ([`crate::MathNode`] for math,
//! [`crate::Node`] for documents). This layer connects that parser to the shared
//! [`math_frontend`] framework ([PFE01](../../../specs/PFE01-pluggable-parser-frontends.md)): it
//! implements the [`MathFrontend`] trait by parsing math with [`parse_math`](crate::parse_math)
//! and **lowering** the LaTeX-shaped [`MathNode`] into the notation-agnostic
//! [`MathExpr`]. After this, a consumer (a rule engine, a CAS, a renderer) lowers *one* neutral
//! tree and gets LaTeX for free — and adding AsciiMath/MathML later is "register another
//! frontend", with no consumer change.
//!
//! ## What "neutral" means here
//!
//! The neutral AST deliberately drops *presentation* and keeps *meaning*, so this lowering:
//! - `\times`, `\cdot`, and juxtaposition all become [`BinOp::Mul`]; `\dfrac`/`\tfrac`/`\frac`
//!   all become [`MathExpr::Frac`] — two source strings that mean the same math lower equal.
//! - fence *style* is dropped: `(…)`, `[…]`, `\left(…\right)` all become [`MathExpr::Group`].
//! - matrix *delimiter* is dropped: `pmatrix`/`bmatrix`/`cases`/… all become [`MathExpr::Matrix`].
//! - `base^sup` → [`BinOp::Pow`]; `base_sub` → [`MathExpr::Subscript`]; both → `Pow(Subscript(…))`.
//! - an accent (`\hat{x}`, `\vec{v}`) is a named unary, lowered to `Call{Other(kind), arg}`.
//!
//! ## Honest gaps (neutral AST has no representation)
//!
//! Two LaTeX math constructs have *no* neutral counterpart today, so rather than fake them we
//! return a well-formed [`FrontendError`] (the parse of that island fails honestly):
//! - `\pm` / `\mp` — the neutral [`BinOp`] has no ± / ∓;
//! - `\binom{n}{k}` — the neutral AST has no binomial.
//!
//! Extending the neutral AST to cover these is a future change to the `math-frontend` crate
//! (and its conformance harness), not a hack here. Everything else L2/L3a can parse lowers.
//!
//! ## Registration
//!
//! `math_frontend`'s own `FrontendRegistry::with_builtins()` stays empty by design — that crate
//! cannot depend on this one (it would be a dependency cycle). Instead the wiring lives here:
//! [`registry`] returns a registry with LaTeX installed, and [`register_latex`] installs it into
//! an existing one. This whole module is gated behind the (default-on) `frontend` cargo feature,
//! so L0–L5 remain dependency-free under `--no-default-features`.

use crate::{MBinOp, MRelOp, MUnOp, MathNode};
use math_frontend::{
    BigOp, BinOp, Capabilities, Func, FrontendError, FrontendRegistry, MathExpr, MathFrontend,
    Number, RelOp, UnaryOp,
};

/// The LaTeX math frontend: parse a math-mode source string into the neutral [`MathExpr`].
pub struct LatexMath;

impl MathFrontend for LatexMath {
    fn name(&self) -> &str {
        "latex"
    }

    fn parse(&self, src: &str) -> Result<MathExpr, FrontendError> {
        // Parse with our own grammar (L2/L3a), then lower the result. A parse error carries a
        // precise byte span; a lowering gap uses the whole-source span (the LaTeX math AST is
        // span-free, so we cannot point finer than "somewhere in this island").
        let node = crate::parse_math(src)
            .map_err(|e| FrontendError::new("latex", e.message, e.span))?;
        lower(node, (0, src.len()))
    }

    fn capabilities(&self) -> Capabilities {
        // The L2/L3a grammar can emit every neutral construct. (Declaring a capability is a
        // promise never to emit *more* than declared; emitting fewer on a given input is fine.)
        Capabilities::all()
    }
}

/// Install the LaTeX frontend into an existing registry (replacing any prior `"latex"`).
pub fn register_latex(registry: &mut FrontendRegistry) {
    registry.register(Box::new(LatexMath));
}

/// A fresh [`FrontendRegistry`] with LaTeX registered as the first frontend — the assembly
/// point the framework's empty `with_builtins()` defers to (see the module docs on cycles).
pub fn registry() -> FrontendRegistry {
    let mut r = FrontendRegistry::new();
    register_latex(&mut r);
    r
}

/// Box helper — keeps the lowering arms readable.
fn b(e: MathExpr) -> Box<MathExpr> {
    Box::new(e)
}

/// Lower one [`MathNode`] (LaTeX-shaped) into the neutral [`MathExpr`]. `span` is the
/// whole-source span used for the (rare) lowering-gap errors. Total and panic-free: recursion is
/// bounded by the tree depth, which [`parse_math`](crate::parse_math) already caps.
fn lower(node: MathNode, span: (usize, usize)) -> Result<MathExpr, FrontendError> {
    Ok(match node {
        MathNode::Num(s) => MathExpr::Number(Number::parse(&s).ok_or_else(|| {
            FrontendError::new("latex", format!("invalid numeric literal {s:?}"), span)
        })?),
        MathNode::Sym(s) => MathExpr::Symbol(s),
        MathNode::Bin(op, x, y) => {
            // Resolve the operator first so `\pm`/`\mp` fail before we allocate the operands.
            let op = lower_binop(op, span)?;
            MathExpr::Bin(op, b(lower(*x, span)?), b(lower(*y, span)?))
        }
        MathNode::Unary(op, x) => {
            let op = match op {
                MUnOp::Neg => UnaryOp::Neg,
                MUnOp::Pos => UnaryOp::Pos,
            };
            MathExpr::Unary(op, b(lower(*x, span)?))
        }
        MathNode::Frac(x, y) => MathExpr::Frac(b(lower(*x, span)?), b(lower(*y, span)?)),
        MathNode::Binom(_, _) => {
            return Err(FrontendError::new(
                "latex",
                "binomial \\binom has no neutral MathExpr representation",
                span,
            ))
        }
        MathNode::Root { degree, radicand } => MathExpr::Root {
            degree: match degree {
                Some(d) => Some(b(lower(*d, span)?)),
                None => None,
            },
            radicand: b(lower(*radicand, span)?),
        },
        MathNode::Script { base, sub, sup } => {
            // `a_i` → Subscript; `a^n` → Pow; `a_i^n` → Pow(Subscript(a, i), n).
            let mut acc = lower(*base, span)?;
            if let Some(s) = sub {
                acc = MathExpr::Subscript(b(acc), b(lower(*s, span)?));
            }
            if let Some(p) = sup {
                acc = MathExpr::Bin(BinOp::Pow, b(acc), b(lower(*p, span)?));
            }
            acc
        }
        MathNode::Call { func, arg } => MathExpr::Call {
            func: lower_func(&func),
            arg: b(lower(*arg, span)?),
        },
        MathNode::BigOp { op, lower: lo, upper, body } => MathExpr::BigOp {
            op: lower_bigop(&op),
            lower: match lo {
                Some(l) => Some(b(lower(*l, span)?)),
                None => None,
            },
            upper: match upper {
                Some(u) => Some(b(lower(*u, span)?)),
                None => None,
            },
            body: b(lower(*body, span)?),
        },
        // An accent is a named unary operator on its argument (`\hat{x}` ≈ hat(x)).
        MathNode::Accent { kind, body } => MathExpr::Call {
            func: Func::Other(kind),
            arg: b(lower(*body, span)?),
        },
        // Fence style is presentation; the meaning is "this is grouped".
        MathNode::Fenced { body, .. } => MathExpr::Group(b(lower(*body, span)?)),
        MathNode::Text(s) => MathExpr::Text(s),
        MathNode::Rel(op, x, y) => {
            MathExpr::Rel(lower_relop(op), b(lower(*x, span)?), b(lower(*y, span)?))
        }
        // Matrix delimiter (pmatrix/bmatrix/cases/…) is presentation; rows/cols carry the math.
        MathNode::Matrix { rows, .. } => {
            let mut out = Vec::with_capacity(rows.len());
            for row in rows {
                let mut cells = Vec::with_capacity(row.len());
                for cell in row {
                    cells.push(lower(cell, span)?);
                }
                out.push(cells);
            }
            MathExpr::Matrix(out)
        }
    })
}

/// Lower a binary operator. `\pm`/`\mp` have no neutral form → spanned error.
fn lower_binop(op: MBinOp, span: (usize, usize)) -> Result<BinOp, FrontendError> {
    Ok(match op {
        MBinOp::Add => BinOp::Add,
        MBinOp::Sub => BinOp::Sub,
        MBinOp::Mul => BinOp::Mul,
        MBinOp::Div => BinOp::Div,
        MBinOp::Pow => BinOp::Pow,
        MBinOp::PlusMinus => {
            return Err(FrontendError::new(
                "latex",
                "\\pm has no neutral MathExpr representation",
                span,
            ))
        }
        MBinOp::MinusPlus => {
            return Err(FrontendError::new(
                "latex",
                "\\mp has no neutral MathExpr representation",
                span,
            ))
        }
    })
}

/// Map a relation operator (1:1 — both enums carry the same eight relations).
fn lower_relop(op: MRelOp) -> RelOp {
    match op {
        MRelOp::Eq => RelOp::Eq,
        MRelOp::Ne => RelOp::Ne,
        MRelOp::Lt => RelOp::Lt,
        MRelOp::Le => RelOp::Le,
        MRelOp::Gt => RelOp::Gt,
        MRelOp::Ge => RelOp::Ge,
        MRelOp::Approx => RelOp::Approx,
        MRelOp::Equiv => RelOp::Equiv,
    }
}

/// Map a function name to a closed [`Func`] variant where known, else preserve it by name.
fn lower_func(name: &str) -> Func {
    match name {
        "sin" => Func::Sin,
        "cos" => Func::Cos,
        "tan" => Func::Tan,
        "cot" => Func::Cot,
        "sec" => Func::Sec,
        "csc" => Func::Csc,
        "arcsin" => Func::Asin,
        "arccos" => Func::Acos,
        "arctan" => Func::Atan,
        "sinh" => Func::Sinh,
        "cosh" => Func::Cosh,
        "tanh" => Func::Tanh,
        "ln" => Func::Ln,
        "log" => Func::Log,
        "exp" => Func::Exp,
        "min" => Func::Min,
        "max" => Func::Max,
        "gcd" => Func::Gcd,
        "lcm" => Func::Lcm,
        "det" => Func::Det,
        other => Func::Other(other.to_string()),
    }
}

/// Map a big-operator name to a closed [`BigOp`] variant where known, else preserve by name.
fn lower_bigop(name: &str) -> BigOp {
    match name {
        "sum" => BigOp::Sum,
        "prod" => BigOp::Prod,
        "int" => BigOp::Int,
        "oint" => BigOp::Oint,
        "coprod" => BigOp::Coprod,
        "lim" => BigOp::Lim,
        other => BigOp::Other(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use math_frontend::check_frontend;

    /// Parse through the frontend, unwrapping for concise assertions.
    fn m(src: &str) -> MathExpr {
        LatexMath.parse(src).expect("parse")
    }

    fn num(n: i64) -> MathExpr {
        MathExpr::Number(Number::from_i64(n))
    }

    #[test]
    fn name_and_capabilities() {
        assert_eq!(LatexMath.name(), "latex");
        let c = LatexMath.capabilities();
        assert!(c.fractions && c.roots && c.powers && c.functions && c.big_operators);
        assert!(c.relations && c.matrices && c.implicit_mul && c.text);
    }

    #[test]
    fn fraction_lowers() {
        assert_eq!(m(r"\frac{1}{2}"), MathExpr::Frac(Box::new(num(1)), Box::new(num(2))));
        // \dfrac is presentation — same neutral tree.
        assert_eq!(m(r"\dfrac{1}{2}"), m(r"\frac{1}{2}"));
    }

    #[test]
    fn power_and_subscript() {
        assert_eq!(m("2^{10}"), MathExpr::Bin(BinOp::Pow, Box::new(num(2)), Box::new(num(10))));
        assert_eq!(
            m("a_i"),
            MathExpr::Subscript(
                Box::new(MathExpr::Symbol("a".into())),
                Box::new(MathExpr::Symbol("i".into())),
            )
        );
        // both: a_i^n → Pow(Subscript(a,i), n)
        match m("a_i^n") {
            MathExpr::Bin(BinOp::Pow, base, _) => {
                assert!(matches!(*base, MathExpr::Subscript(..)));
            }
            other => panic!("expected Pow(Subscript,..), got {other:?}"),
        }
    }

    #[test]
    fn roots() {
        match m(r"\sqrt[3]{x}") {
            MathExpr::Root { degree: Some(d), .. } => assert_eq!(*d, num(3)),
            other => panic!("expected Root with degree, got {other:?}"),
        }
        assert!(matches!(m(r"\sqrt{2}"), MathExpr::Root { degree: None, .. }));
    }

    #[test]
    fn functions_and_big_operators() {
        assert_eq!(
            m(r"\sin x"),
            MathExpr::Call { func: Func::Sin, arg: Box::new(MathExpr::Symbol("x".into())) }
        );
        match m(r"\sum_{i=1}^{n} i") {
            MathExpr::BigOp { op: BigOp::Sum, lower: Some(_), upper: Some(_), .. } => {}
            other => panic!("expected Sum with bounds, got {other:?}"),
        }
    }

    #[test]
    fn func_and_bigop_name_mapping() {
        // The name → closed-variant maps, with an out-of-set name preserved via `Other`.
        assert_eq!(lower_func("sin"), Func::Sin);
        assert_eq!(lower_func("arctan"), Func::Atan);
        assert_eq!(lower_func("zeta"), Func::Other("zeta".into()));
        assert_eq!(lower_bigop("prod"), BigOp::Prod);
        assert_eq!(lower_bigop("bigcup"), BigOp::Other("bigcup".into()));
    }

    #[test]
    fn relations() {
        assert_eq!(
            m("a = b"),
            MathExpr::Rel(
                RelOp::Eq,
                Box::new(MathExpr::Symbol("a".into())),
                Box::new(MathExpr::Symbol("b".into())),
            )
        );
    }

    #[test]
    fn multiplication_is_normalized_across_notations() {
        // \times, \cdot, and juxtaposition all mean Mul → same neutral tree.
        let a_times_b = m(r"a \times b");
        assert_eq!(a_times_b, m(r"a \cdot b"));
        assert_eq!(a_times_b, m("ab"));
        assert!(matches!(a_times_b, MathExpr::Bin(BinOp::Mul, _, _)));
    }

    #[test]
    fn implicit_multiplication() {
        assert_eq!(
            m("2x"),
            MathExpr::Bin(BinOp::Mul, Box::new(num(2)), Box::new(MathExpr::Symbol("x".into())))
        );
    }

    #[test]
    fn symbols_text_and_groups() {
        assert_eq!(m(r"\pi"), MathExpr::Symbol("pi".into()));
        assert_eq!(m(r"\text{kg}"), MathExpr::Text("kg".into()));
        // fence style dropped to Group.
        match m("(a+b)") {
            MathExpr::Group(inner) => assert!(matches!(*inner, MathExpr::Bin(BinOp::Add, ..))),
            other => panic!("expected Group, got {other:?}"),
        }
    }

    #[test]
    fn accent_is_a_named_unary() {
        assert_eq!(
            m(r"\hat{x}"),
            MathExpr::Call {
                func: Func::Other("hat".into()),
                arg: Box::new(MathExpr::Symbol("x".into())),
            }
        );
    }

    #[test]
    fn matrix_lowers_dropping_delimiter_style() {
        match m(r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}") {
            MathExpr::Matrix(rows) => {
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0].len(), 2);
                assert_eq!(rows[0][0], MathExpr::Symbol("a".into()));
            }
            other => panic!("expected Matrix, got {other:?}"),
        }
    }

    #[test]
    fn numbers_stay_exact_not_f64() {
        assert_eq!(m("0.1"), MathExpr::Number(Number::parse("0.1").unwrap()));
        // and the numerator of a fraction is an exact Number, not a float.
        match m(r"\frac{1}{3}") {
            MathExpr::Frac(n, _) => assert_eq!(*n, num(1)),
            other => panic!("expected Frac, got {other:?}"),
        }
    }

    #[test]
    fn neutral_gaps_error_honestly() {
        // \pm / \mp / \binom have no neutral representation → well-formed spanned error.
        for src in [r"a \pm b", r"a \mp b", r"\binom{n}{k}"] {
            let err = LatexMath.parse(src).expect_err("should be a neutral-AST gap");
            assert_eq!(err.frontend, "latex");
            assert!(err.span.0 <= err.span.1 && err.span.1 <= src.len());
        }
    }

    #[test]
    fn parse_error_span_is_propagated() {
        // A genuine parse failure carries the grammar's byte span, naming this frontend.
        let err = LatexMath.parse(r"\frac{1}").expect_err("incomplete frac");
        assert_eq!(err.frontend, "latex");
        assert!(err.span.1 <= r"\frac{1}".len());
    }

    #[test]
    fn registry_installs_latex_as_a_plugin() {
        let r = registry();
        assert_eq!(r.names(), vec!["latex"]);
        assert_eq!(r.parse("latex", "1+2").unwrap(), m("1+2"));
        // unknown frontend errors without panicking.
        assert!(r.parse("asciimath", "1").is_err());
    }

    #[test]
    fn register_into_existing_registry() {
        let mut r = FrontendRegistry::new();
        register_latex(&mut r);
        assert!(r.get("latex").is_some());
    }

    #[test]
    fn conforms_to_the_shared_harness() {
        let report = check_frontend(
            &LatexMath,
            &[
                r"\frac{1}{2}",
                "2^{10}",
                r"\sqrt[3]{x}",
                r"\sin x",
                r"\sum_{i=1}^{n} i",
                "a = b",
                "2x",
                r"\pi",
                r"\text{kg}",
                r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}",
                "a_i",
                r"\hat{x}",
                "(a+b)",
                r"a \pm b",       // gap: must error well-formed, not panic
                r"\binom{n}{k}",  // gap
                r"\frac{1}",      // parse error: span in range
                "",               // empty
            ],
        );
        assert!(report.passed(), "conformance issues: {:?}", report.issues);
    }
}
