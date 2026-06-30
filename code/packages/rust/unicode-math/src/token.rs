//! The unicode-math tokenizer — a small, total, panic-free scanner over **Unicode** math text.
//!
//! Unlike the ASCII-only AsciiMath scanner, this frontend reads the math people and models
//! actually type with real glyphs — `x² + y² = r²`, `√x`, `½`, `π·α`, `a ≤ b`, `2∓1` — so we
//! scan by **codepoint** (not byte), recording a half-open *byte* `Span` for every token (so a
//! caller can still slice the original `&str` to underline an error). The scanner recognises:
//!
//! * **numbers** — `42`, `3.14`, `6.022e23` (ASCII digits; [`Number`] validates the text);
//! * **symbols** — a single ASCII letter (`x`) or a Greek / constant glyph (`π`→`pi`, `Σ`→`Sigma`,
//!   `∞`→`infinity`). Each is ONE token; a run like `xy` is two tokens the parser multiplies,
//!   mirroring the AsciiMath convention (there are no multi-letter variables here);
//! * **superscripts** — a maximal run of `⁰¹²³⁴⁵⁶⁷⁸⁹⁺⁻` normalised to a plain numeral string
//!   (`x²`→`Super("2")`, `x⁻¹`→`Super("-1")`); the parser turns it into a power;
//! * **subscripts** — a maximal run of `₀₁₂₃₄₅₆₇₈₉₊₋` (`a₁`→`Sub("1")`) → a subscript;
//! * **vulgar fractions** — `½ ⅓ ¼ ¾ ⅔ …` → a ready-built `VulgarFrac("1","2")`;
//! * **operators** — `+`, `-`/`−`(U+2212), `×`/`⋅`/`*`(→×), `÷`(→÷), `/`(built-up fraction),
//!   `±`, `∓`, and the roots `√`/`∛`/`∜`;
//! * **relations** — `=`, `≠`, `<`, `≤`, `>`, `≥`, `≈`, `≡`;
//! * **brackets** — `( ) [ ] { }`.
//!
//! Whitespace is skipped. Anything else (including an out-of-scope glyph such as `∑`) is a
//! *returned* spanned error, never a panic, so the tokenizer is total: every input yields
//! `Ok(tokens)` or one `Err(FrontendError)`.

use math_frontend::FrontendError;

/// A half-open byte span `[start, end)` into the source.
pub type Span = (usize, usize);

/// The lexical categories the unicode-math frontend recognises.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    /// A numeric literal, exact source text (e.g. `"3.14"`).
    Num(String),
    /// A single variable letter, or the canonical name of a constant/Greek glyph (`"pi"`).
    Sym(String),
    /// A superscript run normalised to a plain numeral (`"2"`, `"-1"`) — becomes a power.
    Super(String),
    /// A subscript run normalised to a plain numeral (`"1"`) — becomes a subscript.
    Sub(String),
    /// A vulgar-fraction glyph already split into `(numerator, denominator)` numerals.
    VulgarFrac(String, String),
    Plus,
    /// `-` or `−` (U+2212) — binary/unary minus.
    Minus,
    /// `×` / `⋅` / `*` — multiplication.
    Times,
    /// `÷` — division.
    Div,
    /// `/` — built-up fraction.
    Slash,
    /// `±` — plus-or-minus (the pair {a+b, a−b}).
    PlusMinus,
    /// `∓` — minus-or-plus (the opposite pairing).
    MinusPlus,
    /// `√` — square root (degree-less).
    Sqrt,
    /// `∛`/`∜` — nth root with the given fixed degree (3 or 4).
    RootN(u32),
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    /// `≈` — approximately equal.
    Approx,
    /// `≡` — identically equal.
    Equiv,
    /// `^` — explicit superscript / power operator (the ASCII twin of the Unicode superscript
    /// glyphs, so `x^2` ≡ `x²`; needed for big-operator upper bounds like `∑_(i=1)^n`).
    Caret,
    /// `_` — explicit subscript operator (`a_i`; big-operator lower bounds).
    Underscore,
    /// A big operator glyph — `∑ ∏ ∫ ∮ ∐` — carrying its canonical [`BigOp`] name
    /// (`"sum"`, `"prod"`, `"int"`, `"oint"`, `"coprod"`).
    Big(String),
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    /// End of input (always the final token; zero-width span at the end).
    Eof,
}

/// A token plus where it came from (a byte span into the original source).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

