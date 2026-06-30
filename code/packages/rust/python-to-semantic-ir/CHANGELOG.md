# Changelog

All notable changes to `python-to-semantic-ir` are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to semantic versioning.

## 0.1.0 — 2026-06-30

Milestone **M1**: crate skeleton + literal lowering (SIR17 Python →
Semantic IR frontend, B1).

### Added

- Public API per the SIR17 spec:
  - `compile(tree: &GrammarASTNode, module_name: &str) -> Result<Module, PythonLowerError>`
  - `compile_source(source: &str, module_name: &str) -> Result<Module, PythonLowerError>`
    (parses at Python `"3.10"`, then lowers).
  - `PythonLowerError { message, line, column }` (`Debug`, `Clone`,
    `PartialEq`, `Eq`), with `Display`/`Error` impls.
- Literal lowering, peeling the parser's deep precedence-rule onion
  down to the `atom` token:
  - integer literals → `IntLit` (incl. constant-folded `-7`)
  - float literals → `FloatLit` (declares `Feature::Floats`); incl.
    constant-folded `-2.5`
  - `True` / `False` → `BoolLit`
  - `None` → `NilLit`
  - string literals (single- and double-quoted) → `StrLit` (declares
    `Feature::Strings`); the parser pre-resolves escapes.
- Synthesised `main` function: the final top-level expression becomes
  the block value (or `NilLit` when the program is empty); earlier
  top-level expressions become `ExprStmt`s.
- Manifest declares **exactly** the observed features; module metadata
  records `source_language = "python"` and
  `sir_version = CURRENT_SIR_VERSION`.  Every lowered module passes
  `semantic_ir::validate`.
- 19 unit tests (one per literal kind, top-level structure, validator
  round-trip, and error paths) covering ≥ 90% of the M1 surface.
- Package scaffolding: `Cargo.toml` (path deps on `semantic-ir`,
  `coding-adventures-python-parser`, `parser`, `lexer`), `BUILD` /
  `BUILD_windows`, `README.md`, this changelog.  Added the crate to the
  `code/packages/rust` workspace members list.

### Deferred

Out of scope for M1; each returns a clear
`PythonLowerError("unsupported in M1: <rule>")` so later milestones
slot in at the same site:

- **M2** — variable references (`x`) and assignment (`x = 1`,
  `assign_suffix`), first-occurrence `LetBinding` vs `Assign`.
- **M3** — arithmetic / comparison / boolean operators, control flow
  (`if` / `while` / `for`), unary minus on non-literals.
- **M4** — `def` functions, `lambda`/closures, calls.
- **M5** — sequences, maps, indexing.
- Full SIR17 "out of scope" set: classes, exceptions, generators,
  comprehensions, decorators, multi-target assignment, slicing,
  default/keyword args, string methods, `with`, imports, `async`,
  `global`/`nonlocal`, f-strings.
