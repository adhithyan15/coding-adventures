# AOT00-T8 — adaptive safepoint scheduling (generational auto-enactment) (design)

> Status: **design, pre-implementation.**
>
> The precision-ladder **algorithms** are complete
> (`mark-and-sweep ✓ → interior-precise ✓ → generational ✓ → precise-roots ✓ → compacting ✓ → incremental ✓`),
> and [`AOT00-T5`](AOT00-T5-variable-length-ref-arrays.md) closed the object-model gap (arrays are
> precise + movable). What remains is **policy**: `AdaptivePolicy` (`gc-core/src/policy.rs`)
> evaluates a `GcProfile` and recommends an algorithm switch, but today only *one* of its three
> recommendations is actually carried out automatically. This rung closes that gap for the
> **Generational** recommendation.

---

## 1. The problem — only `Compacting` is enacted at the safepoint

`AdaptivePolicy::evaluate` returns the single highest-priority recommendation, in this order
(`policy.rs` doc comment):

1. **Incremental** — max pause time exceeds budget.
2. **Generational** — EMA survival ratio is low (most objects die young).
3. **Compacting** — fragmentation is high.

`gc-core-capi`'s `__gc_safepoint` — the paced, auto-collect entry every native-AOT program's
`safepoint` op calls — consults exactly one of these signals: `FlatHeap::should_compact()`. If
fragmentation is high *and nothing higher-priority fired*, the safepoint runs a compacting
collection instead of a plain precise one. That wiring is real end-to-end automation (`should_compact`
→ `__gc_collect_compacting`).

The **Generational** recommendation has no equivalent. `FlatHeap::collect_minor` /
`collect_minor_region` exist and are correct (proven by the write-barrier + promotion-barrier
differentials in the generational arc), but nothing ever calls them automatically — a minor
collection only happens if some *specific* consumer (`vm-core`, today) chooses to call it
directly. A native-AOT program compiled purely through `twig-aot` never runs a minor GC no matter
how low its survival ratio is; every automatic collection is a full scan of the whole live set,
old generation included. The generational collector — the highest-value payoff for
high-allocation-rate, short-lived-object workloads (exactly the JS/Ruby/Python shape T5 targeted)
— sits unused at the one call site that matters for AOT binaries.

**Incremental** is intentionally out of scope here: unlike a compacting or minor collection, it
is not a single call — `incremental_start`/`_step`/`_finish` is a stateful three-call protocol
that must interleave with the mutator across many safepoints. Auto-driving it needs a persistent
per-heap "am I mid-cycle" state machine at the capi layer, a materially different (and separately
reviewable) feature. This spec's fix keeps that decision advisory, unchanged from today.

---

## 2. The starvation hazard — why "always honor Generational" is unsound

The naive fix — `if should_collect_minor() { collect_minor } else { ... }` — has a real bug. The
EMA survival ratio is *sticky*: a workload that keeps the ratio below threshold keeps
`AdaptivePolicy` recommending `Generational` every single time it's asked, cycle after cycle. If
the safepoint always honors that recommendation, **no full collection ever runs again**. A minor
collection *by definition* never scans or frees the old generation (`collect_minor`'s own doc
comment: "Old objects are never scanned or freed"). So old-generation garbage — objects that
died after being tenured — would accumulate forever: not a use-after-free, but an unbounded,
policy-driven memory leak, which for an always-on adaptive heuristic is exactly the kind of
"switched on and never noticed" bug worth designing out rather than documenting around.

**Fix: bound consecutive minors.** Track how many minor collections have run *in a row* since the
last full collection (`collect_region`/`collect_precise`/`collect_mixed`/`collect_compacting` —
any of them resets the streak; only a minor collection increments it). Once the streak reaches a
cap, `should_collect_minor()` returns `false` regardless of what the policy says, forcing the next
paced collection to be full and reclaim whatever accumulated in the old generation. This is the
same shape as a real generational collector's "major GC every N minors" trigger, chosen over
tracking old-generation bytes directly (which would need new accounting threaded through every
sweep/compact/tenure path) because it is a strictly simpler, still-sound bound: it does not need
to know *how much* old garbage exists, only that a full sweep runs often enough that it can never
be starved out.

The cap is tunable (mirroring the existing `tenure_age` getter/setter pattern) rather than a bare
constant, since the right value trades off "how much old churn a workload can tolerate before its
next full pause" against "how much minor-GC throughput a workload gives up for early full
collects" — a workload-specific choice, not a universal one.

---

## 2b. The barrier-coverage hazard — why automatic enactment needs an explicit attestation

A second, more severe hazard surfaced by adversarial security review of the first draft: minor
collection's correctness — not just its pacing — depends on the remembered set being *complete*.
`collect_minor`'s own doc comment says it plainly: "every old→young store must have called
`write_barrier`; a missed old→young pointer whose only path to a young object is through that
old parent would let the young object be wrongly freed." That is a **use-after-free**, not the
§2 leak.

