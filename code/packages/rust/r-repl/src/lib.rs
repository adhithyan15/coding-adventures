//! # R REPL — an interactive Read-Eval-Print loop for the R language.
//!
//! [`RRepl`] wraps a persistent [`RInterpreter`] (which itself reuses the shared
//! S evaluator) and adds the interactive behaviors a console session needs —
//! statement continuation, auto-print of visible results, and surfacing of
//! `print()` output. It is the direct sibling of `s-repl`'s `SRepl`; only the
//! interpreter differs (R parsing instead of S). See `code/specs/R00-r-language.md`.
//!
//! Like the S REPL it is hand-rolled rather than built on the generic `repl`
//! crate, because the interpreter is single-threaded (its environments are
//! `Rc<RefCell<…>>`) and cannot meet that crate's `Send + Sync` bound. An R
//! session is inherently sequential anyway.

use coding_adventures_r_runtime::RInterpreter;
use std::io::{BufRead, Write};

/// What the REPL should do after being fed one physical line.
#[derive(Debug, PartialEq, Eq)]
pub enum ReplResponse {
    /// Text to display (may be empty — print nothing then).
    Output(String),
    /// The current statement is incomplete; read another line (`+ ` prompt).
    NeedMore,
    /// End the session.
    Quit,
}

/// A persistent interactive R session.
pub struct RRepl {
    interp: RInterpreter,
    buffer: String,
}

impl Default for RRepl {
    fn default() -> Self {
        Self::new()
    }
}

impl RRepl {
    /// Start a new session with all (shared) built-ins available.
    pub fn new() -> Self {
        RRepl {
            interp: RInterpreter::new(),
            buffer: String::new(),
        }
    }

    /// The prompt to show next: `> ` for a fresh statement, `+ ` while
    /// continuing an incomplete one.
    pub fn prompt(&self) -> &'static str {
        if self.buffer.is_empty() {
            "> "
        } else {
            "+ "
        }
    }

    /// Feed one physical input line (without its trailing newline).
    pub fn feed(&mut self, line: &str) -> ReplResponse {
        if self.buffer.is_empty() {
            match line.trim() {
                "q()" | "quit()" | ":quit" => return ReplResponse::Quit,
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

        match self.interp.eval_str(&src) {
            // The runtime already auto-prints a visible top-level result (through
            // the shared S3 `print` generic) into `printed`, so we just surface
            // that — appending again would double the output.
            Ok(outcome) => ReplResponse::Output(outcome.printed),
            Err(e) => ReplResponse::Output(format!("Error: {e}\n")),
        }
    }

    /// Whether the REPL is mid-way through an incomplete statement.
    pub fn is_continuing(&self) -> bool {
        !self.buffer.is_empty()
    }
}

/// Is the accumulated source an incomplete statement — unbalanced `(`/`[`/`{`
/// or an unterminated string? Strings and `#` comments are skipped so brackets
/// inside them do not affect the count. (Identical to the S REPL's rule.)
fn is_incomplete(src: &str) -> bool {
    let mut depth: i32 = 0;
    let mut in_string: Option<char> = None;
    let mut in_comment = false;

    for ch in src.chars() {
        if in_comment {
            if ch == '\n' {
                in_comment = false;
            }
            continue;
        }
        if let Some(quote) = in_string {
            if ch == quote {
                in_string = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' => in_string = Some(ch),
            '#' => in_comment = true,
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            _ => {}
        }
    }

    depth > 0 || in_string.is_some()
}

/// Drive a full interactive R session over the given reader and writer.
pub fn run<R: BufRead, W: Write>(mut reader: R, mut writer: W) -> std::io::Result<()> {
    let mut repl = RRepl::new();
    writeln!(writer, "R (reusing the S evaluator) — type q() to quit.")?;

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

    fn run_lines(lines: &[&str]) -> String {
        let mut repl = RRepl::new();
        let mut out = String::new();
        for line in lines {
            if let ReplResponse::Output(s) = repl.feed(line) {
                out.push_str(&s);
            }
        }
        out
    }

    #[test]
    fn auto_prints_visible_results() {
        assert_eq!(
            run_lines(&["data_frame <- c(1, 2, 3)", "mean(data_frame)"]),
            "[1] 2\n"
        );
    }

    #[test]
    fn assignment_is_invisible() {
        assert_eq!(run_lines(&["x <- 5"]), "");
        assert_eq!(run_lines(&["x = 5"]), ""); // `=` assignment also invisible
    }

    #[test]
    fn continuation_across_parens_and_braces() {
        let mut repl = RRepl::new();
        assert_eq!(repl.feed("mean(c(1,"), ReplResponse::NeedMore);
        assert_eq!(repl.prompt(), "+ ");
        assert_eq!(
            repl.feed("2, 3))"),
            ReplResponse::Output("[1] 2\n".to_string())
        );
        assert_eq!(
            run_lines(&["s <- 0", "for (i in 1:3) {", "  s <- s + i", "}", "s"]),
            "[1] 6\n"
        );
    }

    #[test]
    fn print_output_then_value() {
        assert_eq!(run_lines(&["print(c(1, 2))"]), "[1] 1 2\n");
    }

    #[test]
    fn errors_are_recoverable() {
        let mut repl = RRepl::new();
        assert_eq!(
            repl.feed("nope"),
            ReplResponse::Output("Error: object 'nope' not found\n".to_string())
        );
        assert_eq!(
            repl.feed("1 + 1"),
            ReplResponse::Output("[1] 2\n".to_string())
        );
    }

    #[test]
    fn quit_and_blank() {
        assert_eq!(RRepl::new().feed("q()"), ReplResponse::Quit);
        assert_eq!(RRepl::new().feed(""), ReplResponse::Output(String::new()));
        assert!(!RRepl::new().is_continuing());
    }

    #[test]
    fn run_driver_scripted_session() {
        let input = b"data_frame <- c(1, 2, 3)\nmean(data_frame)\nq()\n";
        let mut output = Vec::new();
        run(std::io::Cursor::new(&input[..]), &mut output).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("[1] 2"), "missing result in {text:?}");
        assert!(text.contains("> "));
    }

    #[test]
    fn run_driver_handles_eof_and_continuation() {
        let input = b"sum(1,\n2,\n3)\n";
        let mut output = Vec::new();
        run(std::io::Cursor::new(&input[..]), &mut output).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("[1] 6"));
        assert!(text.contains("+ "));
    }
}
