//! The AsciiMath parser — tokens → the neutral [`MathExpr`].
//!
//! A precedence-climbing recursive-descent parser. Precedence, lowest binds loosest:
//!
//! ```text
//!   relation   a = b   a <= b   a != b              (left-assoc)
//!   add/sub    a + b   a - b
//!   mul        a * b   a xx b   a cdot b   a -: b    and JUXTAPOSITION (a b ⇒ a·b)
//!   frac       a / b                                 (binds the adjacent simple exprs)
//!   unary      -a   +a                               (a sign *run* is folded with a loop)
//!   script     a^b   a_b   a_b^c                     (operand is one atom)
//!   atom       number | symbol | f(x) | sqrt x | root(n)(x) | (group) | "text"
//! ```
//!
//! ## Two safety properties (both deliberate)
//! * **Bounded recursion.** Only *nesting* (groups, scripts, function/root arguments)
//!   recurses, and every such descent charges [`MAX_DEPTH`]; adversarial nesting therefore
//!   returns a spanned error instead of overflowing the stack.
//! * **Loops for chains.** Left-associative chains (`a+a+…`, juxtaposition, `a/a/…`) and
//!   sign runs (`---a`) are built with `while`/`for` loops, *not* recursion — so an
//!   arbitrarily long chain costs O(1) parser stack. (The neutral tree it builds is still
//!   left-nested; dropping a *pathologically* deep such tree is a separate concern tracked
//!   for the math-frontend AST, not introduced here.)

use crate::token::{tokenize, Token, TokenKind};
use math_frontend::{BigOp, BinOp, Func, FrontendError, MathExpr, Number, RelOp, UnaryOp};

/// Maximum *nesting* depth before we refuse with a spanned error (never overflow).
///
/// Each nesting level descends ~9 parser functions (`relation→…→atom→group`), so the cap is
/// kept well below what would exhaust a 2 MiB test-thread stack — the guard must fire *before*
/// the stack does. Real AsciiMath never nests anywhere near this deep; adversarial input that
/// does gets a clean spanned error instead of an abort.
const MAX_DEPTH: usize = 64;

