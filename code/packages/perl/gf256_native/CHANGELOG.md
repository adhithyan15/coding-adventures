# Changelog — CodingAdventures::GF256Native (Perl)

## [Unreleased]

### Changed

- `BUILD_windows` now builds the Rust cdylib with the host MSVC toolchain, stages `GF256Native.<dlext>` under `lib/auto/CodingAdventures/GF256Native/` and runs `prove` against it through `code/scripts/check-perl-extension-windows.ps1` (native Strawberry Perl, `perl-bridge` compiles its C shim and links the interpreter's CORE import library). It replaces the "Perl import library not available" skip.

### Added

- `t/01-basic.t`: real tests for every XSUB (`add`, `subtract`, `multiply`, `divide`, `power`, `inverse`, plus the zero-divisor `die` path), run by both `BUILD` and `BUILD_windows` in place of the old load-only smoke line.

## [0.01] — 2026-04-03

### Added

- Initial release: Perl XS extension wrapping the Rust `gf256` crate via
  `perl-bridge`.
- Boot function: `boot_CodingAdventures__GF256Native`.
- Exposed XSUBs: `add`, `subtract`, `multiply`, `divide`, `power`, `inverse`.
- Elements passed as Perl integers (0–255).
- Division-by-zero and inverse-of-zero guarded with `catch_unwind` → Perl `die`.
