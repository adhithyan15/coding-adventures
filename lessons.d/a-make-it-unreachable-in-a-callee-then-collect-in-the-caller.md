# A "make it unreachable in a callee, then collect in the caller" GC differential is not reliable on `aarch64-backend` — the callee's vacated frame sits inside the always-conservative `[sp, start_fp)` gap (AOT00-T8, write-barrier follow-up)

Building a real compiled-and-executed proof for `aarch64-backend`'s new `field_store`
write-barrier emission (mirroring the LLVM-backend sibling test, which *does* work —
see `iir-to-llvm`'s changelog), I tried: `main` allocates `parent`, minor-collects it
to tenure it old, calls `helper(parent)` (a separate `IIRFunction`) which allocates
`child` and `field_store`s it into `parent`, then `main` minor-collects again and
checks `child`'s survival via `gc_live_bytes()`. The idea: once `helper` returns,
`child`'s only named local is gone from every currently-live frame, so its survival
should depend purely on the barrier's remembered-set edge.

It passed — with the barrier call *and* with it temporarily removed (a `TEMP-REVERT-
CHECK` comment, `git diff` restored immediately after). A vacuous pass, confirmed via
a full `cargo clean -p aarch64-backend -p twig-aot` rebuild both times (ruling out
stale-binary caching, a mistake this session already hit once with the LLVM sibling
test — see that lesson if it exists). Diagnostic instrumentation (returning
`gc_live_bytes()`/`freed` at each stage as the exit code) showed the object was never
even freed by the **already-shipped, already-well-tested** `__gc_collect_precise` in
the identical shape — with *no* barrier or `parent`/`field_store` involved at all,
just "allocate `child` in a helper, never reference it again, minor/precise-collect in
the caller." So this was never about the barrier, or about my new code; it's a
property of the collector's own stack walk.

Root cause: `gc-core-capi/src/precise_walk.rs`'s own module doc names it explicitly —
`[sp, start_fp)`, "the collector's own frames, below the first walked frame," is
**always** scanned conservatively, with no stack map, regardless of what used to be
there. `start_fp` is the first frame-pointer-mapped frame reached by walking up from
the collector's own entry — i.e. the caller (`main`, in this test). Since the stack
grows down, *every* function `main` has ever called and returned from — including
`helper`, long after it returned — occupies addresses strictly below `main`'s own
frame pointer, i.e. **inside** this exact gap. The collector cannot distinguish "my
own internal call frames" from "some long-since-returned user frame that happens to
sit in the same address range" — both get the identical bias-to-leak conservative
scan. So `child`'s stale address, sitting untouched in whatever stack slot `helper`
last wrote it to, is conservatively rediscovered on every subsequent collect call made
from `main`, no matter how many frames deep or how long ago `helper` returned — the
"once popped, a callee's locals are gone" intuition a heap-allocated-object test
naturally reaches for **does not hold** across this specific gap.

**This is not a bug** — it's the same deliberate bias-to-leak / never-under-mark
design every collect entry in this codebase already documents, and changing it (e.g.
zeroing callee frames on return) would be a real, unrelated engineering project with
its own costs, not a quick fix. It also is not specific to the new minor-collect
entry: the already-shipped `__gc_collect_precise` reproduces it identically, in a
shape with zero write-barrier involvement, proving the *test's* premise was wrong, not
the code under test (the exact same "swap in the pre-existing, already-shipped
collector and reproduce the identical failure" diagnostic move as the lesson directly
above — it is worth repeating as a default first step whenever a *new* GC entry point
looks broken).

**Fix (this session): don't chase a real-execution differential for this backend
across a callee-return boundary.** Fall back to the unit-level relocation-symbol
assertion (`aarch64-backend`'s own `compile_with_relocs` — assert the expected
`BL __twig_gc_write_barrier` relocation is emitted, confirmed load-bearing via a
revert-check at that level) as the primary evidence, exactly as `array_ref_tracing.rs`
concluded for a different but structurally similar reason ("the actual, reliable
regression proof lives at the `gc-core` level," not the compiled pipeline) — and say
so plainly in the PR: this GC-codegen change has strictly weaker end-to-end
verification than its LLVM-backend sibling, which *does* have a working real-execution
differential (LLVM's own SSA-value liveness/register allocation, not this backend's
per-function whole-lifetime stack-map declaration plus this specific conservative
gap, is presumably why that one works) — a genuine, structural asymmetry between the
two backends' testability, not a gap this session chose to leave open.

**If a real-execution differential across this exact boundary is ever needed again:**
the gap is a property of *where the collect call sits relative to the first mapped
frame*, not of the object itself — a design that kept the collect call and the
target object at a *shallower* call depth than any already-returned frame (so the
returned frame's memory is genuinely below, not overlapping, the conservative-gap
scan) or that deliberately clobbered the vacated stack region before collecting might
work, but is fragile by construction and wasn't pursued here given the unit-level
signal was already sufficient and reliable.

**Actual fix:** allocate a **batch** of dead objects (`DEAD_BATCH = 64`), none ever
bound to a Rust local that outlives its own loop iteration — so no *specific* address
needs to survive the collect call for verification. An unoptimized debug build's stray
stack/register garbage can only accidentally retain a small, bounded number of stale
addresses (bounded by how many registers/stack slots a call site can spill — this file's
own `spill_and_sp` bounds a maximal spill at 18 words), so among 64 independent,
never-named allocations, the overwhelming majority are certain to have no stale
reference anywhere on the stack. `freed >= DEAD_BATCH - STRAY_TOLERANCE` (tolerance 8)
is then a deterministic, generous bound — confirmed via 20 consecutive local runs, all
clean, after being flaky often enough in CI to get flagged as a known issue in the first
place. Applied to `gc-core-capi/src/stack_scan.rs`'s four affected tests.
