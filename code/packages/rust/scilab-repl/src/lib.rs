//! # Scilab REPL — an interactive Read-Eval-Print loop for Scilab (a subset).
//!
//! [`ScilabRepl`] wraps a persistent [`Interpreter`] and adds the console
//! behaviours an interactive session needs: line continuation across open
//! brackets and an unterminated `if`/`select`/`while`/`for`/`function` block,
//! and echoing of unsuppressed results. Mirrors `matlab-repl`'s overall shape
//! (the same "hand-rolled, not built on the generic `repl` crate, because
//! the interpreter is single-threaded and a console session is sequential
//! anyway" reasoning) while carrying forward `maple-repl`'s own
//! `read_bounded_line` fix (see that function's own doc comment).
//!
//! ## Continuation tracking is simpler than `matlab-repl`'s
//!
//! `matlab-repl::is_incomplete` must special-case "an `end` word inside an
//! index (`A(end)`) does not open/close a block" because MATLAB's `end` is a
//! genuinely context-sensitive token (block-closer *and* last-index
//! sentinel, MA10 §1 finding 5's own citation). Scilab's classic/preferred
//! last-index token is `$` — an entirely different, always-unambiguous
//! character (never a bare word at all) — so this crate's own
//! [`is_incomplete`] needs no such exception: **every** occurrence of the
//! word `end`/`endfunction` in un-commented, non-string source is
//! unambiguously a block closer.
//!
//! One genuine addition relative to `matlab-repl`'s own scanner: Scilab's
//! `/* ... */` block comments (MA10 §1 finding 3) may span multiple
//! *physical* lines (unlike MATLAB's `%{`/`%}`, which — per `matlab-repl`'s
//! own doc comment — needs no such tracking because it is never checked for
//! at all beyond the ordinary `%`-to-end-of-line rule), so [`is_incomplete`]
//! carries an `in_block_comment` flag across lines, mirroring how it already
//! carries `in_string` across lines for an (unterminated, and therefore
//! presumably still-open) double-quoted string.

use coding_adventures_scilab_runtime::Interpreter;
use std::io::{BufRead, Write};

/// What the REPL should do after being fed one physical line.
#[derive(Debug, PartialEq, Eq)]
pub enum ReplResponse {
    /// Text to display (may be empty).
    Output(String),
    /// The current statement is incomplete; read another line.
    NeedMore,
    /// End the session.
    Quit,
}

/// A persistent interactive Scilab session.
pub struct ScilabRepl {
    interp: Interpreter,
    buffer: String,
}

impl Default for ScilabRepl {
    fn default() -> Self {
        Self::new()
    }
}

impl ScilabRepl {
    pub fn new() -> Self {
        ScilabRepl {
            interp: Interpreter::new(),
            buffer: String::new(),
        }
    }

