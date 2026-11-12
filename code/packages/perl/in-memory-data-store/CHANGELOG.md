# Changelog

## [Unreleased]

### Changed

- `BUILD_windows` now runs the tests through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths). The previous recipe ran Strawberry's `prove` script under the MSYS perl on PATH and died with `Can't locate TAP/Harness/Env.pm`.

### Fixed

- `BUILD_windows` passes dependency library paths with `-I` instead of `set PERL5LIB=...`, which MSYS perl on CI splits on `:` and so never found.

## 0.1.0 - 2026-07-17

- Add the pure Perl RESP facade for command frames, incremental streams, and
  binary-safe engine responses.
