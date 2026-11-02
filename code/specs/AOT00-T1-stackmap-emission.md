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
   prologue spills them. "Reference parameter" means *may hold a heap word* by its
   **declared** type, not by the type the specialiser computes with: a dynamic
   (`any` / `polymorphic`) parameter is one tagged word that is as often a pair or
   closure as an integer, so it is a root even when arithmetic reads it as `i64`.
   The backends derive param root-ness from `FunctionContext.params` (deny-list:
   only machine scalars are non-roots), so a driver must hand them the declared
   types. **Divergence fixed in #18492:** twig-aot used to retype `any` params to
   `i64` in place (for signed comparisons), which dropped every Twig parameter from
   every stack map. Non-moving collections were unaffected — allocation-triggered
   ones scan the stack conservatively, and under `gc_collect_precise` the caller's
   `any` temporary still rooted the argument — but a compacting collection inside
   the callee moved the argument, rewrote the caller's slot, and left the callee's
   parameter pointing at the freed from-space copy (a use-after-free, also for a
   closure's captured values, which reach the lifted body as parameters). twig-aot
   now types a *copy* of the function with those params read as `i64`
   (`specialisation_view`) and builds `FunctionContext` from the declared params,
   so the machine code is unchanged and the parameter's slot is named at every
   safepoint.
   **The same holds for every local that may hold a heap word**, whatever type the
   specialiser picks for the instruction that writes it. A `let` binding, each arm
   of an `if` writing the join variable, and a copy of a call result whose type was
   unknown (a recursive call) are all `mov`s that propagation or
   `default_any_to_i64` type `i64`, so the follow-up to #18492 found their slots
   unrooted too: compaction rewrote the original and left the copy stale. twig-aot's
   `root_heap_word_copies` now marks, after specialisation, every variable that is a
   non-scalar-typed param, the dest of a root-typed instruction, or a `mov_*` copy of
   such a variable (to a fixed point), and sets `ty = "any"` on the integer `mov_*`s
   that define it. Only `ty` changes, never the mnemonic, and the backends read `ty`
   only for the stack map (and for array element widths, which `mov` never has), so
   the code bytes are unchanged. Arithmetic results stay scalar. Regression tests:
   `twig-aot/tests/gc_param_roots.rs` (compacting and precise collections spliced
   into the callee for the parameter, a captured value, a `let` copy, a copy chain, a
   recursive call's result held in a `let`, and `if` joins of the parameter with a
   literal and with a fresh pair; a 4 MiB allocation-pressure run; and a valgrind
   memcheck run) and the `gc_param_root_tests` unit tests.
   **Every GC allocation is a root, whatever type it was given (#18766).** The
   copy rule above seeds only from instructions that are already root-typed, and
   the string lowering types `alloc_bytes` `i64` (a buffer address is "an integer"
   to the byte stores around it). The buffer is a gc-core block, so its slot and
   the `mov` alias that is the string variable were in no stack map, and a
   precise or compacting collection freed the buffer. A conservative scan does
   not cover for an unnamed slot: in a frame with a stack map the walker trusts
   the map and scans nothing else. `root_heap_word_copies` therefore first sets
   `ty = "any"` on every instruction that allocates a GC block and carries a
   machine-scalar type: the `alloc_bytes` and `alloc_array` ops, and
   `call_builtin` of `alloc_bytes`, `str_concat`, `str_slice`, `input_str` and
   `dyn_pair_new` (each checked against its runtime entry). The raw `gc_alloc`
   builtin is left to its declared type on purpose: no compiler emits it, and the
   precise-GC differentials use an `i64`-typed `gc_alloc` result as the
   non-reference look-alike a precise collection must reclaim. The copy rule then
   roots their copies. Only `ty` changes; no backend reads it on
   these instructions except for the stack map, so the code bytes are unchanged.
   (`dyn_pair_new` is on the list because `infer_types` keys types by variable
   name: a cons result whose destination is also written by an integer `const`
   takes the integer's `i64`.)
3. **Declare only reference-typed slots.** Declaring a slot that holds a plain
   integer is not free. Under a non-moving collection the word goes through the
   validated lookup, so an integer that happens to name a block only keeps it alive
   one more cycle. Under **compaction** a declared slot is *rewritten* when its word
   names a moved object: `FlatHeap::forwarded` matches the exact payload address
   `base` **and** the seven tagged forms `base | 1` … `base | 7` (`word & !7 == base`
   with a non-zero tag), so eight integer values per moved object would be silently
   changed into its new address. (An interior address is pinned instead, never
   rewritten.) The deny-list keeps the machine scalars out for that reason, but a
   dynamic `any` slot — a parameter, a copy, a call result — cannot be told apart
   from a reference by type: it holds a boxed integer (`n << 3`, which collides
   only with an exact `base`) or, where the boxed/raw confusion of #18480 leaves a
   raw integer, any of the eight. **Known limit, accepted:** such a slot is declared,
   so a Twig integer numerically equal to a moved object's address (or one of its
   tagged forms) is rewritten by a compaction. It needs a value the program computed
   to equal a live heap address; it disappears when LANG85 (one dynamic-value
   representation, spec PR #18494) makes every dynamic word uniformly tagged, so
   that a word's tag alone says whether it is a reference.

Naming a slot the executed path has not written yet is safe: every slot word goes
through the same validated candidate-pointer lookup as a conservative scan. (It is
not *precise*: frames are not zeroed, so such a slot can hold a stale copy of a
reference from an earlier, deeper call and keep a dead object alive for a cycle.
That stale copy is also why the bugs in §4.2 survived non-moving tests.)

### 4.2 Roots outside the stack: globals (#18765)

Stack maps describe frames. A compiled program's **globals** — every top-level
Twig `define` that a function reads — live in the `_twig_globals` data block (slot
`k` at byte `8·k`, addressed by `global_load` / `global_store`), which no stack map
names and no stack scan reaches. Until #18765 no collector rooted them:

| collector | an object only a global names |
|-----------|-------------------------------|
| precise (`gc_collect_precise`) | not marked → **freed**; the global dangles |
| compacting (`gc_collect_compacting`, or `__gc_safepoint` choosing to compact) | if a stack slot also names it (`main`'s slot for the `define`'s value does), **moved** and that slot rewritten; the global keeps the from-space address, which the sweep frees |
| conservative (`__gc_collect`, allocation-triggered) | not marked → **freed** |

Non-moving collections usually got away with it, because a stale word in some
named-but-unwritten frame slot (§4.1) held the address too. Compaction moves the
object and rewrites only the stack words, so it exposes the bug every time.

**Rule G (global roots).** At start-up the image registers its whole globals block
with gc-core-capi:

```text
__gc_register_global_roots(&_twig_globals, n_globals)
```

and every collection entry — `__gc_collect`, `__gc_collect_minor`, the precise,
minor-precise, incremental-start, compacting and minor-compacting bodies (so also
`__gc_safepoint` and the `__twig_gc_*` aliases) — appends the address of every
registered slot to the **precise root slots** it hands the heap, next to the
stack walk's. A global slot is thus treated exactly like a stack-map slot: its
word is marked, and a compacting collection writes a moved object's new address
back into it.

- **Emission.** The generated `__gc_init_stackmaps` makes the call, before its
  stack-map registrations, when the module has at least one global. The block's
  address is the one argument the system linker relocates, with the same reloc a
  `global_load` uses: `ADRP`+`ADD` (`ARM64_RELOC_PAGE21`/`PAGEOFF12`) on AArch64,
  `lea reg, [rip + _twig_globals]` (`PcRel32`) on x86-64. SysV passes `rdi`/`rsi`;
  Win64 passes `rcx`/`rdx` inside a 32-byte shadow-space reservation. User
  function code is unchanged; only `__gc_init_stackmaps` gains these instructions.
- **Why precise slots, not a conservative region.** A conservative root pins its
  target and, through the pinning wave, everything reachable from it, so a global
  holding a large list would never be compacted. Precise slots keep it movable. The
  price is the §4.1 item 3 known limit, now also for globals: a global holding a
  plain integer equal to a moved object's address (or a tagged form of it) is
  rewritten.
- **Why every slot.** Rooting only globals "that may hold a heap word" would make
  soundness depend on every store into a global being typed correctly — the
  property #18492 and #18766 showed does not hold. All slots are registered.
- **Registry contract.** `__gc_register_global_roots(base, count)` rejects a null
  or misaligned base, `count <= 0` or `> 2²⁴`, a wrapping range, and a range that
  overlaps a different registered block; registering the same block again is a
  no-op. The block must stay readable and writable while registered (a data
  section does). `__gc_reset` does not clear it (like stack maps);
  `__gc_global_roots_reset` does, for tests.
- **Minor and incremental collections.** A store into a global has no write
  barrier, so the global slots are roots of every minor cycle (that is how a young
  object only a global names is found). An incremental cycle snapshots them at
  `start` with the stack roots and gives them the same treatment as stack slots.

Regression tests: `twig-aot/tests/gc_global_and_buffer_roots.rs` (compacting and
precise collections spliced in front of the global read, with `main`'s copy kept
and cleared, for a pair, a list of pairs and a second global slot; a 4 MiB
allocation-pressure run; valgrind memcheck; and the #18766 string-buffer cases),
and gc-core-capi's `registered_global_slots_are_roots_of_every_collector`, which
parks objects where no stack word can reach them and checks the conservative,
precise and compacting collectors against an unregistered control.

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
the module's function table and registers each. Before the first function it
registers the module's globals block with `__gc_register_global_roots` (§4.2).

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
remaining requirement is on the **runtime collector crate** (`gc-core-capi`): the walk
needs one trustworthy *anchor* — a frame record `[fp] = caller fp`, `[fp+8] = return
address into the caller` — at the collect entry, and once maps exist a wrong anchor is
unsafe (security review of #8571, finding Q1b: a garbage `[fp+8]` aliasing a mapped
return address could exclude a live span).

The first version read the anchor from `rbp`/`x29` inside the Rust entry
(`current_fp()`) and relied on `-Cforce-frame-pointers=yes`. That is not enough on
every ABI: on x86-64 Win64 LLVM sets `rbp` with `lea rbp, [rsp+N]` even with the flag,
so `[rbp]` is a local, not a saved frame pointer (security review of #18337).

**Resolution (flag-independent):** every precise entry (`__gc_collect_precise`,
`__gc_collect_minor_precise`, `__gc_collect_incremental_start`,
`__gc_collect_compacting`, `__gc_collect_minor_compacting`, `__gc_safepoint` and the
`__twig_gc_*` aliases of them) is a **naked trampoline** that builds the record itself
and passes its address to the Rust body as the first argument:

| ABI          | trampoline                                                                 |
|--------------|----------------------------------------------------------------------------|
| x86-64 SysV  | `push rbp; mov rbp, rsp; mov rdi, rbp; call body; pop rbp; ret`             |
| x86-64 Win64 | `push rbp; mov rbp, rsp; sub rsp, 32; mov rcx, rbp; call body; add rsp, 32; pop rbp; ret` (32 = shadow space) |
| AArch64      | `stp x29, x30, [sp, #-16]!; mov x29, sp; mov x0, x29; bl body; ldp x29, x30, [sp], #16; ret` |

Each carries hand-written unwind info (`.cfi_*` on SysV/AArch64, `.seh_*` on Win64)
so debuggers and SEH can walk through it. `twig-aot` still builds `gc-core-capi` with
`-Cforce-frame-pointers=yes`; the anchor no longer depends on it, but it keeps any
frames between the trampoline's caller and the Twig code walkable. Verify by
disassembling `__gc_collect_precise` for each target (see `stack_scan.rs` §2b).

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
6. **Walk anchor** for `gc-core-capi` (§7): landed as `-Cforce-frame-pointers=yes` in
   `twig-aot` (#18337), then made flag-independent with frame-building trampolines.
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
