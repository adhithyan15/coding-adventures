# Changelog — `aarch64-backend`

## 0.36.1 - 2026-08-02 - fix stale "no GC" comment on cons-cell allocation

The comment block introducing the heap-cons-cell lowering still described
the pre-AOT00 allocator (`__twig_alloc_bytes`, "V1 leaks... no GC"), even
though the `alloc` op it sits directly above has allocated cons cells
through the GC-managed, movable `__twig_gc_alloc_pair` path since 0.34.0.
No behavior change — corrects the comment to match the code.

## 0.36.0 - 2026-07-31 - boolean array elements

The native array element-size helper now accepts `bool`, using the backend's
existing fixed word cell representation for erased `array<T>` values.

## 0.35.0 - 2026-07-30 - runtime string ordering builtin

Register `str_cmp` as a two-argument, i64-returning `V1_BUILTINS` helper. The
generic AArch64 call path now resolves it to `__twig_str_cmp`, whose runtime ABI
returns the shared lexical `-1`/`0`/`1` result.

## 0.34.0 - 2026-07-30 — records are movable: `alloc` → `__twig_gc_alloc_pair`

- The default (2-word pair) **`alloc`** op — the record/union constructor cell — now lowers to
  `BL __twig_gc_alloc_pair` (the movable `{0,8}` pair allocator) instead of the kind-0 conservative
  `__twig_gc_alloc`. A Twig record's two boxed-`any` fields are thereby traced precisely and the
  cell is **relocated** under a compacting collect, on par with cons cells / closures. An explicit
  non-pair `alloc` size keeps the conservative kind-0 path (unknown layout → a precise ref-map
  would be unsound). Unit test `pair_alloc_uses_movable_pair_allocator`.

## 0.33.0 - 2026-07-29 — `gc_register_ref_array_kind` builtin (frontend array GC, AOT00-T5 §7)

- **`gc_register_ref_array_kind` `BuiltinSig` row** (`(fixed, fixed_count, tail_from) -> kind_id`)
  — a `call_builtin` lowers to a `BL` to `__twig_gc_register_ref_array_kind` via the generic
  `__twig_<name>` dispatch (three args in x0/x1/x2), the C-ABI seam a language frontend's **array**
  type calls to declare its layout so the collector traces — and under compaction relocates — the
  array + its elements precisely instead of pinning them. Adding the row is the whole change (no
  per-name lowering). Test `gc_register_ref_array_kind_emits_external_twig_call`.

## 0.32.0 - 2026-07-27 — `gc_collect_incremental_*` builtin trio (frontend incremental GC, AOT00-T4 §6)

- Three new `V1_BUILTINS` entries — `gc_collect_incremental_start` (0 args, void),
  `gc_collect_incremental_step` (1 arg = budget, returns 1 done / 0 more),
  `gc_collect_incremental_finish` (0 args, returns freed count) — the bounded-pause collection
  cycle. The generic `__twig_<name>` `call_builtin` dispatch auto-emits `BL
  __twig_gc_collect_incremental_*` (no per-name lowering). A native program can now drive an
  incremental collection. New emission unit test asserts the three external reloc symbols.

## 0.31.0 - 2026-07-24 — `gc_collect_compacting` builtin (frontend GC.compact, AOT00-T3 §5)

- New `V1_BUILTINS` entry `gc_collect_compacting` (0 args, returns the freed count) — the
  moving/compacting collect. The generic `__twig_<name>` `call_builtin` dispatch auto-emits a
  `BL __twig_gc_collect_compacting`, so no per-name lowering is needed; it sits beside
  `gc_collect` / `gc_collect_precise`. A native program can now trigger a compaction. New
  emission unit test asserts the external reloc symbol.

## 0.30.0 - 2026-07-21 — V1_BUILTINS: GC collection + observability (AOT00-T1 increment C)

Four `call_builtin` entries the native GC-stress differential drives, resolving to the
`__twig_gc_*` aliases in gc-core-capi: `gc_collect` (forced conservative full collect,
void), `gc_collect_precise` (precise-roots frame walk, returns freed count),
`gc_live_bytes` (live payload bytes), and `gc_stackmap_count` (registered-function
count). No new lowering — the generic `call_builtin` marshaller emits the `BL`.

## 0.29.0 - 2026-07-20 — V1_BUILTINS: dyn_null_p (native `null?`)

Part of the fix restoring McCarthy-lisp list programs on the native-AOT / LLVM backends (`lang-aot` `lang_matrix`). See the umbrella commit for the full story: `null?` was never routed to a runtime call on the tagged native/LLVM path (breaking every cons-walk helper), `list-ref`/`assoc` unboxed a raw-int index/key (→ wrong element), a top-level `(null? …)` predicate result was unboxed instead of truthy-coerced, and cons-cell field access failed the JVM verifier. Verified end-to-end: native list-ref/assoc/length/reverse/append/null? all correct.
## 0.28.0 - 2026-07-18 (GC stack maps: `compile_with_globals_and_stackmap`)

Second implementation rung of `AOT00-T1-stackmap-emission.md`: the backend now
computes each function's **GC stack map** — the data that lets
`__gc_collect_precise` resolve a real frame instead of falling back to a
conservative scan.

- **`compile_with_globals_and_stackmap(ctx, ir, global_slots)`** — like
  `compile_with_globals`, but also returns one `StackMapRecord` per call return
  address naming the frame slots that may hold GC references. Built via
  `gc_core::StackMapBuilder` (Rule R1), so the ordering hazards are handled there.
  It mirrors `compile_with_globals` rather than `compile` deliberately: a function
  with a safepoint has by definition at least one call, whose `BL` is an unpatched
  placeholder until the linker applies its `ExternalReloc` — an entry point that
  dropped the relocs (or that could not accept `global_slots`) would hand back code
  that cannot be linked, with no signal.
