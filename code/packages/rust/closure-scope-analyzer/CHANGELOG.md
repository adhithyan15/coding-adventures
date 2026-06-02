# Changelog

All notable changes to the `coding-adventures-closure-scope-analyzer` crate will be documented in this file.

## [0.2.0] - 2026-06-02

### Added — CLOC13.0 minimal analyzer body

Replaces the v0.1.0 identity-style empty `analyze` with a real walk of `program.body` that surfaces **top-level declarations**:

- `VariableDeclaration` (`var` / `let` / `const`) — one `Binding` per `VariableDeclarator`, with `VarKind → BindingKind` mapping (`Var → Var`, `Let → Let`, `Const → Const`). Multi-declarator forms (`const a = 1, b = 2;`) emit one binding each.
- `FunctionDeclaration` — one `Binding` with `kind = Function` carrying the function name.
- All bindings land in `ScopeId::GLOBAL`. The global scope's `bindings` list mirrors the global table.
- `declared_at` is populated from the AST's `Identifier.cv` when CV tracing is on (otherwise stays `None`).
- `BindingTarget::Identifier` is the only Phase 1 variant; the match is total today. When Phase 3 adds destructuring patterns, they'll need their own arms.

### Activates 5 consumer passes simultaneously

This PR is the wire-then-activate completion. The five pass bodies (CLOC13.A..E, PRs #4766, #4773, #4775, #4777, #4778) all consume `bindings` via the analyzer's public surface. They went from "candidate scan finds zero" to "candidate scan finds real top-level decls" with **zero PR-side churn** — no rebases, no API changes.

`changed = false` is still hard-pinned in every consumer pass. Lighting up real bindings only makes the candidate scans non-empty; the apply step (CLOC13.{A,B,C,D,E}.1) is a per-pass follow-up.

### Deferred to CLOC13.0.1 (tracked inline in `fn analyze`)

1. **Function body scopes.** A `FunctionDeclaration` should create a `ScopeKind::Function` child scope holding params + nested decls. Today we only emit the function's name binding in `GLOBAL`.
2. **Block scopes.** `let`/`const` inside a `BlockStatement` should land in a `ScopeKind::Block` child scope. Today nested blocks are ignored.
3. **Var hoisting.** A `var x` inside a block must bind in the enclosing *function* scope. Pre-walk pattern documented inline.
4. **`References`.** Identifier use sites should produce `Reference`s. Today the references vec is empty. This is the biggest remaining gap — `remove-unused-vars` and `inline` both gate on `uses == 0` / `uses == 1`, so zero references reads as "every binding is unused".
5. **Catch-clause scope** (not in Phase 1 AST yet).
6. **Strict-mode binding semantics** (function-in-block scope).

### Tests added (7 new; 13 total, was 6)

- `top_level_let_surfaces_as_binding_in_global` — end-to-end pin of the binding shape.
- `top_level_var_let_const_map_to_three_kinds` — `VarKind → BindingKind` mapping.
- `top_level_function_declaration_surfaces` — function-name binding shape.
- `multi_declarator_emits_one_binding_per_declarator` — `const a = 1, b = 2;` form.
- `binding_ids_are_dense_and_monotonic` — `BindingId(0), (1), (2)` contract.
- `statement_items_are_skipped_for_now` — pin the deferred-Statement-walk contract; will fail when CLOC13.0.1 starts collecting references.
- `references_are_empty_in_cloc13_0` — pin the deferred-References contract.

All 6 v0.1.0 tests still pass unchanged.

### Bumped 0.1.0 → 0.2.0

Signature of `analyze` is unchanged; the v0.1.0 contract holds. Consumers don't need to recompile against a new API.

## [0.1.0] - 2026-06-01

### Added — CLOC13 unblocker: scaffold + stable API surface

First commit. Ships the **types and the public entry function** that
the five Phase-1 optimisation passes consume:

- `ScopeId`, `BindingId` — opaque newtype handles into the dense
  vectors on `ScopeAnalysis`.
- `Scope { kind, parent, bindings }`, `ScopeKind::{Global, Function,
  Block}`.
- `Binding { name, kind, scope, declared_at }`, `BindingKind::{Var,
  Let, Const, Function, Class, Param}`.
- `Reference { name, from_scope, binding, cv }` — one per identifier
  use site, with the resolved binding (`None` = free global).
- `ScopeAnalysis { scopes, bindings, references }` — the analysis
  output.  Has a `resolve(name, from_scope)` convenience that walks
  the parent chain.
- `analyze(program) -> ScopeAnalysis` — entry function.

**Identity body.** The v0.1.0 `analyze` returns a single global scope
with no bindings or references — i.e., it doesn't yet walk the AST.
The full traversal lands as a follow-up (tracked under CLOC13.0).
This split is deliberate: the **API surface** is the unblocker for the
five consumer passes, so freezing the contract here lets CLOC13.A
(rename), CLOC13.B (inline), CLOC13.C (treeshake), CLOC13.D
(collapse-properties), and CLOC13.E (remove-unused-vars) all proceed
as parallel work streams.

Tests cover: identity-body shape, `ScopeId::GLOBAL == 0`, `resolve`
on empty analysis, `resolve` walking the parent chain, innermost-shadow
wins, and full serde round-trip.

### Rationale

Why a separate crate (rather than putting the analysis in
`closure-pass-pipeline` or `javascript-ast`):

1. **AST stays backend-agnostic.** `javascript-ast` is shared with the
   future V8-on-LANG-VM clone; scope analysis is Closure-specific.
2. **Pipeline stays scheduling-only.** `closure-pass-pipeline` runs
   passes; it doesn't bake in any one pass's data structures.
3. **One build per pipeline run.** Five passes consuming a shared
   analysis beats five passes each rebuilding their own.
4. **Serialisable.** Newtype-wrapped IDs (not pointers) so the
   analysis can dump to a sidecar JSON for the CV pipeline.
