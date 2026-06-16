//! # Maxima REPL — an interactive Read-Eval-Print loop for the Maxima CAS.
//!
//! [`MaximaRepl`] wraps a persistent [`MaximaSession`] (which reuses the entire
//! Macsyma symbolic stack) and adds the console behaviours Maxima users expect:
//!
//! * **`(%i«n») ` / `... ` prompts** — the input prompt shows the next input
//!   index; the continuation prompt is shown while a statement spans lines.
//! * **Line continuation** — Maxima statements end with a terminator: `;` to
//!   display the result or `$` to suppress it. The REPL keeps reading physical
//!   lines until it sees a terminator *outside a string* with all brackets
//!   balanced, then evaluates the whole buffer at once.
//! * **Quit / EOF** — `quit;`, `quit()`, `exit`, or Ctrl-D end the session.
//! * **Non-fatal errors** — a surface error prints and the session continues.
//!
//! This mirrors `octave-repl`'s single-threaded driver; the only Maxima-specific
//! part is the `;`/`$`-terminator continuation rule (vs Octave's `end`/`endX`
//! block rule), because Maxima is statement-terminated, not block-structured at
//! the REPL surface.

use coding_adventures_maxima_runtime::MaximaSession;
use std::io::{BufRead, Write};

/// What the REPL should do after being fed one physical line.
#[derive(Debug, PartialEq, Eq)]
pub enum ReplResponse {
    /// A complete statement (or batch) was evaluated; here is its echo.
    Output(String),
    /// The buffer is not yet a complete statement — read another line.
    NeedMore,
    /// The user asked to leave.
    Quit,
}

/// A persistent interactive Maxima session.
pub struct MaximaRepl {
    session: MaximaSession,
    buffer: String,
    /// The 1-based input index shown in the `(%i«n»)` prompt. It advances once
    /// per *submitted* statement batch, mirroring Maxima's `%i` counter.
    input_index: usize,
}

impl Default for MaximaRepl {
    fn default() -> Self {
        Self::new()
    }
}

impl MaximaRepl {
    pub fn new() -> Self {
        MaximaRepl {
            session: MaximaSession::new(),
            buffer: String::new(),
            input_index: 1,
        }
    }

    /// The prompt to print before reading the next physical line: the input
    /// prompt when starting a fresh statement, the continuation prompt while a
    /// statement is still open.
    pub fn prompt(&self) -> String {
        if self.buffer.is_empty() {
            format!("(%i{}) ", self.input_index)
        } else {
            "... ".to_string()
        }
    }

    /// Feed one physical input line (without its trailing newline).
    pub fn feed(&mut self, line: &str) -> ReplResponse {
        // Quit words are only recognised at the start of a fresh statement, so
        // an identifier like `exit` mid-expression is never hijacked. (Maxima
        // spells these as functions; we also accept the bare words.)
        if self.buffer.is_empty() {
            match line.trim() {
                "quit" | "quit;" | "quit()" | "quit();" | "exit" | "exit;" | "exit()"
                | "exit();" => return ReplResponse::Quit,
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
        self.input_index += 1;
        match self.session.feed(&src) {
            Ok(text) => ReplResponse::Output(text),
            Err(e) => ReplResponse::Output(format!("Error: {e}\n")),
        }
    }

    /// Is a statement currently spanning multiple lines?
    pub fn is_continuing(&self) -> bool {
        !self.buffer.is_empty()
    }
}

/// Is the accumulated source *incomplete* — i.e. should the REPL keep reading?
///
/// A Maxima statement is complete once a terminator (`;` or `$`) has appeared
/// **outside a string** with bracket/paren/brace depth back to zero. So this
/// returns `true` while either:
///
/// * brackets are still open (`depth > 0`), or
/// * a `"`-string is unterminated, or
/// * no `;`/`$` terminator has yet been seen outside a string.
///
/// `"`-delimited strings are tracked (with `\"` escapes) so a `;` *inside* a
/// string — `s : "a;b";` — is not mistaken for a terminator.
fn is_incomplete(src: &str) -> bool {
    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut escaped = false;
    let mut saw_terminator = false;

    for ch in src.chars() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ';' | '$' if depth <= 0 => saw_terminator = true,
            _ => {}
        }
    }

