# Changelog

## [0.1.0] - 2026-04-11

### Added

- `SourcePosition` Struct (file, line, column, length) with `to_s` →
  "file:line:col (len=N)" — exact port of Go SourcePosition
- `SourceToAstEntry` and `SourceToAst` (Segment 1) with `add/1` and
  `lookup_by_node_id/1`
- `AstToIrEntry` and `AstToIr` (Segment 2) with `add/2`,
  `lookup_by_ast_node_id/1`, and `lookup_by_ir_id/1`
- `IrToIrEntry` and `IrToIr` (Segment 3) with `add_mapping/2`,
  `add_deletion/1`, `lookup_by_original_id/1`, and `lookup_by_new_id/1`
- `IrToMachineCodeEntry` and `IrToMachineCode` (Segment 4) with `add/3`,
  `lookup_by_ir_id/1` → [offset, length], and `lookup_by_mc_offset/1`
- `SourceMapChain` combining all four segments with:
  - `add_optimizer_pass/1` for appending IrToIr passes
  - `source_to_mc/1` composite forward query (source → machine code)
  - `mc_to_source/1` composite reverse query (machine code → source)
- Comprehensive minitest suite covering all segments and composite queries,
  including optimizer pass threading and deletion tracking
