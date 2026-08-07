//! # Axiom REPL — an interactive Read-Eval-Print loop for Axiom (a subset).
//!
//! [`AxiomRepl`] wraps a persistent [`AxiomSession`] (which reuses the
//! entire shared symbolic stack, plus this crate's own fixed domain/category
//! layer, MA13 §2/§3) and adds the console behaviours an interactive Axiom
//! session expects:
//!
//! * **`(n) -> ` prompts** — mirroring real Axiom's own numbered interactive
//!   prompt (MA13 §5, confirmed directly against the book: `(1) ->`,
//!   incrementing per computation step) — the closest match among this
//!   repo's existing CAS REPLs (`derive-repl`'s own `#n:` numbered-worksheet
//!   convention is the structural precedent this crate follows, adapted to
//!   Axiom's own confirmed prompt spelling).
//! * **Line continuation** — a statement is read across physical lines until
//!   `(`/`[` bracket depth returns to zero AND no string literal is left
//!   open, tracked by a small state machine that skips over `--` line
//!   comments and `"..."` string contents so a stray bracket character
//!   *inside* one of those never falsely extends (or ends) continuation
//!   (see [`is_incomplete`]'s own doc comment — Axiom, unlike Derive, has
//!   both comments and strings in its lexical surface this cut, so this
//!   heuristic does slightly more work than `derive-repl`'s identical
//!   bracket-only version). This is only a heuristic — whatever is
//!   submitted still passes through [`AxiomSession::feed`], which re-lexes
//!   with the real lexer and applies the size/complexity guards, so a
//!   mismatch can never crash, at worst it submits early or asks for one
//!   more line.
//! * **Quit / EOF** — `)quit`/`quit` (case-insensitive convenience: also
//!   `QUIT`/`Quit`) or Ctrl-D end the session. Real Axiom's own session
//!   commands are `)`-prefixed (confirmed directly, MA13 §4's own deferred
//!   list: "`)`-prefixed system commands... not part of this grammar"); this
//!   REPL recognises `)quit` as a console-layer convenience without adding
//!   any of the rest of that `)`-prefixed command surface, which MA13 §4
//!   explicitly defers as session/tooling surface, not language surface.
//! * **Non-fatal errors** — a surface error prints and the session continues.
//!
//! ## Two known bug classes, checked and fixed from day one
//!
//! This crate is a fresh REPL, but two bug classes have already been found
//! and fixed in sibling REPLs in this repo's history, and are deliberately
//! avoided here from the start rather than risked and re-discovered:
//!
//! 1. **Push-before-size-check ordering** (fixed in `reduce-repl`/
//!    `derive-repl`/`apl-repl`/`j-repl`): [`AxiomRepl::feed`] checks
//!    `self.buffer`'s prospective size *before* ever calling
//!    `self.buffer.push_str`, mirroring `derive-repl::DeriveRepl::feed`'s
//!    identical fix — checking only *after* growing the buffer would let an
//!    arbitrarily large single `line` be copied in (an O(n) copy, possibly
//!    reallocating) before the check meant to bound it ever ran.
//! 2. **Unbounded single-physical-line read before the continuation-buffer
//!    check** (fixed in `j-repl`/`apl-repl`): [`run`] reads each physical
//!    line through [`read_bounded_line`] (capped at [`MAX_LINE_LEN`]) rather
//!    than `BufRead::read_line` directly, which has no length bound of its
//!    own and would fully buffer a single, arbitrarily long line with no
//!    embedded newline before [`AxiomRepl::feed`]'s own size check ever got
//!    a chance to reject anything.

use coding_adventures_axiom_runtime::{AxiomSession, MAX_INPUT_LEN};
use std::io::{BufRead, Write};

/// What the REPL should do after being fed one physical line.
#[derive(Debug, PartialEq, Eq)]
pub enum ReplResponse {
    /// A complete statement was evaluated; here is its echo.
    Output(String),
    /// The buffer is not yet a complete statement — read another line.
    NeedMore,
    /// The user asked to leave.
    Quit,
}

