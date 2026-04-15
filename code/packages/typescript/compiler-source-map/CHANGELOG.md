# Changelog — @coding-adventures/compiler-source-map

## [0.1.0] — 2026-04-11

### Added

- `SourcePosition` interface: file, line (1-based), column (1-based), length
- `sourcePositionToString()`: formats as "file:line:col (len=N)"
- `SourceToAst` class: Segment 1, maps source positions → AST node IDs
  - `add(pos, astNodeId)`, `lookupByNodeId(id) → SourcePosition | null`
- `AstToIr` class: Segment 2, maps AST node IDs → IR instruction ID arrays
  - `add(astNodeId, irIds[])`, `lookupByAstNodeId()`, `lookupByIrId()`
- `IrToIr` class: Segment 3 (one per optimizer pass), maps IR IDs → optimized IR IDs
  - `addMapping(originalId, newIds[])`, `addDeletion(originalId)`
  - `lookupByOriginalId()`, `lookupByNewId()`
  - `deleted: Set<number>` for optimised-away instructions
- `IrToMachineCode` class: Segment 4, maps IR IDs → machine code byte offsets
  - `add(irId, mcOffset, mcLength)`, `lookupByIrId()`, `lookupByMCOffset()`
- `SourceMapChain` class: the full pipeline sidecar
  - `addOptimizerPass(segment)` for appending optimizer segments
  - `sourceToMC(pos)` — composite forward query
  - `mcToSource(mcOffset)` — composite reverse query
- Comprehensive test suite with >95% coverage

### Implementation notes

- TypeScript uses `null` returns where Go uses `nil` pointer returns
- Field names use camelCase (e.g., `astNodeId` vs Go's `AstNodeID`)
- The `deleted` set uses `Set<number>` instead of Go's `map[int]bool`
