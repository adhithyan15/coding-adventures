# LANG71 — TW05-P Part 2: Multi-Module Import Propagation

## Motivation

After LANG70 (TW05-P Part 1), all six data-model modules
(`span.tw`, `token.tw`, `ast.tw`, `iir-types.tw`, `diagnostic.tw`,
`iir-builder.tw`) run under `(typed strict)`.  Five modules remain
in `(typed lenient)`:

| Module | Imports |
|--------|---------|
| `compiler/lexer` | `compiler/span`, `compiler/token` |
| `compiler/cst-parser` | `compiler/token` |
| `compiler/parser` | `compiler/cst-parser`, `compiler/token`, `compiler/ast`, `compiler/span` |
| `compiler/emit` | `compiler/span`, `compiler/ast`, `compiler/iir-types`, `compiler/iir-builder` |
| `compiler/main` | all nine other modules |

These modules call names exported by their dependencies — `make-span`,
`Token`, `TkInteger?`, `token-kind`, `IntLit?`, `emit-program`, etc.
The type checker currently checks each module in isolation: it sees
`make-span` as an unresolved variable because it was declared in
`compiler/span`, not in the module under check.

## Solution

Extend `twig-type-checker` with two new public functions:

### `extract_module_exports`

```rust
pub fn extract_module_exports(
    program: &Program,
    env: &TypeEnv,
) -> HashMap<String, TwigKind>
```

Given a program whose `module_info.exports` list is populated and its
already-built `TypeEnv`, returns a `HashMap` containing only the
exported names (name → kind).  The caller uses this to build the
`extra_globals` seed for dependent modules.

### `check_program_with_globals`

```rust
pub fn check_program_with_globals(
    program: &Program,
    mode_override: Option<TypedMode>,
    extra_globals: &HashMap<String, TwigKind>,
) -> TypeCheckResult<TypedProgram>
```

Identical to `check_program` except that the `TypeEnv` is pre-seeded
with `extra_globals` before `collect_forms` runs Pass 1.  This allows
a calling harness (module driver, IDE, tests) to propagate exported
names from already-checked dependency modules into the environment of
the module under check.

## Cross-module `match` constraint

The five remaining modules intentionally avoid `(match …)` on union
values imported from other modules — variant integer tags are not
propagated by the module driver.  They use predicate functions
(`IntLit?`, `TkInteger?`, etc.) instead.  Because of this, the type
checker does **not** need to propagate `unions` metadata across module
boundaries for exhaustiveness checking; propagating `globals` is
sufficient.

## Modules converted to `(typed strict)`

All five modules are converted once the import propagation is in place:

1. `compiler/lexer` — `(typed lenient)` → `(typed strict)`
2. `compiler/cst-parser` — `(typed lenient)` → `(typed strict)`
3. `compiler/parser` — `(typed lenient)` → `(typed strict)`
4. `compiler/emit` — `(typed lenient)` → `(typed strict)`
5. `compiler/main` — `(typed lenient)` → `(typed strict)`

## Version

`twig-type-checker`: 0.8.0 → 0.9.0

## Tests (`tw05p2_tests`, 5 new)

| Test | Verifies |
|------|---------|
| `lexer_tw_strict_with_imported_globals` | `lexer.tw` OK in strict after span+token exports seeded |
| `cst_parser_tw_strict_with_imported_globals` | `cst-parser.tw` OK in strict after token exports seeded |
| `parser_tw_strict_with_imported_globals` | `parser.tw` OK in strict after cst-parser+token+ast+span exports seeded |
| `emit_tw_strict_with_imported_globals` | `emit.tw` OK in strict after span+ast+iir-types+iir-builder exports seeded |
| `main_tw_strict_with_imported_globals` | `main.tw` OK in strict after all 9 dependency exports seeded |

## Files changed

| File | Change |
|------|--------|
| `code/packages/rust/twig-type-checker/src/lib.rs` | Add `extract_module_exports`, `check_program_with_globals`, `tw05p2_tests` |
| `code/packages/rust/twig-type-checker/Cargo.toml` | 0.8.0 → 0.9.0 |
| `code/packages/rust/twig-type-checker/CHANGELOG.md` | prepend [0.9.0] |
| `code/packages/twig/compiler/lexer.tw` | `(typed lenient)` → `(typed strict)` |
| `code/packages/twig/compiler/cst-parser.tw` | `(typed lenient)` → `(typed strict)` |
| `code/packages/twig/compiler/parser.tw` | `(typed lenient)` → `(typed strict)` |
| `code/packages/twig/compiler/emit.tw` | `(typed lenient)` → `(typed strict)` |
| `code/packages/twig/compiler/main.tw` | `(typed lenient)` → `(typed strict)` |
| `code/specs/LANG71-tw05p2-import-propagation.md` | new (this file) |
