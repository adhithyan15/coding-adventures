# Changelog — compiler_source_map (Elixir)

## 0.1.0 — 2026-04-11

Initial release: Elixir port of the Go `compiler-source-map` package.

### Added

- `SourcePosition` struct with `to_string/1`.
- `SourceToAst` struct with `add/3`, `lookup_by_node_id/2`.
- `AstToIr` struct with `add/3`, `lookup_by_ast_node_id/2`, `lookup_by_ir_id/2`.
- `IrToIr` struct with `new/1`, `add_mapping/3`, `add_deletion/2`,
  `lookup_by_original_id/2`, `lookup_by_new_id/2`.
- `IrToMachineCode` struct with `add/4`, `lookup_by_ir_id/2`, `lookup_by_mc_offset/2`.
- `SourceMapChain` struct with `new/0`, `add_optimizer_pass/2`,
  `source_to_mc/2` (composite forward query), `mc_to_source/2` (composite reverse query).
- Comprehensive ExUnit test suite covering all segments and composite queries.
