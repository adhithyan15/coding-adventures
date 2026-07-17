# AOT00 · T1 — Precise garbage collection (stack maps, exact roots, moving)

**Status:** Draft — design spec (spec-first north-star; sign-off = merge)
**Track:** AOT00 **T1 Runtime — GC** (see
[`AOT00-native-aot-robustness-roadmap.md`](AOT00-native-aot-robustness-roadmap.md)
§3 T1, §5 step 3). This is the "detailed spec" the roadmap's §3 promises for each
track; the roadmap file is the index, this is the T1 chapter.
**Builds on:** [`native-aot-substrate.md`](native-aot-substrate.md) (TWIG-GC Layer 1,
the *conservative* collector that ships today) and the IIR heap/GC opcode surface
in `interpreter-ir` (`alloc`, `field_load`, `field_store`, `is_null`, `safepoint`,
`alloc_closure`, `call_closure`).

---

## 0. One-paragraph summary

Today the native AOT backend reclaims heap memory with a **conservative**
mark-and-sweep collector (`twig_gc.c`): it scans the C stack word-by-word and
treats anything that *looks* like a managed pointer as a root. That is correct
(it never frees a live object) but it is the floor of the GC maturity ladder — it
**cannot move objects** (so it cannot compact or go generational), it **retains
garbage** whenever an integer aliases a heap address, and its root scan is O(stack
× live-set). **Precise GC** is the roadmap's "biggest single robustness lever": the
compiler emits, at each **safepoint**, a **stack map** naming exactly which stack
slots and callee-saved registers hold managed references, plus a **type/field map**
per heap object. With exact roots and exact interior pointers the collector knows
the true object graph — so it can **relocate** objects (compaction defeats
fragmentation), run **generationally** (a write barrier on `field_store` tracks
old→young edges so most collections touch only a small nursery), and eventually run
**concurrently**. This spec defines the contract, the on-disk/in-binary metadata
formats, the per-backend emission plan across all seven engines, and the PR ladder
that climbs conservative → precise → moving → generational, each rung gated by the
T7 differential harness.

---

## 1. Why conservative is the floor (what precise buys)

The shipping collector is deliberately conservative. Its own header comment is
honest about the trade: *"Every word on the C stack is treated as a potential
managed pointer … False positives cause live objects to be retained
unnecessarily."* That design has four structural ceilings, and each is a robustness
axis the roadmap's gate (§4 of the roadmap) wants raised:

| Ceiling (conservative) | Consequence | Precise unlocks |
|---|---|---|
| **Cannot move objects** — a false-positive root might be an integer we must not overwrite, so no pointer can ever be rewritten. | No compaction → fragmentation; no bump-pointer allocation → slower `alloc`; no generational nursery. | Exact roots ⇒ every reference is known and rewritable ⇒ **relocation** is sound. |
| **Retains floating garbage** — any stack integer whose bit-pattern lands in a live payload pins that object. | Memory high-water mark exceeds the true live set; adversarial input can inflate it. | Only real references keep objects alive ⇒ tighter heaps, adversarial-resistant. |
| **Whole-stack scan every cycle** — O(stack depth) per collection, every frame, every word, two probes per word (raw + tag-stripped). | Pause time grows with stack depth even when few roots exist. | Stack map ⇒ visit **only** the live-ref slots named for the current PC ⇒ pause ∝ live roots, not stack size. |
| **No interior-pointer precision** — payloads are scanned conservatively too. | A field holding a look-alike integer pins a phantom child. | Per-type **field map** ⇒ trace exactly the reference fields. |

Precise GC is **not** a rewrite of `twig_gc.c` — it is the same mark/sweep skeleton
with two conservative approximations replaced by compiler-supplied truth:
*stack scan → stack-map lookup* and *payload scan → field-map walk*. The conservative
collector stays in the tree as the **fallback** for any backend or frame that has no
map yet (§7), so the migration is monotone: no program regresses.

---

## 2. The precise-GC contract

Precise collection rests on one invariant, the **safepoint discipline**:

> **Collection happens only at safepoints, and at every safepoint the location of
> every live managed reference is statically known.**

