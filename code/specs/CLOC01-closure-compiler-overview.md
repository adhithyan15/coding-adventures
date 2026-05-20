# CLOC01 — Closure Compiler in Rust: Overview & Pipeline Architecture

## Why this project exists

This spec opens the **CLOC** series — a Rust reimplementation of Google's Closure
Compiler, structured as a pipeline of small, unix-style packages. The motivating
goals are three:

1. **Understand the magic.** Closure Compiler is one of the most aggressive
   JavaScript optimizers ever built. By rebuilding it package-by-package, with
   every transformation visible and traceable, we make its optimization decisions
   legible — not opaque.

2. **Plumb the correlation vector from day one.** Every byte of source must carry
   a CV ID that survives lexing, parsing, type checking, every optimization pass,
   code emission, and source-map generation. The optimizer's "magic" is exactly
   the set of CV contributions it appends along the way. This is a structural
   invariant, not a feature.

3. **Build a JavaScript frontend that's reusable by future backends.** The same
   tokens, AST, and type sidecar that feed this Closure Compiler clone must also
   feed the future V8-in-Rust clone (JS → bytecode → VM). The frontend cannot be
   coupled to either backend. This constraint shapes every interface in the CLOC
   series.

The Closure Compiler is the first consumer. It is not the only consumer. Anything
the CLOC pipeline learns about JavaScript — every grammar rule, every AST node,
every CV contribution shape — must be usable by the V8 clone without modification.

## Scope of "Closure Compiler" here

We are cloning the optimizer, not Google's particular codebase. The deliverable is
a `closurec` binary that:

- Reads JavaScript source (any ES version from ES1 through ES2025).
- Optionally consumes a **type sidecar** (separately produced — see CLOC04).
- Runs a configurable pipeline of optimization passes (DCE, renaming, inlining,
  constant folding, tree shaking, property collapsing, etc.).
- Emits optimized JavaScript plus a standard source-map v3.
- Reports every transformation via a CV chain that resolves each output byte back
  to one or more input bytes.

Out of scope for the MVP:
- The Java codebase of the Google Closure Compiler.
- TypeScript as an input language (deferred — see Section 7).
- Polyfill injection.
- Module bundling beyond what's required for tree shaking.

## Working principle: small unix-style packages

Per the repo's conventions (CLAUDE.md, `feedback_smaller_prs`,
`feedback_parallel_execution`), every component of the pipeline is a separate
publishable Rust crate. A pass is a package. The pass scheduler is a package. The
emitter is a package. The CLI is a package that wires everything together. This
maximizes parallel PR work and lets passes be tested, swapped, and reasoned about
independently.

The shape of every pass package is the same:

```text
input:   javascript-ast::Program  +  Option<type-sidecar::Sidecar>
output:  javascript-ast::Program  +  Vec<correlation_vector::Contribution>
```

A pass never mutates its input. It returns a new tree plus the list of CV
contributions it would append to the surviving nodes. The scheduler merges
contributions and threads the new tree into the next pass.

## The pipeline

Three independent sub-pipelines converge at the AST, and one optimizer pipeline
consumes them:

```text
JavaScript source ──► [JS frontend cascade] ───► javascript-ast::Program ─┐
                                                                          │
JSDoc comments ────► [JSDoc sub-pipeline]  ───► type-sidecar::Sidecar ────┤
                                                                          ├──► [Closure passes]
TypeScript source ─► [TS sub-pipeline]      ──► type-sidecar::Sidecar ────┤        │
                                                                          │        ▼
External annotations (.d.ts-style) ────────────► type-sidecar::Sidecar ───┘    optimized AST
                                                                                   │
                                                                          ┌────────┴────────┐
                                                                          ▼                 ▼
                                                                  closure-emitter   closure-source-map
                                                                          │                 │
                                                                          └───► closurec ◄──┘

                          ... the same javascript-ast also flows into the future
                              V8-in-Rust clone for bytecode lowering. CLOC never
                              assumes any single backend.
```

### Sub-pipeline A — JavaScript frontend (the part we are "fixing up" first)

| Crate | Role |
| --- | --- |
| `javascript-tokens` (new) | Shared `TokenKind` enum, version tag, and span type. Backend-agnostic. |
| `javascript-ast` (new) | Backend-agnostic typed AST. Every node carries a `CvId`. See CLOC02. |
| `javascript-lexer` (existing — repoint) | Wraps `GrammarLexer` over the versioned `ecmascript/esNN.tokens` grammars. Constructor takes `EsVersion`; default = latest. |
| `javascript-parser` (existing — repoint) | Wraps `GrammarParser` over `ecmascript/esNN.grammar`. Emits `javascript-ast::Program`. |

The current `javascript-lexer` and `javascript-parser` already load versioned
grammars from `code/grammars/ecmascript/`. The fix-up is:

1. Replace the empty-string `""` default version with `EsVersion::Es2025`.
2. Stop returning generic `GrammarASTNode`; emit `javascript-ast` nodes instead.
3. Plumb a CV from every token through every AST node (CLOC03).
4. Retire `code/grammars/javascript.{tokens,grammar}` — the stubs are obsolete
   once everything points at the versioned grammars.

### Sub-pipeline B — JSDoc (separate grammar, feeds the type sidecar)

| Crate | Role |
| --- | --- |
| `jsdoc-tokens`, `jsdoc-ast` | Standard token + AST crates. |
| `jsdoc-lexer`, `jsdoc-parser` | Wrap `GrammarLexer`/`GrammarParser` over a new `code/grammars/jsdoc/` grammar pair. |
| `jsdoc-comment-extractor` | Pulls comment spans from a `javascript-ast::Program`, hands raw text to `jsdoc-parser`. |
| `jsdoc-types-extractor` | Walks the JSDoc AST and emits `type-sidecar::Sidecar` records keyed by the JS-side `CvId` the comment annotates. |

### Sub-pipeline C — TypeScript (deferred from MVP)

The TS grammar already exists at `code/grammars/typescript/ts*.grammar`. Wiring is
deferred. The plan is symmetric to the JS frontend: `typescript-tokens`,
`typescript-ast`, `typescript-lexer`, `typescript-parser`, plus
`typescript-types-extractor` that emits the same `type-sidecar` format as JSDoc.

The point of the sidecar format being **shared** across JSDoc and TS is that the
Closure Compiler does not need to know which producer wrote the types. JSDoc and
TS are interchangeable type sources.

### Sub-pipeline D — Closure Compiler proper

| Crate | Role |
| --- | --- |
| `type-sidecar` | Format spec, Rust types, (de)serializers, and CV-keyed lookup. See CLOC04. |
| `type-sidecar-merger` | Merges multiple sidecars (e.g., JSDoc + external `.d.ts`) with a conflict policy. |
| `closure-typechecker` | Consumes `(Program, Sidecar)`. Emits judgments and errors, keyed by CV. |
| `closure-pass-constant-fold` | One pass per crate. Each is independently testable. |
| `closure-pass-dce` | Dead code elimination. |
| `closure-pass-rename` | SIMPLE/ADVANCED renaming. |
| `closure-pass-inline` | Function inlining. |
| `closure-pass-treeshake` | Module-level tree shaking. |
| `closure-pass-collapse-properties` | ADVANCED-mode property collapsing. |
| `closure-pass-remove-unused-vars` | Self-explanatory. |
| `closure-pass-fold-control-flow` | Branch elimination, unreachable code removal. |
| `closure-pass-pipeline` | Pass scheduler. Orders passes, tracks CV contributions, runs to fixed point where appropriate. |
| `closure-emitter` | `javascript-ast::Program → String`. |
| `closure-source-map` | Walks the CV chain to produce a standard source-map v3. |
| `closure-cli` | The `closurec` binary. Argument parsing, file I/O, error reporting. |

Each pass crate is its own PR. The pipeline scheduler is its own PR. Reorderings,
new passes, and disabling passes are all configuration changes — no code change
required for users.

## Backend-agnostic invariants (the V8-reuse contract)

The `javascript-ast` crate is the contract between the frontend and any backend.
Every CLOC-series spec that touches the AST must respect these invariants. They
exist because the same AST will later feed the V8-in-Rust clone.

1. **No backend types in AST nodes.** The AST does not import from
   `closure-*`, `type-sidecar`, IR crates, bytecode crates, or anything specific
   to one consumer. The AST imports only from `javascript-tokens` and
   `correlation-vector`.

2. **Every node carries a CV ID, not a span.** Spans live in CV `Origin` records
   in a parallel log. Nodes hold `CvId` (lightweight, copyable). This keeps node
   structs small and makes the CV log the single source of truth for provenance.

3. **No mutation in the public surface.** All transformations produce new trees.
   In-place mutation makes CV reasoning much harder; we rule it out. (Internally,
   builders may mutate during construction — they expose immutable trees.)

4. **Version tag on `Program`, not on every node.** Nodes that exist in multiple
   ES versions look the same; nodes that are version-gated (e.g., decorators)
   simply do not appear in the AST when parsing an older version. The `Program`
   root records which `EsVersion` produced the tree.

5. **No type information in the AST.** Types live in the sidecar, keyed by CV.
   This is the JSDoc/TS interchangeability principle: the AST is type-blind.

6. **No optimization metadata in the AST.** "This is constant," "this is dead,"
   "this was inlined" — all of these live in CV contributions, not on nodes. An
   AST node is only the syntactic shape.

