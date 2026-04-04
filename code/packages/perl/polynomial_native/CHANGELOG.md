# Changelog — CodingAdventures::PolynomialNative (Perl)

## [0.01] — 2026-04-03

### Added

- Initial release: Perl XS extension wrapping the Rust `polynomial` crate
  via `perl-bridge`.
- Boot function: `boot_CodingAdventures__PolynomialNative`.
- Exposed XSUBs: `normalize`, `degree`, `zero`, `one`, `add`, `subtract`,
  `multiply`, `evaluate`.
- Polynomials passed as Perl array references.
- Written by hand (no XS toolchain); `xs_init!` macro avoided due to
  `concat_idents` not being in stable Rust.
