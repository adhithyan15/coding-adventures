# Changelog

## [Unreleased]

### Fixed

- **`DeriveRepl::feed` now bounds `self.buffer` *before* growing it, not
  after** — the previous order always ran `self.buffer.push_str(line)` +
  `push('\n')` first and only then checked `self.buffer.len() <=
  MAX_INPUT_LEN`, so a single, caller-supplied `line` that was itself
  already oversized was fully copied into `self.buffer` (an O(n) copy,
  possibly reallocating) before the check meant to bound it ever ran, and
  the oversized submission relied on `DeriveSession::feed`'s own internal
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
  `DeriveSession::feed`. The numbered `#n: ` prompt still advances for a
  rejected oversized submission exactly as it does for any other
  submission — `self.input_index` increments in the new early-return path
  too, preserving the existing `#1: ` → `#2: ` test expectation.

## [0.1.0] - 2026-07-13

### Added

- Initial `derive-repl` crate (MA07 D-4, front2 Wave 5): an interactive
  Read-Eval-Print loop and the `derive` binary, wrapping a persistent
  `DeriveSession` (`derive-runtime`).
- `#n: ` numbered-worksheet prompt (MA07 §5), `(`/`[`-bracket-depth line
  continuation (no string/comment state to track — this subset has
  neither), and case-insensitive `QUIT`/`EXIT` to end the session.
- Bounded single-physical-line reads (`read_bounded_line`, 64 KiB cap),
  carrying forward the `j-repl`/`apl-repl` `/security-review` fix ("cap
  unbounded single-line read before continuation-buffer check") rather than
  reintroducing the same gap in a new REPL: `BufRead::read_line` alone has
  no length bound, so a single arbitrarily-long line would fully buffer in
  memory before any of this crate's own size checks ran.
- 18 tests: prompt/continuation behaviour, quit words, error recovery,
  persistent bindings, an end-to-end worksheet program, and the full
  `read_bounded_line` regression suite (oversized line, exact-cap boundary,
  multi-chunk overflow drain, multibyte-character straddle).
