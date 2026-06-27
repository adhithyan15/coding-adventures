//! Category codes — TeX's classification of every input character.
//!
//! In TeX, a character's *meaning* is governed by its **category code** (catcode), not
//! its glyph: `\` begins a control sequence, `{` opens a group, `$` shifts into math,
//! `%` starts a comment, and so on. The tokenizer ([`crate::lexer`]) is driven by these
//! categories — it is, quite literally, a catcode state machine, exactly as TeX's "mouth"
//! is. We use TeX's *default* (plain-LaTeX) assignments; runtime `\catcode` reassignment
//! is out of scope (the documented asymptote — see the crate docs).
//!
//! The classic 16 categories, with the ones LaTeX source actually exercises:
//!
//! | code | category      | characters (default)        |
//! |------|---------------|-----------------------------|
//! | 0    | Escape        | `\`                         |
//! | 1    | BeginGroup    | `{`                         |
//! | 2    | EndGroup      | `}`                         |
//! | 3    | MathShift     | `$`                         |
//! | 4    | AlignTab      | `&`                         |
//! | 5    | EndLine       | carriage return / newline   |
//! | 6    | Parameter     | `#`                         |
//! | 7    | Superscript   | `^`                         |
//! | 8    | Subscript     | `_`                         |
//! | 9    | Ignored       | NUL                         |
//! | 10   | Space         | space, tab                  |
//! | 11   | Letter        | `A..Z`, `a..z`              |
//! | 12   | Other         | digits, punctuation, …      |
//! | 13   | Active        | `~`                         |
//! | 14   | Comment       | `%`                         |
//! | 15   | Invalid       | DEL                         |

/// A TeX category code. (Codes 9/15 — Ignored/Invalid — are folded into the handling of
/// their rare characters; every other category is represented.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Catcode {
    Escape,
    BeginGroup,
    EndGroup,
    MathShift,
    AlignTab,
    EndLine,
    Parameter,
    Superscript,
    Subscript,
    Space,
    Letter,
    Other,
    Active,
    Comment,
}

/// The default (plain-LaTeX) category code of a character.
pub fn catcode(c: char) -> Catcode {
    match c {
        '\\' => Catcode::Escape,
        '{' => Catcode::BeginGroup,
        '}' => Catcode::EndGroup,
        '$' => Catcode::MathShift,
        '&' => Catcode::AlignTab,
        '\n' | '\r' => Catcode::EndLine,
        '#' => Catcode::Parameter,
        '^' => Catcode::Superscript,
        '_' => Catcode::Subscript,
        ' ' | '\t' => Catcode::Space,
        'a'..='z' | 'A'..='Z' => Catcode::Letter,
        '~' => Catcode::Active,
        '%' => Catcode::Comment,
        _ => Catcode::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_assignments() {
        assert_eq!(catcode('\\'), Catcode::Escape);
        assert_eq!(catcode('{'), Catcode::BeginGroup);
        assert_eq!(catcode('}'), Catcode::EndGroup);
        assert_eq!(catcode('$'), Catcode::MathShift);
        assert_eq!(catcode('&'), Catcode::AlignTab);
        assert_eq!(catcode('\n'), Catcode::EndLine);
        assert_eq!(catcode('#'), Catcode::Parameter);
        assert_eq!(catcode('^'), Catcode::Superscript);
        assert_eq!(catcode('_'), Catcode::Subscript);
        assert_eq!(catcode(' '), Catcode::Space);
        assert_eq!(catcode('\t'), Catcode::Space);
        assert_eq!(catcode('x'), Catcode::Letter);
        assert_eq!(catcode('7'), Catcode::Other);
        assert_eq!(catcode('+'), Catcode::Other);
        assert_eq!(catcode('~'), Catcode::Active);
        assert_eq!(catcode('%'), Catcode::Comment);
    }
}
