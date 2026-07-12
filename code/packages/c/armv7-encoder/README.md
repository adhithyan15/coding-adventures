# armv7-encoder (C)

A pure ARMv7-A (A32) instruction encoder in pure ISO C17. A faithful port of the
Rust `armv7-encoder` crate.

ARMv7-A is the 32-bit ARM instruction set of billions of Cortex-A7/A8/A9-era
phone-class SoCs. This encoder knows nothing about IR — it is canonical
instruction-word constants plus typed `encode_*` helpers that return the exact
32-bit machine word.

## API

```c
#include "armv7_encoder.h"

armv7_encode_mov_imm(0, 42);   /* MOV r0, #42  == 0xE3A0002A */
armv7_encode_mov_reg(0, 1);    /* MOV r0, r1   == 0xE1A00001 */
ARMV7_BX_LR;                   /* BX LR        == 0xE12FFF1E */
```

Every value is an exact ARM A32 encoding; the helpers are branch-free bit-ops
(register indices are masked to 4 bits — out-of-range values are the caller's
responsibility, matching the Rust crate). The word constants are macros.

## Portability

Pure ISO C17 — no extensions. Compiles clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../iso-harness).

## Development

```bash
sh BUILD
```
