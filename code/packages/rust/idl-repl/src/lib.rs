//! # IDL REPL — an interactive Read-Eval-Print loop for IDL.
//!
//! [`IdlRepl`] wraps a persistent [`Interpreter`] and adds the interactive
//! behaviours a console needs: a **continuation scanner** deciding when a
//! chunk of typed input is a complete statement, and echoing of
//! `PRINT`/Implied-Print output. It is the sibling of `q-repl`/
//! `scilab-repl`; only the interpreter and the continuation scanner differ.
//! See `code/specs/MA12-idl-language.md` §5.
//!
//! ## The continuation scanner: paren/bracket balance, `$`, and `BEGIN...ENDxxx` depth
//!
//! MA12 §5 assigns this crate a responsibility both `idl-lexer` (MA-12b)
//! and `idl-parser` (MA-12c) explicitly disclosed as deferred to it: `$`
//! (line continuation) has no meaning anywhere in `idl.grammar` at all --
//! `idl-lexer` emits `CONTINUATION` as an ordinary, un-suppressed token and
//! does not swallow the newline that follows it, and `idl-parser`'s own
//! README states plainly "a bare `$` reaching this parser is a syntax error
//! by construction... this is MA-12d's job, not this crate's." This module
//! is that job. Three things must be tracked, at the **raw text** level,
//! before a chunk of input is ever handed to `tokenize_idl`/`parse_idl`:
//!
//! 1. **Paren/bracket balance** (`(`/`)`, `[`/`]`) -- an open group or
//!    subscript can legitimately span several physical lines.
//! 2. **The `$` line-continuation character** -- a trailing `$` (MA12 §3's
//!    own "lexer trivia" bullet) means "this logical line is not finished;
//!    join the next physical line onto it" -- a fundamentally different
//!    signal from bracket balance (a *complete*, balanced expression can
//!    still end in `$`, and a still-open bracket does not need one).
//! 3. **`BEGIN...ENDxxx`/`PRO`/`FUNCTION` block depth** -- `IF...THEN
//!    BEGIN`, `FOR...DO BEGIN`, `WHILE...DO BEGIN`, `REPEAT BEGIN`, a
//!    generic `BEGIN...END`, and `PRO`/`FUNCTION` definitions (all closed
//!    by a bare `END` or their own matched `ENDIF`/`ENDELSE`/`ENDFOR`/
//!    `ENDWHILE`/`ENDREP`, `idl.grammar`'s own header comment) can each
//!    legitimately span many physical lines when typed interactively.
//!
//! ## Comments must still be stripped per physical line, before the join -- a lesson re-confirmed, not skipped
//!
//! `idl.tokens`' own `skip: COMMENT = /;[^\n]*/` entry has **no**
//! pre/post-tokenize hook at all (confirmed directly against that file):
//! unlike Q's `/` (which needs whitespace-adjacency context to tell a
//! comment-opener from the REDUCE adverb), a `;` in IDL is *unconditionally*
//! a comment-opener, so tokenizing a **single, self-contained** physical
//! line with the real lexer already treats everything from an unquoted `;`
//! to that line's own end as trivia, with no ambiguity to resolve. It is
//! tempting to conclude from this that no comment-handling is needed here
//! at all -- but that conclusion is wrong, for exactly the reason `q-repl`'s
//! own top doc comment documents under "Why comments must be pre-blanked
//! *per physical line*": this scanner joins continuation lines into
//! `self.buffer` with a single **space**, never a real `'\n'` (see "Why
//! every join uses a space" below), so a comment's regex, `/;[^\n]*/`,
//! finds no real `'\n'` anywhere in the joined buffer to stop at once two
//! lines are joined -- a `;` on an EARLIER physical line would silently
//! swallow every character after it, including a later line's own closing
//! bracket, straight through to the end of the accumulated buffer. (This
//! was caught by this crate's own test suite before ever shipping: a
//! filled-with-comment-lines regression test that expected `NeedMore` on
//! every filler line instead surfaced a genuine parse error once the
//! statement was finally submitted, because an earlier `;` had eaten the
//! very `)` meant to close it.)
//!
//! The fix ([`strip_trailing_comment`]) is simpler than `q-repl`'s own
//! `blank_line_comment`, precisely because IDL's `;` needs no
//! whitespace-adjacency check: it only needs to track whether the scan
//! position is currently inside a single- or double-quoted string (IDL
//! strings have no escape mechanism, MA12 §2/§4, so this is a plain
//! two-state toggle, not a harder problem) so that a `;` **inside** a
//! string literal (`PRINT, 'a;b'`) is not mistaken for a comment-opener.
//! Every physical line has its trailing comment (if any) stripped by this
//! function *before* it is ever measured for the size cap, appended to
//! `self.buffer`, or tokenized for its own bracket/`$`/block-keyword
//! content -- mirroring `q-repl`'s own "blank before anything else touches
//! `line`" discipline, adapted to IDL's simpler (unconditional) comment
//! rule and its own two quote styles.
//!
//! ## Why every join uses a space, never a real newline
//!
//! A first version of this scanner joined `BEGIN...ENDxxx`/paren/bracket
//! continuations with a real `'\n'` (reasoning that `block_body`'s own
//! `statement_line` production has an *optional* trailing `NEWLINE`, so
//! injecting one seemed harmless) and reserved the space-join only for `$`.
//! That is also wrong: `idl.grammar`'s expression cascade has **no**
//! tolerance for a stray `NEWLINE` token appearing where an operator
//! expects its next operand -- splitting `PRINT, (1 + 2` / `+ 3)` across two
//! fed lines and joining with `'\n'` injects a real `NEWLINE` token between
//! `2` and `+`, which is a genuine parse error there (confirmed directly by
//! this crate's own test suite: `continues_across_an_open_paren`/
//! `continues_across_an_open_bracket` failed under the real-newline join
//! and pass under the space join). A single space is safe for **every**
//! continuation reason at once: it can never merge two adjacent tokens
//! (each physical line's own leading/trailing whitespace already gives
//! tokens breathing room), it never introduces a token of its own (`skip:
//! WHITESPACE` swallows it), and -- once comments are already stripped per
//! line (see above) -- `block_body`'s own optional-`NEWLINE` shape parses a
//! space-joined multi-statement block exactly the same as one written with
//! real newlines.
//!
//! ## Delegating to the real IDL lexer, not a hand-rolled character scan
//!
//! Exactly like `q-repl`, this scanner tokenizes each **newly fed,
//! comment-stripped physical line** (not the whole accumulated buffer --
//! see "Incremental, not whole-buffer, scanning" below) with the real
//! `coding_adventures_idl_lexer::try_tokenize_idl` and counts genuine
//! `LPAREN`/`RPAREN`/`LBRACKET`/`RBRACKET` tokens plus the block-opening/
//! -closing `KEYWORD` tokens (`BEGIN`/`PRO`/`FUNCTION` vs. `END`/`ENDIF`/
//! `ENDELSE`/`ENDFOR`/`ENDWHILE`/`ENDREP`), and checks whether the
//! fragment's own last real token is `CONTINUATION`. A bracket/keyword
//! character sitting inside a string literal is never even tokenized as
//! one, so it can never be miscounted by a naive raw scan.
//!
//! A tokenize *failure* on one fragment (a genuinely unrecognized
//! character -- the only way `try_tokenize_idl` can fail) is treated as
//! "not incomplete": this scanner does not (and, since IDL strings have no
//! escape mechanism and no multi-line form, MA12 §2/§4, cannot usefully)
//! try to guess whether more input would fix it -- it falls through to
//! evaluation immediately, letting the real lex/parse error surface through
//! [`Interpreter::feed`]'s own `Result`, rather than waiting forever.
//!
//! ## Incremental, not whole-buffer, scanning (avoiding the O(n²) bug class)
//!
//! [`IdlRepl`] keeps **running** `(open_parens, open_brackets, block_depth)`
//! counts as instance state, updated by tokenizing only the *newly
//! appended* line on each [`IdlRepl::feed`] call and folding that
//! fragment's own token deltas into the persisted counts -- **not** by
//! re-tokenizing the whole accumulated buffer from scratch every call. This
//! is the same fix `q-repl`'s own `apply_line_bracket_tokens` documents
//! (a prior, whole-buffer-re-tokenizing version of that scanner cost
//! O(buffer length) per call, summing to O(n²) over a continuation that
//! grows one short line at a time) -- applied here from the start rather
//! than discovered as a regression, since this crate is written after that
//! lesson was already paid for once.
//!
//! ## The push-before-size-check ordering (the other lesson already paid for once)
//!
//! This repo already fixed a "the continuation buffer's cap is checked
//! *after* growing the buffer, so a single oversized line can still exceed
//! the cap before anything notices" bug class once, across
//! `reduce-repl`/`derive-repl`/`apl-repl`/`j-repl` (and `q-repl`'s own
//! `MAX_CONTINUATION_BUFFER` doc comment cites it directly, "task #80").
//! [`IdlRepl::feed`] computes the *prospective* buffer size (current length
//! plus separator plus the new content) and checks it against
//! [`MAX_CONTINUATION_BUFFER`] **before** ever calling `push_str` -- never
//! append-then-check. See [`IdlRepl::feed`]'s own doc comment for the exact
//! ordering.
//!
//! Hand-rolled rather than built on the generic `repl` crate, mirroring
//! `q-repl`'s own rationale: the interpreter is single-threaded, and a
//! console session is sequential anyway.

