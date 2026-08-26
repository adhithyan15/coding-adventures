# W22 — Exceptions proposal, second slice: real `catch`/`catch_all` matching

## Purpose and how this slice was chosen

`code/specs/W21-wasm-exceptions-tag-throw-slice.md` shipped `tag`/`throw`
and a deliberately non-catching `try_table` — real spec behavior for "no
catch clause matched," but W21 explicitly named "real catch-clause
matching" as the natural next slice and left its own viability assessment
for a later session. This spec is that assessment, done for real against
live state (the pinned testsuite tree at
`28864811cf03bdbf880733786148feaba339582d`, the real exception-handling
proposal text, and this repo's current code), not a rubber stamp.

### Re-deriving viability from scratch

The task handed to this session asked a specific question: does matching
a thrown tag against `catch $tag $label` need anything beyond (a) a
runtime "currently propagating exception" concept, (b) comparing its tag
index against each catch clause's tag operand, (c) unwinding to the
matching handler with the exception's payload pushed — or does it
genuinely need `exnref`/`catch_ref`/`catch_all_ref` as a hard prerequisite?

Live-fetched the real corpus files at the pinned SHA and read them in
full (not summarized from W21's own prior write-up, which itself warned
"never trust prior counts, re-derive"):

- **`throw.wast`** (already vendored): 11/12 directives graded real Pass
  under W21; the one held-out case, `test-throw-1-2`, is
  `(try_table (catch $e-i32-i32 $h) (call $throw-1-2))` — the throw
  happens inside a **called function**, and the **caller's** `try_table`
  must catch it. W21 correctly identified this as needing real
  cross-function exception propagation, not same-function-only matching.
- **`throw_ref.wast`** (not vendored): every one of its 6 test functions
  uses `catch_ref`/`catch_all_ref` and `exnref` locals. Fully entangled
  with the "reify and rethrow" half of the proposal. Still not viable —
  unchanged from W21's own assessment.
- **`try_table.wast`** (not vendored, 13554 bytes, the largest of the
  four exceptions-proposal files): read in full. This is where the real
  finding is. Its ~30 exported test functions split cleanly into two
  groups:
  - A LARGE majority (`simple-throw-catch`, `unreachable-not-caught`,
    `trap-in-callee`, `catch-complex-1`/`catch-complex-2` (nested
    `try_table`), `throw-catch-param-{i32,f32,i64,f64}`, `catch-param-i32`
    (the SAME cross-function shape `test-throw-1-2` needs),
    `catchless-try`, `return-call-in-try-catch` (tests that a tail call
    escapes the enclosing `try_table`'s catch scope), `try-with-param`,
    `duplicated-catches`/`catch-all-before-catch` (first-listed clause
    wins), `as-br-target`/`as-value-provider` (`try_table` still acts as
    a plain `br` target)) use **only `catch`/`catch_all`** — no
    `exnref`/`catch_ref`/`catch_all_ref` anywhere in their bodies.
  - A smaller minority (`throw-catch_ref-param-*`, `catch_ref1`/`catch_all_ref1`/etc.,
    plus several `assert_invalid` cases) needs `catch_ref`/`catch_all_ref`
    and `exnref` specifically.
  - A THIRD, separate cluster (`catch-imported`, `catch-imported-alias`,
    module 3's `imported-mismatch`) needs the thrown tag's identity to be
    compared **across a module-instance boundary** (the throw originates
    in one instance, the catch lives in another, reached via
    `wasm-conformance`'s `RegistryHost`/`CrossModuleFunction`) — a
    genuinely different, harder problem than same-instance matching: it
    needs real cross-instance tag identity, not just an index comparison.
  - A final small cluster (the module using `(tag (param (ref $t)))`,
    `catch_ref1`/`catch_ref2`/etc.) additionally needs `(ref $t)` — a
    non-null CONCRETE reference type — the exact same type-system gap
    W20 and W21 already found blocking the GC-continuation epic
    (`wasm_types::ValueType` has no non-null/nullable distinction for a
    concrete type index). Confirmed unchanged.

**Conclusion**: real catch-clause matching is NOT an all-or-nothing
proposition. `catch`/`catch_all` matching, scoped to exceptions that
never cross a module-instance boundary, is separable from
`catch_ref`/`catch_all_ref`/`exnref` and from cross-instance tag identity
— and it unlocks the clear majority of `try_table.wast`'s real value plus
`throw.wast`'s one held-out case. This is exactly the smaller viable
sub-slice the task asked to look for.

### Confirming the architecture question directly

The task also asked, concretely: is `MAX_CALL_DEPTH`/the existing
Rust-recursion-based `call_function`/`call_function_inner` design able to
support "the callee threw, unwind to the caller's `try_table`, then keep
running" without a fundamentally different unwind architecture?

Read `wasm-execution/src/lib.rs`'s actual `call_function_inner` (not
assumed from memory): a nested WASM `call` recurses through Rust's own
call stack (`call_function` → `call_function_inner` → recursively, for
`call`/`call_indirect`), each level pushing a `SavedFrame` it restores
back into `ctx` only on its OWN success path — never on error (a plain
`?` short-circuited straight out, previously correct because nothing
downstream of a trap was ever resumable). Confirmed this is fixable
without a new unwind mechanism: every instruction dispatch already runs
through exactly ONE choke point per call-stack level (`call_function_inner`'s
own inlined while loop, and `call_function_impl`'s top-level dedicated-thread
entry). Wrapping that ONE choke point with "does the CURRENT frame's
`label_stack` have a matching `try_table` catch clause?" and, on a
mismatch, performing the SAME `SavedFrame` restoration the success path
already does before letting the error propagate one level further up —
is a mechanical, bounded change, not a new architecture. This is exactly
what this slice implements (see "Architecture" below).

## Scope

### In scope

1. **`wasm-execution`**: real same-instance `catch`/`catch_all` matching.
   - `TrapError` gains `exception: Option<ExceptionPayload>`
     (`instance_id`, `tag_idx`, `values`) alongside the existing
     `is_exception` flag; `TrapError::exception_with_payload(...)` is the
     new constructor real `throw` uses (the old payload-less
     `TrapError::exception(...)` stays, for back-compat / any caller that
     only cares about the flag).
   - The `EXCEPTION_SENTINEL`-based `VMError`↔`TrapError` round-trip
     (the mechanism `virtual-machine`, a shared cross-language-frontend
     crate, forced W21 to invent, since `VMError` itself has no room for
     a real struct) is extended with a second, length-delimited
     structured prefix (`EXCEPTION_FIELD_SEP`) carrying the payload, with
     the free-form message always last and never escaped — the exact
     "peel off a known-count prefix, never blindly split the whole
     string" shape this repo's own composite-key lessons require.
   - `decode_function_body`'s `0x1F` (`try_table`) branch, which W21 read
     and DISCARDED the catch-clause list, now builds a real
     `TryTableInfo { block_type, catches: Vec<CatchClause> }`, spilled
     into a new per-function side-table (`ctx.try_table_infos`) via
     `convert_operand`, mirroring the existing `br_table_targets`/`gc_ops`
     precedent exactly.
   - `Label` gains `catches: Vec<CatchClause>` (empty for every
     `block`/`loop`/`if`/function-entry label; populated only by
     `try_table`).
   - `throw` (`0x08`) now pops the tag's real declared param values
     (looked up via a new `WasmExecutionContext::tags: Vec<u32>`, wired
     from the embedder the same optional-setter way `type_section`
     already is) and carries them, plus a fresh per-context
     `instance_id: u64`, in the resulting `TrapError`.
   - The ONE choke point (now factored as `run_dispatch_loop`, used by
     `call_function_impl`'s non-recursive top-level entry, and inlined
     directly — NOT called as a separate function — inside
     `call_function_inner`'s own recursive dispatch loop; see "A real
     regression, caught and fixed" below for why the inlining matters)
     calls `try_catch_exception` whenever an instruction handler fails:
     walks `ctx.label_stack` innermost-to-outermost, matching the FIRST
     clause (in file order) whose kind is `catch` (tag equality) or
     `catch_all` (always matches) — `catch_ref`/`catch_all_ref` clauses
     are structurally present but never selected (this slice produces no
     `exnref`, so they can never legitimately match). A match performs
     the exact effect of an ordinary `br` to the clause's target label
     (reusing `execute_branch` verbatim), pushing the tag's payload
     values first for `catch` (none for `catch_all`). No match: the
     current invocation restores ITS OWN caller's `SavedFrame` (the same
     restoration the success path performs) before propagating the error
     one level further up — so the ENCLOSING level's own choke point
     always sees correctly-restored state when it runs its own search.
   - **Cross-instance exceptions are deliberately never matched**: an
     exception's `instance_id` (freshly minted per
     `WasmExecutionEngine::new`) must equal the CURRENTLY EXECUTING
     context's own for `try_catch_exception` to even consider it. This is
     what makes same-instance-only catching SAFE rather than silently
     wrong — without it, an unrelated module's coincidentally-equal raw
     tag index could produce a false-positive match (see
     `try_table.wast`'s own `imported-mismatch` test, which specifically
     probes this).
2. **`wasm-runtime`**: `HostInterface` gains `resolve_tag` (default `None`,
   so every pre-existing implementor keeps compiling unchanged);
   `instantiate()`'s `ImportTypeInfo::Tag` arm — previously an
   unconditional link failure, since W21 explicitly deferred real tag
   import resolution — now asks the host for the real tag type and
   checks it against what THIS module's own import declaration expects,
   exactly like `Function` imports already do. `wasm-conformance`'s
   `RegistryHost` implements it for real (cross-module `register`-based
   tag imports, the same mechanism already used for functions/globals/
   memories/tables).
3. **A real, pre-existing latent bug, found and fixed**: `WasmInstance`
   gained a `tags: Vec<u32>` field — the COMBINED imported+defined tag
   index space, "imports first, then declared," built the identical way
   `func_types` already is during `instantiate()`. `module.tags` ALONE
   (like `module.functions`) only ever held module-DEFINED tags' type
   indices — W21's own doc comment says so explicitly — but `throw`/
   `catch`'s own `tag_idx` operand is always encoded in the COMBINED
   space. `wasm-validator` already built its own correctly-combined
   `tag_types` (`type_check.rs::build_module_context`); this crate's
   `build_engine` and `wasm-conformance`'s `RegistryHost::resolve_tag`
   did not, and read `instance.module.tags` directly instead — silently
   off by however many tag imports a module declared. W21 never
   surfaced this (its own `throw` handler never read `ctx.tags` for
   anything observable); this slice's real payload-popping was the first
   code path to actually exercise it, and did, immediately, on the real
   corpus (`try_table.wast`'s `catch-complex-1`/`catch-complex-2`, which
   declare tag imports before several same-arity/different-arity local
   tags — see "Verification" for the exact repro and fix).
4. Vendor `try_table.wast` (pinned SHA), add to `TESTSUITE_FILES`,
   regenerate the baseline.
5. **`wasm_types::ValueType::Exnref`** — a new, deliberately INERT value
   type (real spec byte `-0x17`), recognized by `wasm-wast-parser`'s
   `parse_value_type` so a module MENTIONING `exnref` (e.g. a
   `catch_ref`/`catch_all_ref` target block's declared result type)
   still PARSES and gets a chance to build — critical because W14's
   per-module isolation means a single unrecognized value type anywhere
   in a module fails the WHOLE module, and `try_table.wast`'s own big
   module 2 mixes `catch`/`catch_all`-only functions with
   `catch_ref`/`catch_all_ref`-using ones in the SAME module. Never
   produces or consumes a real `exnref` runtime value (same "parsed but
   inert" treatment W21 already gave `catch_ref`/`catch_all_ref`'s own
   binary shape).

### Explicitly out of scope (this slice)

- **`catch_ref`/`catch_all_ref`/`exnref`/`throw_ref`** — clauses of these
  kinds are parsed, validated (bounds-checked), and structurally present
  on `Label::catches`, but `try_catch_exception` never selects them as a
  match (no `exnref` value is ever produced). `throw_ref` the instruction
  is not implemented at all.
- **Cross-instance tag identity / catching a foreign exception** —
  the real spec's own stated behavior ("`try_table` catches foreign
  exceptions generated from calls to function imports as well") is
  explicitly narrowed away here: an exception is only ever eligible to
  be caught within the SAME `WasmExecutionContext` it was thrown from
  (see `instance_id`, above). `catch-imported`/`catch-imported-alias`/
  `imported-mismatch` (`try_table.wast`) and any hypothetical foreign
  exception from a WASI-style import remain uncaught, always — a
  conservative, safe default (identical in direction to W21's own
  "never catches" default, just narrowed to one specific boundary
  instead of all catching). A real fix needs a tag-identity concept that
  survives crossing a `RegistryHost`/embedder-provided host-function
  call boundary (e.g. a canonical id minted once per tag at
  instantiation and threaded through import resolution the way a real
  engine's `funcref` identity already has to be) — a genuinely separate,
  later slice.
- **`(ref $t)` non-null concrete reference types** — same gap W20/W21
  already named for GC continuation, unchanged. Blocks a handful of
  `try_table.wast` functions regardless of the above (they also need
  `exnref`/`catch_ref`).
- **`(rec ...)` recursive type groups** — same gap, unchanged; blocks
  none of `try_table.wast`'s content newly (only `tag.wast`'s own
  already-known "link-time typing" modules).
- **memory64, real threading, the component model, the JIT tier** —
  unchanged from W07/W20/W21's own re-confirmed assessments.

## Architecture: how catching actually unwinds through nested calls

The corpus's own `catch-param-i32` (and `throw.wast`'s held-out
`test-throw-1-2`) need the exception to survive a REAL nested `call`
before being caught by the CALLER's `try_table`. Concretely, per level of
the Rust recursion `call_function_inner` already performs for `call`/
`call_indirect`:

1. `run_dispatch_loop`'s (or `call_function_inner`'s own inlined
   equivalent) choke point calls the failing instruction's handler. If it
   fails with an exception (recognized via the sentinel-prefixed
   message), `try_catch_exception` searches the CURRENT frame's
   `label_stack` for a match.
2. **No match at this level**: this invocation's own `SavedFrame` (the
   CALLER's state, saved when this invocation began) is restored into
   `ctx` — locals, label stack, control-flow map, and every per-function
   side-table (`br_table_targets`/`gc_ops`/`simd_consts`/
   `try_table_infos`) — before the error propagates via `?` back through
   `call_function` to the 0x10/0x11 handler that invoked it, which is
   ITSELF being dispatched from the ENCLOSING level's own choke point.
   By the time that choke point's own `handler(...)` call returns the
   error, `ctx` already reflects the CALLER's own state, so its own
   `try_catch_exception` search operates on the CORRECT `label_stack`.
3. **A match**: the matching clause's tag payload (if `catch`, not
   `catch_all`) is pushed, then `execute_branch` runs exactly as if an
   ordinary `br` had targeted the clause's label — the SAME mechanism
   every other structured branch in this crate already uses.

