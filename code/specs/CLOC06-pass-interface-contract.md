# CLOC06 — Pass Interface Contract

## What this spec locks down

Every `closure-pass-*` crate ships exactly one optimization pass. CLOC06
defines the **interface contract** every such crate must implement, the
scheduler that runs them, and the operational invariants (determinism, cost
budget, CV plumbing) that make a heterogeneous pass family behave like one
optimizer.

This spec does not pick algorithms for individual passes — those live in each
pass crate's README. It defines the **shape of the box** every pass goes in.

## The `Pass` trait

```rust
pub trait Pass {
    /// Stable canonical name, e.g. "dce", "constant-fold", "rename".
    /// Used as the contribution `source` per CLOC03 and as a key in the
    /// scheduler's dependency graph. Must be unique across all passes.
    fn name(&self) -> &'static str;

    /// Other passes whose effects this pass relies on. The scheduler
    /// runs all `depends_on` passes before this one.
    fn depends_on(&self) -> &[&'static str] { &[] }

    /// Other passes whose results this pass might invalidate. The scheduler
    /// may re-run those (or downstream consumers of those) after this pass.
    fn invalidates(&self) -> &[&'static str] { &[] }

    /// One-shot vs fixed-point.
    /// - OneShot: run exactly once per scheduler invocation.
    /// - FixedPoint: run until run() reports `changed == false`.
    fn iteration_policy(&self) -> IterationPolicy { IterationPolicy::OneShot }

    /// Coarse cost estimate, in arbitrary "pass-units." Used by the
    /// scheduler's budget gating. A pass that walks the whole tree is ~1
    /// unit; a quadratic analysis is ~10; an inliner running a fixed point
    /// is ~50. Tune per pass.
    fn cost(&self) -> u32 { 1 }

    /// The actual transformation. Pure with respect to its inputs (modulo
    /// the &mut CorrelationLog, which is append-only).
    fn run<'p>(&self, ctx: PassContext<'p>) -> Result<PassOutput, PassError>;
}
```

The trait is **object-safe** — passes are usually held as `Box<dyn Pass>` in
the scheduler. The cost of dynamic dispatch is negligible compared to the
walks each pass performs.

### `PassContext` and `PassOutput`

```rust
pub struct PassContext<'p> {
    pub program: &'p Program,             // the current AST (read-only)
    pub sidecar: &'p Sidecar,             // the merged type sidecar
    pub cv:      &'p mut CorrelationLog,  // the shared log
    pub options: &'p PassOptions,         // CLI-driven knobs
    pub prior:   &'p PassResults,         // outputs of passes that ran earlier
}

pub struct PassOutput {
    pub program: Program,                 // the new tree (owned)
    pub contributions: Vec<Contribution>, // appended by the scheduler
    pub changed: bool,                    // fixed-point loop hint
    pub diagnostics: Vec<Diagnostic>,     // warnings/errors raised by the pass
    pub stats: PassStats,                 // metrics: nodes touched, etc.
}
```

`program` is owned in the output so the scheduler can hand the new tree to
the next pass without lifetime gymnastics. Passes that don't change the tree
still return an owned `Program`; the cost is a single `Arc::clone` if the
program is internally interned (planned follow-up — not required for v1).

`changed` is only meaningful for `IterationPolicy::FixedPoint` passes. The
scheduler ignores it otherwise.

`PassResults` is the scheduler-managed history. A pass can ask "did `rename`
run before me? What did it report?" without parsing CV. Pass-internal state
that *isn't* useful to other passes does **not** go in `PassResults`; it goes
in a pass-local map per the convention below.

## Pass-internal state

A pass that needs to remember intermediate state during its own walk (reaching
defs, escape sets, inlining cost estimates) keeps it in a **pass-local map**
keyed by `CvId`:

```rust
struct DcePassState {
    reachable: HashSet<CvId>,
    escape:    HashSet<CvId>,
}
```

It does **not** add this state to the CV log. The CV log is for durable
lineage that downstream consumers (source map, debugger, other passes) need
to read; pass-internal facts are transient analysis state, separate.

This resolves CLOC03 Open Question 3 in favor of pass-local maps. The
exception: if pass A's intermediate state would be useful to pass B (e.g.,
constant-fold tells DCE which expressions became `Never`-typed), the
producing pass writes that to `PassResults`, not to CV.

## The scheduler (`closure-pass-pipeline`)

```rust
pub struct PassPipeline {
    passes: Vec<Box<dyn Pass>>,
    options: PassOptions,
    budget: u32,
}

impl PassPipeline {
    pub fn new(opts: PassOptions) -> Self;
    pub fn add(&mut self, pass: Box<dyn Pass>);
    pub fn add_default_passes(&mut self);   // canonical pass set

    pub fn run(&self, program: Program, sidecar: &Sidecar, cv: &mut CorrelationLog)
        -> Result<PipelineOutput, PassError>;
}
```

### Scheduling algorithm

