//! # Q REPL — an interactive Read-Eval-Print loop for Q.
//!
//! [`QRepl`] wraps a persistent [`Interpreter`] and adds the interactive
//! behaviours a console needs: line continuation across an open bracket,
//! and echoing of auto-printed results. It is the sibling of `j-repl`/
//! `apl-repl`/`matlab-repl`/`s-repl`/`r-repl`; only the interpreter (and
//! the continuation scanner) differ. See
//! `code/specs/MA11-q-language.md`.
//!
//! ## Why this scanner needs brace/bracket balance, not just paren balance
//!
//! Read `j-repl`'s own module doc comment (`code/packages/rust/j-repl/src/lib.rs`)
//! first: it explains that J's/APL's continuation scanner reduces to plain
//! `(`-balance tracking because those languages (this repo's own cuts of
//! them) have no control flow, no user-defined blocks, and no string/char
//! literal type — the *only* grouping construct is parenthesised, so an
//! unbalanced `(` is the *only* way a statement can still be "in progress".
//!
//! Q is genuinely different: it has a real user-defined block construct
//! now — the function literal `{[x;y] stmt; stmt}` (MA11 §2/§3 bullet 1) —
//! which can legitimately span multiple physical lines in an interactive
//! session (a user typing a multi-statement lambda body one line at a
//! time, e.g. `{[x;y]` on one line and ` x+y}` on the next). So this
//! scanner needs **brace** balance (`{`/`}`) in addition to **paren**
//! balance (`(`/`)`, needed here for exactly the same reason it is in
//! J/APL: ordinary grouping and the explicit list-literal form both use
//! parens) — and, since a function literal's optional parameter list is
//! itself bracketed (`{[x;y] ...}`), **bracket** balance (`[`/`]`) too,
//! even though splitting a parameter list across a line break would be
//! unusual style. This module does **not** just copy `j-repl`'s scanner
//! verbatim and assume it is sufficient — see [`is_incomplete`]'s own doc
//! comment for the verification this claim rests on.
//!
//! ## Delegating to the real Q lexer, not a hand-rolled character scan
//!
//! Unlike J/APL's scanner (a raw character count, safe because neither
//! language's comment marker can contain an unbalanced paren that would
//! fool a naive scan), Q's comment marker (`/`, MA11 §3 bullet 2) is a
//! plain ASCII character that could easily appear unbalanced-looking
//! bracket characters *inside* comment text (e.g. `2+3 / a stray ( here`).
//! A naive character-level scan of the raw buffer would be fooled by that
//! stray `(` inside the comment and wait forever for a `)` that will never
//! come. Rather than re-deriving `q-lexer`'s own whitespace-sensitive
//! comment-stripping rule a second time by hand here (which this repo's
//! "no hand-written lexers" discipline warns against duplicating), this
//! scanner tokenizes the accumulated buffer with the *real*
//! `coding_adventures_q_lexer::try_tokenize_q` (which already strips
//! comments via its own pre-tokenize hook) and counts only genuine
//! `LPAREN`/`RPAREN`/`LBRACE`/`RBRACE`/`LBRACKET`/`RBRACKET` *tokens* — a
//! bracket character sitting inside a comment is never even tokenized as
//! one, so it can never be miscounted. Tokenizing a syntactically
//! incomplete-but-not-yet-closed buffer is always safe here: Q's lexer has
//! no notion of an "unterminated" token that could fail on partial input
//! (no multi-line string/char literal in this cut's alphabet at all,
//! MA11 §4), so a `try_tokenize_q` failure here can only mean a genuinely
//! unrecognized character — in that case this scanner reports "not
//! incomplete" (falls through to evaluation) so the *real* lex/parse error
//! surfaces through [`Interpreter::feed`]'s own `Result`, rather than
//! silently waiting for more input forever.
//!
//! Hand-rolled rather than built on the generic `repl` crate, mirroring
//! `j-repl`'s own rationale: the interpreter is single-threaded, and a
//! console session is sequential anyway.

use coding_adventures_q_runtime::Interpreter;
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

/// Upper bound on the pending-continuation buffer (while a bracket of any
/// kind is still unbalanced). Without this, a source that never closes its
/// brackets/braces/parens grows `buffer` without bound before anything is
/// ever parsed — mirrors `j-repl::MAX_CONTINUATION_BUFFER`'s identical
/// "generous but bounded" convention and value.
const MAX_CONTINUATION_BUFFER: usize = 64 * 1024;

