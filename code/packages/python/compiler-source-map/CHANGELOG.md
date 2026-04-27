# Changelog — compiler-source-map (Python)

## [0.1.0] — 2026-04-12

### Added

- `SourcePosition` — frozen dataclass representing a span in a source file
  (file, line, column, length). Hashable and immutable.
- `SourceToAst` — Segment 1: maps source text positions to AST node IDs.
  Methods: `add()`, `lookup_by_node_id()`.
- `AstToIr` — Segment 2: maps AST node IDs to IR instruction ID lists
  (one-to-many). Methods: `add()`, `lookup_by_ast_node_id()`,
  `lookup_by_ir_id()`.
- `IrToIr` — Segment 3: maps original IR IDs to optimised IR IDs after
  an optimizer pass. Supports preservation, replacement, and deletion.
  Methods: `add_mapping()`, `add_deletion()`, `lookup_by_original_id()`,
  `lookup_by_new_id()`.
- `IrToMachineCode` — Segment 4: maps IR instruction IDs to machine code
  byte offsets and lengths. Methods: `add()`, `lookup_by_ir_id()`,
  `lookup_by_mc_offset()`.
- `SourceMapChain` — the full pipeline sidecar. Factory method `new()`.
  Methods: `add_optimizer_pass()`, `source_to_mc()`, `mc_to_source()`.
- Entry types: `SourceToAstEntry`, `AstToIrEntry`, `IrToIrEntry`,
  `IrToMachineCodeEntry`.
- Full test suite with >90% coverage.
- Passes `ruff check` with no errors.
