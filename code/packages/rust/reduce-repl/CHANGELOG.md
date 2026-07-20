# Changelog

## [Unreleased]

### Fixed

- **`ReduceRepl::feed` now bounds `self.buffer` *before* growing it, not
  after** — the previous order always ran `self.buffer.push_str(line)` +
  `push('\n')` first and only then checked `self.buffer.len() <=
  MAX_INPUT_LEN`, so a single, caller-supplied `line` that was itself
  already oversized was fully copied into `self.buffer` (an O(n) copy,
  possibly reallocating) before the check meant to bound it ever ran, and
  the oversized submission relied on `ReduceSession::feed`'s own internal
  length guard to eventually reject it. Found while propagating
  `maple-repl::MapleRepl::feed`'s identical `/security-review` fix (MP-4,
  #8647) to this crate's sibling REPLs — this crate's own shipped
  `read_bounded_line` never feeds `feed` a `line` longer than
  `MAX_LINE_LEN`, but `feed` is a `pub fn` on a `pub struct`, so any other
  embedder calling it directly with an attacker-supplied, unbounded `&str`
  needed the same bound at this layer too, not just at the one shipped
  call site. The buffer is now never grown past the cap; an oversized
  `line` is rejected immediately (buffer cleared, `"input too large:
  exceeds the {MAX_INPUT_LEN}-byte limit"`) without ever reaching
  `ReduceSession::feed`.

## [0.1.0] - 2026-07-18

### Added

- Initial `reduce-repl` crate (MA08 R-4, front2 Wave 5): an interactive
  Read-Eval-Print loop and the `reduce` binary, wrapping a persistent
  `ReduceSession` (`reduce-runtime`) — mirroring `derive-repl`'s driver,
  per MA08 §2/§5's explicit template pointer.
- A **plain, non-numbered** prompt (`"> "`, continuation `"... "`) — MA08
  §2/§5 are explicit that Reduce's own session transcript has no
  numbered-input convention the way Derive's `#n:` or Wolfram's `In[n]:=`
  do, so (unlike `derive-repl`) no result is ever prefixed with an index.
- `(`/`)`-, `{`/`}`- (Reduce's list braces, MA08 §3 — not Derive's `[`/`]`),
  and `<<`/`>>`-aware (Reduce's group-statement delimiters, MA08 §3)
  bracket-depth line continuation, with the `<<`/`>>` two-character pair
  matched by explicit lookahead so a bare comparison `<`/`>` never falsely
  triggers continuation — the one genuinely new tracking case relative to
  `derive-repl`'s simpler `(`/`[` pair.
- Case-insensitive `QUIT`/`EXIT` to end the session, carried forward as a
  REPL-level convenience (not a language feature — real REDUCE's own exit
  command is the semicolon-terminated procedure call `quit;`).
- Bounded single-physical-line reads (`read_bounded_line`, 64 KiB cap),
  carrying forward the `derive-repl`/`j-repl`/`apl-repl` `/security-review`
  fix ("cap unbounded single-line read before continuation-buffer check")
  rather than reintroducing the same gap in a new REPL.
- 21 tests: prompt/continuation behaviour (parens, braces, group-statement
  delimiters, and the `<<`/`>>`-vs-bare-comparison disambiguation), quit
  words, error recovery, persistent bindings, an end-to-end Reduce
  program, and the full `read_bounded_line` regression suite (oversized
  line, exact-cap boundary, multi-chunk overflow drain, multibyte-character
  straddle).
