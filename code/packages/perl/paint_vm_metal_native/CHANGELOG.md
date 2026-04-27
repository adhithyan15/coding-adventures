# Changelog — CodingAdventures::GF256Native (Perl)

## [0.01] — 2026-04-03

### Added

- Initial release: Perl XS extension wrapping the Rust `gf256` crate via
  `perl-bridge`.
- Boot function: `boot_CodingAdventures__GF256Native`.
- Exposed XSUBs: `add`, `subtract`, `multiply`, `divide`, `power`, `inverse`.
- Elements passed as Perl integers (0–255).
- Division-by-zero and inverse-of-zero guarded with `catch_unwind` → Perl `die`.
