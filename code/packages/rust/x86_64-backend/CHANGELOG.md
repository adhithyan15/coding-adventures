# Changelog — `x86_64-backend`

## 0.33.0 - 2026-07-24 — `gc_collect_compacting` builtin (frontend GC.compact, AOT00-T3 §5)

- New `V1_BUILTINS` entry `gc_collect_compacting` (0 args, returns the freed count) — the
  moving/compacting collect, mirroring the aarch64 backend. The generic `__twig_<name>`
  `call_builtin` dispatch auto-emits a `call __twig_gc_collect_compacting` (no per-name
  lowering). A native program can now trigger a compaction. New emission unit test asserts
  the external reloc symbol.

## 0.32.0 - 2026-07-23 — GC safepoints at self-recursive calls (AOT00-T1 x86_64 PR-x4)

Closes the recursive-frame precision gap in `compile_function_with_globals_and_stackmap`.

A cross-function/builtin/libm call is lowered as `call rel32` with a `PltRel32`
relocation, so `build_stack_map` recovers its return address from the reloc
(`patch_offset + 4`). A **self-recursive** `call <fn_name>`, however, is lowered with an
internal label fixup (`call_label`) and carries *no* relocation — so it was invisible to
`build_stack_map`, and a collection fired inside a recursive frame fell back to a
conservative scan (safe, but it could pin integer look-alikes that happen to look like
pointers). This mattered in practice: recursive `dynval`/lisp functions that allocate
(`cons`) are exactly the shape that recurses with live references held across the call.

Now `compile_one_with_globals` records each self-recursive call's return address
(`asm.len()` immediately after the 5-byte `call rel32`) and passes them to
`build_stack_map`, which adds a safepoint at each. `StackMapBuilder::safepoint` dedups and
keeps PCs ascending, so the two safepoint sources compose without ordering hazards. The
recursive frame's live references are now mapped precisely, exactly like every other call
site. Machine code is unchanged — this only augments the derived stack map.

Unlike aarch64 (fixed-width; it post-scans finished code for every `BL`, so it never had
this gap), x86-64 is variable-width and cannot post-scan, so the recursive return address
must be captured at emit time. No new lowering, no ABI change. Tests: two new unit tests —
a purely-recursive function (zero relocs) now yields exactly one safepoint naming its live
`any` slot, and a mixed recursive+builtin function yields two ascending safepoints.

## 0.31.0 - 2026-07-23 — V1_BUILTINS: GC collection + observability (AOT00-T1 x86_64 PR-x3)

Four `call_builtin` entries the native GC-stress differential drives (→ `__twig_gc_*`
aliases in gc-core-capi), mirroring the aarch64 backend: `gc_collect` (forced
conservative full collect, void), `gc_collect_precise` (precise-roots frame walk,
returns freed count), `gc_live_bytes` (live payload bytes), `gc_stackmap_count`
(registered-function count). No new lowering — the generic `call_builtin` marshaller
emits the `CALL`.

## 0.30.0 - 2026-07-22 — GC precise-roots stack-map emission (AOT00-T1)

`compile_function_with_globals_and_stackmap` — the x86-64 analogue of the aarch64
backend's `compile_with_globals_and_stackmap` — returns a `gc_core::StackMapRecord` per
call-return safepoint, naming the reference-typed frame slots live there so
`__gc_collect_precise` can resolve a return address to its exact roots. First step of
porting precise roots to the native x86-64 path (Linux/Windows).

- **Reference slots** are chosen by the same deny-list as aarch64 (`is_gc_root_ty`:
  everything except the machine scalars `u4…void` — notably `any` counts). Their
  `StackMapRecord::slots` values are the RBP-relative *negative* offsets
  `[rbp − 8 − 8·slot]` the walker reads back as `rbp + offset`.
- **Safepoints** are read from the emitted `call rel32` relocations: each `PltRel32`
  reloc's `patch_offset + 4` is the return address after the call. x86-64 is
  variable-width, so unlike the fixed-width aarch64 backend (which post-scans finished
  code) this captures return addresses at their true positions. Every cross-function,
  builtin and libm call — the collection-triggering ones — is covered; a self-recursive
  `call_label` (no reloc) is not yet a safepoint, which costs precision on recursive
  frames (conservative fallback) but is never unsafe.