/// A persistent interactive Axiom session.
pub struct AxiomRepl {
    session: AxiomSession,
    buffer: String,
    /// The 1-based index shown in the `(n) ->` prompt. Advances once per
    /// *submitted* statement, mirroring real Axiom's own numbered prompt.
    input_index: usize,
    /// Running bracket depth (`(`/`[` vs `)`/`]`) of `buffer`, updated
    /// *incrementally* by [`scan_line`] as each new physical line is fed,
    /// rather than recomputed by rescanning the whole buffer on every call
    /// -- see [`scan_line`]'s own doc comment for why a full-buffer rescan
    /// per line is a real, if bounded, algorithmic-complexity concern this
    /// avoids.
    bracket_depth: i32,
    /// Whether `buffer` currently ends inside an unterminated `"..."` string
    /// literal (which, per `axiom.tokens`, may itself span physical lines).
    in_string: bool,
}

impl Default for AxiomRepl {
    fn default() -> Self {
        Self::new()
    }
}

impl AxiomRepl {
    pub fn new() -> Self {
        AxiomRepl {
            session: AxiomSession::new(),
            buffer: String::new(),
            input_index: 1,
            bracket_depth: 0,
            in_string: false,
        }
    }

    /// The prompt to print before reading the next physical line.
    pub fn prompt(&self) -> String {
        if self.buffer.is_empty() {
            format!("({}) -> ", self.input_index)
        } else {
            "   -> ".to_string()
        }
    }

