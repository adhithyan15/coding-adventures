# LANG39 — AOT Global Variables

**Status**: Implementation  
**Depends on**: LANG38 (merged), LANG32 (`global_load`/`global_store` IIR opcodes)

---

## Motivation

Twig programs use `(define x <expr>)` at the top level to create named global
values.  In the interpreter these are dispatched via `call_builtin "global_set"`
and `call_builtin "global_get"`.  `iir-builtin-lowering` converts these to the
`global_store Str(name), val` / `global_load Str(name) → dest` opcodes defined
in LANG32.

The AOT pipeline did not previously handle either form:

- `pre_lower_aot_builtins` did not call `iir-builtin-lowering`
- `aarch64-backend` had no handlers for `global_load` / `global_store`
- `code-packager` emitted only a `__text` section (no `__data` for mutable globals)

LANG39 closes all three gaps.  After this change, programs like

```twig
(define x 5)
(define (f n) (+ n x))
(f 3)   ; → 8
```

compile to a runnable ARM64 Mach-O binary.

---

## Design

### Where globals live: `__DATA, __data` section

Global variables are stored in a zero-initialised 8-byte-per-slot array in the
Mach-O `__DATA` segment (`__data` section).  Each unique global name is assigned
a consecutive slot index at module-compile time:

```
_twig_globals:   [0×8 bytes]    ; slot 0 = first global name
                 [8×8 bytes]    ; slot 1 = second global name
                 ...
```

The array is emitted in the Mach-O object file as a `__data` section and
exported via the `_twig_globals` symbol.  The system linker (`ld`) places it in
the final executable.

### Instruction sequence per access

**`global_store Str("x"), %v0`** (write):

```
LDR  X0, [SP, #slot_of_v0]       ; load value from stack frame
ADRP X1, _twig_globals@PAGE      ; get globals base page (patched by ld)
ADD  X1, X1, _twig_globals@PAGEOFF   ; add page offset (patched by ld)
STR  X0, [X1, #slot_x * 8]       ; write to global slot
```

**`global_load Str("x") → %dest`** (read):

```
ADRP X1, _twig_globals@PAGE
ADD  X1, X1, _twig_globals@PAGEOFF
LDR  X0, [X1, #slot_x * 8]       ; read from global slot
STR  X0, [SP, #slot_of_dest]     ; spill to stack frame
```

### ARM64 relocations in the Mach-O object file

The ADRP and ADD instructions need two Mach-O ARM64 relocations per access so
`ld` can patch the 21-bit page offset and 12-bit page-relative offset:

| Instruction | Reloc type       | r_type | r_pcrel | Purpose |
|-------------|------------------|--------|---------|---------|
| ADRP        | ARM64_RELOC_PAGE21   | 1 | 1 | Page address of `_twig_globals` |
| ADD         | ARM64_RELOC_PAGEOFF12 | 2 | 0 | Low-12-bit offset within page |

Both records reference the `_twig_globals` symbol (symbol table index 1 in the
generated object file; index 0 is reserved for `_main`).

### Mach-O layout

`pack_object_with_globals` produces:

```
Offset          │ Size   │ Content
────────────────┼────────┼─────────────────────────────────────────
0               │ 32     │ mach_header_64 (ncmds=3)
32              │ 232    │ LC_SEGMENT_64 (header + 2 section_64s)
264             │ 24     │ LC_BUILD_VERSION
288             │ 24     │ LC_SYMTAB
── HEADER = 312 ───────────────────────────────────────────────────
312             │ N      │ __text bytes (machine code)
312+N           │ M      │ __data bytes (zero-init globals, M = n_slots * 8)
312+N+M         │ 2*R*8  │ text relocation records (2 per global access)
312+N+M+2*R*8   │ 32     │ 2 nlist_64 entries: _main, _twig_globals
…               │ S      │ string table: "\0_main\0_twig_globals\0"
```

Where `R` = number of global accesses (loads + stores across all functions).

---

## Files changed

| File | Package | Change |
|------|---------|--------|
| `aarch64-encoder/src/lib.rs` | aarch64-encoder 0.2.1 | `+adrp_placeholder(rd)` |
| `aarch64-backend/src/lib.rs` | aarch64-backend 0.2.1 | `GlobalWordReloc`, `compile_with_globals` |
| `code-packager/src/macho_object.rs` | code-packager | `GlobalByteReloc`, `pack_object_with_globals` |
| `twig-aot/Cargo.toml` | twig-aot | add `iir-builtin-lowering` dep |
| `twig-aot/src/lib.rs` | twig-aot | run `lower_global_io`, scan globals, use new packager fn |

---

## Data-type pipeline

```
twig-ir-compiler output:
  call_builtin "global_set" %name_reg %val_reg
  call_builtin "global_get" %name_reg  →  %dest

iir-builtin-lowering::lower_global_io (added to prepare_module_for_aot):
  global_store Str("x"), Var("%val_reg")
  global_load  Str("x")  →  %dest

aot_specialise (fallback path):
  global_store  srcs=[Var("x"), Var("%val_reg")]   (Str→Var lifting)
  global_load   srcs=[Var("x")]  dest=%dest

aarch64-backend emit_global_store / emit_global_load:
  LDR X0, [SP, #val_slot]; ADRP X1; ADD X1; STR X0, [X1, #slot*8]
  ADRP X1; ADD X1; LDR X0, [X1, #slot*8]; STR X0, [SP, #dest_slot]
```

---

## Limitations (out of scope for LANG39)

- **Float globals**: globals are always 64-bit integer slots; float support is deferred
- **String/heap globals**: depend on LANG40 (heap allocation)
- **Cross-module globals**: the current linker is single-module only
- **Slot count cap**: slot index × 8 must fit in a 12-bit LDR/STR offset (≤ 4095 slots, i.e. 32760 bytes)
- **Type propagation**: `global_load` produces a `"any"`-typed result in CIR; downstream arithmetic defaults to `i64` via `default_any_to_i64`

---

## Test plan

- `aarch64-encoder`: `adrp_placeholder` encoding test
- `aarch64-backend`: `global_load` / `global_store` CIR handler tests (round-trip via `compile_with_globals`)
- `code-packager`: `pack_object_with_globals` produces correct Mach-O layout
- `twig-aot`: `(define x 5) x` no longer returns `BackendRefused`; compiles to object bytes
