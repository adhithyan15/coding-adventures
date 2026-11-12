# Changelog

## Unreleased

### Changed

- `BUILD_windows` now runs the test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` path for the `aes` library) instead of a recipe ending in a trailing backslash that the build-tool validator rejects.

### Fixed

- `BUILD_windows` passes dependency library paths with `-I` instead of `set PERL5LIB=...`, which MSYS perl on CI splits on `:` and so never found.

- **Windows build.** New `BUILD_windows`: `set "PERL5LIB=..\aes\lib;%PERL5LIB%" && prove -l -v t\`.
  The shared BUILD's `PERL5LIB=$(cd ../aes && pwd)/lib:... perl Makefile.PL` cannot run under `cmd /C`.

## 0.01 — 2026-04-12

### Added
- ECB mode encryption and decryption (INSECURE, educational)
- CBC mode encryption and decryption with PKCS#7 padding
- CTR mode encryption and decryption (no padding)
- GCM authenticated encryption and decryption with GHASH
- PKCS#7 padding and unpadding utilities
- GF(2^128) multiplication for GHASH
- Comprehensive test suite with NIST SP 800-38A and GCM spec vectors
