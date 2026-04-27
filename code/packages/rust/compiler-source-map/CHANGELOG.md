# Changelog — compiler-source-map

## [0.1.0] — 2026-04-11

Initial release: Rust port of the `compiler-source-map` Go package.

### Added

- `SourcePosition` struct with file, line, column, length fields and `Display` trait
- `SourceToAst` segment: maps source text positions to AST node IDs
  - `add(pos, ast_node_id)` for recording mappings
  - `lookup_by_node_id(id)` for reverse lookups
- `AstToIr` segment: maps AST node IDs to IR instruction IDs (one-to-many)
  - `add(ast_node_id, ir_ids)` for recording one-to-many mappings
  - `lookup_by_ast_node_id(id)` and `lookup_by_ir_id(id)` for both directions
- `IrToIr` segment: maps IR instruction IDs across optimiser passes
  - `add_mapping(original_id, new_ids)` for instruction transformations
  - `add_deletion(original_id)` for optimised-away instructions
  - `lookup_by_original_id()` and `lookup_by_new_id()` for both directions
  - `pass_name` field for identifying which optimiser pass produced this segment
- `IrToMachineCode` segment: maps IR IDs to machine code byte offsets
  - `add(ir_id, mc_offset, mc_length)` for recording machine code mappings
  - `lookup_by_ir_id()` and `lookup_by_mc_offset()` for both directions
- `SourceMapChain` — the central sidecar that flows through the entire pipeline
  - `source_to_mc(pos)` — forward composite query: source position → MC offsets
  - `mc_to_source(offset)` — reverse composite query: MC offset → source position
  - `add_optimizer_pass(segment)` for accumulating optimiser pass segments
- 21 unit tests + 1 doc test (100% pass rate)
