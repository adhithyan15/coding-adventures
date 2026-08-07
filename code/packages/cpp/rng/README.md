# rng (C++)

A pure ISO **C++17**, header-only collection of deterministic pseudo-random
number generators, in namespace `ca::rng`. A faithful port of the Rust `rng`
crate.

It compiles clean under **GCC, Clang, and MSVC** with `-std=c++17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c++17 /permissive- /W4 /WX` on
MSVC), via the shared [`iso-harness`](../../c/iso-harness/). Standard library
only.

## The three generators

| Class | Algorithm |
| --- | --- |
| `ca::rng::Lcg` | 64-bit Linear Congruential Generator (high 32 bits) |
| `ca::rng::Xorshift64` | Marsaglia's Xorshift64 (seed 0 → 1) |
| `ca::rng::Pcg32` | O'Neill's PCG32 (XSH-RR permuted output) |

```cpp
#include "rng.hpp"

ca::rng::Pcg32 g(42);
std::uint32_t r = g.next_u32();
double        f = g.next_float();          // [0, 1)
std::int64_t  d = g.next_int_in_range(1, 6);  // rejection-sampled die roll
```

Each class provides `next_u32`, `next_u64`, `next_float` (a `double` in `[0, 1)`),
and `next_int_in_range` (a shared templated helper, modulo-bias-free). The
generators reproduce the crate's reference values exactly.

**Not cryptographically secure** — don't use for keys or nonces.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests pin the reference values and check determinism, seed divergence, the
zero-seed remap, and range/float bounds.
