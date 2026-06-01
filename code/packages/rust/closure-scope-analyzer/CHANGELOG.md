# Changelog

All notable changes to the `coding-adventures-closure-scope-analyzer` crate will be documented in this file.

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
