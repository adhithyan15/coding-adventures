//! # Wolfram REPL — an interactive Read-Eval-Print loop for the Wolfram Language.
//!
//! [`WolframRepl`] wraps a persistent [`WolframSession`] (which reuses the entire
//! shared symbolic stack) and adds the console behaviours a Mathematica user
//! expects:
//!
//! * **`In[n]:= ` / `... ` prompts** — the input prompt shows the next input
//!   index; the continuation prompt is shown while a statement spans physical
//!   lines.
//! * **Line continuation** — unlike Maxima (terminated by `;`/`$`), a Wolfram
//!   statement is terminated by a **newline** once brackets are balanced and no
//!   string/comment is open. So the REPL keeps reading physical lines while a
//!   `[ ]`/`{ }`/`( )` is still open or a `"…"`/`(* *)` is unterminated, then
//!   submits the whole buffer. The accumulation buffer is size-capped so an input
//!   that never balances cannot grow memory without bound.
//! * **Quit / EOF** — `Quit`, `Quit[]`, `Exit`, `Exit[]` (case-insensitive
//!   convenience: also `quit`/`exit`), or Ctrl-D end the session.
//! * **Non-fatal errors** — a surface error prints and the session continues.
//!
//! This mirrors `maxima-repl`'s single-threaded driver; the only Wolfram-specific
//! part is the *newline*-terminates rule (vs Maxima's `;`/`$`).

use coding_adventures_wolfram_runtime::{WolframSession, MAX_INPUT_LEN};
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

/// A persistent interactive Wolfram session.
pub struct WolframRepl {
    session: WolframSession,
    buffer: String,
    /// The 1-based input index shown in the `In[n]:=` prompt. It advances once
    /// per *submitted* batch, mirroring Mathematica's `In[n]` counter.
    input_index: usize,
}

impl Default for WolframRepl {
    fn default() -> Self {
        Self::new()
    }
}

impl WolframRepl {
    pub fn new() -> Self {
        WolframRepl {
            session: WolframSession::new(),
            buffer: String::new(),
            input_index: 1,
        }
    }

    /// The prompt to print before reading the next physical line.
    pub fn prompt(&self) -> String {
        if self.buffer.is_empty() {
            format!("In[{}]:= ", self.input_index)
        } else {
            "... ".to_string()
        }
    }