/// Parse an AsciiMath source string into the neutral [`MathExpr`]. Total and panic-free.
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
        // `tokenize` always appends Eof, so indexing the last token is safe; the fallback
        // keeps this total even if that invariant were ever violated.
        self.toks.get(self.pos).map(|t| &t.kind).unwrap_or(&TokenKind::Eof)
    }

    /// The token `n` positions ahead (0 == current). Used for the matrix two-token lookahead.
    fn peek_nth(&self, n: usize) -> &TokenKind {
        self.toks.get(self.pos + n).map(|t| &t.kind).unwrap_or(&TokenKind::Eof)
    }

    fn span_here(&self) -> (usize, usize) {
        self.toks
            .get(self.pos)
            .or_else(|| self.toks.last())
            .map(|t| t.span)
            .unwrap_or((0, 0))
    }

    fn advance(&mut self) {
        if self.pos < self.toks.len() {
            self.pos += 1;
        }
    }

    fn error_here(&self, msg: impl Into<String>) -> FrontendError {
        FrontendError::new("asciimath", msg, self.span_here())
    }

    /// Enter one level of *nesting* (the recursion points). Returns an error at the cap so
    /// deeply-nested input fails cleanly rather than overflowing the call stack.
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
            // Explicit multiplicative operators: `*`/`**`, `xx`, `cdot`, `/`-as-`-:`, `div`.
            let op = match self.peek() {
                TokenKind::Star => Some(BinOp::Mul),
                TokenKind::Div => Some(BinOp::Div),
                TokenKind::Ident(w) if w == "xx" || w == "cdot" => Some(BinOp::Mul),
                TokenKind::Ident(w) if w == "div" => Some(BinOp::Div),
                _ => None,
            };
            if let Some(op) = op {
                self.advance();
                let rhs = self.parse_frac()?;
                lhs = MathExpr::Bin(op, Box::new(lhs), Box::new(rhs));
                continue;
            }
            // Implicit multiplication: a new simple expression begins with no operator.
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
        // Collect a sign run with a loop (no recursion → `------x` can't overflow).
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
        // AsciiMath order: subscript then superscript (`a_i^2` ⇒ (a_i)^2). Each operand is
        // a single atom (so `x^2y` is (x^2)·y, the AsciiMath reading).
        if matches!(self.peek(), TokenKind::Underscore) {
            self.advance();
            let sub = self.parse_atom()?;
            base = MathExpr::Subscript(Box::new(base), Box::new(sub));
        }
        if matches!(self.peek(), TokenKind::Caret) {
            self.advance();
            let sup = self.parse_atom()?;
            base = MathExpr::Bin(BinOp::Pow, Box::new(base), Box::new(sup));
        }
        Ok(base)
    }

    /// Does the upcoming token begin a *simple expression* (an atom)? Drives implicit
    /// multiplication — and excludes the operator-words (`xx`/`cdot`/`div`) so they are not
    /// mistaken for operands.
    fn starts_simple(&self) -> bool {
        match self.peek() {
            TokenKind::Num(_) | TokenKind::Text(_) => true,
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => true,
            TokenKind::Ident(w) => !matches!(w.as_str(), "xx" | "cdot" | "div"),
            _ => false,
        }
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
                match Number::parse(&s) {
                    Some(n) => Ok(MathExpr::Number(n)),
                    None => Err(self.error_here(format!("invalid number literal {s:?}"))),
                }
            }
            TokenKind::Text(s) => {
                self.advance();
                Ok(MathExpr::Text(s))
            }
            TokenKind::LBracket | TokenKind::LParen => {
                // Two-token lookahead decides matrix vs group *before* consuming anything, so
                // the parse is single-pass (no backtracking). An outer bracket immediately
                // followed by another opening bracket is the matrix shape `[[…` / `((…`;
                // everything else is ordinary grouping. Committing here (rather than trying
                // then backtracking) keeps cost linear: a deep run like `(((…` descends once,
                // depth-charged, and fails cleanly at MAX_DEPTH instead of re-parsing
                // exponentially.
                if matches!(self.peek_nth(1), TokenKind::LBracket | TokenKind::LParen) {
                    self.parse_matrix()
                } else {
                    self.parse_group()
                }
            }
            TokenKind::LBrace => self.parse_group(),
            TokenKind::Ident(word) => self.parse_ident_atom(&word),
            _ => Err(self.error_here("expected a number, symbol, or '('")),
        }
    }

    /// Parse a matrix `[[a,b],[c,d]]` (rows may use `[…]` or `(…)`), positioned at the outer
    /// opening bracket which is known to be followed by a row-opening bracket. Single-pass and
    /// committed: a malformed shape returns a spanned error (never backtracks, never panics).
    ///
    /// Disambiguation from nested grouping: a 1×1 result (`((a))`, `[[a]]`) is *grouping* — the
    /// single cell is returned unwrapped. A genuine matrix has ≥2 rows or a row with ≥2 cells.
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
            // Each outer item must itself be a bracketed row.
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

    /// A bracketed group. With NO commas, parentheses are *grouping only* — the delimiter style
    /// is dropped and the inner expression is returned directly (so `sqrt(x)` ≡ `sqrt x` and
    /// `(1)/(2)` ≡ `1/2`). With one or more top-level commas, the fence is a LIST — `(a, b, c)`
    /// → `MathExpr::Sequence([a, b, c])` — so the commas are preserved as list structure (a
    /// coordinate tuple, an argument list) rather than rejected. Each item is a full relation.
    /// Any closing bracket is accepted (AsciiMath treats them loosely). Note the matrix shape
    /// `((a,b),(c,d))` is handled earlier by `parse_matrix` (an outer bracket immediately
    /// followed by another opening bracket); this path sees only single-fence groups/lists.
    fn parse_group(&mut self) -> Result<MathExpr, FrontendError> {
        self.advance(); // opening bracket
        let first = self.parse_relation()?;
        let expr = if matches!(self.peek(), TokenKind::Comma) {
            // Comma-separated list → Sequence. Mirrors the matrix row's cell loop: a bounded
            // loop over `parse_relation` (each item already depth-charged), never recursion, so
            // a wide `(a, b, …, z)` cannot overflow the stack. A trailing/doubled comma leaves a
            // non-atom before the next `parse_relation`, which returns a clean spanned error.
            let mut items = vec![first];
            while matches!(self.peek(), TokenKind::Comma) {
                self.advance();
                items.push(self.parse_relation()?);
            }
            MathExpr::Sequence(items)
        } else {
            first
        };
        match self.peek() {
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                self.advance();
                Ok(expr)
            }
            _ => Err(self.error_here("expected a closing bracket")),
        }
    }

    fn parse_ident_atom(&mut self, word: &str) -> Result<MathExpr, FrontendError> {
        // Big operator with optional bounds: `sum_(i=1)^n i`, `int_a^b f`, `lim_(x->0) f`.
        // The bounds attach to the operator itself (consumed here, before parse_script sees
        // any `_`/`^`); the body is the next single atom — the same "one atom argument"
        // convention used by `sqrt`/functions (so `sum_(i=1)^n i + 1` is (sum … i) + 1).
        if let Some(op) = bigop_of(word) {
            self.advance();
            let mut lower: Option<Box<MathExpr>> = None;
            let mut upper: Option<Box<MathExpr>> = None;
            // Accept `_`/`^` in either order, each at most once.
            loop {
                match self.peek() {
                    TokenKind::Underscore if lower.is_none() => {
                        self.advance();
                        lower = Some(Box::new(self.parse_atom()?));
                    }
                    TokenKind::Caret if upper.is_none() => {
                        self.advance();
                        upper = Some(Box::new(self.parse_atom()?));
                    }
                    _ => break,
                }
            }
            let body = self.parse_atom()?;
            return Ok(MathExpr::BigOp { op, lower, upper, body: Box::new(body) });
        }
        // Function application: `sin x`, `ln(x)` — the argument is the next atom.
        if let Some(func) = func_of(word) {
            self.advance();
            let arg = self.parse_atom()?;
            return Ok(MathExpr::Call { func, arg: Box::new(arg) });
        }
        // Diacritical accent: `hat x`, `bar y`, `vec v`, `dot x`, `ddot x`, `tilde a`, `ul x`.
        // The accented thing is the next single atom (same "one atom argument" convention as
        // `sqrt`/functions), lowered to the neutral `MathExpr::Accent` — a mark OVER the body,
        // distinct from a function `Call`. (Needs `math-frontend` >= 0.4.0's Accent node.)
        if let Some(accent) = accent_of(word) {
            self.advance();
            let body = self.parse_atom()?;
            return Ok(MathExpr::Accent { accent: accent.to_string(), body: Box::new(body) });
        }
        // Over/under-set annotations: `overset(a)(b)`/`stackrel(a)(b)` and `underset(a)(b)`
        // (also the paren-free `stackrel a b` form). TWO atoms — the annotation then the base —
        // lowered to the neutral `MathExpr::Overset`/`Underset` (a sub-expression centered OVER /
        // UNDER the base, distinct from `Pow`/`Subscript`). Needs `math-frontend` >= 0.5.0.
        // `stackrel` is LaTeX's name for the over-set form; AsciiMath accepts it as a synonym.
        match word {
            "overset" | "stackrel" => {
                self.advance();
                let over = self.parse_atom()?;
                let base = self.parse_atom()?;
                return Ok(MathExpr::Overset { over: Box::new(over), base: Box::new(base) });
            }
            "underset" => {
                self.advance();
                let under = self.parse_atom()?;
                let base = self.parse_atom()?;
                return Ok(MathExpr::Underset { under: Box::new(under), base: Box::new(base) });
            }
            _ => {}
        }
        match word {
            "sqrt" => {
                self.advance();
                let radicand = self.parse_atom()?;
                Ok(MathExpr::Root { degree: None, radicand: Box::new(radicand) })
            }
            "root" => {
                // `root(n)(x)` or `root n x`: degree then radicand, each one atom.
                self.advance();
                let degree = self.parse_atom()?;
                let radicand = self.parse_atom()?;
                Ok(MathExpr::Root { degree: Some(Box::new(degree)), radicand: Box::new(radicand) })
            }
            "xx" | "cdot" | "div" => {
                // A multiplicative operator with no left operand.
                Err(self.error_here(format!("'{word}' needs a left operand")))
            }
            _ => {
                self.advance();
                if let Some(canon) = constant_of(word) {
                    Ok(MathExpr::Symbol(canon.to_string()))
                } else if word.chars().count() == 1 {
                    Ok(MathExpr::Symbol(word.to_string()))
                } else {
                    // A bare multi-letter run is the implicit product of its single letters
                    // (`xy` ⇒ x·y) — the AsciiMath convention. Built left-assoc with a loop.
                    let mut chars = word.chars();
                    let first = chars.next().expect("identifier is non-empty");
                    let mut e = MathExpr::Symbol(first.to_string());
                    for ch in chars {
                        e = MathExpr::Bin(
                            BinOp::Mul,
                            Box::new(e),
                            Box::new(MathExpr::Symbol(ch.to_string())),
                        );
                    }
                    Ok(e)
                }
            }
        }
    }
}