/// Upper bound on a single *physical* line read from the input stream,
/// applied in [`read_bounded_line`] before [`MAX_CONTINUATION_BUFFER`]'s
/// own check ever runs. Mirrors `j-repl::MAX_LINE_LEN` exactly (see that
/// module's own doc comment for the full rationale — `BufRead::read_line`
/// has no length bound of its own, so without this a single, arbitrarily
/// long physical line would be fully buffered in memory regardless of
/// `MAX_CONTINUATION_BUFFER`).
const MAX_LINE_LEN: u64 = 64 * 1024;

/// Read one physical line, bounded to [`MAX_LINE_LEN`] bytes. Byte-for-byte
/// identical algorithm to `j-repl::read_bounded_line` (see that function's
/// own extensive doc comment for the full rationale of every design
/// decision here — the byte-oriented multibyte-boundary handling, the
/// "oversized decided by all three conditions" rule, and the
/// fully-drain-the-oversized-line loop) — this repl's own line-reading
/// concern is identical to J's/APL's; only the *bracket-balance* scanner
/// below is genuinely new to Q.
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
            let mut limited: std::io::Take<&mut R> = std::io::Read::take(&mut *reader, MAX_LINE_LEN);
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

/// Whether `src` (the accumulated continuation buffer) still has an
/// unbalanced `(`, `{`, or `[` -- i.e. whether the REPL should keep reading
/// more physical lines before handing `src` to the interpreter.
///
/// Tokenizes `src` with the *real* Q lexer (see this module's own top doc
/// comment for why: comment-awareness comes for free this way, with no
/// risk of a stray bracket character *inside* a comment fooling a naive
/// character scan) and counts three *independent* running depths — a
/// bracket type is "still open" only by its *own* count, so `{)` (a brace
/// opened, then a paren closed with no matching open) correctly stays
/// "incomplete" on the strength of the still-unbalanced brace, rather than
/// two mismatched counts happening to cancel out in a combined tally.
///
/// If tokenizing `src` itself fails (a genuinely unrecognized character —
/// the only way `try_tokenize_q` can fail, since this cut's lexer has no
/// literal type that could be "unterminated" on partial input, MA11 §4),
/// this returns `false` (not incomplete) rather than looping forever: the
/// real lex error will surface cleanly through [`Interpreter::feed`]'s own
/// `Result` once this buffer is hu to evaluation, exactly like any other
/// runtime error this REPL already displays without crashing.
///
/// # Verified by hand-tracing exactly the scenario this module's doc
/// comment describes (not just asserted)
///
/// - `{[x;y]` alone: tokenizes to `LBRACE LBRACKET NAME SEMICOLON NAME
///   RBRACKET` -- braces=1, brackets=0 (opened then closed) -- still
///   "incomplete" (`braces > 0`), correctly waiting for the rest of the
///   body and the closing `}`.
/// - Continuing with ` x+y}`: the *combined*, space-joined buffer
///   `{[x;y] x+y}` now tokenizes with braces returning to 0 -- no longer
///   incomplete, handed off to the interpreter as one complete statement.
/// - A complete one-line statement, e.g. `2+2`, has no bracket tokens at
///   all -- all three counters stay `0` -- never spuriously waits.
/// - `(1+2` (open paren, J's/APL's own classic case): `parens = 1`,
///   `braces = 0`, `brackets = 0` -- incomplete on the strength of `parens`
///   alone, exactly like `j-repl`'s own scanner.
/// - `{)`  (mismatched: a brace opened, then a *paren* closed with nothing
///   open): `braces = 1` (LBRACE, never decremented -- the RPAREN only
///   touches the `parens` counter, whose own decrement is clamped at 0 by
///   `.max(0)` since no LPAREN preceded it) -- correctly still
///   "incomplete", unlike a single combined depth counter, which would
///   have let the mismatched close cancel the open out to `0` and stopped
///   waiting prematurely.
fn is_incomplete(src: &str) -> bool {
    let tokens = match coding_adventures_q_lexer::try_tokenize_q(src) {
        Ok(tokens) => tokens,
        Err(_) => return false,
    };
    let mut parens = 0i32;
    let mut braces = 0i32;
    let mut brackets = 0i32;
    for t in &tokens {
        match t.effective_type_name() {
            "LPAREN" => parens += 1,
            "RPAREN" => parens = (parens - 1).max(0),
            "LBRACE" => braces += 1,
            "RBRACE" => braces = (braces - 1).max(0),
            "LBRACKET" => brackets += 1,
            "RBRACKET" => brackets = (brackets - 1).max(0),
            _ => {}
        }
    }
    parens > 0 || braces > 0 || brackets > 0
}