- The emitted machine code is **byte-for-byte identical** to
  `compile_function_with_globals` — the map is derived, not injected. New gc-core
  dependency; 4 unit tests; existing entry points unchanged.

## 0.29.0 - 2026-07-20 — V1_BUILTINS: dyn_null_p (native `null?`)

Part of the fix restoring McCarthy-lisp list programs on the native-AOT / LLVM backends (`lang-aot` `lang_matrix`). See the umbrella commit for the full story: `null?` was never routed to a runtime call on the tagged native/LLVM path (breaking every cons-walk helper), `list-ref`/`assoc` unboxed a raw-int index/key (→ wrong element), a top-level `(null? …)` predicate result was unboxed instead of truthy-coerced, and cons-cell field access failed the JVM verifier. Verified end-to-end: native list-ref/assoc/length/reverse/append/null? all correct.
## 0.28.0 - 2026-07-11 (E6d-2b: dyn_box_int runtime builtin)

E6d-2b: register `dyn_box_int` in `V1_BUILTINS` (`call __dyn_box_int`), mirroring aarch64 + the existing `dyn_unbox_int`.

## 0.27.0 - 2026-07-11 (DVAL01-2: dyn_* builtin names + fix native runtime-symbol emit)

DVAL01-2: mirrors the aarch64 change on x86_64. De-lisps the V1 builtin lisp entries (`lispy_*`->`dyn_*`) and fixes the same latent DVAL01-1a emit bug: `call_builtin` now emits `__<name>` for `dyn_*` helpers (= `__dyn_cons`, matching the runtime) and `__twig_<name>` otherwise, instead of unconditionally `__twig_<name>`. Fixes the 4 previously-red external-symbol unit tests.

## 0.26.0 - 2026-07-11 (DVAL01-1b: rename C runtime file lispy_runtime.c -> dynval_runtime.c)

DVAL01-1b: the shared C runtime file is renamed `lispy_runtime.c` -> `dynval_runtime.c` (and the golden test `lispy_runtime_golden.rs` -> `dynval_runtime_golden.rs`), continuing the de-lisp of the generic dynamic-value substrate (spec DVAL01). Pure file/path rename -- no symbol, ABI, or behaviour change; the link/build path strings that reference the runtime are updated to match. The `lispy-runtime` Rust crate rename follows in DVAL01-1c.

## 0.25.0 - 2026-07-11 (DVAL01-1a: dynamic-value runtime ABI __twig_lispy_* -> __dyn_*)

De-lisp the tagged dynamic-value runtime ABI: every `__twig_lispy_*` C symbol (box_int/unbox_int/cons/car/cdr/pair_p/equal/not/nil/make_symbol/truthy/to_exit_code/tag_*) is renamed to the language-neutral `__dyn_*` (per spec DVAL01). Pure rename -- the 3-bit tag layout, encodings, and runtime behaviour are byte-for-byte unchanged, so any dynamic frontend (not just lisp) can target the same primitives. The GC ABI (`__twig_gc_*`) is untouched.

## 0.24.0 — 2026-07-10 — E4d-BA-arr: `str` array element (BASIC string arrays)

`native_array_elem_size` now accepts a `str` element as an 8-byte element (BASIC
`DIM A$(n)` → `array<str>`). A `str` value on the native backend is already an
8-byte runtime string handle (the address of a `[i64 len][bytes]` block), stored
and loaded as a plain word exactly like an i64, so no separate str load/store path
is needed — twig-aot already materialises the handle into the var's slot. One-line
element-size allowance mirroring the aarch64 backend.

## 0.23.0 — 2026-07-07 — E4-dyn: `str_concat` in V1_BUILTINS (runtime string concat)

`V1_BUILTINS` gains `str_concat { n_args: 2, returns: true }` — the runtime string
concatenation helper `int64_t __twig_str_concat(int64_t a, int64_t b)`. Same
2-arg / returns-i64 shape as `str_eq` (operand handles ride RDI/RSI, the result
handle rides RAX), so the generic `call_builtin` marshaller needs **no new codegen**
— only the table entry. Emitted by twig-aot when a `str_concat` operand is a runtime
handle. Exercised by the `PRINT A$ + B$` matrix cell (x86_64 on CI).

