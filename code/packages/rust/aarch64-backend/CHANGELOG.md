# Changelog — `aarch64-backend`

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