    /// `-->` for a fresh statement (real Scilab's own console prompt), `> `
    /// while continuing an incomplete one. MA10 does not document Scilab's
    /// own continuation-prompt spelling, so `> ` is a judgment call here
    /// (flagged for a reviewer), chosen only to read clearly as "still
    /// typing" alongside the fresh-statement `-->`.
    pub fn prompt(&self) -> &'static str {
        if self.buffer.is_empty() {
            "-->"
        } else {
            "> "
        }
    }

    /// Feed one physical input line (without its trailing newline).
    pub fn feed(&mut self, line: &str) -> ReplResponse {
        if self.buffer.is_empty() {
            match line.trim() {
                "quit" | "exit" => return ReplResponse::Quit,
                _ => {}
            }
        }

        // Bound the accumulation buffer BEFORE growing it -- mirrors
        // `maple_repl::MapleRepl::feed`'s own fix (an endless run of open
        // brackets, or an `if` with no closer, must not buffer unbounded
        // memory; neither may a single caller-supplied `line` that is
        // itself already oversized).
        if self
            .buffer
            .len()
            .saturating_add(line.len())
            .saturating_add(1)
            > coding_adventures_scilab_runtime::MAX_INPUT_LEN
        {
            self.buffer.clear();
            return ReplResponse::Output(format!(
                "input too large: exceeds the {}-byte limit",
                coding_adventures_scilab_runtime::MAX_INPUT_LEN
            ));
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

/// Is the accumulated source an incomplete statement? It is incomplete while
/// a bracket is open, a `"`-string or `/* ... */` block comment is
/// unterminated, or a block keyword (`if`/`select`/`while`/`for`/`function`)
/// has no matching `end`/`endfunction`. `//` line comments are skipped to
/// end of line; `'` is not tracked at all (Scilab strings are always
/// single-line, so a `'`-string can never itself *cause* multi-line
/// continuation the way an unterminated bracket/block can — the same
/// simplification `matlab-repl::is_incomplete`'s own doc comment makes for
/// MATLAB's char arrays).
///
/// This is only a continuation *heuristic*: whatever is submitted still
/// passes through [`Interpreter::feed`], which re-parses with the real
/// lexer/parser and applies its own guards, so a mismatch can never crash —
/// at worst it submits early or asks for one more line.
fn is_incomplete(src: &str) -> bool {
    let mut bracket: i32 = 0;
    let mut blocks: i32 = 0;
    let mut in_string = false;
    let mut in_block_comment = false;

    for raw_line in src.lines() {
        let mut word = String::new();
        let flush = |w: &mut String, blocks: &mut i32| {
            if !w.is_empty() {
                match w.as_str() {
                    "if" | "select" | "while" | "for" | "function" => *blocks += 1,
                    "end" | "endfunction" => *blocks -= 1,
                    _ => {}
                }
                w.clear();
            }
        };
        let chars: Vec<char> = raw_line.chars().collect();
        let n = chars.len();
        let mut i = 0;
        while i < n {
            if in_block_comment {
                if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
                    in_block_comment = false;
                    i += 2;
                } else {
                    i += 1;
                }
                continue;
            }
            if in_string {
                if chars[i] == '"' {
                    in_string = false;
                }
                i += 1;
                continue;
            }
            match chars[i] {
                '/' if chars.get(i + 1) == Some(&'/') => break, // line comment: ignore rest of line
                '/' if chars.get(i + 1) == Some(&'*') => {
                    flush(&mut word, &mut blocks);
                    in_block_comment = true;
                    i += 2;
                    continue;
                }
                '"' => {
                    flush(&mut word, &mut blocks);
                    in_string = true;
                }
                '(' | '[' | '{' => {
                    flush(&mut word, &mut blocks);
                    bracket += 1;
                }
                ')' | ']' | '}' => {
                    flush(&mut word, &mut blocks);
                    bracket -= 1;
                }
                c if c.is_alphanumeric() || c == '_' => {
                    word.push(c);
                    i += 1;
                    continue;
                }
                _ => flush(&mut word, &mut blocks),
            }
            i += 1;
        }
        flush(&mut word, &mut blocks);
    }
    bracket > 0 || blocks > 0 || in_string || in_block_comment
}

/// Upper bound on a single *physical* line read from the input stream,
/// applied in [`read_bounded_line`] before [`ScilabRepl::feed`]'s own
/// `MAX_INPUT_LEN` check ever runs. Mirrors `maple_repl::MAX_LINE_LEN` (same
/// value, same rationale: `BufRead::read_line` has no length bound of its
/// own, so without this a single arbitrarily-long physical line is fully
/// buffered in memory before `feed` gets a chance to reject anything).
const MAX_LINE_LEN: u64 = 64 * 1024;

/// Read one physical line, bounded to [`MAX_LINE_LEN`] bytes.
///
/// - `Ok(None)` — genuine end of input.
/// - `Ok(Some(Ok(line)))` — an ordinary line (including a final,
///   newline-less line at genuine EOF).
/// - `Ok(Some(Err(())))` — a single physical line's true length exceeds the
///   cap; the oversized remainder is fully drained and discarded so the next
///   read always starts at a genuine line boundary.
///
/// Reads raw **bytes** (`read_until(b'\n', ..)`), deciding "oversized?" on
/// the byte run *before* attempting UTF-8 decoding, so a multi-byte
/// character straddling the cap boundary is never misreported as a decoding
/// failure. Mirrors `maple_repl::read_bounded_line`/`reduce_repl`/
/// `derive_repl`/`j_repl`'s identical function exactly.
fn read_bounded_line<R: BufRead>(reader: &mut R) -> std::io::Result<Option<Result<String, ()>>> {
    let mut buf: Vec<u8> = Vec::new();
    let mut limited: std::io::Take<&mut R> = std::io::Read::take(&mut *reader, MAX_LINE_LEN);
    let read = limited.read_until(b'\n', &mut buf)?;
    if read == 0 {
        return Ok(None);
    }
    let hit_cap = buf.len() as u64 == MAX_LINE_LEN && buf.last() != Some(&b'\n');
    if hit_cap {
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
                break;
            }
            saw_more_data = true;
            if chunk.last() == Some(&b'\n') {
                break;
            }
        }
        return Ok(Some(Err(())));
    }
    match String::from_utf8(buf) {
        Ok(line) => Ok(Some(Ok(line))),
        Err(e) => Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
    }
}