## 0.22.0 — 2026-07-07 — E4-dyn: `input_str` in V1_BUILTINS (BASIC string INPUT)

Adds `input_str` (BASIC string `INPUT A$`) to `V1_BUILTINS` as a
0-arg/returns-i64 entry — the exact shape of `input_i64`. The helper
`__twig_input_str` returns an i64 handle to a `[i64 len][bytes]` heap block; the
pointer rides `RAX` like any `alloc_bytes`/`str_eq` result, so **no codegen
change** — only the table entry. Proven on the LLVM column (shared
`@__twig_input_str`) and on native via the aarch64 sibling in `lang_matrix`.

## 0.21.0 — 2026-07-01 — TWIG-GC: `gc_alloc` + `gc_safepoint` in V1_BUILTINS

**V1_BUILTINS additions** (TWIG-GC, native-aot-substrate PR-1): Added
`gc_alloc` (1 arg, returns) and `gc_safepoint` (0 args, no return) so
frontends that emit `call_builtin "gc_alloc"` / `"gc_safepoint"` are dispatched
to `__twig_gc_alloc` / `__twig_gc_safepoint` in `twig_gc.c`.  Mirrors the same
additions made to `aarch64-backend v0.19.0`.

## 0.20.0 — 2026-07-01 — BA-pow `f64_pow` + LANG-STR-RT `str_eq` builtin

**LANG-STR-RT `str_eq`**: Added `BuiltinSig { name: "str_eq", n_args: 2,
returns: true }` to `V1_BUILTINS`.  Matches the aarch64-backend addition —
the callee is `__twig_str_eq` in `twig_runtime.c`.

**BA-pow `f64_pow` (LANG-FULL)**: Loads base into xmm0 via
`load_fp_operand(Rax)`, loads exponent into xmm1 via `load_fp_operand(Rcx)`,
emits `call_rel32("pow", PltRel32)` (System V: xmm0=base, xmm1=exp, result in
xmm0), and stores xmm0 to the dest stack slot.
## 0.19.0 — 2026-06-29 — `f64_atan/f64_tan` via libm `call rel32` (LANG-FULL AL8-arctan)

Extended the transcendental match arm to cover two more ops:
- `f64_atan` → `call atan`  (libm inverse tangent, `PltRel32` external reloc)
- `f64_tan`  → `call tan`   (libm tangent, `PltRel32` external reloc)

Pattern: `movsd xmm0,[src]; call rel32 atan/tan; movsd [dest],xmm0`.
System V AMD64 and MS x64 both pass/return the first f64 in xmm0.

## 0.18.0 — 2026-06-28 — `f64_sin/cos/ln/exp` via libm `call rel32` (LANG-FULL AL8-trig)

Transcendentals call libm via `call_rel32` with `PltRel32` external relocs:
`movsd xmm0,[src]; call sin/cos/log/exp; movsd [dest],xmm0`.
Both System V AMD64 and MS x64 pass/return the first f64 in xmm0.
Mapping: `f64_ln` → `call log` (libm natural log is `log`, not `ln`).
libm is pre-linked on Linux (`-lm`) and macOS (`-lSystem`).

## 0.17.0 — 2026-06-28 — `f64_sqrt` via `SQRTSD` hardware instruction (LANG-FULL AL8-sqrt)

The x86_64 backend now lowers `f64_sqrt dest <- src` to:
`movsd xmm0,[src]; sqrtsd xmm0,xmm0; movsd [dest],xmm0` — one SSE2 hardware
instruction, no libm call.  Uses the new `x86_64-encoder v0.6.0` `sqrtsd` method.

## 0.16.0 — 2026-06-27 — `array<f64>` element support (LANG-FULL BA7)

Native x86_64 arrays now accept `f64` element types in `alloc_array`,
`array_get`, and `array_set`. The layout remains the E5 8-byte
length-prefixed block; f64 elements ride those slots as raw IEEE-754 bits, and
later floating-point operations load them through the existing SSE path.

- Keeps the same explicit unsigned bounds checks and `ud2` trap behavior from
  E5.
