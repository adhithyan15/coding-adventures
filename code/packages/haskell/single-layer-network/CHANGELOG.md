# Changelog

## Unreleased

- `BUILD_windows` now runs `cabal test` unconditionally. It used to run Cabal only if
  `%ProgramFiles%\cabal\bin\cabal.exe` existed, which it never does on the CI runner, so the
  tests were silently skipped on Windows.
- Add `BUILD_windows` (the repo's `cmd.exe` idiom, `if exist "%ProgramFiles%\cabal\bin\cabal.exe" (...)`), so Windows CI no longer runs the POSIX `sh` `BUILD` through `cmd /C`.