/// A persistent interactive Q session.
pub struct QRepl {
    interp: Interpreter,
    buffer: String,
}

impl Default for QRepl {
    fn default() -> Self {
        Self::new()
    }
}

impl QRepl {
    pub fn new() -> Self {
        QRepl {
            interp: Interpreter::new(),
            buffer: String::new(),
        }
    }

    /// `>> ` for a fresh statement, `... ` while continuing an incomplete
    /// one (an open `(`/`{`/`[`).
    pub fn prompt(&self) -> &'static str {
        if self.buffer.is_empty() {
            ">> "
        } else {
            "... "
        }
    }

    /// Feed one physical input line (without its trailing newline).
    ///
    /// # The buffer-size-cap ordering (checked BEFORE appending, not after)
    ///
    /// This repo already paid down the "push-before-size-check" bug class
    /// once, across every sibling REPL (task #80, PRs already merged) — the
    /// continuation buffer's own cap must be checked *before* the new line
    /// is appended, or the cap can still be exceeded by up to one whole
    /// line's worth of bytes (the ordering `j-repl::JRepl::feed` already
    /// fixed, and mirrored here verbatim: compute `separator_len` first,
    /// check the *prospective* total size, and only `push_str` after that
    /// check passes).
    pub fn feed(&mut self, line: &str) -> ReplResponse {
        if self.buffer.is_empty() {
            match line.trim() {
                "quit" | "exit" | "quit()" | "exit()" => return ReplResponse::Quit,
                _ => {}
            }
        }

        // Bound the accumulation buffer BEFORE growing it -- see this
        // method's own doc comment. `separator_len` accounts for the
        // joining space this method adds below when the buffer already
        // holds a still-open continuation.
        let separator_len = if self.buffer.is_empty() { 0 } else { 1 };
        if self
            .buffer
            .len()
            .saturating_add(separator_len)
            .saturating_add(line.len())
            > MAX_CONTINUATION_BUFFER
        {
            self.buffer.clear();
            return ReplResponse::Output(format!(
                "Error: statement exceeds the {MAX_CONTINUATION_BUFFER}-byte continuation limit; discarded\n"
            ));
        }

        if separator_len == 0 {
            self.buffer.push_str(line);
        } else {
            // Joining with a single space (not a real '\n') keeps a still-
            // open `{...}`/`(...)`/`[...]` on one logical line -- Q's own
            // NEWLINE token is significant between top-level statements
            // (`q.grammar`'s `line = statement NEWLINE | statement |
            // NEWLINE`), and injecting one *inside* a still-open bracket
            // would hand the parser a genuinely broken program, exactly
            // mirroring `j-repl`'s identical rationale for its own simpler
            // (paren-only) case.
            self.buffer.push(' ');
            self.buffer.push_str(line);
        }

        if is_incomplete(&self.buffer) {
            return ReplResponse::NeedMore;
        }

        let src = std::mem::take(&mut self.buffer);
        if src.trim().is_empty() {
            return ReplResponse::Output(String::new());
        }
        match self.interp.feed(&format!("{src}\n")) {
            Ok(text) => ReplResponse::Output(text),
            Err(e) => ReplResponse::Output(format!("Error: {e}\n")),
        }
    }

    pub fn is_continuing(&self) -> bool {
        !self.buffer.is_empty()
    }
}

