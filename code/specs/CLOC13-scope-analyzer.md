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

## CLOC13.0 — minimal analyzer body (v0.2.0, PR #4787)

The first body activation.  Walks `program.body` and surfaces
*top-level* declarations as `Binding`s in `ScopeId::GLOBAL`.  The
public `analyze` signature is unchanged; consumers don't recompile
against a new API.

What's covered:

- **`VariableDeclaration`** (`var`/`let`/`const`).  One `Binding`
  per `VariableDeclarator`, with the `VarKind → BindingKind`
  mapping (`Var → Var`, `Let → Let`, `Const → Const`).  Multi-
  declarator forms (`const a = 1, b = 2;`) emit one binding each.
- **`FunctionDeclaration`**.  One `Binding` with
  `kind = Function` carrying the function name.
- All bindings land in `ScopeId::GLOBAL`.
- `declared_at` is populated from the AST's `Identifier.cv` when
  CV tracing is on.

Deferred to CLOC13.0.1+ (tracked inline in `fn analyze`):

1. **Function-body scopes** — a `FunctionDeclaration` should
   create a `ScopeKind::Function` child scope holding its
   `FunctionParam`s + nested decls.  Today only the function's
   name binding is emitted in `GLOBAL`.
2. **Block scopes** — `let`/`const` inside a `BlockStatement`
   should land in a `ScopeKind::Block` child of the enclosing
   scope.  Today nested blocks are ignored.
3. **Var hoisting** — a `var x` inside a block must bind in the
   enclosing *function* scope, not the block.  Pattern: pre-walk
   the function body to collect `var` declarations, emit them
   against the function scope, then walk normally.
4. **References** — the biggest remaining gap.  Identifier use
   sites should produce `Reference`s.  Today `references` is
   empty.  `remove-unused-vars` and `inline` both gate on
   `uses == 0` / `uses == 1`, so zero references reads as
   "every binding is unused".  CLOC13.0.1's primary job.
5. **Catch-clause scope** (not in Phase 1 AST yet).
6. **Strict-mode binding semantics** (function-in-block scope).

### Failsafe for forward compatibility

The body uses an irrefutable
`let BindingTarget::Identifier(id) = ...;` pattern.  Phase 1
ships only that `BindingTarget` variant, so the destructure is
total today.  When Phase 3 adds destructuring patterns
(`ArrayPattern`, `ObjectPattern`), this becomes a compile error
— exactly the right failsafe.  No silent miscompilation.

## CLOC13.0.1 — references + nested scopes (queued)

The follow-up that closes the biggest gap.  Walks
`Statement::ExpressionStatement` and `FunctionDeclaration.body` to
emit `Reference`s for bare `Identifier` expressions.  Concurrently
introduces nested scopes (Function + Block) so the parent-chain
walk does real work.

### Test the body left behind

The CLOC13.0 PR pinned two contracts that CLOC13.0.1 will need
to update:

- `statement_items_are_skipped_for_now` — will fail when 13.0.1
  starts walking statements.  Update to assert references emerge
  from statement walks.
- `references_are_empty_in_cloc13_0` — same; the test's purpose
  was to make the deferred-References contract fail loudly the
  moment 13.0.1 starts collecting refs.

These were intentionally written to fail.  They're the
breadcrumb trail back to this section.

## CLOC13.{A,B,C,D,E}.1 — per-pass apply steps

Each of the five Phase-1 optimisation passes ships in two parts:

1. **Body PR (CLOC13.{A..E})** — wires the pass to consume
   `ScopeAnalysis`, identifies candidates, hard-pins
   `changed = false`.  All 5 merged: #4766, #4773, #4775, #4777,
   #4778.
2. **Apply-step PR (CLOC13.{A..E}.1)** — lifts the hard-pin,
   actually mutates the program, sets `changed` based on
   genuine mutation count.

The split exists to keep review surface small and to let the 5
body PRs land in parallel against a frozen API without
introducing FixedPoint infinite-loop hazards.

### Apply-step pattern (codified by CLOC13.E.1, PR #4790)

```
1. Restrict the candidate set to scope == ScopeId::GLOBAL
   (the only scope CLOC13.0 populates).
2. Collect the surviving candidate names into a
   HashSet<String>.
3. Walk program.body. For each item:
   - Declaration::VariableDeclaration: partition declarators by
     name ∈ dead_names.
     * all dead → drop item.
     * mixed → emit a new VariableDeclaration with surviving
       declarators only, preserving `kind` + `cv`.
     * all live → push original verbatim.
   - Declaration::FunctionDeclaration: pass-specific (remove-
     unused-vars passes through; treeshake drops by name).
   - ProgramItem::Statement: passthrough until CLOC13.0.1
     ships statement walks.
4. changed = mutation_count > 0.
```

### Why the apply step is safe under `FixedPoint`

Each iteration reduces the binding set strictly.  Removed
declarations produce no new bindings, so next iteration's
eligibility scan finds fewer dead entries.  Fixed point reaches
in at most one additional iteration after the first non-empty
mutation — bindings can only stop being dead by gaining a
reference, which a removal never adds.

The pattern works because removals are *monotonic*.  A pass that
*rewrites* bindings (e.g., CLOC13.A.1 rename) needs a different
convergence argument — see that PR's CHANGELOG when it lands.

### Cross-PR coupling: apply-step ships before activator

Each apply-step PR (CLOC13.{A..E}.1) is observably-identity
when `analysis.bindings` / `analysis.references` are still
empty.  Under the v0.1.0 / pre-CLOC13.0 scope-analyzer:

- candidate set = ∅ → mutation count = 0 → `changed = false`,
  passthrough on every item.
- Identical observable behavior to the body PR's v0.2.0 state.

Apply-step PRs can ship *before* CLOC13.0 lands without
behaviour regressions.  They light up the moment the analyzer's
body lands and starts populating bindings — no follow-up
rebase needed.

## Stage delivery — PR map

| Stage      | PR    | Status |
|------------|-------|--------|
| Unblocker (scaffold + frozen API) | #4763 | merged |
| CLOC13.E (remove-unused-vars body)  | #4766 | merged |
| CLOC13.D (collapse-properties body) | #4773 | merged |
| CLOC13.C (treeshake body)          | #4775 | merged |
| CLOC13.A (rename body)             | #4777 | merged |
| CLOC13.B (inline body)             | #4778 | merged |
| CLOC13.0 (minimal analyzer body)   | #4787 | open   |
| CLOC13.E.1 (remove-unused-vars apply) | #4790 | open   |
| CLOC13.0.1 (references + nested scopes) | queued | |
| CLOC13.{C,A,B,D}.1 (apply steps)   | queued | |