`gc-core` cannot verify this on its own — the barrier is a *producer* obligation, enforced (or
not) in an entirely different crate than the collector. Auditing every current producer of the
shared `field_store` IIR op:

- `vm-core`'s `handle_gc_field_store` (`dispatch.rs`) calls `ctx.heap.write_barrier(...)` on every
  store. **Barrier-correct.**
- `aarch64-backend`, `x86_64-backend`, and `iir-to-llvm`'s `field_store` lowering each emit a bare
  store (`str`/`mov`/`store` respectively) with **no barrier call** — confirmed by grep: `write_barrier`
  appears nowhere in any of the three crates. **Not barrier-correct.**

Those three backends are exactly the ones that emit `safepoint` at loop back-edges and function
entries (forwarded to `__gc_safepoint` via the `__twig_gc_safepoint` alias) — i.e. exactly the
automatic call site this spec's §3.2 wires up. With `DEFAULT_TENURE_AGE = 1` (immediate tenuring)
and `AdaptivePolicy`'s generational threshold (`ema_survival_ratio < 0.15`) describing an ordinary
allocation-heavy loop rather than a corner case, an unconditional `should_collect_minor` wired into
`__gc_safepoint` would make every AOT-compiled program that stores a reference into an
already-tenured object a live UAF, not a latent one — the missing barriers exist today, but no
automatic collection site could ever reach a minor cycle before this spec, so they were harmless.