    /// Feed one physical input line (without its trailing newline).
    pub fn feed(&mut self, line: &str) -> ReplResponse {
        // Quit words are only recognised at the start of a fresh statement, so an
        // identifier like `Exit` mid-expression is never hijacked.
        if self.buffer.is_empty() {
            match line.trim() {
                "Quit" | "Quit[]" | "quit" | "Exit" | "Exit[]" | "exit" => {
                    return ReplResponse::Quit
                }
                _ => {}
            }
        }
        self.buffer.push_str(line);
        self.buffer.push('\n');

        // Bound the accumulation buffer: a stream that never balances (an
        // unterminated string/comment, or endless open brackets) must not buffer
        // unbounded memory. Once over the size the session would reject anyway,
        // submit so `feed` returns the clean "too large" error and resets.
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
/// A Wolfram statement is complete at a newline once bracket depth is zero and no
/// string/comment is open. So this returns `true` while either brackets are still
/// open, or a `"`-string or `(* *)` comment is unterminated. Strings and comments
/// are tracked exactly as the lexer treats them — a `(` inside a string or a `"`
/// inside a comment does not change depth — so a stray bracket in either does not
/// wedge the prompt. This is only a continuation heuristic: whatever is submitted
/// still passes through [`WolframSession::feed`], which re-lexes with the real
/// lexer and applies the size/complexity guards, so a mismatch can never crash —
/// at worst it submits early or asks for one more line.
fn is_incomplete(src: &str) -> bool {
    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut in_comment = false;
    let mut escaped = false;

    let mut chars = src.chars().peekable();
    while let Some(ch) = chars.next() {
        if in_comment {
            // Non-nesting `(* … *)` closes at the first `*)`.
            if ch == '*' && chars.peek() == Some(&')') {
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
            '(' if chars.peek() == Some(&'*') => {
                chars.next();
                in_comment = true;
            }
            '"' => in_string = true,
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            _ => {}
        }
    }

    in_string || in_comment || depth > 0
}

/// Drive a full interactive Wolfram session over the given reader and writer.
pub fn run<R: BufRead, W: Write>(mut reader: R, mut writer: W) -> std::io::Result<()> {
    let mut repl = WolframRepl::new();
    writeln!(
        writer,
        "Wolfram Language (on the shared symbolic stack) — one statement per line, type Quit to exit."
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
    fn a_single_line_statement_evaluates_immediately() {
        let mut r = WolframRepl::new();
        assert!(matches!(r.feed("1 + 2*3"), ReplResponse::Output(t) if t.contains("7")));
    }

    #[test]
    fn continues_while_a_bracket_is_open() {
        let mut r = WolframRepl::new();
        assert_eq!(r.feed("f[1,"), ReplResponse::NeedMore); // open bracket
                                                            // Once balanced, a newline terminates and it submits (f is unknown, so it
                                                            // echoes the unevaluated application).
        assert!(matches!(r.feed("2]"), ReplResponse::Output(t) if t.contains("f[1, 2]")));
    }

    #[test]
    fn continues_while_a_brace_list_spans_lines() {
        let mut r = WolframRepl::new();
        assert_eq!(r.feed("{1,"), ReplResponse::NeedMore);
        assert!(matches!(r.feed("2, 3}"), ReplResponse::Output(t) if t.contains("{1, 2, 3}")));
    }

    #[test]
    fn a_bracket_inside_a_string_does_not_continue() {
        let mut r = WolframRepl::new();
        // The `[` is inside the string, so depth stays 0 and the line completes.
        assert!(matches!(r.feed("\"a[b\""), ReplResponse::Output(_)));
    }

    #[test]
    fn an_unterminated_string_keeps_reading() {
        let mut r = WolframRepl::new();
        assert_eq!(r.feed("\"open"), ReplResponse::NeedMore);
        assert!(matches!(r.feed("closed\""), ReplResponse::Output(_)));
    }

    #[test]
    fn a_bracket_inside_a_comment_does_not_continue() {
        let mut r = WolframRepl::new();
        assert!(matches!(r.feed("(* [ *) 2 + 2"), ReplResponse::Output(t) if t.contains("4")));
    }

    #[test]
    fn an_unterminated_buffer_is_submitted_once_over_the_size_cap() {
        let mut r = WolframRepl::new();
        let big = "(".repeat(MAX_INPUT_LEN + 16);
        match r.feed(&big) {
            ReplResponse::Output(t) => assert!(t.contains("too large"), "got {t:?}"),
            other => panic!("expected an over-size submission, got {other:?}"),
        }
        // Buffer reset, so the prompt is back to a fresh input prompt at In[2].
        assert_eq!(r.prompt(), "In[2]:= ");
    }

    #[test]
    fn prompts_switch_between_input_and_continuation() {
        let mut r = WolframRepl::new();
        assert_eq!(r.prompt(), "In[1]:= ");
        assert_eq!(r.feed("g["), ReplResponse::NeedMore);
        assert_eq!(r.prompt(), "... ", "continuation prompt while open");
        assert!(r.is_continuing());
        assert!(matches!(r.feed("x]"), ReplResponse::Output(_)));
        assert_eq!(r.prompt(), "In[2]:= ", "input index advanced");
    }

    #[test]
    fn quit_words_leave_the_session() {
        assert_eq!(WolframRepl::new().feed("Quit"), ReplResponse::Quit);
        assert_eq!(WolframRepl::new().feed("Quit[]"), ReplResponse::Quit);
        assert_eq!(WolframRepl::new().feed("Exit"), ReplResponse::Quit);
        assert_eq!(WolframRepl::new().feed("exit"), ReplResponse::Quit);
    }

    #[test]
    fn an_error_is_printed_and_the_session_survives() {
        let mut r = WolframRepl::new();
        assert!(matches!(r.feed("1 +"), ReplResponse::Output(t) if t.contains("Error")));
        // still usable afterwards
        assert!(matches!(r.feed("1 + 1"), ReplResponse::Output(t) if t.contains("2")));
    }

    #[test]
    fn bindings_persist_across_lines() {
        let mut r = WolframRepl::new();
        assert!(matches!(r.feed("x = 5;"), ReplResponse::Output(_)));
        assert!(matches!(r.feed("x + 1"), ReplResponse::Output(t) if t.contains("6")));
    }

    #[test]
    fn run_drives_a_session_to_eof() {
        let input = b"1 + 2*3\nQuit\n" as &[u8];
        let mut output = Vec::new();
        run(input, &mut output).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("7"), "expected 7 in session: {text:?}");
        assert!(
            text.contains("In[1]:="),
            "expected the input prompt: {text:?}"
        );
    }

    #[test]
    fn run_handles_bare_eof_without_quit() {
        let input = b"2 + 2\n" as &[u8];
        let mut output = Vec::new();
        run(input, &mut output).unwrap();
        assert!(String::from_utf8(output).unwrap().contains("4"));
    }

    #[test]
    fn blank_lines_are_harmless() {
        let mut r = WolframRepl::new();
        assert_eq!(r.feed(""), ReplResponse::Output(String::new()));
        assert_eq!(r.prompt(), "In[1]:= ", "a blank line does not advance In");
    }
}