use coding_adventures_idl_runtime::Interpreter;
use lexer::token::{Token, TokenType};
use std::io::{BufRead, Write};

/// What the REPL should do after being fed one physical line.
#[derive(Debug, PartialEq, Eq)]
pub enum ReplResponse {
    /// Text to display (may be empty -- e.g. after a silent assignment).
    Output(String),
    /// The current statement is incomplete; read another line (`... ` prompt).
    NeedMore,
    /// End the session.
    Quit,
}

/// Upper bound on the pending-continuation buffer (while a bracket/block is
/// still open, or the last line ended in `$`). Mirrors
/// `q_repl::MAX_CONTINUATION_BUFFER`'s identical "generous but bounded"
/// convention and value.
const MAX_CONTINUATION_BUFFER: usize = 64 * 1024;

/// Upper bound on a single *physical* line read from the input stream,
/// applied in [`read_bounded_line`] before [`MAX_CONTINUATION_BUFFER`]'s
/// own check ever runs. Mirrors `q_repl::MAX_LINE_LEN` exactly.
const MAX_LINE_LEN: u64 = 64 * 1024;

/// Read one physical line, bounded to [`MAX_LINE_LEN`] bytes. Byte-for-byte
/// identical algorithm to `q_repl::read_bounded_line` (see that function's
/// own doc comment for the full rationale of every design decision here);
/// this REPL's own line-reading concern is identical to every sibling's --
/// only the *continuation* scanner below is genuinely new to IDL.
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

