# W23 — Exceptions proposal, third slice: cross-instance tag identity

## Purpose and how this slice was chosen

`code/specs/W22-wasm-exceptions-catch-clause-matching.md` shipped real
same-instance `catch`/`catch_all` matching and named two things it
deliberately left out: (a) cross-instance tag identity (an exception
thrown in one module instance, caught by a `try_table` in another,
reached via `register`/module linking), and (b) `catch_ref`/
`catch_all_ref` + real `exnref` as a reified value type. This session's
task was to genuinely re-derive which of those (or of the two other
still-open post-MVP epics, GC continuation and memory64) is the smallest
self-contained next slice — not to rubber-stamp W22's own guess.

### Investigating cross-instance tag identity directly

Read `wasm-execution/src/lib.rs`'s actual exception machinery (not
assumed from W22's write-up):

- `ExceptionPayload` carries `instance_id` (a value freshly minted by a
  process-wide `AtomicU64`, `NEXT_INSTANCE_ID`, once per **top-level**
  `call_function_impl` invocation — not once per `WasmInstance`) and
  `tag_idx` (a raw index into the CURRENTLY EXECUTING context's own
  combined tag-index space).
- `try_catch_exception` matches a `catch` clause by comparing
  `clause.tag_idx == exception.tag_idx`, gated by an early
  `instance_id == 0 || instance_id != ctx.instance_id` check that refuses
  to even consider a match whenever the exception crossed into a
  different top-level call.
