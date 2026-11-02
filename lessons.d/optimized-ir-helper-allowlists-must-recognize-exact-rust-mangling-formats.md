---
category: Testing & coverage
---

# Optimized IR helper allowlists must recognize exact Rust mangling formats

The AES release-codegen gate admitted only legacy `_ZN` symbols for the
reviewed zeroize and cleanup-abort helpers. Rust 1.99 emitted v0 `_R` symbols
for those same paths, so Linux and macOS CI rejected the expected zeroizer.

Recognize the exact reviewed paths under both formats, validating the optional
v0 crate disambiguator and legacy build hash. Do not accept arbitrary `_R`
symbols or any name merely containing `zeroize_bytes`: that would allow an
unreviewed helper to hide secret-dependent control flow. Exercise real v0
symbols and adversarial foreign-path, suffix, and malformed-hash fixtures.