/// Convert a [`Token`]'s 1-based **character** column into a byte offset
/// into `line`, so a token found by tokenizing can be used to slice the
/// original `&str` safely (a `NAME`/`STRING` earlier on the line could
/// contain multi-byte characters, so a plain `column - 1` byte index would
/// be wrong for those; this walks `char_indices` instead of assuming one
/// byte per character).
fn column_to_byte_index(line: &str, column: usize) -> usize {
    let char_idx = column.saturating_sub(1);
    line.char_indices()
        .nth(char_idx)
        .map(|(b, _)| b)
        .unwrap_or(line.len())
}

/// The result of scanning one already-tokenizable physical line fragment.
struct LineScan {
    /// The text to actually append to the accumulated buffer: the
    /// comment-stripped line, or (if it ends in `$`) everything **before**
    /// the `$` --  `$` itself must never reach `tokenize_idl`/`parse_idl`
    /// (it has no grammar production at all, `idl-parser`'s own README),
    /// so it is stripped here, at the point this scanner first recognizes
    /// it.
    content: String,
    /// Join the *next* physical line onto `content` with a single space
    /// (never a real newline -- see this module's own top doc comment,
    /// "Why every join uses a space"). True exactly when this line's own
    /// last real token was `CONTINUATION`.
    join_with_space: bool,
}

/// Find the byte offset where a `;`-comment begins on this raw physical
/// line, if any -- tracking single-/double-quoted string state so a `;`
/// **inside** a string literal (`PRINT, 'a;b'`) is not mistaken for a
/// comment-opener. IDL strings have no escape mechanism (MA12 §2/§4), so
/// this is a plain two-state toggle, not the harder whitespace-adjacency
/// problem Q's own `/` needs -- see this module's own top doc comment,
/// "Comments must still be stripped per physical line."
fn find_comment_start(line: &str) -> Option<usize> {
    let mut in_single = false;
    let mut in_double = false;
    for (i, c) in line.char_indices() {
        match c {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            ';' if !in_single && !in_double => return Some(i),
            _ => {}
        }
    }
    None
}

/// Strip a trailing `;`-comment (if any) from one physical line, returning
/// everything before it. See [`find_comment_start`]'s own doc comment.
fn strip_trailing_comment(line: &str) -> &str {
    match find_comment_start(line) {
        Some(idx) => &line[..idx],
        None => line,
    }
}

