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
//! - an accent (`\hat{x}`, `\vec{v}`) lowers to the neutral [`MathExpr::Accent`] (a diacritic
//!   over its body), distinct from a named-function `Call`.
//! - `\pm` / `\mp` → [`BinOp::PlusMinus`] / [`BinOp::MinusPlus`] (the ± / ∓ pair operators).
//! - `\binom{n}{k}` → [`MathExpr::Binom`] (a binomial coefficient — no division bar).
//!
//! Every LaTeX math construct L2/L3a can parse now lowers to a neutral counterpart: the two
//! former gaps (± / ∓ and binomials) were closed by extending the `math-frontend` neutral AST
//! and its conformance harness, then wiring them here — no faking, no hack.
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

/// An internal-imbalance error — produced only if the trampoline below were ever malformed
/// (it never is). Keeps `lower` panic-free instead of `unwrap`-ing the value stack.
fn imbalance(span: (usize, usize)) -> FrontendError {
    FrontendError::new("latex", "internal lowering stack imbalance", span)
}

/// A deferred "assemble this parent from already-lowered children on the value stack" step.
/// Each variant pops a fixed number of `MathExpr`s and pushes the rebuilt node.
enum Build {
    Bin(BinOp),
    Unary(UnaryOp),
    Frac,
    Binom,
    Overset,
    Underset,
    Root { has_degree: bool },
    Script { has_sub: bool, has_sup: bool },
    Call(Func),
    Accent(String),
    BigOp { op: BigOp, has_lower: bool, has_upper: bool },
    Group,
    Rel(RelOp),
    Matrix { row_lens: Vec<usize> },
}

/// One unit of work: either lower a raw node, or assemble a parent from finished children.
enum Task {
    Node(MathNode),
    Build(Build),
}

/// Lift the owned `MathNode` out of a boxed child, leaving a cheap leaf (`Sym("")`) in its
/// place. Needed because `MathNode: Drop` forbids moving a field out of an owned value (E0509);
/// the emptied parent then drops shallowly. Mirrors the `take` helper in `MathNode`'s own `Drop`.
fn take_box(b: &mut Box<MathNode>) -> MathNode {
    std::mem::replace(b.as_mut(), MathNode::Sym(String::new()))
}

