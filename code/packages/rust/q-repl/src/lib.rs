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
//! verbatim and assume it is sufficient — see [`QRepl::feed`]'s and
//! [`apply_line_bracket_tokens`]'s own doc comments for the verification
//! this claim rests on.
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
//! comment-stripping rule a second time by hand here for *tokenizing*
//! purposes (which this repo's "no hand-written lexers" discipline warns
//! against duplicating), this scanner tokenizes each physical line with the
//! *real* `coding_adventures_q_lexer::try_tokenize_q` (which already strips
//! comments via its own pre-tokenize hook) and counts only genuine
//! `LPAREN`/`RPAREN`/`LBRACE`/`RBRACE`/`LBRACKET`/`RBRACKET` *tokens* — a
//! bracket character sitting inside a comment is never even tokenized as
//! one, so it can never be miscounted.
//!
//! ## Why comments must be pre-blanked *per physical line*, before the join
//! (not just accounted for when checking completeness)
//!
//! An earlier version of this scanner tokenized the *whole accumulated,
//! space-joined* `self.buffer` on every call. That has a real bug: `/`
//! blanks from itself through the next **real** `'\n'` or end of input, but
//! this REPL joins continuation lines with a single **space** (never a
//! real `'\n'`, see [`QRepl::feed`]'s own doc comment for why) — so a
//! comment opened on one physical line, once joined into the single-line
//! buffer, has no real `'\n'` left to stop at and silently blanks *every
//! subsequent physical line* too, including whatever closing bracket was
//! supposed to complete the statement. The session would then wait for
//! more input forever (the "closing" text was erased before it could be
//! counted), and — just as badly — even if completeness were somehow
//! detected some other way, hand raw `self.buffer` to [`Interpreter::feed`]
//! and the identical problem recurs there: tokenizing the whole
//! (still-comment-bearing, still-`'\n'`-free) buffer at evaluation time
//! would blank the exact same "rest of the statement" a second time.
//!
//! The fix ([`blank_line_comment`]) blanks each physical line's own
//! trailing comment (if any) to spaces **before** it is ever appended to
//! `self.buffer` or tokenized for bracket-counting — both problems have the
//! same root cause (a comment's real extent is only known one physical
//! line at a time, before the lossy space-join), so both are fixed by the
//! same per-line pre-processing step, not two separate patches. This is a
//! narrow, deliberate, *documented* duplication of `q-lexer`'s comment rule
//! (that crate's own `strip_slash_comments` is private and built for a
//! whole, possibly-multi-line source, not a single line) — see
//! `blank_line_comment`'s own doc comment for why this narrower scope
//! makes the duplication sound rather than a maintenance hazard.
//!
//! ## Incremental, not whole-buffer, bracket counting (avoiding O(n²) cost)
//!
//! Tokenizing the *entire* accumulated buffer from scratch on every single
//! physical line fed (the earlier version's approach) costs O(buffer
//! length) per call; summed over a continuation that grows to the full
//! `MAX_CONTINUATION_BUFFER` one short line at a time, the cumulative cost
//! is O(n²) in the number of lines. [`QRepl`] instead tracks **running**
//! `(parens, braces, brackets)` counts as instance state and, on each
//! `feed()` call, tokenizes only the *newly appended* (comment-blanked)
//! line fragment, applying that fragment's own tokens to the running
//! counts **one token at a time** ([`apply_line_bracket_tokens`]), clamping
//! each counter at 0 immediately after every token that touches it — O(line
//! length) per call, O(buffer length) total across an entire continuation,
//! not O(buffer length²). This is sound because no token in this cut's
//! grammar can span a line-fragment boundary (no multi-line string/number
//! literal, MA11 §4) — tokenizing one fragment at a time and replaying its
//! tokens against the *persisted* running state gives the identical bracket
//! counts whole-buffer tokenization would have, every time. **Critically,
//! this must clamp per token against the persisted state, not compute an
//! independent net delta for the whole fragment and clamp once** — see
//! `apply_line_bracket_tokens`'s own doc comment for a concrete case where
//! the two diverge (a security-review finding from an earlier version of
//! this function that computed net deltas).
//!
//! Tokenizing a syntactically incomplete-but-not-yet-closed fragment is
//! always safe here: Q's lexer has no notion of an "unterminated" token
//! that could fail on partial input (no multi-line string/char literal in
//! this cut's alphabet at all, MA11 §4), so a `try_tokenize_q` failure here
//! can only mean a genuinely unrecognized character — in that case this
//! scanner reports "not incomplete" (falls through to evaluation) so the
//! *real* lex/parse error surfaces through [`Interpreter::feed`]'s own
//! `Result`, rather than silently waiting for more input forever.
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