fn err(msg: impl Into<String>, span: Span) -> FrontendError {
    FrontendError::new("unicode-math", msg, span)
}

/// Map a Greek letter or math constant glyph to its canonical neutral [`MathExpr::Symbol`] name,
/// or `None` if `c` is not such a glyph. Names match the AsciiMath frontend's table so the two
/// notations agree on one `Symbol` string (`π` and `pi` both → `"pi"`), the whole point of a
/// shared neutral AST.
fn constant_glyph(c: char) -> Option<&'static str> {
    Some(match c {
        // lowercase Greek
        'α' => "alpha", 'β' => "beta", 'γ' => "gamma", 'δ' => "delta", 'ε' => "epsilon",
        'ζ' => "zeta", 'η' => "eta", 'θ' => "theta", 'ι' => "iota", 'κ' => "kappa",
        'λ' => "lambda", 'μ' => "mu", 'ν' => "nu", 'ξ' => "xi", 'ο' => "omicron",
        'π' => "pi", 'ρ' => "rho", 'σ' => "sigma", 'τ' => "tau", 'υ' => "upsilon",
        'φ' => "phi", 'χ' => "chi", 'ψ' => "psi", 'ω' => "omega",
        // uppercase Greek (the visually distinct ones)
        'Γ' => "Gamma", 'Δ' => "Delta", 'Θ' => "Theta", 'Λ' => "Lambda", 'Ξ' => "Xi",
        'Π' => "Pi", 'Σ' => "Sigma", 'Υ' => "Upsilon", 'Φ' => "Phi", 'Ψ' => "Psi", 'Ω' => "Omega",
        // constants / sets
        '∞' => "infinity", 'ℝ' => "reals", 'ℕ' => "naturals", 'ℤ' => "integers",
        'ℚ' => "rationals", 'ℂ' => "complexes", '∅' => "emptyset", '∂' => "partial", '∇' => "nabla",
        _ => return None,
    })
}

/// Map a superscript codepoint to the plain character it stands for, or `None`.
fn superscript_char(c: char) -> Option<char> {
    Some(match c {
        '⁰' => '0', '¹' => '1', '²' => '2', '³' => '3', '⁴' => '4',
        '⁵' => '5', '⁶' => '6', '⁷' => '7', '⁸' => '8', '⁹' => '9',
        '⁺' => '+', '⁻' => '-',
        _ => return None,
    })
}

/// Map a subscript codepoint to the plain character it stands for, or `None`.
fn subscript_char(c: char) -> Option<char> {
    Some(match c {
        '₀' => '0', '₁' => '1', '₂' => '2', '₃' => '3', '₄' => '4',
        '₅' => '5', '₆' => '6', '₇' => '7', '₈' => '8', '₉' => '9',
        '₊' => '+', '₋' => '-',
        _ => return None,
    })
}

/// Map a vulgar-fraction glyph to `(numerator, denominator)`, or `None`.
fn vulgar_fraction(c: char) -> Option<(&'static str, &'static str)> {
    Some(match c {
        '½' => ("1", "2"), '⅓' => ("1", "3"), '⅔' => ("2", "3"),
        '¼' => ("1", "4"), '¾' => ("3", "4"),
        '⅕' => ("1", "5"), '⅖' => ("2", "5"), '⅗' => ("3", "5"), '⅘' => ("4", "5"),
        '⅙' => ("1", "6"), '⅚' => ("5", "6"),
        '⅛' => ("1", "8"), '⅜' => ("3", "8"), '⅝' => ("5", "8"), '⅞' => ("7", "8"),
        '⅐' => ("1", "7"), '⅑' => ("1", "9"), '⅒' => ("1", "10"),
        _ => return None,
    })
}

