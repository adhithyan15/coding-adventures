# AOT00-T1 · Stack-map emission from the native backends

*Sub-spec of [`AOT00-T1-precise-gc.md`](AOT00-T1-precise-gc.md) §11 item 3. This
document is the detailed design of the one remaining rung of the precise-**roots**
ladder: making the native code generators **emit** the stack maps that the runtime
already knows how to consume.*

---

## 1. Where we are

The precise-root **runtime core ladder is complete** and merged:

| Rung | What it added | PR |
|------|---------------|----|
| interior-precise | `register_kind` ref-field maps | #8512 |
| generational | young/old + write barrier | #8526 |
| precise-roots core | `StackMapRecord` / `StackMapTable` / `frame_root_slots` / `collect_precise` | #8539 |
| registry | `__gc_register_stackmap` + `resolve(ret_addr)` (code-addr → record) | #8548 |
| mixed collect | `FlatHeap::collect_mixed(slots, regions)` | #8557 |
| walk logic | `precise_walk::build_precise_roots` (fp-chain → slots + regions) | #8566 |
| collect entry | `__gc_collect_precise()` (asm fp/sp capture → walk → `collect_mixed`) | #8571 |

So today a native program can already call `__gc_collect_precise()`, and it will
unwind its own frame-pointer chain and collect. **But no frame resolves against a
stack map**, because *no backend emits any records*. `resolve(ret)` returns `None`
for every return address, so every frame falls back to the conservative
`[fp, caller_fp)` region scan — correct, but no more precise than `__gc_collect`.

**This spec closes that gap:** the aarch64 and x86-64 backends compute, per compiled
function, the set of stack slots that hold live GC references at each safepoint,
serialize them as `StackMapRecord`s, and register them via `__gc_register_stackmap`
at image start-up. Then — and only then — precise roots actually fire in production.

### 1.1 What the backend already gives us (grounded)

The native backends make this *much* easier than a general JIT would, because their
frame model is deliberately simple:

- **Fixed slot-per-variable frames.** Each backend assigns every CIR variable one
  permanent 8-byte stack slot, keyed by name, monotonic and never reused
  (`aarch64-backend/src/lib.rs` `RegAlloc::slot_of` :147; `x86_64-backend/src/lib.rs`
  :236). A slot's offset is therefore **stable across the whole function** — there is
  no need to know *when* a value lives in which register.
- **Frame pointers are already established.** aarch64 emits `stp x29,x30,[sp,#-N]!`
  then `add x29, sp, #0` — so **`x29 == sp`** for the whole frame
  (`aarch64-backend/src/lib.rs` :451–464); x86-64 emits `push rbp; mov rbp, rsp; sub
  rsp, N` (:456–461). Mutator frames are thus walkable by
  `build_precise_roots`, and the slot offsets are already the FP-relative offsets the
  record format wants (see §3).
- **All references live on the stack at safepoints.** The slot-spill model keeps only
  transient scratch values in registers (x0–x2 / rax) and stores every dest back to
  its slot before the next instruction. At an instruction boundary — where every
  safepoint sits — no GC reference is register-only, so a record with
  `callee_saved_mask = 0` is accurate (the aarch64 `safepoint` lowering comment
  already relies on this: `aarch64-backend/src/lib.rs` :1350).
- **Reference-ness is already typed.** A value is a GC reference iff its IIR
  `type_hint` is a `ref<T>` string (`interpreter-ir/src/opcodes.rs` `is_ref_type`
  :29; prefix `ref<` :108). Parameters carry declared types too.

### 1.2 The four gaps to fill

1. **No `safepoint` op is ever generated.** The opcode exists
   (`interpreter-ir/src/opcodes.rs` :362) and aarch64 can lower it
   (`aarch64-backend/src/lib.rs` :1352 → `bl __twig_gc_safepoint`), but no frontend or
   pass inserts one. *Call sites* are also implicit safepoints (their return address
   must resolve), and none are recorded.
2. **No slot→ref-type table at the backend layer.** Ref-ness is scattered across
   per-instruction `type_hint`s; the backend must reconstruct slot→`ref<…>` by
   joining `RegAlloc.slots` with instruction dests + param types.