- Retains fixed 8-byte native array elements and rejects non-8-byte types.
- Verified by `x86_64-backend` unit tests and the BASIC BA7 matrix cell that
  stores fractional `DATA` through `array<f64>` on the native column.

## 0.15.0 — 2026-06-23 — int ⇄ real conversions (LANG-FULL E8 PR-6b)

Dispatch the three IIR numeric-conversion ops to x86_64 SSE — completing E8's
**seventh and final backend** (after VM/JIT, LLVM, WASM, JVM, CLR, aarch64):

| IIR op | x86_64 sequence |
|--------|-----------------|
| `int_to_real` | `mov rax,[src]; cvtsi2sd xmm0,rax; movsd [dest],xmm0` |
| `real_to_int_trunc` | `movsd xmm0,[src]; cvttsd2si rax,xmm0; mov [dest],rax` |
| `real_to_int_floor` | `movsd xmm0,[src]; roundsd xmm0,xmm0,1; cvttsd2si rax,xmm0; mov [dest],rax` |

True 64-bit i64↔f64 (full registers), like aarch64. The ops arrive with their
bare IIR names (the `specialise` pass passes unrecognised ops through unchanged),
so the backend matches them directly. `roundsd …,1` rounds toward −∞ (floor);
`cvttsd2si` truncates toward zero and yields the integer-indefinite `0x8000…0`
on NaN/±∞/out-of-range (no trap) — documented divergence, shared with
JVM/aarch64.

RUN-verified end-to-end through real x86_64 codegen executed in the
**x86-simulator** (`tests/sse_floats.rs`): `floor(int_to_real(45) − 2.7) ⇒ 42`
and `trunc(42.3) ⇒ 42`, matching the LLVM/WASM/VM/JVM/CLR/aarch64 matrix-cell
value. Requires x86_64-encoder ≥ 0.5.0 and x86-simulator ≥ 0.7.6.

## 0.14.0 — 2026-06-21 — bounds-checked arrays (LANG-FULL E5 PR-4c) — completes E5

The four E5 array opcodes now lower to raw x86_64, using the **static**
length-prefixed model with an **explicit** `ud2` bounds trap (the native target
has no managed runtime to bounds-check for it, unlike JVM/CLR):

| op | x86_64 |
|----|--------|
| `alloc_array dest <- count` | `mov rdi,count; shl rdi,3; add rdi,8; call __twig_alloc_bytes; mov [rax],count; dest=rax` |
| `array_get dest <- handle, idx` | `mov rdx,[base]; cmp idx,rdx; jb ok; ud2; ok: shl idx,3; add base,idx; mov dest,[base+8]` |
| `array_set handle, idx, val` | same bounds check; `mov [base+idx*8+8], val` |
| `array_len dest <- handle` | `mov dest,[base]` |

- Layout `[i64 length][elem 0][elem 1]…`, handle = block base; the length header
  is at `[base+0]`, elements at `[base + 8 + idx*8]`. Allocation reuses the same
  `__twig_alloc_bytes` runtime helper the Brainfuck byte-tape calls.
- **Bounds check**: one **unsigned** `cmp idx, len` + `jb` skips a `ud2` trap when
  in range — `jb` (below = unsigned `<`) catches both `>= len` and a negative
  index. The x86_64 twin of LLVM's `icmp uge`+`llvm.trap` / WASM's `i64.ge_u`+
  `unreachable`.
- Element width is a fixed **8 bytes** (the AOT specialiser drops the `array<T>`
  result type to `any`, so the stride isn't on `instr.ty`; `array_get`/`array_set`
  validate the element is `i64`/`u64`; 0.16.0 adds `f64`). Only
  **pre-existing, byte-verified encoders** are
  reused (`shl_imm8`/`add_imm32`/`cmp`/`jcc`/`mov_*`/`ud2`/`call_rel32`) — no new
  encodings.
- 2 new unit tests (≥2 `ud2` traps emitted; non-`i64` element refused). The ALGOL
  array matrix `Prog` runs on **NativeAot** — aarch64 locally, **x86_64 on the
  Linux CI runner** → exit 42. **This completes E5 across all 7 backends.**

## 0.13.0 — 2026-06-20 — `f64` (ALGOL `real`) SSE2 codegen (LANG-FULL E3) — completes E3

