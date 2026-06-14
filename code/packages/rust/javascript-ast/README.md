# coding-adventures-javascript-ast

Backend-agnostic JavaScript AST. The contract between the JS frontend and every
consumer downstream: the Closure-Compiler-clone (typechecker, passes, emitter),
the future V8-in-Rust clone (bytecode lowering), JSDoc and TypeScript
type-extractors, IDE tooling.

Per [CLOC02](../../../specs/CLOC02-javascript-ast.md), the AST holds **only the
syntactic shape**. Spans, type information, optimization metadata, and parent
chains all live in adjacent stores keyed by `CvId` — never on AST nodes
themselves. That's what makes the same AST safe to share across multiple
backends.

## Dependency whitelist

- `coding-adventures-correlation-vector` — for the `CvId` type.
- `coding-adventures-javascript-tokens` — for `EsVersion`.

Nothing else. No `closure-*`, no `type-sidecar`, no IR or bytecode crates. The
whitelist is what keeps the AST reusable.

## What's here in v1 (this crate's first PR)

- [`Program`] — the AST root.
- [`SourceType`] — `Script` or `Module`.
- A `CvId` type alias for `String` (matching the current `correlation-vector`
  representation; see the module docs for the migration plan).

## What's coming (follow-up PRs)

Per CLOC02:

- `Statement`, `Expression`, `Declaration`, `Pattern`, `Class`, `Module`,
  `Literal` variants — landing in their own files (`statement.rs`,
  `expression.rs`, …) so each diff stays reviewable.
- A `walk` visitor pattern over the whole tree.
- Builder helpers.

## Why this is a separate crate

If the AST lived in `javascript-parser`, every consumer would depend on the
parser. Splitting it out keeps the layering clean and lets the future V8 clone
reuse the parser's output without taking on the parser's dependencies.