**Fix: an explicit, off-by-default attestation.** `FlatHeap::should_collect_minor` gains a new
precondition, checked *before* the streak cap: `self.auto_minor` (default `false`), set only via
`FlatHeap::set_auto_minor(true)` / the capi `__gc_set_auto_minor(1)`. This is not a correctness
mechanism gc-core can enforce (it's an attestation, not a proof) — it is a deliberate, documented
"you must know what you are doing" gate, the same shape as `unsafe` itself: the default keeps
`__gc_safepoint`'s behavior byte-for-byte what it was before this spec (safe, matches every
existing producer's actual guarantees), and turning it on is a one-line, loudly-documented opt-in
a future producer takes only after it emits the barrier on every store. `vm-core` does not need
it (it doesn't call `__gc_safepoint`/`gc-core-capi` at all — its own `handle_safepoint` calls
`should_collect`/`should_compact` directly and could, in a follow-up, opt into
`should_collect_minor` too, since it *is* barrier-correct). Closing the gap for the native/LLVM
backends — emitting `__gc_write_barrier` from their `field_store` lowering — is future work,
tracked separately; it is what would let an embedder responsibly call `__gc_set_auto_minor(1)`.

---

## 3. Design

### 3.1 `gc-core` (`flat_heap.rs`)

- `minor_streak: u32` field on `FlatHeap` (init `0`).
- `max_minor_streak: u32` field (default `DEFAULT_MAX_MINOR_STREAK = 8`), with
  `set_max_minor_streak(u32)` (clamped to a minimum of `1` — `0` would forbid every minor
  collection, defeating the feature) / `max_minor_streak() -> u32` accessors, mirroring
  `set_tenure_age`/`tenure_age`.
- Every **full**-collect method (`collect_region`, `collect_precise`, `collect_mixed`,
  `collect_compacting`) resets `self.minor_streak = 0` in its finishing tail, alongside the
  existing stats bookkeeping.
- `minor_finish` (the shared tail of `collect_minor`/`collect_minor_region`, and the new
  `collect_minor_mixed` below) increments `self.minor_streak = self.minor_streak.saturating_add(1)`.
- `auto_minor: bool` field (init `false`), with `set_auto_minor(bool)` / `auto_minor() -> bool`
  accessors — the barrier-coverage attestation gate from §2b.
- `pub fn should_collect_minor(&self) -> bool` — mirrors `should_compact`, gated by §2b's
  attestation:
  ```rust
  pub fn should_collect_minor(&self) -> bool {
      if !self.auto_minor || self.minor_streak >= self.max_minor_streak {
          return false; // unattested, or bound old-generation growth — force a full collect
      }
      matches!(
          AdaptivePolicy::default().evaluate(&self.profile),
          PolicyDecision::SuggestSwitch(GcAlgorithm::Generational, _)
      )
  }
  ```
  Same policy-priority deference `should_compact` already documents: `AdaptivePolicy::evaluate`
  returns exactly one recommendation, so `should_collect_minor` only fires when Generational is
  the *top* signal (Incremental didn't fire) — no separate ordering logic needed here.
- `pub unsafe fn collect_minor_mixed(&mut self, root_slots: &[usize], regions: &[(*const u8, usize)]) -> GcCycleStats`
  — the young-generation analogue of `collect_mixed`: mark every `root_slots` word and every
  `regions` candidate with `young_only = true`, then delegate to the existing `minor_finish`.
  Needed because the only minor entry points today take *either* explicit root values
  (`collect_minor`) *or* one conservative region (`collect_minor_region`) — neither matches the
  precise-slots-plus-conservative-regions shape `build_precise_roots` produces, which
  `collect_precise`/`collect_mixed`/`collect_compacting` already consume. Same safety contract as
  `collect_mixed` (every slot address and region span must be readable).
- `minor_finish` (the shared tail all three minor entries call) now also calls
  `adapt_threshold(prev_live)`, threaded in from each caller's pre-mark `self.live_bytes` — a
  review-caught gap: without it, a heap sitting over threshold after a minor cycle stayed
  `should_collect() == true`, re-walking the stack at *every* subsequent safepoint until a full
  collect eventually happened to run and adapt it. Every full-collect entry already does this;
  minor cycles now match.

### 3.2 `gc-core-capi`

- `__gc_collect_minor_precise() -> i64` — new `#[no_mangle]` entry, byte-for-byte the same
  spill/frame-walk/gate structure as `__gc_collect_precise`, calling
  `h.collect_minor_mixed(&slots, &regions)` instead of `h.collect_mixed(...)`. Always directly
  callable — the attestation gate (§2b) applies only to `__gc_safepoint`'s *automatic* choice to
  run a minor cycle, not to this explicit entry (an explicit caller is presumed to know its own
  barrier coverage, the same trust level `__gc_collect_minor`/`collect_minor` already carry).
- `__gc_set_auto_minor(on: i64)` / `__gc_is_auto_minor() -> i64` — thin C ABI wrapper over
  `FlatHeap::set_auto_minor`/`auto_minor` (§2b).
- `__gc_safepoint` priority, extended:
  ```rust
  pub unsafe extern "C" fn __gc_safepoint() -> i64 {
      if !with_heap(|h| h.should_collect()) { return 0; }
      if with_heap(|h| h.should_collect_minor()) {
          __gc_collect_minor_precise()
      } else if with_heap(|h| h.should_compact()) {
          __gc_collect_compacting()
      } else {
          __gc_collect_precise()
      }
  }
  ```
  Minor is checked *before* compacting, matching `AdaptivePolicy`'s own stated priority
  (Generational outranks Compacting) — consistent with `should_compact`'s doc comment, which
  already describes deferring to a higher-priority signal. `should_collect_minor` folds in the
  §2b attestation check, so this dispatch needs no separate gate of its own.
- No new `twig_compat` alias: the automatic enactment flows entirely through the already-aliased
  `__twig_gc_safepoint` → `__gc_safepoint`. (`collect_minor`/`collect_minor_region` themselves
  were never twig-aliased either — there is no existing native builtin that calls them directly,
  unlike `gc_collect_precise`/`gc_collect_compacting`, so there is no parity gap to close here.)

### 3.3 What does *not* change

- The Incremental recommendation stays advisory (§1).
- Nothing about `collect_minor`, the write barrier, or tenuring changes; this spec adds a
  *scheduling* decision on top of already-proven mechanism.

> **Update (follow-up PR, same day):** at initial merge, `vm-core`'s own `safepoint` opcode was
> listed above as unaffected. It no longer is — `vm-core` is the barrier-correct producer §2b/§6
> called out, and a same-day follow-up wired `run_safepoint` to check `should_collect_minor`
> (before `should_compact`, same priority as `__gc_safepoint`) and attested `set_auto_minor(true)`
> once in `VMCore::new()`. See `vm-core`'s own CHANGELOG (0.23.0) for the details; §6 below is
> updated to match.

---

## 4. Safety argument

- **Soundness of the minor collection itself** is unchanged — this spec adds no new collection
  algorithm, only a new call site for the existing, already-reviewed `collect_minor`/write-barrier
  machinery (generational arc, PR #8526 and follow-ups).
- **The attestation gate (§2b) is the load-bearing correctness piece for the automatic path.**
  `should_collect_minor` hardcodes `false` while `auto_minor` is `false` (the default), so
  `__gc_safepoint`'s minor branch is unreachable, and its behavior is byte-for-byte identical to
  before this spec, for every existing producer. Enabling it is a one-line, explicitly-documented
  action an embedder takes only after confirming its own barrier coverage — `gc-core` cannot
  verify this itself (the barrier is emitted, or not, in a different crate entirely), so this is
  an attestation contract, not a provable invariant; the risk of getting it wrong (a real UAF) is
  stated plainly on every touchpoint (the field, both setters, both doc-comment call sites).
- **The streak cap is the second correctness-relevant piece**, orthogonal to the gate above (a
  leak, not a UAF, and only reachable once `auto_minor` is already true). It is a
  monotonically-increasing counter reset only by an actual full collection, so old-generation
  garbage can accumulate for at most `max_minor_streak` consecutive paced collections before a
  full collect is forced — a hard, easily-tested bound, not a heuristic that can silently regress.
- **Degenerate inputs are safe by construction:** `set_max_minor_streak(0)` clamps to `1`
  (mirroring `set_tenure_age`'s `0`→`1` clamp), so `should_collect_minor` can be made to always
  defer to a full collect but never to lock in an infinite minor-only regime. `set_auto_minor`
  takes a plain `bool` (Rust) / any nonzero `i64` (capi) — no clamp needed, no invalid state
  representable.
- **`collect_minor_mixed` carries the identical safety contract** as `collect_mixed` (every slot
  address and region span readable) — no new unsafe reasoning, just the existing precise/region
  mark logic with `young_only = true` and the existing `minor_finish` tail.
- **Pacing correctness:** `minor_finish`'s new `adapt_threshold` call uses the same `prev_live`
  captured before the mark phase that every full-collect entry already uses — same formula, same
  caller-supplied input, no new safety-relevant logic.

---

## 5. Testing plan

- `gc-core` unit tests, mirroring `should_compact_follows_adaptive_policy_fragmentation_signal`:
  - **the attestation gate itself**: `should_collect_minor() == false` with every other condition
    (enough cycles, low survival ratio) satisfied but `auto_minor` still at its default `false`;
    `true` once `set_auto_minor(true)` is called — this is the direct regression test for the §2b
    security-review fix.
  - low survival ratio + attested → `should_collect_minor() == true`.
  - the streak cap: force `max_minor_streak` consecutive minor collections under a sustained
    low-survival, attested profile, assert the *next* `should_collect_minor()` call is `false`
    even though the policy would still recommend Generational; assert a full collect resets the
    streak.
  - priority: an attested profile where both Generational and Compacting signals would fire —
    Generational wins (mirrors the existing pause-outranks-fragmentation test for `should_compact`).
- `gc-core-capi`: `__gc_set_max_minor_streak`/`__gc_max_minor_streak` clamp test (mirrors
  `c_abi_set_and_get_tenure_age_clamps`); `__gc_set_auto_minor`/`__gc_is_auto_minor` default-off +
  round-trip test; plus an end-to-end smoke test for `__gc_collect_minor_precise` itself (called
  directly, not gated by `auto_minor` — see §3.2) — mirroring `__gc_collect_precise`'s own smoke
  test, plus the property that actually distinguishes it: a genuinely-unrooted OLD object survives
  (checked via `__gc_kind_of`, not a conservative-scan-dependent `freed` count — see lessons.md,
  the `freed` count from a raw stack scan is not deterministic in a debug build and is the wrong
  signal to assert a *specific* new behavior on). `__gc_safepoint`'s three-way dispatch itself is
  exercised by inspection plus the already-thorough `should_collect_minor`/`should_compact`
  gc-core unit tests it calls — matching the existing precedent that `should_compact`'s wiring
  into `__gc_safepoint` also has no dedicated capi-level dispatch-integration test.
- `cargo miri test -p gc-core` — clean, 116/116 (the crate this PR substantively changes).
  `cargo miri test -p gc-core-capi` fails independently of this PR: reproduced identically on
  unmodified `origin/main` in `precise_walk::tests::all_unmapped_frames_become_conservative_regions`
  (a Stacked-Borrows violation in a pre-existing synthetic-stack-walk test, `precise_walk.rs:176` —
  a file this PR does not touch). Flagged as a separate follow-up rather than fixed here.

---

## 6. Follow-up (not in this PR)

The mechanism this spec ships (`should_collect_minor`, `collect_minor_mixed`,
`__gc_collect_minor_precise`, the streak cap, the attestation gate) is complete and tested, but
**no producer can turn it on yet**: `vm-core` is barrier-correct but doesn't call
`__gc_safepoint`; the native/LLVM backends call `__gc_safepoint` but aren't barrier-correct. The
actual payoff — a real AOT-compiled program running cheaper minor collections automatically —
needs a follow-up that emits `__gc_write_barrier` from `field_store` lowering in
`aarch64-backend`, `x86_64-backend`, and `iir-to-llvm`, then calls `__gc_set_auto_minor(1)` once
per process (e.g. from the same `__gc_init_stackmaps`-style startup hook the precise-roots rung
uses) with a differential proving an old→young store survives a minor collection end-to-end on
real hardware. `vm-core`'s own `handle_safepoint` opting into `should_collect_minor` too (it is
barrier-correct today) is a smaller, independent follow-up.
