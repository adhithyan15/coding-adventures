//! # S REPL — an interactive Read-Eval-Print loop for historical Bell Labs S.
//!
//! [`SRepl`] wraps a persistent [`Interpreter`] and adds the interactive
//! behaviors a console session needs:
//!
//! - **Continuation.** A line with unbalanced `(`/`[`/`{` or an open string is
//!   incomplete; the REPL keeps reading (showing the `+ ` continuation prompt)
//!   until the statement is whole — exactly how S's console behaves.
//! - **Auto-print.** A *visible* top-level result is printed using S's
//!   `[i]`-prefixed vector layout. Assignments and loops are invisible.
//! - **`print()` output.** Anything a program prints during evaluation is shown
//!   ahead of the auto-printed result.
//!
//! ## Why this is hand-rolled rather than built on the `repl` crate
//!
//! The generic `repl` crate runs evaluation on a background thread and so
//! requires the language backend to be `Send + Sync`. The S interpreter is
//! deliberately single-threaded — its environments are `Rc<RefCell<…>>`, which
//! S closures share and mutate — so it cannot satisfy that bound. The S session
//! is inherently sequential anyway (each line mutates the global environment),
//! so a direct single-threaded driver is the right fit.

use coding_adventures_s_runtime::{format_value, Interpreter};
use std::io::{BufRead, Write};

/// What the REPL should do after being fed one physical line.
#[derive(Debug, PartialEq, Eq)]
pub enum ReplResponse {
    /// Text to display to the user (may be empty — print nothing then).
    Output(String),
    /// The current statement is incomplete; read another line (`+ ` prompt).
    NeedMore,
    /// End the session.
    Quit,
}

/// A persistent interactive S session.
pub struct SRepl {
    interp: Interpreter,
    /// Accumulated text of an as-yet-incomplete statement.
    buffer: String,
}

impl Default for SRepl {
    fn default() -> Self {
        Self::new()
    }
}

