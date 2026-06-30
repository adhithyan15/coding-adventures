//! The math grammar (L2) — parses the **raw content of a math island**
//! ([`crate::Node::Math`]) into a [`MathNode`] tree.
//!
//! L1 keeps each `$…$` / `\[…\]` island as its exact inner source; this layer turns that
//! source into structured mathematics: fractions, roots, scripts, big operators,
//! functions, fenced groups, relations — with operator **precedence** resolved by a
//! precedence-climbing parser.
//!
//! ## Precedence (low → high)
//!
//! ```text
//!   relations  (=, <, >, \le, \ge, \ne, …)
//!   add / sub  (+, -)
//!   mul / div  (*, /, \times, \cdot, \div, \pm, \mp, AND implicit juxtaposition: 2x)
//!   unary      (leading + / -)
//!   scripts    (a^b, a_i — bind to the preceding atom)
//!   atoms      (number, symbol, {group}, (fence), \frac, \sqrt, \sum, \sin, …)
//! ```
//!
//! ## Round-trip
//!
//! Braces `{…}` are *invisible grouping*: the parser returns their inner node directly
//! (no wrapper), and [`MathNode::to_latex`] re-inserts `{…}` only where precedence
//! requires it. So `parse_math(&node.to_latex()) == node` (AST-equality). Visible
//! delimiters `(…)`, `[…]`, `|…|`, and `\left…\right` are kept as [`MathNode::Fenced`].
//!
//! Total and panic-free: every malformed input yields a spanned [`ParseError`]; recursion
//! is depth-guarded.

use crate::error::ParseError;
use crate::lexer::tokenize;
use crate::token::{Token, TokenKind};

// The precedence chain (relation→add→mul→unary→postfix→atom) descends several stack
// frames per nesting level, and `enter()` fires twice per level (relation + atom). Keep
// this conservative so the guard trips well within even a small (2 MB test-thread) stack
// rather than overflowing — real math never nests anywhere near this deep.
const MAX_DEPTH: usize = 128;

/// Binary operators in math.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MBinOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    /// `\pm`
    PlusMinus,
    /// `\mp`
    MinusPlus,
}

/// Unary prefix operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MUnOp {
    Neg,
    Pos,
}

/// Relational operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MRelOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Approx,
    Equiv,
}

/// A node in the math AST.
#[derive(Debug, Clone, PartialEq)]
pub enum MathNode {
    /// A numeric literal (digits, optional single dot), kept as written.
    Num(String),
    /// A variable or named constant. A single ASCII letter (`x`) or a command-name symbol
    /// (`pi`, `infty`, `alpha`); rendered `\pi` etc. when not a single ASCII char.
    Sym(String),
    Bin(MBinOp, Box<MathNode>, Box<MathNode>),
    Unary(MUnOp, Box<MathNode>),
    /// `\frac{A}{B}` (and `\dfrac`/`\tfrac`/`\cfrac`).
    Frac(Box<MathNode>, Box<MathNode>),
    /// `\binom{A}{B}`.
    Binom(Box<MathNode>, Box<MathNode>),
    /// `\overset{over}{base}` / `\stackrel{over}{base}` — an annotation centered OVER the base.
    Overset { over: Box<MathNode>, base: Box<MathNode> },
    /// `\underset{under}{base}` — an annotation centered UNDER the base.
    Underset { under: Box<MathNode>, base: Box<MathNode> },
    /// `\sqrt[n]{x}` (`degree` is `None` for a square root).
    Root {
        degree: Option<Box<MathNode>>,
        radicand: Box<MathNode>,
    },
    /// `base^sup_sub` — at least one of `sup`/`sub` is present.
    Script {
        base: Box<MathNode>,
        sub: Option<Box<MathNode>>,
        sup: Option<Box<MathNode>>,
    },
    /// A named function application: `\sin x`, `\ln(x)`.
    Call { func: String, arg: Box<MathNode> },
    /// A big operator with optional bound scripts: `\sum_{i=1}^{n} body`.
    BigOp {
        op: String,
        lower: Option<Box<MathNode>>,
        upper: Option<Box<MathNode>>,
        body: Box<MathNode>,
    },
    /// An accent over its argument: `\hat{x}`, `\vec{v}`.
    Accent { kind: String, body: Box<MathNode> },
    /// A visibly-delimited group: `(…)`, `[…]`, `|…|`, or `\left…\right`.
    Fenced {
        left: String,
        body: Box<MathNode>,
        right: String,
    },
    /// Prose embedded in math: `\text{…}`, `\mathrm{…}`.
    Text(String),
    /// A relation: `a = b`, `x \le y`.
    Rel(MRelOp, Box<MathNode>, Box<MathNode>),
    /// A math environment with row/column structure (L3): the `matrix` family
    /// (`matrix`/`pmatrix`/`bmatrix`/`vmatrix`/…), `cases`, the alignment environments
    /// (`aligned`/`align`), and the general `array`/`subarray` grids. `rows` is a list of
    /// rows; each row is a list of cells, split on `&` (columns) and `\\` (rows). `env` is the
    /// environment name verbatim (case-sensitive, so `bmatrix` ≠ `Bmatrix`), which also fixes
    /// the delimiters when rendered.
    ///
    /// `col_spec` is the **mandatory column-alignment argument** of `\begin{array}{…}` /
    /// `\begin{subarray}{…}` — e.g. `"ccc"`, `"l|r"`, `"p{3cm}"` — captured verbatim so the
    /// node round-trips (`to_latex` re-emits it). It is `None` for every other environment
    /// (which take no such argument). Alignment is *presentation*: the neutral `MathExpr`
    /// lowering drops `col_spec` entirely (an `array` and a `pmatrix` with the same cells lower
    /// to the same `MathExpr::Matrix`), per PFE01 §2.2.
    ///
    /// ```text
    ///   \begin{pmatrix} a & b \\ c & d \end{pmatrix}
    ///   → Matrix { env: "pmatrix", col_spec: None,        rows: [[a, b], [c, d]] }
    ///   \begin{array}{cc} a & b \\ c & d \end{array}
    ///   → Matrix { env: "array",   col_spec: Some("cc"),  rows: [[a, b], [c, d]] }
    /// ```
    Matrix {
        env: String,
        col_spec: Option<String>,
        rows: Vec<Vec<MathNode>>,
    },
}

/// Drop a [`MathNode`] **iteratively** so freeing a deeply-nested tree cannot overflow the
/// stack.
///
/// `parse_math` builds left-associative chains (`a+a+a+…`, `1/1/1/…`) and juxtaposition with
/// *loops*, not recursion — by design — so a small input like `"1" + "+1".repeat(100_000)`
/// parses fine into a `Bin(Add, Bin(Add, …))` tree nested 100k deep. But the compiler's
/// default destructor for a recursive `Box`-owning enum is itself recursive: dropping such a
/// tree would recurse 100k frames and abort the process (an uncatchable stack overflow) when
/// the `MathNode` simply goes out of scope. Since `parse_math` is public and panic-free, the
/// AST must be safe to drop at any depth — so we dismantle it with an explicit heap worklist
/// instead of the call stack. (Mirrors `math_frontend::MathExpr`'s `Drop`.)
///
/// O(1) stack depth: move each node's boxed children onto a `Vec` worklist (replacing them in
/// place with a cheap leaf), pop, repeat. By the time any node is finally dropped its children
/// are leaves, so the generated destructor recurses at most one trivial level.
impl Drop for MathNode {
    fn drop(&mut self) {
        let mut stack: Vec<MathNode> = Vec::new();
        take_children(self, &mut stack);
        while let Some(mut node) = stack.pop() {
            take_children(&mut node, &mut stack);
            // `node` now owns only leaf children, so dropping it here is shallow.
        }
    }
}

