# Changelog

## [0.1.0] - 2026-11-08

### Added
- Extracted standalone `@coding-adventures/warrant-compiler` package with zero runtime dependencies.
- Guarded recursive-descent Warrant parser with node quotas and recursion guards (VER04c).
- Exact bounded layout capture with synthetic INDENT/DEDENT/NEWLINE tokens (VER04b).
- Pure AST model evaluator supporting integers, booleans, sequences, arrays, and tuples.
- Verification runner for examples and universal claims with integrated PRNG generator algebra.
- Standalone differential test suite against canonical `warrant-syntax-v1` corpus and `ct-compare.warrant`.
