//! # Reduce REPL — an interactive Read-Eval-Print loop for Reduce (a subset).
//!
//! [`ReduceRepl`] wraps a persistent [`ReduceSession`] (which reuses the
//! entire shared symbolic stack) and adds the console behaviours an
//! interactive Reduce user expects. Mirrors `derive-repl`'s driver almost
//! exactly (MA08 §2/§5 call it out as the template), with one deliberate
//! difference:
//!
//! * **No numbered prompt.** Real REDUCE's own interactive session prompt
//!   actually *is* numbered (`1: `, `2: `, …) — but MA08 §2/§5 are explicit
//!   that this subset's `reduce-repl` is "a plain read-eval-print loop, no
//!   numbering", the same way [`ReduceSession::feed`] (unlike
//!   `DeriveSession::feed`'s `#n:` lines) never prefixes a result. So the
//!   prompt here is a fixed `"> "` (continuation: `"... "`, identical to
//!   `derive-repl`'s spelling), not `#n:`/`In[n]:=`.
//! * **Line continuation** tracks `(`/`)` **and** `{`/`}` (Reduce's list
//!   braces, MA08 §3 — NOT Derive's `[`/`]`) **and** the two-character
//!   `<<`/`>>` group-statement delimiters (MA08 §3), distinguished from the
//!   single-character `<`/`>` comparison operators by looking one
//!   character ahead — naively counting every bare `<`/`>` would
//!   miscount `a < b` as an opening/closing group delimiter. Reduce has no
//!   strings or comments in this subset (MA08 §4), so — like `derive-repl`
//!   — there is no quote/comment state to track either.
//! * **Quit / EOF** — `QUIT`, `EXIT` (case-insensitive convenience, mirroring
//!   `derive-repl`'s identical convenience even though real REDUCE's own
//!   exit command is the semicolon-terminated procedure call `quit;`), or
//!   Ctrl-D end the session.
//! * **Non-fatal errors** — a surface error prints and the session continues.
//!
//! This mirrors `derive-repl`'s single-threaded driver, carrying forward its
//! `read_bounded_line` fix verbatim: [`run`] reads each physical line
//! through [`read_bounded_line`] (capped at [`MAX_LINE_LEN`]) rather than
//! `BufRead::read_line` directly — see that function's own doc comment for
//! the full rationale (unbounded `read_line`, and the multibyte-character/
//! cap-boundary edge case).

use coding_adventures_reduce_runtime::{ReduceSession, MAX_INPUT_LEN};
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

/// A persistent interactive Reduce session.
pub struct ReduceRepl {
    session: ReduceSession,
    buffer: String,
}

impl Default for ReduceRepl {
    fn default() -> Self {
        Self::new()
    }
}

impl ReduceRepl {
    pub fn new() -> Self {
        ReduceRepl {
            session: ReduceSession::new(),
            buffer: String::new(),
        }
    }

    /// The prompt to print before reading the next physical line.
    ///
    /// A plain, non-numbered prompt (MA08 §2/§5 — see the module doc
    /// comment) — `"> "` for a fresh statement, `"... "` while one spans
    /// multiple physical lines.
    pub fn prompt(&self) -> String {
        if self.buffer.is_empty() {
            "> ".to_string()
        } else {
            "... ".to_string()
        }
    }