/// Move every boxed child of `n` onto `out`, leaving `n` holding cheap leaves in their place.
/// A leaf (`Num`/`Sym`/`Text`) contributes nothing. Used only by [`MathNode`]'s `Drop`.
fn take_children(n: &mut MathNode, out: &mut Vec<MathNode>) {
    // Swap a boxed child out for a leaf (no allocation: `String::new()` doesn't allocate).
    fn take(b: &mut Box<MathNode>, out: &mut Vec<MathNode>) {
        out.push(std::mem::replace(b.as_mut(), MathNode::Sym(String::new())));
    }
    fn take_opt(b: &mut Option<Box<MathNode>>, out: &mut Vec<MathNode>) {
        if let Some(boxed) = b.take() {
            out.push(*boxed);
        }
    }
    match n {
        MathNode::Num(_) | MathNode::Sym(_) | MathNode::Text(_) => {}
        MathNode::Bin(_, a, b) | MathNode::Frac(a, b) | MathNode::Binom(a, b) | MathNode::Rel(_, a, b) => {
            take(a, out);
            take(b, out);
        }
        MathNode::Overset { over: a, base: b } | MathNode::Underset { under: a, base: b } => {
            take(a, out);
            take(b, out);
        }
        MathNode::Unary(_, a) => take(a, out),
        MathNode::Root { degree, radicand } => {
            take_opt(degree, out);
            take(radicand, out);
        }
        MathNode::Script { base, sub, sup } => {
            take(base, out);
            take_opt(sub, out);
            take_opt(sup, out);
        }
        MathNode::Call { arg, .. } => take(arg, out),
        MathNode::BigOp { lower, upper, body, .. } => {
            take_opt(lower, out);
            take_opt(upper, out);
            take(body, out);
        }
        MathNode::Accent { body, .. } => take(body, out),
        MathNode::Fenced { body, .. } => take(body, out),
        MathNode::Matrix { rows, .. } => {
            for row in std::mem::take(rows) {
                out.extend(row);
            }
        }
    }
}

/// Parse LaTeX math-mode source (the inner content of an island) into a [`MathNode`].
pub fn parse_math(src: &str) -> Result<MathNode, ParseError> {
    let raw = tokenize(src)?;
    // Math mode ignores whitespace and comments; drop them up front (keep Eof).
    let toks: Vec<Token> = raw
        .into_iter()
        .filter(|t| !matches!(t.kind, TokenKind::Space | TokenKind::Par | TokenKind::Comment(_)))
        .collect();
    let mut p = MathParser { toks: &toks, pos: 0, depth: 0 };
    let node = p.parse_relation()?;
    match &p.peek().kind {
        TokenKind::Eof => Ok(node),
        _ => {
            let sp = p.peek().span;
            Err(ParseError::new("unexpected trailing tokens in math", sp.start, sp.end))
        }
    }
}

// ---- command classification tables --------------------------------------------
fn rel_op(name: &str) -> Option<MRelOp> {
    Some(match name {
        "le" | "leq" => MRelOp::Le,
        "ge" | "geq" => MRelOp::Ge,
        "ne" | "neq" => MRelOp::Ne,
        "approx" => MRelOp::Approx,
        "equiv" => MRelOp::Equiv,
        _ => return None,
    })
}
fn mul_op(name: &str) -> Option<MBinOp> {
    Some(match name {
        "times" | "cdot" | "ast" | "star" => MBinOp::Mul,
        "div" => MBinOp::Div,
        "pm" => MBinOp::PlusMinus,
        "mp" => MBinOp::MinusPlus,
        _ => return None,
    })
}
fn is_frac(name: &str) -> bool {
    matches!(name, "frac" | "dfrac" | "tfrac" | "cfrac")
}
fn is_binom(name: &str) -> bool {
    matches!(name, "binom" | "dbinom" | "tbinom")
}
fn is_function(name: &str) -> bool {
    matches!(
        name,
        "sin" | "cos" | "tan" | "cot" | "sec" | "csc" | "sinh" | "cosh" | "tanh"
            | "arcsin" | "arccos" | "arctan" | "ln" | "log" | "exp" | "min" | "max"
            | "gcd" | "lcm" | "det" | "dim" | "deg" | "arg"
    )
}
fn is_bigop(name: &str) -> bool {
    matches!(
        name,
        "sum" | "prod" | "int" | "oint" | "coprod" | "bigcup" | "bigcap" | "lim"
    )
}
fn is_accent(name: &str) -> bool {
    matches!(
        name,
        "hat" | "bar" | "vec" | "tilde" | "dot" | "ddot" | "overline" | "underline"
            | "widehat" | "widetilde"
    )
}
fn is_text(name: &str) -> bool {
    matches!(
        name,
        "text" | "mathrm" | "mathbf" | "mathit" | "mathsf" | "mathtt" | "mathcal"
            | "mathbb" | "operatorname"
    )
}
/// Math environments with `&`/`\\` row/column structure (L3). Case-sensitive — `bmatrix`
/// (square brackets) and `Bmatrix` (braces) are different environments. The `array`/`subarray`
/// grids take a **mandatory** column-spec argument (`\begin{array}{cc}`) — see
/// [`env_takes_col_spec`]; it is stored on [`MathNode::Matrix::col_spec`]. The text-mode
/// `tabular` family and document-mode list environments are deliberately **not** here; an
/// unknown `\begin{…}` is rejected with a spanned error, never mis-parsed.
fn is_math_env(name: &str) -> bool {
    matches!(
        name,
        "matrix"
            | "pmatrix"
            | "bmatrix"
            | "Bmatrix"
            | "vmatrix"
            | "Vmatrix"
            | "smallmatrix"
            | "cases"
            | "dcases"
            | "aligned"
            | "gathered"
            | "align"
            | "align*"
            | "split"
            | "array"
            | "subarray"
    )
}

/// Which math environments require the mandatory `{column-spec}` argument right after
/// `\begin{…}`. `array` (`\begin{array}{l|cr} …`) and `subarray` (`\begin{subarray}{c} …`,
/// used inside big-operator limits) do; every matrix/cases/alignment environment does not.
fn env_takes_col_spec(name: &str) -> bool {
    matches!(name, "array" | "subarray")
}

/// The `_lower` and `^upper` bound scripts captured for a big operator.
type BigOpScripts = (Option<Box<MathNode>>, Option<Box<MathNode>>);

struct MathParser<'a> {
    toks: &'a [Token],
    pos: usize,
    depth: usize,
}