- **Root-ness is a DENY-list, not an allow-list.** A slot is a root unless its type
  is a provable machine scalar (`u4…u64`, `i8…i64`, `f32`, `f64`, `bool`, `void`).
  Everything else — `any`, `str`, `ref<…>`, `array<…>`, and any type string added
  later — is treated as a potential reference.
  - This polarity is the whole correctness story. An earlier draft keyed root-ness on
    `ref<…>` and was **dead code in production**: `aot_core::specialise` validates
    every type against its own allow-list, which does not contain `ref<…>`, so every
    reference is erased to `"any"` before reaching this backend. That draft would
    have emitted records naming *nothing* — and an empty record is authoritative,
    suppressing the frame's conservative scan and freeing live objects. A regression
    test now compiles the exact shape the specialiser produces (`alloc` typed
    `"any"`) and asserts it is rooted.
  - `any` is not a fallback but the *normal* type of every dynamic value
    (`__dyn_cons` allocates through `__twig_gc_alloc`), so it must be a root.
    Honest cost: in a dyn-heavy function most slots are `any`, so the map approaches
    a conservative scan for that frame. The win is real for statically-typed
    frontends, where integer/float/bool slots — the ones producing heap-address
    look-alikes — are excluded outright.
- **A missing slot or an out-of-range offset is now a hard `BackendError`**, not a
  skipped iteration: silently dropping a root is a use-after-free, so it must never
  be the quiet path. (Neither is reachable today.)
- **Offsets need no translation.** `RegAlloc` hands out SP-relative offsets and the
  prologue pins `fp = sp`, so an SP-relative offset *is* the FP-relative offset the
  record format wants. Slot lookups are **read-only** (`slots.get`, never
  `slot_of`) — minting a slot post-compile would silently grow the frame the code
  was already generated against.
- **Call sites are found by scanning the finished code** for `BL`/`BLR` rather than
  hooking the ~10 scattered emission sites, so a newly added call can never silently
  escape the map. AArch64 is fixed-width and the assembler stores instructions only
  (no inline literal pools), so every 4-byte word is a real instruction.
- The stack map is **pure metadata**: a test asserts `compile` and the stack-map
  entry point emit byte-identical code.
- 9 new tests, including the production-shape `any`-alloc regression above, the
  scalar-vs-root type classification (with an unknown type failing safe as a root),
  BL/BLR detection (patched `imm26`, ragged tail), reference parameters, empty
  records for scalar-only functions, and codegen invariance.

Emitting these into `.rodata` and registering them via `__gc_register_stackmap` at
start-up is the next rung — that is when precise roots actually fire.

## 0.27.0 - 2026-07-11 (E6d-2b: dyn_box_int runtime builtin)

E6d-2b: register `dyn_box_int` in `V1_BUILTINS` (`uint64_t __dyn_box_int(int64_t)`), so dynamic arithmetic that re-boxes a machine result at runtime lowers to `bl __dyn_box_int`. Mirrors the existing `dyn_unbox_int`.

## 0.26.0 - 2026-07-11 (DVAL01-2: dyn_* builtin names + fix native runtime-symbol emit)

DVAL01-2: the V1 builtin table's lisp entries are de-lisped (`lispy_cons`->`dyn_cons`, ... `lispy_to_exit_code`->`dyn_to_exit_code`). **Also fixes a latent bug left by DVAL01-1a**: the `call_builtin` emit hard-coded `__twig_<name>` for *all* helpers, so the tagged-value builtins emitted `__twig_lispy_cons` -- a symbol the runtime (which exports `__dyn_cons`) does not provide. The emit now routes `dyn_*` names to `__<name>` (= `__dyn_cons`) and everything else to `__twig_<name>`, matching `dynval_runtime.c` + the LLVM backend. Fixes the 4 previously-red `call_builtin`->external-symbol unit tests (real programs were unaffected: they lower cons/car via the structural alloc path).

## 0.25.0 - 2026-07-11 (DVAL01-1b: rename C runtime file lispy_runtime.c -> dynval_runtime.c)

DVAL01-1b: the shared C runtime file is renamed `lispy_runtime.c` -> `dynval_runtime.c` (and the golden test `lispy_runtime_golden.rs` -> `dynval_runtime_golden.rs`), continuing the de-lisp of the generic dynamic-value substrate (spec DVAL01). Pure file/path rename -- no symbol, ABI, or behaviour change; the link/build path strings that reference the runtime are updated to match. The `lispy-runtime` Rust crate rename follows in DVAL01-1c.

## 0.24.0 - 2026-07-11 (DVAL01-1a: dynamic-value runtime ABI __twig_lispy_* -> __dyn_*)

De-lisp the tagged dynamic-value runtime ABI: every `__twig_lispy_*` C symbol (box_int/unbox_int/cons/car/cdr/pair_p/equal/not/nil/make_symbol/truthy/to_exit_code/tag_*) is renamed to the language-neutral `__dyn_*` (per spec DVAL01). Pure rename -- the 3-bit tag layout, encodings, and runtime behaviour are byte-for-byte unchanged, so any dynamic frontend (not just lisp) can target the same primitives. The GC ABI (`__twig_gc_*`) is untouched.

## 0.23.0 — 2026-07-10 — E4d-BA-arr: `str` array element (BASIC string arrays)

