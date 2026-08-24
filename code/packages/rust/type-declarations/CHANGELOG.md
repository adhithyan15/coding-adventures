# Changelog — type-declarations

## Unreleased — 2026-08-18

Brought under CI: the crate had no `BUILD` file, so nothing ever ran its tests
or linted it.

### Added

- **`BUILD` file — this crate is now built, tested and linted in CI.**

  This crate is a member of the `code/packages/rust` workspace, so it compiled
  whenever a sibling with a `BUILD` file pulled it in as a path dependency. But
  the build tool discovers work by scanning for `BUILD` files, so with none of
  its own it was never a package in its own right: its **test targets were never
  compiled, its assertions never ran, and `cargo clippy --all-targets -- -D
  warnings` never linted it**, on any platform. Adding `BUILD` puts it under the
  same per-package clippy gate and test run as every other watched Rust crate.

  The BUILD is the repo-standard one-liner, `cargo test -p type-declarations -- --nocapture`,
  kept on a single line: the build tool runs each BUILD line as its own
  `sh -c`, so a backslash continuation would silently truncate the command.
  It was verified green locally first — clippy `-D warnings` clean and a full
  unfiltered `cargo test --no-fail-fast` passing — per the "expect to find
  existing breakage when you start watching a long-unwatched package" rule in
  `lessons.md`.

## [0.1.0] — 2026-05-14

### Added (LANG50 — Generic Grammar Type Checker)

Initial release.  Pure data crate — no dependencies.

- `TypeDeclarations` — container for named types, global bindings, and typed
  mode; analogous to a TypeScript `.d.ts` file.
- `KindDecl` enum: `Int`, `Bool`, `Nil`, `Symbol`, `Str`, `List`,
  `Named(String)`, `Function { arity }`, `Any`.
- `KindDecl::to_iir_hint() -> &'static str` — maps to the IIR `type_hint`
  strings understood by `jit-core` and `aot-core`:
  `Int → "i64"`, `Bool → "bool"`, `Str → "str"`, `Function → "closure"`,
  everything else → `"any"`.
- `KindDecl::is_concrete_hint() -> bool` — true when the kind maps to a
  non-`"any"` IIR hint.
- `TypedModeDecl` enum: `Off`, `Lenient`, `Strict`.
- `NamedTypeDecl` enum: `Record { fields }`, `Union { variants }`,
  `Alias { target }`.
- `FieldDecl { name, kind }`, `VariantDecl { name, fields }`.
- `TypeDeclarations::resolve` — follows alias chains (depth-limited to 32,
  returns `Any` on cycle).
- `TypeDeclarations::union_variants` — returns variant names for exhaustiveness
  checking.
- `AnnotatedNode` — `GrammarASTNode`-shaped tree with `KindDecl` on every
  node; the central compilation artifact flowing from type checker into IIR
  emission.
- `AnnotatedNode::iir_hint()` — shorthand for `self.kind.to_iir_hint()`.
- `AnnotatedNode::child_node(rule)`, `node_children()`, `position()`.
- `AnnotatedChild` enum: `Node(AnnotatedNode)`, `Token { text, line, column }`.
- 11 unit tests.