impl SRepl {
    /// Start a new session with all built-ins available.
    pub fn new() -> Self {
        SRepl {
            interp: Interpreter::new(),
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

    /// Feed one physical input line (without its trailing newline) and get the
    /// REPL's response.
    pub fn feed(&mut self, line: &str) -> ReplResponse {
        // Quit words are only recognized at the start of a fresh statement.
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
            Ok(outcome) => {
                let mut out = outcome.printed;
                if outcome.visible {
                    out.push_str(&format_value(&outcome.value).join("\n"));
                    out.push('\n');
                }
                ReplResponse::Output(out)
            }
            Err(e) => ReplResponse::Output(format!("Error: {e}\n")),
        }
    }

    /// Whether the REPL is mid-way through an incomplete statement.
    pub fn is_continuing(&self) -> bool {
        !self.buffer.is_empty()
    }
}

/// Is the accumulated source an incomplete statement — unbalanced brackets or
/// an unterminated string literal? Strings and `#` comments are skipped so that
/// brackets inside them do not affect the count.
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

/// Drive a full interactive session over the given reader and writer: show
/// prompts, read lines (with continuation), evaluate, and print results. The
/// loop ends on a quit word or EOF. Generic over the I/O streams so it can be
/// driven by stdin/stdout in the `s` binary and by in-memory buffers in tests.
pub fn run<R: BufRead, W: Write>(mut reader: R, mut writer: W) -> std::io::Result<()> {
    let mut repl = SRepl::new();
    writeln!(writer, "S — historical Bell Labs S (v1). Type q() to quit.")?;

    loop {
        write!(writer, "{}", repl.prompt())?;
        writer.flush()?;

        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            // EOF (Ctrl-D).
            writeln!(writer)?;
            break;
        }
        // Strip the trailing newline (and a Windows carriage return).
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

    /// Drive the REPL with a sequence of lines and concatenate its output.
    fn run(lines: &[&str]) -> String {
        let mut repl = SRepl::new();
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
        assert_eq!(run(&["x <- c(1, 2, 3)", "mean(x)"]), "[1] 2\n");
    }

    #[test]
    fn assignment_is_not_auto_printed() {
        assert_eq!(run(&["x <- 5"]), "");
    }

    #[test]
    fn print_output_then_value() {
        // print(x) emits the value; the call result is invisible (no double).
        assert_eq!(run(&["print(c(1, 2))"]), "[1] 1 2\n");
    }

    #[test]
    fn continuation_across_unbalanced_parens() {
        let mut repl = SRepl::new();
        assert_eq!(repl.feed("mean(c(1,"), ReplResponse::NeedMore);
        assert_eq!(repl.prompt(), "+ ");
        assert_eq!(
            repl.feed("2, 3))"),
            ReplResponse::Output("[1] 2\n".to_string())
        );
        assert_eq!(repl.prompt(), "> ");
    }

    #[test]
    fn continuation_across_braces() {
        let out = run(&["s <- 0", "for (i in 1:3) {", "  s <- s + i", "}", "s"]);
        assert_eq!(out, "[1] 6\n");
    }

    #[test]
    fn continuation_across_open_string() {
        let mut repl = SRepl::new();
        assert_eq!(repl.feed("x <- \"abc"), ReplResponse::NeedMore);
    }

    #[test]
    fn errors_are_reported_and_recoverable() {
        let mut repl = SRepl::new();
        let r = repl.feed("nope");
        assert_eq!(
            r,
            ReplResponse::Output("Error: object 'nope' not found\n".to_string())
        );
        // The session continues afterward.
        assert_eq!(
            repl.feed("1 + 1"),
            ReplResponse::Output("[1] 2\n".to_string())
        );
    }

    #[test]
    fn quit_words() {
        assert_eq!(SRepl::new().feed("q()"), ReplResponse::Quit);
        assert_eq!(SRepl::new().feed(":quit"), ReplResponse::Quit);
    }

    #[test]
    fn blank_line_does_nothing() {
        assert_eq!(SRepl::new().feed(""), ReplResponse::Output(String::new()));
    }

    #[test]
    fn historical_underscore_session() {
        assert_eq!(run(&["y _ c(10, 20)", "sum(y)"]), "[1] 30\n");
    }

    #[test]
    fn state_persists_between_lines() {
        assert_eq!(run(&["a <- 10", "b <- 20", "a + b"]), "[1] 30\n");
    }

    #[test]
    fn is_continuing_reflects_buffer_state() {
        let mut repl = SRepl::new();
        assert!(!repl.is_continuing());
        repl.feed("c(1,");
        assert!(repl.is_continuing());
    }

    // --- The run() driver (the binary's logic) --------------------------

    #[test]
    fn run_drives_a_scripted_session_until_quit() {
        let input = b"x <- c(1, 2, 3)\nmean(x)\nq()\n";
        let mut output = Vec::new();
        super::run(std::io::Cursor::new(&input[..]), &mut output).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("[1] 2"), "missing result in: {text:?}");
        assert!(text.contains("> "), "missing prompt in: {text:?}");
    }

    #[test]
    fn run_ends_cleanly_at_eof() {
        // No quit word — input simply ends; run() must return Ok.
        let input = b"1 + 1\n";
        let mut output = Vec::new();
        super::run(std::io::Cursor::new(&input[..]), &mut output).unwrap();
        assert!(String::from_utf8(output).unwrap().contains("[1] 2"));
    }

    #[test]
    fn run_continues_across_multiple_lines() {
        let input = b"sum(c(1,\n2,\n3))\nq()\n";
        let mut output = Vec::new();
        super::run(std::io::Cursor::new(&input[..]), &mut output).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("[1] 6"), "missing result in: {text:?}");
        // The continuation prompt should have appeared.
        assert!(
            text.contains("+ "),
            "missing continuation prompt in: {text:?}"
        );
    }
}