/// Blank out a `/`-to-end-of-line comment on a **single physical line**
/// (never containing an embedded `'\n'` — every line reaching [`QRepl::feed`]
/// has already had its own trailing newline stripped by [`run`]'s caller
/// logic before `feed` ever sees it).
///
/// Mirrors `q-lexer`'s own (private) `strip_slash_comments` pre-tokenize
/// hook exactly (MA11 §3 bullet 2's rule: a `/` preceded by whitespace, or
/// at the very start of the line, opens a comment that runs to the next
/// real `'\n'` or end of input) — necessarily re-derived here, since that
/// function is private to `q-lexer` and is built to scan a whole,
/// potentially multi-line source, tracking whitespace-adjacency across
/// real embedded newlines. This is **sound to duplicate at this narrower
/// scope**, not a maintenance hazard, specifically because a single
/// physical line never contains an embedded `'\n'` at all: the full rule
/// ("blank until the next real `'\n'` or end of input") degenerates
/// exactly to "blank to the end of this line", with no multi-line state to
/// track — and because a real, previously-processed physical line always
/// had a real `'\n'` (itself whitespace-like) immediately before the next
/// one in genuine multi-line source, treating "start of this line" as
/// whitespace-like (this function's own initial state, matching
/// `q-lexer`'s identical "start of input counts as whitespace-like" rule)
/// reproduces the exact same decision the real, whole-source algorithm
/// would have made at that position.
///
/// See this module's own top doc comment ("Why comments must be
/// pre-blanked *per physical line*") for why this must happen **before**
/// a line is folded into `self.buffer` at all, not merely accounted for
/// when checking completeness: this REPL joins continuation lines with a
/// single space, never a real `'\n'` (MA11's own significant-`NEWLINE`
/// grammar rule forbids injecting one mid-expression), so a comment that
/// is not blanked here would have no real `'\n'` left anywhere in the
/// joined buffer to stop at — silently erasing every subsequent line, at
/// both the completeness-check stage and, just as importantly, at the
/// final evaluation stage (`Interpreter::feed` tokenizes the very same
/// joined buffer and would hit the identical problem).
fn blank_line_comment(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(n);
    let mut prev_is_whitespace_like = true; // start of line counts as whitespace-like
    let mut i = 0;
    while i < n {
        let c = chars[i];
        if c == '/' && prev_is_whitespace_like {
            // Comment: blank through to the end of this line -- there is
            // no embedded '\n' to stop at any earlier.
            for _ in i..n {
                out.push(' ');
            }
            break;
        }
        out.push(c);
        prev_is_whitespace_like = matches!(c, ' ' | '\t' | '\r' | '\n');
        i += 1;
    }
    out
}

/// Apply one already-comment-blanked line fragment's own bracket tokens to
/// the running `(*parens, *braces, *brackets)` counts **one token at a
/// time**, clamping each counter at 0 immediately after every single token
/// that touches it — exactly mirroring the original whole-buffer
/// algorithm's own per-token clamp (`(count - 1).max(0)` applied at each
/// closing bracket as it is encountered), just replayed over one fragment
/// at a time instead of the whole buffer from position 0.
///
/// # Why this must clamp per token, not compute one net delta and clamp once
///
/// An earlier version of this function computed a single **net** delta for
/// the whole line (summing every token's +1/-1 contribution) and let the
/// caller clamp the *running total* only once, after folding that delta
/// in. That diverges from the original whole-buffer algorithm whenever a
/// line's own tokens dip a counter below zero **mid-line** before a later,
/// genuinely-unmatched open of the same type: an excess close must be
/// "forgiven" (clamped to 0) at the exact point it occurs, so it can never
/// later be arithmetically cancelled out by a subsequent open within the
/// same line. Concretely, `")("` as a lone first line: per-token clamping
/// gives `RPAREN` -> clamped to 0, then `LPAREN` -> 1 (still incomplete,
/// matching the original algorithm); a net-delta-then-clamp-once approach
/// computes `-1 + 1 = 0` and reports "complete" instead — silently wrong.
/// (Found and confirmed by a second security-review round; see this
/// module's own regression tests for both this single-line case and a
/// realistic multi-line sequence with the identical shape.)
///
/// This per-token-clamped walk over just the new fragment is mathematically
/// identical to replaying the *entire* whole-buffer algorithm from
/// scratch: a per-token-clamped walk's end state is fully determined by its
/// starting value and its remaining tokens, regardless of whether that
/// starting value came from replaying from 0 or from resuming a prior
/// clamped walk that already processed everything before it — which is
/// exactly what makes tokenizing only the new fragment (not the whole
/// buffer) a sound, O(line length) replacement for O(buffer length)
/// whole-buffer re-tokenization on every call.
///
/// Returns `false` if tokenizing `line` itself failed (a genuinely
/// unrecognized character — the only way `try_tokenize_q` can fail, since
/// this cut's lexer has no literal type that could be "unterminated" on
/// partial input, MA11 §4) **without touching any of the three counters**;
/// the caller treats this the same way the original whole-buffer version
/// did — proceed to evaluation immediately rather than waiting forever,
/// letting the real lex/parse error surface through [`Interpreter::feed`]'s
/// own `Result`.
fn apply_line_bracket_tokens(
    line: &str,
    parens: &mut i32,
    braces: &mut i32,
    brackets: &mut i32,
) -> bool {
    let tokens = match coding_adventures_q_lexer::try_tokenize_q(line) {
        Ok(tokens) => tokens,
        Err(_) => return false,
    };
    for t in &tokens {
        match t.effective_type_name() {
            "LPAREN" => *parens += 1,
            "RPAREN" => *parens = (*parens - 1).max(0),
            "LBRACE" => *braces += 1,
            "RBRACE" => *braces = (*braces - 1).max(0),
            "LBRACKET" => *brackets += 1,
            "RBRACKET" => *brackets = (*brackets - 1).max(0),
            _ => {}
        }
    }
    true
}

