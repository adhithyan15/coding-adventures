# Changelog — CodingAdventures::PolynomialNative (Perl)

## [Unreleased]

### Changed

- `BUILD_windows` now builds the Rust cdylib with the host MSVC toolchain, stages `PolynomialNative.<dlext>` under `lib/auto/CodingAdventures/PolynomialNative/` and runs `prove` against it through `code/scripts/check-perl-extension-windows.ps1` (native Strawberry Perl, `perl-bridge` compiles its C shim and links the interpreter's CORE import library). It replaces the "Perl import library not available" skip.

### Added

- `t/01-basic.t`: real tests for the XSUBs (`zero`, `one`, `normalize`, `degree`, `add`, `subtract`, `multiply`, `evaluate`, `divide`, `modulo` and the bad-input `die` paths), run by both `BUILD` and `BUILD_windows` in place of the old load-only smoke line.

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