/// Strip `line`'s own trailing comment, tokenize what remains with the real
/// IDL lexer, and fold its bracket/block-keyword tokens into the
/// *persisted* running counts -- see this module's own top doc comment,
/// "Incremental, not whole-buffer, scanning."
///
/// `BEGIN`/`PRO`/`FUNCTION` each open one level of block depth; the generic
/// `END` and every matched terminator (`ENDIF`/`ENDELSE`/`ENDFOR`/
/// `ENDWHILE`/`ENDREP`) each close one -- mirroring `idl.grammar`'s own
/// documented "a bare END always also closes any block" rule, so this
/// scanner does not need to track *which* opener a given closer is meant to
/// match, only the aggregate depth.
fn scan_line(line: &str, parens: &mut i32, brackets: &mut i32, block_depth: &mut i32) -> LineScan {
    let code = strip_trailing_comment(line);
    let tokens = match coding_adventures_idl_lexer::try_tokenize_idl(code) {
        Ok(t) => t,
        // A genuinely unrecognized character: don't touch the running
        // counts, and don't try to guess this is "incomplete" -- fall
        // through to evaluation so the real error surfaces.
        Err(_) => {
            return LineScan {
                content: code.to_string(),
                join_with_space: false,
            }
        }
    };
    let real: Vec<&Token> = tokens
        .iter()
        .filter(|t| t.type_ != TokenType::Eof)
        .collect();
    for t in &real {
        match t.effective_type_name() {
            "LPAREN" => *parens += 1,
            "RPAREN" => *parens = (*parens - 1).max(0),
            "LBRACKET" => *brackets += 1,
            "RBRACKET" => *brackets = (*brackets - 1).max(0),
            "KEYWORD" => match t.value.as_str() {
                "BEGIN" | "PRO" | "FUNCTION" => *block_depth += 1,
                "END" | "ENDIF" | "ENDELSE" | "ENDFOR" | "ENDWHILE" | "ENDREP" => {
                    *block_depth = (*block_depth - 1).max(0)
                }
                _ => {}
            },
            _ => {}
        }
    }
    if let Some(last) = real.last() {
        if last.effective_type_name() == "CONTINUATION" {
            let byte_idx = column_to_byte_index(code, last.column);
            return LineScan {
                content: code[..byte_idx].to_string(),
                join_with_space: true,
            };
        }
    }
    LineScan {
        content: code.to_string(),
        join_with_space: false,
    }
}

/// A persistent interactive IDL session.
pub struct IdlRepl {
    interp: Interpreter,
    buffer: String,
    /// Running open-paren/open-bracket/`BEGIN...ENDxxx`-depth counts for the
    /// CURRENT continuation, updated incrementally (see this module's own
    /// top doc comment).
    open_parens: i32,
    open_brackets: i32,
    block_depth: i32,
}

impl Default for IdlRepl {
    fn default() -> Self {
        Self::new()
    }
}

impl IdlRepl {
    pub fn new() -> Self {
        IdlRepl {
            interp: Interpreter::new(),
            buffer: String::new(),
            open_parens: 0,
            open_brackets: 0,
            block_depth: 0,
        }
    }

