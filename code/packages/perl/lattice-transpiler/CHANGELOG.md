# Changelog — CodingAdventures::LatticeTranspiler (Perl)

## [0.1.0] — 2026-03-29

### Added

- Initial implementation of `CodingAdventures::LatticeTranspiler`.
- `transpile($source)` — parse Lattice source and compile to CSS.
  Returns `($css, undef)` on success or `(undef, $error_message)` on failure.
- `transpile_file($path)` — read a file and transpile it.
  Returns `($css, undef)` on success or `(undef, $error_message)` on failure.
- Error handling via `eval{}` wrapping `LatticeParser->parse()` and
  `LatticeAstToCss->compile()` — all exceptions caught and returned as
  error strings.
- Test suite with Test2::V0 covering success path, error path, file
  transpilation, and all major Lattice features end-to-end.