    /// Feed one physical input line (without its trailing newline).
    pub fn feed(&mut self, line: &str) -> ReplResponse {
        // Quit words are only recognised at the start of a fresh statement,
        // so an identifier named `QUIT`/`EXIT` mid-expression is never
        // hijacked.
        if self.buffer.is_empty() {
            match line.trim() {
                "QUIT" | "quit" | "Quit" | "EXIT" | "exit" | "Exit" => return ReplResponse::Quit,
                _ => {}
            }
        }
        self.buffer.push_str(line);
        self.buffer.push('\n');

        // Bound the accumulation buffer: a stream that never balances (an
        // endless run of open brackets) must not buffer unbounded memory.
        // Once over the size the session would reject anyway, submit so
        // `feed` returns the clean "too large" error and resets.
        if self.buffer.len() <= MAX_INPUT_LEN && is_incomplete(&self.buffer) {
            return ReplResponse::NeedMore;
        }

        let src = std::mem::take(&mut self.buffer);
        if src.trim().is_empty() {
            return ReplResponse::Output(String::new());
        }
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

/// Upper bound on a single *physical* line read from the input stream,
/// applied in [`read_bounded_line`] before [`ReduceRepl::feed`]'s own
/// `MAX_INPUT_LEN` check ever runs. `BufRead::read_line` has no length bound
/// of its own — it grows its buffer until it sees a `\n` or EOF — so without
/// this, a single, arbitrarily long physical line (no embedded newline at
/// all) is fully buffered in memory before `feed` ever gets a chance to
/// reject anything. Mirrors `derive-repl`/`j-repl`/`apl-repl`'s own
/// `MAX_LINE_LEN` fix exactly (same value, same rationale).
const MAX_LINE_LEN: u64 = 64 * 1024;

/// Read one physical line, bounded to [`MAX_LINE_LEN`] bytes.
///
/// - `Ok(None)` — genuine end of input.
/// - `Ok(Some(Ok(line)))` — an ordinary line. Includes a final, newline-less
///   line at genuine EOF (same as `BufRead::read_line`'s own contract).
/// - `Ok(Some(Err(())))` — a single physical line's true length exceeds the
///   cap; the oversized remainder is fully drained and discarded so the next
///   read always starts at a genuine line boundary.
///
/// Reads raw **bytes** (`read_until(b'\n', ..)`), not `BufRead::read_line`
/// directly, deciding "oversized?" on the byte run *before* attempting UTF-8
/// decoding — `Take` stops at an arbitrary byte offset with no notion of a
/// character boundary, so a valid UTF-8 line whose true length only slightly
/// exceeds the cap could have a multi-byte character straddle that exact
/// offset, and validating on the byte run through `read_line` alone would
/// misreport that as a decoding failure rather than an oversized line.
/// Mirrors `derive-repl::read_bounded_line`/`j-repl::read_bounded_line`
/// exactly.
fn read_bounded_line<R: BufRead>(reader: &mut R) -> std::io::Result<Option<Result<String, ()>>> {
    let mut buf: Vec<u8> = Vec::new();
    // Explicit UFCS + an explicit reborrow (`&mut *reader`): `Read`/`BufRead`
    // are implemented both for `R` itself and for `&mut R`, and ordinary
    // autoref method resolution prefers the by-value `R` candidate even when
    // the receiver is already a reference — silently trying to MOVE
    // `*reader` instead of taking a bounded view of it. Naming the trait and
    // the exact `&mut R` receiver explicitly removes that ambiguity.
    let mut limited: std::io::Take<&mut R> = std::io::Read::take(&mut *reader, MAX_LINE_LEN);
    let read = limited.read_until(b'\n', &mut buf)?;
    if read == 0 {
        return Ok(None);
    }
    let hit_cap = buf.len() as u64 == MAX_LINE_LEN && buf.last() != Some(&b'\n');
    if hit_cap {
        // The initial capped read filled up with no newline in sight — keep
        // reading further capped chunks (discarding them) until either a
        // real `\n` is found (genuinely oversized) or a chunk comes back
        // empty (a maximal-but-not-oversized line at genuine EOF).
        let mut saw_more_data = false;
        loop {
            let mut chunk: Vec<u8> = Vec::new();
            let mut limited: std::io::Take<&mut R> =
                std::io::Read::take(&mut *reader, MAX_LINE_LEN);
            let n = limited.read_until(b'\n', &mut chunk)?;
            if n == 0 {
                if !saw_more_data {
                    return match String::from_utf8(buf) {
                        Ok(line) => Ok(Some(Ok(line))),
                        Err(e) => Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
                    };
                }
                break; // true EOF reached while draining real overflow
            }
            saw_more_data = true;
            if chunk.last() == Some(&b'\n') {
                break; // found the oversized line's real end
            }
        }
        return Ok(Some(Err(())));
    }
    match String::from_utf8(buf) {
        Ok(line) => Ok(Some(Ok(line))),
        Err(e) => Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
    }
}

/// Is the accumulated source *incomplete* — i.e. should the REPL keep
/// reading?
///
/// A Reduce statement (unlike Derive's, whose separator IS the newline) is
/// terminated by `;`/`$`, and this subset's grammar tolerates a final
/// statement with no terminator at all (`reduce-parser`'s own
/// `bare_trailing_statement_with_no_terminator_parses` test) — so this
/// heuristic tracks bracket balance, not terminators, exactly mirroring
/// `derive-repl::is_incomplete`'s identical "wait for balance, then submit
/// the whole buffer" shape, just over Reduce's own bracket vocabulary:
/// `(`/`)` (grouping and calls), `{`/`}` (list literals — MA08 §3, NOT
/// Derive's `[`/`]`), and the two-character `<<`/`>>` group-statement
/// delimiters, which must be matched as a *pair* — a bare single `<` or `>`
/// is an ordinary comparison operator (MA08 §3) and must NOT perturb the
/// depth count.
///
/// This is only a continuation *heuristic*: whatever is submitted still
/// passes through [`ReduceSession::feed`], which re-lexes with the real
/// lexer and applies the size/complexity guards, so a mismatch can never
/// crash — at worst it submits early (e.g. a statement deliberately split
/// across lines with no open bracket at all, like `x :=\n5;`) or asks for
/// one more line.
fn is_incomplete(src: &str) -> bool {
    let chars: Vec<char> = src.chars().collect();
    let mut depth: i32 = 0;
    let mut i = 0;
    while i < chars.len() {
        match (chars[i], chars.get(i + 1)) {
            ('<', Some('<')) => {
                depth += 1;
                i += 2;
                continue;
            }
            ('>', Some('>')) => {
                depth -= 1;
                i += 2;
                continue;
            }
            ('(', _) | ('{', _) => depth += 1,
            (')', _) | ('}', _) => depth -= 1,
            _ => {}
        }
        i += 1;
    }
    depth > 0
}

/// Drive a full interactive Reduce session over the given reader and writer.
pub fn run<R: BufRead, W: Write>(mut reader: R, mut writer: W) -> std::io::Result<()> {
    let mut repl = ReduceRepl::new();
    writeln!(
        writer,
        "Reduce (on the shared symbolic stack) — one statement per line, type QUIT to exit."
    )?;

    loop {
        write!(writer, "{}", repl.prompt())?;
        writer.flush()?;

        let line = match read_bounded_line(&mut reader)? {
            None => {
                writeln!(writer)?;
                break;
            }
            Some(Err(())) => {
                writeln!(
                    writer,
                    "Error: line exceeds the {MAX_LINE_LEN}-byte limit; discarded"
                )?;
                writer.flush()?;
                continue;
            }
            Some(Ok(line)) => line,
        };
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
        let mut r = ReduceRepl::new();
        assert!(matches!(r.feed("1 + 2*3;"), ReplResponse::Output(t) if t.contains('7')));
    }

    #[test]
    fn continues_while_a_paren_is_open() {
        let mut r = ReduceRepl::new();
        assert_eq!(r.feed("h(1,"), ReplResponse::NeedMore);
        assert!(matches!(r.feed("2);"), ReplResponse::Output(_)));
    }

    #[test]
    fn continues_while_a_brace_is_open() {
        // A list literal spanning lines — buffers until the closing `}`.
        let mut r = ReduceRepl::new();
        assert_eq!(r.feed("{1,"), ReplResponse::NeedMore);
        assert!(matches!(
            r.feed("2, 3};"),
            ReplResponse::Output(t) if t.contains("{1, 2, 3}")
        ));
    }

    #[test]
    fn continues_while_a_group_statement_is_open() {
        // `<<`/`>>` group-statement delimiters, not bare `<`/`>`.
        let mut r = ReduceRepl::new();
        assert_eq!(r.feed("<< a := 1;"), ReplResponse::NeedMore);
        assert!(matches!(r.feed("a + 1 >>;"), ReplResponse::Output(_)));
    }

    #[test]
    fn a_bare_comparison_does_not_trigger_group_statement_continuation() {
        // `a < b` must NOT be mistaken for a half-open `<<` group delimiter.
        let mut r = ReduceRepl::new();
        assert!(matches!(r.feed("1 < 2;"), ReplResponse::Output(_)));
    }

    #[test]
    fn prompts_switch_between_input_and_continuation_with_no_numbering() {
        let mut r = ReduceRepl::new();
        assert_eq!(r.prompt(), "> ");
        assert_eq!(r.feed("h("), ReplResponse::NeedMore);
        assert_eq!(r.prompt(), "... ", "continuation prompt while open");
        assert!(r.is_continuing());
        assert!(matches!(r.feed("0);"), ReplResponse::Output(_)));
        assert_eq!(
            r.prompt(),
            "> ",
            "prompt returns to the plain, non-numbered form"
        );
    }

    #[test]
    fn quit_words_leave_the_session() {
        assert_eq!(ReduceRepl::new().feed("QUIT"), ReplResponse::Quit);
        assert_eq!(ReduceRepl::new().feed("quit"), ReplResponse::Quit);
        assert_eq!(ReduceRepl::new().feed("EXIT"), ReplResponse::Quit);
        assert_eq!(ReduceRepl::new().feed("exit"), ReplResponse::Quit);
    }

    #[test]
    fn an_error_is_printed_and_the_session_survives() {
        let mut r = ReduceRepl::new();
        assert!(matches!(r.feed("1 +"), ReplResponse::Output(t) if t.contains("Error")));
        assert!(matches!(r.feed("1 + 1;"), ReplResponse::Output(t) if t.contains('2')));
    }

    #[test]
    fn bindings_persist_across_lines() {
        let mut r = ReduceRepl::new();
        assert!(matches!(r.feed("x := 5;"), ReplResponse::Output(_)));
        assert!(matches!(r.feed("x + 1;"), ReplResponse::Output(t) if t.contains('6')));
    }

    #[test]
    fn an_unterminated_buffer_is_submitted_once_over_the_size_cap() {
        let mut r = ReduceRepl::new();
        let big = "(".repeat(MAX_INPUT_LEN + 16);
        match r.feed(&big) {
            ReplResponse::Output(t) => assert!(t.contains("too large"), "got {t:?}"),
            other => panic!("expected an over-size submission, got {other:?}"),
        }
        assert_eq!(r.prompt(), "> ");
    }

    #[test]
    fn run_drives_a_session_to_eof() {
        let input = b"1 + 2*3;\nQUIT\n" as &[u8];
        let mut output = Vec::new();
        run(input, &mut output).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains('7'), "expected 7 in session: {text:?}");
        assert!(text.contains("> "), "expected the plain prompt: {text:?}");
    }

    #[test]
    fn run_handles_bare_eof_without_quit() {
        let input = b"2 + 2;\n" as &[u8];
        let mut output = Vec::new();
        run(input, &mut output).unwrap();
        assert!(String::from_utf8(output).unwrap().contains('4'));
    }

    #[test]
    fn blank_lines_are_harmless() {
        let mut r = ReduceRepl::new();
        assert_eq!(r.feed(""), ReplResponse::Output(String::new()));
        assert_eq!(r.prompt(), "> ", "a blank line does not change the prompt");
    }

    #[test]
    fn a_reduce_program_evaluates_end_to_end() {
        let mut r = ReduceRepl::new();
        assert!(matches!(r.feed("h(x) := x*x;"), ReplResponse::Output(_)));
        assert!(matches!(
            r.feed("h(5);"),
            ReplResponse::Output(t) if t.contains("25")
        ));
    }

    // --- read_bounded_line: carried forward from derive-repl/j-repl/apl-repl
    //     (the /security-review fix, task #32) --------------------------

    #[test]
    fn read_bounded_line_rejects_a_line_exceeding_the_cap_without_buffering_it_whole() {
        let oversized = "+".repeat(MAX_LINE_LEN as usize * 3);
        let mut input = oversized.as_bytes();
        assert_eq!(read_bounded_line(&mut input).unwrap(), Some(Err(())));
        let _ = read_bounded_line(&mut input);
    }

    #[test]
    fn read_bounded_line_at_exactly_the_cap_with_a_trailing_newline_is_not_oversized() {
        let mut line = "+".repeat(MAX_LINE_LEN as usize - 1);
        line.push('\n');
        assert_eq!(line.len() as u64, MAX_LINE_LEN);
        let mut input = line.as_bytes();
        assert_eq!(read_bounded_line(&mut input).unwrap(), Some(Ok(line)));
    }

    #[test]
    fn read_bounded_line_treats_an_exactly_cap_sized_final_line_as_ordinary_not_oversized() {
        let line = "+".repeat(MAX_LINE_LEN as usize);
        let mut input = line.as_bytes();
        assert_eq!(
            read_bounded_line(&mut input).unwrap(),
            Some(Ok(line.clone()))
        );
        assert_eq!(read_bounded_line(&mut input).unwrap(), None);
    }

    #[test]
    fn read_bounded_line_fully_drains_a_line_spanning_multiple_cap_chunks() {
        let oversized = "+".repeat(MAX_LINE_LEN as usize * 5 / 2);
        let input = format!("{oversized}\n1+1;\n");
        let mut input = input.as_bytes();
        assert_eq!(read_bounded_line(&mut input).unwrap(), Some(Err(())));
        assert_eq!(
            read_bounded_line(&mut input).unwrap(),
            Some(Ok("1+1;\n".to_string()))
        );
    }

    #[test]
    fn read_bounded_line_handles_a_multibyte_char_straddling_the_cap_boundary() {
        let mut oversized: Vec<u8> = vec![b'a'; MAX_LINE_LEN as usize - 1];
        oversized.extend_from_slice("é".as_bytes()); // 2-byte char, straddles the cap
        oversized.push(b'\n');
        let mut input = oversized.as_slice();
        assert_eq!(read_bounded_line(&mut input).unwrap(), Some(Err(())));
    }

    #[test]
    fn run_reports_an_oversized_line_cleanly_and_keeps_the_session_alive() {
        let oversized = "+".repeat(MAX_LINE_LEN as usize * 2);
        let input = format!("{oversized}\n1+1;\nQUIT\n");
        let mut output = Vec::new();
        run(input.as_bytes(), &mut output).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(
            text.contains("exceeds"),
            "expected an oversized-line error, got: {text}"
        );
        assert!(
            text.contains('2'),
            "session must keep working after an oversized line, got: {text}"
        );
    }

    #[test]
    fn run_executes_the_correct_follow_up_statement_after_a_multi_chunk_oversized_line() {
        let oversized = "+".repeat(MAX_LINE_LEN as usize * 5 / 2);
        let input = format!("{oversized}\n1+1;\nQUIT\n");
        let mut output = Vec::new();
        run(input.as_bytes(), &mut output).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(
            text.contains("exceeds"),
            "expected an oversized-line error, got: {text}"
        );
        assert!(
            text.contains('2'),
            "the real follow-up statement (1+1), not a fragment of the \
             discarded line, must be what gets evaluated, got: {text}"
        );
    }
}
