//! The tokenizer — a catcode-driven **state machine** over the source bytes.
//!
//! # Why a state machine (TeX is one)
//!
//! A character's meaning in LaTeX depends on what the scanner is doing and on its
//! [category code](crate::catcode): `\` begins a control sequence (and what follows —
//! letters vs one symbol — changes how much is consumed), `%` skips to end of line, `$`
//! toggles math mode, a blank line means a paragraph break. None of that is a regular
//! language; it is a stateful scan, mirroring TeX's "mouth".
//!
//! # The two pieces of state
//!
//! 1. **A mode stack** (`Text` is primary; `Math{display}` is pushed by `$`/`\(`/`\[`/`$$`
//!    and popped by the matching close). LaTeX *starts in text mode* — the inverse of a
//!    math-only tokenizer — and whitespace is significant in text but not in math, so the
//!    current mode changes how spaces are handled.
//! 2. **A position cursor** over a pre-collected `(byte_offset, char)` vector, so every
//!    emitted token carries an exact byte span while we still iterate by Unicode scalar.
//!
//! ```text
//!   TEXT ──$ / \( / \[ / $$──▶ MATH{display}
//!     ▲                          │
//!     └────── $ / \) / \] ───────┘
//! ```
//!
//! Ordinary characters are emitted one-per-`Char` (as TeX does); coalescing runs into
//! words is the parser's job, not the tokenizer's.

use crate::catcode::{catcode, Catcode};
use crate::error::LexError;
use crate::token::{Token, TokenKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Text,
    Math { display: bool },
}