/// Tokenize a unicode-math source string. Total and panic-free: returns the token list
/// (terminated by [`TokenKind::Eof`]) or a single spanned [`FrontendError`].
pub fn tokenize(src: &str) -> Result<Vec<Token>, FrontendError> {
    // Materialise (byte_offset, char) pairs so we can look ahead within runs while still
    // recording true byte spans. All slicing uses these byte offsets, so it is panic-free
    // even though the input is full Unicode.
    let chars: Vec<(usize, char)> = src.char_indices().collect();
    let len = src.len();
    let end_of = |i: usize| chars.get(i).map(|&(b, _)| b).unwrap_or(len);

    let mut toks = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let (start, c) = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        // ── numbers ────────────────────────────────────────────────────────────────────────
        if c.is_ascii_digit() || (c == '.' && matches!(chars.get(i + 1), Some(&(_, d)) if d.is_ascii_digit())) {
            let j = scan_number(&chars, i);
            toks.push(Token { kind: TokenKind::Num(src[start..end_of(j)].to_string()), span: (start, end_of(j)) });
            i = j;
            continue;
        }
        // ── superscript run → a power exponent ───────────────────────────────────────────────
        if superscript_char(c).is_some() {
            let mut s = String::new();
            let mut j = i;
            while let Some(&(_, ch)) = chars.get(j) {
                match superscript_char(ch) {
                    Some(p) => { s.push(p); j += 1; }
                    None => break,
                }
            }
            toks.push(Token { kind: TokenKind::Super(s), span: (start, end_of(j)) });
            i = j;
            continue;
        }
        // ── subscript run → a subscript index ────────────────────────────────────────────────
        if subscript_char(c).is_some() {
            let mut s = String::new();
            let mut j = i;
            while let Some(&(_, ch)) = chars.get(j) {
                match subscript_char(ch) {
                    Some(p) => { s.push(p); j += 1; }
                    None => break,
                }
            }
            toks.push(Token { kind: TokenKind::Sub(s), span: (start, end_of(j)) });
            i = j;
            continue;
        }
        // ── single-codepoint tokens ──────────────────────────────────────────────────────────
        let one = (start, end_of(i + 1));
        if let Some((n, d)) = vulgar_fraction(c) {
            toks.push(Token { kind: TokenKind::VulgarFrac(n.into(), d.into()), span: one });
            i += 1;
            continue;
        }
        if let Some(name) = constant_glyph(c) {
            toks.push(Token { kind: TokenKind::Sym(name.into()), span: one });
            i += 1;
            continue;
        }
        let kind = match c {
            'a'..='z' | 'A'..='Z' => TokenKind::Sym(c.to_string()),
            '+' => TokenKind::Plus,
            '-' | '−' => TokenKind::Minus,
            '×' | '⋅' | '*' => TokenKind::Times,
            '÷' => TokenKind::Div,
            '/' => TokenKind::Slash,
            '±' => TokenKind::PlusMinus,
            '∓' => TokenKind::MinusPlus,
            '√' => TokenKind::Sqrt,
            '∛' => TokenKind::RootN(3),
            '∜' => TokenKind::RootN(4),
            '^' => TokenKind::Caret,
            '_' => TokenKind::Underscore,
            '∑' => TokenKind::Big("sum".into()),
            '∏' => TokenKind::Big("prod".into()),
            '∫' => TokenKind::Big("int".into()),
            '∮' => TokenKind::Big("oint".into()),
            '∐' => TokenKind::Big("coprod".into()),
            '=' => TokenKind::Eq,
            '≠' => TokenKind::Ne,
            '<' => TokenKind::Lt,
            '≤' => TokenKind::Le,
            '>' => TokenKind::Gt,
            '≥' => TokenKind::Ge,
            '≈' => TokenKind::Approx,
            '≡' => TokenKind::Equiv,
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '[' => TokenKind::LBracket,
            ']' => TokenKind::RBracket,
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            _ => return Err(err(format!("unexpected character {c:?}"), one)),
        };
        toks.push(Token { kind, span: one });
        i += 1;
    }
    toks.push(Token { kind: TokenKind::Eof, span: (len, len) });
    Ok(toks)
}