### Added — native double-precision codegen on x86_64

Mirrors the aarch64-backend f64 lowering with SSE2:

- **`const_f64`** materialises the IEEE-754 bits in a GPR and stores them — a
  double rides its 8-byte stack slot as raw bits (no XMM reg to load a constant).
- **`add`/`sub`/`mul`/`div_f64`** → `movsd xmm0/xmm1, [slot]`; `addsd`/`subsd`/
  `mulsd`/`divsd`; `movsd [slot], xmm0`. IEEE division by zero is `±inf`/`NaN`.
- **`cmp_*_f64`** → `ucomisd` + `setcc`, with operand-order + condition chosen for
  IEEE-**ordered** semantics: `<`/`<=` compare reversed (`ucomisd b,a` + `seta`/
  `setae`), `>`/`>=` direct (`seta`/`setae`), `==` = `sete` AND `setnp` (ZF=1 &&
  PF=0), `!=` = `setne` OR `setp` (ZF=0 || PF=1) — a NaN operand makes ordered
  compares false (`!=` true), matching every other backend.

**This completes E3-native — ALGOL reals now run on all 7 backends.** x86_64 is
not locally runnable (no x86 ISA simulator), so this is verified by **structural
exact-opcode tests** + **byte-for-byte encoding checks against the system
assembler**, and **executed on the lang-aot matrix `NativeAot` column on the
Linux-x86 CI runner** (the aarch64 half runs the same matrix cell on Apple
Silicon). Integer programs are untouched (FP path keys on `ty == "f64"`). Uses
`x86_64-encoder` 0.4.0's SSE2 instructions.

## 0.12.0 — 2026-06-15 — narrow-width unsigned masking (LANG-FULL E2, native-AOT leg)

Mirrors the `aarch64-backend` 0.10.0 change so narrow-width wrap is uniform across
both native host arches. A 64-bit register holds the full result of `add_u8
200, 100` (= 300); to make `uⁿ` types wrap mod-2ⁿ like the other backends, every
narrow **unsigned** op now appends `movabs rcx, <mask>; and <dst>, rcx`:

- `add_u8 200, 100` → `44`, `sub_u8 0, 1` → `255`, `mul_u8 16, 16` → `0`
- `not_u8 0` → `255`, `shl_u8 1, 8` → `0`; `u16`/`u32` wrap at their widths

Masking covers `add`/`sub`/`mul`/`div`/`mod`/`and`/`or`/`xor`/`shl`/`shr`/`neg`/`not`
for `u4`/`u8`/`u16`/`u32`; full-width and signed types are unchanged. See
`mask_narrow`. New structural tests prove the mask bytes are emitted (and that
`i64` is never masked); the **executed** value proof for x86_64 is the `lang-aot`
matrix on a Linux x86_64 CI runner (no in-repo x86 JIT loader — the `aarch64-backend`
provides the directly-executed value proof). Unblocks Nib **N6** / Oct **O2**.

## 0.11.0 — 2026-06-10 — McCarthy lambda (F7): `lispy_to_exit_code` builtin (LANG77 / W14b)

Adds `lispy_to_exit_code` to `V1_BUILTINS` (→ `call __twig_lispy_to_exit_code`), the
universal program-exit coercion for a polymorphic lambda result (W13b) — mirroring
the `aarch64-backend` change so native McCarthy lambda is uniform across both host
arches. New unit test `lispy_to_exit_code_lowers`.

## 0.10.0 — 2026-06-04 — ATOM/EQ predicate + truthy helpers (LANG77 / McCarthy L3b-2c-2)

Adds four `V1_BUILTINS` rows — `lispy_pair_p` (1), `lispy_not` (1),
`lispy_equal` (2), `lispy_truthy` (1), all returning a value → `CALL
__twig_lispy_*`. These back `ATOM` (`not(pair?)`), `EQ` (`equal?`) and the
`COND` truthiness normaliser the `lower_lisp_repr` pass inserts before
`jmp_if_false`. No new opcodes — the generic `call_builtin` dispatch handles
them. New host-independent test: the ATOM/EQ predicate + truthy sequence
lowers and emits the four external relocs.

