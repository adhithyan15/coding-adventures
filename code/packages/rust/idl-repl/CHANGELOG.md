# Changelog

## [0.1.0] - 2026-07-24

### Added

- Initial release — **MA-12d** of the IDL frontend (spec
  [`MA12`](../../../specs/MA12-idl-language.md)): an interactive REPL
  (`IdlRepl`) and the `idl` binary, wrapping a persistent
  `coding_adventures_idl_runtime::Interpreter`.
- **The continuation scanner** (MA12 §5, this crate's one genuinely new
  piece, explicitly deferred here by both `idl-lexer` and `idl-parser`):
  tracks, at the raw text level before any chunk is handed to
  `tokenize_idl`/`parse_idl`:
  - **Paren/bracket balance** (`(`/`)`, `[`/`]`).
  - **The `$` line-continuation character** — `idl-lexer` emits
    `CONTINUATION` as an ordinary token and does not swallow the following
    newline; `idl-parser`'s own grammar has no production for it at all
    (a bare `$` is a syntax error there by construction). This crate joins
    the next physical line onto the current one (with the trailing `$`
    itself stripped) whenever the just-scanned line's own last real token
    is `CONTINUATION`.
  - **`BEGIN...ENDxxx`/`PRO`/`FUNCTION` block depth** — `BEGIN`/`PRO`/
    `FUNCTION` each open one level; the generic `END` and every matched
    terminator (`ENDIF`/`ENDELSE`/`ENDFOR`/`ENDWHILE`/`ENDREP`) each close
    one, mirroring `idl.grammar`'s own "a bare `END` always also closes any
    block" rule (no need to track which opener a given closer matches).
  - Delegates to the real `coding_adventures_idl_lexer::try_tokenize_idl`
    (not a hand-rolled character scan) on only the newly fed physical line
    each call, folding that fragment's own token deltas into **persisted**
    running counts — O(line length) per call, O(total input) over an
    entire continuation, not O(n²) from whole-buffer re-tokenization.
- **Comment stripping, applied from the start** (`strip_trailing_comment`/
  `find_comment_start`): although IDL's `;` comment is *unconditional*
  (unlike Q's whitespace-adjacency-dependent `/`), this scanner still must
  strip each physical line's own trailing comment *before* appending it to
  the accumulated buffer or tokenizing it — because continuation lines are
  joined with a single space, never a real `'\n'` (see below), a comment's
  own `/;[^\n]*/` regex would otherwise find no real newline to stop at
  once two lines are joined, silently swallowing every character after it
  including a later line's own closing bracket. Caught by this crate's own
  test suite (`per_line_scanning_cost_does_not_scale_with_the_existing_buffer_size`
  surfaced a genuine parse error instead of a clean statement completion)
  before ever shipping, not discovered as a later regression. Simpler than
  `q-repl`'s own `blank_line_comment` (a plain single-/double-quote
  in/out toggle suffices, since IDL's `;` needs no whitespace-adjacency
  check and its strings have no escape mechanism) but the same *kind* of
  fix, applied with the same discipline (strip before anything else
  touches the line).
- **Every continuation join uses a single space, never a real newline** —
  confirmed necessary (not just sufficient) by this crate's own test
  suite: a real `'\n'` between physical lines injects a genuine, significant
  `NEWLINE` token that `idl.grammar`'s expression cascade has no tolerance
  for between an operator and its next operand (`continues_across_an_open_paren`/
  `continues_across_an_open_bracket` failed under a real-newline join and
  pass under a space join), while `block_body`'s own optional-trailing-
  `NEWLINE` production parses a space-joined multi-statement block
  identically to one written with real newlines (confirmed directly
  against `idl.grammar`).
- The push-before-size-check discipline (`MAX_CONTINUATION_BUFFER`,
  64 KiB): the prospective buffer size (current length + separator + new
  content) is checked *before* ever calling `push_str` — this repo's own
  previously-paid-down "task #80" bug class (`reduce-repl`/`derive-repl`/
  `apl-repl`/`j-repl`, cited directly in `q-repl`'s own doc comment),
  applied here from the start.
- `read_bounded_line` (`MAX_LINE_LEN`, 64 KiB) bounding a single physical
  line read from the input stream, byte-for-byte the same algorithm
  `q-repl`'s own uses.
- The `idl` binary (`src/main.rs`), driving `run()` over stdin/stdout.
- 26 unit tests covering: a complete one-line statement never spuriously
  waiting, silent-assignment/auto-print Implied-Print semantics, paren and
  bracket continuation, mismatched-bracket-type forgiveness, `$`
  continuation (single-hop, chained across 3+ lines, and confirmed to join
  with a space not a real newline), multi-line `IF...THEN BEGIN...ENDIF`,
  a multi-line `FOR` loop, a multi-line `PRO` definition, nested
  `IF...ELSE BEGIN` blocks, a stray bracket/`$` inside a comment not
  fooling the scanner, a comment opened mid-continuation not swallowing
  the rest of the statement (the regression this crate's own test suite
  caught), a `;` inside a string literal not being mistaken for a comment,
  the continuation-buffer size cap (both "discarded once exceeded" and
  "checked before appending, not after"), incremental (not O(n²))
  per-line scanning cost, `quit`/`exit` commands, non-fatal error display,
  session persistence across lines, and full `run()`-level end-to-end
  scenarios (a plain session, a `$` continuation, a multi-line `PRO`
  definition, and an oversized-line report that keeps the session alive).

### Notes

- Hand-rolled rather than built on the generic `repl` crate, mirroring
  `q-repl`'s own rationale: the interpreter is single-threaded, and a
  console session is sequential anyway.
- `idl-lexer` and `idl-parser` were **not** modified.
