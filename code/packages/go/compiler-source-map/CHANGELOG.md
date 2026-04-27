# Changelog

All notable changes to the `compiler-source-map` package will be documented
in this file.

## [0.1.0] — 2026-04-12

### Added

- `SourcePosition` type representing a span of characters in a source file
- `SourceToAst` segment mapping source positions to AST node IDs
- `AstToIr` segment mapping AST node IDs to IR instruction IDs (1:many)
- `IrToIr` segment mapping IR instruction IDs through optimizer passes,
  with support for preserved, replaced, and deleted instructions
- `IrToMachineCode` segment mapping IR instruction IDs to machine code
  byte offsets and lengths
- `SourceMapChain` composing all segments with forward (`SourceToMC`) and
  reverse (`MCToSource`) composite queries
- Bidirectional lookups on every segment type
- Full test suite with 20+ test cases covering identity passes, contraction,
  deletion, multi-pass chains, and incomplete chains