impl<'a> MathParser<'a> {
    fn peek(&self) -> &Token {
        &self.toks[self.pos.min(self.toks.len() - 1)]
    }
    fn bump(&mut self) -> Token {
        let t = self.toks[self.pos.min(self.toks.len() - 1)].clone();
        if self.pos < self.toks.len() - 1 {
            self.pos += 1;
        }
        t
    }
    fn err<T>(&self, msg: impl Into<String>) -> Result<T, ParseError> {
        let sp = self.peek().span;
        Err(ParseError::new(msg, sp.start, sp.end))
    }
    fn enter(&mut self) -> Result<(), ParseError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return self.err(format!("math nesting too deep (>{MAX_DEPTH})"));
        }
        Ok(())
    }

    // relations: lowest precedence, left-associative
    fn parse_relation(&mut self) -> Result<MathNode, ParseError> {
        self.enter()?;
        let mut lhs = self.parse_add()?;
        loop {
            let op = match &self.peek().kind {
                TokenKind::Char('=') => Some(MRelOp::Eq),
                TokenKind::Char('<') => Some(MRelOp::Lt),
                TokenKind::Char('>') => Some(MRelOp::Gt),
                TokenKind::ControlWord(w) => rel_op(w),
                _ => None,
            };
            match op {
                Some(o) => {
                    self.bump();
                    let rhs = self.parse_add()?;
                    lhs = MathNode::Rel(o, Box::new(lhs), Box::new(rhs));
                }
                None => break,
            }
        }
        self.depth -= 1;
        Ok(lhs)
    }

    fn parse_add(&mut self) -> Result<MathNode, ParseError> {
        let mut lhs = self.parse_mul()?;
        loop {
            let op = match &self.peek().kind {
                TokenKind::Char('+') => Some(MBinOp::Add),
                TokenKind::Char('-') => Some(MBinOp::Sub),
                _ => None,
            };
            match op {
                Some(o) => {
                    self.bump();
                    let rhs = self.parse_mul()?;
                    lhs = MathNode::Bin(o, Box::new(lhs), Box::new(rhs));
                }
                None => break,
            }
        }
        Ok(lhs)
    }

    fn parse_mul(&mut self) -> Result<MathNode, ParseError> {
        let mut lhs = self.parse_unary()?;
        loop {
            // explicit multiplicative operator?
            let explicit = match &self.peek().kind {
                TokenKind::Char('*') => Some(MBinOp::Mul),
                TokenKind::Char('/') => Some(MBinOp::Div),
                TokenKind::ControlWord(w) => mul_op(w),
                _ => None,
            };
            if let Some(o) = explicit {
                self.bump();
                let rhs = self.parse_unary()?;
                lhs = MathNode::Bin(o, Box::new(lhs), Box::new(rhs));
                continue;
            }
            // implicit multiplication by juxtaposition (2x, a(b), \frac..\frac..)
            if self.starts_atom() {
                let rhs = self.parse_unary()?;
                lhs = MathNode::Bin(MBinOp::Mul, Box::new(lhs), Box::new(rhs));
                continue;
            }
            break;
        }
        Ok(lhs)
    }

    /// Does the current token begin a new atom (for implicit-multiplication detection)?
    fn starts_atom(&self) -> bool {
        match &self.peek().kind {
            TokenKind::Char(c) => {
                c.is_ascii_alphanumeric() || matches!(c, '(' | '[' | '|' | '.')
            }
            TokenKind::BeginGroup => true,
            TokenKind::ControlWord(w) => {
                // Operators and closers don't start a new atom: a relation (`\le`), a
                // multiplicative op (`\times`), or `\right`/`\end`. Everything else
                // (`\frac`, `\sqrt`, `\sum`, `\alpha`, …) does.
                rel_op(w).is_none()
                    && mul_op(w).is_none()
                    && !matches!(w.as_str(), "right" | "end")
            }
            _ => false,
        }
    }

    fn parse_unary(&mut self) -> Result<MathNode, ParseError> {
        // Collect a run of leading signs *iteratively* (not by self-recursion): a chain
        // like `----x` is driven directly by input length, so recursing one stack frame
        // per sign would let adversarial input (thousands of `-`) overflow the stack
        // before any depth guard fires. We gather the signs, parse one operand, then fold
        // the operators back on from the inside out.
        let mut ops = Vec::new();
        loop {
            match &self.peek().kind {
                TokenKind::Char('+') => {
                    self.bump();
                    ops.push(MUnOp::Pos);
                }
                TokenKind::Char('-') => {
                    self.bump();
                    ops.push(MUnOp::Neg);
                }
                _ => break,
            }
            // The sign run is part of one nesting level; guard it so a multi-megabyte
            // string of signs errors instead of allocating without bound.
            self.enter()?;
        }
        let mut node = self.parse_postfix()?;
        // We `enter()`ed once per sign above; balance the depth counter back out now that
        // the run is fully consumed (the operand parse has its own guarded descent).
        self.depth -= ops.len();
        for op in ops.into_iter().rev() {
            node = MathNode::Unary(op, Box::new(node));
        }
        Ok(node)
    }

    /// An atom plus any trailing `^`/`_` scripts.
    fn parse_postfix(&mut self) -> Result<MathNode, ParseError> {
        let base = self.parse_atom()?;
        let mut sub = None;
        let mut sup = None;
        loop {
            match &self.peek().kind {
                TokenKind::Superscript => {
                    self.bump();
                    if sup.is_some() {
                        return self.err("double superscript");
                    }
                    sup = Some(Box::new(self.parse_script_arg()?));
                }
                TokenKind::Subscript => {
                    self.bump();
                    if sub.is_some() {
                        return self.err("double subscript");
                    }
                    sub = Some(Box::new(self.parse_script_arg()?));
                }
                _ => break,
            }
        }
        if sub.is_none() && sup.is_none() {
            Ok(base)
        } else {
            Ok(MathNode::Script { base: Box::new(base), sub, sup })
        }
    }

    /// The argument of a `^`/`_`: a single atom, or a braced group.
    fn parse_script_arg(&mut self) -> Result<MathNode, ParseError> {
        self.parse_atom()
    }

    /// Read a mandatory argument: a `{group}` or, LaTeX-style, the next single atom.
    fn read_arg(&mut self) -> Result<MathNode, ParseError> {
        if matches!(self.peek().kind, TokenKind::BeginGroup) {
            self.read_group()
        } else {
            self.parse_atom()
        }
    }

    /// Read a `{ … }` group as a transparent sub-expression (no wrapper node).
    fn read_group(&mut self) -> Result<MathNode, ParseError> {
        if !matches!(self.peek().kind, TokenKind::BeginGroup) {
            return self.err("expected '{'");
        }
        self.bump(); // {
        let inner = self.parse_relation()?;
        if !matches!(self.peek().kind, TokenKind::EndGroup) {
            return self.err("expected '}'");
        }
        self.bump(); // }
        Ok(inner)
    }

    fn parse_atom(&mut self) -> Result<MathNode, ParseError> {
        self.enter()?;
        let node = self.parse_atom_inner()?;
        self.depth -= 1;
        Ok(node)
    }

    fn parse_atom_inner(&mut self) -> Result<MathNode, ParseError> {
        match self.peek().kind.clone() {
            TokenKind::Char(c) if c.is_ascii_digit() || c == '.' => Ok(self.read_number()),
            TokenKind::Char(c) if c.is_ascii_alphabetic() => {
                self.bump();
                Ok(MathNode::Sym(c.to_string()))
            }
            TokenKind::Char('(') => self.read_fence(')'),
            TokenKind::Char('[') => self.read_fence(']'),
            TokenKind::Char('|') => self.read_bar_fence(),
            TokenKind::BeginGroup => self.read_group(),
            TokenKind::ControlWord(name) => self.parse_command(&name),
            TokenKind::ControlSymbol(c) => {
                // e.g. \{ \} as a bare symbol
                self.bump();
                Ok(MathNode::Sym(c.to_string()))
            }
            _ => self.err("expected a math atom"),
        }
    }

    fn read_number(&mut self) -> MathNode {
        let mut s = String::new();
        let mut seen_dot = false;
        while let TokenKind::Char(c) = self.peek().kind {
            if c.is_ascii_digit() {
                s.push(c);
                self.bump();
            } else if c == '.' && !seen_dot {
                seen_dot = true;
                s.push(c);
                self.bump();
            } else {
                break;
            }
        }
        MathNode::Num(s)
    }

    /// Read `( … )` / `[ … ]` up to the matching `close` char.
    fn read_fence(&mut self, close: char) -> Result<MathNode, ParseError> {
        let open = if let TokenKind::Char(c) = self.peek().kind { c } else { unreachable!() };
        self.bump(); // opening delimiter
        let body = self.parse_relation()?;
        match self.peek().kind {
            TokenKind::Char(c) if c == close => {
                self.bump();
                Ok(MathNode::Fenced {
                    left: open.to_string(),
                    body: Box::new(body),
                    right: close.to_string(),
                })
            }
            _ => self.err(format!("expected '{close}'")),
        }
    }

    /// Read `| … |` (absolute value).
    fn read_bar_fence(&mut self) -> Result<MathNode, ParseError> {
        self.bump(); // opening |
        let body = self.parse_relation()?;
        match self.peek().kind {
            TokenKind::Char('|') => {
                self.bump();
                Ok(MathNode::Fenced {
                    left: "|".into(),
                    body: Box::new(body),
                    right: "|".into(),
                })
            }
            _ => self.err("expected closing '|'"),
        }
    }

    fn parse_command(&mut self, name: &str) -> Result<MathNode, ParseError> {
        if let Some(rel) = rel_op(name) {
            // a relation control word in atom position is malformed
            let _ = rel;
            return self.err(format!("unexpected relation \\{name}"));
        }
        if name == "begin" {
            return self.parse_environment();
        }
        if name == "end" {
            // `\end` only ever closes an environment opened by `parse_environment`; reaching
            // it here means there was no matching `\begin`.
            return self.err("unexpected \\end without matching \\begin");
        }
        if is_frac(name) {
            self.bump();
            let num = self.read_arg()?;
            let den = self.read_arg()?;
            return Ok(MathNode::Frac(Box::new(num), Box::new(den)));
        }
        if is_binom(name) {
            self.bump();
            let a = self.read_arg()?;
            let b = self.read_arg()?;
            return Ok(MathNode::Binom(Box::new(a), Box::new(b)));
        }
        // `\overset{over}{base}` / `\stackrel{over}{base}` — annotation OVER base;
        // `\underset{under}{base}` — annotation UNDER base. Two mandatory args, the
        // annotation first then the base (amsmath order), like `\frac`/`\binom`.
        if name == "overset" || name == "stackrel" {
            self.bump();
            let over = self.read_arg()?;
            let base = self.read_arg()?;
            return Ok(MathNode::Overset { over: Box::new(over), base: Box::new(base) });
        }
        if name == "underset" {
            self.bump();
            let under = self.read_arg()?;
            let base = self.read_arg()?;
            return Ok(MathNode::Underset { under: Box::new(under), base: Box::new(base) });
        }
        if name == "sqrt" {
            self.bump();
            let degree = if matches!(self.peek().kind, TokenKind::Char('[')) {
                self.bump(); // [
                let d = self.parse_relation()?;
                if !matches!(self.peek().kind, TokenKind::Char(']')) {
                    return self.err("expected ']' for \\sqrt degree");
                }
                self.bump(); // ]
                Some(Box::new(d))
            } else {
                None
            };
            let radicand = self.read_arg()?;
            return Ok(MathNode::Root { degree, radicand: Box::new(radicand) });
        }
        if name == "left" {
            self.bump();
            let left = self.read_delimiter()?;
            let body = self.parse_relation()?;
            if !matches!(&self.peek().kind, TokenKind::ControlWord(w) if w == "right") {
                return self.err("expected \\right");
            }
            self.bump(); // \right
            let right = self.read_delimiter()?;
            return Ok(MathNode::Fenced { left, body: Box::new(body), right });
        }
        if is_text(name) {
            self.bump();
            let content = self.read_raw_text_group()?;
            return Ok(MathNode::Text(content));
        }
        if is_accent(name) {
            self.bump();
            let body = self.read_arg()?;
            return Ok(MathNode::Accent { kind: name.to_string(), body: Box::new(body) });
        }
        if is_bigop(name) {
            self.bump();
            let (lower, upper) = self.read_bigop_scripts()?;
            let body = self.parse_unary()?;
            return Ok(MathNode::BigOp {
                op: name.to_string(),
                lower,
                upper,
                body: Box::new(body),
            });
        }
        if is_function(name) {
            self.bump();
            let arg = self.parse_postfix()?;
            return Ok(MathNode::Call { func: name.to_string(), arg: Box::new(arg) });
        }
        // anything else (greek letters, \infty, \partial, unknown commands) is a symbol
        self.bump();
        Ok(MathNode::Sym(name.to_string()))
    }

    /// The `_lower` / `^upper` bound scripts of a big operator, in either order.
    fn read_bigop_scripts(&mut self) -> Result<BigOpScripts, ParseError> {
        let mut lower = None;
        let mut upper = None;
        loop {
            match &self.peek().kind {
                TokenKind::Subscript if lower.is_none() => {
                    self.bump();
                    lower = Some(Box::new(self.parse_script_arg()?));
                }
                TokenKind::Superscript if upper.is_none() => {
                    self.bump();
                    upper = Some(Box::new(self.parse_script_arg()?));
                }
                _ => break,
            }
        }
        Ok((lower, upper))
    }

    /// A `\left`/`\right` delimiter: a single Char delimiter (`( [ | . )`), or a control
    /// symbol (`\{ \langle …`).
    fn read_delimiter(&mut self) -> Result<String, ParseError> {
        match self.peek().kind.clone() {
            TokenKind::Char(c) => {
                self.bump();
                Ok(c.to_string())
            }
            TokenKind::ControlWord(w) => {
                self.bump();
                Ok(format!("\\{w}"))
            }
            TokenKind::ControlSymbol(c) => {
                self.bump();
                Ok(format!("\\{c}"))
            }
            _ => self.err("expected a delimiter after \\left/\\right"),
        }
    }

    /// Capture a `{ … }` group's raw characters (for `\text{…}`).
    fn read_raw_text_group(&mut self) -> Result<String, ParseError> {
        if !matches!(self.peek().kind, TokenKind::BeginGroup) {
            return self.err("\\text must be followed by {…}");
        }
        self.bump(); // {
        let mut s = String::new();
        loop {
            match self.peek().kind.clone() {
                TokenKind::EndGroup => {
                    self.bump();
                    return Ok(s);
                }
                TokenKind::Eof => return self.err("unterminated \\text group"),
                TokenKind::Char(c) => {
                    s.push(c);
                    self.bump();
                }
                // anything else inside \text we render back approximately as a space-free
                // token; for L2's purposes text content is plain characters.
                other => {
                    return self.err(format!("unsupported token in \\text: {other:?}"));
                }
            }
        }
    }

    // ---- environments (L3) ----------------------------------------------------

    /// Parse `\begin{env} … \end{env}` into [`MathNode::Matrix`]. Called with the cursor on
    /// the `\begin` control word. The body is a grid of cells: `&` separates columns, `\\`
    /// separates rows. Nested environments work because each cell is parsed recursively (the
    /// inner `\begin…\end` is consumed by its own call), and the whole descent is
    /// depth-guarded by the enclosing [`Self::parse_atom`].
    fn parse_environment(&mut self) -> Result<MathNode, ParseError> {
        self.bump(); // \begin
        let env = self.read_env_name()?;
        if !is_math_env(&env) {
            return self.err(format!("unsupported environment: {env}"));
        }
        // `array`/`subarray` carry a mandatory `{column-spec}` argument before the grid body.
        let col_spec = if env_takes_col_spec(&env) {
            Some(self.read_col_spec(&env)?)
        } else {
            None
        };
        let rows = self.parse_env_rows(&env)?;
        Ok(MathNode::Matrix { env, col_spec, rows })
    }

    /// Read the mandatory `{column-spec}` argument of `\begin{array}{…}` (and `subarray`).
    /// The spec is captured as its literal text — column letters (`l`/`c`/`r`), `|` rules,
    /// `p{3cm}` paragraph columns, `*{n}{…}` repeats, `@{…}`/`>{…}`/`<{…}` inserts — so the
    /// node round-trips exactly; the neutral lowering drops it (alignment is presentation).
    ///
    /// Brace-nesting aware (so `p{3cm}` is captured whole) and **iterative**: a `depth`
    /// counter over a flat loop, never recursion, so an adversarial `{{{{…` cannot overflow
    /// the stack — it is bounded by the input length, like the rest of the tokenizer output.
    fn read_col_spec(&mut self, env: &str) -> Result<String, ParseError> {
        if !matches!(self.peek().kind, TokenKind::BeginGroup) {
            return self.err(format!("expected '{{column-spec}}' after \\begin{{{env}}}"));
        }
        self.bump(); // opening {
        let mut spec = String::new();
        let mut depth = 1usize;
        loop {
            match self.peek().kind.clone() {
                TokenKind::BeginGroup => {
                    depth += 1;
                    spec.push('{');
                    self.bump();
                }
                TokenKind::EndGroup => {
                    self.bump();
                    depth -= 1;
                    if depth == 0 {
                        return Ok(spec);
                    }
                    spec.push('}');
                }
                TokenKind::Char(c) => {
                    spec.push(c);
                    self.bump();
                }
                TokenKind::Space => {
                    spec.push(' ');
                    self.bump();
                }
                // `>{\centering}` etc. embed control sequences in the spec; keep them verbatim.
                TokenKind::ControlWord(w) => {
                    spec.push('\\');
                    spec.push_str(&w);
                    spec.push(' '); // a control word needs a trailing space so it re-lexes whole
                    self.bump();
                }
                TokenKind::ControlSymbol(c) => {
                    spec.push('\\');
                    spec.push(c);
                    self.bump();
                }
                TokenKind::Eof => {
                    return self.err(format!("unterminated column-spec in \\begin{{{env}}}"));
                }
                other => {
                    return self.err(format!("unexpected token in column-spec: {other:?}"));
                }
            }
        }
    }

    /// Read an environment name from the `{…}` after `\begin`/`\end`. Names are letters with
    /// an optional trailing `*` (`align*`).
    fn read_env_name(&mut self) -> Result<String, ParseError> {
        if !matches!(self.peek().kind, TokenKind::BeginGroup) {
            return self.err("expected '{' after \\begin/\\end");
        }
        self.bump(); // {
        let mut s = String::new();
        loop {
            match self.peek().kind.clone() {
                TokenKind::EndGroup => {
                    self.bump();
                    return Ok(s);
                }
                TokenKind::Char(c) => {
                    s.push(c);
                    self.bump();
                }
                TokenKind::Eof => return self.err("unterminated environment name"),
                other => return self.err(format!("unexpected token in environment name: {other:?}")),
            }
        }
    }

    /// Parse the grid body up to the matching `\end{env}`. Each cell is one math expression;
    /// truly-empty cells (e.g. `a & & b`) are a documented limitation and produce a spanned
    /// error rather than a silent empty node.
    fn parse_env_rows(&mut self, env: &str) -> Result<Vec<Vec<MathNode>>, ParseError> {
        let mut rows: Vec<Vec<MathNode>> = Vec::new();
        let mut row: Vec<MathNode> = Vec::new();
        loop {
            // Terminator: \end{name} — verify the name matches the open.
            if matches!(&self.peek().kind, TokenKind::ControlWord(w) if w == "end") {
                self.bump(); // \end
                let close = self.read_env_name()?;
                if close != env {
                    return self.err(format!("\\begin{{{env}}} closed by \\end{{{close}}}"));
                }
                if !row.is_empty() {
                    rows.push(row);
                }
                return Ok(rows);
            }
            if matches!(self.peek().kind, TokenKind::Eof) {
                return self.err(format!("unterminated environment \\begin{{{env}}}"));
            }
            // A cell. `parse_relation` stops at `&`, `\\`, and `\end` (none of them start an
            // atom), so it consumes exactly one cell and always makes progress.
            let cell = self.parse_relation()?;
            row.push(cell);
            // The separator that follows the cell.
            match &self.peek().kind {
                TokenKind::AlignTab => {
                    self.bump(); // & → next column in this row
                }
                TokenKind::ControlSymbol('\\') => {
                    self.bump(); // \\ → end this row, start a new one
                    rows.push(std::mem::take(&mut row));
                }
                // \end on the next turn closes the environment (handled at loop top).
                TokenKind::ControlWord(w) if w == "end" => {}
                TokenKind::Eof => {
                    return self.err(format!("unterminated environment \\begin{{{env}}}"));
                }
                _ => return self.err("expected '&', '\\\\', or \\end in environment"),
            }
        }
    }
}

