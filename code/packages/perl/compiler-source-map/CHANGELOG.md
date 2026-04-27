# Changelog — CodingAdventures::CompilerSourceMap

## [0.01] — 2026-04-11

### Added

- Initial Perl port of the Go `compiler-source-map` package.
- `SourcePosition` — character span in a source file: `file`, `line`, `column`, `length` fields, `to_string()`.
- `SourceToAst` — Segment 1 container with `add($pos, $ast_node_id)` and `lookup_by_node_id($id)`.
- `AstToIr` — Segment 2 container with `add($ast_node_id, $ir_ids)`, `lookup_by_ast_node_id($id)`, `lookup_by_ir_id($id)`.
- `IrToIr` — Segment 3 container with `new($pass_name)`, `add_mapping($orig, $new_ids)`, `add_deletion($orig)`, `lookup_by_original_id($id)`, `lookup_by_new_id($id)`.
- `IrToMachineCode` — Segment 4 container with `add($ir_id, $mc_offset, $mc_length)`, `lookup_by_ir_id($id)`, `lookup_by_mc_offset($offset)`.
- `SourceMapChain` — full pipeline sidecar with `new_chain()`, `add_optimizer_pass($seg)`, `source_to_mc($pos)`, `mc_to_source($offset)`.
- `CodingAdventures::CompilerSourceMap` — top-level module loading all sub-modules.
- Comprehensive test suite in `t/compiler_source_map.t` covering all types and composite queries.