    in_string || depth > 0 || !saw_terminator
}

/// Drive a full interactive Maxima session over the given reader and writer.
pub fn run<R: BufRead, W: Write>(mut reader: R, mut writer: W) -> std::io::Result<()> {
    let mut repl = MaximaRepl::new();
    writeln!(
        writer,
        "Maxima (on the Macsyma symbolic stack) — end statements with ; or $, type quit; to exit."
    )?;

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
    fn continues_until_a_semicolon_terminator() {
        let mut r = MaximaRepl::new();
        assert_eq!(r.feed("diff(x^3,"), ReplResponse::NeedMore); // open paren
        assert_eq!(r.feed("  x)"), ReplResponse::NeedMore); // balanced, but no ;/$
        assert!(matches!(r.feed(";"), ReplResponse::Output(t) if t.contains('3')));
    }

    #[test]
    fn dollar_is_also_a_terminator_and_suppresses() {
        let mut r = MaximaRepl::new();
        // `x : 5$` is complete (ends in $) and prints nothing.
        assert_eq!(r.feed("x : 5$"), ReplResponse::Output(String::new()));
        // The binding persists across statements.
        assert!(matches!(r.feed("x + 1;"), ReplResponse::Output(t) if t.contains('6')));
    }

    #[test]
    fn a_semicolon_inside_a_string_is_not_a_terminator() {
        let mut r = MaximaRepl::new();
        // The ; is inside the string, so the statement is still open until the
        // closing quote and the real terminator.
        assert_eq!(r.feed("s : \"a;b\""), ReplResponse::NeedMore);
        assert!(matches!(r.feed(";"), ReplResponse::Output(_)));
    }

    #[test]
    fn prompts_switch_between_input_and_continuation() {
        let mut r = MaximaRepl::new();
        assert_eq!(r.prompt(), "(%i1) ");
        assert_eq!(r.feed("factor("), ReplResponse::NeedMore);
        assert_eq!(r.prompt(), "... ", "continuation prompt while open");
        assert!(matches!(r.feed("x^2 - 1);"), ReplResponse::Output(_)));
        assert_eq!(r.prompt(), "(%i2) ", "input index advanced");
    }

    #[test]
    fn quit_words_leave_the_session() {
        assert_eq!(MaximaRepl::new().feed("quit;"), ReplResponse::Quit);
        assert_eq!(MaximaRepl::new().feed("quit()"), ReplResponse::Quit);
        assert_eq!(MaximaRepl::new().feed("exit"), ReplResponse::Quit);
    }

    #[test]
    fn an_error_is_printed_and_the_session_survives() {
        let mut r = MaximaRepl::new();
        assert!(matches!(r.feed("@#$;"), ReplResponse::Output(t) if t.contains("Error")));
        // still usable afterwards
        assert!(matches!(r.feed("1 + 1;"), ReplResponse::Output(t) if t.contains('2')));
    }

    #[test]
    fn run_drives_a_session_to_eof() {
        let input = b"1 + 2*3;\nquit;\n" as &[u8];
        let mut output = Vec::new();
        run(input, &mut output).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains('7'), "expected 7 in session: {text:?}");
        assert!(
            text.contains("(%i1)"),
            "expected the input prompt: {text:?}"
        );
    }

    #[test]
    fn run_handles_bare_eof_without_quit() {
        // No quit; just EOF — the loop should end cleanly.
        let input = b"2 + 2;\n" as &[u8];
        let mut output = Vec::new();
        run(input, &mut output).unwrap();
        assert!(String::from_utf8(output).unwrap().contains('4'));
    }
}