## 0.9.0 — 2026-06-04 — lisp int unbox helper (LANG77 / McCarthy L3b-2c-1)

Adds one `V1_BUILTINS` row — `lispy_unbox_int` (1 arg, returns) → `CALL
__twig_lispy_unbox_int` — the helper the new `lower_lisp_repr` pass inserts
at the program-exit boundary to turn a tagged integer back into a raw
machine word for the process exit code. No new opcodes; the generic
`call_builtin` dispatch handles it.

New host-independent test: the full boxed `(CAR (CONS 7 9))` sequence (boxed
atoms → `lispy_cons` → `lispy_car` → `lispy_unbox_int` → ret) lowers and
emits external relocs to all three runtime symbols.

## 0.8.0 — 2026-06-04 — lisp runtime calls (LANG77 / McCarthy L3b-2b)

Adds three rows to the `V1_BUILTINS` helper table — `lispy_cons` (2 args),
`lispy_car` (1), `lispy_cdr` (1), all returning a value — so `call_builtin
"lispy_cons"` etc. dispatch to `CALL __twig_lispy_cons` in the linked C lisp
runtime (`twig-aot/runtime/lispy_runtime.c`). These are the runtime-call
form of cons/car/cdr (produced by
`iir_builtin_lowering::lower_heap_builtins_runtime`), keeping lisp values
NaN-box tagged rather than raw words.

**No new opcodes or emitter logic** — the existing generic `call_builtin`
dispatch marshals the args into the SysV/MsX64 arg registers and emits the
CALL with a `PltRel32` external relocation; the table rows are the entire
change. The L3b-1 `alloc`/`field_*` emitters remain as general-purpose heap
ops (no longer on the McCarthy cons path).

Two new host-independent tests: `(CAR (CONS 7 9))` via the runtime path
emits external relocations to `__twig_lispy_cons`/`__twig_lispy_car`, and a
wrong-arity `lispy_cons` call is softly refused.

## 0.7.0 — 2026-06-04 — heap cons cells (McCarthy Lisp L3b)

Mirror of the aarch64-backend 0.5.0 change: lower `alloc` / `field_store` /
`field_load` / `is_null` (the heap ops `lower_heap_builtins` produces from
`cons`/`car`/`cdr`/`null?`) on x86-64.

* **`alloc`** — `mov arg0, 16; call __twig_alloc_bytes; mov [rbp+slot], rax`.
* **`field_store ptr, idx, val`** — `mov [ptr + idx*8], val` (`mov_mem_r64`).
* **`field_load ptr, idx -> dest`** — `mov dest, [ptr + idx*8]`
  (`mov_r64_mem`); field 0 = car, field 1 = cdr.
* **`is_null x -> dest`** — `cmp x, 0; sete al; movzx rax, al`.
* Raw-word values (no NaN-boxing); `(CAR (CONS 7 9))` → raw `7`.  3 new
  unit tests mirroring the aarch64 ones.

## 0.6.0 — 2026-05-20 (LANG76 — byte memory ops + heap allocation)

Three new CIR opcodes that complete the substrate for Brainfuck and
future array work:

- `alloc_bytes <n> -> <dest>` — sugar for `call_builtin "alloc_bytes",
  n`; emits the same CALL into `__twig_alloc_bytes` and stores the
  returned pointer (RAX) into `dest`.  Also registered in
  `V1_BUILTINS` so `call_builtin "alloc_bytes"` works equivalently.
- `load_byte <ptr>, <offset> -> <dest>` — reads one byte from `[ptr +
  offset]`, zero-extends to 64 bits, stores into `dest`.  Lowers to:
  `mov rax,ptr; mov rcx,off; add rax,rcx; movzx rax,byte [rax]; mov
  [rbp+dest_slot],rax`.
- `store_byte <ptr>, <offset>, <value>` — writes the low 8 bits of
  `value` to `[ptr + offset]`.  Lowers to: `mov rax,ptr; mov rcx,off;
  add rax,rcx; mov rdx,val; mov byte [rax], dl`.

**Error handling.**  Missing operands → `MalformedInstr`; supplying a
dest to `store_byte` → `MalformedInstr`.

