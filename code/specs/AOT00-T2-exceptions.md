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