/// Relational operator for a token, if it is one (`=`, `!=`, `<`, `<=`, `>`, `>=`, `~~`, `-=`).
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

/// Does `word` name a multi-letter keyword the parser treats specially — a named function
/// (`sin`), big operator (`sum`), accent (`hat`), constant/symbol (`pi`), one of the structural
/// words `sqrt`/`root`/`overset`/`stackrel`/`underset`, or a multiplicative operator word
/// (`xx`/`cdot`/`div`)?
///
/// This is the **single source of truth** for "is this a keyword", reused by the tokenizer's
/// longest-match scan ([`crate::token`]) so a glued run like `sinx` splits as `sin`·`x` and
/// `pir` as `pi`·`r` (AsciiMath's greedy rule), instead of the letter product `s·i·n·x`. It
/// delegates to the very same lookup tables the parser dispatches on, so the two can never
/// drift apart — adding a function/symbol here is automatic.
///
/// The operator words `xx`/`cdot`/`div` MUST be listed: the scan takes the longest keyword at
/// the *start* of a run, so without `cdot` here the word `cdot` would find no keyword prefix,
/// peel its leading `c`, and then mis-split the trailing `dot` as the accent — `cdot` ⇒
/// `c·hat-less dot`. Listing them makes the scan take `cdot`/`xx`/`div` whole (the parser then
/// reads them as the operator they are). The bare `text` word is intentionally *absent*: the
/// `text(…)` form is a tokenizer concern, and a lone `text` contains no keyword segment, so it
/// already stays one identifier. Single letters are never keywords, so the tokenizer only ever
/// calls this with length ≥ 2.
pub(crate) fn is_keyword(word: &str) -> bool {
    func_of(word).is_some()
        || bigop_of(word).is_some()
        || accent_of(word).is_some()
        || constant_of(word).is_some()
        || matches!(word, "sqrt" | "root" | "overset" | "stackrel" | "underset" | "xx" | "cdot" | "div")
}

