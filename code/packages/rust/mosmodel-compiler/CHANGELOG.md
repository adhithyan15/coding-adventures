# Changelog — mosmodel-compiler

## [0.1.1] — 2026-05-10

### Changed

- File extension for mosaic interface source files changed from `.mosmodel` to
  `.mil` (mosaic interface language) throughout all documentation and specs.
  Companion extensions `.moslayout` → `.mll` and `.mosstyle` → `.msl` also
  updated for consistency.  No API or grammar changes; purely a naming
  simplification.

## [0.1.0] — 2026-05-07

### Added

- Initial implementation of the mosmodel component interface language compiler.
- `tokenize()` — tokenizes `.mil` source text into `Vec<Token>` using the
  embedded `mosmodel.tokens` grammar via `GrammarLexer`.
- `compile()` — full pipeline: tokenize → parse → analyze → validate → emit.
- `MosmodelComponent` IR — typed representation of a component's slots and emits.
- `SlotDecl`, `EmitDecl`, `EmitParam` — typed IR nodes.
- `SlotType` enum — text, number, bool, image, color, node, list<T>, Component(name).
- `EmitPayloadType` enum — text, number, bool, color, Component(name).
  (image and node excluded per spec §2: events carry data, not rendered subtrees.)
- `SlotDefault` enum — Text, Number, Bool inline defaults.
- `validate()` — semantic validation: unique names, name conflicts between slots
  and emits, type-compatible defaults, no defaults on non-defaultable types.
- `emit_descriptor_json()` — serializes the component to interface descriptor JSON
  (consumed by moslayout and mosstyle compilers).
- `emit_rust_binding()` — generates a Rust struct binding for the Metal/paint-vm
  backend with builder-pattern methods for every slot and emit.
- Embedded parser grammar in `_grammar.rs` (both token grammar and parser grammar)
  following the auto-generated pattern from `grammar-tools`.
- 34 unit tests covering: all lexer token types, happy-path compilation for
  Button / Grid / FormulaBar, all semantic validation error cases, and utility
  functions.
- 1 doctest for the `compile()` API.
