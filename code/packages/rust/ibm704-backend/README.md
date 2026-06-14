# ibm704-backend

IBM 704 backend for `jit-core` / `aot-core`.

L4 of the McCarthy Lisp implementation — see
[`MCCARTHY-LISP-PLAN.md`](../../../specs/MCCARTHY-LISP-PLAN.md).
Mirror of `ge225-backend` / `intel4004-backend` / `armv7-backend` /
`intel8008-backend` / `riscv-backend` in shape, just for the
36-bit IBM 704 (1954) — the silicon Lisp was first run on
in 1959.

## v0.1.0 scope — minimal viable

Same scope as the other minimal-viable historical-arch backends:
just enough to keep the lang-aot IBM 704 e2e smoke test passing
byte-for-byte.

| CIR family | Status |
|------------|--------|
| `const_*` (15-bit unsigned immediate, single-var case) | ✓ → `CLA n` |
| `ret_*` (value matches the last `const_*` dest) | ✓ → `HTR 0` (halt) |
| `ret_void` | ✓ → `HTR 0` |
| Anything else | returns `None` (AOT reports the gap; per the v0.1.0 scope decision the historical-arch backends only handle no-CONS programs) |

Per the McCarthy Lisp plan, **CONS is not supported on the IBM
704 backend in v0.1.0** (the historical-arch backends don't have
a heap allocator yet).  This matches the v0.1.0 scope decision
for every historical-arch lane in the migration.

## Wire format

5 bytes per 36-bit word, low byte first, high 4 bits of the top
byte zeroed.  Same packing convention `ge225-encoder` uses
(20-bit words → 3 bytes) extended to 36 bits.

## Twig `42` canonical sequence

```
CLA 42      ; opcode=0o500, address=42 → word 0xA_0000_002A
HTR  0      ; opcode=0o420, address=0  → word 0x8_8000_0000   (canonical halt)
```

Packed on disk: `[0x2A, 0x00, 0x00, 0x00, 0x0A, 0x00, 0x00, 0x00, 0x80, 0x08]`
(10 bytes).