- Read `wasm-conformance`'s `RegistryHost`/`CrossModuleFunction` (the
  cross-module linking infrastructure from WASM05/W10): a call through an
  imported function re-enters `WasmRuntime::call_typed` against the
  CALLEE's own instance, which builds a brand-new `WasmExecutionEngine` —
  and therefore a fresh `instance_id` — for that one call. Confirmed by
  tracing the actual `Result` plumbing: `CrossModuleFunction::call`'s
  `Err(TrapError)` flows back through `host_func.call(...).map_err(VMError::from)?`
  in the `call`/`call_indirect`/tail-call dispatch arm (`wasm-execution`
  line ~9780), which is the SAME choke point (`run_dispatch_loop`/
  `call_function_inner`'s inlined loop) that already calls
  `try_catch_exception` for every other instruction failure. **The
  propagation path already works today** — an exception thrown inside a
  cross-instance host-function call already reaches the caller's own
  `try_catch_exception` search, unassisted. The ONLY reason it's never
  caught is the `instance_id` gate refusing to even look, and even if that
  gate were removed, comparing raw `tag_idx` values across two modules'
  independently-numbered tag index spaces would be meaningless (and
  actively wrong — two unrelated tags at coincidentally the same raw index
  would falsely match).

This is the real finding: **cross-instance tag identity is not an
architecture problem, it's an identity-representation problem.** The
fix is a canonical, globally-unique tag identity minted once per real tag
*definition* (not per top-level call, not per raw index), threaded through
`resolve_tag` so an import adopts the SAME identity as the tag it imports,
compared instead of (not in addition to) the old `instance_id`/`tag_idx`
pair.

Confirmed against the real, already-vendored `try_table.wast` (pinned SHA
`28864811cf03bdbf880733786148feaba339582d`) that this is exactly what its
three currently-failing cross-instance `assert_return` directives need,
no more:

- **`catch-imported`**: module 2 imports tag `test.e0` as `$imported-e0`
  and function `test.throw` as `$imported-throw`; `$imported-throw`'s body
  (in the OTHER instance) throws `test`'s own local `$e0`. Needs the
  SAME canonical identity on both the exporting instance's `$e0` and the
  importing instance's `$imported-e0`, and the exception to survive the
  `CrossModuleFunction::call` boundary — both already fall out of the fix
  above.
- **`catch-imported-alias`**: a SAME-instance case that is nonetheless
  unreachable by raw index — the module imports `test.e0` TWICE, as
  `$imported-e0` (combined index 0) and `$imported-e0-alias` (index 1). It
  throws via the alias (index 1) and catches by the other name (index 0).
  Raw-index comparison can never match two different indices; canonical
  identity does, because both imports resolve to the same underlying tag
  and so get the same identity.
- **`imported-mismatch`**: the safety case. A DIFFERENT module imports
  `test.throw` (which throws `test`'s `$e0`) but declares its OWN,
  unrelated local tag also spelled `$e0`. An inner `try_table (catch $e0
  ...)` must NOT match (different tag, coincidentally same raw index and
  name), falling through to an outer `catch_all`, which — per the real
  spec text ("`try_table` catches foreign exceptions generated from calls
  to function imports as well") — MUST match regardless of instance origin,
  landing on the function's own trailing `(i32.const 3)`. This is also
  the proof that the OLD `instance_id` gate was too conservative in the
  other direction: it made `catch_all` wrongly refuse to catch ANY
  cross-instance exception, when the real spec says `catch_all` catches
  unconditionally. Removing the gate and replacing raw-index comparison
  with real identity comparison fixes both directions at once: `catch`
  becomes correctly permissive (matches the SAME tag across instances) and
  correctly strict (never matches a DIFFERENT tag that merely shares a
  raw index), while `catch_all` becomes correctly unconditional.

All three directives live in the file already vendored by W22 — no new
corpus file, no wast-parser change, no validator change needed. This is
a real, self-contained, corpus-graded slice.

### Re-checking the other three candidates, live, before committing to this one

- **`exnref`/`catch_ref`/`catch_all_ref`**: `exnref` would need to be a
  real, reified handle to "the exception currently being caught" —
  pushed as an actual runtime value a program can inspect, store in a
  local, and later `throw_ref` back out. This repo's `WasmValue` enum has
  no variant that fits (`Ref(Option<u32>)` indexes the GC struct heap;
  an exnref isn't a struct). Building it needs either a new `WasmValue`
  variant plumbed through every exhaustive match on that enum (there are
  dozens — same shape of blast radius W13's `V128` addition had) or
  reusing the GC heap for a new "boxed exception" object kind — either
  way, a new representation decision, not a threading exercise. On top of
  that, per W22's own reading (confirmed unchanged by a fresh read of
  `try_table.wast` for this session): a real cluster of `catch_ref`
  directives ALSO needs `(ref $t)` non-null concrete reference types,
  which `wasm_types::ValueType` still has no room for at all (the same
  gap W20 named blocking GC continuation, below) — so even a complete
  `exnref` implementation would not clear every currently-failing
  `catch_ref` directive without ALSO solving that separate type-system
  gap. Confirmed larger than cross-instance identity: two independent
  hard problems (a new value representation, plus a type-system gap
  shared with a different epic), not one.
- **GC continuation (`call_ref`, non-null concrete ref types)**: read
  `code/specs/W20-wasm-gc-i31-conformance.md` fresh. Its own
  "re-checking the other candidates" section (itself a re-derivation, not
  inherited) found `br_on_null`/`br_on_non_null`/`ref_as_non_null.wast`
  ALL require `call_ref` and non-null `(ref $t)` reference types together,
  which do not exist in this repo's type system at all — explicitly out of
  scope there and unchanged by anything shipped since (W21/W22 are pure
  exceptions-proposal work; neither touched `wasm_types::ValueType`).
  Still XL: no smaller sub-slice was found this session either — the same
  `(ref $t)` gap blocks it that blocks part of `exnref`.
- **memory64**: not previously scoped anywhere in this repo's specs
  (confirmed via `grep -rn memory64 code/specs/` returning nothing before
  this document). A from-scratch investigation: memory64 replaces the
  `i32` address type for `memory.size`/`memory.grow`/every load/store/
  bulk-memory instruction's effective address computation with `i64`,
  for memories declared with a new `is_64` flag. This is NOT additive the
  way tag identity is — every one of `wasm-execution`'s existing load/
  store/`memory.copy`/`memory.fill`/`memory.init` handlers (dozens of call
  sites) computes its address as `u32` today and would need a per-memory
  branch on address width, `wasm-validator` would need the same branch in
  its own bounds-checking rules, and `wasm-wast-parser`/the binary parser
  need the new memory-type encoding bit. No corpus file targets memory64
  in the pinned `WebAssembly/testsuite` tree at all (it lives in a
  separate, not-yet-merged proposal repo at this SHA — confirmed via
  `gh api repos/WebAssembly/testsuite/git/trees/28864811cf03bdbf880733786148feaba339582d`
  listing no `memory64` file), so there would be no real,
  currently-vendorable conformance win even after doing the work. Clearly
  larger AND lower-value than cross-instance tag identity this round.

**Conclusion**: cross-instance tag identity is the smallest, most
self-contained win of the four — a real, already-vendored, already-graded
3-directive fix with no new value representation, no new type-system
gap, and no new corpus needed, following the exact same "optional setter,
combined index space, sentinel-encoded wire payload" mechanical patterns
W21/W22 already established. `exnref`, GC continuation, and memory64 all
remain genuinely out of scope this round, for the specific, re-verified
reasons above.

## Scope

### In scope

1. **A real, canonical tag identity, minted once per tag *definition***:
   - `wasm-runtime`: a new process-wide `NEXT_TAG_IDENTITY: AtomicU64`
     (starts at 1; 0 stays reserved as "no identity assigned", mirroring
     `wasm-execution`'s own `NEXT_INSTANCE_ID`/`0` convention). Every
     module-DEFINED tag gets a freshly minted identity at `instantiate()`
     time (once per real instance, NOT per call — unlike
     `WasmExecutionContext::instance_id`, this must survive across
     multiple calls on the same `WasmInstance`, so it lives on
     `WasmInstance` itself, not on the ephemeral per-call `ctx`).
   - `WasmInstance` gains `tag_identities: Vec<u64>`, the same combined
     imported+defined index space `tags: Vec<u32>` already builds,
     constructed the identical "imports first, then declared" way.
   - `HostInterface::resolve_tag`'s return type changes from
     `Option<FuncType>` to `Option<(FuncType, u64)>` — the second element
     is the EXPORTING instance's own already-minted identity for that
     tag, so an import adopts it verbatim rather than minting a new,
     unrelated one. Two implementors exist in this repo
     (`wasm-conformance::RegistryHost`, a `wasm-runtime` test-only stub);
     both updated. Every other `HostInterface` implementor (this repo has
     none beyond these two, confirmed via a repo-wide grep) keeps
     compiling unchanged, since `resolve_tag` is a defaulted trait method
     (`None`).
   - `RegistryHost::resolve_tag` returns `instance.tag_identities[index]`
     alongside the existing `instance.module.types[...]`-derived
     `FuncType` — no new lookup machinery, just reading a field that
     already lines up with `instance.tags[index]`.
2. **Threading the identity into the engine, mirroring `tags`/
   `set_tags` exactly**:
   - `wasm-runtime::build_engine`: `engine.set_tag_identities(instance.tag_identities.clone())`,
     placed right next to the existing `engine.set_tags(...)` call.
   - `wasm-execution::WasmExecutionEngine` gains a `tag_identities: Vec<u64>`
     field (empty by default) and `set_tag_identities` (new optional
     setter, identical shape to `set_tags`).
   - `wasm-execution::WasmExecutionContext` gains `tag_identities: Vec<u64>`,
     cloned from the engine into `ctx` at `call_function_impl` construction
     time, the same way `ctx.tags` already is.
3. **Real matching, replacing (not layering on top of) the old
   `instance_id`-gated raw-index comparison**:
   - `ExceptionPayload` gains `tag_identity: u64` (0 = "no identity was
     configured for this throw" — the fallback sentinel below).
     `TrapError::exception_with_payload` takes it as a new parameter;
     `throw`'s handler looks it up via
     `ctx.tag_identities.get(tag_idx).copied().unwrap_or(0)`, mirroring
     how it already looks up the tag's declared param types via
     `ctx.tags`/`ctx.types`.
   - The sentinel-prefixed wire format (`encode_exception_payload`/
     `decode_exception_payload`, the mechanism `VMError`↔`TrapError`
     round-trips through, since the shared `virtual-machine` crate's
     `VMError` has no room for a real struct) gains one more
     length-delimited field for `tag_identity`, in the same
     "peel off a known-count prefix" shape the existing fields already
     use — never a blind whole-string split.
   - `try_catch_exception`'s old blanket `instance_id == 0 || instance_id
     != ctx.instance_id` early return is REMOVED. In its place: a
     `Catch` clause matches when BOTH sides have a real (non-zero)
     identity and they're equal; a `CatchAll` clause matches
     unconditionally, exactly as the real spec requires (including a
     foreign/cross-instance exception — this is the `imported-mismatch`
     fix). When a throw carries `tag_identity == 0` OR the catching
     context's own `tag_identities` is empty (every existing hand-built
     unit test in this crate that constructs a `WasmExecutionEngine`
     directly and only calls `set_tags`, never `set_tag_identities`),
     `Catch` falls back to the OLD raw-index comparison — safe precisely
     because such a test only ever runs within a single `ctx`/call, so
     there is no cross-instance ambiguity to get wrong. This keeps every
     pre-existing `wasm-execution` unit test passing unmodified.
   - `instance_id`/`ExceptionPayload::instance_id` themselves are left in
     place (still minted, still carried) — removing them outright would
     touch call sites with no remaining behavioral purpose to fix; they
     simply stop being read by the matching logic. Their doc comments are
     updated to say so explicitly, so a future reader doesn't assume they
     still gate anything.
4. No new corpus file: `try_table.wast` is already vendored (W22). No
   `wasm-wast-parser`/`wasm-validator` changes: the text/binary shapes
   these three directives use (`register`, tag imports, `catch`,
   `catch_all`) already parse and validate correctly today — confirmed by
   the pre-change baseline already grading these exact three directives
   as real `assert_return` FAILURES (wrong runtime answer), not
   `NotYetSupported`/parse errors.

### Explicitly out of scope (this slice)

- **`catch_ref`/`catch_all_ref`/`exnref`/`throw_ref`** — unchanged from
  W22; still needs a new reified value representation, and partially the
  same `(ref $t)` gap as GC continuation. See "the other three
  candidates" above.
- **`(ref $t)` non-null concrete reference types, `(rec ...)` recursive
  type groups** — unchanged from W20/W21/W22.
- **memory64, real threading, the component model, the JIT tier** —
  unchanged; memory64 freshly re-scoped above and confirmed both bigger
  and lower-value (no corpus coverage exists for it in this repo's pinned
  testsuite tree at all).

## Verification

- **Unit tests** (`wasm-execution`): a same-instance regression test that
  `set_tag_identities` with two DIFFERENT identities on two different
  local tags still refuses to cross-match (the `imported-mismatch` shape,
  reproduced directly); a test that two DIFFERENT local tag indices
  sharing the SAME configured identity DO match (the `catch-imported-alias`
  shape); a test that `catch_all` matches even when the thrown
  `tag_identity` has NO corresponding entry in the catching context's own
  `tag_identities` (the cross-instance/foreign-exception case); every
  pre-existing exception/catch test from W21/W22 re-run unmodified to
  confirm the fallback path preserves old behavior exactly.
- **Unit tests** (`wasm-runtime`): `instantiate()` mints a fresh,
  non-zero identity per module-defined tag; two SEPARATE `instantiate()`
  calls on the same module produce DIFFERENT identities (never reused);
  a tag import adopts the resolved identity from the host rather than
  minting its own.
- **Real corpus, measured** (`cargo run --bin wasm_conformance_report`,
  full JSON diff against the pre-change baseline, confirming ONLY
  `try_table.wast`'s stats move):
  - Pre-change (confirmed by reading the committed baseline before any
    edit): `assert_return` 25 pass / 13 fail / 5 not-yet-supported.
  - Expected post-change: `assert_return` 28 pass / 10 fail / 5
    not-yet-supported (`catch-imported`, `catch-imported-alias`,
    `imported-mismatch` move from fail to pass; the remaining 10 fails are
    all `catch_ref`/`exnref`-dependent directives, individually confirmed
    unrelated to this slice).
  - No other file's stats change (no parser/validator changes were made).
- **Downstream consumers**: `cargo test -p lang-aot` (this campaign's
  own named must-check consumer for anything touching shared
  `wasm-execution`/`wasm-runtime` state), plus every other path-dependent
  crate (`wasm-validator`, `wasm-conformance`, `wasm-wast-parser`,
  `twig-to-wasm`, `nib-wasm-compiler`, `brainfuck-wasm-compiler`,
  `ir-to-wasm-compiler`, `iir-to-wasm`, `twig-demo`) built/tested to
  confirm the `HostInterface::resolve_tag` signature change doesn't break
  any of them (none override it; only `RegistryHost` and a
  `wasm-runtime`-internal test stub do).
- `/security-review` before push, per this repo's standing workflow.
- Docker (`linux/amd64`) verification of `cargo test -p wasm-execution`,
  `cargo test -p wasm-runtime`, `cargo test -p wasm-conformance`, and
  `cargo test -p wasm-conformance --test testsuite_conformance
  corpus_matches_the_committed_baseline` before pushing.
