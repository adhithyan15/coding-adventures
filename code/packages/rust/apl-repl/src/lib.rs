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

/// Upper bound on the pending-continuation buffer (while a `(` is still
/// unbalanced). Without this, a source that never closes its parens grows
/// `buffer` without bound before anything is ever parsed — low severity for
/// a human typing at a terminal, but a real memory-exhaustion vector if this
/// REPL is ever driven by a network-facing or otherwise less-trusted line
/// source. 64 KiB is far more than any legitimate hand-written APL statement
/// needs, mirroring `wolfram-runtime::MAX_INPUT_LEN`'s "generous but bounded"
/// convention for a single logical unit of input.
const MAX_CONTINUATION_BUFFER: usize = 64 * 1024;

/// Upper bound on a single *physical* line read from the input stream,
/// applied in [`read_bounded_line`] before [`MAX_CONTINUATION_BUFFER`]'s own
/// check ever runs. `BufRead::read_line` has no length bound of its own — it
/// grows its buffer until it sees a `\n` or EOF — so without this, a single,
/// arbitrarily long physical line (no embedded newline at all) is fully
/// buffered in memory before `AplRepl::feed` ever gets a chance to reject
/// anything, regardless of `MAX_CONTINUATION_BUFFER`. Same bound, same
/// rationale, as that constant — the two together mean neither one physical
/// line nor an accumulated multi-line continuation can grow unbounded.
const MAX_LINE_LEN: u64 = 64 * 1024;