**Tests added (5):** alloc_bytes records `__twig_alloc_bytes` PltRel32
reloc + RAX→dest store; load_byte byte-sequence assertion; store_byte
byte-sequence assertion (forced empty REX); load_byte missing operand
refusal; store_byte with dest refusal.

## 0.5.0 — 2026-05-20 (LANG75 — generic `call_builtin` dispatch)

Adds a single CIR opcode `call_builtin "<name>", <args>` that dispatches
to runtime helpers via the V1 helper table.  Closes the per-helper
hard-coding pattern established by `io_out` (which is now sugar for
`call_builtin "print_i64"`).

**New CIR opcode:**

- `call_builtin "<name>", <arg0>, <arg1>, …` — looks `name` up in the
  V1 helper table, marshals args into the ABI's argument registers,
  emits `call rel32` against `__twig_<name>` with a `PltRel32`
  external relocation, and (if the helper returns) stores `RAX` into
  the dest slot.

**V1 helper table (shared with `aarch64-backend`):**

| Name           | Args     | Returns |
|----------------|----------|---------|
| `print_i64`    | `[i64]`  | no      |
| `putchar`      | `[i32]`  | no      |
| `getchar`      | `[]`     | yes     |
| `print_string` | `[ptr,i64]` | no   |
| `input_i64`    | `[]`     | yes     |
| `exit`         | `[i32]`  | no      |

**Error handling.**  Unknown helper names, wrong arity, and dest/void
mismatches all produce `BackendError::MalformedInstr` (the spec's
"BackendRefused" — a soft refusal, not a panic).

**Behaviour preserved.**  The existing `io_out` dispatch is unchanged
and still emits the same bytes; `call_builtin "print_i64", v` produces
the same `call __twig_print_i64` it always did.

**Tests added (9):** marshal arg into RDI / RCX (SysV / MS x64),
`getchar` stores RAX to dest, `print_string` marshals two args,
unknown name refuses, wrong arity refuses, void-with-dest refuses,
returning-without-dest refuses, `print_i64`-via-builtin matches
`io_out`.

## 0.4.0 — 2026-05-14 (LANG43 phase 6 — globals + io_out)

Adds the last CIR opcodes needed to match `aarch64-backend`'s
LANG39/LANG40/LANG41 coverage.  After this release, the same Twig
programs that compile and run end-to-end on macOS ARM64 will compile
and run end-to-end on Linux x86-64 and Windows x86-64 once LANG45
(object emitters) and LANG46 (twig-aot driver) land.

**New CIR opcodes:**

- `global_load name → dest` — read from a slot in the `_twig_globals`
  data section:
  ```
  lea  rax, [rip + _twig_globals]   ; PcRel32 reloc
  mov  rax, [rax + slot*8]          ; load 64-bit value
  mov  [rbp + dest_slot], rax
  ```
  Note that x86-64's RIP-relative addressing collapses ARM64's
  ADRP+ADD pair into a single `LEA` + a single `PcRel32` reloc
  record — much simpler than the AArch64 `GlobalWordReloc` shape.

- `global_store name, val` — write to a slot:
  ```
  mov  rcx, [val_slot]               ; load value
  lea  rax, [rip + _twig_globals]    ; PcRel32 reloc
  mov  [rax + slot*8], rcx
  ```

- `io_out val` — call `__twig_print_i64`:
  ```
  mov  <arg0>, [rbp + val_slot]      ; SysV: RDI; MS x64: RCX
  call __twig_print_i64               ; PltRel32 reloc
  ```
  Stack alignment at the call is correct without per-call adjustment:
  the prologue established RSP ≡ 0 (mod 16), and CALL pushes 8 bytes
  for the return address, giving RSP ≡ 8 (mod 16) at helper entry —
  exactly what the ABI requires.  MS x64 shadow space is already
  reserved in the prologue.

**New public function:**

- `compile_function_with_globals(ctx, ir, abi, global_slots) ->
  (Vec<u8>, Vec<Reloc>)` — resolves global names through `global_slots`
  into slot indices and emits the corresponding `PcRel32` relocs
  alongside any cross-function / runtime `PltRel32` relocs.

`compile_function` and `compile_function_with_relocs` are unchanged
for callers that don't use globals (passing an empty slot map is
equivalent).

