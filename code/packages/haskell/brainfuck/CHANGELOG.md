# Changelog

## Unreleased

- `BUILD_windows` runs under `cmd.exe`: the POSIX `sh` form (`command -v`, `/dev/null`) failed there, so it now uses the repo's `if exist "%ProgramFiles%\cabal\bin\cabal.exe" (...)` idiom.

## 0.1.0

- Add a Haskell Brainfuck tokenizer/parser package for compiler pipeline
  convergence.