    /// Feed one physical input line (without its trailing newline).
    pub fn feed(&mut self, line: &str) -> ReplResponse {
        // Quit words are only recognised at the start of a fresh statement,
        // so an identifier merely named `quit` mid-expression is never
        // hijacked.
        if self.buffer.is_empty() {
            match line.trim() {
                ")quit" | "quit" | "QUIT" | "Quit" | ")QUIT" => return ReplResponse::Quit,
                _ => {}
            }
        }

        // Bound the accumulation buffer BEFORE growing it -- the
        // push-before-size-check ordering bug fixed in
        // reduce-repl/derive-repl/apl-repl/j-repl (see the module doc
        // comment's point 1): checking only after `push_str` would let an
        // arbitrarily large single `line` be copied into `self.buffer` (an
        // O(n) copy, possibly reallocating) before this check ever ran.
        if self
            .buffer
            .len()
            .saturating_add(line.len())
            .saturating_add(1)
            > MAX_INPUT_LEN
        {
            self.buffer.clear();
            self.bracket_depth = 0;
            self.in_string = false;
            self.input_index += 1;
            return ReplResponse::Output(format!(
                "input too large: exceeds the {MAX_INPUT_LEN}-byte limit"
            ));
        }

        self.buffer.push_str(line);
        self.buffer.push('\n');
        scan_line(line, &mut self.bracket_depth, &mut self.in_string);

        if self.bracket_depth > 0 || self.in_string {
            return ReplResponse::NeedMore;
        }

        let src = std::mem::take(&mut self.buffer);
        self.bracket_depth = 0;
        self.in_string = false;
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

/// Upper bound on a single *physical* line read from the input stream,
/// applied in [`read_bounded_line`] before [`AxiomRepl::feed`]'s own
/// `MAX_INPUT_LEN` check ever runs. `BufRead::read_line` has no length
/// bound of its own -- it grows its buffer until it sees a `\n` or EOF --
/// so without this, a single, arbitrarily long physical line (no embedded
/// newline at all) is fully buffered in memory before `feed` ever gets a
/// chance to reject anything. Mirrors `j-repl`/`apl-repl`/`derive-repl`'s
/// own `MAX_LINE_LEN` fix exactly (same value, same rationale).
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
/// decoding -- `Take` stops at an arbitrary byte offset with no notion of a
/// character boundary, so a valid UTF-8 line whose true length only slightly
/// exceeds the cap could have a multi-byte character straddle that exact
/// offset, and validating on the byte run through `read_line` alone would
/// misreport that as a decoding failure rather than an oversized line.
/// Mirrors `derive-repl::read_bounded_line`/`j-repl::read_bounded_line`/
/// `apl-repl::read_bounded_line` exactly.
fn read_bounded_line<R: BufRead>(reader: &mut R) -> std::io::Result<Option<Result<String, ()>>> {
    let mut buf: Vec<u8> = Vec::new();
    // Explicit UFCS + an explicit reborrow (`&mut *reader`): `Read`/`BufRead`
    // are implemented both for `R` itself and for `&mut R`, and ordinary
    // autoref method resolution prefers the by-value `R` candidate even when
    // the receiver is already a reference -- silently trying to MOVE
    // `*reader` instead of taking a bounded view of it. Naming the trait and
    // the exact `&mut R` receiver explicitly removes that ambiguity.
    let mut limited: std::io::Take<&mut R> = std::io::Read::take(&mut *reader, MAX_LINE_LEN);
    let read = limited.read_until(b'\n', &mut buf)?;
    if read == 0 {
        return Ok(None);
    }
    let hit_cap = buf.len() as u64 == MAX_LINE_LEN && buf.last() != Some(&b'\n');
    if hit_cap {
        // The initial capped read filled up with no newline in sight -- keep
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

/// Scan one freshly-fed physical `line` (no embedded newline), updating
/// `depth` (bracket nesting) and `in_string` (open-string state) to reflect
/// having consumed it -- the incremental counterpart of a "rescan the whole
/// buffer" check.
///
/// Axiom's `(`/`[` grouping, blocks, and calls (round parens) and list
/// literals (square brackets) are complete once bracket depth returns to
/// zero -- but unlike `derive-repl`'s identical heuristic, Axiom's lexical
/// surface this cut also has `--` line comments and `"..."` string literals
/// (`axiom.tokens` SECTION 3/4), and a stray `(`/`[`/`"` *inside* either of
/// those must never be counted -- `x := "a (unbalanced paren"` is already a
/// complete statement, and a `--` comment's content is arbitrary text that
/// is never re-lexed as code. This scan tracks "currently inside a string"
/// (which may itself span physical lines, so `in_string` is carried in by
/// the caller rather than reset per line) and stops at a `--`-opened
/// comment (comments never span lines, so there is nothing left worth
/// scanning in `line` once one starts) before ever looking at a character
/// for bracket-depth purposes, so those two constructs cannot falsely
/// extend (or end) continuation.
///
/// **Why incremental, not a full-buffer rescan per line:** an earlier
/// version of this function took the *entire accumulated buffer* and
/// recomputed `depth`/`in_string` from scratch on every physical line fed
/// to [`AxiomRepl::feed`]. For a single statement spanning N physical
/// lines, that is `1 + 2 + ... + N` = O(N²) total character-visits before
/// the statement completes -- bounded (since `AxiomRepl::feed`'s own
/// `MAX_INPUT_LEN` check caps the buffer at 64 KiB), but real, wasted,
/// attacker-influenceable CPU work for something that is O(N) total when
/// each line only re-scans *itself*. This is only a heuristic either way:
/// whatever is submitted still passes through `AxiomSession::feed`, which
/// re-lexes with the real `axiom-lexer` and applies the real
/// size/complexity guards, so a mismatch here can never crash -- at worst
/// it submits early or asks for one more line.
fn scan_line(line: &str, depth: &mut i32, in_string: &mut bool) {
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if *in_string {
            if ch == '"' {
                *in_string = false;
            }
            continue;
        }
        match ch {
            '"' => *in_string = true,
            '-' if chars.peek() == Some(&'-') => {
                // A `--` line comment: the rest of THIS line is its content
                // -- nothing more to scan (comments never span lines).
                return;
            }
            '(' | '[' => *depth += 1,
            ')' | ']' => *depth -= 1,
            _ => {}
        }
    }
}

/// Drive a full interactive Axiom session over the given reader and writer.
pub fn run<R: BufRead, W: Write>(mut reader: R, mut writer: W) -> std::io::Result<()> {
    let mut repl = AxiomRepl::new();
    writeln!(
        writer,
        "Axiom (on the shared symbolic stack) -- one statement per line, type )quit to exit."
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
        let mut r = AxiomRepl::new();
        assert!(matches!(r.feed("1 + 2*3"), ReplResponse::Output(t) if t.contains('7')));
    }

    #[test]
    fn continues_while_a_paren_is_open() {
        let mut r = AxiomRepl::new();
        assert_eq!(r.feed("f(1,"), ReplResponse::NeedMore);
        assert!(matches!(r.feed("2)"), ReplResponse::Output(_)));
    }

    #[test]
    fn continues_while_a_bracket_is_open() {
        let mut r = AxiomRepl::new();
        assert_eq!(r.feed("[1,"), ReplResponse::NeedMore);
        assert!(matches!(
            r.feed("2, 3]"),
            ReplResponse::Output(t) if t.contains("[1, 2, 3]")
        ));
    }

    #[test]
    fn continues_across_a_multi_line_block() {
        let mut r = AxiomRepl::new();
        assert_eq!(r.feed("(a := 1;"), ReplResponse::NeedMore);
        assert!(matches!(r.feed("a + 1)"), ReplResponse::Output(t) if t.contains('2')));
    }

    #[test]
    fn a_paren_inside_a_string_does_not_trigger_continuation() {
        let mut r = AxiomRepl::new();
        assert!(matches!(
            r.feed("\"a (unbalanced paren\""),
            ReplResponse::Output(_)
        ));
    }

    #[test]
    fn a_paren_inside_a_comment_does_not_trigger_continuation() {
        let mut r = AxiomRepl::new();
        assert!(matches!(
            r.feed("1 + 1 -- a comment with an unbalanced ( paren"),
            ReplResponse::Output(t) if t.contains('2')
        ));
    }

    #[test]
    fn an_unterminated_string_still_asks_for_more() {
        let mut r = AxiomRepl::new();
        assert_eq!(r.feed("\"unterminated"), ReplResponse::NeedMore);
    }

    #[test]
    fn a_string_open_across_several_physical_lines_carries_state_correctly() {
        // Regression test for the incremental `scan_line` refactor (fixed a
        // security-review finding: an earlier full-buffer rescan per line
        // was O(n^2) worst case): `in_string` must carry over correctly
        // across MULTIPLE separately-fed lines, not just within one.
        let mut r = AxiomRepl::new();
        assert_eq!(r.feed("x := \"line one"), ReplResponse::NeedMore);
        assert_eq!(r.feed("line two ( not a real paren"), ReplResponse::NeedMore);
        assert!(matches!(r.feed("line three\""), ReplResponse::Output(_)));
    }

    #[test]
    fn bracket_depth_carries_correctly_across_a_comment_on_an_intermediate_line() {
        // A `(` opened on one line, an intervening line whose OWN `--`
        // comment contains an unbalanced `)`, then the real closing `)` on
        // a third line -- confirms the incremental scan's per-line comment
        // handling doesn't corrupt the carried-over bracket depth.
        let mut r = AxiomRepl::new();
        assert_eq!(r.feed("f(1,"), ReplResponse::NeedMore);
        assert_eq!(
            r.feed("-- a comment with a stray ) in it"),
            ReplResponse::NeedMore
        );
        assert!(matches!(r.feed("2)"), ReplResponse::Output(_)));
    }

    #[test]
    fn prompts_switch_between_input_and_continuation() {
        let mut r = AxiomRepl::new();
        assert_eq!(r.prompt(), "(1) -> ");
        assert_eq!(r.feed("f("), ReplResponse::NeedMore);
        assert_eq!(r.prompt(), "   -> ", "continuation prompt while open");
        assert!(r.is_continuing());
        assert!(matches!(r.feed("0)"), ReplResponse::Output(_)));
        assert_eq!(r.prompt(), "(2) -> ", "input index advanced");
    }

    #[test]
    fn quit_words_leave_the_session() {
        assert_eq!(AxiomRepl::new().feed(")quit"), ReplResponse::Quit);
        assert_eq!(AxiomRepl::new().feed("quit"), ReplResponse::Quit);
        assert_eq!(AxiomRepl::new().feed("QUIT"), ReplResponse::Quit);
    }

    #[test]
    fn an_error_is_printed_and_the_session_survives() {
        let mut r = AxiomRepl::new();
        assert!(matches!(r.feed("1 +"), ReplResponse::Output(t) if t.contains("Error")));
        assert!(matches!(r.feed("1 + 1"), ReplResponse::Output(t) if t.contains('2')));
    }

    #[test]
    fn bindings_persist_across_lines() {
        let mut r = AxiomRepl::new();
        assert!(matches!(r.feed("x := 5"), ReplResponse::Output(_)));
        assert!(matches!(r.feed("x + 1"), ReplResponse::Output(t) if t.contains('6')));
    }

    #[test]
    fn declared_domain_persists_and_is_enforced_across_lines() {
        let mut r = AxiomRepl::new();
        assert!(matches!(r.feed("a : PositiveInteger"), ReplResponse::Output(_)));
        assert!(matches!(
            r.feed("a := -1"),
            ReplResponse::Output(t) if t.contains("Cannot convert")
        ));
    }

    #[test]
    fn an_unterminated_buffer_is_submitted_once_over_the_size_cap() {
        let mut r = AxiomRepl::new();
        let big = "(".repeat(MAX_INPUT_LEN + 16);
        match r.feed(&big) {
            ReplResponse::Output(t) => assert!(t.contains("too large"), "got {t:?}"),
            other => panic!("expected an over-size submission, got {other:?}"),
        }
        assert_eq!(r.prompt(), "(2) -> ");
    }

    #[test]
    fn run_drives_a_session_to_eof() {
        let input = b"1 + 2*3\n)quit\n" as &[u8];
        let mut output = Vec::new();
        run(input, &mut output).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains('7'), "expected 7 in session: {text:?}");
        assert!(text.contains("(1)"), "expected the input prompt: {text:?}");
    }

    #[test]
    fn run_handles_bare_eof_without_quit() {
        let input = b"2 + 2\n" as &[u8];
        let mut output = Vec::new();
        run(input, &mut output).unwrap();
        assert!(String::from_utf8(output).unwrap().contains('4'));
    }

    #[test]
    fn blank_lines_are_harmless() {
        let mut r = AxiomRepl::new();
        assert_eq!(r.feed(""), ReplResponse::Output(String::new()));
        assert_eq!(r.prompt(), "(1) -> ", "a blank line does not advance the counter");
    }

    #[test]
    fn an_axiom_session_evaluates_end_to_end() {
        let mut r = AxiomRepl::new();
        assert!(matches!(
            r.feed("power(x: Integer, n: NonNegativeInteger): Integer == x ** n"),
            ReplResponse::Output(_)
        ));
        assert!(matches!(
            r.feed("power(2, 8)"),
            ReplResponse::Output(t) if t.contains("256")
        ));
    }

    // --- read_bounded_line: the two known bug classes ------------------

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
        let input = format!("{oversized}\n1+1\n");
        let mut input = input.as_bytes();
        assert_eq!(read_bounded_line(&mut input).unwrap(), Some(Err(())));
        assert_eq!(
            read_bounded_line(&mut input).unwrap(),
            Some(Ok("1+1\n".to_string()))
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
        let input = format!("{oversized}\n1+1\n)quit\n");
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
        let input = format!("{oversized}\n1+1\n)quit\n");
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