/// Tokenize a LaTeX string into a flat token stream ending in [`TokenKind::Eof`]. Returns
/// a spanned [`LexError`] on a malformed control sequence (e.g. a trailing `\`); never
/// panics.
pub fn tokenize(src: &str) -> Result<Vec<Token>, LexError> {
    let chars: Vec<(usize, char)> = src.char_indices().collect();
    let end = src.len();
    let n = chars.len();
    let off = |i: usize| chars.get(i).map(|&(o, _)| o).unwrap_or(end);

    let mut out: Vec<Token> = Vec::new();
    let mut modes: Vec<Mode> = vec![Mode::Text];
    let mut i = 0;

    while i < n {
        let (start, c) = chars[i];
        match catcode(c) {
            // ---- escape: a control sequence ------------------------------------
            Catcode::Escape => {
                i += 1;
                if i >= n {
                    return Err(LexError::new(
                        "trailing '\\' at end of input (expected a control-sequence name)",
                        start,
                        end,
                    ));
                }
                let d = chars[i].1;
                if catcode(d) == Catcode::Letter {
                    // control WORD: a run of letters
                    let name_start = i;
                    while i < n && catcode(chars[i].1) == Catcode::Letter {
                        i += 1;
                    }
                    let name: String = chars[name_start..i].iter().map(|&(_, ch)| ch).collect();
                    out.push(Token::new(TokenKind::ControlWord(name), start, off(i)));
                    // TeX absorbs the spaces (and tabs) following a control word.
                    while i < n && matches!(chars[i].1, ' ' | '\t') {
                        i += 1;
                    }
                } else {
                    // control SYMBOL — or a math-delimiter control symbol.
                    match d {
                        '(' => {
                            modes.push(Mode::Math { display: false });
                            i += 1;
                            out.push(Token::new(TokenKind::MathOn { display: false }, start, off(i)));
                        }
                        '[' => {
                            modes.push(Mode::Math { display: true });
                            i += 1;
                            out.push(Token::new(TokenKind::MathOn { display: true }, start, off(i)));
                        }
                        ')' => {
                            pop_math(&mut modes);
                            i += 1;
                            out.push(Token::new(TokenKind::MathOff { display: false }, start, off(i)));
                        }
                        ']' => {
                            pop_math(&mut modes);
                            i += 1;
                            out.push(Token::new(TokenKind::MathOff { display: true }, start, off(i)));
                        }
                        _ => {
                            i += 1;
                            out.push(Token::new(TokenKind::ControlSymbol(d), start, off(i)));
                        }
                    }
                }
            }

            // ---- group braces --------------------------------------------------
            Catcode::BeginGroup => {
                i += 1;
                out.push(Token::new(TokenKind::BeginGroup, start, off(i)));
            }
            Catcode::EndGroup => {
                i += 1;
                out.push(Token::new(TokenKind::EndGroup, start, off(i)));
            }

            // ---- math shift: `$` or `$$` --------------------------------------
            Catcode::MathShift => {
                let display = i + 1 < n && chars[i + 1].1 == '$';
                i += if display { 2 } else { 1 };
                let in_math = matches!(modes.last(), Some(Mode::Math { .. }));
                if in_math {
                    pop_math(&mut modes);
                    out.push(Token::new(TokenKind::MathOff { display }, start, off(i)));
                } else {
                    modes.push(Mode::Math { display });
                    out.push(Token::new(TokenKind::MathOn { display }, start, off(i)));
                }
            }

            // ---- single-character categories -----------------------------------
            Catcode::AlignTab => {
                i += 1;
                out.push(Token::new(TokenKind::AlignTab, start, off(i)));
            }
            Catcode::Parameter => {
                i += 1;
                out.push(Token::new(TokenKind::Parameter, start, off(i)));
            }
            Catcode::Superscript => {
                i += 1;
                out.push(Token::new(TokenKind::Superscript, start, off(i)));
            }
            Catcode::Subscript => {
                i += 1;
                out.push(Token::new(TokenKind::Subscript, start, off(i)));
            }
            Catcode::Active => {
                i += 1;
                out.push(Token::new(TokenKind::Active(c), start, off(i)));
            }

            // ---- comment: `%` to end of line -----------------------------------
            Catcode::Comment => {
                i += 1; // past '%'
                let text_start = i;
                while i < n && chars[i].1 != '\n' && chars[i].1 != '\r' {
                    i += 1;
                }
                let text: String = chars[text_start..i].iter().map(|&(_, ch)| ch).collect();
                let comment_end = off(i);
                // TeX eats the line ending after a comment (so no spurious space).
                if i < n && chars[i].1 == '\r' {
                    i += 1;
                }
                if i < n && chars[i].1 == '\n' {
                    i += 1;
                }
                out.push(Token::new(TokenKind::Comment(text), start, comment_end));
            }

            // ---- whitespace: significant in text, ignored in math --------------
            Catcode::Space | Catcode::EndLine => {
                let run_start = i;
                let mut newlines = 0usize;
                while i < n && matches!(catcode(chars[i].1), Catcode::Space | Catcode::EndLine) {
                    if chars[i].1 == '\n' {
                        newlines += 1;
                    }
                    i += 1;
                }
                if matches!(modes.last(), Some(Mode::Math { .. })) {
                    // insignificant in math mode — emit nothing
                } else if newlines >= 2 {
                    out.push(Token::new(TokenKind::Par, chars[run_start].0, off(i)));
                } else {
                    out.push(Token::new(TokenKind::Space, chars[run_start].0, off(i)));
                }
            }

            // ---- ordinary characters (letters and others) ----------------------
            Catcode::Letter | Catcode::Other => {
                i += 1;
                out.push(Token::new(TokenKind::Char(c), start, off(i)));
            }
        }
    }

    out.push(Token::new(TokenKind::Eof, end, end));
    Ok(out)
}

