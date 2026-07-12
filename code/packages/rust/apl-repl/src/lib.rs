//! # APL REPL — an interactive Read-Eval-Print loop for APL.
//!
//! [`AplRepl`] wraps a persistent [`Interpreter`] and adds the interactive
//! behaviours a console needs: line continuation across an open `(`, and
//! echoing of auto-printed results. It is the sibling of `matlab-repl`/
//! `s-repl`/`r-repl`; only the interpreter (and the continuation scanner)
//! differ. See `code/specs/MA05-apl-language.md`.
//!
//! ## Why this scanner is so much simpler than MATLAB's
//!
//! `matlab-repl::is_incomplete` tracks bracket balance *and* unterminated
//! block keywords (`if`/`for`/`while`/`switch`/`try`/`function`) *and*
//! `"`-string state, because MATLAB's grammar has all three. APL (this
//! language cut, MA05 §4) has **none** of that: no control flow, no
//! user-defined functions/blocks, and no string/char literal type at all
//! (`apl.tokens` has no `STRING` token). Re-reading `apl.grammar`: the only
//! grouping construct is `LPAREN value_expr RPAREN`, so an unbalanced `(` is
//! the *only* way a statement can still be "in progress" — this scanner
//! reduces to plain paren-balance tracking.
//!
//! ## Why continuation lines are joined with a space, not a real newline
//!
//! `apl.tokens`' own SECTION 5 doc comment notes this first cut does **not**
//! drop newlines inside `(...)` — every parenthesised expression must stay
//! on one physical line, unlike MATLAB's `[...]` (where a newline is a
//! meaningful row separator the grammar *does* handle specially). So while a
//! `(` is still open, joining physical lines with a literal `'\n'` would
//! hand the parser a genuinely broken program (a `NEWLINE` token where
//! `value_expr` expects a continuation or `)`). Joining with a single space
//! instead keeps the accumulated source syntactically one logical line —
//! exactly as if the user had typed it all without pressing Enter.
//!
//! Hand-rolled rather than built on the generic `repl` crate, mirroring
//! `matlab-repl`'s own rationale: the interpreter is single-threaded, and a
//! console session is sequential anyway.

use coding_adventures_apl_runtime::Interpreter;
use std::io::{BufRead, Write};

/// What the REPL should do after being fed one physical line.
#[derive(Debug, PartialEq, Eq)]
pub enum ReplResponse {
    /// Text to display (may be empty — e.g. after a silent assignment).
    Output(String),
    /// The current statement is incomplete; read another line (`... ` prompt).
    NeedMore,
    /// End the session.
    Quit,
}

/// A persistent interactive APL session.
pub struct AplRepl {
    interp: Interpreter,
    buffer: String,
}

impl Default for AplRepl {
    fn default() -> Self {
        Self::new()
    }
}

impl AplRepl {
    pub fn new() -> Self {
        AplRepl {
            interp: Interpreter::new(),
            buffer: String::new(),
        }
    }

    /// `>> ` for a fresh statement, `... ` while continuing an incomplete one
    /// (an open `(`).
    pub fn prompt(&self) -> &'static str {
        if self.buffer.is_empty() {
            ">> "
        } else {
            "... "
        }
    }

    /// Feed one physical input line (without its trailing newline).
    pub fn feed(&mut self, line: &str) -> ReplResponse {
        if self.buffer.is_empty() {
            match line.trim() {
                "quit" | "exit" | "quit()" | "exit()" => return ReplResponse::Quit,
                _ => {}
            }
            self.buffer.push_str(line);
        } else {
            // See this module's doc comment: joining with a space (not a
            // real '\n') keeps a still-open `(...)` on one logical line.
            self.buffer.push(' ');
            self.buffer.push_str(line);
        }

        if paren_depth(&self.buffer) > 0 {
            return ReplResponse::NeedMore;
        }

        let src = std::mem::take(&mut self.buffer);
        if src.trim().is_empty() {
            return ReplResponse::Output(String::new());
        }
        // `apl.grammar`'s `line = statement NEWLINE | statement | NEWLINE`
        // accepts a bare `statement` with no trailing NEWLINE, but adding
        // one keeps every call site consistent (and matches every fixture
        // in `apl-parser`'s own test suite).
        match self.interp.feed(&format!("{src}\n")) {
            Ok(text) => ReplResponse::Output(text),
            Err(e) => ReplResponse::Output(format!("Error: {e}\n")),
        }
    }

    pub fn is_continuing(&self) -> bool {
        !self.buffer.is_empty()
    }
}

/// Count of unbalanced `(` in `src` (never negative — a stray `)` with no
/// matching `(` does not make the statement "more open"; it is left for the
/// parser to report as its own clean syntax error).
fn paren_depth(src: &str) -> i32 {
    let mut depth: i32 = 0;
    for ch in src.chars() {
        match ch {
            '(' => depth += 1,
            ')' => depth = (depth - 1).max(0),
            _ => {}
        }
    }
    depth
}

/// Drive a full interactive APL session over the given reader and writer.
pub fn run<R: BufRead, W: Write>(mut reader: R, mut writer: W) -> std::io::Result<()> {
    let mut repl = AplRepl::new();
    writeln!(writer, "APL (on array-runtime) — type quit to exit.")?;

    loop {
        write!(writer, "{}", repl.prompt())?;
        writer.flush()?;

        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            writeln!(writer)?;
            break;
        }
        let line = line.strip_suffix('\n').unwrap_or(&line);
        let line = line.strip_suffix('\r').unwrap_or(line);

        match repl.feed(line) {
            ReplResponse::Output(text) => {
                write!(writer, "{text}")?;
                writer.flush()?;
            }
            ReplResponse::NeedMore => {}
            ReplResponse::Quit => break,
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_expression_prints_assignment_is_silent() {
        let mut r = AplRepl::new();
        assert!(matches!(r.feed("2+2"), ReplResponse::Output(t) if t.contains('4')));
        assert_eq!(r.feed("A←5"), ReplResponse::Output(String::new()));
    }

    #[test]
    fn continues_across_an_open_paren() {
        let mut r = AplRepl::new();
        assert_eq!(r.feed("(1+2"), ReplResponse::NeedMore);
        assert!(r.is_continuing());
        assert!(matches!(r.feed("+3)"), ReplResponse::Output(t) if t.contains('6')));
        assert!(!r.is_continuing());
    }

    #[test]
    fn quit_commands() {
        assert_eq!(AplRepl::new().feed("quit"), ReplResponse::Quit);
        assert_eq!(AplRepl::new().feed("exit"), ReplResponse::Quit);
    }

    #[test]
    fn errors_are_shown_not_fatal() {
        let mut r = AplRepl::new();
        assert!(matches!(r.feed("undefined_var"), ReplResponse::Output(t) if t.contains("Error")));
        // The session keeps going.
        assert!(matches!(r.feed("1+1"), ReplResponse::Output(t) if t.contains('2')));
    }

    #[test]
    fn session_persists_across_lines() {
        let mut r = AplRepl::new();
        r.feed("A←10");
        assert!(matches!(r.feed("A+5"), ReplResponse::Output(t) if t.contains("15")));
    }

    #[test]
    fn run_drives_a_session_to_eof() {
        let input = "A←3\nA×2\nquit\n".as_bytes();
        let mut output = Vec::new();
        run(input, &mut output).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains('6'));
    }
}