## 0.3.0 — 2026-05-14 (LANG43 phase 5 — calls + relocations)

Adds cross-function `call` lowering and external relocation surfacing.

**New CIR opcode:**

- `call callee_name, arg0, …, argN` — argument marshalling into the
  ABI's argument registers (System V: RDI/RSI/RDX/RCX/R8/R9 up to 6;
  MS x64: RCX/RDX/R8/R9 up to 4), then `CALL rel32` to the callee, then
  store RAX into the destination slot.
  - Self-recursive calls (callee == current function) emit
    `call_label(entry_label)` resolved within this function's bytes
    by the encoder's fixup pass.
  - Cross-function calls emit `call_rel32(callee_name, PltRel32)` and
    record an external relocation for the AOT linker to patch after
    all function bodies are concatenated.

**New public function:**

- `compile_function_with_relocs(ctx, ir, abi) -> (Vec<u8>, Vec<Reloc>)`
  — returns both the function bytes and the list of external
  relocations.  `compile_function` is unchanged for callers that
  don't need them.

Re-exports `x86_64_encoder::ExternalReloc as Reloc` for parity with
`aarch64-backend`'s `Reloc` re-export.

## 0.2.0 — 2026-05-14 (LANG43 — LANG38-parity wave)

Extends the V1 backend with the same opcodes `aarch64-backend` gained
in its LANG38 release.  Same CIR coverage now compiles on both
backends.

**New CIR opcodes:**

- `div_<ty>`, `mod_<ty>` — integer division and modulo.  Signed types
  use `CQO` + `IDIV`; unsigned types use `XOR rdx, rdx` + `DIV`.
  Quotient lives in `RAX`, remainder in `RDX` (sequenced by hand —
  no register-allocator surprises because RAX/RCX/RDX were already
  reserved in V1).
- `and_<ty>`, `or_<ty>`, `xor_<ty>` — bitwise logical (64-bit).
- `not_<ty>` — bitwise complement (`NOT r/m64`).
- `shl_<ty>` — logical shift left (`SHL r/m64, CL`).
- `shr_<ty>` — arithmetic shift right (`SAR`) for signed types,
  logical shift right (`SHR`) for unsigned types.
- `neg_<ty>` — two's-complement negate (`NEG r/m64`).

All shifts use `CL` as the count register (x86-64 ISA constraint);
the backend pre-loads `rhs` into `RCX` before issuing the shift.

Still out of scope (added by later phases):
- Calls + external relocations (phase 5)
- Globals + `io_out` (phase 6)
- Floats / closures

## 0.1.0 — 2026-05-14 (LANG43)

Initial release.  V1 backend matching the `aarch64-backend` V1 baseline.

**ABIs supported:**

- System V AMD64 (Linux, macOS x86-64, FreeBSD) — arg regs RDI/RSI/RDX/RCX/R8/R9
- Microsoft x64 (Windows) — arg regs RCX/RDX/R8/R9, 32-byte shadow space reserved in prologue

Both ABIs share the same CIR lowering logic — only the prologue's arg
register set and shadow-space allocation differ.

**CIR coverage:**

- `const_<ty>` — integer + bool literals
- `mov_<ty>` — typed copy
- `add_<ty>`, `sub_<ty>`, `mul_<ty>` — integer arithmetic (64-bit; result not masked to width)
- `cmp_<rel>_<ty>` — signed and unsigned comparisons, 6 predicates × signed/unsigned
- `label`, `jmp`, `jmp_if_true`, `jmp_if_false` — control flow
- `ret_<ty>`, `ret_void` — return (loads value into RAX before epilogue)
- `type_assert` — lowered to `UD2` trap (AOT has no deopt path)

**Out of scope for V1 (added by follow-up phases):**

- Division / modulo (`IDIV` / `DIV` — phase 4)
- Logical (AND/OR/XOR/NOT) and shifts (phase 4)
- Calls + external relocations (phase 5)
- Globals + `io_out` (phase 6)
- Floats / SSE
- Closures
- Local register allocator (V1 uses pure stack spill)

**Register allocation:** stack spill.  Every virtual lives at
`[rbp - 8 - slot_idx*8]`.  RAX, RCX, RDX reserved as scratch.
