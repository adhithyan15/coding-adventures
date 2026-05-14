# Changelog — type-declarations

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
