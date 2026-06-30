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
use math_frontend::{BigOp, BinOp, Func, FrontendError, MathExpr, Number, RelOp, UnaryOp};

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

    /// The token `n` positions ahead (0 == current). Used for the matrix two-token lookahead.
    fn peek_nth(&self, n: usize) -> &TokenKind {
        self.toks.get(self.pos + n).map(|t| &t.kind).unwrap_or(&TokenKind::Eof)
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
        // Subscript then superscript (`a₁²` ⇒ (a₁)²), each at most once — the natural reading.
        // Either form is accepted: a Unicode glyph run (a numeral, e.g. `₁`/`²`) or the explicit
        // ASCII operator (`_`/`^`) whose operand is the next single atom (`a_i`, `x^2` ≡ `x²`).
        match self.peek().clone() {
            TokenKind::Sub(s) => {
                self.advance();
                base = MathExpr::Subscript(Box::new(base), Box::new(self.numeral(&s)?));
            }
            TokenKind::Underscore => {
                self.advance();
                base = MathExpr::Subscript(Box::new(base), Box::new(self.parse_atom()?));
            }
            _ => {}
        }
        match self.peek().clone() {
            TokenKind::Super(s) => {
                self.advance();
                base = MathExpr::Bin(BinOp::Pow, Box::new(base), Box::new(self.numeral(&s)?));
            }
            TokenKind::Caret => {
                self.advance();
                base = MathExpr::Bin(BinOp::Pow, Box::new(base), Box::new(self.parse_atom()?));
            }
            _ => {}
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
                | TokenKind::Func(_)
                | TokenKind::VulgarFrac(_, _)
                | TokenKind::Sqrt
                | TokenKind::RootN(_)
                | TokenKind::Big(_)
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
            TokenKind::Func(name) => {
                // A named function applied to the next single atom (`sin x`, `log(x)`), the same
                // "one atom argument" convention as roots — so `sin x + 1` is `(sin x) + 1`.
                self.advance();
                Ok(MathExpr::Call { func: func_of(&name), arg: Box::new(self.parse_atom()?) })
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
            TokenKind::Big(name) => {
                // A big operator (`∑ ∏ ∫ ∮ ∐`) with optional lower/upper bounds and a body.
                // Bounds attach to the operator itself (consumed here, before parse_script sees
                // any script), written either with ASCII `_`/`^` (a full atom each, so
                // `∑_(i=1)^n`) or with a Unicode sub/superscript glyph (a numeral). The body is
                // the next single atom — the same "one atom argument" rule used by roots, so
                // `∑ x + 1` is `(∑ x) + 1`.
                self.advance();
                let op = bigop_of(&name);
                let mut lower: Option<Box<MathExpr>> = None;
                let mut upper: Option<Box<MathExpr>> = None;
                loop {
                    match self.peek().clone() {
                        TokenKind::Underscore if lower.is_none() => {
                            self.advance();
                            lower = Some(Box::new(self.parse_atom()?));
                        }
                        TokenKind::Sub(s) if lower.is_none() => {
                            self.advance();
                            lower = Some(Box::new(self.numeral(&s)?));
                        }
                        TokenKind::Caret if upper.is_none() => {
                            self.advance();
                            upper = Some(Box::new(self.parse_atom()?));
                        }
                        TokenKind::Super(s) if upper.is_none() => {
                            self.advance();
                            upper = Some(Box::new(self.numeral(&s)?));
                        }
                        _ => break,
                    }
                }
                let body = self.parse_atom()?;
                Ok(MathExpr::BigOp { op, lower, upper, body: Box::new(body) })
            }
            TokenKind::LBracket | TokenKind::LParen => {
                // Two-token lookahead decides matrix vs group before consuming anything (so the
                // parse is single-pass, no backtracking): an outer bracket immediately followed by
                // another opening bracket is the matrix shape `[[…` / `((…`; everything else is
                // ordinary grouping. (Same rule as the AsciiMath frontend.)
                if matches!(self.peek_nth(1), TokenKind::LBracket | TokenKind::LParen) {
                    self.parse_matrix()
                } else {
                    self.parse_group()
                }
            }
            TokenKind::LBrace => self.parse_group(),
            _ => Err(self.error_here("expected a number, symbol, root, big operator, or '('")),
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

    /// A matrix `[[a,b],[c,d]]` (rows may use `[…]` or `(…)`), positioned at the outer opening
    /// bracket which is known to be followed by a row-opening bracket. Single-pass and committed:
    /// a malformed shape returns a spanned error (never backtracks, never panics). A 1×1 result
    /// (`((a))`, `[[a]]`) is *grouping* — the single cell is returned unwrapped; a genuine matrix
    /// has ≥2 rows or a row with ≥2 cells. (Mirrors the AsciiMath frontend.)
    fn parse_matrix(&mut self) -> Result<MathExpr, FrontendError> {
        self.enter()?; // charge the matrix nesting level (paired exit below)
        let result = self.parse_matrix_inner();
        self.exit();
        result
    }

    fn parse_matrix_inner(&mut self) -> Result<MathExpr, FrontendError> {
        self.advance(); // outer opening bracket
        let mut rows: Vec<Vec<MathExpr>> = Vec::new();
        loop {
            match self.peek() {
                TokenKind::LBracket | TokenKind::LParen => self.advance(),
                _ => return Err(self.error_here("expected a bracketed matrix row")),
            }
            let mut cells = vec![self.parse_relation()?];
            while matches!(self.peek(), TokenKind::Comma) {
                self.advance();
                cells.push(self.parse_relation()?);
            }
            match self.peek() {
                TokenKind::RBracket | TokenKind::RParen => self.advance(),
                _ => return Err(self.error_here("expected a closing bracket for the matrix row")),
            }
            rows.push(cells);
            match self.peek() {
                TokenKind::Comma => {
                    self.advance();
                    continue;
                }
                TokenKind::RBracket | TokenKind::RParen => {
                    self.advance(); // outer closing bracket
                    break;
                }
                _ => return Err(self.error_here("expected ',' or a closing bracket in the matrix")),
            }
        }
        let width = rows[0].len();
        if rows.iter().any(|r| r.len() != width) {
            return Err(self.error_here("matrix rows must all have the same length"));
        }
        if rows.len() == 1 && width == 1 {
            // `((a))` / `[[a]]` is double grouping, not a 1×1 matrix — unwrap the single cell.
            return Ok(rows.into_iter().next().and_then(|r| r.into_iter().next()).expect("1×1"));
        }
        Ok(MathExpr::Matrix(rows))
    }
}

/// The neutral [`Func`] for a recognised function name. The tokenizer only emits names that
/// [`crate::token`]'s `is_function` accepted, so `Other` is a defensive fallback (the closed
/// variants below cover the whole accepted set, matching the AsciiMath frontend's `func_of`).
fn func_of(name: &str) -> Func {
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
        _ => Func::Other(name.to_string()),
    }
}

/// The neutral [`BigOp`] for a big-operator glyph's canonical name. The tokenizer only ever
/// emits the five known names, so `Other` is a defensive fallback (never reached in practice).
fn bigop_of(name: &str) -> BigOp {
    match name {
        "sum" => BigOp::Sum,
        "prod" => BigOp::Prod,
        "int" => BigOp::Int,
        "oint" => BigOp::Oint,
        "coprod" => BigOp::Coprod,
        _ => BigOp::Other(name.to_string()),
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