3. **No `pc_offset` capture.** Neither backend records the assembler position at a
   `bl`/`call`, which is exactly the `StackMapRecord.pc_offset` (return address minus
   function start) the registry keys on.
4. **No liveness.** Slots are never freed, so "which refs are live *at* this PC" is
   unknown. §4 gives a safe flow-insensitive rule (R1) that needs no liveness pass, and
   defers a real pass to a later refinement rung.

---

## 2. Design overview — the emission pipeline

```
  IIR function (typed)                     per-function StackMapPlan
  ──────────────────                       ─────────────────────────
   params + instrs         backend          records: [(pc_offset,
   with type_hints   ─────────────────►      frame_size, mask=0,
                       (§3 build plan)        slots=[fp-rel i32])]
                                                      │
                                   (§5 serialize to .rodata + startup ctor)
                                                      ▼
                              __gc_register_stackmap(func_start, func_len,
                                  n, pc_offsets, frame_sizes, null, counts, slots)
                                                      │
                            (runtime, already built)  ▼
                       __gc_collect_precise → resolve(ret) → frame_root_slots → collect_mixed
```

The backend produces a **`StackMapPlan`** (a Rust value) while it lowers a function;
a small emitter turns each module's plans into a read-only data table plus a start-up
constructor that registers them. The runtime side is untouched — this spec adds only
*producers* for an ABI that already has a *consumer*.

---

## 3. Naming a root slot (offsets are drop-in)

`StackMapRecord.slots` are **FP-relative byte offsets** (`i32`, may be negative;
`gc-core/src/flat_heap.rs` :1006). Both backends' slot offsets map in with **no
translation**:

- **aarch64:** slots are SP-relative and `fp == sp`, so a slot at `sp + k` is at
  `fp + k` — positive `i32` `k` (reserved `[sp+0]`=saved fp, `[sp+8]`=saved lr are
  never GC slots).
- **x86-64:** slots are already RBP-relative: slot *n* at `rbp - 8 - 8n`
  (`x86_64-backend/src/lib.rs` :248) → the negative `i32` offset directly.

So the record's `slots` vector is just the FP-relative offsets of the ref-typed slots
that are roots at a given PC. `frame_size` can be the backend's computed frame size
(informational for this rung; the walk does not need it because `fp` brackets the
frame) and `callee_saved_mask = 0` (all refs on stack — §1.1).

---

## 4. Which slots are roots at a safepoint (liveness, the hard part)

The precise answer is *"every slot holding a reference that is still live-after this
PC."* We have no liveness pass, so we adopt a **sound over-approximation** that needs
none, and refine later:

> **Rule R1 (flow-insensitive).** Every safepoint in a function names **every** stack
> slot that function ever uses to hold a GC reference.

Why R1 is **safe**: the named set is a superset of the live set at every PC by
construction, so a live root can never be missed. It can only *over*-approximate —
naming a reference slot that is dead at this particular PC — which retains floating
garbage for a cycle exactly as the conservative scan would. It is still **strictly
better** than the conservative scan, because it excludes **every non-reference slot**
(integers, floats, booleans) — the main win: a stack integer that look-alikes a heap
address no longer pins anything.

R1 is trivial to compute in the single lowering pass the backends already make:
collect the ref slots and the safepoint PCs independently, then join them at the end.
`gc_core::StackMapBuilder` implements exactly this.

> ### WARNING — why NOT the flow-sensitive "defined at or before" rule
>
> An earlier draft of this spec proposed *"a ref slot is a root at PC p iff its
> defining instruction appears at or before p in the function's linear instruction
> order."* **That rule is unsound and must not be used.** It silently equates the
> order the backend *emits* code with the order the machine *executes* it, which a
> backward edge breaks:
>
> ```text
>   loop_top:  call use(x)      <- safepoint emitted here, before x's definition
>              x = alloc()      <- slot defined AFTER it in emission order
>              b loop_top       <- on iteration 2+, x IS live at the safepoint
> ```
>
> The record for `loop_top` would omit a slot holding a live reference. And an
> **incomplete record is more dangerous than a missing one**: the walker treats a
> `resolve` hit as authoritative and *skips the conservative scan of that frame*, so
> the omission frees a live object rather than merely retaining garbage. The same trap
> springs for any backend that lays blocks out of execution order (outlined cold
> paths, tail duplication, landing pads emitted last).
>
> Because a builder cannot *detect* such a violation, `StackMapBuilder` is
> order-independent by construction rather than trusting the backend to avoid it.