/// Drive a full interactive Scilab session over the given reader and writer.
pub fn run<R: BufRead, W: Write>(mut reader: R, mut writer: W) -> std::io::Result<()> {
    let mut repl = ScilabRepl::new();
    writeln!(
        writer,
        "Scilab (on array-runtime) — type quit or exit to leave."
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
    fn echoes_a_result_and_suppresses_with_semicolon() {
        let mut r = ScilabRepl::new();
        assert!(matches!(r.feed("2 + 2"), ReplResponse::Output(t) if t.contains('4')));
        assert_eq!(r.feed("x = 5;"), ReplResponse::Output(String::new()));
    }

    #[test]
    fn continues_across_open_brackets() {
        let mut r = ScilabRepl::new();
        assert_eq!(r.feed("A = [1 2"), ReplResponse::NeedMore);
        assert!(r.is_continuing());
        assert!(matches!(r.feed("3 4];"), ReplResponse::Output(_)));
        assert!(!r.is_continuing());
    }

    #[test]
    fn continues_across_an_unterminated_while_block() {
        let mut r = ScilabRepl::new();
        assert_eq!(r.feed("s = 0;"), ReplResponse::Output(String::new()));
        assert_eq!(r.feed("i = 0;"), ReplResponse::Output(String::new()));
        assert_eq!(r.feed("while i < 3"), ReplResponse::NeedMore);
        assert_eq!(r.feed("  s = s + i;"), ReplResponse::NeedMore);
        assert_eq!(r.feed("  i = i + 1;"), ReplResponse::NeedMore);
        assert!(matches!(r.feed("end"), ReplResponse::Output(_)));
        assert!(matches!(r.feed("s"), ReplResponse::Output(t) if t.contains('3')));
    }

    #[test]
    fn continues_across_an_unterminated_function_definition() {
        let mut r = ScilabRepl::new();
        assert_eq!(r.feed("function y = f(x)"), ReplResponse::NeedMore);
        assert_eq!(r.feed("  y = x * 2;"), ReplResponse::NeedMore);
        assert!(matches!(r.feed("endfunction"), ReplResponse::Output(_)));
        assert!(matches!(r.feed("f(3)"), ReplResponse::Output(t) if t.contains('6')));
    }

    #[test]
    fn a_single_line_if_needs_no_continuation() {
        let mut r = ScilabRepl::new();
        assert!(matches!(
            r.feed("if 1 then y = 1, end"),
            ReplResponse::Output(_)
        ));
    }

    #[test]
    fn quit_and_exit_commands() {
        assert_eq!(ScilabRepl::new().feed("quit"), ReplResponse::Quit);
        assert_eq!(ScilabRepl::new().feed("exit"), ReplResponse::Quit);
    }

    #[test]
    fn errors_are_shown_not_fatal() {
        let mut r = ScilabRepl::new();
        assert!(matches!(r.feed("undefined_var"), ReplResponse::Output(t) if t.contains("Error")));
        assert!(matches!(r.feed("1 + 1"), ReplResponse::Output(t) if t.contains('2')));
    }

    #[test]
    fn prompts_switch_between_fresh_and_continuation() {
        let mut r = ScilabRepl::new();
        assert_eq!(r.prompt(), "-->");
        assert_eq!(r.feed("A = [1"), ReplResponse::NeedMore);
        assert_eq!(r.prompt(), "> ");
        assert!(matches!(r.feed("2];"), ReplResponse::Output(_)));
        assert_eq!(r.prompt(), "-->");
    }

    #[test]
    fn run_drives_a_session_to_eof() {
        let input = b"x = 3;\nx * 2\nquit\n" as &[u8];
        let mut output = Vec::new();
        run(input, &mut output).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains('6'));
    }

    #[test]
    fn run_handles_bare_eof_without_quit() {
        let input = b"2 + 2\n" as &[u8];
        let mut output = Vec::new();
        run(input, &mut output).unwrap();
        assert!(String::from_utf8(output).unwrap().contains('4'));
    }

    #[test]
    fn block_comment_spanning_multiple_lines_does_not_perturb_tracking() {
        let mut r = ScilabRepl::new();
        assert_eq!(r.feed("x = 1 /* a comment"), ReplResponse::NeedMore);
        assert_eq!(r.feed("   spanning lines, with if/end words inside"), ReplResponse::NeedMore);
        assert!(matches!(r.feed("*/ + 2"), ReplResponse::Output(t) if t.contains('3')));
    }

    // --- read_bounded_line: carried forward from maple-repl -----------------

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
    fn run_reports_an_oversized_line_cleanly_and_keeps_the_session_alive() {
        let oversized = "+".repeat(MAX_LINE_LEN as usize * 2);
        let input = format!("{oversized}\n1+1\nquit\n");
        let mut output = Vec::new();
        run(input.as_bytes(), &mut output).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("exceeds"), "got: {text}");
        assert!(text.contains('2'), "got: {text}");
    }

    #[test]
    fn an_unterminated_buffer_is_submitted_once_over_the_size_cap() {
        let mut r = ScilabRepl::new();
        let big = "(".repeat(coding_adventures_scilab_runtime::MAX_INPUT_LEN + 16);
        match r.feed(&big) {
            ReplResponse::Output(t) => assert!(t.contains("too large"), "got {t:?}"),
            other => panic!("expected an over-size submission, got {other:?}"),
        }
        assert_eq!(r.prompt(), "-->");
    }
}
