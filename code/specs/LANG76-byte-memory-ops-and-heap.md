# LANG76 — Byte-level memory ops + heap allocation for the AOT chain

**Status:** Draft — 2026-05-20
**Depends on:** LANG43, LANG44, LANG75
**Unblocks:** BF07, PL05 (arrays), OCT02 (byte arrays / strings)

## Motivation

The AOT backends (`x86_64-backend`, `aarch64-backend`) today only
addressing the data layout produced by `LANG39-aot-globals.md`: a
fixed-size `_twig_globals` slab of 8-byte slots accessed by `global_load`
and `global_store`.  There is no general byte-level memory addressing.
That blocks:

- **Brainfuck** — the language is fundamentally a 30 000-byte tape with
  byte-granular `+`/`-`/`<`/`>`/`.`/`,`.  `brainfuck-iir-compiler`
  already emits `load_mem` / `store_mem` IIR ops but neither backend
  lowers them.
- **BASIC arrays** (`DIM A(100)`) — even integer arrays need
  `[base + idx*8]` addressing through a runtime-allocated buffer.
- **Strings** in any frontend — strings are byte buffers with a length.
- **Oct memory** — the Intel-8008 frontend models a small RAM that
  needs byte addressing.

LANG76 introduces three new CIR opcodes and one runtime helper.

## Non-goals

- **Garbage collection.**  V1 is malloc/never-free; long-running
  programs leak, which is fine for AOT'd command-line scripts.  GC is
  a separate later spec.
- **Bounds checking.**  V1 has none.  An array spec layered on top can
  add `type_assert` guards if it wants.
- **Atomic memory ops.**  Concurrency is out of scope.
- **`mmap` / file-backed memory.**  Process heap only.

## New CIR opcodes

### `alloc_bytes <byte_count> -> <ptr>`

Allocates a buffer of `byte_count` bytes (zero-initialised) on the
process heap.  Lowers to `call_builtin "alloc_bytes", <byte_count>`
under the hood per LANG75; this op is sugar for the common case so
frontends don't have to think about argument register marshalling.

The result is a 64-bit pointer (treated as `i64` in IIR).  The
pointer is valid until process exit; nothing frees it in V1.

### `load_byte <ptr>, <offset> -> <dest>`

Reads one byte from `[ptr + offset]` and zero-extends to a 64-bit
destination.

| Backend | Lowering |
|---|---|
| x86_64 | `movzx rax, byte ptr [rbp + ptr_slot]` (load pointer) → `movzx rcx, byte ptr [rax + rcx_offset]`.  Actually: load ptr → rax; load offset → rcx; `movzx rax, byte ptr [rax + rcx]`; store rax → dest. |
| aarch64 | `ldrb` w-form: `ldrb w0, [x0, x1]` after loading ptr/offset; store to slot. |

### `store_byte <ptr>, <offset>, <value>`

Writes the low 8 bits of `<value>` to `[ptr + offset]`.  No dest.

| Backend | Lowering |
|---|---|
| x86_64 | load ptr → rax; load offset → rcx; load value → rdx; `mov byte ptr [rax + rcx], dl`. |
| aarch64 | `strb w_value, [x_ptr, x_offset]`. |

### Sketch: 64-bit word ops (deferred to a future spec)

`load_word_64` / `store_word_64` would mirror the byte ops with
8-byte alignment requirements.  Not needed for BF; deferred until
arrays-of-i64 land in a higher-level frontend.

## Runtime helper

`twig_runtime.c` gains:

```c
#include <stdlib.h>
#include <string.h>

void *__twig_alloc_bytes(int64_t n) {
    if (n <= 0) return NULL;
    void *p = calloc(1, (size_t)n);   /* zero-initialised */
    return p;
}
```

The pointer is returned via `RAX` / `X0` per LANG75's call ABI.  Frontends
treat the result as an `i64` pointer and store it into a virtual slot
the same way any other call result is stored.

## Backend tests

For each backend (`x86_64-backend`, `aarch64-backend`):

- `load_byte_zero_extends`: store `0xFF` to a 1-byte buffer, load
  back, assert RAX/X0 reads `0x00000000_000000FF` (no sign extension).
- `store_byte_writes_only_low_8`: write `0x1234_5678_9ABC_DEF0`,
  read back, assert only `0xF0` landed at the target byte.
- `load_then_store_round_trip`: allocate 16 bytes, write a pattern,
  read it back, compare.
- `alloc_bytes_returns_valid_pointer`: V1 just asserts the return
  value is non-zero; the actual `calloc` lives in the runtime
  archive and is tested at the end-to-end smoke level.

## End-to-end test

`twig-aot/tests/heap_byte_io_smoke.rs` (host-gated):

```text
A program that:
  1. alloc_bytes 4 -> buf
  2. store_byte buf, 0, 'H'
  3. store_byte buf, 1, 'i'
  4. store_byte buf, 2, '\n'
  5. call_builtin "print_string", buf, 3
  6. ret 0

Run the executable, capture stdout, assert it printed "Hi\n".
```

Same shape on Linux and Windows runners.

## Risk register

| Risk | Mitigation |
|---|---|
| Byte ops on x86-64 need careful REX prefix handling for `SPL`/`BPL`/`SIL`/`DIL` 8-bit registers | Stick to using `RAX` (`AL`) and `RDX` (`DL`) as scratch — both are legacy-byte-addressable without REX surprises.  Existing scratch policy already reserves these. |
| `calloc` failure (OOM) returns NULL; programs with no null-check crash | Document that V1 doesn't bounds-check or null-check.  Crashes on OOM are acceptable for command-line scripts. |
| Heap memory leaks across calls in long-running programs | V1 has no GC.  Documented limitation.  Future spec can introduce arenas or `__twig_free`. |
