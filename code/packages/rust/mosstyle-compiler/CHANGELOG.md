# Changelog — mosstyle-compiler

## [Unreleased]

### Added - MSL transition declarations

- Added part-level and state-local `transition` declarations with optional
  easing values.
- Duration and easing token references are resolved into a typed
  `StyleTransition` IR and preserved in the backend-neutral style map JSON.
- Generated Lattice emits deterministic comma-separated `transition`
  declarations in authored order.
- Validation now reports `TransitionPropertyNotDeclared` when a transition
  references a property without a base declaration in the same part.

### Added - `unmatched_parts()` to catch style parts that match no layout part

New public `unmatched_parts(&StyleDef, part_map_json) -> Vec<UnmatchedPart>`.
Unlike `validate` — which accepts a sub-path `a/b` as long as its top-level `a`
exists — this reports style parts whose FULL name is not an exact member of the
layout's part map, with a `suggestion` when the sub-path tail is itself an
exported part (the classic stale-naming typo). This is what mosaic-compile uses
to warn when e.g. `Grid.light.msl` writes `sheet/cell` while the composition
exports a flat `cell`: `validate` passed it, but the emitter styled `cell`, so
the light-theme grid rendered with no gridlines and nobody noticed until the web
switcher first rendered the light theme.

### Added - Lattice style emission

- `CompileOutput` now includes `lattice`, a first-class scoped Lattice source
  artifact generated from the resolved `StyleDef`.
- Added `emit_lattice(&StyleDef)` and tests that pass the generated source
  through the Rust Lattice transpiler.
- `CompileOutput` no longer returns CSS. Web callers that need CSS should
  compile `CompileOutput.lattice` through `lattice-transpiler`.

### Added — structural states (`even`, `odd`) for row-stripe sub-parts

- `VALID_STATES` now includes `even` and `odd` alongside the existing
  interaction states (`hover`/`pressed`/`focused`/`disabled`/`selected`/
  `editing`/`error`).
- Unlike interaction states (which depend on input devices), structural
  states resolve from a primitive's child position. Used by Grid's
  `sheet/data-row` sub-part (WA4) to declare alternating row colours:
  `part sheet/data-row { state even { ... } state odd { ... } }`.
- Backwards-compatible: existing `.msl` files don't reference these
  state names; no behaviour change for them.

## [0.1.1] — 2026-07-14

### Fixed — defense-in-depth recursion-depth cap

`parse_style` built its `GrammarParser` with no recursion-depth cap.
Unlike its sibling crates in the mosaic family, tracing every rule in this
grammar (`style_def -> part_def -> part_item -> {state_block |
property_decl} -> style_value`) confirms there is **no recursive shape at
all** — `state_block` only reaches `property_decl` (a terminal), never
back to `part_def`/`state_block`/`style_def`, so the maximum static call
depth is fixed (~5 rule-frames) regardless of input size. There is no
adversarial deep-nesting DoS vector to calibrate against here.

Added `MAX_RULE_DEPTH` set to the shared crate's generic
`parser::grammar_parser::DEFAULT_MAX_RULE_DEPTH` (128) anyway, for
defense-in-depth and consistency with the rest of the mosaic family — at
25x the grammar's real maximum call depth, it can never reject a
legitimate mosstyle file. One new regression test confirms a style file
with 200 flat (non-nested) parts still parses cleanly under the cap.

## [0.1.0] — 2026-05-11

### Added

- Initial implementation of the `.msl` (Mosaic Style Language) compiler.
- `TokenGrammar` and `ParserGrammar` embedded in `_grammar.rs` (no runtime file I/O).
- `tokenize()` — wraps `GrammarLexer`; resolves DIMENSION, HASH_COLOR, TOKEN_REF as
  custom token types via `type_name` (not new `TokenType` variants — they map to
  `TokenType::Name` with `type_name` metadata per the GrammarLexer contract).
- `parse_style()` — wraps `GrammarParser` with the embedded parser grammar.
- `analyze()` — converts the raw `GrammarASTNode` into typed `StyleDef` IR.
- `validate()` — checks part-name existence against the optional part-map JSON.
- `emit_lattice()` - produces scoped Lattice using `.mos-{Component}-{part}` class names.
- `compile()` — convenience function running all stages.
- **Default dark token palette** — `$color-surface`, `$color-text-primary`, etc. resolve
  to hex literals at compile time (UI15 §1 palette).
- **State selectors**: `hover`→`:hover`, `pressed`→`:active`, `focused`→`:focus-visible`,
  `disabled`/`selected`/`editing`/`error` → class-name selectors.
- Unit tests cover tokenizer, Lattice emission, token resolution, state blocks, and validation.

### Grammar

```
file          = style_def ;
style_def     = KEYWORD NAME LBRACE { part_def } RBRACE ;
part_def      = KEYWORD NAME LBRACE { part_item } RBRACE ;
part_item     = state_block | property_decl ;
state_block   = KEYWORD NAME LBRACE { property_decl } RBRACE ;
property_decl = NAME COLON style_value SEMICOLON ;
style_value   = TOKEN_REF | HASH_COLOR | DIMENSION | NUMBER | STRING | NAME ;
```

### Bug fixes

- `GrammarParser::new` takes `Vec<Token>` and `ParserGrammar` by value (not by reference).
  Fixed incorrect `&tokens` and `&grammar` borrows in `parse_style()`.
- `TokenType::Lbrace` → `TokenType::LBrace` (correct capitalisation in test assertion).
