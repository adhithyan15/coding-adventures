# Changelog

## [0.1.0] - 2026-07-28

### Added

- Initial `axiom-repl` crate + the `axiom` binary (MA-13d, front2 Wave 7):
  an interactive Read-Eval-Print loop over `axiom-runtime`'s `AxiomSession`,
  mirroring `derive-repl`'s structural precedent adapted to Axiom's own
  confirmed numbered prompt (`(n) ->`, MA13 §5) rather than Derive's `#n:`.
- Line continuation across `(`/`[` bracket depth, with a small state
  machine skipping `--` line comments and `"..."` string contents before
  counting brackets — Axiom, unlike Derive, has both comments and strings
  in its lexical surface this cut, so a naive bracket-only heuristic would
  misfire on a bracket character inside either.
- `)quit` (plus `quit`/`QUIT` convenience spellings) and Ctrl-D end the
  session; a surface error prints and the session continues.
- Two known REPL bug classes, checked and fixed from day one (not
  reintroduced): (1) push-before-size-check ordering — the accumulation
  buffer's prospective size is checked *before* `push_str`, matching the fix
  already applied in `reduce-repl`/`derive-repl`/`apl-repl`/`j-repl`; (2)
  unbounded single-physical-line read before the continuation-buffer check —
  `run` reads through a `read_bounded_line` capped at 64 KiB rather than
  `BufRead::read_line` directly, matching the fix already applied in
  `j-repl`/`apl-repl`. Both are covered by regression tests.
- 30+ unit tests covering single/multi-line continuation (parens, brackets,
  blocks, string/comment-aware bracket skipping), prompt switching, quit
  words, error recovery, persistent bindings/declared-domain enforcement
  across lines, both bug-class regressions, and an end-to-end `run` driver.