/// Scan a numeric literal beginning at char-index `start`, returning the end char-index
/// (exclusive). Grammar mirrors AsciiMath/[`Number::parse`]: `digits? ('.' digits)? ([eE][+-]?
/// digits)?`, with the exponent consumed only when well-formed (so `2e` is `2` then `e`).
fn scan_number(chars: &[(usize, char)], start: usize) -> usize {
    let at = |k: usize| chars.get(k).map(|&(_, c)| c);
    let mut i = start;
    while matches!(at(i), Some(d) if d.is_ascii_digit()) {
        i += 1;
    }
    if at(i) == Some('.') && matches!(at(i + 1), Some(d) if d.is_ascii_digit()) {
        i += 1;
        while matches!(at(i), Some(d) if d.is_ascii_digit()) {
            i += 1;
        }
    }
    if matches!(at(i), Some('e') | Some('E')) {
        let mut j = i + 1;
        if matches!(at(j), Some('+') | Some('-')) {
            j += 1;
        }
        if matches!(at(j), Some(d) if d.is_ascii_digit()) {
            j += 1;
            while matches!(at(j), Some(d) if d.is_ascii_digit()) {
                j += 1;
            }
            i = j;
        }
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<TokenKind> {
        tokenize(src).unwrap().into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn numbers_symbols_operators() {
        assert_eq!(kinds("3.14 + x"), vec![
            TokenKind::Num("3.14".into()), TokenKind::Plus, TokenKind::Sym("x".into()), TokenKind::Eof,
        ]);
    }

    #[test]
    fn greek_and_constants_map_to_canonical_names() {
        assert_eq!(kinds("π")[0], TokenKind::Sym("pi".into()));
        assert_eq!(kinds("Σ")[0], TokenKind::Sym("Sigma".into()));
        assert_eq!(kinds("∞")[0], TokenKind::Sym("infinity".into()));
    }

    #[test]
    fn superscript_and_subscript_runs_normalise() {
        assert_eq!(kinds("x²")[1], TokenKind::Super("2".into()));
        assert_eq!(kinds("x⁻¹")[1], TokenKind::Super("-1".into()));
        assert_eq!(kinds("a₁")[1], TokenKind::Sub("1".into()));
    }

    #[test]
    fn vulgar_fractions_split() {
        assert_eq!(kinds("½")[0], TokenKind::VulgarFrac("1".into(), "2".into()));
        assert_eq!(kinds("⅔")[0], TokenKind::VulgarFrac("2".into(), "3".into()));
    }

    #[test]
    fn unicode_operators_and_relations() {
        assert_eq!(kinds("a × b")[1], TokenKind::Times);
        assert_eq!(kinds("a ⋅ b")[1], TokenKind::Times);
        assert_eq!(kinds("a ÷ b")[1], TokenKind::Div);
        assert_eq!(kinds("a − b")[1], TokenKind::Minus); // U+2212
        assert_eq!(kinds("a ± b")[1], TokenKind::PlusMinus);
        assert_eq!(kinds("a ∓ b")[1], TokenKind::MinusPlus);
        assert_eq!(kinds("a ≤ b")[1], TokenKind::Le);
        assert_eq!(kinds("a ≥ b")[1], TokenKind::Ge);
        assert_eq!(kinds("a ≠ b")[1], TokenKind::Ne);
        assert_eq!(kinds("a ≈ b")[1], TokenKind::Approx);
        assert_eq!(kinds("√x")[0], TokenKind::Sqrt);
        assert_eq!(kinds("∛x")[0], TokenKind::RootN(3));
    }

    #[test]
    fn exponent_only_consumed_when_well_formed() {
        assert_eq!(kinds("6.022e23")[0], TokenKind::Num("6.022e23".into()));
        assert_eq!(kinds("2e"), vec![
            TokenKind::Num("2".into()), TokenKind::Sym("e".into()), TokenKind::Eof,
        ]);
    }

    #[test]
    fn spans_are_byte_offsets_into_unicode_source() {
        // `π` is two bytes; the `²` after `x` starts at byte 1.
        let t = tokenize("x²").unwrap();
        assert_eq!(t[0].span, (0, 1)); // x
        assert_eq!(t[1].span, (1, 3)); // ² (3 bytes)
        for tok in tokenize("π² ≤ ∞").unwrap() {
            assert!(tok.span.0 <= tok.span.1 && tok.span.1 <= "π² ≤ ∞".len());
        }
    }

    #[test]
    fn big_operators_and_explicit_scripts() {
        // PR-2: big-operator glyphs carry their canonical BigOp name.
        assert_eq!(kinds("∑")[0], TokenKind::Big("sum".into()));
        assert_eq!(kinds("∏")[0], TokenKind::Big("prod".into()));
        assert_eq!(kinds("∫")[0], TokenKind::Big("int".into()));
        // ASCII `^`/`_` are explicit script operators (twins of the Unicode glyphs).
        assert_eq!(kinds("x^2"), vec![
            TokenKind::Sym("x".into()), TokenKind::Caret, TokenKind::Num("2".into()), TokenKind::Eof,
        ]);
        assert_eq!(kinds("a_i")[1], TokenKind::Underscore);
    }

    #[test]
    fn errors_are_spanned_not_panics() {
        // An out-of-scope glyph (⊗ is not handled) is a clean spanned error, never a panic.
        let e = tokenize("⊗").unwrap_err();
        assert_eq!(e.frontend, "unicode-math");
        assert!(e.span.0 <= e.span.1 && e.span.1 <= "⊗".len());
    }
}
