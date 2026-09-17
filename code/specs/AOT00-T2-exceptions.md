# AOT00 · T2 — Structured exceptions & unwinding (throw/catch, unwind tables, traces)

**Status:** Draft — design spec (spec-first north-star; sign-off = merge)
**Track:** AOT00 **T2 Runtime — exceptions & unwinding** (see
[`AOT00-native-aot-robustness-roadmap.md`](AOT00-native-aot-robustness-roadmap.md)
§3 T2). The roadmap file is the index; this is the T2 chapter.
**Designed with:** [`AOT00-T1-precise-gc.md`](AOT00-T1-precise-gc.md) — T1 §9 calls
for T2 and T1 to **share one frame-descriptor format and one stack-walker**; this
spec honors that. Reuses debug info from [`LANG14-native-debug-info.md`](LANG14-native-debug-info.md)
and [`LANG25-native-aot-debugger-end-to-end.md`](LANG25-native-aot-debugger-end-to-end.md)
for stack traces.

---

## 0. One-paragraph summary

Today a recoverable error is a **trap**: an out-of-range `array_get`/`array_set`, an
`unbox` of null, a `real → integer` conversion that doesn't fit, or an integer
divide-by-zero **aborts** the program (`RunResult::Trapped`). There is no way for a
program to *catch* an error, run a cleanup, and continue — and no stack trace to say
*where* it happened. **T2** replaces "traps only" with **structured exceptions**:
IIR `throw`/`catch`/`landingpad` ops, a per-frame **unwinding table** describing
which catch handlers and cleanups (finalizers) cover each program range, a runtime
**unwinder** that walks the stack running cleanups and transferring control to the
matching handler, and **stack traces** built from the same frame descriptors. The
central design decision — the one the T1 spec insists on — is that **exception
unwinding and GC root enumeration are the same stack walk**: one frame-descriptor
format encodes *both* "where the live references are" (T1) and "what handlers/cleanups
cover this PC" (T2), so the walker is written once and both tracks stay in lockstep.
Existing traps become a built-in `Trap` exception, so nothing that traps today
changes behavior unless a program chooses to catch it.

---

## 1. Why traps are the floor (what structured exceptions buy)

Traps are correct-but-blunt: they guarantee memory safety (no out-of-bounds read)
by **ending the program**. That is the floor of the exceptions maturity axis, and
each of its limits is a robustness lever the roadmap wants raised:

| Limit (traps only) | Consequence | Structured exceptions unlock |
|---|---|---|
| **Uncatchable** — a trap aborts; no `catch`. | A library error kills the whole process; no graceful degradation. | `throw`/`catch` — a program recovers, retries, or reports. |
| **No cleanup** — abort skips `finally`/destructors. | Leaked OS resources (files, sockets), half-updated state. | The unwinder runs **cleanups** (finalizers) on every frame it unwinds. |
| **No location** — a trap says *what*, not *where*. | Debugging is guesswork; no actionable error. | **Stack traces** from frame descriptors + LANG14/25 debug info. |
| **No user errors** — only built-in traps exist. | Frontends can't model their own error types (ALGOL `alarm`, Lisp `error`, exceptions). | A first-class exception **value** any frontend throws and matches on. |

T2 is not a rewrite of the trap sites — it **generalises** them: a trap becomes
`throw Trap{kind}`, and a program that installs no handler sees exactly today's
behavior (unwind to top → abort with the same code). Catchability is purely
additive.

---

## 2. The exception contract

Structured exceptions rest on the **unwind discipline**:

> **A `throw` transfers control to the nearest dynamically-enclosing handler whose
> catch range covers the throw's PC, running every intervening frame's cleanups
> first; if no handler exists, the program aborts (today's trap behavior).**

Three obligations, one per producer/consumer — deliberately mirroring T1 §2:

1. **Compiler (each backend) — emit unwind tables.** For every function, record the
   **handler/cleanup ranges**: which PC intervals are covered by a `catch` (and for
   which exception *kind*), and which have a **cleanup** (finalizer) that must run
   while unwinding through them. Emitted alongside — and sharing the frame layout of
   — T1's stack maps (§4).
2. **Runtime (unwinder) — consume them.** On `throw`, walk the stack frame by frame
   (the **same walk** T1 uses to enumerate roots): at each frame look up its unwind
   record by return address; run any cleanup; if a matching catch is found, restore
   that frame's SP/FP and jump to its landing pad; else continue to the caller.
3. **Mutator (generated code) — mark ranges & landing pads.** `catch`/`landingpad`
   ops delimit the covered ranges and name the recovery block; `throw` raises. Object
   lifetimes that need cleanup (a runtime string, an open handle) register a cleanup
   over their live range.

If a frame has no unwind record, it is treated as **cleanup-less and handler-less**
(unwind straight through) — never as unsound. Absent tables anywhere ⇒ the throw
propagates to the top and aborts, i.e. the trap fallback.

---

## 3. The shared frame descriptor (T1 ∪ T2)

This is the spec's keystone, and the reason to design T2 now, while T1 is fresh.
A **single per-safepoint / per-call-site frame descriptor** carries both tracks'
metadata:

```
FrameDescriptor {              // keyed by return address / safepoint id
  // ── T1 (GC) ────────────────────────────────
  frame_size   : u32           // to find the caller's frame
  gc_slots[]   : LocDelta      // stack offsets / regs holding managed refs
  // ── T2 (exceptions) ───────────────────────
  cleanup      : opt<PadRef>   // finalizer landing pad for this frame, if any
  catches[]    : { kind_mask, catch_pad : PadRef }   // handlers covering this PC
}
```

- **One walker.** `walk_frames(fp, pc)` yields `(frame, descriptor)` pairs. T1's
  collector reads `gc_slots`; T2's unwinder reads `cleanup`/`catches`. Neither track
  re-implements frame walking, return-address lookup, or FP-chain unwinding — a class
  of subtle bugs (off-by-one frames, wrong SP restore) written and tested **once**.
- **One emitter seam.** Each backend emits *one* table per function; T1 fills the
  `gc_slots` columns, T2 fills the `cleanup`/`catches` columns. They cannot drift out
  of sync because they are the same record.
- **GC during unwind is safe.** A cleanup may allocate (e.g. build an error string),
  triggering GC mid-unwind. Because the same descriptor names the live refs at that
  point, the collector sees correct roots even with a half-unwound stack — the
  invariant an *ad hoc* separate exception table could not guarantee.

Frames the descriptor doesn't cover fall back to conservative GC (T1 §7) **and**
cleanup-less unwind (§2) — the two fallbacks compose.

---

## 4. Unwind tables: format and lookup

### 4.1 What a function records

Per function, a table of records sorted by `pc_offset` (shared with T1's stack-map
table — §3):

```
UnwindRecord {
  pc_lo, pc_hi : u32           // the covered PC range within the function
  cleanup_pad  : opt<u32>      // offset of the cleanup block, if any
  n_catches    : u16
  catches[]    : { kind_mask : u32, handler_pad : u32 }
}
```

`kind_mask` is a bitset over exception **kinds** (§6): `Trap`-family (bounds, null,
div0, conv), plus frontend-defined kinds allocated from a registry (the same
`KindRegistry` idea T1/gc-core use for heap kinds — one registry mechanism, two
uses). A `catch` with an all-ones mask is "catch anything". Encoding is
delta-compressed (LEB128), like the stack maps.

### 4.2 The unwind walk (two-phase)

Standard two-phase unwinding (as Itanium C++ ABI / SEH use), so cleanups only run
once a handler is known to exist:

```
throw(exc):
  # Phase 1 — SEARCH: find the target frame without touching state.
  (fp, pc) = current
  while frame is mapped:
      rec = lookup_unwind(pc)
      for c in rec.catches:
          if c.kind_mask covers exc.kind: target = (fp, c.handler_pad); goto phase2
      (fp, pc) = caller_of(fp)            # same caller_of() as the GC walk
  abort(exc)                              # no handler → trap behavior

  # Phase 2 — CLEANUP: unwind to target, running cleanups.
  (fp, pc) = current
  while (fp, pc) != target frame:
      rec = lookup_unwind(pc)
      if rec.cleanup_pad: run_cleanup(fp, rec.cleanup_pad)   # may alloc → GC-safe (§3)
      (fp, pc) = caller_of(fp)
  restore SP/FP to target; jump to target.handler_pad with exc bound
```

Two phases matter because a cleanup that itself throws, or a program that wants
"is this catchable?" semantics, must not have already destroyed state — the search
phase is side-effect-free. (A first backend cut may collapse to one phase where the
language has no such subtlety; the format supports both.)

---

## 5. Per-backend plan (all seven engines)

Cross-backend **agreement** is mandatory (roadmap §4): a `throw`/`catch` program must
produce identical observables everywhere. The engines split by who owns unwinding:

| Engine | Unwinder | T2 work |
|---|---|---|
| **VM** (`vm-core`) | Rust control flow | Model an exception as a `Result`/dedicated `Unwind(exc)` signal threaded through the dispatch loop; `catch` frames pop to the handler; cleanups run on the way. Already precise; T2 aligns *observable* semantics with the native columns. |
| **JIT** (`jit-core`) | same as VM | Same as VM. |
| **JVM** (`iir-to-jvm-class-file`) | **host JVM exceptions** | **Delegate.** Lower `throw`→`athrow`, `catch`→a `try`/`catch` with an exception-table entry, cleanups→`finally`. Map IIR exception kinds to JVM classes. The host already gives stack traces. |
| **CLR** (`iir-to-cil-bytecode`) | **host CLR exceptions** | **Delegate**, symmetric: `throw`, `.try`/`catch`/`finally` clauses, exception types. |
| **NativeAot** (aarch64 / x86_64) | **ours** (`gc-core`/runtime unwinder) | **The real T2 work.** Emit unwind records into the shared frame table; a runtime `__unwind_raise(exc)` performs the two-phase walk (§4.2) reusing the GC stack-walker. Cleanups reuse the runtime's calling convention. |
| **LLVM** (`iir-to-llvm`) | **ours**, or LLVM's `invoke`/`landingpad` | Two options: (a) emit `invoke` + `landingpad` + `resume` and let LLVM's unwinder drive our personality function; or (b) a first cut using `setjmp`/`longjmp` per `catch` scope (simple, non-zero-cost) then graduate to (a). Start with the personality-function path to share the native runtime's tables. |
| **WASM** (`iir-to-wasm`) | **Wasm EH** (`try`/`catch`/`throw`) or shadow unwinder | Where the **exception-handling proposal** is available, lower to `try`/`catch`/`throw` + tags. Otherwise a portable **shadow unwinder**: a side stack of active handler scopes the codegen pushes/pops, walked on `throw` — parity with the other linear-memory columns. |

**Consequence:** four of seven engines (VM, JIT, JVM, CLR) delegate to control flow /
a host exception mechanism — T2's genuinely new engineering is the **three
linear-memory / native columns** (NativeAot, LLVM, WASM), *exactly the same three*
that T1 touches, sharing *exactly the same* frame descriptors and stack-walker. T1
and T2 are one runtime effort split across two tracks.

---

## 6. Exception value model & the trap migration

- An **exception** is a heap object (a `gc-core` heap kind — §3's registry): a `kind`
  tag + optional payload (message string, offending index, …). It is a GC root while
  in flight (the unwinder holds it), traced precisely because it is an ordinary heap
  object.
- **Built-in `Trap` kinds** cover today's trap sites — `Bounds`, `Null` (unbox),
  `DivZero`, `ConvRange` (real→int) — so the migration is behavior-preserving:
  - Each existing trap site emits `throw Trap{kind, detail}` instead of aborting.
  - A program with no handler unwinds to the top and aborts **with the same exit
    signal it has today** (T7 agreement proves this).
  - A program that installs `catch Trap` can now recover — the new capability.
- **Frontend kinds** (ALGOL fault handling, a Lisp `condition`, a real exception
  type) register their own kinds and throw/catch them; the mechanism is generic.

---

## 7. Stack traces

Because the unwinder already walks frames with descriptors, a **stack trace** is that
walk plus a PC→source mapping:

- Each `FrameDescriptor`'s return address maps to `(function, line)` via the LANG14
  native debug-info sidecar / LANG25 DWARF, exactly as the debugger consumes it.
- On an uncaught exception (before abort), the runtime walks once to render
  `kind: message` + `  at fn (file:line)` per frame — a real, actionable error where
  today there is only an exit code.
- Managed columns (JVM/CLR) get traces from the host for free; the native/LLVM/WASM
  columns build them from the shared descriptors, so trace *content* agrees where the
  debug info is present.

---

## 8. Testing & gating (T7 is the harness)

Per the roadmap, T7 (conformance-at-scale differential harness) gates T2. Layers:

1. **Trap-migration agreement (existing T7).** Every generated program that traps
   today (bounds, null, div0, conv-range) must, with T2 on and no handler installed,
   abort with the **identical** observable on all seven engines. Making traps into
   uncaught exceptions must be invisible. The existing `lang_matrix.rs` trap cells
   (`Expect::Trap`) are the regression wall.
2. **Throw/catch differential (new, T2-specific).** Generate programs with random
   nested `try`/`catch`/`throw` (and a mix of caught vs. escaping kinds) and assert
   identical printed observables across engines — the control-flow analogue of the
   T7 loop/branch slices, exercising handler selection, unwind depth, and cleanup
   ordering.
3. **Cleanup-ordering property.** A program that pushes N cleanups and throws past
   them must run them **in reverse order exactly once** — asserted by having each
   cleanup print a token; the sequence must match across engines. Catches the classic
   unwinder bugs (skipped/duplicated cleanup, wrong frame boundary).
4. **GC-during-unwind property (T1×T2).** A cleanup that allocates while unwinding
   must not corrupt the heap — a checksum walk (T1 §8.3) taken across a throw whose
   cleanups allocate, proving the shared descriptor keeps roots precise mid-unwind.

A rung is "done" only when its layer is green across the matrix in CI.

---

## 9. Interaction with the rest of AOT00

- **T1 (precise GC)** — shares the frame descriptor, the stack-walker, and the kind
  registry (§3). T2 should land *after or alongside* T1's frame-descriptor format is
  fixed, so both fill one record. This is the dependency that motivates specifying T2
  now.
- **T3 (concurrency)** — per-thread unwinders + exception state; the unwinder must be
  re-entrant and thread-local. Out of T2's core.
- **T4/T5 (whole-program / optimization)** — unwind tables participate in DCE (drop
  handlers for kinds never thrown) and must survive inlining (merge caller/callee
  ranges). Noted for those tracks.
- **T8 (platforms)** — unwind tables are emitted per object format (`.eh_frame`/
  `.gcc_except_table` on ELF, `.pdata`/`.xdata` on COFF/SEH, custom section on WASM);
  the *format* here is platform-neutral, the *emission* is per-platform (as with T1's
  stack maps).

---

## 10. Non-goals / honesty

- **Not** zero-cost in the first cut: a `setjmp`/`longjmp` or shadow-unwinder start is
  acceptable to get catchability; zero-cost table-driven unwinding (no cost on the
  non-throwing path) is the graduation target, not the entry point.
- **Not** changing trap-only programs' behavior: uncaught exceptions abort with
  today's observable; T7 agreement is the proof. New capability is only visible when a
  program installs a handler.
- **Not** delivering condition systems / resumable exceptions (Lisp-style restart) in
  T2 core — the value model leaves room, but resumption is a later slice.
- **Not** a separate stack-walk from T1: if T2 grows its own frame walker, that is a
  design failure — §3 is the whole point.

---

## 11. First PRs (proposed)

Ship this spec (spec-first). Then, gated on T1's frame-descriptor format existing:

1. **IIR ops + exception value model** — add `throw`/`catch`/`landingpad` to
   `interpreter-ir` and the `Trap`-kind built-ins; no backend changes yet (ops are
   inert until lowered). Unit-tested at the IIR level.
2. **VM/JIT throw/catch** — model unwinding in the `vm-core` dispatch loop
   (`Unwind(exc)` signal + handler frames + cleanup ordering). The reference oracle
   for the differential.
3. **Trap → `throw Trap{…}` migration on VM/JIT** — existing trap sites raise
   catchable exceptions; T7 trap-agreement (§8.1) proves no behavior change.
4. **Throw/catch differential harness** (§8.2) over VM/JIT/WASM.
5. **NativeAot unwind tables + runtime unwinder** reusing T1's frame walker; then
   LLVM (`invoke`/`landingpad`) and WASM (Wasm-EH / shadow), each its own PR under
   this spec, gated by T7.

Each subsequent rung/back-end is its own PR under this spec, in the roadmap's T2 slot.

---

## 12. Implementation sequencing (execution plan, 2026-09)

This section supersedes §11's sketch with a concrete, slice-by-slice plan, mirroring
how this initiative's T6 (non-ALGOL language completeness) track was actually
executed — each slice is independently mergeable, individually tested, and gated by
T7 agreement before the next one starts. **Do not attempt more than one slice per
PR.** A slice's status is tracked inline below (`PLANNED` / `LANDED — PR #N`) and
this section must be updated when a slice lands, so the plan stays truthful about
what's actually shipped vs. still pending — the same discipline T1's own spec
correction (top of `AOT00-T1-precise-gc.md`) models.

### Why this order (recap of §5's per-backend split, turned into a PR ladder)

§5 already establishes that four of seven engines (VM, JIT, JVM, CLR) either model
unwinding as ordinary control flow or delegate to a host exception mechanism, while
three (NativeAot, LLVM, WASM) need real unwind-table emission sharing T1's frame
descriptor. That structural fact drives the ladder: get the *ops* and the *reference
semantics* (VM/JIT) proven correct and differential-tested first — cheaply, with no
codegen — then propagate to the delegating hosts (JVM/CLR, also cheap — they hand
unwinding to a GC/runtime that already has it), and do the genuinely new engineering
(NativeAot/LLVM/WASM unwind tables) last, each as its own PR, because that is where
the shared-stack-walker risk with T1's GC lives and where a mistake is most
expensive to unwind (pun intended) if it ships broken.

### Slice 1 — IIR ops + exception value model (THIS PR)

**Status: LANDED — PR [#15419](https://github.com/adhithyan15/coding-adventures/pull/15419).**

- Add `throw` / `catch` / `landingpad` to `interpreter-ir::opcodes` (taxonomy only:
  `is_known_op`, `is_value_producing`, `has_side_effects`) — no crate outside
  `interpreter-ir` is touched, so no program that exists today can even construct
  one of these instructions, let alone execute it. This is the identical shape
  LANG28 used to land its 27 concurrency opcodes before any backend understood them
  ("Phase 28A — this file only").
- Add `interpreter_ir::exception_kind` — dotted-path exception kind names
  (`"Trap.Bounds"`, `"Trap.Null"`, `"Trap.DivZero"`, `"Trap.ConvRange"`, common
  ancestor `"Trap"`, catch-all `"*"`) plus `kind_matches()`, the reference semantics
  every later backend's `kind_mask` bitset must agree with.
- Extend `IIRModule::validate()`: a `catch`'s `try_end_label`/`handler_label` must
  resolve to a `label` in the same function (generalizing the existing
  undefined-branch-target check), and `handler_label` must be immediately followed
  by a `landingpad` — structural well-formedness, checked at the same layer that
  already catches duplicate function names and undefined jump targets.
- **Acceptance criterion (executable proof, not a design doc):** `cargo test -p
  interpreter-ir` — new unit tests assert (a) the three ops are recognised /
  correctly categorised, (b) `kind_matches` implements the exact-match /
  ancestor-match / catch-all / no-cross-sibling-match truth table in the module doc
  comment, (c) `validate()` accepts a well-formed hand-built try/catch IIR function
  and rejects each of: undefined `try_end_label`, undefined `handler_label`, and a
  `handler_label` not immediately followed by `landingpad`. All 104 existing
  `interpreter-ir` unit tests plus new ones, and all 41 doctests, pass unmodified —
  proving this slice is additive.
- **Zero behavior change for every existing program:** confirmed by `cargo check -p
  lang-aot` (which transitively compiles `vm-core`, `jit-core`, every `iir-to-*`
  backend, and every frontend) building clean with no changes outside
  `interpreter-ir` — nothing anywhere pattern-matches on these three new strings yet.
- **Not decided here (deferred, not a blocker):** the exact IIR encoding chosen for
  `catch` (three operands — kind name, try-end label, handler label — rather than a
  separate `catch_end`/region-object) is a technical call, made because it mirrors
  the existing `jmp`/`label` idiom this flat (non-basic-block) IR already uses
  everywhere else. It is *not* pinned by the T2 design spec (§0/§11 name the three
  op mnemonics but not their operand shape) and may be revisited in Slice 2 if the
  `vm-core` dispatch loop wants a different shape (e.g. an explicit handler-frame
  stack rather than scanning `catch` instructions inline) — that would be a
  same-crate follow-up, not a rethink of this slice's public surface.

### Slice 2 — VM/JIT throw/catch (the reference oracle)

**Status: PLANNED.**

- Model unwinding in `vm-core`'s dispatch loop: a `throw` produces an `Unwind(exc)`
  control signal (alongside today's normal/`ret` signals) that the dispatch loop
  propagates outward; a `catch` region pushes a handler descriptor (kind →
  handler_label) onto a per-call-frame handler stack that `landingpad` binds from
  when found.
- Cleanup ordering: for this first cut, "cleanup" == running any `catch` whose kind
  does *not* match but whose region *does* cover the unwind point — actually, more
  precisely per §2 obligation 2/§4.2: any frame the exception passes through without
  a matching handler needs a defined cleanup story. **Product/design question to
  resolve at the start of this slice** (not blocking Slice 1): whether v1 ships
  *only* `throw`/`catch` (no `finally`/destructor cleanups yet — cleanups deferred
  to a slice of their own) or ships both together. The T2 spec's own contract (§2
  obligation 2, §8.3 "cleanup-ordering property") treats cleanups as core, but VM/JIT
  can prove `throw`/`catch` handler-selection correctness *before* adding cleanup
  registration — recommend splitting: **Slice 2a = throw/catch handler selection
  only, Slice 2b = cleanup/finally ordering**, both still VM/JIT-only, both gated by
  T7, so neither PR is oversized.
- `jit-core`: same signal, since it falls back to the VM's interpreter loop for
  anything it hasn't compiled (per §5's "same as VM" row) — likely requires no new
  JIT-specific code in the first cut, only that JIT-compiled code correctly
  deopts/falls back across a `throw`.
- **Acceptance criterion:** a hand-built IIR program that `throw`s inside a `catch`
  region, is caught, and resumes execution after the handler — asserted by its
  printed output — passing identically under both the plain interpreter and the JIT
  path. This becomes the reference oracle every other engine's differential result is
  compared against.

### Slice 3 — Trap → `throw Trap{…}` migration (VM/JIT only)

**Status: PLANNED.** Depends on Slice 2.

- Each of the four existing trap sites (`array_get`/`array_set` bounds, `unbox`
  null, integer `div`/`mod` by zero, `real_to_int_*` range) emits
  `throw Trap.<Kind>{...}` instead of aborting, on VM/JIT only.
- **Acceptance criterion — T7 trap-agreement (§8.1):** every existing
  `lang_matrix.rs` `Expect::Trap` cell for VM/JIT must still observe **the same
  process-exit signal** when no handler is installed (the new `throw` unwinds all
  the way to the top of an empty handler stack and aborts exactly as today's trap
  does) — a direct diff against current CI baselines, not a new assertion. A new
  cell class (a `Expect::Trap` program wrapped in `catch "Trap"`) proves the new
  *capability*: the program now exits 0 having printed a recovery message instead of
  aborting.
- This is the slice where a real regression would be easiest to introduce silently
  (an exit-code or `stderr` text change on an existing trap program) — extra
  scrutiny/review weight here specifically.

### Slice 4 — Throw/catch differential harness

**Status: PLANNED.** Depends on Slice 2 (needs the reference oracle) and benefits
from Slice 3 (broadens the corpus with real trap-shaped programs) but does not
strictly require it.

- Extend `lang_matrix.rs` (or a sibling differential-test module) with generated
  nested `try`/`catch`/`throw` programs (mix of caught vs. escaping kinds, nested
  handlers, sibling-kind misses) over VM/JIT initially (WASM once Slice 7 lands).
- **Acceptance criterion:** printed observables agree bit-for-bit across every
  engine the harness currently covers, for every generated program, seeded
  deterministically (matching this repo's existing seeded-PRNG differential style).

### Slice 5 — NativeAot unwind tables + runtime unwinder

**Status: PLANNED.** Depends on Slices 1–4 (needs a proven reference semantics to
diff against) **and** on T1 Rung A's stack-map emission
(`AOT00-T1-stackmap-emission.md`) landing first on NativeAot, since this slice's
whole premise is reusing that same per-function frame-descriptor table (§3) rather
than growing a second one.

- Emit `UnwindRecord`s (§4.1) into the shared frame table at `catch` boundaries;
  implement `__unwind_raise(exc)` doing the two-phase walk (§4.2) via the *same*
  `caller_of()` step T1's GC root-enumeration walk uses.
- **Acceptance criterion:** a NativeAot-compiled throw/catch program from Slice 4's
  corpus produces output identical to the VM/JIT reference, **and** T1's existing GC
  differential/stress tests (`AOT00-T1-precise-gc.md` §8) stay green — i.e. this
  slice must not regress GC precision while extending the same walker. This is the
  slice the "be extra conservative" guidance in this task is really about; if it
  cannot be kept small, split further along §7's fallback boundary (e.g. land
  cleanup-less `throw`/`catch` — abort-with-trace on any frame needing a cleanup —
  before adding cleanup/finalizer support on native).

### Slice 6 — LLVM backend

**Status: PLANNED.** Per §5/§10: start with `setjmp`/`longjmp` per `catch` scope
(simple, non-zero-cost, proves correctness) before graduating to `invoke` +
`landingpad` + a personality function sharing NativeAot's tables (zero-cost,
follow-up slice). Two sub-slices, not one.

### Slice 7 — WASM backend

**Status: PLANNED.** Shadow-unwinder first (portable, parity with NativeAot/LLVM's
linear-memory model) per §5; Wasm-EH (`try`/`catch`/`throw` proposal) as a later,
engine-capability-gated follow-up, exactly as `iir-to-wasm` already gates other
proposal-dependent features.

### Slice 8 — JVM delegate

**Status: PLANNED.** Lower `throw`→`athrow`, `catch`→ a real exception-table entry,
cleanups→`finally`; map IIR kind names to a small fixed hierarchy of generated JVM
exception classes (one per built-in `Trap.*` kind plus a generic
`FrontendException` carrying the kind name as a string field for anything else).
**Product decision to flag:** whether frontend-defined kinds (a Lisp `condition`, an
ALGOL `alarm`) get their own generated JVM class per kind, or all share one carrier
class distinguished by the string field — affects what a JVM-side catch clause can
express and is a real API-shape choice, not a technical implementation detail.

### Slice 9 — CLR delegate

**Status: PLANNED.** Symmetric to Slice 8 (`throw`, `.try`/`.catch`/`.finally`
clauses); same product decision about per-kind vs. shared exception types applies
and should be answered once, consistently, across both slices 8 and 9.

### Slice 10 — Stack traces (LANG14/25 integration)

**Status: PLANNED.** Per §7, this is nearly free once Slice 5 lands (native
frame descriptors already resolve return addresses; LANG14's debug-info sidecar
already maps those to `(function, line)` for the debugger) — wire the uncaught-
exception path to render `kind: message` + `  at fn (file:line)` per frame before
aborting, instead of a bare exit code. JVM/CLR (Slices 8/9) get traces from the host
for free and need no work here beyond confirming trace *content* is comparably
useful (not byte-identical — formats legitimately differ per §7).

### Product/architecture decisions this plan surfaces (not resolved here)

Per this task's explicit ask, three decisions in the slices above are real product
calls, not technical details this loop should resolve unilaterally:

1. **Slice 2:** whether v1 ships cleanups (`finally`/destructors) alongside
   `throw`/`catch` or as a follow-on slice (2b). Recommendation above: split them.
2. **Slices 8/9:** whether a frontend-defined exception kind gets its own generated
   host exception class (JVM/CLR) or shares one generic carrier distinguished by a
   string field. This determines what a *host-language* (Java/C#) caller catching
   across an FFI boundary can express, which is a user-facing API surface, not an
   implementation detail.
3. **Not in this plan at all, flagged per §10's own non-goal:** how frontends with an
   *existing*, differently-shaped error model — ALGOL's `alarm` (a labelled restart
   point, closer to Common Lisp's `handler-bind`/restarts than to `try`/`catch`) and
   Lisp's own `condition`/`error` system (which the language itself may eventually
   want *resumable* semantics for) — should map onto `throw`/`catch`/`landingpad` at
   all. §10 explicitly excludes resumable/condition-system semantics from T2's core.
   Forcing ALGOL's `alarm` through a one-shot `throw`/`catch` mechanism may be either
   the right generalisation or a lossy fit; that call belongs to whoever owns the
   ALGOL/Lisp frontends when their slice comes up, not to this plan.

### Explicitly out of scope for the whole T2 track (per §10, restated for the plan)

Resumable/condition-system semantics, per-thread unwinder state (T3's job), and
unwind-table participation in DCE/inlining (T4/T5's job) are not part of any slice
above — each is either already a named non-goal (§10) or belongs to a different
track (§9).
