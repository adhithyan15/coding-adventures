# Changelog

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
