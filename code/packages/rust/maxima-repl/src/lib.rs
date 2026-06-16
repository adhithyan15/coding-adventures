//! # Maxima REPL — an interactive Read-Eval-Print loop for the Maxima CAS.
//!
//! [`MaximaRepl`] wraps a persistent [`MaximaSession`] (which reuses the entire
//! Macsyma symbolic stack) and adds the console behaviours Maxima users expect:
//!
//! * **`(%i«n») ` / `... ` prompts** — the input prompt shows the next input
//!   index; the continuation prompt is shown while a statement spans lines.
//! * **Line continuation** — Maxima statements end with a terminator: `;` to
//!   display the result or `$` to suppress it. The REPL keeps reading physical
//!   lines until it sees a terminator *outside a string or `/* */` comment* with
//!   all brackets balanced, then evaluates the whole buffer at once. The
//!   accumulation buffer is size-capped so an input that never terminates cannot
//!   grow memory without bound.
//! * **Quit / EOF** — `quit;`, `quit()`, `exit`, or Ctrl-D end the session.
//! * **Non-fatal errors** — a surface error prints and the session continues.
//!
//! This mirrors `octave-repl`'s single-threaded driver; the only Maxima-specific
//! part is the `;`/`$`-terminator continuation rule (vs Octave's `end`/`endX`
//! block rule), because Maxima is statement-terminated, not block-structured at
//! the REPL surface.

use coding_adventures_maxima_runtime::{MaximaSession, MAX_INPUT_LEN};
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

        // Bound the accumulation buffer: a stream that never satisfies the
        // continuation rule (an unterminated string/comment, or endless open
        // brackets) must not grow memory without limit. Once we are over the
        // size the session itself would reject anyway, stop waiting and submit
        // it so `feed` returns the clean "input too large" error and the buffer
        // is reset — rather than buffering unbounded input.
        if self.buffer.len() <= MAX_INPUT_LEN && is_incomplete(&self.buffer) {
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
/// * a `"`-string or a `/* */` comment is unterminated, or
/// * no `;`/`$` terminator has yet been seen outside a string/comment.
///
/// Strings (`"…"`, with `\"` escapes) and C-style `/* … */` comments are tracked
/// exactly as the macsyma lexer treats them — non-nesting comments, and a `"`
/// only opens a string outside a comment — so a `;` *inside* either (`s :
/// "a;b";`, or `/* end; */ x;`) is not mistaken for a terminator, and a stray
/// `"` inside a comment does not wedge the prompt into a phantom string. This is
/// only a continuation heuristic: whatever is finally submitted still passes
/// through `MaximaSession::feed`, which re-lexes with the real lexer and applies
/// the size/complexity guards, so a mismatch here can never crash — at worst it
/// submits early or asks for one more line.
fn is_incomplete(src: &str) -> bool {
    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut in_comment = false;
    let mut escaped = false;
    let mut saw_terminator = false;

    let mut chars = src.chars().peekable();
    while let Some(ch) = chars.next() {
        if in_comment {
            // A non-nesting `/* … */` comment closes at the first `*/`.
            if ch == '*' && chars.peek() == Some(&'/') {
                chars.next();
                in_comment = false;
            }
            continue;
        }
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
            // `/*` opens a comment (only outside a string — handled above).
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                in_comment = true;
            }
            '"' => in_string = true,
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ';' | '$' if depth <= 0 => saw_terminator = true,
            _ => {}
        }
    }

    in_string || in_comment || depth > 0 || !saw_terminator
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
    fn a_terminator_inside_a_comment_is_not_a_terminator() {
        // `/* … ; … */` — the `;` is inside a block comment, so the statement is
        // still open until the real terminator after the comment closes.
        let mut r = MaximaRepl::new();
        assert_eq!(r.feed("x : 1 /* end; here */"), ReplResponse::NeedMore);
        assert!(matches!(r.feed("+ 2;"), ReplResponse::Output(t) if t.contains('3')));
    }

    #[test]
    fn a_quote_inside_a_comment_does_not_wedge_the_prompt() {
        // A lone `"` inside a comment must not flip the continuation logic into a
        // never-ending "string" — the statement completes at its real `;`.
        let mut r = MaximaRepl::new();
        assert!(
            matches!(r.feed("/* a \" quote */ 2 + 2;"), ReplResponse::Output(t) if t.contains('4'))
        );
    }

    #[test]
    fn an_unterminated_buffer_is_submitted_once_it_passes_the_size_cap() {
        // A never-closing bracket run must not buffer unbounded memory: once the
        // accumulation exceeds the size cap it is submitted and the session
        // returns the clean "too large" error instead of asking for more forever.
        let mut r = MaximaRepl::new();
        let big = "(".repeat(MAX_INPUT_LEN + 16);
        match r.feed(&big) {
            ReplResponse::Output(t) => assert!(t.contains("too large"), "got {t:?}"),
            other => panic!("expected an over-size submission, got {other:?}"),
        }
        // The buffer was reset, so the prompt is back to a fresh input prompt.
        assert_eq!(r.prompt(), "(%i2) ");
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
