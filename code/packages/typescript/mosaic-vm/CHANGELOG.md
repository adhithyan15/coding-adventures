# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-04

### Added

- `MosaicVM` class — generic depth-first tree walker over `MosaicComponent` IR
- `MosaicRenderer` interface — visitor protocol for code-generation backends:
  `beginComponent`, `endComponent`, `beginNode`, `endNode`, `beginWhen`,
  `endWhen`, `beginEach`, `endEach`, `renderSlotChild`
- `ResolvedValue`, `ResolvedProperty` — normalized property values passed to
  `beginNode`; `MosaicValue.number` normalized to `ResolvedValue.dimension`
- `OutputFile` and `EmitResult` — return types from `endComponent`
- `MosaicVM.run(renderer): EmitResult` — single entry point; fires all visitor
  callbacks in depth-first order then returns the renderer's emitted files
- Support for `when` blocks: `beginWhen(slotName)` / `endWhen()` callbacks
- Support for `each` blocks: `beginEach(slotName, itemName)` / `endEach()` callbacks
- Support for `@slotRef` child nodes: `renderSlotChild(slotName, slotType)` callback
- 31 tests, 100% coverage