/// Read one physical line, bounded to [`MAX_LINE_LEN`] bytes.
///
/// - `Ok(None)` — genuine end of input.
/// - `Ok(Some(Ok(line)))` — an ordinary line. Includes a final,
///   newline-less line at genuine EOF (same as [`BufRead::read_line`]'s own
///   contract — an unterminated last line is not "oversized", it's just
///   short of a full cap's worth of bytes).
/// - `Ok(Some(Err(())))` — a single physical line reached the cap before
///   any `\n` appeared. The oversized remainder of that same line is
///   drained and discarded (up to one more [`MAX_LINE_LEN`]-byte chunk,
///   itself bounded for the same reason) so a well-behaved caller's next
///   read starts at (or very near) a genuine line boundary instead of
///   picking up mid-line.
///
/// Reads raw **bytes** (`read_until(b'\n', ..)`), not [`BufRead::read_line`]
/// directly, and only attempts UTF-8 decoding *after* confirming whether the
/// byte run stopped because of a real `\n` or because it genuinely
/// exhausted the cap. `read_line` validates UTF-8 over whatever byte run it
/// stops at — and `Take` stops at an arbitrary **byte** offset with no
/// notion of a character boundary, so an ordinary, fully valid UTF-8 line
/// whose true length only slightly exceeds [`MAX_LINE_LEN`] can have a
/// multi-byte character straddle that exact offset, making `read_line` alone
/// report a spurious `InvalidData` I/O error (fatal — it aborts the whole
/// session, see `main.rs`) for input that isn't actually malformed, just
/// long. Deciding "oversized?" on bytes first means a cap that happens to
/// split a character is correctly treated as just another oversized line,
/// not a decoding failure.
///
/// Oversized is judged by **both** "no trailing `\n`" *and* "the byte count
/// actually hit the cap" — not `\n`-absence alone. `read_until` also stops
/// (with no trailing `\n`) at genuine EOF, e.g. a final line with no closing
/// newline; checking length too keeps that ordinary, short, valid case from
/// being misclassified as oversized and silently dropped.
fn read_bounded_line<R: BufRead>(reader: &mut R) -> std::io::Result<Option<Result<String, ()>>> {
    let mut buf: Vec<u8> = Vec::new();
    // Explicit UFCS + an explicit reborrow (`&mut *reader`), not plain
    // `reader.take(...)` method-call syntax: `Read`/`BufRead` are
    // implemented both for `R` itself and for `&mut R`, and ordinary
    // autoref method resolution prefers the *by-value* `R` candidate over
    // the reference one even when the receiver expression is already a
    // reference -- silently trying to MOVE `*reader` (illegal, since we
    // only borrow it) instead of taking a bounded view of it. Naming the
    // trait and the exact `&mut R` receiver explicitly removes that
    // ambiguity.
    let mut limited: std::io::Take<&mut R> = std::io::Read::take(&mut *reader, MAX_LINE_LEN);
    let read = limited.read_until(b'\n', &mut buf)?;
    if read == 0 {
        return Ok(None);
    }
    let hit_cap = buf.len() as u64 == MAX_LINE_LEN && buf.last() != Some(&b'\n');
    if hit_cap {
        // The cap was genuinely exhausted with no newline in sight -- discard
        // the rest of this same oversized line (one more bounded chunk; if
        // the line somehow exceeds even that, the next call will simply
        // report another oversized chunk rather than hang or grow memory
        // further, which is a safe, if repetitive, degradation for a threat
        // model this crate does not otherwise need to defend against today).
        // This path is taken purely on byte length -- never on whether `buf`
        // happens to be valid UTF-8 -- so a cap that splits a multi-byte
        // character lands here too, not in the `from_utf8` error arm below.
        let mut discard: Vec<u8> = Vec::new();
        let mut limited: std::io::Take<&mut R> = std::io::Read::take(&mut *reader, MAX_LINE_LEN);
        // Propagated with `?`, not swallowed: a genuine I/O error draining
        // the remainder (e.g. a broken pipe) is a real failure, not just
        // "no more oversized bytes to discard" -- it shouldn't be misreported
        // as an ordinary oversized-line event.
        limited.read_until(b'\n', &mut discard)?;
        return Ok(Some(Err(())));
    }
    // The byte run genuinely ended at a real `\n` within the cap, so any
    // UTF-8 error here is real malformed input, not a truncation artifact --
    // propagated as an I/O error, same as `BufRead::read_line` would have
    // done for this same (uncapped) byte run.
    match String::from_utf8(buf) {
        Ok(line) => Ok(Some(Ok(line))),
        Err(e) => Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
    }
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

        if self.buffer.len() > MAX_CONTINUATION_BUFFER {
            self.buffer.clear();
            return ReplResponse::Output(format!(
                "Error: statement exceeds the {MAX_CONTINUATION_BUFFER}-byte continuation limit; discarded\n"
            ));
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
    fn an_unbounded_continuation_is_discarded_not_grown_forever() {
        let mut r = AplRepl::new();
        assert_eq!(r.feed("(1"), ReplResponse::NeedMore);
        // Keep feeding lines that never close the paren, well past the cap.
        let filler = "+1".repeat(MAX_CONTINUATION_BUFFER / 2 + 10);
        match r.feed(&filler) {
            ReplResponse::Output(t) => assert!(t.contains("Error")),
            other => panic!("expected an Error output once the cap is exceeded, got {other:?}"),
        }
        // The buffer was discarded, not left growing -- a fresh statement works.
        assert!(!r.is_continuing());
        assert!(matches!(r.feed("1+1"), ReplResponse::Output(t) if t.contains('2')));
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

    #[test]
    fn read_bounded_line_returns_a_final_line_without_a_trailing_newline() {
        // A genuine EOF before the cap with no trailing '\n' at all (e.g.
        // `printf '%s' quit` piped in, or a file with no final newline) is
        // an ordinary short line, not an oversized one -- the byte run
        // stopped at EOF, not because the cap was exhausted.
        let mut input = "quit".as_bytes();
        assert_eq!(
            read_bounded_line(&mut input).unwrap(),
            Some(Ok("quit".to_string()))
        );
        assert_eq!(read_bounded_line(&mut input).unwrap(), None);
    }

    #[test]
    fn run_quits_cleanly_on_a_final_line_without_a_trailing_newline() {
        let input = "quit".as_bytes(); // no trailing '\n' anywhere in the input
        let mut output = Vec::new();
        run(input, &mut output).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(
            !text.contains("exceeds"),
            "a short, unterminated final line must not be treated as oversized, got: {text}"
        );
    }

    #[test]
    fn read_bounded_line_returns_an_ordinary_line_unchanged() {
        let mut input = "A←5\nA×2\n".as_bytes();
        assert_eq!(
            read_bounded_line(&mut input).unwrap(),
            Some(Ok("A←5\n".to_string()))
        );
        assert_eq!(
            read_bounded_line(&mut input).unwrap(),
            Some(Ok("A×2\n".to_string()))
        );
        assert_eq!(read_bounded_line(&mut input).unwrap(), None);
    }

    #[test]
    fn read_bounded_line_rejects_a_line_exceeding_the_cap_without_buffering_it_whole() {
        // A single physical line with no embedded newline at all, well past
        // MAX_LINE_LEN -- exactly the shape `BufRead::read_line` alone would
        // buffer in full before anything else got a chance to reject it.
        let oversized = "+".repeat(MAX_LINE_LEN as usize * 3);
        let mut input = oversized.as_bytes();
        assert_eq!(read_bounded_line(&mut input).unwrap(), Some(Err(())));
        // The next read should reach EOF (the whole oversized "line" -- one
        // capped read plus one capped discard chunk -- was consumed, not
        // left half-read for a following call to pick up mid-garbage).
        // This particular input is 3x the cap, so a second oversized chunk
        // may still remain; either an Err or eventual None is acceptable --
        // what matters is the first call didn't attempt to allocate the
        // whole 3x-cap string at once, which the cap itself already proves.
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
    fn read_bounded_line_handles_a_multibyte_char_straddling_the_cap_boundary() {
        // A line whose true length only slightly exceeds MAX_LINE_LEN, with
        // a 2-byte UTF-8 character ('é') positioned so the cap lands right
        // in the middle of it. Before the byte-oriented rewrite, this made
        // `read_line` (validating whatever partial byte run `Take` handed
        // it) return a spurious `InvalidData` I/O error -- fatal, per
        // `run`'s `?` propagation -- for a line that is not malformed, just
        // long. It must now be treated exactly like any other oversized
        // line: `Some(Err(()))`, not a hard I/O error.
        let mut oversized: Vec<u8> = vec![b'a'; MAX_LINE_LEN as usize - 1];
        oversized.extend_from_slice("é".as_bytes()); // 2-byte char, straddles the cap
        oversized.push(b'\n');
        let mut input = oversized.as_slice();
        assert_eq!(read_bounded_line(&mut input).unwrap(), Some(Err(())));
    }

    #[test]
    fn run_survives_a_multibyte_char_straddling_the_cap_boundary() {
        let mut oversized: Vec<u8> = vec![b'a'; MAX_LINE_LEN as usize - 1];
        oversized.extend_from_slice("é".as_bytes());
        let mut input = oversized;
        input.extend_from_slice(b"\n1+1\nquit\n");
        let mut output = Vec::new();
        run(input.as_slice(), &mut output).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("exceeds"), "expected an oversized-line error, got: {text}");
        assert!(
            text.contains('2'),
            "session must keep working after a boundary-splitting oversized line, got: {text}"
        );
    }

    #[test]
    fn run_reports_an_oversized_line_cleanly_and_keeps_the_session_alive() {
        let oversized = "+".repeat(MAX_LINE_LEN as usize * 2);
        let input = format!("{oversized}\n1+1\nquit\n");
        let mut output = Vec::new();
        run(input.as_bytes(), &mut output).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("exceeds"), "expected an oversized-line error, got: {text}");
        assert!(
            text.contains('2'),
            "session must keep working after an oversized line, got: {text}"
        );
    }
}
