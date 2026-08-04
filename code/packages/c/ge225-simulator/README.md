# ge225-simulator (C)

A **GE-225 CPU simulator** in pure ISO C17. A faithful port of the Rust
[`ge225-simulator`](../../rust/ge225-simulator) crate: a fetch-decode-execute
emulator for the GE-225 (1959), the mainframe **Dartmouth BASIC** was designed
on. It models the 20-bit machine — accumulator `A`, extension `Q`,
index-register groups, a bit-addressed memory, the console typewriter and card
reader, and the full memory-reference / fixed / shift instruction set.

## Highlights

- **Instruction assembly** — `ge225_encode_instruction` / `ge225_assemble_fixed`
  / `ge225_assemble_shift`, and `ge225_pack_words` / `ge225_unpack_words` (3
  bytes per 20-bit word).
- **The simulator** — `ge225_new` / `ge225_free`, `ge225_load_words`,
  `ge225_step` / `ge225_run`, plus state accessors (`ge225_get_a`, `_q`, `_pc`,
  `ge225_read_word`, `ge225_get_x_word`, …), the console typewriter, and the
  card reader.
- Double-precision (40-bit) arithmetic (`DLD`/`DAD`/`MPY`/`DVD`/…), index-register
  ops, and the elaborate shift family (`SAN`, `SNA`, `NAQ`, `SCD`, `SLD`, `NOR`,
  …).

## UB-safe arithmetic

The Rust source leans on `i32`/`i64` wrapping shifts (e.g. `(v << 24) >> 24` for
sign extension). Ported naively those would be signed-overflow **undefined
behaviour** in C. This port does every left shift and double-word bit-shuffle on
**unsigned** types (defined wrap) and keeps only arithmetic right shifts signed
(C makes those implementation-defined-arithmetic, matching Rust). The whole
suite is clean under AddressSanitizer + UndefinedBehaviorSanitizer.

## API

```c
#include "ge225_simulator.h"

Ge225Simulator *s = ge225_new(4096);
int32_t prog[] = {/* LDA 10 */ 0, /* ADD 11 */ 0, /* STA 12 */ 0, ...};
ge225_load_words(s, prog, n, 0);
ge225_run(s, 4);
int32_t a = ge225_get_a(s);            /* accumulator */
int32_t out; ge225_read_word(s, 12, &out);
ge225_free(s);
```

Errors surface as a `GeStatus` (`GE_ERR_ADDRESS_OUT_OF_RANGE`,
`GE_ERR_DIVIDE_BY_ZERO`, `GE_ERR_RANGE`, …).

## Building

Builds through the shared [`iso-harness`](../iso-harness) engine under every ISO
C compiler on `PATH` with `-std=c17 -pedantic-errors -Wall -Wextra -Werror`:

```sh
sh BUILD          # POSIX: gcc and/or clang
```

Each compiler prints `N checks, 0 failed`.
