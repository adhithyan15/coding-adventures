## [0.406.0] - UNRELEASED

- Preserve exact runtime-real formatter provenance when built-in `sqrt` maps `ln(exp(...))` over an abs-normalized bounded runtime `sign` chain before `entier`. Signed operands, nested exponentials, user-declared overrides, and non-sign-rooted runtime mappings remain conservative.