/// A persistent interactive Q session.
pub struct QRepl {
    interp: Interpreter,
    buffer: String,
    /// Running open-bracket counts for the CURRENT continuation, updated
    /// incrementally (one line's own delta at a time) rather than
    /// recomputed by re-tokenizing all of `buffer` on every call — see this
    /// module's own top doc comment, "Incremental, not whole-buffer,
    /// bracket counting".
    open_parens: i32,
    open_braces: i32,
    open_brackets: i32,
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
            open_parens: 0,
            open_braces: 0,
            open_brackets: 0,
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
    ///
    /// # Comment blanking happens first, before anything else touches `line`
    ///
    /// `line` is comment-blanked ([`blank_line_comment`]) *before* it is
    /// measured for the size cap, appended to `self.buffer`, or tokenized
    /// for its own bracket delta — see this module's own top doc comment,
    /// "Why comments must be pre-blanked per physical line", for why this
    /// single step is what fixes both the comment-swallowing bug and (by
    /// construction) keeps the incremental bracket count correct. Blanking
    /// only replaces characters with spaces (same length), so the size
    /// check's byte-count math is unaffected either way.
    ///
    /// # Precondition: `line` is a single physical line (no embedded `'\n'`)
    ///
    /// [`blank_line_comment`]'s own soundness (see its doc comment) relies
    /// on `line` never containing an embedded `'\n'` — true by construction
    /// at this crate's own sole call site ([`run`], which always strips a
    /// physical line's own trailing newline before calling `feed`), but
    /// `feed`/`QRepl` are public API: a future direct caller passing a
    /// string with an embedded `'\n'` would silently reintroduce a scoped
    /// version of the exact comment-swallowing bug this module's own
    /// `CHANGELOG.md` documents. Asserted here (debug-only, zero release
    /// cost) so a violation fails loudly in tests/debug builds rather than
    /// silently misbehaving.
    pub fn feed(&mut self, line: &str) -> ReplResponse {
        debug_assert!(
            !line.contains('\n'),
            "QRepl::feed expects a single physical line without an embedded newline"
        );
        if self.buffer.is_empty() {
            match line.trim() {
                "quit" | "exit" | "quit()" | "exit()" => return ReplResponse::Quit,
                _ => {}
            }
        }

        let line = blank_line_comment(line);

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
            self.open_parens = 0;
            self.open_braces = 0;
            self.open_brackets = 0;
            return ReplResponse::Output(format!(
                "Error: statement exceeds the {MAX_CONTINUATION_BUFFER}-byte continuation limit; discarded\n"
            ));
        }

        if separator_len == 0 {
            self.buffer.push_str(&line);
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
            self.buffer.push_str(&line);
        }

