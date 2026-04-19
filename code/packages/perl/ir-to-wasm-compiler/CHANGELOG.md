# Changelog

## [0.1.0] - 2026-04-18

### Added

- Conservative lowering from `compiler-ir` programs to Perl Wasm module
  hashrefs.
- Structured loop and if lowering for the IR patterns emitted by the current
  Brainfuck and Nib frontends.
- WASI syscall lowering for the Brainfuck source lane.
- Bounds checks for function parameter counts and data declarations so hostile
  IR cannot force unbounded type or data-segment allocation during lowering.