    /// `>> ` for a fresh statement, `... ` while continuing an incomplete
    /// one.
    pub fn prompt(&self) -> &'static str {
        if self.buffer.is_empty() {
            ">> "
        } else {
            "... "
        }
    }

    pub fn is_continuing(&self) -> bool {
        !self.buffer.is_empty()
    }

    fn reset_continuation_state(&mut self) {
        self.buffer.clear();
        self.open_parens = 0;
        self.open_brackets = 0;
        self.block_depth = 0;
    }

    /// Feed one physical input line (without its trailing newline).
    ///
    /// # The buffer-size-cap ordering (checked BEFORE appending, not after)
    ///
    /// See this module's own top doc comment, "The push-before-size-check
    /// ordering." The prospective total size (current buffer length, plus
    /// the join separator, plus the new content) is computed and checked
    /// against [`MAX_CONTINUATION_BUFFER`] *before* this method ever calls
    /// `push_str` -- so the buffer can never even momentarily hold more
    /// than the cap allows.
    ///
    /// # Precondition: `line` is a single physical line (no embedded `'\n'`)
    ///
    /// True by construction at this crate's own sole call site ([`run`],
    /// which always strips a physical line's own trailing newline before
    /// calling `feed`) -- asserted here (debug-only) so a future direct
    /// caller violating it fails loudly rather than silently misbehaving.
    pub fn feed(&mut self, line: &str) -> ReplResponse {
        debug_assert!(
            !line.contains('\n'),
            "IdlRepl::feed expects a single physical line without an embedded newline"
        );
        if self.buffer.is_empty() {
            match line.trim() {
                "quit" | "exit" => return ReplResponse::Quit,
                _ => {}
            }
        }

        let scan = scan_line(
            line,
            &mut self.open_parens,
            &mut self.open_brackets,
            &mut self.block_depth,
        );

        let separator_len = if self.buffer.is_empty() { 0 } else { 1 };
        if self
            .buffer
            .len()
            .saturating_add(separator_len)
            .saturating_add(scan.content.len())
            > MAX_CONTINUATION_BUFFER
        {
            self.reset_continuation_state();
            return ReplResponse::Output(format!(
                "Error: statement exceeds the {MAX_CONTINUATION_BUFFER}-byte continuation limit; discarded\n"
            ));
        }

        if self.buffer.is_empty() {
            self.buffer.push_str(&scan.content);
        } else {
            // Every continuation join uses a single SPACE, never a real
            // newline -- not just for the `$` case (which obviously must
            // not inject a `NEWLINE` token into the middle of a still-open
            // expression), but for the open-paren/bracket and
            // `BEGIN...ENDxxx` cases too: a real `'\n'` character here
            // becomes a genuine, significant `NEWLINE` token (IDL's own
            // grammar, unlike whitespace, does NOT skip it), and nothing in
            // `idl.grammar`'s expression cascade tolerates a `NEWLINE`
            // appearing where an operator expects its next operand (e.g.
            // splitting `(1 + 2` / `+ 3)` across two fed lines would
            // otherwise inject a `NEWLINE` between `2` and `+`, a genuine
            // parse error). `block_body`'s own `statement_line` production
            // has an OPTIONAL trailing `NEWLINE` precisely so a space-joined
            // multi-statement block parses identically to one written with
            // real newlines (confirmed directly against `idl.grammar`'s own
            // `statement_line = statement { STMT_SEP statement } [ NEWLINE ]
            // | NEWLINE` rule) -- so using one join strategy everywhere is
            // not just simpler than special-casing by continuation reason,
            // it is the only choice that is correct for every reason at
            // once.
            self.buffer.push(' ');
            self.buffer.push_str(&scan.content);
        }

        let still_incomplete = scan.join_with_space
            || self.open_parens > 0
            || self.open_brackets > 0
            || self.block_depth > 0;
        if still_incomplete {
            return ReplResponse::NeedMore;
        }

        let src = std::mem::take(&mut self.buffer);
        self.open_parens = 0;
        self.open_brackets = 0;
        self.block_depth = 0;
        if src.trim().is_empty() {
            return ReplResponse::Output(String::new());
        }
        match self.interp.feed(&format!("{src}\n")) {
            Ok(text) => ReplResponse::Output(text),
            Err(e) => ReplResponse::Output(format!("Error: {e}\n")),
        }
    }
}

