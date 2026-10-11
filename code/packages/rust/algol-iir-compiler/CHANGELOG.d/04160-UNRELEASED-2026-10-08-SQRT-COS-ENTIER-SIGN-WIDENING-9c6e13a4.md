## [0.416.0] - UNRELEASED

- Preserve runtime-real formatter provenance through `sqrt(cos(...))` when built-in `entier` normalizes a nonnegative unit-bounded built-in `sign`-rooted result.
- Restrict the shared pre-cosine `sqrt` proof to unit-bounded chains, keeping positive but greater-than-one exponential ranges fail-closed.
