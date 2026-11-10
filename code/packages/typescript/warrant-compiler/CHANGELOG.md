# Changelog

## [Unreleased]

### Fixed
- The canonical `ct-compare.warrant` tests asserted 1 claim and 13 results, but the spec now has 9 claims (VER-P030), so they failed on `main`. They now expect 9 claims and 21 results (12 examples + 9 claims).
- Claim refinements such as `len(a) == len(b)` over two independent `Bytes` binders (accepted ~1 time in 33) exhausted the generator's 100-attempt rejection-sampling cap, which with a fixed seed made `select_picks` fail deterministically with "Generator filter exhausted". The cap is now 1000 (still a hard bound; failure probability ~4e-14 per sample).

## [0.1.0] - 2026-11-08

### Added
- Extracted standalone `@coding-adventures/warrant-compiler` package with zero runtime dependencies.
- Guarded recursive-descent Warrant parser with node quotas and recursion guards (VER04c).
- Exact bounded layout capture with synthetic INDENT/DEDENT/NEWLINE tokens (VER04b).
- Pure AST model evaluator supporting integers, booleans, sequences, arrays, and tuples.
- Verification runner for examples and universal claims with integrated PRNG generator algebra.
- Standalone differential test suite against canonical `warrant-syntax-v1` corpus and `ct-compare.warrant`.
