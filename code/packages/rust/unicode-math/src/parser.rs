//! The unicode-math parser — tokens → the neutral [`MathExpr`].
//!
//! A precedence-climbing recursive-descent parser, the same shape as the AsciiMath one.
//! Precedence, lowest binds loosest:
//!
//! ```text
//!   relation   a = b   a ≤ b   a ≠ b                 (left-assoc)
//!   add/sub    a + b   a − b   a ± b   a ∓ b
//!   mul        a × b   a ⋅ b   a ÷ b   and JUXTAPOSITION (2x, xy ⇒ ·)
//!   frac       a / b                                 (built-up fraction)
//!   unary      −a   +a                               (a sign run, folded with a loop)
//!   script     a²   a₁   a₁²                         (super/subscript on one atom)
//!   atom       number | symbol | ½ | √x | ∛x | (group)
//! ```
//!
//! ## Two safety properties (both deliberate, mirroring the AsciiMath frontend)
//! * **Bounded recursion** — only *nesting* (groups, roots, scripts) recurses, and every such
//!   descent charges [`MAX_DEPTH`], so adversarial nesting returns a spanned error rather than
//!   overflowing the stack.
//! * **Loops for chains** — left-associative chains (`a+a+…`, juxtaposition) and sign runs
//!   (`−−−a`) are built with loops, not recursion, so an arbitrarily long chain costs O(1)
//!   parser stack.

use crate::token::{tokenize, Token, TokenKind};
use math_frontend::{BinOp, FrontendError, MathExpr, Number, RelOp, UnaryOp};

/// Maximum *nesting* depth before refusing with a spanned error (never overflow). Kept well
/// below what would exhaust a test-thread stack so the guard fires before the stack does.
const MAX_DEPTH: usize = 64;

/// Parse a unicode-math source string into the neutral [`MathExpr`]. Total and panic-free.
pub fn parse(src: &str) -> Result<MathExpr, FrontendError> {
    let toks = tokenize(src)?;
    let mut p = Parser { toks: &toks, pos: 0, depth: 0 };
    let e = p.parse_relation()?;
    match p.peek() {
        TokenKind::Eof => Ok(e),
        _ => Err(p.error_here("unexpected trailing input")),
    }
}

struct Parser<'a> {
    toks: &'a [Token],
    pos: usize,
    depth: usize,
}

