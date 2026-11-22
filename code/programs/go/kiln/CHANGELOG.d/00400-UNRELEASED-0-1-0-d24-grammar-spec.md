## [0.1.0] - UNRELEASED

### Changed (spec only, D24)

- KILN04 section 4.9: the strace syntax layer moves onto the repo grammar
  toolchain (`code/grammars/strace`, shared `strace-lexer`/`strace-parser`).
  Kiln keeps all policy. Adds fail-closed parsing with reasons mapped from the
  failing token, new `size` caps (depth 64, 65,536 tokens per line), the
  divergence policy with an empty divergence table, the differential-testing
  gates, the 8 MB/s and 3x flip gate, and the eight-PR split.
- KILN00 section 17 and this README: Kiln may depend on in-repo Go modules
  (wired with `replace`), never on third-party modules; no hand-written
  lexers or parsers for Kiln's own inputs (user directive, 2026-11-16; also
  covers KILN01).
- No code changes.
