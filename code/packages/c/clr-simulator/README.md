# clr-simulator (C)

A **CLR (.NET Common Language Runtime) bytecode simulator** in pure ISO C17. A
faithful port of the Rust [`clr-simulator`](../../rust/clr-simulator) crate: a
type-inferring, stack-based virtual machine for a subset of Microsoft's CIL (the
CLR's bytecode). A companion to the other historical/managed VM ports in the
repo.

## What makes the CLR interesting

Unlike the JVM's typed opcodes (`iadd`, `fadd`, …), the CLR **infers operand
types from the stack**: one `add` works for every numeric type. Values are
either 32-bit integers or object references (`null`, or an index into a heap of
`object[]` arrays), and this simulator implements exactly enough of the ISA to
run boxing, object arrays, method calls, and conditional branches.

## Value model

| kind | meaning |
|------|---------|
| `CLR_INT` | a 32-bit integer |
| `CLR_REF`, `ref_some == 0` | the null reference |
| `CLR_REF`, `ref_some == 1` | a heap index (`ref_idx`) into an `object[]` |

Stack, local, and argument **slots** are optional (`ClrSlot.present == 0` is an
unset slot, distinct from a null value).

## Bounds safety

The Rust original indexes slices (`bytecode[pc + 1]`), which *panics* on an
out-of-range access — safe, because Rust bounds-checks every index. C does not,
so this port treats the bytecode as **untrusted input**: every operand read and
every heap/array index is checked, returning a `ClrStatus` where the Rust code
would have panicked. Arithmetic wraps modulo 2^32 through `uint32_t`, so there is
no signed-overflow UB.

## API

```c
#include "clr_simulator.h"

/* ldc.i4 1; ldc.i4 2; add; stloc.0; ldloc.0; ret  ->  local 0 == 3 */
static const uint8_t prog[] = {0x17, 0x18, 0x58, 0x0A, 0x06, 0x2A};
ClrSimulator *sim = clr_new();
clr_load(sim, prog, sizeof prog, 16);
size_t steps;
clr_run(sim, 100, &steps);          /* CLR_OK, steps == 6, halted */
ClrSlot loc;
clr_local_at(sim, 0, &loc);         /* loc.value.i == 3 */
clr_free(sim);
```

- `clr_new` / `clr_free` — lifecycle (`clr_free` is NULL-safe).
- `clr_load` / `clr_load_program` — load one method, or a whole method table.
- `clr_step` / `clr_run` — execute one / many instructions (status-returning).
- `clr_halted` / `clr_pc` / `clr_stack_len` / `clr_stack_at` / `clr_stack_top`
  / `clr_local_at` — inspection.
- `clr_encode_ldc_i4` / `clr_encode_stloc` / `clr_encode_ldloc` — compact
  instruction encoders.

Errors are reported through the `ClrStatus` enum (`CLR_ERR_DIVIDE_BY_ZERO`,
`CLR_ERR_INDEX_OUT_OF_RANGE`, `CLR_ERR_BYTECODE_OVERRUN`, …).

## Building

Builds through the shared [`iso-harness`](../iso-harness) engine under every ISO
C compiler on `PATH` with `-std=c17 -pedantic-errors -Wall -Wextra -Werror`:

```sh
sh BUILD          # POSIX: gcc and/or clang
```

Each compiler prints `N checks, 0 failed`.