impl Parser<'_> {
    fn peek(&self) -> &TokenKind {
        self.toks.get(self.pos).map(|t| &t.kind).unwrap_or(&TokenKind::Eof)
    }

    fn span_here(&self) -> (usize, usize) {
        self.toks.get(self.pos).or_else(|| self.toks.last()).map(|t| t.span).unwrap_or((0, 0))
    }

    fn advance(&mut self) {
        if self.pos < self.toks.len() {
            self.pos += 1;
        }
    }

    fn error_here(&self, msg: impl Into<String>) -> FrontendError {
        FrontendError::new("unicode-math", msg, self.span_here())
    }

    /// Enter one level of *nesting* (the recursion points). Returns an error at the cap.
    fn enter(&mut self) -> Result<(), FrontendError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            Err(self.error_here("expression nested too deeply"))
        } else {
            Ok(())
        }
    }
    fn exit(&mut self) {
        self.depth -= 1;
    }

    // ---- relation < add < mul < frac < unary < script < atom --------------------

    fn parse_relation(&mut self) -> Result<MathExpr, FrontendError> {
        let mut lhs = self.parse_add()?;
        while let Some(op) = rel_of(self.peek()) {
            self.advance();
            let rhs = self.parse_add()?;
            lhs = MathExpr::Rel(op, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_add(&mut self) -> Result<MathExpr, FrontendError> {
        let mut lhs = self.parse_mul()?;
        loop {
            let op = match self.peek() {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                TokenKind::PlusMinus => BinOp::PlusMinus,
                TokenKind::MinusPlus => BinOp::MinusPlus,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_mul()?;
            lhs = MathExpr::Bin(op, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_mul(&mut self) -> Result<MathExpr, FrontendError> {
        let mut lhs = self.parse_frac()?;
        loop {
            // explicit multiplicative operators
            let op = match self.peek() {
                TokenKind::Times => Some(BinOp::Mul),
                TokenKind::Div => Some(BinOp::Div),
                _ => None,
            };
            if let Some(op) = op {
                self.advance();
                let rhs = self.parse_frac()?;
                lhs = MathExpr::Bin(op, Box::new(lhs), Box::new(rhs));
                continue;
            }
            // implicit multiplication: a new simple expression begins with no operator.
            if self.starts_simple() {
                let rhs = self.parse_frac()?;
                lhs = MathExpr::Bin(BinOp::Mul, Box::new(lhs), Box::new(rhs));
                continue;
            }
            break;
        }
        Ok(lhs)
    }

    fn parse_frac(&mut self) -> Result<MathExpr, FrontendError> {
        let mut lhs = self.parse_unary()?;
        while matches!(self.peek(), TokenKind::Slash) {
            self.advance();
            let rhs = self.parse_unary()?;
            lhs = MathExpr::Frac(Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> Result<MathExpr, FrontendError> {
        // collect a sign run with a loop (no recursion → `−−−x` can't overflow)
        let mut signs: Vec<UnaryOp> = Vec::new();
        loop {
            match self.peek() {
                TokenKind::Minus => { signs.push(UnaryOp::Neg); self.advance(); }
                TokenKind::Plus => { signs.push(UnaryOp::Pos); self.advance(); }
                _ => break,
            }
        }
        let mut e = self.parse_script()?;
        for op in signs.into_iter().rev() {
            e = MathExpr::Unary(op, Box::new(e));
        }
        Ok(e)
    }

    fn parse_script(&mut self) -> Result<MathExpr, FrontendError> {
        let mut base = self.parse_atom()?;
        // subscript then superscript (`a₁²` ⇒ (a₁)²), each at most once — the natural reading.
        if let TokenKind::Sub(s) = self.peek().clone() {
            self.advance();
            base = MathExpr::Subscript(Box::new(base), Box::new(self.numeral(&s)?));
        }
        if let TokenKind::Super(s) = self.peek().clone() {
            self.advance();
            base = MathExpr::Bin(BinOp::Pow, Box::new(base), Box::new(self.numeral(&s)?));
        }
        Ok(base)
    }

    /// Build a [`MathExpr::Number`] from a normalised super/subscript numeral string, or a
    /// spanned error if it is not a well-formed numeral (e.g. a lone `⁻`).
    fn numeral(&self, s: &str) -> Result<MathExpr, FrontendError> {
        Number::parse(s)
            .map(MathExpr::Number)
            .ok_or_else(|| self.error_here(format!("invalid script numeral {s:?}")))
    }

    /// Does the upcoming token begin a *simple expression* (an atom)? Drives implicit mul.
    fn starts_simple(&self) -> bool {
        matches!(
            self.peek(),
            TokenKind::Num(_)
                | TokenKind::Sym(_)
                | TokenKind::VulgarFrac(_, _)
                | TokenKind::Sqrt
                | TokenKind::RootN(_)
                | TokenKind::LParen
                | TokenKind::LBracket
                | TokenKind::LBrace
        )
    }

    fn parse_atom(&mut self) -> Result<MathExpr, FrontendError> {
        self.enter()?;
        let result = self.parse_atom_inner();
        self.exit();
        result
    }

    fn parse_atom_inner(&mut self) -> Result<MathExpr, FrontendError> {
        match self.peek().clone() {
            TokenKind::Num(s) => {
                self.advance();
                Number::parse(&s)
                    .map(MathExpr::Number)
                    .ok_or_else(|| self.error_here(format!("invalid number literal {s:?}")))
            }
            TokenKind::Sym(s) => {
                self.advance();
                Ok(MathExpr::Symbol(s))
            }
            TokenKind::VulgarFrac(n, d) => {
                self.advance();
                Ok(MathExpr::Frac(Box::new(self.numeral(&n)?), Box::new(self.numeral(&d)?)))
            }
            TokenKind::Sqrt => {
                self.advance();
                Ok(MathExpr::Root { degree: None, radicand: Box::new(self.parse_atom()?) })
            }
            TokenKind::RootN(n) => {
                self.advance();
                Ok(MathExpr::Root {
                    degree: Some(Box::new(MathExpr::Number(Number::from_i64(n as i64)))),
                    radicand: Box::new(self.parse_atom()?),
                })
            }
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => self.parse_group(),
            _ => Err(self.error_here("expected a number, symbol, root, or '('")),
        }
    }

    /// A bracketed group: the delimiter style is dropped and the inner expression returned
    /// directly (so `(a + b)` and `[a + b]` mean the same). Any closing bracket is accepted.
    fn parse_group(&mut self) -> Result<MathExpr, FrontendError> {
        self.advance(); // opening bracket
        let inner = self.parse_relation()?;
        match self.peek() {
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                self.advance();
                Ok(inner)
            }
            _ => Err(self.error_here("expected a closing bracket")),
        }
    }
}

/// Relational operator for a token, if it is one.
fn rel_of(kind: &TokenKind) -> Option<RelOp> {
    Some(match kind {
        TokenKind::Eq => RelOp::Eq,
        TokenKind::Ne => RelOp::Ne,
        TokenKind::Lt => RelOp::Lt,
        TokenKind::Le => RelOp::Le,
        TokenKind::Gt => RelOp::Gt,
        TokenKind::Ge => RelOp::Ge,
        TokenKind::Approx => RelOp::Approx,
        TokenKind::Equiv => RelOp::Equiv,
        _ => return None,
    })
}