These invariants are what makes the V8 clone reuse possible without copy-and-edit
divergence. They are also what makes the Closure Compiler clone debuggable — the
AST is small and language-shaped, and all the optimizer's "magic" is in the CV
log next to it.

## Correlation vector plumbing (summary; details in CLOC03)

The CV log is created at the start of a compile and shared across every package.
Every artifact participates:

- The lexer calls `cv.create(Origin{source, location: "line:col"})` for every
  token. `CvId` is stored on the token.
- The parser inherits each token's `CvId` onto the AST node that consumes it. For
  nodes built from multiple tokens, it calls `cv.merge(parents)` to create a new
  ID with multiple parents.
- The typechecker calls `cv.contribute(node_id, Contribution{source:
  "typechecker", tag: "judged", meta: {...}})` for every node it judges.
- Every pass calls `cv.contribute(...)` on nodes it inspects. Passes that remove
  nodes call `cv.delete(node_id)`. Passes that synthesize new nodes call
  `cv.create` with a synthetic origin.
- The emitter records, for every byte it writes, which `CvId` produced it.
- The source-map generator walks the CV log backwards to find each output byte's
  root `Origin` (file:line:col) and emits source-map v3 mappings.

The CV log can be turned off in production (the `enabled` flag); IDs are still
allocated but no contributions are stored, so passes incur near-zero overhead.

## Versioning the JS frontend

Per the user's directive: **a single versioned package**. `javascript-lexer` and
`javascript-parser` each expose a constructor that takes an `EsVersion` enum.
Default is the latest (`EsVersion::Es2025`). The grammar tree under
`code/grammars/ecmascript/` (already complete) backs every version.

```rust
let lexer = create_javascript_lexer(source, EsVersion::Es2025)?;
let ast   = parse_javascript(source, EsVersion::Es5)?;
```

We do **not** split into one crate per ES version. The grammar files already
encode the version boundary; the Rust packages just dispatch on the enum.

## Specs in the CLOC series

This series will grow as the project does. The initial set:

| Spec | Title |
| --- | --- |
| **CLOC01** | This document — overview, pipeline, invariants. |
| **CLOC02** | `javascript-ast` design — node types, CV, version tag, backend-agnostic contract. |
| **CLOC03** | Correlation-vector plumbing through the JS pipeline (lexer → parser → passes → emitter). |
| CLOC04 | `type-sidecar` format — the JSDoc/TS lingua franca. |
| CLOC05 | JSDoc sub-pipeline — grammar, lexer/parser, types extractor. |
| CLOC06 | Pass interface contract — what every `closure-pass-*` crate must implement. |
| CLOC07 | Source-map generation from the CV log. |
| CLOC08 | `closurec` CLI surface — flags, modes, error reporting. |

CLOC01-03 are foundational and land first. CLOC04-08 follow as their respective
sub-pipelines are scheduled.

## Staged PR plan

Per `feedback_smaller_prs` and `feedback_parallel_pr_workflow`:

**Stage 0 — Foundational specs (sequential, tiny PRs):**
- PR: CLOC01 (this doc)
- PR: CLOC02
- PR: CLOC03

**Stage 1 — JS frontend fix-up (parallel after CLOC02-03 land):**
- PR: Retire `code/grammars/javascript.{tokens,grammar}` stubs.
- PR: Create `javascript-tokens` crate.
- PR: Create `javascript-ast` crate, no CV plumbing yet (just the types).
- PR: Plumb CV through `javascript-lexer` (every token gets a `CvId`).
- PR: Plumb CV through `javascript-parser`, switch output to `javascript-ast`.
- PR: Default `EsVersion` to `Es2025`; remove the empty-string "generic" mode.

**Stage 2 — Sidecar + JSDoc (parallel after Stage 1):**
- PRs per CLOC04 and CLOC05.

**Stage 3 — Closure passes (massively parallel):**
- One PR per pass crate.
- PR for `closure-pass-pipeline` scheduler.
- PR for `closure-typechecker`.

**Stage 4 — Output:**
- PR: `closure-emitter`.
- PR: `closure-source-map`.
- PR: `closure-cli` (the `closurec` binary).

**Stage 5 — TypeScript sub-pipeline (deferred to here):**
- Mirror of Stage 1 but for TS, emitting into the same `type-sidecar` format.

Estimated total: 40-50 small PRs. Most of Stages 2-4 run in parallel. Stage 1 is
the only one that bottlenecks the rest.

## What this spec does **not** cover

- Specific opcode-level optimization algorithms — those live in each
  `closure-pass-*` spec (TBD).
- The exact wire format of the type sidecar — CLOC04.
- The pass interface in Rust trait form — CLOC06.
- The `closurec` CLI ergonomics — CLOC08.

This is the map. The territory comes next.