This means catching "for free" gets the real spec's `return_call`
interaction right too: `return-call-in-try-catch`/
`return-call-indirect-in-try-catch` are the corpus's OWN test that a tail
call escapes the enclosing `try_table`'s scope entirely — WASM16's
existing tail-call transition resets `ctx.label_stack` for the new
function WITHOUT pushing a new `SavedFrame` (no logical call-stack growth
for a tail call), so by the time the tail-called function's own
(uncaught) exception is searched, the ORIGINAL caller's `try_table` frame
is already gone from `label_stack` — no special-casing needed.

The label-depth arithmetic itself: a catch clause's `label_rel_depth` is
resolved by both `wasm-wast-parser` and `wasm-validator` BEFORE
`try_table`'s own label exists (the label depth is relative to
`try_table`'s ENCLOSING scope, not to itself) — so at the moment
`try_catch_exception` runs, with `try_table`'s own `Label` still on
`ctx.label_stack` at index `try_label_idx`, the real target's
`execute_branch`-style depth is `ctx.label_stack.len() - try_label_idx +
label_rel_depth` (verified against the corpus's own `duplicated-catches`/
`catch-all-before-catch`/`catch-complex-1`/`catch-complex-2` nested-block
shapes — see Verification).

## A real regression, caught and fixed: `DEDICATED_STACK_SIZE`