/// Lower one [`MathNode`] (LaTeX-shaped) into the neutral [`MathExpr`].
///
/// **Iterative on purpose.** A LaTeX math tree can be *arbitrarily* deep along a left-
/// associative spine — `a+a+a+…`, juxtaposition `aaa…`, a sign run `---…x`, a chained
/// relation — because the parser builds those with loops, not nesting, so `parse_math`'s
/// `MAX_DEPTH` (which bounds only *structural* nesting) does **not** bound them. A naive
/// recursive lowering would therefore overflow the stack on adversarial input — an
/// *uncatchable* abort, breaking the `MathFrontend` "total / panic-free" contract. So we walk
/// the tree with an explicit work stack and an explicit value stack: stack usage is O(1) in the
/// call frame regardless of tree depth, and the input is consumed spine-node-by-spine-node (no
/// deep recursive `Drop` of the input either). `span` is the whole-source span used for the
/// (rare) lowering-gap errors.
fn lower(root: MathNode, span: (usize, usize)) -> Result<MathExpr, FrontendError> {
    let mut work: Vec<Task> = vec![Task::Node(root)];
    let mut vals: Vec<MathExpr> = Vec::new();

    // Pop a finished child off the value stack (only fails on an internal imbalance).
    macro_rules! pop {
        () => {
            vals.pop().ok_or_else(|| imbalance(span))?
        };
    }

    while let Some(task) = work.pop() {
        match task {
            // --- Decompose a node: push its assembler, then its children in REVERSE natural
            //     order so the first natural child lands on top and is processed first. ---
            // `MathNode` now implements `Drop` (iterative, see math.rs), so its fields cannot be
            // moved out of an owned value by a by-value `match` (E0509). We instead match by
            // `&mut` and lift each child out with `take_box` / `Option::take` / `mem::take`,
            // leaving a cheap leaf behind; the emptied `node` then drops shallowly at arm end.
            Task::Node(mut node) => match &mut node {
                MathNode::Num(s) => {
                    let s = std::mem::take(s);
                    vals.push(MathExpr::Number(Number::parse(&s).ok_or_else(|| {
                        FrontendError::new("latex", format!("invalid numeric literal {s:?}"), span)
                    })?))
                }
                MathNode::Sym(s) => vals.push(MathExpr::Symbol(std::mem::take(s))),
                MathNode::Text(s) => vals.push(MathExpr::Text(std::mem::take(s))),
                // `\binom{n}{k}` → the neutral binomial coefficient (no division bar).
                MathNode::Binom(x, y) => {
                    let (x, y) = (take_box(x), take_box(y));
                    work.push(Task::Build(Build::Binom));
                    work.push(Task::Node(y));
                    work.push(Task::Node(x));
                }
                // `\overset{over}{base}` / `\stackrel` → neutral Overset; `\underset` → Underset.
                // A centered annotation over/under the base, distinct from Pow/Subscript.
                // (Needs `math-frontend` ≥ 0.5.0's Overset/Underset nodes.)
                MathNode::Overset { over, base } => {
                    let (over, base) = (take_box(over), take_box(base));
                    work.push(Task::Build(Build::Overset));
                    work.push(Task::Node(base));
                    work.push(Task::Node(over));
                }
                MathNode::Underset { under, base } => {
                    let (under, base) = (take_box(under), take_box(base));
                    work.push(Task::Build(Build::Underset));
                    work.push(Task::Node(base));
                    work.push(Task::Node(under));
                }
                MathNode::Bin(op, x, y) => {
                    let bop = lower_binop(*op);
                    let (x, y) = (take_box(x), take_box(y));
                    work.push(Task::Build(Build::Bin(bop)));
                    work.push(Task::Node(y));
                    work.push(Task::Node(x));
                }
                MathNode::Rel(op, x, y) => {
                    let rop = lower_relop(*op);
                    let (x, y) = (take_box(x), take_box(y));
                    work.push(Task::Build(Build::Rel(rop)));
                    work.push(Task::Node(y));
                    work.push(Task::Node(x));
                }
                MathNode::Unary(op, x) => {
                    let op = match *op {
                        MUnOp::Neg => UnaryOp::Neg,
                        MUnOp::Pos => UnaryOp::Pos,
                    };
                    let x = take_box(x);
                    work.push(Task::Build(Build::Unary(op)));
                    work.push(Task::Node(x));
                }
                MathNode::Frac(x, y) => {
                    let (x, y) = (take_box(x), take_box(y));
                    work.push(Task::Build(Build::Frac));
                    work.push(Task::Node(y));
                    work.push(Task::Node(x));
                }
                MathNode::Root { degree, radicand } => {
                    let has_degree = degree.is_some();
                    let deg = degree.take();
                    let rad = take_box(radicand);
                    work.push(Task::Build(Build::Root { has_degree }));
                    work.push(Task::Node(rad));
                    if let Some(d) = deg {
                        work.push(Task::Node(*d));
                    }
                }
                MathNode::Script { base, sub, sup } => {
                    let (has_sub, has_sup) = (sub.is_some(), sup.is_some());
                    let sub_n = sub.take();
                    let sup_n = sup.take();
                    let base_n = take_box(base);
                    work.push(Task::Build(Build::Script { has_sub, has_sup }));
                    if let Some(p) = sup_n {
                        work.push(Task::Node(*p));
                    }
                    if let Some(s) = sub_n {
                        work.push(Task::Node(*s));
                    }
                    work.push(Task::Node(base_n));
                }
                MathNode::Call { func, arg } => {
                    let func = std::mem::take(func);
                    let arg = take_box(arg);
                    work.push(Task::Build(Build::Call(lower_func(&func))));
                    work.push(Task::Node(arg));
                }
                // A diacritical accent lowers to the neutral `MathExpr::Accent` (a mark over its
                // body) — distinct from a named-function `Call`, so `\hat{x}` stays a hat *over*
                // `x`, not the function `hat(x)`. (Needs `math-frontend` ≥ 0.4.0's Accent node.)
                MathNode::Accent { kind, body } => {
                    let kind = std::mem::take(kind);
                    let body = take_box(body);
                    work.push(Task::Build(Build::Accent(kind)));
                    work.push(Task::Node(body));
                }
                MathNode::BigOp { op, lower: lo, upper, body } => {
                    let (has_lower, has_upper) = (lo.is_some(), upper.is_some());
                    let opname = std::mem::take(op);
                    let lo_n = lo.take();
                    let up_n = upper.take();
                    let body_n = take_box(body);
                    work.push(Task::Build(Build::BigOp {
                        op: lower_bigop(&opname),
                        has_lower,
                        has_upper,
                    }));
                    work.push(Task::Node(body_n));
                    if let Some(u) = up_n {
                        work.push(Task::Node(*u));
                    }
                    if let Some(l) = lo_n {
                        work.push(Task::Node(*l));
                    }
                }
                // Fence style is presentation; the meaning is "this is grouped".
                MathNode::Fenced { body, .. } => {
                    let body = take_box(body);
                    work.push(Task::Build(Build::Group));
                    work.push(Task::Node(body));
                }
                // Matrix delimiter (pmatrix/bmatrix/cases/…) is presentation; cells carry it.
                MathNode::Matrix { rows, .. } => {
                    let rows = std::mem::take(rows);
                    let row_lens: Vec<usize> = rows.iter().map(|r| r.len()).collect();
                    work.push(Task::Build(Build::Matrix { row_lens }));
                    // Push cells in reverse so cell (0,0) is processed first.
                    for row in rows.into_iter().rev() {
                        for cell in row.into_iter().rev() {
                            work.push(Task::Node(cell));
                        }
                    }
                }
            },

            // --- Assemble a parent: children were pushed in natural order, so pop them in
            //     reverse and rebuild. ---
            Task::Build(build) => match build {
                Build::Bin(op) => {
                    let y = pop!();
                    let x = pop!();
                    vals.push(MathExpr::Bin(op, Box::new(x), Box::new(y)));
                }
                Build::Rel(op) => {
                    let y = pop!();
                    let x = pop!();
                    vals.push(MathExpr::Rel(op, Box::new(x), Box::new(y)));
                }
                Build::Unary(op) => {
                    let x = pop!();
                    vals.push(MathExpr::Unary(op, Box::new(x)));
                }
                Build::Frac => {
                    let y = pop!();
                    let x = pop!();
                    vals.push(MathExpr::Frac(Box::new(x), Box::new(y)));
                }
                Build::Binom => {
                    let k = pop!();
                    let n = pop!();
                    vals.push(MathExpr::Binom(Box::new(n), Box::new(k)));
                }
                Build::Overset => {
                    let base = pop!();
                    let over = pop!();
                    vals.push(MathExpr::Overset { over: Box::new(over), base: Box::new(base) });
                }
                Build::Underset => {
                    let base = pop!();
                    let under = pop!();
                    vals.push(MathExpr::Underset { under: Box::new(under), base: Box::new(base) });
                }
                Build::Root { has_degree } => {
                    let radicand = pop!();
                    let degree = if has_degree { Some(Box::new(pop!())) } else { None };
                    vals.push(MathExpr::Root { degree, radicand: Box::new(radicand) });
                }
                Build::Script { has_sub, has_sup } => {
                    // `a_i` → Subscript; `a^n` → Pow; `a_i^n` → Pow(Subscript(a, i), n).
                    let sup = if has_sup { Some(pop!()) } else { None };
                    let sub = if has_sub { Some(pop!()) } else { None };
                    let mut acc = pop!(); // base
                    if let Some(s) = sub {
                        acc = MathExpr::Subscript(Box::new(acc), Box::new(s));
                    }
                    if let Some(p) = sup {
                        acc = MathExpr::Bin(BinOp::Pow, Box::new(acc), Box::new(p));
                    }
                    vals.push(acc);
                }
                Build::Call(func) => {
                    let arg = pop!();
                    vals.push(MathExpr::Call { func, arg: Box::new(arg) });
                }
                Build::Accent(accent) => {
                    let body = pop!();
                    vals.push(MathExpr::Accent { accent, body: Box::new(body) });
                }
                Build::BigOp { op, has_lower, has_upper } => {
                    let body = pop!();
                    let upper = if has_upper { Some(Box::new(pop!())) } else { None };
                    let lower = if has_lower { Some(Box::new(pop!())) } else { None };
                    vals.push(MathExpr::BigOp { op, lower, upper, body: Box::new(body) });
                }
                Build::Group => {
                    let inner = pop!();
                    vals.push(MathExpr::Group(Box::new(inner)));
                }
                Build::Matrix { row_lens } => {
                    let total: usize = row_lens.iter().sum();
                    let mut flat = Vec::with_capacity(total);
                    for _ in 0..total {
                        flat.push(pop!());
                    }
                    flat.reverse(); // back to natural (0,0), (0,1), … order
                    let mut it = flat.into_iter();
                    let mut out = Vec::with_capacity(row_lens.len());
                    for len in row_lens {
                        let mut cells = Vec::with_capacity(len);
                        for _ in 0..len {
                            cells.push(it.next().ok_or_else(|| imbalance(span))?);
                        }
                        out.push(cells);
                    }
                    vals.push(MathExpr::Matrix(out));
                }
            },
        }
    }

    // Exactly one value remains: the fully-lowered root.
    match vals.len() {
        1 => Ok(vals.pop().expect("len checked")),
        _ => Err(imbalance(span)),
    }
}

