# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-07-22

### Added

- Initial release — the interactive REPL and the **`q` binary** (item
  MA-11d, spec [`MA11`](../../../specs/MA11-q-language.md)), a sibling of
  `j-repl`/`apl-repl`/`matlab-repl`/`s-repl`/`r-repl` over the `q-runtime`
  interpreter.
- `QRepl` with a persistent workspace, `>> `/`... ` prompts, and `quit`/
  `exit`/EOF handling; `run()` drives a full stdio session.
- **Brace/bracket/paren continuation scanning — the one genuinely new
  concern beyond `j-repl`'s plain paren-balance scanner.** Q has a real
  user-defined block construct (`{[x;y] stmt; stmt}`, MA11 §2/§3 bullet 1)
  that J/APL never had, so a statement can legitimately still be "in
  progress" across a still-open `{`/`[` too, not just `(`. `is_incomplete`
  tracks three *independent* running depths (a mismatched `{)` correctly
  stays incomplete on the strength of the still-open brace, rather than
  two mismatched counts cancelling out in a single combined tally) by
  tokenizing the accumulated buffer with the *real* `q-lexer`
  (`coding_adventures_q_lexer::try_tokenize_q`) — delegating comment-
  awareness to the lexer's own already-correct pre-tokenize hook, rather
  than re-deriving Q's whitespace-sensitive `/`-comment rule a second time
  by hand (which would risk drifting out of sync with `q-lexer`'s own
  rule). Verified by hand-tracing the exact multi-line function-literal
  scenario this crate's own module doc comment describes (`{[x;y]` on one
  line, ` x+y}` on the next), a split parameter-list case (`{[x;` /
  `y] ...}`), a complete one-line statement never spuriously waiting, and a
  stray unbalanced `(` sitting inside a comment not fooling the scanner.
- Continuation lines are joined with a space rather than a real newline
  (mirrors `j-repl`'s identical rationale, generalized to Q's own
  significant `NEWLINE` token between top-level statements).
- `/`-to-end-of-line comments need zero REPL-level handling of their own —
  they're stripped entirely by `q-lexer`'s pre-tokenize hook before this
  crate's scanner ever sees a token for them (see above).
- Errors are surfaced (`Error: …`) without ending the session.
- `MAX_CONTINUATION_BUFFER` (64 KiB) caps the pending-continuation buffer
  while any bracket type is still unbalanced, and `MAX_LINE_LEN` (64 KiB)
  caps a single physical line read from the input stream — both ported
  forward from `j-repl`'s own already-security-reviewed guards (task #80,
  PRs already merged), **including the fixed push-order**: the size cap is
  checked *before* `self.buffer` is grown, never after, so the cap cannot
  be exceeded by up to one line's worth of bytes the way an earlier
  (already-fixed-elsewhere) ordering bug in every sibling REPL once allowed
  — this crate never reintroduces that bug class, it starts from the
  already-corrected ordering.
- `read_bounded_line` is `j-repl`'s own already-reviewed byte-oriented line
  reader (multibyte-boundary-safe, "oversized decided by all three
  conditions" discipline, full-drain-the-oversized-line loop), replicated
  here verbatim — this repl's own line-reading concern is identical to
  J's/APL's; only the bracket-balance scanner above is genuinely new to Q.
