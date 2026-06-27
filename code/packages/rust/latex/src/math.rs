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
        match &self.peek().kind {
            TokenKind::Char('+') => {
                self.bump();
                Ok(MathNode::Unary(MUnOp::Pos, Box::new(self.parse_unary()?)))
            }
            TokenKind::Char('-') => {
                self.bump();
                Ok(MathNode::Unary(MUnOp::Neg, Box::new(self.parse_unary()?)))
            }
            _ => self.parse_postfix(),
        }
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
        _ => 6, // atoms: Num, Sym, Frac, Root, Fenced, Call, BigOp, Accent, Binom, Text
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
        match n {
            MathNode::Frac(a, b) => {
                assert!(matches!(*a, MathNode::Bin(MBinOp::Mul, ..)));
                assert_eq!(*b, num("3"));
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
        match n {
            MathNode::BigOp { op, lower, upper, body } => {
                assert_eq!(op, "sum");
                assert!(matches!(lower.as_deref(), Some(MathNode::Rel(MRelOp::Eq, ..))));
                assert_eq!(upper.as_deref(), Some(&sym("n")));
                assert_eq!(*body, sym("i"));
            }
            other => panic!("expected BigOp, got {other:?}"),
        }
    }

    #[test]
    fn left_right_fence_with_power() {
        // \left(\frac{a}{b}\right)^2
        let n = parse_math(r"\left(\frac{a}{b}\right)^2").unwrap();
        match n {
            MathNode::Script { base, sup, .. } => {
                assert!(matches!(*base, MathNode::Fenced { .. }));
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
        ] {
            round_trips(s);
        }
    }
}