The IIR already reifies the safepoint: opcode `safepoint` ("yield to GC if
collection pending; may_alloc"), emitted by frontends at loop back-edges and
function entries, lowered today to `__twig_gc_safepoint()`. Two more opcodes are
already GC-aware in their doc-comments — `field_store` notes it "may emit write
barrier" and an opcode comment already anticipates that "a GC safepoint is active
(live refs may be relocated by a moving GC)." The seam exists; T1 fills it.

The contract has three obligations, one per producer/consumer:

1. **Compiler (each backend) — emit maps.** For every safepoint (and every call
   site, which is an implicit safepoint because the callee may collect), record a
   **stack map**: the set of locations (stack offsets relative to the frame
   pointer, plus which callee-saved registers) that hold a managed reference at
   that program point. For every heap object *kind*, record a **field map**: which
   payload offsets are references.
2. **Runtime (collector) — consume maps.** At a collection, walk the call-stack
   frame by frame; for each frame look up the stack map keyed by its return address
   / safepoint id; read exactly those slots as the roots; trace object graphs using
   field maps; if moving, rewrite each traced location to the object's new address.
3. **Mutator (generated code) — honor barriers & spills.** Between safepoints the
   generated code may hold references in registers the map doesn't mention; that is
   fine because **no collection happens between safepoints**. At each safepoint the
   code must have spilled (or the map must name the register per its callee-saved
   ABI slot) every live reference. `field_store` into an old-generation object emits
   the **write barrier** that records the old→young edge.

If any of the three is missing for a given frame/object, that frame/object falls
back to conservative treatment (§7) — never to unsoundness.

---

## 3. Object model: the typed header (TWIG-ROM)

Precise tracing needs to know, given a heap pointer, **which payload words are
references**. That requires a per-object type tag and a table from tag → field map.
This is exactly `native-aot-substrate.md`'s **Layer 2 (TWIG-ROM)**; T1 promotes it
from "nice to have" to "required", and pins its layout.

The current `gc_header_t` is 32 bytes: `next(8) | size(8) | marked(1) | _pad(15)`.
T1 spends part of that padding on a type id and GC bookkeeping — **no size change**,
so the 16-byte payload alignment (needed by the NaN-box low-3-bits-clear invariant)
is preserved:

```
┌──────────┬──────────┬────────┬───────────┬───────────┬─────────────┐
│ next (8) │ size (8) │ mark(1)│ gen (1)   │ _rsv (2)  │ type_id (4) │
└──────────┴──────────┴────────┴───────────┴───────────┴─────────────┘
  0          8          16       17          18          20…24  (payload @ 32)
```

- **`type_id` (u32)** indexes the **type descriptor table** the compiler emits into
  the binary (a `.rodata` array). Descriptor `type_id → { n_fields, ref_offsets[] }`
  (byte offsets within the payload that hold references). `type_id == 0` is the
  reserved **opaque/blob** type: no reference fields (raw bytes — strings, byte
  tapes), traced as a leaf. Conservative fallback uses `type_id == TYPE_CONSERVATIVE`
  which means "scan this payload word-by-word" — the bridge that lets un-migrated
  allocations coexist.
- **`gen` (u8)** is the generation number (0 = young/nursery, 1 = old, …), unused
  until the generational rung (§6.3); zero-initialised, inert before then.
- Reference values on the native columns keep the existing tagged representation
  (low-3-bits HEAP tag for NaN-boxed dyn-values; raw aligned pointer for `alloc`),
  so `type_id` classifies the *object*, while the *reference's* own tag says whether
  a word is a reference at all. Both are needed: the field map says "offset 8 is a
  reference field," and the tag check confirms a particular word is a live tagged ref
  vs. an inline immediate stored in a union slot.

The descriptor table is produced from IIR type information already present at
`alloc`/`alloc_closure` sites (the `alloc` op carries a "kind K → ref<K>" hint;
closures have a known capture layout — see `ClosureTypeHint`). Frontends that do not
yet carry precise kinds emit `TYPE_CONSERVATIVE` and lose no correctness.

---

## 4. Stack maps: format and lookup

A **stack map** answers: *at this program counter, where are the live references?*

### 4.1 What a safepoint records

For each safepoint the compiler emits a record:

```
StackMapRecord {
  pc_offset      : u32   // offset of the safepoint/return address from fn start
  frame_size     : u32   // bytes; lets the walker find the caller's frame
  num_slots      : u16   // live reference slots at this PC
  slots[]        : LocDelta   // each: a stack offset (from FP) OR a saved-reg id
  callee_regs    : u16   // bitmask of callee-saved regs holding refs here
}
```

Records are grouped per function and sorted by `pc_offset` so the walker can
binary-search by return address. The whole table is a single `.rodata` section
(native), a custom section (WASM), or a side table keyed by method+ILoffset
(JVM/CLR — but those delegate; see §5). Encoding is delta-compressed (LEB128 offsets)
because most safepoints share most of their live set with the previous one — the
same technique LLVM's `stackmaps` and HotSpot's OopMaps use.

### 4.2 Root enumeration (the stack walk)

At collection time, `gc_mark()` is replaced (for mapped frames) by:

```
fp = current frame pointer
pc = return address of the collector's caller  // the safepoint that called us
while frame is mapped:
    rec = lookup_stackmap(pc)          // binary search by pc_offset
    for each slot in rec.slots:        // exact roots — no whole-stack scan
        root = load(fp + slot.offset)  // or a saved callee-reg
        push_root(root)
    pc = load(fp + return_addr_offset) // unwind one frame
    fp = load(fp + saved_fp_offset)
```

This replaces the conservative `gc_scan_region(sp … stack_base)` loop. Frames with
**no** record (e.g. a C runtime frame, or an un-migrated backend) are scanned
conservatively — correctness preserved, precision lost only there.

### 4.3 Call sites are implicit safepoints

A callee may allocate and thus collect, so **every call site** is a safepoint: the
caller's live references across the call must be in the map keyed by that call's
return address. This is the common case and is what makes precise GC "just work"
without the frontend peppering explicit `safepoint` ops — the backend inserts a
stack-map record at each call return address automatically.

---

## 5. Per-backend emission plan (all seven engines)

Cross-backend **agreement** is the matrix invariant (roadmap §4), so T1 must land on
every column without diverging observable behavior. The engines split into three
natural classes by who owns the collector:

| Engine | Collector | T1 work |
|---|---|---|
| **VM** (`vm-core`) | Rust `Vec<Value>` side-table heap | **Precise by construction.** A `Value` is a tagged Rust enum — the interpreter always knows which slots are references. No stack maps needed; "roots" are the live VM registers + globals. Already exact; T1 only aligns its *observable* GC semantics (when finalization/round-trips occur) with the native columns. |
| **JIT** (`jit-core`) | same as VM (cold interpretation) | Same as VM. |
| **JVM** (`iir-to-jvm-class-file`) | **host JVM GC** (already precise, moving, generational) | **Delegate.** Lower `alloc`→object/array allocation, `field_*`→`getfield`/`putfield`, refs are real JVM references the host GC already traces precisely. T1 = ensure we emit real reference types (not `long` handles) so the host GC sees them. No stack maps we author. |
| **CLR** (`iir-to-cil-bytecode`) | **host CLR GC** (precise, moving, generational) | **Delegate**, symmetric to JVM: emit managed object refs, `ldfld`/`stfld`, let CoreCLR's GC trace. |
| **NativeAot** (aarch64 / x86_64) | **`twig_gc.c`** (ours) | **The real T1 work.** Emit stack maps at call sites + `safepoint`s; emit the type-descriptor table from `alloc` kinds; teach `twig_gc.c` the precise mark path (§4.2) and, later, relocation + barriers. |
| **LLVM** (`iir-to-llvm`) | **`twig_gc.c`** (ours, linked in) | Use LLVM's **`gc.statepoint`/`gc.relocate`** intrinsics (or the shadow-stack `gc.root` for a first cut) so LLVM spills references at safepoints and lets us walk them; register our `twig` GC strategy name. First rung may use shadow-stack (simpler, non-moving) then graduate to statepoints (moving-capable). |
| **WASM** (`iir-to-wasm`) | **`twig_gc.c`** (in linear memory) or **Wasm-GC** | Two paths: (a) **shadow stack** in linear memory — the codegen spills live refs to a side stack the collector walks (portable, works on any wasm engine); or (b) target the **Wasm GC proposal** (`ref`/`struct`/`array`, host-collected) where available. Start with shadow stack for portability parity with the other linear-memory columns. |

**Consequence:** four of seven engines (VM, JIT, JVM, CLR) are *already* precise or
delegate to a precise host — T1's genuinely new engineering is the **three
linear-memory / native columns** (NativeAot, LLVM, WASM) that share `twig_gc.c`.
That is also exactly where the conservative collector lives today, so T1 is scoped
to one collector and three code emitters, not seven.

---

## 6. The precision ladder (rungs, each its own PR)

T1 is not one PR. It is a monotone climb; every rung keeps the matrix green and is
gated by T7 (§8). Conservative remains the fallback throughout.

### 6.1 Rung A — precise **non-moving** mark/sweep (roots only)

- Emit stack maps at call sites + `safepoint`s on **NativeAot** first (smallest,
  we own the whole pipeline).
- `twig_gc.c` gains `gc_mark_precise()`: walk frames via §4.2, read exact roots,
  but still **sweep in place** (no relocation). Payloads still scanned
  conservatively until §6.2.
- Fallback: any unmapped frame → existing conservative scan. So this rung is
  strictly *additive* precision — it can't regress.
- **Win measured:** floating garbage from stack-integer false roots disappears;
  root-scan cost drops from O(stack) to O(live-roots). A GC-stress differential
  (allocate N, drop, collect, assert `live_bytes` hits the true live set exactly)
  shows the conservative slack closing.

### 6.2 Rung B — precise **interior** tracing (field maps)

- Emit the type-descriptor table; stamp `type_id` in the header at `alloc`.
- `gc_scan_region` over a payload → `gc_trace_fields(type_id)`: visit only ref
  offsets. `TYPE_CONSERVATIVE` still scans word-by-word (fallback).
- Now the *entire* trace is exact for mapped objects: exact roots + exact fields.

### 6.3 Rung C — **moving** collector (compaction)

- With exact roots+fields, references are **rewritable**. Add a copying/compacting
  phase: relocate survivors, update every traced location to the new address.
- Requires the **read/write discipline** the IIR already anticipates ("live refs
  may be relocated by a moving GC"): between safepoints, code may cache a raw
  pointer; across a safepoint it must reload from a rooted slot. The backend
  guarantees this by keeping references in stack slots / statepoint-relocated SSA
  values across calls.
- LLVM column moves from shadow-stack `gc.root` to `gc.statepoint`/`gc.relocate`
  (statepoints yield the relocated SSA value, which is how LLVM supports moving GC).
- **Win:** compaction defeats fragmentation; bump-pointer allocation makes `alloc`
  O(1) with no free-list.

### 6.4 Rung D — **generational** (nursery + write barrier)

- Two spaces: a small **nursery** (gen 0) and an **old** space (gen 1). Most objects
  die young, so most collections trace only the nursery.
- The **write barrier** the IIR's `field_store` already reserves fires on
  old→young stores, recording the source in a **remembered set** so a nursery
  collection can treat those as roots without scanning the old space.
- `gc.threshold`'s adaptive logic (already in `twig_gc.c`) generalises to
  per-generation thresholds.

### 6.5 Rung E — **concurrent / incremental** (later, T3-adjacent)

- Out of T1's core; noted for completeness. Tri-color marking with a load/store
  barrier, safepoint-based handshake to pause mutator threads briefly. Depends on
  T3 (threads/safepoints). Listed so the header format (mark byte, gen byte) leaves
  room (`_rsv`) for a color field without another layout change.

---

## 7. Fallback & coexistence (why nothing regresses)

The design is a **strict superset** of the conservative collector:

- A frame with a stack-map record → precise root scan. A frame without one →
  conservative scan of that frame only. (Mixed stacks are fine: the walker switches
  per frame.)
- An object with a real `type_id` → field-map trace. An object stamped
  `TYPE_CONSERVATIVE` → word-by-word payload scan.
- Before **any** relocation rung (≤ §6.2) the collector never moves objects, so
  even a fully-conservative frame is safe. Relocation (§6.3) is only enabled once a
  build is **wholly mapped** (a link-time assertion: no `TYPE_CONSERVATIVE`, no
  unmapped safepoint), because moving requires *every* root to be precise. Builds
  that still contain conservative frames stay non-moving — they get rungs A/B
  precision but not C. This gate is per-build, checked at link time, and is how we
  ship precision incrementally without ever risking a moved object under a
  conservative root.

The conservative collector therefore is not deleted; it becomes the **safety net**
under a precision that fills in backend-by-backend, object-kind-by-object-kind.

---

## 8. Testing & gating (T7 is the harness)

Per the roadmap, T7 (conformance-at-scale differential harness) is T1's gate. Three
test layers:

1. **Agreement (existing T7).** Every generated program must still produce identical
   observables on all seven engines with precise GC on. GC is an *implementation*
   detail — turning it precise must not change any program's output. The existing
   `lang_matrix.rs` differential slices are the regression wall.
2. **GC-stress differential (new, T1-specific).** Generate allocation-heavy programs
   (long lists, deep closure chains, churn loops with `safepoint`s) and assert (a)
   output agreement across engines and (b) on the native/LLVM/WASM columns,
   `__twig_gc_live_bytes()` converges to the **true** live set after a forced
   collection — the metric that proves precise beats conservative (conservative
   over-retains; precise hits it on the nose). This directly exercises §6.1's win.
3. **Moving-safety property (rung C+).** After each collection, assert every live
   object's contents are intact and every reference resolves (a checksum walk of the
   object graph pre/post collection). Catches a missed root or an un-rewritten
   interior pointer — the classic moving-GC bug — deterministically via the seeded
   PRNG.

A rung is "done" only when its layer is green across the matrix in CI (the native/
LLVM/WASM columns run locally + in CI where the toolchain is present; conservative
fallback keeps absent-toolchain rows valid).

---

## 9. Interaction with the rest of AOT00

- **L1 (DVAL substrate)** is the precondition the roadmap names ("depends on L1 so
  the value model is uniform") — precise tracing needs one value representation to
  classify. E6d completing the dynamic-value model on all seven engines is what
  unblocks T1 now.
- **T2 (exceptions/unwinding)** shares the **stack-walk** machinery: the same frame
  descriptors that enumerate roots also drive exception unwinding. T1's stack maps
  and T2's unwind tables should share one frame-descriptor format — design them
  together so the walker is written once.
- **T3 (concurrency)** consumes T1's safepoint handshake for a concurrent collector
  (rung E).
- **T8 (platforms)** must carry the stack-map/`.rodata` emission across win/mac/linux
  × x64/arm64 — the format is platform-neutral but the section emission is per-object-
  format (Mach-O / ELF / COFF).

---

## 10. Non-goals / honesty

- **Not** a from-scratch collector: T1 reuses `twig_gc.c`'s mark/sweep skeleton,
  linked list, adaptive threshold, and OOM-safe mark stack. It replaces two
  conservative approximations with compiler truth.
- **Not** delivering concurrent GC in T1 (that's rung E / T3). T1's core deliverable
  is precise + moving + generational on the three linear-memory columns.
- **Not** changing observable program behavior: precise GC must be output-invisible;
  T7 agreement is the proof. The only observable is memory/pause metrics.
- **Not** claiming this reaches GraalVM/CoreCLR GC maturity in one arc — those are
  decades of tuning. T1 gets us onto the *precise* rung of the ladder, which is the
  gate everything above (compaction, generational, concurrent) needs.

---

## 11. First PR (proposed)

Ship this spec (spec-first). Then rung A, minimally:

1. **NativeAot stack-map emission at call sites** (aarch64 first) → a `.rodata`
   table + a `__twig_stackmap_lookup` the collector can binary-search.
2. **`twig_gc.c` `gc_mark_precise()`** consuming that table for mapped frames, with
   the conservative scan as the per-frame fallback.
3. **GC-stress differential** (§8.2) proving `live_bytes` tightens on the NativeAot
   column while staying output-identical to the other six engines.

Each subsequent rung/back-end is its own PR under this spec, gated by T7, in the
roadmap's T1 slot.