// ---- rendering / round-trip ---------------------------------------------------

/// Precedence of a node for parenthesization during rendering (higher binds tighter).
fn prec(n: &MathNode) -> u8 {
    match n {
        MathNode::Rel(..) => 1,
        MathNode::Bin(MBinOp::Add | MBinOp::Sub | MBinOp::PlusMinus | MBinOp::MinusPlus, ..) => 2,
        MathNode::Bin(MBinOp::Mul | MBinOp::Div, ..) => 3,
        MathNode::Unary(..) => 4,
        MathNode::Bin(MBinOp::Pow, ..) | MathNode::Script { .. } => 5,
        _ => 6, // atoms: Num, Sym, Frac, Root, Fenced, Call, BigOp, Accent, Binom, Overset/Underset, Text
    }
}

impl MathNode {
    /// Render back to LaTeX math source. `parse_math(&node.to_latex()) == node`.
    pub fn to_latex(&self) -> String {
        let mut s = String::new();
        self.write(&mut s, 0);
        s
    }

    /// Write `child`, wrapping in invisible `{…}` if its precedence is below `min`.
    fn write_child(child: &MathNode, out: &mut String, min: u8) {
        if prec(child) < min {
            out.push('{');
            child.write(out, 0);
            out.push('}');
        } else {
            child.write(out, min);
        }
    }

