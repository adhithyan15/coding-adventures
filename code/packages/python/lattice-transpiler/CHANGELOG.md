# Changelog

## [0.1.0] - 2026-03-22

### Added

- `transpile_lattice()` function: single entry point for Lattice-to-CSS transpilation
- Three-stage pipeline: parse (lattice-parser) -> transform (lattice-ast-to-css) -> emit (CSSEmitter)
- `minified` option for production-ready compressed CSS output
- `indent` option for configurable indentation (default: 2 spaces)
- Error propagation from all pipeline stages (LexerError, GrammarParseError, LatticeError)
