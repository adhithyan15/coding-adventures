# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Fixed

- **(MEDIUM, round 2) `apply_line_bracket_tokens` (formerly
  `line_bracket_delta`) computed one net delta per line and clamped the
  running total only once, instead of clamping per token against the
  persisted running state as the original whole-buffer algorithm did.**
  These diverge whenever a line's own tokens, combined with the count
  already open, dip a counter below zero *mid-line* before a later,
  genuinely-unmatched open of the same type — an excess close must be
  "forgiven" (clamped to 0) at the exact point it occurs, not allowed to
  arithmetically cancel a later open within the same line. Concrete
  counterexample: `")("` as a lone first line — per-token clamping
  correctly reports "incomplete" (the `)` is forgiven, then the `(`
  opens); the net-delta approach computed `-1 + 1 = 0` and wrongly
  reported "complete". A realistic multi-line sequence
  (`feed("(1")`, `feed("))+((2")`, `feed(")")`) triggers the same
  divergence one line early. Fixed by threading `&mut i32` references to
  the three running counters straight into the token loop and clamping
  each one immediately after every token that touches it — mathematically
  identical to replaying the whole-buffer algorithm from scratch, since a
  per-token-clamped walk's end state is fully determined by its starting
  value and remaining tokens regardless of where that starting value came
  from. Added two regression tests for the counterexamples above.
- **(LOW/INFO) `blank_line_comment`'s soundness depends on an unenforced
  precondition** (every line reaching `feed` has no embedded `'\n'`) —
  added a `debug_assert!` at the top of `feed` so a future direct caller
  violating it fails loudly in tests/debug builds instead of silently
  reintroducing a scoped version of the comment-swallowing bug below.
- **(HIGH) A `/`-comment opened on one physical line of a still-open
  continuation silently swallowed every subsequently-typed line, forever.**
  The previous scanner tokenized the *whole accumulated, space-joined*
  `self.buffer` on every call — but `q-lexer`'s comment rule blanks from `/`
  through the next **real** `'\n'` or end of input, and this REPL joins
  continuation lines with a single **space**, never a real `'\n'`. A
  comment opened on one physical line therefore had no real `'\n'` left to
  stop at once joined, and silently erased every following line — including
  whatever closing bracket was supposed to complete the statement — leaving
  the session stuck reporting `NeedMore` forever (until the 64 KiB
  continuation cap eventually fired and discarded the whole thing,
  including legitimately-typed program text). `quit`/`exit` couldn't escape
  it either, since those are only recognized when the buffer is empty.
  Concrete repro: `feed("(1 / comment")` then `feed("+2)")` never
  evaluated to `3`. Fixed by [`blank_line_comment`]: each physical line's
  own trailing comment is now blanked to spaces **before** it is folded
  into `self.buffer` at all (not merely accounted for when checking
  completeness) — a comment's real extent is only knowable one physical
  line at a time, before the lossy space-join loses track of where each
  line ended.
- **(MEDIUM) O(n²) cumulative CPU cost from re-tokenizing the whole
  buffer on every fed line.** The previous `is_incomplete` re-tokenized the
  entire accumulated buffer from scratch on every physical line fed while a
  bracket remained open — cost per call scaled with the buffer's current
  length, so cumulative cost across a continuation that grows one short
  line at a time was O(n²) in the number of lines. `QRepl` now tracks
  running `(parens, braces, brackets)` counts as instance state and
  tokenizes only the newly-appended (already comment-blanked) line
  fragment on each call, folding its own delta into the running totals —
  O(line length) per call, O(buffer length) total per continuation, not
  O(buffer length²). Sound because no token in this cut's grammar can span
  a line-fragment boundary (no multi-line string/number literal, MA11 §4).
  Solving both findings with the same per-line pre-processing step (rather
  than two separate patches) falls out naturally: once each line's own
  comment is blanked and tokenized independently, a comment on one line can
  no longer reach past that line's own end, by construction.
- Added regression tests: `a_comment_opened_mid_continuation_does_not_swallow_the_rest_of_the_statement`
  and `a_comment_inside_a_multi_line_function_literal_does_not_swallow_the_closing_brace`
  (Finding 1), and `per_line_scanning_cost_does_not_scale_with_the_existing_buffer_size`
  (Finding 2 — a comparative timing measurement: the same number of
  trivial filler lines against a small vs. a ~60 KiB pre-existing buffer
  must cost approximately the same, not scale with the existing buffer
  size).
- Corrects this file's own earlier (now inaccurate) claim that "comments
  need zero REPL-level handling of their own" — see above; this crate now
  does its own narrow, documented, single-physical-line-scoped comment
  blanking, precisely because whole-buffer delegation to `q-lexer` turned
  out to be unsound for a REPL that joins lines with spaces rather than
  real newlines.

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
