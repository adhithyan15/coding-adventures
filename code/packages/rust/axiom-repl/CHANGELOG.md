# Changelog

## [0.1.0] - 2026-07-28

### Added

- Initial `axiom-repl` crate + the `axiom` binary (MA-13d, front2 Wave 7):
  an interactive Read-Eval-Print loop over `axiom-runtime`'s `AxiomSession`,
  mirroring `derive-repl`'s structural precedent adapted to Axiom's own
  confirmed numbered prompt (`(n) ->`, MA13 §5) rather than Derive's `#n:`.
- Line continuation across `(`/`[` bracket depth, with a small state
  machine (`scan_line`) skipping `--` line comments and `"..."` string
  contents before counting brackets — Axiom, unlike Derive, has both
  comments and strings in its lexical surface this cut, so a naive
  bracket-only heuristic would misfire on a bracket character inside
  either. Bracket-depth/open-string state is tracked *incrementally* as
  `AxiomRepl` fields, updated by scanning only each newly-fed physical
  line — not recomputed by rescanning the whole accumulated buffer on every
  line (see the next bullet).
- `)quit` (plus `quit`/`QUIT` convenience spellings) and Ctrl-D end the
  session; a surface error prints and the session continues.
- Three issues checked and fixed from day one (not reintroduced), two of
  them already known bug classes from sibling REPLs' own history, the
  third found in this crate's own security review before merge: (1)
  push-before-size-check ordering — the accumulation buffer's prospective
  size is checked *before* `push_str`, matching the fix already applied in
  `reduce-repl`/`derive-repl`/`apl-repl`/`j-repl`; (2) unbounded
  single-physical-line read before the continuation-buffer check — `run`
  reads through a `read_bounded_line` capped at 64 KiB rather than
  `BufRead::read_line` directly, matching the fix already applied in
  `j-repl`/`apl-repl`; (3) an O(n²) continuation-scan rescanning the entire
  buffer from scratch on every fed line (bounded by `MAX_INPUT_LEN`, so not
  severe, but real wasted CPU work) — fixed by making `scan_line` update
  bracket-depth/open-string state incrementally instead. All three are
  covered by regression tests.
- 32+ unit tests covering single/multi-line continuation (parens, brackets,
  blocks, string/comment-aware bracket skipping, and state carried
  correctly across several separately-fed lines), prompt switching, quit
  words, error recovery, persistent bindings/declared-domain enforcement
  across lines, all three issue regressions, and an end-to-end `run`
  driver.