`native_array_elem_size` now accepts a `str` element as an 8-byte element (BASIC
`DIM A$(n)` → `array<str>`). A `str` value on the native backend is already an
8-byte runtime string handle (the address of a `[i64 len][bytes]` block), stored
and loaded as a plain word exactly like an i64, so no separate str load/store path
is needed — twig-aot already materialises the handle into the var's slot. One-line
element-size allowance mirroring the x86_64 backend.

## 0.22.0 — 2026-07-07 — E4-dyn: `str_concat` in V1_BUILTINS (runtime string concat)

`V1_BUILTINS` gains `str_concat { n_args: 2, returns: true }` — the runtime string
concatenation helper `int64_t __twig_str_concat(int64_t a, int64_t b)`. Same
2-arg / returns-i64 shape as `str_eq` (both operand handles ride x0/x1, the result
handle rides x0), so the generic `call_builtin` marshaller needs **no new codegen**
— only the table entry. Emitted by twig-aot when a `str_concat` operand is a runtime
handle. Run-verified locally via the `PRINT A$ + B$` matrix cell (aarch64).

## 0.21.0 — 2026-07-07 — E4-dyn: `input_str` in V1_BUILTINS (BASIC string INPUT)

Adds `input_str` (BASIC string `INPUT A$`) to `V1_BUILTINS` as a
0-arg/returns-i64 entry — the exact shape of `input_i64`. The helper
`__twig_input_str` returns an i64 handle to a `[i64 len][bytes]` heap block; the
pointer rides `x0` like any `alloc_bytes`/`str_eq` result, so **no codegen
change** — only the table entry. Run-verified via `lang-aot`'s `lang_matrix`
(`10 INPUT A$ / 20 PRINT A$ / 30 END`, stdin `"OK"` → `OK`).

## 0.20.0 — 2026-07-01 — Large-frame split prologue/epilogue (AL-multidim)

**Problem**: ALGOL 60 programs that use a 2D array emit many IIR variables
(lower bounds, strides, size temporaries, loop counters, etc.), producing
stack frames larger than 504 bytes.  The pre-indexed `STP X29,X30,[SP,#-N]!`
instruction uses a 7-bit signed immediate × 8 (range −504 … +504), so frames
above 504 bytes were rejected with `BackendError::FrameTooLarge`.

**Fix — split prologue for large frames** (`emit_function_prologue`):
- Frames ≤ 504 bytes: unchanged — `STP X29,X30,[SP,#-frame]!` (pre-indexed,
  combined allocate + save in one instruction).
- Frames > 504 bytes: emits `SUB SP, SP, #frame` (12-bit unsigned immediate,
  covers up to 4080 bytes = 508 variable slots) followed by two `STR`
  instructions: `STR X29, [SP, #0]` (saves FP) and `STR X30, [SP, #8]`
  (saves LR).  Variable slots start at `[SP, #16]` — identical layout in
  both paths.

**Matching split epilogue** (`emit_epilogue`):
- Frames ≤ 504 bytes: unchanged — `LDP X29,X30,[SP],#frame` (post-indexed).
- Frames > 504 bytes: `LDR X29, [SP, #0]`, `LDR X30, [SP, #8]`, `ADD SP,
  SP, #frame` (12-bit immediate via `add_imm`).

**New frame limit**: 4080 bytes (raised from 504).  `BackendError::FrameTooLarge`
is only returned for frames > 4080.

**61 unit tests pass** (no new tests added; existing tests exercise both paths
because the small-frame path is unchanged and the large-frame path is exercised
by the new AL-multidim matrix cell in `lang-aot`).

## 0.19.0 — 2026-07-01 — TWIG-GC: `alloc` → `__twig_gc_alloc`, `safepoint` lowering

**`alloc` op** (TWIG-GC, native-aot-substrate PR-1):
- Now reads the allocation size from `srcs[0]` (a compile-time `Int`) instead
  of hardcoding 16 bytes.  Falls back to 16 if `srcs[0]` is absent (legacy IIR).
- Calls `__twig_gc_alloc` (TWIG-GC) instead of `__twig_alloc_bytes`.  The
  returned pointer is tracked by the conservative GC and freed when unreachable.

**`safepoint` op**: Previously returned `UnsupportedOp`.  Now lowers to
`BL __twig_gc_safepoint` — a no-arg, no-return call that triggers a GC cycle
when `gc_live_bytes >= gc_threshold`.  Emitted by frontends at loop back-edges.

**V1_BUILTINS additions**: `gc_alloc` (1 arg, returns) and `gc_safepoint`
(0 args, no return) so frontends can emit `call_builtin "gc_alloc"` and
`call_builtin "gc_safepoint"` directly without going through the `alloc` op.

## 0.18.0 — 2026-07-01 — BA-pow `f64_pow` + LANG-STR-RT `str_eq` builtin

**LANG-STR-RT `str_eq`**: Added `BuiltinSig { name: "str_eq", n_args: 2,
returns: true }` to `V1_BUILTINS`.  The callee is `__twig_str_eq(int64_t a,
int64_t b) -> int64_t` in `twig_runtime.c`, which reads the 8-byte length
header from each LANG-STR-RT buffer and `memcmp`s the data regions.  Required
for `str_eq` when one or both operands are function parameters.

**BA-pow `f64_pow` (LANG-FULL)**: Added `f64_pow` block: loads base into D0
via `load_fp_operand`, loads exponent into D1, emits `BL pow` via
`bl_external("pow")` (AAPCS64: D0=base, D1=exp, result in D0), and stores D0
to the dest stack slot.  This is the first two-argument floating-point external
call in the aarch64-backend.
## 0.17.0 — 2026-06-29 — `f64_atan/f64_tan` via libm `BL` (LANG-FULL AL8-arctan)

