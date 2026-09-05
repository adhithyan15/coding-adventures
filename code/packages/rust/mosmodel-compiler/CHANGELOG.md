# Changelog — mosmodel-compiler

## [Unreleased]

### Added — `one-of`, a closed-set slot type (UI49, #14036)

```
slot variant : one-of primary secondary danger ;
```

A keyword axis used to be declared `slot variant : text` with its legal values
in a **comment**, so nothing could validate a value, reject a typo, or
enumerate the set to check that a stylesheet or a story covered it. Six toolkit
components shipped `variant`/`size` slots whose values were accepted and
silently discarded.

`SlotType::OneOf(Vec<String>)` carries the values in declaration order. A
repeated value is rejected rather than deduplicated, because it is almost
certainly a typo and it would make "does this cover every value?" ambiguous for
anything consuming the set.

Ordering in the grammar matters and is commented at the rule: `one_of_type`,
`list_type`, and `scalar_type` all begin with `KEYWORD`, so the longer
productions must be tried first. `slot_type` getting this wrong is exactly the
defect that would have broken all 72 `list<>` declarations in the repository
(#14067).

The host-facing type is a string on every backend today, except TypeScript,
where a closed set lowers to a **union** (`"primary" | "secondary" | "danger"`)
— so passing an undeclared variant is a compile error in the generated host
rather than a value ignored at runtime. Lowering to native enums elsewhere is
UI49 open question 2.

## [0.1.2] — 2026-07-14

### Fixed — recursion-depth guard against native stack overflow (DoS)

`compile` built its `GrammarParser` with no recursion-depth cap, even
though `mosmodel-compiler` is reachable via the `mosaic` CLI on arbitrary
`.mil` files — a real, not theoretical, attack surface. Deeply-nested
`list<list<list<...>>>` slot-type input would recurse until it overflowed
the native thread stack — an uncatchable process abort — before this
crate's own `Result`-returning entry points ever got a chance to report
anything.

Measured (binary search, uncapped parser, the true default per-test-thread
stack — no `RUST_MIN_STACK` override, no explicit `Builder::stack_size`,
matching what `cargo test` and a production caller both actually get —
debug build, adversarial 5000-level input): safe through 289 rule-frames,
crashes at 290. Added a bespoke `MAX_RULE_DEPTH = 200` — about 31% below
that floor — and wired it into `compile` via `.with_max_depth(...)`.

- Added `MAX_RULE_DEPTH: usize = 200` and wired it into `compile`.
- 3 new regression tests: deep adversarial input on an enlarged-stack
  thread returns a clean `Err`, input at the measured real-nesting
  boundary (97 levels) still parses while one level past it doesn't, and
  the cap trips before the native stack would overflow even on a
  default-stack thread.

No change to behaviour for any input that nests below the cap.

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
