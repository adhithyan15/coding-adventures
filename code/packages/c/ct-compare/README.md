# ct-compare (C)

A pure ISO **C17** library for **constant-time** byte comparison. A faithful
port of the Rust `ct-compare` crate.

It compiles clean under **GCC, Clang, and MSVC** with `-std=c17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c17 /permissive- /W4 /WX` on
MSVC), via the shared [`iso-harness`](../iso-harness/). Standard library only.

## Why constant-time?

A naive `memcmp` — or an equality loop that returns as soon as it finds a
mismatch — runs for a different amount of time depending on **where** two values
first differ. When one of the values is a secret (a MAC/auth tag, a derived key,
a password hash), that timing is a side channel: an attacker who can measure it
can recover the secret one byte at a time.

These routines do the **same work for every byte** regardless of its value: they
fold all differences into an accumulator with no data-dependent branch, then
inspect the accumulator once at the end. An optimiser barrier (a read through a
`volatile` object — the pure-ISO stand-in for Rust's `core::hint::black_box`)
keeps the compiler from re-introducing an early exit.

## API

```c
#include "ct_compare.h"

/* Same length AND same bytes? (length is treated as public.) */
int  ct_eq(const uint8_t *a, size_t alen, const uint8_t *b, size_t blen);

/* Same n bytes? (no length check — both buffers must be >= n bytes.) */
int  ct_eq_fixed(const uint8_t *a, const uint8_t *b, size_t n);

/* Branchless select: writes a if choice != 0, else b, into out (n bytes). */
void ct_select_bytes(const uint8_t *a, const uint8_t *b, int choice, size_t n,
                     uint8_t *out);

/* Constant-time equality of two 64-bit values. */
int  ct_eq_u64(uint64_t a, uint64_t b);
```

`ct_eq` compares the length first — length is not secret — and only the byte
comparison is constant-time. All routines return `1`/`0` for true/false.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests check equality, first/last-byte differences, and — the key property — that
**every single-bit flip at every byte position** is detected, which a
short-circuiting bug would miss.