1. **Topological sort** the `Pass` list by `depends_on`. Any cycle is an
   error (refuse to run).
2. For each pass in order:
   - If a `depends_on` pass invalidated this one's preconditions (via
     `invalidates`), re-run the prerequisite first (best-effort, bounded).
   - Run the pass; gather output.
   - If `iteration_policy == FixedPoint` and `output.changed`, loop the same
     pass again. Cap at `options.max_fixed_point_iters` (default 16) to
     avoid runaway.
   - Track cumulative cost. If `cumulative > budget`, log a budget warning
     and stop scheduling further iterations of this pass (but continue with
     later passes).
3. After all passes run, emit one `PipelineOutput` with the final program,
   accumulated diagnostics, and per-pass stats.

The scheduler appends a contribution per pass invocation:

```rust
cv.contribute(program.cv, Contribution {
    source: "pipeline",
    tag:    "ran-pass",
    meta:   json!({ "pass": pass.name(), "iteration": i, "cost": pass.cost() }),
});
```

### Determinism

The scheduler runs passes in deterministic order. Within a pass, every walk
must be deterministic — passes iterate `HashMap`s in key-sorted order, never
in `HashMap`'s default order. The pass test harness (below) asserts
determinism by running each pass twice and diffing outputs.

This rules out passes that depend on wall-clock time, system entropy, or
thread scheduling. Multi-threaded passes are allowed; their reductions must
be commutative and associative, and the final write order must be
canonicalized.

### Error aggregation

A pass returns `Err(PassError)` only for **catastrophic** conditions
(internal invariant violation, malformed input). Recoverable issues
(unrecognized JSDoc tag, unhandled type construct) go in `output.diagnostics`
as warnings. The scheduler collects diagnostics across passes; the CLI
prints them in source-order using CV `Origin` resolution per CLOC03.

A pass returning `Err` aborts the pipeline. The output is whatever the
scheduler accumulated up to that point plus the error.

## Pass options

```rust
pub struct PassOptions {
    pub mode: Mode,                     // SIMPLE | ADVANCED | CUSTOM(Vec<&str>)
    pub strict: bool,                   // promote select warnings to errors
    pub max_fixed_point_iters: u32,     // default 16
    pub cost_budget: u32,               // default 10_000 "pass-units"
    pub enabled_passes: Vec<String>,    // CUSTOM mode whitelist
    pub disabled_passes: Vec<String>,
    pub extension: HashMap<String, serde_json::Value>,  // per-pass knobs
}

pub enum Mode { Simple, Advanced, Custom }
```

Closure Compiler's SIMPLE/ADVANCED distinction is enforced at the scheduler
level: SIMPLE adds a curated subset (no rename, no collapse-properties,
local inlining only); ADVANCED adds the full set. The pass crates themselves
don't know about modes — they always run when scheduled.

## Canonical pass set

Each pass is its own crate (`closure-pass-<name>/`). The scheduler's
`add_default_passes` adds them in this order (ADVANCED mode):

| Pass | What it does (one paragraph) |
| --- | --- |
| `constant-fold` | Folds compile-time-evaluable expressions: `2 + 3 → 5`, `"foo" + "bar" → "foobar"`, `true && x → x`, `typeof "s" → "string"`. Uses the type sidecar to short-circuit when the type is `Never` or a literal. Tagged contributions: `folded`, `simplified`. `IterationPolicy::FixedPoint` because folds can expose further folds. |
| `fold-control-flow` | Eliminates dead branches: `if (false) { A } else { B } → B`, removes unreachable code after `return`/`throw`, collapses `switch` arms that can't match, fuses nested blocks. Reads `constant-fold`'s output to find `Never`-typed conditions. Tagged: `branch-eliminated`, `dead-arm-removed`. |
| `dce` | Dead code elimination. Walks the program from entry/exported declarations to mark reachable nodes; unmarked nodes get `cv.delete()` with tag `deleted` and meta `{reason: "unreachable"}`. Uses sidecar purity flags (`pure`, `no_side_effects`) to delete side-effect-free expression statements. `IterationPolicy::FixedPoint` because deletion can free further nodes. |
| `remove-unused-vars` | A narrow DCE specialization: variables whose binding is never read get deleted along with their initializer (only when the initializer is pure). Tagged: `deleted` with meta `{reason: "unused"}`. |
| `inline` | Function inlining. Replaces calls with the callee body when the callee is small, single-use, or marked `@inline` in the sidecar. Reads sidecar `pure` for safety. Tagged: `inlined` with meta `{callee_cv}`. May increase code size — gated by the cost budget. |
| `collapse-properties` | ADVANCED-mode property collapsing: `obj.method()` where `obj` is a static namespace becomes `obj$method()`. Reads sidecar `readonly` and `pure` to ensure correctness. Tagged: `flattened` with meta `{from_path, to_name}`. |
| `treeshake` | Module-level shaking. Walks export graph from entry modules; unexported and unreachable bindings get deleted. Tagged: `deleted` with meta `{reason: "unexported"}`. |
| `rename` | Symbol renaming. SIMPLE renames only function locals; ADVANCED renames module-level and property names (with `collapse-properties` providing the static-namespace boundary). Tagged: `renamed` with meta `{from, to}`. The renamer's mapping is published in `PassResults` so source maps and debuggers can reverse-resolve. |

