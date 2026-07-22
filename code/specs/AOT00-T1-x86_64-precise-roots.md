# AOT00-T1 — x86-64 precise-roots port (design)

Status: **design** (implementation-ready). Ports the aarch64 precise-roots GC emission
(increments A/B/C, PRs #8782/#8787/#8798) to the native **x86-64** path so
`__gc_collect_precise` is precise on Linux/Windows, not just aarch64 macOS.

Prerequisite already merged: **x86_64-backend 0.30.0** — `compile_function_with_globals_and_stackmap`
emits a `gc_core::StackMapRecord` per call-return safepoint (ref slots at negative
`[rbp − 8 − 8·slot]` offsets; safepoints from `PltRel32` reloc `patch_offset + 4`). This
spec covers the *twig-aot* half: injecting the entry wrapper + `__gc_init_stackmaps`
registration into the x86-64 object path.

This is written spec-first deliberately: unlike the aarch64 work, the x86-64 registration
codegen **cannot be executed on the aarch64 dev machine** — its only faithful validator is
the native x86-64 `ubuntu-latest` CI runner (§6). Nailing the three intricate pieces
(entry interposition, 8-arg ABI marshalling, RIP-relative `func_start`) before blind codegen
is the point.

---

## 1. What differs from aarch64 (the three real deltas)

The aarch64 emission (increment A/B/C) established the shape: a synthetic `__gc_aot_entry`
wrapper is the image entry; it calls `__gc_init_stackmaps` (which registers every function's
stack map) then the user entry, returning its result verbatim; the wrapper is itself
registered (empty map) so the whole generated call chain is precise. Three things change on
x86-64.

### Delta 1 — entry interposition: rename `main`, wrapper **is** `main`

aarch64 macOS produces a **standalone** Mach-O whose entry *symbol* (`_main`) the packager
points at the wrapper's `__text` offset — a pure offset redirect.

x86-64 is different: `compile_module_linux_x86_64_object` / `_windows_…` emit a relocatable
object that is linked with **`cc`** (`twig-aot/src/lib.rs` link path + `linux_x86_64_smoke`).
libc's crt0 provides `_start`, which calls the symbol **`main`** and routes `main`'s return
through `exit()` (so the exit code is `main`'s return). So the entry is a *named symbol*
(`main`), not an `e_entry` offset the object controls.

**Approach:** before compiling, rewrite the module —

- rename the user entry function `main` → `__twig_user_main` (a reserved name; guard it like
  the other reserved GC symbols), and update `module.entry_point`;
- emit the wrapper under the name **`main`** so libc's `_start` calls it.

The wrapper then `call`s `__gc_init_stackmaps`, `call`s `__twig_user_main`, and returns its
`rax` verbatim; crt0 turns that into `exit(rax)`. No packager change — the object simply
defines `main` = the wrapper and `__twig_user_main` = the old body. (For a program whose
entry is *not* named `main`, keep the entry name and interpose under that name instead.)

### Delta 2 — `__gc_register_stackmap` is an **8-argument** call; x86-64 spills to the stack

`__gc_register_stackmap(func_start, func_len, num_records, pc_offsets, frame_sizes,
callee_masks, slot_counts, slots_flat)` takes 8 args. aarch64 AAPCS64 has 8 GPR arg
registers (x0–x7) — one register per arg, trivial. x86-64 does **not**:

| ABI | GPR arg regs | args in regs | args on stack |
|-----|--------------|--------------|---------------|
| **SysV** (Linux) | `rdi, rsi, rdx, rcx, r8, r9` | 1–6 | 7 (`slot_counts`), 8 (`slots_flat`) |
| **MsX64** (Windows) | `rcx, rdx, r8, r9` | 1–4 | 5–8, **plus 32-byte shadow space** |

So the init codegen is **ABI-parameterised** (`X86_64Abi`, already threaded through
`compile_module_x86_64_to_text`). Per registered function, the call sequence is:

**SysV:**
```
mov  rdi, <func_start>          ; RIP-relative lea, see Delta 3
mov  rsi, imm(func_len)
mov  rdx, imm(num_records)
lea  rcx, [rip + pc_offsets]    ; RIP-relative to the data pool
xor  r8d, r8d                   ; frame_sizes = NULL
xor  r9d, r9d                   ; callee_masks = NULL
; stack args (pushed high→low so [rsp+0]=arg7, [rsp+8]=arg8), 16-aligned:
lea  rax, [rip + slots_flat] ; push rax     ; arg8
lea  rax, [rip + slot_counts]; push rax     ; arg7
call __gc_register_stackmap
add  rsp, 16                   ; pop the two stack args
```
**MsX64:** args 1–4 in `rcx/rdx/r8/r9`; `sub rsp, 32+32` (32 shadow + 32 for args 5–8, keep
16-aligned); `mov [rsp+32]=arg5 … [rsp+56]=arg8`; `call`; `add rsp, 64`. Stack stays
16-aligned at the `call` in both ABIs.

**Stack-alignment invariant:** the wrapper enters 16-aligned after `push rbp`; `__gc_init_stackmaps`
must keep `rsp ≡ 0 (mod 16)` at each `call __gc_register_stackmap` (SysV pushes exactly 2×8 =
16 bytes → still aligned; MsX64 reserves a 16-multiple). Get this wrong → a movaps in the
runtime faults. This is the single most error-prone part and the reason to spec it.

The three data arrays per function (`pc_offsets: u32[]`, `slot_counts: i32[]`,
`slots_flat: i32[]`) are compile-time constants, emitted as a **data pool** after the init's
`ret` (never executed) and addressed RIP-relatively — the x86-64 analogue of the aarch64
`adr`-to-embedded-data pool. Alignment: pool arrays must be ≥4-byte aligned for the registry's
`*const u32/i32` reads (pad the pool; function offsets are already ≥1-byte, so align to 4).

### Delta 3 — `func_start` via **RIP-relative `lea`**, not `adrp`/`adr`

aarch64 used `ADR` (PC-relative, byte-granular, base-independent) after ruling out `ADRP`
(page immediate can't be baked — [[feedback_adrp_page_immediate_cannot_be_baked]]). x86-64's
natural equivalent is **`lea reg, [rip + disp32]`**: `disp32 = target_off − (rip_after_lea)`,
which — exactly like `ADR` — is a link-time constant independent of the load base, because
`rip` and the target both carry the base and it cancels. No relocation, ±2 GiB reach.

Patched in a new **pass-2b** in `compile_module_x86_64_to_text` (mirroring the aarch64
pass-2b and the existing x86-64 cross-call pass-2): emit `lea reg, [rip+0]` placeholders,
record `(disp32_byte_offset, target_fn)`, and after `link()` set
`disp32 = target_off − (disp32_byte_offset + 4)` (the `+4` = distance from the disp32 slot to
the end of the `lea`, same convention as the existing `call rel32` patch). The pool `lea`s to
init-internal data are resolved at generation time (init-internal displacement is layout-
independent), like the aarch64 pool `adr`.

---

## 2. Reuse from the aarch64 implementation

Structurally identical (copy the shapes, swap the encoder + ABI):

- **`FnStackMap { name, len, records }`** and the `Vec<(u32 pc, Vec<i32> slots)>` record shape
  — reused verbatim; `compile_function_with_globals_and_stackmap` already returns matching
  records.
- **Reserved-symbol guard** — reject a user function OR entry named `__gc_aot_entry` /
  `__gc_init_stackmaps` / `__twig_user_main`.
- **Register the wrapper too** (empty ref-slot map, records from `call_return_offsets` of the
  wrapper bytes) — the increment-C fix ([[feedback_precise_walk_maps_every_frame_in_chain]]):
  otherwise the precise walk conservatively re-scans `__twig_user_main`'s frame (its caller,
  the wrapper, would be unmapped) and re-pins look-alikes.
- **Skip record-less functions** (leaves) — conservative frame, as before.

`call_return_offsets` on x86-64 can't post-scan (variable-width) — derive the wrapper's
safepoints the same way the backend does: from its `PltRel32` relocs' `patch_offset + 4`
(the wrapper makes exactly two calls: init + user entry).

---

## 3. Where the code goes (`twig-aot/src/lib.rs`)

1. `compile_module_x86_64_to_text`: switch pass-1 to
   `x86_64_backend::compile_function_with_globals_and_stackmap`; collect `fn_maps`.
2. Rename-entry transform (Delta 1) in `prepare_module_for_aot` or just before compile.
3. After pass-1: reserved guard; build wrapper (`main`) + its records; append wrapper to
   `fn_maps`; `build_gc_init_stackmaps_x86_64(&fn_maps, abi)` → (init bytes, ext relocs,
   `func_start` lea relocs); push init + wrapper to `fn_results`.
4. `link()` (already there) → offsets.
5. New **pass-2b**: patch each `func_start` `lea` disp32 from offsets.
6. Existing pass-2 patches the wrapper's two intra-module `call rel32`s in place (no change).

New encoder helpers likely needed in `x86_64-encoder` (additive, unit-testable):
`lea_rip_placeholder(reg) -> patch_offset`, raw `push`/`pop` (exist), `xor_r32_r32` (zero a
reg), `mov_r64_imm64`, and a `emit_data_u32` (raw pool word). Confirm against the encoder
before adding; prefer existing methods.

---

## 4. Proof obligations (mirror aarch64)

1. **Transparent** — every existing x86-64 execute test (`linux_x86_64_smoke`) still produces
   its exact exit code with the wrapper interposed. This runs on `ubuntu-latest` (§6) and is
   the increment-A-equivalent transparency proof.
2. **Registration ran** — a program returning `gc_stackmap_count()` exits `> 0` (needs the
   `gc_stackmap_count` builtin on the x86-64 backend too — add it, mirroring aarch64).
3. **`func_start` correct** — unit test decodes the emitted `lea` disp32 and asserts it
   resolves to the target function's `__text` offset (base-independent, like the aarch64
   `func_start_adr_resolves_to_target_offset`). Runs on any host.
4. **`live_bytes` differential** — the GC-stress program (i64 look-alike reclaimed by precise,
   pinned by conservative) exits `0` precise / `64` conservative, and the keep-a-ref variant.
   Runs on `ubuntu-latest`.

---

## 5. PR breakdown (small, each CI-validated)

- **PR-x1** (this spec).
- **PR-x2** — entry rename + wrapper + **no-op** `__gc_init_stackmaps` (increment-A analogue).
  Transparency proof #4.1 on CI. Small, low-risk.
- **PR-x3** — fill `build_gc_init_stackmaps_x86_64` (Deltas 2+3) + wrapper registration
  (increment B+C analogue) + pass-2b. Unit test #4.3 locally; differential #4.4 on CI.
- **PR-x4** — self-recursive-call safepoints (backend precision follow-up) + any encoder gaps.

---

## 6. Testing strategy — why CI, and the sim as an option

The aarch64 differentials run locally because the dev machine is aarch64 macOS. x86-64 has no
local execution here (no x86-64 Mach-O packager for Rosetta; ELF can't run on macOS). So:

- **Primary: `ubuntu-latest` CI** is native x86-64 Linux and already **executes** the twig
  x86-64 pipeline (`linux_x86_64_smoke` compiles → `cc`-links → runs → checks exit code). The
  transparency + differential tests are `#[cfg(all(target_os = "linux", target_arch =
  "x86_64"))]` and run there faithfully (real code, real runtime, real stack walk). This is the
  same fidelity as the aarch64 differential, just on a different runner — the authoritative
  validator for this arc.
- **Locally validatable now:** the `func_start` `lea` decode (#4.3) and the init/wrapper
  byte-structure — pure functions, no execution.
- **Optional future — the x86-simulator.** `code/packages/rust/x86-simulator` already runs the
  twig x86-64 column locally with a **host-call bridge** (`host_call` dispatches an external
  runtime symbol to a host Rust shim reading simulated `rdi/rsi/…`). Adding `__twig_gc_*` shims
  is straightforward, but faithful **precise** collection would need the collector to walk the
  *simulated* stack + heap. The clean way is a gc-core walk over an abstract memory image
  (`build_precise_roots` reading via a `read(addr) -> u64` accessor instead of raw pointers) so
  the sim runs the **real** collector logic against sim memory — not a re-implementation (which
  would risk sim/prod divergence, exactly the class of bug real execution caught in increment C
  [[feedback_precise_walk_maps_every_frame_in_chain]]). Worthwhile for a fast, portable,
  deterministic smoke test, but **not on the critical path** — CI native x86-64 is the ground
  truth. Track separately.

---

## 7. Risks

- **Stack alignment** at the `__gc_register_stackmap` call (Delta 2) — the top failure mode;
  covered by an explicit 16-alignment accounting per ABI + CI execution catching a fault.
- **Entry rename** interacting with `prepare_module_for_aot`'s existing passes (globals,
  closures, dyn-repr) — do the rename early and verify the passes are name-agnostic.
- **MsX64 shadow space** — easy to under-reserve; CI Windows (`windows-latest`) builds but does
  it *execute* the x86-64 tests? If not, MsX64 is build-validated only; note the gap and lean on
  SysV/Linux execution for the ABI-marshalling proof, treating MsX64 as a parallel encoding.
