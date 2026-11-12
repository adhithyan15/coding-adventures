# Changelog

## Unreleased

- `BUILD_windows` now runs `cabal test all` unconditionally. It used to run Cabal only if
  `%ProgramFiles%\cabal\bin\cabal.exe` existed, which it never does on the CI runner, so the
  tests were silently skipped on Windows.

## 0.1.1 — Unreleased

- Consume the bounded HF04 corpus and validate the complete 128-bit length field without huge allocations.

## 0.1.0

- Initial scaffold.