/// Lower a binary operator (1:1 — every LaTeX math operator has a neutral form, including
/// `\pm`/`\mp` which map to the meaning-bearing `±`/`∓`).
fn lower_binop(op: MBinOp) -> BinOp {
    match op {
        MBinOp::Add => BinOp::Add,
        MBinOp::Sub => BinOp::Sub,
        MBinOp::Mul => BinOp::Mul,
        MBinOp::Div => BinOp::Div,
        MBinOp::Pow => BinOp::Pow,
        MBinOp::PlusMinus => BinOp::PlusMinus,
        MBinOp::MinusPlus => BinOp::MinusPlus,
    }
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
        match &m("a_i^n") {
            MathExpr::Bin(BinOp::Pow, base, _) => {
                assert!(matches!(**base, MathExpr::Subscript(..)));
            }
            other => panic!("expected Pow(Subscript,..), got {other:?}"),
        }
    }

    #[test]
    fn roots() {
        match &m(r"\sqrt[3]{x}") {
            MathExpr::Root { degree: Some(d), .. } => assert_eq!(**d, num(3)),
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
        match &m("(a+b)") {
            MathExpr::Group(inner) => assert!(matches!(**inner, MathExpr::Bin(BinOp::Add, ..))),
            other => panic!("expected Group, got {other:?}"),
        }
    }

    #[test]
    fn accent_lowers_to_neutral_accent_node() {
        // `\hat{x}` is a diacritic OVER x — the neutral `MathExpr::Accent`, distinct from the
        // named function `hat(x)` (which would be `\hat(x)` text / `\operatorname`).
        assert_eq!(
            m(r"\hat{x}"),
            MathExpr::Accent {
                accent: "hat".into(),
                body: Box::new(MathExpr::Symbol("x".into())),
            }
        );
        // A different accent over a different body is distinct, and the body lowers recursively.
        assert_eq!(
            m(r"\vec{v}"),
            MathExpr::Accent { accent: "vec".into(), body: Box::new(MathExpr::Symbol("v".into())) }
        );
    }

    #[test]
    fn overset_underset_lower_to_neutral_nodes() {
        // `\overset{a}{b}` / `\stackrel{a}{b}` → a centered over the b; `\underset{a}{b}` under it.
        // Distinct from Pow/Subscript (which `b^a` / `b_a` produce). Both args lower recursively.
        assert_eq!(
            m(r"\overset{a}{R}"),
            MathExpr::Overset {
                over: Box::new(MathExpr::Symbol("a".into())),
                base: Box::new(MathExpr::Symbol("R".into())),
            }
        );
        assert_eq!(
            m(r"\underset{a}{R}"),
            MathExpr::Underset {
                under: Box::new(MathExpr::Symbol("a".into())),
                base: Box::new(MathExpr::Symbol("R".into())),
            }
        );
        // `\stackrel` is amsmath's name for the over-set form → identical lowering.
        assert_eq!(m(r"\stackrel{a}{R}"), m(r"\overset{a}{R}"));
        // Distinct from a superscript and from each other.
        assert_ne!(m(r"\overset{a}{R}"), m(r"R^{a}"));
        assert_ne!(m(r"\overset{a}{R}"), m(r"\underset{a}{R}"));
    }

    #[test]
    fn matrix_lowers_dropping_delimiter_style() {
        match &m(r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}") {
            MathExpr::Matrix(rows) => {
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0].len(), 2);
                assert_eq!(rows[0][0], MathExpr::Symbol("a".into()));
            }
            other => panic!("expected Matrix, got {other:?}"),
        }
    }

    #[test]
    fn array_lowers_dropping_column_spec() {
        // The `{cc}` alignment argument is presentation (PFE01 §2.2) — the neutral lowering
        // drops it, so an `array` lowers to the SAME MathExpr::Matrix as the equivalent
        // pmatrix. Two source strings that mean the same math produce the same MathExpr.
        let from_array = m(r"\begin{array}{cc} a & b \\ c & d \end{array}");
        let from_pmatrix = m(r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}");
        assert_eq!(from_array, from_pmatrix);
        match &from_array {
            MathExpr::Matrix(rows) => {
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[1][1], MathExpr::Symbol("d".into()));
            }
            other => panic!("expected Matrix, got {other:?}"),
        }
    }

    #[test]
    fn xrightarrow_lowers_as_overset_on_an_arrow_symbol() {
        // A labelled arrow carries no dedicated neutral node — it reuses Overset/Underset
        // (PFE01: presentation-equivalent forms collapse). `\xrightarrow{f}` lowers to exactly
        // the same MathExpr as the explicit `\overset{f}{\rightarrow}`.
        assert_eq!(m(r"\xrightarrow{f}"), m(r"\overset{f}{\rightarrow}"));
        assert_eq!(
            m(r"\xrightarrow{f}"),
            MathExpr::Overset {
                over: Box::new(MathExpr::Symbol("f".into())),
                base: Box::new(MathExpr::Symbol("rightarrow".into())),
            }
        );
        // The optional below-label lowers to the same Underset-over-Overset as the explicit form.
        assert_eq!(
            m(r"\xrightarrow[g]{f}"),
            m(r"\underset{g}{\overset{f}{\rightarrow}}")
        );
        // Different commands keep distinct base arrows.
        assert_ne!(m(r"\xrightarrow{f}"), m(r"\xleftarrow{f}"));
    }

    #[test]
    fn numbers_stay_exact_not_f64() {
        assert_eq!(m("0.1"), MathExpr::Number(Number::parse("0.1").unwrap()));
        // and the numerator of a fraction is an exact Number, not a float.
        match &m(r"\frac{1}{3}") {
            MathExpr::Frac(n, _) => assert_eq!(**n, num(1)),
            other => panic!("expected Frac, got {other:?}"),
        }
    }

    #[test]
    fn plusminus_minusplus_and_binom_lower() {
        // The two former neutral-AST gaps now lower to real, meaning-bearing nodes.
        assert!(matches!(m(r"a \pm b"), MathExpr::Bin(BinOp::PlusMinus, _, _)));
        assert!(matches!(m(r"a \mp b"), MathExpr::Bin(BinOp::MinusPlus, _, _)));
        // \binom{n}{k} → Binom(n, k), with the arguments in source order (not swapped).
        match &m(r"\binom{n}{k}") {
            MathExpr::Binom(n, k) => {
                assert_eq!(**n, MathExpr::Symbol("n".into()));
                assert_eq!(**k, MathExpr::Symbol("k".into()));
            }
            other => panic!("expected Binom, got {other:?}"),
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
    fn deep_operator_chains_do_not_overflow() {
        // These build O(n)-deep left-spine trees that `parse_math`'s nesting `MAX_DEPTH` does
        // NOT bound (the parser builds chains with loops). A recursive lowering overflowed the
        // stack here (an uncatchable abort); the iterative trampoline must lower them fine. We
        // run on the default test-thread stack (~2 MiB), where the old recursion died at ~1000.
        let n = 4000;

        // a + a + a + … (left-nested Bin(Add))
        let add = format!("a{}", " + a".repeat(n));
        assert!(matches!(m(&add), MathExpr::Bin(BinOp::Add, _, _)));

        // a a a … (implicit multiplication → left-nested Bin(Mul))
        let mul = "a".repeat(n);
        assert!(matches!(m(&mul), MathExpr::Bin(BinOp::Mul, _, _)));

        // a = a = … (chained relation → left-nested Rel)
        // (A long *sign* run is instead bounded by the parser itself — `parse_unary` charges
        // `MAX_DEPTH` per sign — so it errors at parse time and never reaches lowering.)
        let rel = format!("a{}", " = a".repeat(n));
        assert!(matches!(m(&rel), MathExpr::Rel(RelOp::Eq, _, _)));
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
                r"a \pm b",       // ± → BinOp::PlusMinus
                r"\binom{n}{k}",  // binomial → MathExpr::Binom
                r"\frac{1}",      // parse error: span in range
                "",               // empty
            ],
        );
        assert!(report.passed(), "conformance issues: {:?}", report.issues);
    }
}
