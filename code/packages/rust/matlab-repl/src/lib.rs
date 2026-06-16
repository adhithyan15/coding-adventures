//! # MATLAB REPL — an interactive Read-Eval-Print loop for MATLAB.
//!
//! [`MatlabRepl`] wraps a persistent [`Interpreter`] and adds the interactive
//! behaviours a console needs: line continuation across open brackets and
//! unterminated `if`/`for`/`while`/`switch`/`try`/`function` blocks, and echoing
//! of unsuppressed results. It is the sibling of `s-repl`/`r-repl`; only the
//! interpreter differs. See `code/specs/MA01-matlab-language.md`.
//!
//! Hand-rolled rather than built on the generic `repl` crate because the
//! interpreter is single-threaded; a console session is sequential anyway.

use coding_adventures_matlab_runtime::Interpreter;
use std::io::{BufRead, Write};

/// What the REPL should do after being fed one physical line.
#[derive(Debug, PartialEq, Eq)]
pub enum ReplResponse {
    /// Text to display (may be empty).
    Output(String),
    /// The current statement is incomplete; read another line (`... ` prompt).
    NeedMore,
    /// End the session.
    Quit,
}

/// A persistent interactive MATLAB session.
pub struct MatlabRepl {
    interp: Interpreter,
    buffer: String,
}

impl Default for MatlabRepl {
    fn default() -> Self {
        Self::new()
    }
}

impl MatlabRepl {
    pub fn new() -> Self {
        MatlabRepl {
            interp: Interpreter::new(),
            buffer: String::new(),
        }
    }

    /// `>> ` for a fresh statement, `... ` while continuing an incomplete one.
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
        }

        self.buffer.push_str(line);
        self.buffer.push('\n');

        if is_incomplete(&self.buffer) {
            return ReplResponse::NeedMore;
        }

        let src = std::mem::take(&mut self.buffer);
        if src.trim().is_empty() {
            return ReplResponse::Output(String::new());
        }
        match self.interp.feed(&src) {
            Ok(text) => ReplResponse::Output(text),
            Err(e) => ReplResponse::Output(format!("Error: {e}\n")),
        }
    }

    pub fn is_continuing(&self) -> bool {
        !self.buffer.is_empty()
    }
}

/// Is the accumulated source an incomplete statement? It is incomplete while a
/// bracket is open, a `"`-string is unterminated, or a block keyword
/// (`if`/`for`/`while`/`switch`/`try`/`function`) has no matching `end`. `%` line
/// comments and `"` strings are skipped; `'` is treated as transpose (char
/// arrays are single-line, so they never extend a statement).
fn is_incomplete(src: &str) -> bool {
    let mut bracket: i32 = 0;
    let mut blocks: i32 = 0;
    let mut in_string = false;

    for raw_line in src.lines() {
        let mut word = String::new();
        let flush = |w: &mut String, blocks: &mut i32, bracket: i32| {
            if !w.is_empty() {
                match w.as_str() {
                    "if" | "for" | "while" | "switch" | "try" | "function" | "parfor" => {
                        *blocks += 1
                    }
                    // `end` closes a block only at bracket depth 0; inside
                    // brackets it is the index sentinel.
                    "end" if bracket == 0 => *blocks -= 1,
                    _ => {}
                }
                w.clear();
            }
        };
        for ch in raw_line.chars() {
            if in_string {
                if ch == '"' {
                    in_string = false;
                }
                continue;
            }
            match ch {
                '%' => break, // line comment: ignore the rest of the line
                '"' => {
                    flush(&mut word, &mut blocks, bracket);
                    in_string = true;
                }
                '(' | '[' | '{' => {
                    flush(&mut word, &mut blocks, bracket);
                    bracket += 1;
                }
                ')' | ']' | '}' => {
                    flush(&mut word, &mut blocks, bracket);
                    bracket -= 1;
                }
                c if c.is_alphanumeric() || c == '_' => word.push(c),
                _ => flush(&mut word, &mut blocks, bracket),
            }
        }
        flush(&mut word, &mut blocks, bracket);
    }
    bracket > 0 || blocks > 0 || in_string
}

/// Drive a full interactive MATLAB session over the given reader and writer.
pub fn run<R: BufRead, W: Write>(mut reader: R, mut writer: W) -> std::io::Result<()> {
    let mut repl = MatlabRepl::new();
    writeln!(writer, "MATLAB (on array-runtime) — type quit to exit.")?;

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
    fn echoes_a_result_and_suppresses_with_semicolon() {
        let mut r = MatlabRepl::new();
        assert!(matches!(r.feed("2 + 2"), ReplResponse::Output(t) if t.contains("4")));
        assert_eq!(r.feed("x = 5;"), ReplResponse::Output(String::new()));
    }

    #[test]
    fn continues_across_open_brackets() {
        let mut r = MatlabRepl::new();
        assert_eq!(r.feed("A = [1 2"), ReplResponse::NeedMore);
        assert!(r.is_continuing());
        assert!(matches!(r.feed("3 4];"), ReplResponse::Output(_)));
        assert!(!r.is_continuing());
    }

    #[test]
    fn continues_across_an_unterminated_block() {
        let mut r = MatlabRepl::new();
        assert_eq!(r.feed("s = 0;"), ReplResponse::Output(String::new()));
        assert_eq!(r.feed("for i = 1:3"), ReplResponse::NeedMore);
        assert_eq!(r.feed("  s = s + i;"), ReplResponse::NeedMore);
        assert!(matches!(r.feed("end"), ReplResponse::Output(_)));
        assert!(matches!(r.feed("s"), ReplResponse::Output(t) if t.contains("6")));
    }

    #[test]
    fn end_inside_an_index_does_not_open_a_block() {
        // `A(end)` must not be read as opening a block via the word `end`.
        let mut r = MatlabRepl::new();
        r.feed("A = [10 20 30];");
        assert!(matches!(r.feed("A(end)"), ReplResponse::Output(t) if t.contains("30")));
    }

    #[test]
    fn quit_commands() {
        assert_eq!(MatlabRepl::new().feed("quit"), ReplResponse::Quit);
        assert_eq!(MatlabRepl::new().feed("exit"), ReplResponse::Quit);
    }

    #[test]
    fn errors_are_shown_not_fatal() {
        let mut r = MatlabRepl::new();
        assert!(matches!(r.feed("undefined_var"), ReplResponse::Output(t) if t.contains("Error")));
        // The session keeps going.
        assert!(matches!(r.feed("1 + 1"), ReplResponse::Output(t) if t.contains("2")));
    }

    #[test]
    fn run_drives_a_session_to_eof() {
        let input = b"x = 3;\nx * 2\nquit\n" as &[u8];
        let mut output = Vec::new();
        run(input, &mut output).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("6"));
    }
}