/// Drive a full interactive Q session over the given reader and writer.
pub fn run<R: BufRead, W: Write>(mut reader: R, mut writer: W) -> std::io::Result<()> {
    let mut repl = QRepl::new();
    writeln!(writer, "Q (on array-runtime) — type quit to exit.")?;

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
        let mut r = QRepl::new();
        assert!(matches!(r.feed("2+2"), ReplResponse::Output(t) if t.trim() == "4"));
        assert_eq!(r.feed("a:5"), ReplResponse::Output(String::new()));
    }

    // ── The genuinely new concern: brace/bracket continuation ─────────────

    #[test]
    fn continues_across_an_open_paren_exactly_like_j_repl() {
        let mut r = QRepl::new();
        assert_eq!(r.feed("(1+2"), ReplResponse::NeedMore);
        assert!(r.is_continuing());
        assert!(matches!(r.feed("+3)"), ReplResponse::Output(t) if t.trim() == "6"));
        assert!(!r.is_continuing());
    }

    #[test]
    fn continues_across_a_multi_line_function_literal_body() {
        // The scenario this crate's own module doc comment hand-traces:
        // `{[x;y]` on one line, ` x+y}` on the next -- a genuinely new
        // continuation shape with no J/APL precedent (neither has a
        // user-defined block construct at all).
        let mut r = QRepl::new();
        assert_eq!(r.feed("f:{[x;y]"), ReplResponse::NeedMore);
        assert!(r.is_continuing());
        assert_eq!(r.feed(" x+y}"), ReplResponse::Output(String::new()));
        assert!(!r.is_continuing());
        // Confirm the function was actually captured correctly (not just
        // that the scanner stopped waiting) by calling it in a follow-up
        // line.
        assert!(matches!(r.feed("2 f 3"), ReplResponse::Output(t) if t.trim() == "5"));
    }

    #[test]
    fn continues_across_a_split_bracketed_parameter_list() {
        // The optional parameter list's own `[`/`]` -- a real, if unusual,
        // way to split a line, and the reason this scanner needs bracket
        // balance too, not just brace balance.
        let mut r = QRepl::new();
        assert_eq!(r.feed("f:{[x;"), ReplResponse::NeedMore);
        assert_eq!(r.feed("y] x*y}"), ReplResponse::Output(String::new()));
        assert!(matches!(r.feed("2 f 3"), ReplResponse::Output(t) if t.trim() == "6"));
    }

    #[test]
    fn a_complete_one_line_statement_never_spuriously_waits() {
        let mut r = QRepl::new();
        // Every kind of complete-on-one-line statement this language has:
        // a function literal fully closed, a list literal fully closed,
        // and plain grouping fully closed.
        assert!(matches!(r.feed("f:{x+y}"), ReplResponse::Output(t) if t.is_empty()));
        assert!(matches!(r.feed("(1;2;3)"), ReplResponse::Output(_)));
        assert!(matches!(r.feed("(1+2)*3"), ReplResponse::Output(t) if t.trim() == "9"));
    }

    #[test]
    fn a_stray_bracket_character_inside_a_comment_does_not_fool_the_scanner() {
        // The real risk a naive character-level scan would have: `/` opens
        // a Q comment (MA11 §3 bullet 2), and the comment text below
        // contains an unbalanced `(` that must NOT be counted, since
        // `q-lexer`'s own pre-tokenize hook already blanks it out before
        // this scanner ever sees a token for it.
        let mut r = QRepl::new();
        match r.feed("1+1 / a stray ( in a comment") {
            ReplResponse::Output(t) => assert_eq!(t.trim(), "2"),
            other => panic!("expected an immediate result, not NeedMore -- got {other:?}"),
        }
        assert!(!r.is_continuing());
    }

    #[test]
    fn mismatched_bracket_types_stay_incomplete_on_their_own_open_count() {
        // `{)`-shaped input: a brace opened, then a paren "closed" with
        // nothing open -- must stay incomplete on the strength of the
        // still-open BRACE, not have the mismatched close cancel it out
        // (see `is_incomplete`'s own doc comment for the full rationale).
        assert!(is_incomplete("{)"));
    }

    #[test]
    fn an_unbounded_continuation_is_discarded_not_grown_forever() {
        let mut r = QRepl::new();
        assert_eq!(r.feed("(1"), ReplResponse::NeedMore);
        let filler = "+1".repeat(MAX_CONTINUATION_BUFFER / 2 + 10);
        match r.feed(&filler) {
            ReplResponse::Output(t) => assert!(t.contains("Error")),
            other => panic!("expected an Error output once the cap is exceeded, got {other:?}"),
        }
        assert!(!r.is_continuing());
        assert!(matches!(r.feed("1+1"), ReplResponse::Output(t) if t.trim() == "2"));
    }

    #[test]
    fn buffer_cap_is_checked_before_appending_not_after() {
        // A direct regression test for the exact bug class this repo
        // already paid down once (task #80): feed a single line whose
        // length ALONE already exceeds the cap (never mind any
        // accumulation) -- if the size check ran AFTER appending, this
        // single `push_str` would already have grown `buffer` past the
        // cap before the check ever fired, but the cap would still
        // (eventually) catch it on a LATER call once the accumulated
        // total was inspected again; checking BEFORE means it is caught
        // on THIS very call, with the buffer never having held the
        // oversized content at all. We confirm the stronger property: the
        // buffer is fully cleared and empty immediately after this single
        // call, not left holding a too-large-but-not-yet-noticed value.
        let mut r = QRepl::new();
        let _ = r.feed("(");
        let oversized = "+1".repeat(MAX_CONTINUATION_BUFFER);
        let response = r.feed(&oversized);
        assert!(matches!(response, ReplResponse::Output(t) if t.contains("Error")));
        assert!(!r.is_continuing(), "buffer must be cleared, not left oversized");
    }

    #[test]
    fn quit_commands() {
        assert_eq!(QRepl::new().feed("quit"), ReplResponse::Quit);
        assert_eq!(QRepl::new().feed("exit"), ReplResponse::Quit);
    }

    #[test]
    fn errors_are_shown_not_fatal() {
        let mut r = QRepl::new();
        assert!(matches!(r.feed("undefined_var"), ReplResponse::Output(t) if t.contains("Error")));
        assert!(matches!(r.feed("1+1"), ReplResponse::Output(t) if t.trim() == "2"));
    }

    #[test]
    fn session_persists_across_lines() {
        let mut r = QRepl::new();
        r.feed("a:10");
        assert!(matches!(r.feed("a+5"), ReplResponse::Output(t) if t.trim() == "15"));
    }

    #[test]
    fn zero_based_til_survives_a_real_repl_session() {
        let mut r = QRepl::new();
        assert!(matches!(r.feed("!5"), ReplResponse::Output(t) if t.trim() == "0 1 2 3 4"));
    }

    #[test]
    fn run_drives_a_session_to_eof() {
        let input = "a:3\na*2\nquit\n".as_bytes();
        let mut output = Vec::new();
        run(input, &mut output).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains('6'));
    }

    #[test]
    fn run_handles_a_multi_line_function_literal_end_to_end() {
        let input = "f:{[x;y]\n x+y}\n2 f 3\nquit\n".as_bytes();
        let mut output = Vec::new();
        run(input, &mut output).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains('5'), "expected 2 f 3 == 5 in output, got: {text}");
    }

    #[test]
    fn read_bounded_line_returns_a_final_line_without_a_trailing_newline() {
        let mut input = "quit".as_bytes();
        assert_eq!(
            read_bounded_line(&mut input).unwrap(),
            Some(Ok("quit".to_string()))
        );
        assert_eq!(read_bounded_line(&mut input).unwrap(), None);
    }

    #[test]
    fn run_quits_cleanly_on_a_final_line_without_a_trailing_newline() {
        let input = "quit".as_bytes();
        let mut output = Vec::new();
        run(input, &mut output).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(!text.contains("exceeds"));
    }

    #[test]
    fn read_bounded_line_rejects_a_line_exceeding_the_cap_without_buffering_it_whole() {
        let oversized = "+".repeat(MAX_LINE_LEN as usize * 3);
        let mut input = oversized.as_bytes();
        assert_eq!(read_bounded_line(&mut input).unwrap(), Some(Err(())));
        let _ = read_bounded_line(&mut input);
    }

    #[test]
    fn run_reports_an_oversized_line_cleanly_and_keeps_the_session_alive() {
        let oversized = "+".repeat(MAX_LINE_LEN as usize * 2);
        let input = format!("{oversized}\n1+1\nquit\n");
        let mut output = Vec::new();
        run(input.as_bytes(), &mut output).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("exceeds"), "expected an oversized-line error, got: {text}");
        assert!(text.contains('2'), "session must keep working after an oversized line, got: {text}");
    }
}