        // Incremental, per-token-clamped bracket update (see this module's
        // own top doc comment, "Incremental, not whole-buffer, bracket
        // counting", and `apply_line_bracket_tokens`'s own doc comment for
        // exactly why this must clamp per token against the PERSISTED
        // running state, not compute an independent net delta for the
        // whole line and clamp once) -- tokenizes only the just-appended
        // (already comment-blanked) line, not the whole accumulated
        // buffer. A tokenize failure on this one fragment forces an
        // immediate attempt at evaluation (matching the previous
        // whole-buffer version's identical "don't wait forever on a
        // genuinely bad character" behavior), regardless of the running
        // counts.
        let tokenized_ok = apply_line_bracket_tokens(
            &line,
            &mut self.open_parens,
            &mut self.open_braces,
            &mut self.open_brackets,
        );
        let still_incomplete = tokenized_ok
            && (self.open_parens > 0 || self.open_braces > 0 || self.open_brackets > 0);
        if still_incomplete {
            return ReplResponse::NeedMore;
        }

        self.open_parens = 0;
        self.open_braces = 0;
        self.open_brackets = 0;
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

    /// Security-review regression (Finding 1, HIGH): a `/`-comment opened on
    /// one physical line of a still-open continuation must NOT swallow
    /// every subsequently-typed line. Before the per-line comment-blanking
    /// fix, the second `feed()` call below returned `NeedMore` forever (the
    /// combined space-joined buffer has no real `'\n'` for the comment to
    /// stop at, so it erased `"+2)"` along with the comment) instead of
    /// completing the statement and evaluating it to `3`.
    #[test]
    fn a_comment_opened_mid_continuation_does_not_swallow_the_rest_of_the_statement() {
        let mut r = QRepl::new();
        assert_eq!(r.feed("(1 / comment"), ReplResponse::NeedMore);
        assert!(r.is_continuing());
        match r.feed("+2)") {
            ReplResponse::Output(t) => assert_eq!(t.trim(), "3"),
            other => panic!(
                "expected the statement to complete and evaluate to 3, got {other:?} \
                 (a comment on the first line must not swallow the second line)"
            ),
        }
        assert!(!r.is_continuing());
    }

    /// The same scenario, but with the comment-opening line ending in an
    /// otherwise-legitimate trailing space before the continuation, and
    /// with a brace/bracket continuation instead of a paren -- confirms the
    /// fix isn't accidentally specific to the exact repro shape above.
    #[test]
    fn a_comment_inside_a_multi_line_function_literal_does_not_swallow_the_closing_brace() {
        let mut r = QRepl::new();
        assert_eq!(r.feed("f:{[x;y] / defines add"), ReplResponse::NeedMore);
        match r.feed("x+y}") {
            ReplResponse::Output(t) => assert_eq!(t, ""),
            other => panic!("expected the function definition to complete silently, got {other:?}"),
        }
        assert!(matches!(r.feed("2 f 3"), ReplResponse::Output(t) if t.trim() == "5"));
    }

    /// Security-review regression (Finding 2, MEDIUM): re-tokenizing the
    /// *entire* accumulated buffer on every fed line costs O(buffer length)
    /// per call, summing to O(n²) over a continuation that grows one short
    /// line at a time.
    ///
    /// A pure "N lines must complete within X seconds" bound would be
    /// confounded by `try_tokenize_q`'s own fixed per-call cost (rebuilding
    /// ~30 token patterns from `_grammar::token_grammar()` every call,
    /// independent of input length) — that fixed cost is paid by both the
    /// old whole-buffer approach and this one, and could easily dominate at
    /// small N regardless of which is used, telling us nothing about
    /// *scaling*. Instead this is a **comparative** measurement: feed the
    /// exact same `n` trivial filler lines twice — once against a small
    /// starting buffer, once against a buffer already pre-grown to just
    /// under the continuation cap — and confirm the timing is
    /// approximately the same either way. Every filler line here is an
    /// all-comment line (`"/"`, blanked to a single space by
    /// `blank_line_comment`) specifically so it never adds any real parse-
    /// tree depth (avoiding `q-parser`'s own `MAX_RULE_DEPTH`, which caps a
    /// genuine nested/chained expression at only ~13-26 terms — far too
    /// shallow to build a meaningfully large N through real expression
    /// content). If the whole-buffer-re-tokenization bug were still
    /// present, the "large starting buffer" run would take dramatically
    /// longer, since each of its `n` calls would re-scan the *entire*
    /// (large) buffer instead of just its own tiny new fragment.
    #[test]
    fn per_line_scanning_cost_does_not_scale_with_the_existing_buffer_size() {
        fn time_n_filler_lines(r: &mut QRepl, n: usize) -> std::time::Duration {
            let start = std::time::Instant::now();
            for _ in 0..n {
                assert_eq!(r.feed("/"), ReplResponse::NeedMore);
            }
            start.elapsed()
        }

        let n = 500;

        // Scenario A: small starting buffer.
        let mut small = QRepl::new();
        assert_eq!(small.feed("(0"), ReplResponse::NeedMore);
        let small_time = time_n_filler_lines(&mut small, n);

        // Scenario B: buffer already grown to just under the 64 KiB
        // continuation cap via a single long all-comment padding line (one
        // `feed()` call, not timed) -- still trivially shallow to parse
        // (it's all blanked to whitespace), so no recursion-depth risk.
        let mut large = QRepl::new();
        assert_eq!(large.feed("(0"), ReplResponse::NeedMore);
        let padding = format!("/{}", "x".repeat(60_000));
        assert_eq!(large.feed(&padding), ReplResponse::NeedMore);
        let large_time = time_n_filler_lines(&mut large, n);

        let ratio = large_time.as_secs_f64() / small_time.as_secs_f64().max(1e-6);
        assert!(
            ratio < 5.0,
            "feeding {n} trivial filler lines took {large_time:?} against a ~60 KiB \
             pre-existing buffer vs {small_time:?} against a small one ({ratio:.1}x) -- \
             per-line cost should not scale with the EXISTING buffer size (O(1) \
             amortized per line, not O(buffer length))"
        );

        // Sanity check the fast path didn't corrupt anything: closing
        // `small`'s continuation (untouched by all this comment filler,
        // which contributes zero real tokens) must still evaluate to
        // plain `0`.
        match small.feed(")") {
            ReplResponse::Output(t) => assert_eq!(t.trim(), "0"),
            other => panic!("expected the statement to close cleanly to 0, got {other:?}"),
        }
    }