Extended the transcendental match arm to cover two more ops:
- `f64_atan` → `BL atan`  (libm inverse tangent)
- `f64_tan`  → `BL tan`   (libm tangent)

Pattern: `ldr_d D0,[src]; BL atan/tan (PltRel32 external reloc); str_d D0,[dest]`.
AAPCS64 passes and returns f64 in D0 — no register adjustment needed.

## 0.16.0 — 2026-06-28 — `f64_sin/cos/ln/exp` via libm `BL` (LANG-FULL AL8-trig)

Transcendentals call libm via AArch64's `BL` instruction with an external reloc:
`ldr_d D0,[src]; BL sin/cos/log/exp; str_d D0,[dest]`.
AAPCS64 passes and returns f64 in D0, so no register adjustment is needed.
Mapping: `f64_ln` → `BL log` (libm natural log is `log`, not `ln`).
libm is pre-linked on macOS (`-lSystem`) and Linux (`-lm`).

## 0.15.0 — 2026-06-28 — `f64_sqrt` via `FSQRT` hardware instruction (LANG-FULL AL8-sqrt)

The aarch64 backend now lowers `f64_sqrt dest <- src` to:
`ldr_d D0,[src]; fsqrt D0,D0; str_d D0,[dest]` — one hardware instruction,
no libm call.  Uses the new `aarch64-encoder v0.6.0` `fsqrt` method.

## 0.14.0 — 2026-06-27 — `array<f64>` element support (LANG-FULL BA7)

Native aarch64 arrays now accept `f64` element types in `alloc_array`,
`array_get`, and `array_set`. The layout remains the E5 8-byte
length-prefixed block; f64 elements ride those slots as raw IEEE-754 bits, and
later floating-point operations load them through the existing FP path.

- Keeps the same explicit unsigned bounds checks and `udf #0xDEAD` trap behavior
  from E5.
- Retains fixed 8-byte native array elements and rejects non-8-byte types.
- Verified by `aarch64-backend` unit tests and the BASIC BA7 matrix cell that
  stores fractional `DATA` through `array<f64>` on the native column.

## 0.13.0 — 2026-06-23 — int ⇄ real conversions (LANG-FULL E8 PR-6a)

