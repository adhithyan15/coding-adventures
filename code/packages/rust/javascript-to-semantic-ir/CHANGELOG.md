# Changelog

All notable changes to `javascript-to-semantic-ir` are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## 0.1.0

First release — SIR19 milestone **M1 (crate skeleton + literal
lowering)**.

### Added

- Crate skeleton: `Cargo.toml` (path deps on `semantic-ir`,
  `coding-adventures-javascript-parser`, `parser`, `lexer`), `BUILD` /
  `BUILD_windows`, `README.md`, and this changelog.
- Public API:
  - `compile(tree: &GrammarASTNode, module_name: &str) -> Result<Module, JsLowerError>`
    — lower an already-parsed JavaScript CST.
  - `compile_source(source: &str, module_name: &str) -> Result<Module, JsLowerError>`
    — parse (with the `es2020` grammar) then lower, surfacing parse
    failures as `JsLowerError`s with a best-effort `line:column`.
  - `JsLowerError { message, line, column }`
    (`Debug`/`Clone`/`PartialEq`/`Eq`, plus `Display` + `std::error::Error`).
- Literal lowering from the **generic** `GrammarASTNode`:
  - integer-shaped number literal → `IntLit`;
  - decimal / exponent number literal → `FloatLit`;
  - `true` / `false` → `BoolLit`;
  - `null` **and** `undefined` → `NilLit` (the JS distinction is
    intentionally lost in v0);
  - string literal (single- or double-quoted) → `StrLit`.
- Synthesises an exported `main` function whose body's tail value is the
  final top-level literal (or `NilLit` for empty source).
- Stamps `metadata.source_language = "javascript"` and
  `metadata.sir_version = CURRENT_SIR_VERSION`, and emits a feature
  manifest declaring exactly the observed features (`Strings`,
  `Floats`), so every produced module passes `semantic_ir::validate`.
- 17 unit tests: one per literal kind, `compile_source` structural
  checks, a validate round-trip, and error paths (operator expression,
  bare identifier, parse failure, position extractor).

### Deferred

The following are explicitly **out of scope for M1** and currently
return a clear `JsLowerError`. They are scheduled for later milestones,
tracked against the SIR19 spec
(`code/specs/SIR19-javascript-to-semantic-ir.md`):

- **M2 — variables & operators:** variable references (`VarRef`),
  `let`/`const`/`var` (`LetBinding`), re-assignment (`Assign`),
  arithmetic / comparison / logical operators, unary `!`/`-`, loose vs.
  strict equality normalisation.
- **M3 — control flow:** `if`/`else`, `while`, `for` (`ForRange`),
  `for-of` (`ForEach`).
- **M4 — functions & closures:** `function` declarations, arrow
  functions (`MakeClosure`), `return`, calls (`DirectCall` /
  `IndirectCall`), `console.log` → `BuiltinCall("print", …)`.
- **M5 — collections:** array literals (`SeqLit`), indexing
  (`SeqIndex`), `.length` (`SeqLen`), object literals (`MapLit`),
  member/`[]` access (`MapGet`).
- **Template literals** (backtick strings, e.g. `` `a ${x} b` ``):
  deferred — these are a distinct token/rule from plain strings and will
  desugar to `+`-concatenation in a later milestone.
- **Non-decimal numeric forms:** hex (`0x…`), octal (`0o…`), binary
  (`0b…`), and `BigInt` (`10n`) literals are rejected in M1.
- Everything in the SIR19 spec "Out of scope (deferred)" section:
  classes, exceptions, generators, `async`/`await`, destructuring,
  spread/rest, default parameters, ES modules, `eval`, regex.
