# ibm704-backend

Minimal IBM 704 backend for `jit-core` / `aot-core` and the McCarthy Lisp
historical target.

## Supported CIR

| CIR family | Lowering |
|---|---|
| `const_*` with values 0–32767 | `CLA` from an addressable literal pool |
| `ret_*` of the current accumulator variable | `HTR 0` |
| `ret_void` | `HTR 0` |
| Anything else | Explicit unsupported-operation error |

`CLA Y` reads memory word `Y`; its address field is not an immediate. The
backend therefore lays out instructions first and data literals second. It
rejects output larger than the IBM 704's 32K-word address space.
Module emitters use `compile_at` so each function's literal addresses are
relocated to its absolute load address and the complete concatenated image
cannot cross the 32K-word boundary.

## Canonical `42`

```text
address 0: CLA 2
address 1: HTR 0
address 2: +42
```

The canonical big-endian transport is:

```text
01 40 00 00 02  00 00 00 00 00  00 00 00 00 2A
```

The backend is still intentionally emit-only. RCPU-003 will wire this byte
stream to the Rust IBM 704 functional simulator.