/// Map an identifier to a big operator, or `None`. `int`=∫, `oint`=∮, `prod`=∏, `coprod`=∐.
/// Map an AsciiMath accent keyword to its canonical neutral accent name (the `accent` field of
/// [`MathExpr::Accent`]), or `None` if `word` is not an accent. AsciiMath spells accents as
/// prefix words taking one argument: `hat x`, `bar y`/`overline y`, `vec v`, `dot x`, `ddot x`,
/// `tilde a`, `ul x`/`underline x`. We normalise synonyms to one canonical name so two spellings
/// of the same mark lower equal.
fn accent_of(word: &str) -> Option<&'static str> {
    Some(match word {
        "hat" => "hat",
        "bar" | "overline" => "bar",
        "ul" | "underline" => "underline",
        "vec" => "vec",
        "dot" => "dot",
        "ddot" => "ddot",
        "tilde" => "tilde",
        _ => return None,
    })
}

fn bigop_of(word: &str) -> Option<BigOp> {
    Some(match word {
        "sum" => BigOp::Sum,
        "prod" => BigOp::Prod,
        "int" => BigOp::Int,
        "oint" => BigOp::Oint,
        "coprod" => BigOp::Coprod,
        "lim" => BigOp::Lim,
        _ => return None,
    })
}

/// Map an identifier to a named [`Func`], or `None` if it is not a known function.
fn func_of(word: &str) -> Option<Func> {
    Some(match word {
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
        _ => return None,
    })
}