/// Drive a full interactive IDL session over the given reader and writer.
pub fn run<R: BufRead, W: Write>(mut reader: R, mut writer: W) -> std::io::Result<()> {
    let mut repl = IdlRepl::new();
    writeln!(writer, "IDL (on array-runtime) — type quit to exit.")?;

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
    fn a_complete_one_line_statement_never_spuriously_waits() {
        let mut r = IdlRepl::new();
        assert!(matches!(r.feed("PRINT, 1 + 2"), ReplResponse::Output(t) if t.trim() == "3"));
    }

    #[test]
    fn assignment_is_silent_bare_expression_auto_prints() {
        let mut r = IdlRepl::new();
        assert_eq!(r.feed("x = 5"), ReplResponse::Output(String::new()));
        assert!(matches!(r.feed("x"), ReplResponse::Output(t) if t.trim() == "5"));
    }

    // ── Paren/bracket continuation ───────────────────────────────────────

    #[test]
    fn continues_across_an_open_paren() {
        let mut r = IdlRepl::new();
        assert_eq!(r.feed("PRINT, (1 + 2"), ReplResponse::NeedMore);
        assert!(r.is_continuing());
        assert!(matches!(r.feed(" + 3)"), ReplResponse::Output(t) if t.trim() == "6"));
        assert!(!r.is_continuing());
    }

    #[test]
    fn continues_across_an_open_bracket() {
        let mut r = IdlRepl::new();
        assert_eq!(r.feed("a = [1, 2,"), ReplResponse::NeedMore);
        assert!(matches!(r.feed(" 3]"), ReplResponse::Output(t) if t.is_empty()));
        assert!(matches!(r.feed("PRINT, a"), ReplResponse::Output(t) if t.trim() == "1 2 3"));
    }

    #[test]
    fn mismatched_bracket_types_stay_incomplete_on_their_own_open_count() {
        // A bracket opened, then a paren "closed" with nothing open -- must
        // stay incomplete on the strength of the still-open BRACKET, the
        // stray RPAREN forgiven (clamped to 0), not carried negative.
        let (mut parens, mut brackets, mut depth) = (0i32, 0i32, 0i32);
        scan_line("[)", &mut parens, &mut brackets, &mut depth);
        assert_eq!((parens, brackets, depth), (0, 1, 0));

        let mut r = IdlRepl::new();
        assert_eq!(r.feed("a = [)"), ReplResponse::NeedMore);
    }

    // ── `$` continuation -- the genuinely new scanner concern ────────────

    #[test]
    fn dollar_continuation_joins_the_next_line_as_one_logical_line() {
        let mut r = IdlRepl::new();
        assert_eq!(r.feed("PRINT, 1 + $"), ReplResponse::NeedMore);
        assert!(r.is_continuing());
        assert!(matches!(r.feed("2"), ReplResponse::Output(t) if t.trim() == "3"));
        assert!(!r.is_continuing());
    }

    #[test]
    fn dollar_continuation_can_chain_across_more_than_two_lines() {
        let mut r = IdlRepl::new();
        assert_eq!(r.feed("PRINT, 1 + $"), ReplResponse::NeedMore);
        assert_eq!(r.feed("2 + $"), ReplResponse::NeedMore);
        assert!(matches!(r.feed("3"), ReplResponse::Output(t) if t.trim() == "6"));
    }

    #[test]
    fn dollar_continuation_does_not_inject_a_real_newline() {
        // A real newline mid-expression would break IDL's own significant-
        // NEWLINE statement grammar ("1 +\n2" is NOT a valid statement);
        // this confirms the join is a plain space, not '\n'.
        let mut r = IdlRepl::new();
        assert_eq!(r.feed("x = 1 + $"), ReplResponse::NeedMore);
        assert!(matches!(r.feed("2"), ReplResponse::Output(t) if t.is_empty()));
        assert!(matches!(r.feed("PRINT, x"), ReplResponse::Output(t) if t.trim() == "3"));
    }

    // ── `BEGIN...ENDxxx` block-depth continuation ────────────────────────

    #[test]
    fn continues_across_a_multi_line_if_then_begin_endif() {
        let mut r = IdlRepl::new();
        assert_eq!(r.feed("IF 1 GT 0 THEN BEGIN"), ReplResponse::NeedMore);
        assert!(r.is_continuing());
        assert_eq!(r.feed(" y = 1"), ReplResponse::NeedMore);
        assert!(matches!(r.feed("ENDIF"), ReplResponse::Output(t) if t.is_empty()));
        assert!(!r.is_continuing());
        assert!(matches!(r.feed("PRINT, y"), ReplResponse::Output(t) if t.trim() == "1"));
    }

    #[test]
    fn continues_across_a_multi_line_for_loop() {
        let mut r = IdlRepl::new();
        assert_eq!(r.feed("total = 0"), ReplResponse::Output(String::new()));
        assert_eq!(r.feed("FOR i = 1, 3 DO BEGIN"), ReplResponse::NeedMore);
        assert_eq!(r.feed(" total = total + i"), ReplResponse::NeedMore);
        assert!(matches!(r.feed("ENDFOR"), ReplResponse::Output(t) if t.is_empty()));
        assert!(matches!(r.feed("PRINT, total"), ReplResponse::Output(t) if t.trim() == "6"));
    }

    #[test]
    fn continues_across_a_multi_line_pro_definition() {
        let mut r = IdlRepl::new();
        assert_eq!(r.feed("PRO greet, name"), ReplResponse::NeedMore);
        assert_eq!(r.feed(" PRINT, name"), ReplResponse::NeedMore);
        assert!(matches!(r.feed("END"), ReplResponse::Output(t) if t.is_empty()));
        assert!(!r.is_continuing());
        assert!(matches!(r.feed("greet, 'world'"), ReplResponse::Output(t) if t.trim() == "world"));
    }

    #[test]
    fn continues_across_nested_if_else_begin_blocks() {
        let mut r = IdlRepl::new();
        assert_eq!(r.feed("IF 0 GT 1 THEN BEGIN"), ReplResponse::NeedMore);
        assert_eq!(r.feed(" y = 1"), ReplResponse::NeedMore);
        assert_eq!(r.feed("ENDIF ELSE BEGIN"), ReplResponse::NeedMore);
        assert_eq!(r.feed(" y = 2"), ReplResponse::NeedMore);
        assert!(matches!(r.feed("ENDELSE"), ReplResponse::Output(t) if t.is_empty()));
        assert!(matches!(r.feed("PRINT, y"), ReplResponse::Output(t) if t.trim() == "2"));
    }

    // ── Comments: unconditional `;`, but STILL must be stripped per line ────

    #[test]
    fn a_stray_bracket_and_dollar_inside_a_comment_do_not_fool_the_scanner() {
        // `;` is an UNCONDITIONAL comment marker in idl.tokens (no
        // whitespace-adjacency hook, unlike Q's `/`) -- a bracket or `$`
        // character after it is never tokenized as a real token at all.
        let mut r = IdlRepl::new();
        match r.feed("PRINT, 1 + 1 ; a stray ( and $ in a comment") {
            ReplResponse::Output(t) => assert_eq!(t.trim(), "2"),
            other => panic!("expected an immediate result, not NeedMore -- got {other:?}"),
        }
        assert!(!r.is_continuing());
    }

    /// Regression for the bug this module's own top doc comment documents
    /// under "Comments must still be stripped per physical line": a `;`
    /// comment opened on one physical line of a still-open continuation
    /// must NOT swallow a LATER line's own closing bracket once the two are
    /// space-joined into one buffer (the comment's own regex has no real
    /// `'\n'` left to stop at in the joined buffer). Before
    /// `strip_trailing_comment` was applied per-line, this returned a
    /// parse error (the `;` had eaten the closing `)` along with the rest
    /// of the first line) instead of completing the statement.
    #[test]
    fn a_comment_opened_mid_continuation_does_not_swallow_the_rest_of_the_statement() {
        let mut r = IdlRepl::new();
        assert_eq!(r.feed("PRINT, (1 ; comment"), ReplResponse::NeedMore);
        assert!(r.is_continuing());
        match r.feed("+ 2)") {
            ReplResponse::Output(t) => assert_eq!(t.trim(), "3"),
            other => panic!(
                "expected the statement to complete and evaluate to 3, got {other:?} \
                 (a comment on the first line must not swallow the second line)"
            ),
        }
        assert!(!r.is_continuing());
    }

    #[test]
    fn a_semicolon_inside_a_string_literal_is_not_mistaken_for_a_comment() {
        assert_eq!(strip_trailing_comment("PRINT, 'a;b'"), "PRINT, 'a;b'");
        assert_eq!(
            strip_trailing_comment("PRINT, 'a' ; real comment"),
            "PRINT, 'a' "
        );
        let mut r = IdlRepl::new();
        assert!(matches!(r.feed("PRINT, 'a;b'"), ReplResponse::Output(t) if t.trim() == "a;b"));
    }

    // ── DoS guards: buffer cap, checked BEFORE appending ─────────────────

    #[test]
    fn an_unbounded_continuation_is_discarded_not_grown_forever() {
        let mut r = IdlRepl::new();
        assert_eq!(r.feed("PRINT, (1"), ReplResponse::NeedMore);
        let filler = "+1".repeat(MAX_CONTINUATION_BUFFER / 2 + 10);
        match r.feed(&filler) {
            ReplResponse::Output(t) => assert!(t.contains("Error")),
            other => panic!("expected an Error output once the cap is exceeded, got {other:?}"),
        }
        assert!(!r.is_continuing());
        assert!(matches!(r.feed("PRINT, 1+1"), ReplResponse::Output(t) if t.trim() == "2"));
    }

    #[test]
    fn buffer_cap_is_checked_before_appending_not_after() {
        // A direct regression test for the exact bug class this repo
        // already paid down once (task #80): a single line whose length
        // ALONE already exceeds the cap must be caught on THIS call, with
        // the buffer never having held the oversized content at all.
        let mut r = IdlRepl::new();
        let _ = r.feed("PRINT, (");
        let oversized = "+1".repeat(MAX_CONTINUATION_BUFFER);
        let response = r.feed(&oversized);
        assert!(matches!(response, ReplResponse::Output(t) if t.contains("Error")));
        assert!(
            !r.is_continuing(),
            "buffer must be cleared, not left oversized"
        );
    }

    // ── Incremental (not O(n^2)) scanning cost ───────────────────────────

    #[test]
    fn per_line_scanning_cost_does_not_scale_with_the_existing_buffer_size() {
        // Each filler line is a minimal one-character comment (`;`) so that
        // `n` iterations' own total contribution to the buffer stays small
        // relative to `MAX_CONTINUATION_BUFFER` even on top of the ~60 KiB
        // starting buffer the "large" scenario below front-loads --
        // otherwise the cap itself (working exactly as intended) would
        // trip partway through the loop and confound this timing
        // comparison with an unrelated `NeedMore`-vs-`Error` mismatch.
        fn time_n_filler_lines(r: &mut IdlRepl, n: usize) -> std::time::Duration {
            let start = std::time::Instant::now();
            for _ in 0..n {
                assert_eq!(r.feed(";"), ReplResponse::NeedMore);
            }
            start.elapsed()
        }

        let n = 500;

        let mut small = IdlRepl::new();
        assert_eq!(small.feed("PRINT, (0"), ReplResponse::NeedMore);
        let small_time = time_n_filler_lines(&mut small, n);

        let mut large = IdlRepl::new();
        assert_eq!(large.feed("PRINT, (0"), ReplResponse::NeedMore);
        let padding = format!("; {}", "x".repeat(60_000));
        assert_eq!(large.feed(&padding), ReplResponse::NeedMore);
        let large_time = time_n_filler_lines(&mut large, n);

        let ratio = large_time.as_secs_f64() / small_time.as_secs_f64().max(1e-6);
        assert!(
            ratio < 5.0,
            "feeding {n} trivial filler lines took {large_time:?} against a ~60 KiB \
             pre-existing buffer vs {small_time:?} against a small one ({ratio:.1}x) -- \
             per-line cost should not scale with the EXISTING buffer size"
        );

        match small.feed(")") {
            ReplResponse::Output(t) => assert_eq!(t.trim(), "0"),
            other => panic!("expected the statement to close cleanly to 0, got {other:?}"),
        }
    }

    // ── quit/exit, errors, sessions ───────────────────────────────────────

    #[test]
    fn quit_commands() {
        assert_eq!(IdlRepl::new().feed("quit"), ReplResponse::Quit);
        assert_eq!(IdlRepl::new().feed("exit"), ReplResponse::Quit);
    }

    #[test]
    fn errors_are_shown_not_fatal() {
        let mut r = IdlRepl::new();
        assert!(
            matches!(r.feed("PRINT, undefined_var"), ReplResponse::Output(t) if t.contains("Error"))
        );
        assert!(matches!(r.feed("PRINT, 1+1"), ReplResponse::Output(t) if t.trim() == "2"));
    }

    #[test]
    fn session_persists_across_lines() {
        let mut r = IdlRepl::new();
        r.feed("a = 10");
        assert!(matches!(r.feed("PRINT, a + 5"), ReplResponse::Output(t) if t.trim() == "15"));
    }

    #[test]
    fn run_drives_a_session_to_eof() {
        let input = "a = 3\nPRINT, a*2\nquit\n".as_bytes();
        let mut output = Vec::new();
        run(input, &mut output).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains('6'));
    }

    #[test]
    fn run_handles_a_dollar_continuation_end_to_end() {
        let input = "PRINT, 1 + $\n2\nquit\n".as_bytes();
        let mut output = Vec::new();
        run(input, &mut output).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains('3'), "expected 1+2==3 in output, got: {text}");
    }

    #[test]
    fn run_handles_a_multi_line_pro_definition_end_to_end() {
        let input = "PRO greet, name\n PRINT, name\nEND\ngreet, 'hi'\nquit\n".as_bytes();
        let mut output = Vec::new();
        run(input, &mut output).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("hi"), "expected greet output in: {text}");
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
    fn run_reports_an_oversized_line_cleanly_and_keeps_the_session_alive() {
        let oversized = "+".repeat(MAX_LINE_LEN as usize * 2);
        let input = format!("{oversized}\nPRINT, 1+1\nquit\n");
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
}
