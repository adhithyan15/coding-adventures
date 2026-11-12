# Changelog

## [Unreleased]

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries, with the full compiler, IR, Wasm and VM dependency chain) instead of skipping.

## [0.1.0] - 2026-04-18

### Added

- End-to-end Brainfuck to Wasm orchestration for Perl.
- File-writing helper and pipeline result object with IR, module, and binary
  artifacts.
- Runtime-backed coverage for output, input, and cat-style Brainfuck programs.
