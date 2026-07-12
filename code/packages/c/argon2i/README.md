# argon2i (C)

**Argon2i** — data-independent memory-hard password hashing (RFC 9106) — in pure
ISO C17. A faithful port of the Rust `argon2i` crate.

Argon2 (the Password Hashing Competition winner) fills a large memory matrix
(`memory_cost` KiB) with a BLAKE2b-derived compression function and reads it back
so that an attacker cannot trade memory for speed. The *i* variant picks each
reference block from a deterministic pseudo-random stream that does **not** depend
on the password or any memory contents — a constant memory-access pattern that
defeats side-channel observers, at the cost of being the easiest variant for
GPUs/ASICs to parallelise. **Prefer Argon2id for password hashing.**

The addressing stream (RFC 9106 §3.4.2) generates the `(J1, J2)` pairs by running
the compression function twice over a counter block, so the reference index never
depends on secret data.

```
H0        = BLAKE2b(params || pass || salt || key || ad)
B[i][0/1] = H'(H0 || 0/1 || i)                  (first two columns per lane)
B[i][j]   = G(B[i][j-1], B[l'][z'])             (fill; XOR into place after pass 0)
tag       = H'(XOR of the last column across lanes)
```

`G` is the Argon2 compression, `H'` the variable-length BLAKE2b extender, and
`(l', z')` the data-independent reference block. Built on the sibling
[`blake2b`](../blake2b) package.

## API

The tag is written into a **caller-provided buffer** of `tag_length` bytes.

```c
#include "argon2i.h"

uint8_t tag[32];
Argon2iOptions opts = {0};            /* or pass NULL for no key / no AD */
opts.key = key; opts.key_len = 8;
Argon2iStatus st = argon2i(password, pw_len, salt, salt_len,
                           /*time*/ 3, /*memory KiB*/ 32, /*parallelism*/ 4,
                           /*tag_length*/ 32, &opts, tag);
```

Parameters: `time_cost` (>= 1), `memory_cost` in KiB (>= 8*parallelism),
`parallelism` (1..2^24-1), `tag_length` (>= 4). `opts` may be NULL. Status codes
cover each invalid-parameter case (short/long salt, small tag, memory below
minimum, zero time cost, bad parallelism, unsupported version) plus
`ARGON2D_ALLOC_ERROR` (the `calloc(m', 128 words)` matrix uses a checked
multiply).

## Portability

Pure ISO C17 — compiles clean under GCC, Clang, and MSVC with `-pedantic-errors`
/ `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../iso-harness).

## Development

```bash
# Compile and run the RFC 9106 §5.2 vector test under every C compiler.
sh BUILD
```