**Refinement rung (later, own spec/PR):** a backward liveness pass over the CIR to
compute live-out sets per safepoint, shrinking R1's set to exactly-live — but only a
pass reasoning about *execution* paths, never emission order. It is a pure precision
gain (never a safety change) and is explicitly out of scope here (§8).

### 4.1 Safety contract on the backend

Getting a record *wrong* is worse than emitting none, so these are obligations:

1. **Spill before the safepoint.** Every GC reference live across a safepoint must be
   in a stack slot of the current frame there, and that slot declared. The builder
   describes *stack slots only* (`callee_saved_mask` is always `0`), so a reference
   kept solely in a callee-saved register across a call is named by **nobody** —
   neither the caller's record nor the callee's — and is freed while live. The
   slot-per-variable native backends satisfy this naturally (§1.1).
2. **Declare incoming reference parameters** — they arrive in registers and the
   prologue spills them.
3. **Declare only reference-typed slots** (a non-reference slot is a wasted root, not
   unsafe).

Naming a slot the executed path has not written yet is safe: every slot word goes
through the same validated candidate-pointer lookup as a conservative scan.

---

## 5. Safepoints, `pc_offset`, and registration

**Which PCs get a record.** Two kinds of safepoint, in priority order:

1. **Call sites (mandatory).** Every `bl`/`call` return address is a PC the walker can
   observe (it is exactly what sits at `[fp+8]` in the *caller's* frame). Each must
   have a record or the caller frame silently falls back to conservative. The backend
   captures the assembler offset **immediately after** the branch instruction (that is
   the return address) as `pc_offset`.
2. **Explicit `safepoint` ops (secondary).** Once a pass inserts `safepoint` at loop
   back-edges (§6), those also get records. Lower priority because a program that only
   ever collects *inside a call* (the common case — `__gc_alloc` is a call) is covered
   by (1) alone.

**`pc_offset` = return-address − function-start.** The backend tracks the byte length
of code emitted so far for the current function; at each safepoint it records that
offset. `StackMapTable::lookup` binary-searches these exact offsets
(`gc-core/src/flat_heap.rs` :1031), and `resolve(ret)` subtracts the function start
before looking up (`gc-core-capi/src/stackmap_registry.rs` :17), so the two agree by
construction.

**Registration.** For each function the emitter calls, once at start-up:

```
__gc_register_stackmap(func_start, func_len, n_records,
                       pc_offsets[], frame_sizes[], /*callee_masks=*/null,
                       slot_counts[], slots_flat[])
```

(`frame_sizes`/`callee_masks` may be null → read as zero;
`gc-core-capi/src/lib.rs` :244.) `func_start`/`func_len` come from the linker's symbol
for the function; the parallel arrays live in `.rodata`. A single generated
constructor (or an explicit `__gc_init_stackmaps()` the runtime start-up calls) walks
the module's function table and registers each.

---

## 6. Inserting `safepoint` ops (prerequisite, small)

Call sites need no new op (they are already branches). For loop-back-edge safepoints,
a tiny IIR pass inserts a `safepoint` op at each back-edge target so long-running
allocation-free loops still yield to the collector. This mirrors `twig_gc.c`'s
original `__twig_gc_safepoint` motivation and is a **separate, small PR** that can land
independently — the aarch64 lowering already exists (`:1352`); x86-64 needs the
matching `op == "safepoint"` handler added (today it only routes through the builtin
dispatch, §1.2 gap 1).

---

## 7. Frame-pointer requirement

The **mutator** frames are already walkable (§1.1 — both backends set fp). The
remaining requirement is on the **runtime collector crate** (`gc-core-capi`): its own
`__gc_collect_precise` frame must have a valid frame pointer for `current_fp()` to be
a correct anchor once maps exist (see `stack_scan.rs` `current_fp` docs and the
security review of #8571, finding Q1b). This is tracked as a **prerequisite of this
rung**: build `gc-core-capi` with `-Cforce-frame-pointers=yes` for the x86-64 targets
(guaranteed by ABI on `aarch64-apple-darwin`). Verify by disassembling
`__gc_collect_precise`. *(Background task already filed.)*

---

## 8. Non-goals (this rung)

- **A real liveness pass** — R1 (§4) ships first; exact live-out is a refinement rung.
- **References in callee-saved registers** — the slot-spill model keeps refs on the
  stack at safepoints, so `callee_saved_mask` stays 0; a register-allocating backend
  would need the mask, which is a later concern.
- **Moving / compacting GC** — precise roots are the *gate* for it, not delivered here.
- **LLVM / WASM columns** — this rung is the two hand-written native backends
  (aarch64 first, then x86-64). LLVM has its own `gc.statepoint` machinery and is a
  separate arc; WASM roots are handled by the host and are out of scope.
- **Changing observable behavior** — precise GC must stay output-invisible; the T7
  differential (§9) is the proof.

---

## 9. Proof obligations

1. **Output-invisible (T7).** The NativeAot column must stay byte-for-byte identical to
   the other engines across the generative differential harness (`AOT00-T7`). Precise
   roots change *only* memory/pause metrics, never program output.
2. **`live_bytes` tightens (GC-stress).** A GC-stress program that spills a
   non-reference integer look-alike next to a real reference must show the NativeAot
   column reclaim the look-alike-pinned garbage that the conservative column retains —
   the same differential shape used for the interior-precise rung (`AOT00-T1` §8.2),
   now on *roots*.
3. **Registry round-trip.** Unit test: build a `StackMapPlan` for a small function,
   register it, and assert `resolve(func_start + pc_offset)` returns the expected slot
   set (host-side `extern "C"`, like the existing `precise_walk` synthetic-stack
   tests).

---

## 10. PR breakdown (small, spec-first, each gated by T7)

1. **This spec.** (spec-first)
2. **`StackMapPlan` builder in a shared crate** (e.g. `codegen-core` or a new
   `gc-stackmap` helper): given a function's ordered instrs + params + the
   name→slot-offset map, produce records under Rule R1. Pure, unit-tested, no
   backend wiring yet.
3. **aarch64: capture `pc_offset` at call sites** and thread the name→offset map into
   the plan builder; expose the per-function `StackMapPlan` (still not emitted).
4. **aarch64: emit `.rodata` table + `__gc_init_stackmaps` start-up registration**;
   wire `twig-aot` start-up to call it. Registry round-trip test (§9.3).
5. **GC-stress differential** (§9.2) proving `live_bytes` tightens on NativeAot while
   T7 stays green. *This is the first PR where precise roots visibly fire.*
6. **force-frame-pointers** for `gc-core-capi` (§7) — may land earlier if #4 needs it.
7. **x86-64 parity:** `op == "safepoint"` lowering + `pc_offset` capture + emission.
8. **Back-edge `safepoint` insertion pass** (§6).
9. **Liveness-pass refinement** (§4) — exact live-out, own spec.

Each subsequent PR is gated by the T7 differential in the roadmap's T1 slot, exactly
as the runtime ladder was.

---

## 11. Summary

The runtime can already collect precisely; it just has nothing to resolve against.
This rung feeds it: the native backends' fixed, frame-pointer-anchored, name-keyed
slot model means a root map is a near-mechanical join of *slot offsets* (already
FP-relative) with *ref-typed dests* (already in `type_hint`), snapshotted at each
call site under a liveness rule (R1) that needs no analysis pass and is provably safe.
The first visible payoff — the GC-stress `live_bytes` differential — is four small PRs
away, and it turns the whole precise-roots ladder from *plumbed* into *load-bearing*.