The three IIR numeric-conversion ops now lower to aarch64 — the sixth backend
(after VM/JIT, LLVM, WASM, JVM, CLR) to gain them and a step toward the E8
matrix proof (PR-7's ALGOL `entier`):

| IIR op | aarch64 sequence |
|--------|------------------|
| `int_to_real` | `ldr x0,[src]; scvtf d0,x0; str d0,[dest]` |
| `real_to_int_trunc` | `ldr d0,[src]; fcvtzs x0,d0; str x0,[dest]` |
| `real_to_int_floor` | `ldr d0,[src]; frintm d0,d0; fcvtzs x0,d0; str x0,[dest]` |

The native integer model is a true 64-bit `i64` (full `Xn` registers), so these
are real i64↔f64 conversions (unlike the CLR/JVM 32-bit scalar model). The ops
arrive with their bare IIR names — the `aot-core::specialise` pass passes
unrecognised ops through unchanged — so the backend matches them directly rather
than via a typed `_<ty>` suffix.

**Trap divergence (documented):** `fcvtzs` rounds toward zero and *saturates* on
NaN/±∞/out-of-range (ARM never traps), a divergence from the VM's fail-closed
trap shared with the JVM backend; every finite, in-range value (all
`entier`/coercion produces) converts identically.

Verified by **executing generated machine code on real Apple Silicon**
(`e8_conversions_execute`): `floor(int_to_real(45) − 2.7) = 42`,
`trunc(42.3) = 42`, and the sign-sensitive `floor(−2.7) = −3` vs `trunc(−2.7) =
−2` (proving `frintm` rounds toward −∞, not toward zero). Requires
aarch64-encoder ≥ 0.5.0 (`scvtf`/`fcvtzs`/`frintm`).

## 0.12.0 — 2026-06-21 — bounds-checked arrays (LANG-FULL E5 PR-4c) — completes E5

The four E5 array opcodes now lower to raw aarch64, using the **static**
length-prefixed model with an **explicit** `udf` bounds trap:

| op | aarch64 |
|----|---------|
| `alloc_array dest <- count` | `x0=count; lsl x0,#3; add x0,#8; bl __twig_alloc_bytes; str count,[x0]; dest=x0` |
| `array_get dest <- handle, idx` | `ldr x2,[base]; cmp idx,x2; b.lo ok; udf #0xDEAD; ok: lsl idx,#3; add base,idx; ldr dest,[base,#8]` |
| `array_set handle, idx, val` | same bounds check; `str val,[base+idx*8, #8]` |
| `array_len dest <- handle` | `ldr dest,[base]` |

- Layout `[i64 length][elem 0][elem 1]…`, handle = block base; length at
  `[base+0]`, elements at `[base + idx*8, #8]`. Allocation reuses the shared
  `__twig_alloc_bytes` runtime helper (the byte-tape's allocator).
- **Bounds check**: one **unsigned** `cmp idx, len` + `b.lo` skips a `udf #0xDEAD`
  trap when in range — `b.lo` (LO = unsigned `<`) catches both `>= len` and a
  negative index. The aarch64 twin of LLVM's `icmp uge`+`llvm.trap`.
- Element width fixed at **8 bytes** (the AOT specialiser collapses `array<T>`→
  `any`; `array_get`/`array_set` validate `i64`/`u64`; 0.14.0 adds `f64`).
  Reuses only pre-existing encoders (`lsl_reg`/`add_imm`/`cmp`/`b_cond`/`ldr`/
  `str_`/`udf`/`bl_external`).
- 2 new unit tests (≥2 `udf` traps; non-`i64` element refused). The ALGOL array
  matrix `Prog` runs on **NativeAot** and was executed **locally on this Apple
  Silicon host → exit 42**. **This completes E5 across all 7 backends.**

## 0.11.0 — 2026-06-20 — `f64` (ALGOL `real`) arithmetic + comparisons (LANG-FULL E3)

### Added — native double-precision codegen on aarch64

The backend rejected `const_f64`/float operands. It now lowers `f64` (enabler
E3 — ALGOL `real`):

- **`const_f64`** materialises the IEEE-754 bit pattern in `X0` and `str`s it —
  a double rides its 8-byte stack slot as raw bits, identical to an integer
  slot, so loading a *constant* needs no FP register.
- **`add_f64`/`sub_f64`/`mul_f64`/`div_f64`** load both operands into `D0`/`D1`
  (`ldr_d`), run `fadd`/`fsub`/`fmul`/`fdiv`, and `str_d` the result. IEEE
  division by zero is `±inf`/`NaN` (no trap) — matching every other backend.
- **`cmp_*_f64`** load `D0`/`D1`, `fcmp`, then `cset X0, <cond>` (the boolean is
  an `int` 0/1). The condition codes give IEEE **ordered** semantics: a NaN
  operand makes `<`/`<=`/`>`/`>=`/`==` false (`!=` true) — `Lt`→`MI`, `Le`→`LS`,
  `Gt`→`GT`, `Ge`→`GE`, `Eq`→`EQ`, `Ne`→`NE`.

**Verified by RUNNING on real Apple-Silicon hardware** (`jit-loader-macos`
installs the generated code and *calls* it): `2.5 * 2.0` → the bits of `5.0`,
`7.0 / 2.0` → `3.5`, and all six comparisons return the right 0/1. Plus a
host-agnostic structural test (compiles on the x86 CI box). Integer programs are
untouched (the FP path keys on `ty == "f64"`). Uses `aarch64-encoder` 0.4.0's FP
instructions. (x86_64 SSE + the lang-aot matrix `NativeAot` proof are the E3-native
follow-up.)

## 0.10.0 — 2026-06-15 — narrow-width unsigned masking (LANG-FULL E2, native-AOT leg)

Native registers are 64-bit, so a `u8` add of `200 + 100` previously computed
`300` — the result was **not** truncated to the declared width, unlike the other
backends (vm-core, jit-core, wasm, jvm, cil) which already wrap. This release
closes the native-AOT leg of enabler **E2**: every narrow **unsigned** op
(`u4`/`u8`/`u16`/`u32`) now masks its result with a follow-up
`mov X2, #mask; and X0, X0, X2`, so:

- `add_u8 200, 100` → `44` (300 mod 256)
- `sub_u8 0, 1` → `255`, `mul_u8 16, 16` → `0`
- `not_u8 0` → `255`, `shl_u8 1, 8` → `0`
- `u16`/`u32` wrap at their widths (a 64-bit register does **not** wrap a `u32`
  add for free, so the mask is what makes `u32` correct here — unlike wasm)

Masking covers `add`/`sub`/`mul`/`div`/`mod`/`and`/`or`/`xor`/`shl`/`shr`/`neg`/`not`;
for the ops whose result is already in range (bitwise of masked operands, div/mod,
right shifts) the mask is a provably-redundant no-op kept for uniformity. Full-width
(`u64`/`i64`) and signed narrow types are unchanged (signed narrow would need
sign-extension, not a plain mask, and no frontend emits it). See `mask_narrow_x0`.

New tests: structural (the mask instructions are emitted; `i64` is never masked)
plus an **executed** proof — the generated ARM64 is installed via `jit-loader-macos`
and called, asserting the wrapped values directly (Apple-Silicon macOS). Unblocks the
Nib **N6** / Oct **O2** wrap-semantics frontend items.

## 0.9.0 — 2026-06-10 — McCarthy lambda (F7): `lispy_to_exit_code` builtin (LANG77 / W14b)

Adds `lispy_to_exit_code` to `V1_BUILTINS` (→ `BL __twig_lispy_to_exit_code`), the
universal program-exit coercion for a polymorphic lambda result (W13b). This was the
*only* gap for native `LAMBDA` on the tagged-word backend — cross-function `call`
(Twig `fib`), the `any`/`ref<Lispy…>` value model (cons), and the shared arg-boxing +
result-coercion passes were already in place. Native lambda now runs (verified
end-to-end on macOS arm64). New unit test `lispy_to_exit_code_lowers`.

## 0.8.0 — 2026-06-04 — ATOM/EQ predicate + truthy helpers (LANG77 / McCarthy L3b-2c-2)

Adds four `V1_BUILTINS` rows — `lispy_pair_p` (1), `lispy_not` (1),
`lispy_equal` (2), `lispy_truthy` (1), all returning a value → `BL
__twig_lispy_*`. These back `ATOM` (`not(pair?)`), `EQ` (`equal?`) and the
`COND` truthiness normaliser the `lower_lisp_repr` pass inserts before
`jmp_if_false`. No new opcodes — the generic `call_builtin` dispatch handles
them. New host-independent test: the ATOM/EQ predicate + truthy sequence
lowers and emits the four external relocs.

## 0.7.0 — 2026-06-04 — lisp int unbox helper (LANG77 / McCarthy L3b-2c-1)

Adds one `V1_BUILTINS` row — `lispy_unbox_int` (1 arg, returns) → `BL
__twig_lispy_unbox_int` — the helper the new `lower_lisp_repr` pass inserts
at the program-exit boundary to turn a tagged integer back into a raw
machine word for the process exit code. No new opcodes; the generic
`call_builtin` dispatch handles it.

New host-independent test: the full boxed `(CAR (CONS 7 9))` sequence (boxed
atoms → `lispy_cons` → `lispy_car` → `lispy_unbox_int` → ret) lowers and
emits external relocs to all three runtime symbols.

## 0.6.0 — 2026-06-04 — lisp runtime calls (LANG77 / McCarthy L3b-2b)

Adds three rows to the `V1_BUILTINS` helper table — `lispy_cons` (2 args),
`lispy_car` (1), `lispy_cdr` (1), all returning a value — so `call_builtin
"lispy_cons"` etc. dispatch to `BL __twig_lispy_cons` in the linked C lisp
runtime (`twig-aot/runtime/lispy_runtime.c`). These are the runtime-call
form of cons/car/cdr (produced by
`iir_builtin_lowering::lower_heap_builtins_runtime`), keeping lisp values
NaN-box tagged rather than raw words.

**No new opcodes or emitter logic** — the existing generic `call_builtin`
dispatch marshals the args into x0/x1 per AAPCS64 and emits the BL with an
external relocation; the table rows are the entire change. The L3b-1
`alloc`/`field_*` emitters remain as general-purpose heap ops (no longer on
the McCarthy cons path).

Two new host-independent tests: `(CAR (CONS 7 9))` via the runtime path
emits external relocations to `__twig_lispy_cons`/`__twig_lispy_car`, and a
wrong-arity `lispy_cons` call is softly refused.

## 0.5.0 — 2026-06-04 — heap cons cells (McCarthy Lisp L3b)

Lower the four word-granular heap ops that
`iir_builtin_lowering::lower_heap_builtins` produces from a Lisp frontend's
`cons`/`car`/`cdr`/`null?`, so a cons-of-integers program compiles to native:

* **`alloc -> dest`** — a fresh 2-word (16-byte) `LispyPair` cell, via the
  same `__twig_alloc_bytes` runtime helper `alloc_bytes` uses (V1 leaks; no
  GC).
* **`field_store ptr, idx, val`** / **`field_load ptr, idx -> dest`** —
  word load/store at byte offset `idx*8` (field 0 = car, field 1 = cdr).
  The index is a compile-time `Int` immediate; a non-literal index or a
  `field_store` with a dest is a `MalformedInstr`.
* **`is_null x -> dest`** — `dest = (x == 0)` (nil is the 0 word), via
  `cmp` + `cset eq`.
* Values are **raw 64-bit words** — no NaN-boxing — so `(CAR (CONS 7 9))`
  round-trips to a raw `7`.  3 new unit tests (cons/car lowers; is_null
  lowers; field_store-with-dest and non-literal-index rejected).

## 0.4.0 — 2026-05-20 (LANG76 — byte memory ops + heap allocation)

Three new CIR opcodes mirroring the LANG76 work in `x86_64-backend`
0.6.0:

- `alloc_bytes <n> -> <dest>` — sugar for `call_builtin "alloc_bytes",
  n`.  Loads n into X0, emits `BL __twig_alloc_bytes`, stores X0 into
  the dest slot.
- `load_byte <ptr>, <offset> -> <dest>` — `ldr x0,[sp,ptr]; ldr
  x1,[sp,off]; add x0,x0,x1; ldrb w0,[x0]; str x0,[sp,dest]`.  The
  LDRB instruction zero-extends to 32 bits and AArch64 zeroes the
  upper 32 bits of X0 automatically — exactly the 64-bit
  zero-extension semantics LANG76 specifies.
- `store_byte <ptr>, <offset>, <value>` — `ldr x0,[sp,ptr]; ldr
  x1,[sp,off]; add x0,x0,x1; ldr x2,[sp,val]; strb w2,[x0]`.

**Tests added (5):** alloc_bytes records `__twig_alloc_bytes` BL
placeholder; load_byte/store_byte emit the expected ldrb/strb words
(`0x39400000` / `0x39000002`); load_byte missing operand refusal;
store_byte with dest refusal.

## 0.3.0 — 2026-05-20 (LANG75 — generic `call_builtin` dispatch)

Adds a single CIR opcode `call_builtin "<name>", <args>` that
dispatches to runtime helpers via the V1 helper table.  Mirrors the
LANG75 work in `x86_64-backend` 0.5.0 — both backends now share the
same six-entry helper table, so a frontend that emits `call_builtin
"putchar", c` produces a working `BL __twig_putchar` on aarch64 and a
working `CALL __twig_putchar` on x86_64 with no per-target divergence.

**New CIR opcode:**

- `call_builtin "<name>", <arg0>, <arg1>, …` — looks `name` up in the
  V1 helper table, loads each arg into `x0..x7` per AAPCS64, emits
  `BL __twig_<name>` (placeholder; AOT linker patches), and (if the
  helper returns) stores `x0` into the dest slot.

**V1 helper table (same as `x86_64-backend`):**

| Name           | Args     | Returns |
|----------------|----------|---------|
| `print_i64`    | `[i64]`  | no      |
| `putchar`      | `[i32]`  | no      |
| `getchar`      | `[]`     | yes     |
| `print_string` | `[ptr,i64]` | no   |
| `input_i64`    | `[]`     | yes     |
| `exit`         | `[i32]`  | no      |

**Error handling.**  Unknown helper names, wrong arity, and dest/void
mismatches all return `BackendError::MalformedInstr` (the spec's
"BackendRefused" — a soft refusal, not a panic).

**Behaviour preserved.**  `io_out` is unchanged and still emits the
same single `BL __twig_print_i64`; `call_builtin "print_i64", v`
produces the same bytes via the new generic path.

**Tests added (6):** putchar emits BL reloc, getchar stores x0 to
dest, print_string records two arg loads, unknown name refuses, wrong
arity refuses, print_i64-via-builtin matches io_out.

## 0.2.3 — 2026-05-13 (LANG41)

**Remove self-contained `emit_print_helper`; resolve `__twig_print_i64` from
portable C runtime archive instead.**

LANG40 injected a 208-byte ARM64 function that used macOS raw `write(2)`
syscall numbers (`x16=4`, `SVC #0x80`) baked in as ARM64 instruction words.
LANG41 removes this macOS-specific helper entirely.

### Removed

- `emit_print_helper() → Vec<u8>` — public API function deleted.
  Callers previously used this to inject a self-contained integer-print
  subroutine alongside user code; the symbol `__twig_print_i64` is now
  left unresolved in the object file for the system linker (`ld`) to
  resolve from `libtwig_aot_runtime.a` (built by `twig-aot`'s `build.rs`).

### Retained

- `io_out` CIR handler still emits `LDR X0, [X19, #offset]` + `BL __twig_print_i64`.
  The BL produces a `Reloc { symbol: "__twig_print_i64", … }` placeholder
  exactly as before — the only change is that twig-aot no longer injects the
  helper into the link; instead it writes the runtime archive to a temp file
  and passes it to `ld`.

### Tests removed

- `emit_print_helper_has_prologue`
- `emit_print_helper_ends_with_ret`
- `emit_print_helper_size_is_52_words`

Remaining tests `io_out_emits_bl_reloc` and `io_out_missing_src_errors` are
unchanged and still pass.

---

## 0.2.2 — 2026-05-13 (LANG40)

**`io_out` CIR handler + self-contained `__twig_print_i64` helper.**

### New CIR opcode handled

| CIR opcode | ARM64 sequence | Notes |
|------------|----------------|-------|
| `io_out Var(val)` | `LDR X0 + BL __twig_print_i64` | Loads value into X0; helper injected by twig-aot |

### New public API

- `emit_print_helper() → Vec<u8>` — emits a self-contained 52-instruction
  (208-byte) ARM64 function that converts a signed 64-bit integer (in `x0`)
  to decimal ASCII and writes it to stdout followed by `'\n'`, using the
  macOS `write(2)` syscall (`x16 = 4`, `SVC #0x80`).

### Implementation notes

- **No external symbols** — the helper lives in `__TEXT/__text` alongside user
  functions and is resolved by the existing cross-function BL linker in
  `twig-aot::compile_module_to_text_raw`, avoiding the need for `_printf`
  stubs or dyld machinery.
- **Algorithm**: UDIV+MSUB digit-extraction loop writing bytes backwards into
  a 32-byte stack buffer; `STRB Wt,[Xn,#-1]!` (from `aarch64-encoder` 0.2.2)
  decrements the write pointer and stores each ASCII digit in one instruction.
  Special-cases `x0 == 0`.  Prepends `'-'` for negatives.
- **Frame**: 48 bytes (16-byte aligned).  `'\n'` written to `[sp+48]` which
  lies in macOS's 128-byte red zone (safe for SVC helper functions).
- **Verified encodings**: all 52 instruction words verified against ARM ARM
  (DDI 0487).  `emit_print_helper_size_is_52_words` enforces the count.

### Tests (5 new)

| Test | Asserts |
|------|---------|
| `io_out_emits_bl_reloc` | exactly one `ExternalReloc { symbol: "__twig_print_i64" }` |
| `io_out_missing_src_errors` | error on zero srcs |
| `emit_print_helper_has_prologue` | first word = `0xA9BD7BFD` (STP x29,x30,[sp,#-48]!) |
| `emit_print_helper_ends_with_ret` | last word = `0xD65F03C0` (RET) |
| `emit_print_helper_size_is_52_words` | exactly 208 bytes |

## 0.2.1 — 2026-05-13 (LANG39)

**Global variable load / store lowering.**

Wires the `global_load` and `global_store` CIR opcodes into the dispatch table.

### New CIR opcodes handled

| CIR opcode | ARM64 sequence | Notes |
|------------|----------------|-------|
| `global_load Var(name)` | `ADRP X1 + ADD X1 + LDR X0 + STR X0` | 4 instructions; reads from `_twig_globals[slot*8]` |
| `global_store Var(name), val` | `LDR X0 + ADRP X1 + ADD X1 + STR X0` | 4 instructions; writes to `_twig_globals[slot*8]` |

### New public API

- `compile_with_globals(ctx, ir, global_slots) → (bytes, ExternalRelocs, GlobalWordRelocs)` —
  like `compile_with_relocs` but also accepts a `HashMap<String, usize>` mapping global names
  to slot indices and returns `Vec<GlobalWordReloc>` for Mach-O ARM64 relocation emission.

- `GlobalWordReloc { adrp_word: usize, add_word: usize }` — word-index pair for one
  `ARM64_RELOC_PAGE21` + `ARM64_RELOC_PAGEOFF12` relocation site.

### Implementation notes

- The ADRP and ADD are placeholder instructions (`ADRP Xd, #0` / `ADD X1, X1, #0`);
  the system linker patches the immediates when producing the final executable.
- The LDR/STR slot offset (`slot * 8`) is baked in at compile time.
- 5 new unit tests cover the opcode handlers, slot offset encoding, error handling,
  and multi-global reloc counting.

## 0.2.0 — 2026-05-13 (LANG38)

**Division, modulo, bitwise logic, shifts, negate, and bitwise-NOT lowering.**

Wires the 11 new `aarch64-encoder` instructions (0.2.0) into the CIR opcode
dispatch table.  These are the ops that blocked any Twig program using
integer division (e.g. number parsers) or bitwise manipulation.

### New CIR opcodes handled

| CIR mnemonic family | Lowering | Notes |
|---------------------|----------|-------|
| `div_<ty>` | `SDIV` (signed) / `UDIV` (unsigned) | 1 instruction |
| `mod_<ty>` | `SDIV`/`UDIV` then `MSUB` | 2 instructions; uses X2 as scratch |
| `and_<ty>` | `AND` | — |
| `or_<ty>` | `ORR` | — |
| `xor_<ty>` | `EOR` | — |
| `shl_<ty>` | `LSLV` | shift amount mod 64 (ARM architectural) |
| `shr_<ty>` | `ASRV` for `i*`; `LSRV` for `u*` | signed/unsigned based on type suffix |
| `neg_<ty>` | `NEG` | two's-complement negate |
| `not_<ty>` | `MVN` | bitwise NOT |

### Implementation notes

- Signed vs unsigned is determined by `ty.starts_with('i')`, matching the
  same convention used by comparisons.
- `mod_<ty>` uses X2 as an additional scratch register for the intermediate
  quotient.  The stack-spill allocator keeps every live value in a fixed
  stack slot, so X2 is free between instructions — no aliasing hazard.
- New helpers: `emit_div`, `emit_bitwise` (+ `BitwiseKind`), `emit_shift`
  (+ `ShiftKind`).

14 new backend tests exercise each opcode family.

## 0.1.2 — 2026-05-13

### Added

- **Cross-function `BL` relocations** — new `compile_with_relocs` public
  entry point returns `(Vec<u8>, Vec<Reloc>)`.  Each `Reloc` records the
  word index of a placeholder `BL #0` instruction that targets a function
  outside the current binary.  The two-pass AOT linker in `twig-aot` uses
  these to patch the final linked image with correct PC-relative offsets.
- `Reloc` is a re-export of `aarch64_encoder::ExternalReloc`.

### Changed

- Cross-function `call` instructions now emit a `BL #0` placeholder via
  `Assembler::bl_external` instead of returning `Err(UnsupportedOp)`.
  Self-recursive calls continue to emit a direct `BL` to the body-entry
  label.

## 0.1.0 — 2026-05-05

Initial release.  ARM64 native-code backend for jit-core / aot-core,
implementing the shared `Backend` trait via `Backend::compile_function`.

### Implemented CIR coverage

- Constants: `const_u8` … `const_u64`, `const_i8` … `const_i64`, `const_bool`
- Integer arithmetic (typed): `add_<ty>`, `sub_<ty>`, `mul_<ty>`
- Comparisons: `cmp_eq_<ty>` … `cmp_ge_<ty>` (signed and unsigned)
- Control flow: `label`, `jmp`, `jmp_if_true`, `jmp_if_false`
- Returns: `ret_<ty>`, `ret_void`
- Type guards: `type_assert` lowered to `udf` trap

### Register allocation

Stack-spill: every CIR virtual register lives at a fixed 8-byte stack slot.
Each instruction loads sources into scratch `x0..x2`, performs the op, and
stores the destination back.  Trivially correct; suboptimal performance.
A real allocator can replace it without changing the public API.

### AAPCS64 prologue / epilogue

```
stp  fp, lr, [sp, #-frame]!
mov  fp, sp
str  x0..x7, [sp, #(slot)]    ; spill incoming args
<body>
ldp  fp, lr, [sp], #frame
ret
```

Up to 8 parameters are supported.  Frame must fit a 12-bit unsigned offset
(≈ 4088 bytes / ~512 virtual registers).

### Out of scope (deferred)

- Float operations
- `call_runtime`, `send`, `load_property`, `store_property`
- Width-truncation for u8/u16/u32 results
- Real register allocation

## 0.1.1 — 2026-05-05

### Added
- `mov_<ty>` lowering — typed register-to-register move (load + store
  via the stack-spill regalloc).  Used by aot-core when lowering
  `call_builtin "_move"`.

### Fixed
- **Stack frame layout bug**: virtual register slot 0 was at `[sp + 0]`,
  but the prologue's `stp fp, lr, [sp, #-frame]!` saves `fp` at the
  same offset.  The first `str x0, [sp]` clobbered the saved `fp`,
  so the function's `ldp fp, lr, [sp], #frame` epilogue restored a
  garbage `fp` and `ret` returned to a garbage address — instant
  SIGSEGV.

  Fix: virtual slot offsets now start at +16 to leave room for the
  saved `fp/lr`.  The frame-size cap drops from 4080 to 504 bytes —
  reflecting the actual `stp_pre`/`ldp_post` 7-bit signed immediate
  range (the prior 4080 was wishful thinking).

### Note

The fix is what made real Twig programs (`(+ 30 12)`, `(if ...)`)
actually run end-to-end on Apple Silicon.  Pre-fix, the encoder + IR
+ Mach-O writer were all correct, but the program SIGSEGV'd on return
because of the saved-fp clobber.
