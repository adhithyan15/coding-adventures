# CLOC13 — closure-scope-analyzer

> **Status:** v0.1.0 ships the **API surface only** (scaffold).  The
> real `analyze` body lands as a follow-up under CLOC13.0.
>
> **Why this spec exists:** five Phase-1 optimisation passes need scope
> + symbol-table information.  Rather than have each pass build its own
> ad-hoc walker, we ship one analyzer crate and freeze its API
> surface here, so the five consumer passes (CLOC13.A through
> CLOC13.E) can land as **parallel work streams** off this contract.

## The five consumer streams

Once this scaffold is in `main`, these five PRs become eligible
to start in parallel.  They don't conflict because each one edits a
different pass crate:

| Stream      | Pass                                  | Uses from analyzer                        |
|-------------|---------------------------------------|--------------------------------------------|
| CLOC13.A    | `closure-pass-rename`                | scopes, bindings, references (collision)  |
| CLOC13.B    | `closure-pass-inline`                | references (callee free-var reachability) |
| CLOC13.C    | `closure-pass-treeshake`             | references (root reachability scan)       |
| CLOC13.D    | `closure-pass-collapse-properties`   | bindings (alias rewrite safety)           |
| CLOC13.E    | `closure-pass-remove-unused-vars`    | references (use-count per binding)        |

All five consume the **same `ScopeAnalysis` object** — built once by
the pipeline, threaded into each pass.

## Public API surface (locked in v0.1.0)

```rust
pub struct ScopeId(pub u32);   // newtype index into ScopeAnalysis.scopes
pub struct BindingId(pub u32); // newtype index into ScopeAnalysis.bindings

impl ScopeId { pub const GLOBAL: ScopeId; }

pub enum ScopeKind { Global, Function, Block }

pub struct Scope {
    pub kind: ScopeKind,
    pub parent: Option<ScopeId>,
    pub bindings: Vec<BindingId>,
}

pub enum BindingKind { Var, Let, Const, Function, Class, Param }

pub struct Binding {
    pub name: String,
    pub kind: BindingKind,
    pub scope: ScopeId,
    pub declared_at: Option<CvId>,
}

pub struct Reference {
    pub name: String,
    pub from_scope: ScopeId,
    pub binding: Option<BindingId>,   // None = free global
    pub cv: Option<CvId>,
}

pub struct ScopeAnalysis {
    pub scopes:     Vec<Scope>,
    pub bindings:   Vec<Binding>,
    pub references: Vec<Reference>,
}

impl ScopeAnalysis {
    pub fn resolve(&self, name: &str, from: ScopeId) -> Option<BindingId>;
}

pub fn analyze(program: &Program) -> ScopeAnalysis;
```

This contract will not change in v0.2.0 (when the analyzer body
lands).  Consumer passes can pin to it now.

## Design decisions

### Why a separate crate

1. **AST stays backend-agnostic.** `javascript-ast` is shared with the
   future V8-on-LANG-VM clone; lexical scope is a Closure-specific
   concept.
2. **Pipeline stays scheduling-only.** `closure-pass-pipeline` runs
   passes; it doesn't bake in any one pass's data structures.
3. **One build per pipeline run.** Five passes consuming a shared
   analysis beats five passes each rebuilding their own.
4. **Serialisable.** Newtype-wrapped IDs (not pointers) so the
   analysis can dump to a sidecar JSON for the CV pipeline.

### Why IDs, not pointers

Pass crates can hold the `ScopeAnalysis` independently of the
`&Program` borrow.  They walk the analysis to decide WHAT to change,
then walk the program afterward to apply changes.  Pointer-based
references would force every pass to keep the borrow alive across
the entire pass — clashes with the editing step.

### Why one global scope is reserved at `ScopeId(0)`

Every program has exactly one global scope, and consumer passes need
to refer to it without a name lookup.  Fixing it at index 0 means
`ScopeId::GLOBAL` is a `const` and the rename pass's "is this an
externally-visible name?" check is free.

### Why `Reference.binding: Option<BindingId>`

`None` means the lookup walked past the global scope without finding
a match.  Examples: `console.log(x)` from a script-mode program
without a declared `console`.  The treeshake / remove-unused-vars
passes treat `None`-resolved references as **definitely used
externally** — the safe over-approximation.

### Why `Class` is in `BindingKind` but not the AST

The AST will gain `ClassDeclaration` in CLOC09 Phase 1.x once
`closure-typechecker` needs it.  Pre-allocating the `BindingKind`
variant means the analyzer's match exhaustiveness will catch
follow-up work without churning the public API.

## Out of scope for v0.1.0

- The actual traversal of `Program` — `analyze` returns a single
  global scope with no bindings.  Real walk lands under CLOC13.0.
- `with (…)` statement (gap-tracking; `with` is not in Phase 1 AST).
- Module-level imports / exports — those land alongside the
  module-graph crate (separate, not blocking this).
- TDZ enforcement — analyzer reports declarations and references;
  the pass crates decide what to do about hoisted vs TDZ access.

## Test surface (v0.1.0)

- `analyze_returns_global_scope_only` — pinning the scaffold shape.
- `global_scope_id_is_zero` — pinning the `ScopeId::GLOBAL` constant.
- `resolve_in_empty_analysis_returns_none` — null-path safety.
- `resolve_walks_parent_chain_and_finds_outer_binding` — basic
  parent-walk.
- `resolve_innermost_shadow_wins` — name-shadowing rule.
- `analysis_round_trips_via_serde` — wire format stability.

Coverage > 95%.