/// Pop one `Math` mode if the top is math; never pop the bottom `Text` (a stray `\)`/`$`
/// in text mode leaves the stack intact rather than underflowing).
fn pop_math(modes: &mut Vec<Mode>) {
    if matches!(modes.last(), Some(Mode::Math { .. })) {
        modes.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::TokenKind::*;

    fn kinds(src: &str) -> Vec<TokenKind> {
        let mut t: Vec<TokenKind> =
            tokenize(src).expect("tokenize").into_iter().map(|t| t.kind).collect();
        assert_eq!(t.pop(), Some(Eof), "stream must end in Eof");
        t
    }

    #[test]
    fn plain_text_is_chars_and_spaces() {
        assert_eq!(
            kinds("ab c"),
            vec![Char('a'), Char('b'), Space, Char('c')]
        );
    }

    #[test]
    fn runs_of_space_collapse_to_one_space() {
        assert_eq!(kinds("a   \t b"), vec![Char('a'), Space, Char('b')]);
    }

    #[test]
    fn single_newline_is_a_space_two_is_a_paragraph() {
        assert_eq!(kinds("a\nb"), vec![Char('a'), Space, Char('b')]);
        assert_eq!(kinds("a\n\nb"), vec![Char('a'), Par, Char('b')]);
        assert_eq!(kinds("a\n  \n  b"), vec![Char('a'), Par, Char('b')]); // blank line w/ spaces
    }

    #[test]
    fn control_word_and_absorbed_spaces() {
        // `\alpha  x` → ControlWord("alpha") then Char('x') (the spaces are absorbed).
        assert_eq!(kinds(r"\alpha  x"), vec![ControlWord("alpha".into()), Char('x')]);
    }

    #[test]
    fn control_word_stops_at_nonletter() {
        assert_eq!(kinds(r"\section*"), vec![ControlWord("section".into()), Char('*')]);
    }

    #[test]
    fn control_symbols_and_line_break() {
        assert_eq!(kinds(r"\,"), vec![ControlSymbol(',')]);
        assert_eq!(kinds(r"\{"), vec![ControlSymbol('{')]);
        assert_eq!(kinds(r"\%"), vec![ControlSymbol('%')]);
        assert_eq!(kinds(r"a \\ b"), vec![Char('a'), Space, ControlSymbol('\\'), Space, Char('b')]);
    }

    #[test]
    fn groups_and_scripts_and_params() {
        assert_eq!(
            kinds("{a}^_#&"),
            vec![BeginGroup, Char('a'), EndGroup, Superscript, Subscript, Parameter, AlignTab]
        );
    }

    #[test]
    fn dollar_toggles_inline_math() {
        assert_eq!(
            kinds("a$x$b"),
            vec![Char('a'), MathOn { display: false }, Char('x'), MathOff { display: false }, Char('b')]
        );
    }

    #[test]
    fn double_dollar_is_display_math() {
        assert_eq!(
            kinds("$$x$$"),
            vec![MathOn { display: true }, Char('x'), MathOff { display: true }]
        );
    }

    #[test]
    fn latex_delimiters_open_and_close_math() {
        assert_eq!(
            kinds(r"\(x\)"),
            vec![MathOn { display: false }, Char('x'), MathOff { display: false }]
        );
        assert_eq!(
            kinds(r"\[x\]"),
            vec![MathOn { display: true }, Char('x'), MathOff { display: true }]
        );
    }

    #[test]
    fn whitespace_is_insignificant_inside_math() {
        // spaces between `$ … $` emit no Space tokens.
        assert_eq!(
            kinds("$a   b$"),
            vec![MathOn { display: false }, Char('a'), Char('b'), MathOff { display: false }]
        );
    }

    #[test]
    fn comment_runs_to_end_of_line_and_eats_the_newline() {
        // `a% foo\nb` → Char(a), Comment(" foo"), Char(b) — no Space from the newline.
        assert_eq!(
            kinds("a% foo\nb"),
            vec![Char('a'), Comment(" foo".into()), Char('b')]
        );
    }

    #[test]
    fn tilde_is_an_active_character() {
        assert_eq!(kinds("a~b"), vec![Char('a'), Active('~'), Char('b')]);
    }

    #[test]
    fn a_realistic_snippet() {
        // "Let $x$ be." — text, inline math, text.
        assert_eq!(
            kinds(r"Let $x$ be."),
            vec![
                Char('L'), Char('e'), Char('t'), Space,
                MathOn { display: false }, Char('x'), MathOff { display: false },
                Space, Char('b'), Char('e'), Char('.'),
            ]
        );
    }

    #[test]
    fn spans_are_correct_including_multibyte() {
        let toks = tokenize("é$x$").unwrap();
        // 'é' is 2 bytes (0..2); '$' at 2..3; 'x' at 3..4; '$' at 4..5; Eof at 5..5.
        assert_eq!(toks[0].kind, TokenKind::Char('é'));
        assert_eq!((toks[0].span.start, toks[0].span.end), (0, 2));
        assert_eq!(toks[1].kind, TokenKind::MathOn { display: false });
        assert_eq!((toks[1].span.start, toks[1].span.end), (2, 3));
        assert_eq!(toks.last().unwrap().kind, TokenKind::Eof);
    }

    #[test]
    fn trailing_backslash_errors_with_span() {
        let e = tokenize(r"a \").unwrap_err();
        assert!(e.message.contains("trailing '\\'"));
        assert_eq!(e.span, (2, 3));
    }

    #[test]
    fn stray_close_math_in_text_does_not_underflow() {
        // `\)` with no open math: emits MathOff, stack stays sane, no panic.
        assert_eq!(kinds(r"a\)b"), vec![Char('a'), MathOff { display: false }, Char('b')]);
    }

    #[test]
    fn empty_input_is_just_eof() {
        assert_eq!(tokenize("").unwrap().len(), 1);
    }
}