Pass order matters: `constant-fold` and `fold-control-flow` run early to
expose `dce` opportunities; `dce` and `remove-unused-vars` interleave with
`inline` (inlining can expose deletion; deletion can expose inlining);
`treeshake` runs late (after most local optimization); `rename` runs last
(it doesn't enable further passes but produces unreadable code, so we want
all debuggable passes done first).

## CV contribution conventions

Per CLOC03 §"Pass contribution conventions," the canonical tag table is the
authoritative list. Each pass's tags from the table above must appear in
that pass's tests. Tags **not** in the canonical list must be documented in
the pass crate's README.

Recap of the conventions:

- `source` field is the pass's `name()`.
- One contribution per change, not per visit.
- Deletions append a contribution *before* `cv.delete()` so the meta
  explains the deletion reason.
- Synthesized nodes call `cv.merge(parents, Origin{source: name()})`.

## Testing harness

`closure-pass-test-util` (test-only crate) provides:

```rust
pub fn run_pass<P: Pass>(pass: &P, src: &str) -> PassFixture;

pub struct PassFixture {
    pub before: Program,
    pub after:  Program,
    pub contributions: Vec<Contribution>,
    pub diagnostics: Vec<Diagnostic>,
}

impl PassFixture {
    pub fn assert_contribution(&self, source: &str, tag: &str, count: usize);
    pub fn assert_no_diagnostics(&self);
    pub fn snapshot_program(&self, golden_path: &str);
    pub fn assert_deterministic<P: Pass>(&self, pass: &P) -> Self;
}
```

Every pass crate has at least:

1. **One contribution test per canonical tag.**
2. **One snapshot test per common transformation.** Golden ASTs are JSON in
   `tests/golden/`.
3. **One determinism test.** Pass `assert_deterministic` is mandatory.
4. **One sidecar-driven test.** Show the pass behaves differently with
   `pure`/`readonly` attributes set vs unset, where applicable.

The harness uses the real `javascript-parser` (once Stage 1 lands) so test
fixtures are JS source strings, not hand-built ASTs.

## Operational invariants (the checklist)

Every pass PR is reviewed against:

1. Does the crate implement `Pass` with a unique `name()`?
2. Is `name()` listed in CLOC03's canonical tag table, or documented in the
   crate's README if novel?
3. Are `depends_on` and `invalidates` declared even if empty?
4. Is `iteration_policy` set correctly?
5. Are all `HashMap`/`HashSet` iterations sorted, or replaced with
   `BTreeMap`/`BTreeSet`?
6. Does the pass tolerate `cv.enabled == false`?
7. Does it skip nodes whose sidecar `ty` is `Opaque` or `Unknown` when
   correctness depends on the type?
8. Does every transformation append a contribution with a meaningful `meta`?
9. Are there contribution + snapshot + determinism + sidecar-driven tests?
10. Is coverage ≥ 90%?

Any "no" answer blocks merge.

## What this spec does **not** cover

- Per-pass algorithms — those live in each pass crate's README, with their
  own design rationale.
- The internal representation a pass uses for its analysis (CFG, SSA,
  dominator tree). Each pass picks its own; the public surface is
  AST-in/AST-out.
- Parallelism within the scheduler. The default scheduler is single-
  threaded for determinism; an optional parallel scheduler that runs
  independent passes concurrently is a follow-up.
- The exact format of `PassResults` cross-pass data exchange. Each pass
  publishes only what consumers need; we'll spec specific entries when the
  passes are implemented.

## Open questions

1. **Cross-pass invalidation precision.** Right now `invalidates` is a coarse
   blacklist. A real optimizer might want fine-grained invalidation
   ("changes to `cv` set X invalidates analysis Y"). MVP keeps it coarse and
   re-runs whole passes. Revisit if budget pressure forces it.
2. **Parallel pass execution.** Passes with disjoint `depends_on` graphs
   could run concurrently. Out of scope for MVP; the scheduler interface
   doesn't preclude it.
3. **Persistent caches across compiles.** If a pass's analysis is expensive
   and the inputs haven't changed, can we reuse the result from a prior
   compile? Requires a content-hashing layer above CV. Not blocking.
4. **Pass plugins.** Should third parties be able to ship passes via dynamic
   loading? MVP says no — all passes are linked in. The trait is
   public, so a fork is the supported extension story.
5. **Diagnostics severity model.** Currently a flat `Diagnostic { severity,
   message, cv }`. Closure Compiler has a richer model (warning groups,
   per-group enable/disable, `@suppress` interaction). Defer to CLOC08
   (CLI spec).
