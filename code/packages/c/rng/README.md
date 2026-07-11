# rng (C)

A pure ISO **C17** collection of deterministic pseudo-random number generators. A
faithful port of the Rust `rng` crate.

It compiles clean under **GCC, Clang, and MSVC** with
`-std=c17 -pedantic-errors -Wall -Wextra -Werror` (and `/std:c17 /permissive-
/W4 /WX` on MSVC), via the shared [`iso-harness`](../iso-harness/). No compiler
extensions, no third-party dependencies.

## The three generators

| Type | Algorithm | Notes |
| --- | --- | --- |
| `rng_lcg` | 64-bit Linear Congruential Generator | returns the high 32 bits |
| `rng_xorshift64` | Marsaglia's Xorshift64 | three XOR-shifts, no multiply; seed 0 → 1 |
| `rng_pcg32` | O'Neill's PCG32 (XSH-RR) | LCG state with a permuted output |

Each has the same interface — `init`, `next_u32`, `next_u64`, `next_float` (a
`double` in `[0, 1)`), and `next_int_in_range` (rejection-sampled to avoid modulo
bias):

```c
#include "rng.h"

rng_pcg32 g;
rng_pcg32_init(&g, 42);
uint32_t r  = rng_pcg32_next_u32(&g);
double   f  = rng_pcg32_next_float(&g);       /* [0, 1) */
int64_t  d  = rng_pcg32_next_int_in_range(&g, 1, 6);  /* a die roll */
```

**These are not cryptographically secure** — don't use them for keys or nonces
(see the sibling `chacha20-poly1305` / `csprng` for that).

## Implementation notes

- **Pure ISO, no 128-bit type.** All arithmetic is 32/64-bit unsigned (which
  wraps by definition in C), so no `unsigned __int128` extension is needed.
- **Faithful output.** The generators reproduce the crate's reference values
  exactly (LCG seed 1 → `1817669548, 2187888307, 2784682393`, etc.), so a C and a
  Rust program seeded identically produce identical streams.
- The PCG rotate-right uses the `(32 - rot) & 31` form so a zero rotation is
  well-defined (no undefined shift-by-32).

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests pin the reference values for all three generators and check determinism,
seed divergence, the Xorshift64 zero-seed remap, `next_float` range, and
`next_int_in_range` bounds.