    fn write(&self, out: &mut String, _min: u8) {
        match self {
            MathNode::Num(s) => out.push_str(s),
            MathNode::Sym(s) => {
                if s.len() == 1 && s.chars().all(|c| c.is_ascii_alphanumeric()) {
                    out.push_str(s);
                } else {
                    out.push('\\');
                    out.push_str(s);
                    out.push(' ');
                }
            }
            MathNode::Bin(op, a, b) => {
                let p = prec(self);
                let opstr = match op {
                    MBinOp::Add => " + ",
                    MBinOp::Sub => " - ",
                    MBinOp::Mul => " \\cdot ",
                    MBinOp::Div => " / ",
                    MBinOp::Pow => "^",
                    MBinOp::PlusMinus => " \\pm ",
                    MBinOp::MinusPlus => " \\mp ",
                };
                if *op == MBinOp::Pow {
                    // base binds tighter than the operator; exponent is braced
                    Self::write_child(a, out, 6);
                    out.push('^');
                    out.push('{');
                    b.write(out, 0);
                    out.push('}');
                } else {
                    Self::write_child(a, out, p);
                    out.push_str(opstr);
                    // right operand needs the next-higher precedence to stay left-assoc
                    Self::write_child(b, out, p + 1);
                }
            }
            MathNode::Unary(op, a) => {
                out.push(if *op == MUnOp::Neg { '-' } else { '+' });
                Self::write_child(a, out, 4);
            }
            MathNode::Frac(a, b) => {
                out.push_str("\\frac{");
                a.write(out, 0);
                out.push_str("}{");
                b.write(out, 0);
                out.push('}');
            }
            MathNode::Binom(a, b) => {
                out.push_str("\\binom{");
                a.write(out, 0);
                out.push_str("}{");
                b.write(out, 0);
                out.push('}');
            }
            MathNode::Overset { over, base } => {
                out.push_str("\\overset{");
                over.write(out, 0);
                out.push_str("}{");
                base.write(out, 0);
                out.push('}');
            }
            MathNode::Underset { under, base } => {
                out.push_str("\\underset{");
                under.write(out, 0);
                out.push_str("}{");
                base.write(out, 0);
                out.push('}');
            }
            MathNode::Root { degree, radicand } => {
                out.push_str("\\sqrt");
                if let Some(d) = degree {
                    out.push('[');
                    d.write(out, 0);
                    out.push(']');
                }
                out.push('{');
                radicand.write(out, 0);
                out.push('}');
            }
            MathNode::Script { base, sub, sup } => {
                Self::write_child(base, out, 6);
                if let Some(sb) = sub {
                    out.push('_');
                    out.push('{');
                    sb.write(out, 0);
                    out.push('}');
                }
                if let Some(sp) = sup {
                    out.push('^');
                    out.push('{');
                    sp.write(out, 0);
                    out.push('}');
                }
            }
            MathNode::Call { func, arg } => {
                out.push('\\');
                out.push_str(func);
                out.push(' ');
                Self::write_child(arg, out, 6);
            }
            MathNode::BigOp { op, lower, upper, body } => {
                out.push('\\');
                out.push_str(op);
                if let Some(l) = lower {
                    out.push('_');
                    out.push('{');
                    l.write(out, 0);
                    out.push('}');
                }
                if let Some(u) = upper {
                    out.push('^');
                    out.push('{');
                    u.write(out, 0);
                    out.push('}');
                }
                out.push(' ');
                Self::write_child(body, out, 4);
            }
            MathNode::Accent { kind, body } => {
                out.push('\\');
                out.push_str(kind);
                out.push('{');
                body.write(out, 0);
                out.push('}');
            }
            MathNode::Fenced { left, body, right } => {
                // `\left`-style multi-char delimiters start with a backslash.
                if left.starts_with('\\') || right.starts_with('\\') {
                    out.push_str("\\left");
                    out.push_str(left);
                    body.write(out, 0);
                    out.push_str("\\right");
                    out.push_str(right);
                } else {
                    out.push_str(left);
                    body.write(out, 0);
                    out.push_str(right);
                }
            }
            MathNode::Text(t) => {
                out.push_str("\\text{");
                out.push_str(t);
                out.push('}');
            }
            MathNode::Rel(op, a, b) => {
                let opstr = match op {
                    MRelOp::Eq => " = ",
                    MRelOp::Ne => " \\ne ",
                    MRelOp::Lt => " < ",
                    MRelOp::Le => " \\le ",
                    MRelOp::Gt => " > ",
                    MRelOp::Ge => " \\ge ",
                    MRelOp::Approx => " \\approx ",
                    MRelOp::Equiv => " \\equiv ",
                };
                Self::write_child(a, out, 1);
                out.push_str(opstr);
                Self::write_child(b, out, 2);
            }
            MathNode::Matrix { env, col_spec, rows } => {
                out.push_str("\\begin{");
                out.push_str(env);
                out.push('}');
                // `array`/`subarray` re-emit their mandatory `{column-spec}` argument.
                if let Some(spec) = col_spec {
                    out.push('{');
                    out.push_str(spec);
                    out.push('}');
                }
                for (ri, row) in rows.iter().enumerate() {
                    if ri > 0 {
                        out.push_str(" \\\\ ");
                    }
                    for (ci, cell) in row.iter().enumerate() {
                        if ci > 0 {
                            out.push_str(" & ");
                        }
                        cell.write(out, 0);
                    }
                }
                out.push_str("\\end{");
                out.push_str(env);
                out.push('}');
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn num(s: &str) -> MathNode {
        MathNode::Num(s.into())
    }
    fn sym(s: &str) -> MathNode {
        MathNode::Sym(s.into())
    }

    fn round_trips(src: &str) {
        let a = parse_math(src).expect("parse");
        let r = a.to_latex();
        let b = parse_math(&r).expect("re-parse");
        assert_eq!(a, b, "round-trip: {src:?} -> {r:?}");
    }

    #[test]
    fn overset_underset_parse_and_round_trip() {
        // Two mandatory args (annotation then base); `\stackrel` is the over-set synonym.
        assert!(matches!(
            parse_math(r"\overset{a}{R}").unwrap(),
            MathNode::Overset { .. }
        ));
        assert!(matches!(
            parse_math(r"\underset{a}{R}").unwrap(),
            MathNode::Underset { .. }
        ));
        assert!(matches!(
            parse_math(r"\stackrel{a}{R}").unwrap(),
            MathNode::Overset { .. }
        ));
        round_trips(r"\overset{a}{R}");
        round_trips(r"\underset{x+1}{y}");
        round_trips(r"a + \overset{\ast}{b}");
    }

    #[test]
    fn numbers_and_symbols() {
        assert_eq!(parse_math("12").unwrap(), num("12"));
        assert_eq!(parse_math("3.14").unwrap(), num("3.14"));
        assert_eq!(parse_math("x").unwrap(), sym("x"));
        assert_eq!(parse_math(r"\pi").unwrap(), sym("pi"));
    }

    #[test]
    fn precedence_add_mul() {
        // a + b*c → Add(a, Mul(b,c))
        let n = parse_math("a + b*c").unwrap();
        assert_eq!(
            n,
            MathNode::Bin(
                MBinOp::Add,
                Box::new(sym("a")),
                Box::new(MathNode::Bin(MBinOp::Mul, Box::new(sym("b")), Box::new(sym("c"))))
            )
        );
    }

    #[test]
    fn implicit_multiplication() {
        // 2x → Mul(2, x)
        assert_eq!(
            parse_math("2x").unwrap(),
            MathNode::Bin(MBinOp::Mul, Box::new(num("2")), Box::new(sym("x")))
        );
        // 2\pi
        assert!(matches!(parse_math(r"2\pi").unwrap(), MathNode::Bin(MBinOp::Mul, ..)));
    }

    #[test]
    fn latex_mul_div_spellings_unify() {
        for s in [r"a \times b", r"a \cdot b", "a*b"] {
            assert_eq!(
                parse_math(s).unwrap(),
                MathNode::Bin(MBinOp::Mul, Box::new(sym("a")), Box::new(sym("b")))
            );
        }
        assert_eq!(
            parse_math(r"a \div b").unwrap(),
            MathNode::Bin(MBinOp::Div, Box::new(sym("a")), Box::new(sym("b")))
        );
    }

    #[test]
    fn frac_with_nested_mul() {
        // \frac{12 \times 15}{3}
        let n = parse_math(r"\frac{12 \times 15}{3}").unwrap();
        // Match by reference — `MathNode` implements `Drop`, so a by-value `match` can't
        // move fields out (E0509). `&n` borrows; `**a` derefs &Box<MathNode> → MathNode.
        match &n {
            MathNode::Frac(a, b) => {
                assert!(matches!(**a, MathNode::Bin(MBinOp::Mul, ..)));
                assert_eq!(**b, num("3"));
            }
            other => panic!("expected Frac, got {other:?}"),
        }
    }

    #[test]
    fn power_with_braced_exponent() {
        // 2^{10}
        assert_eq!(
            parse_math("2^{10}").unwrap(),
            MathNode::Script { base: Box::new(num("2")), sub: None, sup: Some(Box::new(num("10"))) }
        );
    }

    #[test]
    fn nth_root() {
        // \sqrt[3]{27}
        assert_eq!(
            parse_math(r"\sqrt[3]{27}").unwrap(),
            MathNode::Root { degree: Some(Box::new(num("3"))), radicand: Box::new(num("27")) }
        );
    }

    #[test]
    fn sum_with_bounds() {
        // \sum_{i=1}^{n} i
        let n = parse_math(r"\sum_{i=1}^{n} i").unwrap();
        match &n {
            MathNode::BigOp { op, lower, upper, body } => {
                assert_eq!(op.as_str(), "sum");
                assert!(matches!(lower.as_deref(), Some(MathNode::Rel(MRelOp::Eq, ..))));
                assert_eq!(upper.as_deref(), Some(&sym("n")));
                assert_eq!(**body, sym("i"));
            }
            other => panic!("expected BigOp, got {other:?}"),
        }
    }

    #[test]
    fn left_right_fence_with_power() {
        // \left(\frac{a}{b}\right)^2
        let n = parse_math(r"\left(\frac{a}{b}\right)^2").unwrap();
        match &n {
            MathNode::Script { base, sup, .. } => {
                assert!(matches!(**base, MathNode::Fenced { .. }));
                assert_eq!(sup.as_deref(), Some(&num("2")));
            }
            other => panic!("expected Script over Fenced, got {other:?}"),
        }
    }

    #[test]
    fn relation_and_function() {
        assert!(matches!(parse_math("a = b").unwrap(), MathNode::Rel(MRelOp::Eq, ..)));
        assert!(matches!(parse_math(r"x \le y").unwrap(), MathNode::Rel(MRelOp::Le, ..)));
        assert!(matches!(parse_math(r"\sin x").unwrap(), MathNode::Call { .. }));
    }

    #[test]
    fn text_in_math() {
        assert_eq!(parse_math(r"\text{mg}").unwrap(), MathNode::Text("mg".into()));
    }

    #[test]
    fn errors_are_spanned_not_panics() {
        assert!(parse_math(r"\frac{1}").is_err()); // missing 2nd arg → eof
        assert!(parse_math("(a+b").is_err()); // unterminated paren
        assert!(parse_math(r"\left(x").is_err()); // missing \right
        assert!(parse_math("a^b^c").is_err()); // double superscript
    }

    #[test]
    fn deep_nesting_is_bounded() {
        let deep = "(".repeat(5000);
        assert!(parse_math(&deep).is_err());
    }

    #[test]
    fn long_sign_chain_is_bounded_not_overflow() {
        // A run of leading signs is driven by input length; it must error (depth-guarded)
        // rather than recurse a stack frame per sign and overflow.
        let signs = "-".repeat(100_000);
        assert!(parse_math(&format!("{signs}1")).is_err());
        // A short, legal sign run still parses and round-trips.
        assert!(matches!(parse_math("--x").unwrap(), MathNode::Unary(MUnOp::Neg, _)));
        round_trips("-x");
        round_trips("--x");
    }

    #[test]
    fn round_trip_corpus() {
        for s in [
            "12",
            "a + b",
            "a + b \\cdot c",
            "2x",
            "\\frac{12 \\times 15}{3}",
            "2^{10}",
            "\\sqrt[3]{27}",
            "\\sum_{i=1}^{n} i",
            "\\left(\\frac{a}{b}\\right)^2",
            "a = b + c",
            "x \\le y",
            "\\hat{x} + \\bar{y}",
            "\\pi r^2",
            "\\begin{pmatrix} a & b \\\\ c & d \\end{pmatrix}",
            "\\begin{bmatrix} 1 & 0 \\\\ 0 & 1 \\end{bmatrix}",
            "\\begin{cases} 1 & x > 0 \\\\ 0 & x \\le 0 \\end{cases}",
            "\\begin{matrix} a \\\\ b \\\\ c \\end{matrix}",
            "\\begin{pmatrix} a & b \\\\ c & d \\end{pmatrix}^2",
        ] {
            round_trips(s);
        }
    }

    // ---- L3: environments -----------------------------------------------------

    #[test]
    fn pmatrix_two_by_two() {
        let n = parse_math(r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}").unwrap();
        match &n {
            MathNode::Matrix { env, rows, .. } => {
                assert_eq!(env.as_str(), "pmatrix");
                assert_eq!(
                    rows,
                    &vec![
                        vec![sym("a"), sym("b")],
                        vec![sym("c"), sym("d")],
                    ]
                );
            }
            other => panic!("expected Matrix, got {other:?}"),
        }
    }

    #[test]
    fn bracket_delimiters_are_case_sensitive() {
        // bmatrix (square) vs Bmatrix (braces) are distinct environments.
        assert!(matches!(
            parse_math(r"\begin{bmatrix} 1 \end{bmatrix}").unwrap(),
            MathNode::Matrix { ref env, .. } if env == "bmatrix"
        ));
        assert!(matches!(
            parse_math(r"\begin{Bmatrix} 1 \end{Bmatrix}").unwrap(),
            MathNode::Matrix { ref env, .. } if env == "Bmatrix"
        ));
    }

    #[test]
    fn cases_with_conditions() {
        // \begin{cases} f & cond \\ g & cond \end{cases}
        let n = parse_math(r"\begin{cases} 1 & x > 0 \\ 0 & x \le 0 \end{cases}").unwrap();
        match &n {
            MathNode::Matrix { env, rows, .. } => {
                assert_eq!(env.as_str(), "cases");
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0].len(), 2);
                assert!(matches!(rows[0][1], MathNode::Rel(MRelOp::Gt, ..)));
                assert!(matches!(rows[1][1], MathNode::Rel(MRelOp::Le, ..)));
            }
            other => panic!("expected Matrix, got {other:?}"),
        }
    }

    #[test]
    fn cells_hold_full_expressions() {
        let n = parse_math(r"\begin{matrix} \frac{a}{b} & x^2 \\ 1+1 & c \end{matrix}").unwrap();
        match &n {
            MathNode::Matrix { rows, .. } => {
                assert!(matches!(rows[0][0], MathNode::Frac(..)));
                assert!(matches!(rows[0][1], MathNode::Script { .. }));
                assert!(matches!(rows[1][0], MathNode::Bin(MBinOp::Add, ..)));
            }
            other => panic!("expected Matrix, got {other:?}"),
        }
    }

    #[test]
    fn single_column_multiple_rows() {
        let n = parse_math(r"\begin{matrix} a \\ b \\ c \end{matrix}").unwrap();
        match &n {
            MathNode::Matrix { rows, .. } => {
                assert_eq!(rows, &vec![vec![sym("a")], vec![sym("b")], vec![sym("c")]]);
            }
            other => panic!("expected Matrix, got {other:?}"),
        }
    }

    #[test]
    fn trailing_row_separator_is_tolerated() {
        // A trailing `\\` before `\end` does not create an empty final row.
        let n = parse_math(r"\begin{matrix} a \\ b \\ \end{matrix}").unwrap();
        match &n {
            MathNode::Matrix { rows, .. } => assert_eq!(rows.len(), 2),
            other => panic!("expected Matrix, got {other:?}"),
        }
    }

    #[test]
    fn nested_environments() {
        let n = parse_math(
            r"\begin{pmatrix} \begin{pmatrix} a \end{pmatrix} & b \\ c & d \end{pmatrix}",
        )
        .unwrap();
        match &n {
            MathNode::Matrix { rows, .. } => {
                assert!(matches!(rows[0][0], MathNode::Matrix { .. }));
            }
            other => panic!("expected nested Matrix, got {other:?}"),
        }
    }

    #[test]
    fn matrix_can_be_scripted() {
        // \begin{pmatrix}…\end{pmatrix}^2 — a matrix is an atom, so postfix scripts attach.
        let n = parse_math(r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}^2").unwrap();
        match &n {
            MathNode::Script { base, sup, .. } => {
                assert!(matches!(**base, MathNode::Matrix { .. }));
                assert_eq!(sup.as_deref(), Some(&num("2")));
            }
            other => panic!("expected Script over Matrix, got {other:?}"),
        }
    }

    #[test]
    fn environment_errors_are_spanned_not_panics() {
        assert!(parse_math(r"\begin{matrix} a \end{pmatrix}").is_err()); // begin/end mismatch
        assert!(parse_math(r"\begin{matrix} a & b").is_err()); // unterminated
        assert!(parse_math(r"\begin{foobar} a \end{foobar}").is_err()); // unsupported env
        assert!(parse_math(r"\begin{matrix} & b \end{matrix}").is_err()); // empty cell (limitation)
        assert!(parse_math(r"\end{matrix}").is_err()); // stray \end
        assert!(parse_math(r"\begin matrix").is_err()); // missing { after \begin
    }

    // ---- L3: the `array` / `subarray` grids (mandatory column-spec) -------------

    #[test]
    fn array_captures_column_spec_and_cells() {
        // \begin{array}{cc} a & b \\ c & d \end{array} — the {cc} is the alignment argument,
        // captured on col_spec; the grid is the same shape as a pmatrix.
        let n = parse_math(r"\begin{array}{cc} a & b \\ c & d \end{array}").unwrap();
        match &n {
            MathNode::Matrix { env, col_spec, rows } => {
                assert_eq!(env.as_str(), "array");
                assert_eq!(col_spec.as_deref(), Some("cc"));
                assert_eq!(
                    rows,
                    &vec![vec![sym("a"), sym("b")], vec![sym("c"), sym("d")]]
                );
            }
            other => panic!("expected Matrix, got {other:?}"),
        }
    }

    #[test]
    fn array_column_spec_keeps_rules_and_alignment_letters() {
        // Vertical rules (`|`) and l/c/r letters are part of the spec and captured verbatim.
        let n = parse_math(r"\begin{array}{l|cr} 1 & 2 & 3 \end{array}").unwrap();
        match &n {
            MathNode::Matrix { col_spec, rows, .. } => {
                assert_eq!(col_spec.as_deref(), Some("l|cr"));
                assert_eq!(rows, &vec![vec![num("1"), num("2"), num("3")]]);
            }
            other => panic!("expected Matrix, got {other:?}"),
        }
    }

    #[test]
    fn array_column_spec_handles_braced_groups() {
        // `p{3cm}` is a paragraph column whose own `{…}` must be captured whole (nesting-aware).
        let n = parse_math(r"\begin{array}{p{3cm}c} a & b \end{array}").unwrap();
        match &n {
            MathNode::Matrix { col_spec, .. } => {
                assert_eq!(col_spec.as_deref(), Some("p{3cm}c"));
            }
            other => panic!("expected Matrix, got {other:?}"),
        }
    }

    #[test]
    fn subarray_takes_a_column_spec_too() {
        // \begin{subarray}{c} … \end{subarray} (used inside big-operator limits).
        let n = parse_math(r"\begin{subarray}{c} i \\ j \end{subarray}").unwrap();
        match &n {
            MathNode::Matrix { env, col_spec, rows } => {
                assert_eq!(env.as_str(), "subarray");
                assert_eq!(col_spec.as_deref(), Some("c"));
                assert_eq!(rows, &vec![vec![sym("i")], vec![sym("j")]]);
            }
            other => panic!("expected Matrix, got {other:?}"),
        }
    }

    #[test]
    fn array_round_trips_through_to_latex() {
        // parse → to_latex → parse must be a fixed point, col-spec and all.
        for src in [
            r"\begin{array}{cc} a & b \\ c & d \end{array}",
            r"\begin{array}{l|cr} 1 & 2 & 3 \end{array}",
            r"\begin{array}{p{3cm}c} a & b \end{array}",
            r"\begin{subarray}{c} i \\ j \end{subarray}",
        ] {
            let once = parse_math(src).unwrap();
            let twice = parse_math(&once.to_latex()).unwrap();
            assert_eq!(once, twice, "round-trip changed the tree for {src:?}");
        }
    }

    #[test]
    fn matrix_family_still_has_no_column_spec() {
        // Environments that take no alignment argument keep col_spec == None.
        let n = parse_math(r"\begin{pmatrix} a \end{pmatrix}").unwrap();
        assert!(matches!(n, MathNode::Matrix { col_spec: None, .. }));
    }

    #[test]
    fn array_without_column_spec_is_a_spanned_error() {
        // The `{col-spec}` argument is mandatory — its absence is a clean error, not a panic.
        assert!(parse_math(r"\begin{array} a & b \end{array}").is_err());
        assert!(parse_math(r"\begin{array}{cc} a & b").is_err()); // unterminated body
        assert!(parse_math(r"\begin{array}{cc a \end{array}").is_err()); // unterminated col-spec
    }

    #[test]
    fn deep_column_spec_braces_do_not_overflow() {
        // An adversarial `{{{{…}}}}` inside the col-spec is captured by a flat depth counter,
        // never recursion, so it cannot overflow the stack.
        let spec = format!("{}{}", "{".repeat(20_000), "}".repeat(20_000));
        let src = format!(r"\begin{{array}}{{{spec}}} a \end{{array}}");
        assert!(parse_math(&src).is_ok());
    }

    // ---- deep-tree Drop safety -------------------------------------------------
    //
    // The parser's `parse_add`/`parse_mul`/`parse_relation` build LEFT-nested chains via
    // loops with no per-term depth charge, so a long chain like `1+1+1+…` yields an
    // O(n)-deep `Bin` tree even though MAX_DEPTH bounds nesting. The *compiler-generated*
    // destructor for such a tree recurses one stack frame per level → stack overflow (an
    // uncatchable abort) on a deep-enough tree. `impl Drop for MathNode` dismantles the
    // tree with a heap worklist instead, so these must complete without overflowing.

    #[test]
    fn deep_left_nested_tree_drops_without_overflow() {
        // Build `((…((1+1)+1)+…)+1)` 200k levels deep directly (bypassing the parser's
        // depth limits), then let it drop at end of scope. Pre-Drop-impl this aborted.
        let mut node = num("1");
        for _ in 0..200_000 {
            node = MathNode::Bin(MBinOp::Add, Box::new(node), Box::new(num("1")));
        }
        // Touch it so the optimizer can't elide construction, then drop implicitly.
        assert!(matches!(node, MathNode::Bin(MBinOp::Add, ..)));
    }

    #[test]
    fn deep_parsed_chain_drops_without_overflow() {
        // The same hazard but reached through the real parser: a long `+` chain parses to
        // a deep left-nested tree. Keep it under MAX_DEPTH-per-nesting (these are loop
        // iterations, not nesting) but long enough that a recursive Drop would overflow.
        let src = format!("1{}", "+1".repeat(50_000));
        let tree = parse_math(&src).expect("long additive chain parses");
        drop(tree); // explicit: the heap-worklist Drop must not overflow
    }

    #[test]
    fn deep_unary_chain_drops_without_overflow() {
        // A `Unary` spine (single-child nodes) is the other deep shape; verify it too.
        let mut node = num("1");
        for _ in 0..200_000 {
            node = MathNode::Unary(MUnOp::Neg, Box::new(node));
        }
        assert!(matches!(node, MathNode::Unary(MUnOp::Neg, _)));
    }
}