/// Map an identifier to a canonical constant/symbol name, or `None` for an ordinary variable.
///
/// This is the AsciiMath **symbol table**: a fixed dictionary of multi-letter words that name a
/// single mathematical glyph rather than a product of single-letter variables. (Without it, `pi`
/// would parse as `p · i` under the implicit-product rule — see [`parse_ident_atom`].) Every entry
/// lowers to a [`MathExpr::Symbol`] carrying the canonical name; symbol emission is *not* a
/// declared [`Capabilities`] flag, so growing this table is purely additive — no consumer or
/// conformance change.
///
/// Naming convention: lowercase Greek is kept verbatim (`alpha`); uppercase Greek is capitalized
/// (`Sigma`); the blackboard number sets get word names (`reals`); arrows and set/logic operators
/// use their familiar TeX-ish long names (`rightarrow`, `subseteq`, `forall`) so that two notations
/// for the same glyph can later agree on one `Symbol` string. `oo`/`infty` canonicalize to
/// `infinity`.
///
/// Deferred to PR-3b (documented in the ASM01 spec): the bare English keyword spellings `in`, `and`,
/// `or`, `not` (they need care — e.g. `in` also appears inside big-operator bounds such as
/// `sum_(i in S)`), AsciiMath's two-letter short forms (`sub`, `sup`, `uu`, `nn`, `AA`, `EE`), and
/// punctuation arrows (`->`, `=>`) which are a tokenizer concern, not an identifier word.
fn constant_of(word: &str) -> Option<&'static str> {
    Some(match word {
        // ── Greek, lowercase ──────────────────────────────────────────────────────────────────
        "alpha" => "alpha",
        "beta" => "beta",
        "gamma" => "gamma",
        "delta" => "delta",
        "epsilon" => "epsilon",
        "zeta" => "zeta",
        "eta" => "eta",
        "theta" => "theta",
        "iota" => "iota",
        "kappa" => "kappa",
        "lambda" => "lambda",
        "mu" => "mu",
        "nu" => "nu",
        "xi" => "xi",
        "omicron" => "omicron",
        "pi" => "pi",
        "rho" => "rho",
        "sigma" => "sigma",
        "tau" => "tau",
        "upsilon" => "upsilon",
        "phi" => "phi",
        "chi" => "chi",
        "psi" => "psi",
        "omega" => "omega",
        // Variant glyphs are distinct symbols, kept under their `var…` names.
        "varepsilon" => "varepsilon",
        "vartheta" => "vartheta",
        "varpi" => "varpi",
        "varrho" => "varrho",
        "varsigma" => "varsigma",
        "varphi" => "varphi",
        // ── Greek, uppercase (only the visually-distinct letters AsciiMath spells capitalized) ──
        "Gamma" => "Gamma",
        "Delta" => "Delta",
        "Theta" => "Theta",
        "Lambda" => "Lambda",
        "Xi" => "Xi",
        "Pi" => "Pi",
        "Sigma" => "Sigma",
        "Upsilon" => "Upsilon",
        "Phi" => "Phi",
        "Psi" => "Psi",
        "Omega" => "Omega",
        // ── Blackboard number sets ────────────────────────────────────────────────────────────
        "NN" => "naturals",
        "ZZ" => "integers",
        "QQ" => "rationals",
        "RR" => "reals",
        "CC" => "complexes",
        // ── Arrows (word forms; punctuation arrows are deferred to the tokenizer) ──────────────
        "rarr" | "rightarrow" => "rightarrow",
        "larr" | "leftarrow" => "leftarrow",
        "harr" | "leftrightarrow" => "leftrightarrow",
        "uarr" | "uparrow" => "uparrow",
        "darr" | "downarrow" => "downarrow",
        "implies" => "implies",
        "iff" => "iff",
        "mapsto" => "mapsto",
        // ── Set / logic operators (long names; bare keywords deferred to PR-3b) ────────────────
        "notin" => "notin",
        "subset" => "subset",
        "subseteq" => "subseteq",
        "supset" => "supset",
        "supseteq" => "supseteq",
        "cup" => "union",
        "cap" => "intersection",
        "emptyset" => "emptyset",
        "forall" => "forall",
        "exists" => "exists",
        "aleph" => "aleph",
        // ── PR-3b: AsciiMath two-letter short forms + bare-keyword spellings ───────────────────
        // The compact AsciiMath spellings of the set/logic operators above, plus the four English
        // keywords. Like every other entry these lower to `Symbol` — there is no `In`/`Subset`
        // relation in the neutral `RelOp`, so a symbol standing for the glyph is the faithful
        // representation (and `i in S` is then the juxtaposition `i · ∈ · S`, which is harmless
        // inside a big-operator bound like `sum_(i in S)` — no parse breakage).
        "in" => "in",
        "and" => "and",
        "or" => "or",
        "not" => "not",
        "sub" => "subset",     // short form of `subset`
        "sube" => "subseteq",  // short form of `subseteq`
        "sup" => "supset",     // short form of `supset`
        "supe" => "supseteq",  // short form of `supseteq`
        "uu" => "union",       // short form of `cup`
        "nn" => "intersection", // short form of `cap`
        "AA" => "forall",      // ∀
        "EE" => "exists",      // ∃
        // ── Misc. operators / relations / decoration ──────────────────────────────────────────
        "partial" => "partial",
        "nabla" | "grad" => "nabla",
        "propto" | "prop" => "propto",
        "perp" => "perp",
        "angle" => "angle",
        "deg" => "degree",
        "ldots" => "ldots",
        "cdots" => "cdots",
        "vdots" => "vdots",
        "ddots" => "ddots",
        // ── Infinity ──────────────────────────────────────────────────────────────────────────
        "oo" | "infty" => "infinity",
        _ => return None,
    })
}