    #[test]
    fn mismatched_bracket_types_stay_incomplete_on_their_own_open_count() {
        // `{)`-shaped input: a brace opened, then a paren "closed" with
        // nothing open -- must stay incomplete on the strength of the
        // still-open BRACE, not have the mismatched close cancel it out
        // (see `apply_line_bracket_tokens`'s own doc comment for the full
        // rationale). The stray RPAREN clamps `parens` to 0 immediately
        // (not -1) -- it is "forgiven", not carried as a negative value.
        let (mut parens, mut braces, mut brackets) = (0i32, 0i32, 0i32);
        assert!(apply_line_bracket_tokens("{)", &mut parens, &mut braces, &mut brackets));
        assert_eq!((parens, braces, brackets), (0, 1, 0));

        let mut r = QRepl::new();
        assert_eq!(r.feed("{)"), ReplResponse::NeedMore);
    }

    /// Security-review round 2 regression: the simplest counterexample
    /// where clamping a whole-line NET delta once (an earlier, incorrect
    /// version of `apply_line_bracket_tokens`) diverges from clamping PER
    /// TOKEN as the original whole-buffer algorithm did. `")("` as a lone
    /// first line: per-token clamping gives `RPAREN` -> forgiven (clamped
    /// to 0), then `LPAREN` -> 1 (still incomplete); a net-delta approach
    /// computes `-1 + 1 = 0` and would (wrongly) report "complete".
    #[test]
    fn a_line_with_more_closes_than_opens_forgives_the_excess_rather_than_cancelling_a_later_open() {
        let (mut parens, mut braces, mut brackets) = (0i32, 0i32, 0i32);
        assert!(apply_line_bracket_tokens(")(", &mut parens, &mut braces, &mut brackets));
        assert_eq!((parens, braces, brackets), (1, 0, 0));

        let mut r = QRepl::new();
        assert_eq!(r.feed(")("), ReplResponse::NeedMore);
    }

    /// The same counterexample, at the full `QRepl` session level with a
    /// realistic multi-line shape: `feed("(1")`, `feed("))+((2")`,
    /// `feed(")")`. One real `(` from the first line remains unmatched
    /// throughout (the middle line's first close matches it, its second
    /// close is forgiven, then it opens two MORE parens; the final line's
    /// lone close only accounts for one of those two). A net-delta-per-line
    /// approach would let the middle line's own `-2 + 2 = 0` net delta
    /// erase all memory that one of its closes was already spurious,
    /// making the final `")"` look (wrongly) like it completes the
    /// statement one line early.
    #[test]
    fn a_realistic_multi_line_sequence_with_excess_closes_still_waits_for_the_real_unmatched_open() {
        let mut r = QRepl::new();
        assert_eq!(r.feed("(1"), ReplResponse::NeedMore);
        assert_eq!(r.feed("))+((2"), ReplResponse::NeedMore);
        assert_eq!(
            r.feed(")"),
            ReplResponse::NeedMore,
            "one real unmatched '(' remains open from the very first line -- \
             must still wait for '... ', not prematurely declare the statement complete"
        );
        assert!(r.is_continuing());
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