Extracting the dispatch-and-catch loop into a shared `run_dispatch_loop`
function and calling it from BOTH `call_function_inner` (the recursive,
per-nested-call path) and `call_function_impl` (the non-recursive
top-level entry) added one extra Rust stack frame per NESTED call level
— a real, measured regression: the vendored `call_indirect.wast` (whose
own recursive test needs a depth close to `MAX_CALL_DEPTH`'s 1200-level
ceiling, itself bisected with only a ~1.5x safety margin per that
constant's own doc comment) started overflowing the real OS thread stack
again, confirmed via `cargo run --bin wasm_conformance_report` genuinely
aborting with `stack overflow`, not a theoretical concern. Fixed two ways:
`call_function_inner`'s own loop keeps the catch-or-propagate logic
INLINED (not a call to the shared helper) — it's on the hot recursive
path, `call_function_impl`'s is not — and `DEDICATED_STACK_SIZE` was
doubled (8 MiB → 16 MiB) as a generous, round margin restoration, with
`MAX_CALL_DEPTH` deliberately left UNCHANGED (raising it would need a
full re-bisection per that constant's own historical methodology; this
slice doesn't need or want a higher ceiling, just its existing margin
back). Re-confirmed clean: the full vendored corpus completes without a
stack overflow.

## Verification

- **Unit tests** (`wasm-execution`): same-instance `catch` payload
  delivery (i32), `catch_all` matching, first-match-wins among multiple
  structurally-matching clauses (mirroring `duplicated-catches`), a
  matching clause now catching what W21's own test proved never used to
  match, a non-matching clause still propagating uncaught, and — the
  architecturally load-bearing case — an exception thrown by a CALLED
  function caught by the CALLER's `try_table` with its 2-value payload
  delivered (mirroring `test-throw-1-2` exactly).
- **Unit tests** (`wasm-runtime`): `instantiate()` builds the correctly
  COMBINED tag index space for a module with one tag import followed by
  two differently-typed local tags (the exact shape that reproduced the
  latent bug); an incompatible tag import is rejected at link time.
- **Real corpus, measured** (`cargo run --bin wasm_conformance_report`,
  diffed against the pre-change baseline — confirmed via a full JSON
  diff that ONLY `tag.wast`/`throw.wast`/`try_table.wast` moved, no other
  already-vendored file's stats changed):
  - `throw.wast`: `assert_return` 1/2 → **2/2**, `assert_exception`
    stays 7/7 — `test-throw-1-2` (the one directive W21 documented as a
    deliberate `Fail`) now passes for real.
  - `tag.wast`: `module` 1/2 → **2/2** (real, additional, non-obvious
    gain from the `resolve_tag` fix: its own tag-import-testing module
    previously failed to link at all).
  - `try_table.wast` (newly vendored): `module` 5/6 real (1 NotYetSupported
    — the `(ref $t)`/`elem declare`-using module, correctly out of
    scope), `assert_return` 25/38 real passes (13 real fails, ALL
    individually confirmed to be exactly the documented out-of-scope
    cases: 10 `catch_ref`-using directives, `catch-imported`/
    `catch-imported-alias`/`imported-mismatch`'s cross-instance cases —
    zero unexplained failures), `assert_trap` 2/2, `assert_exception`
    4/4, `assert_invalid` 5/9 real (4 NotYetSupported, all
    `exnref`/`catch_ref`-specific type-mismatch rules this slice
    deliberately doesn't implement — see "Explicitly out of scope"),
    `assert_malformed` 2/2, `register` 1/1.
- **Downstream consumers**: `cargo test -p lang-aot` (the McCarthy Lisp
  WASM backend, this campaign's own named "must re-check" consumer) and
  the full `wasm-runtime`/`wasm-validator`/`wasm-wast-parser` suites all
  pass, aside from one pre-existing, unrelated failure
  (`closure_identity_returns_captured_value`, confirmed to fail
  identically on an unmodified `origin/main` checkout before this
  session's changes — an `iir-builtin-lowering` gap, not touched here).
- `/security-review` before push, per this repo's standing workflow.
- Docker (`linux/amd64`) verification of `cargo test -p wasm-execution`,
  `cargo test -p wasm-runtime`, `cargo test -p wasm-conformance`, and
  `cargo test -p wasm-conformance --test testsuite_conformance
  corpus_matches_the_committed_baseline` before pushing.
