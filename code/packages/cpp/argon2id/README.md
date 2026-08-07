# argon2id (C++)

**Argon2id** — hybrid memory-hard password hashing (RFC 9106) — in pure ISO
C++17, header-only, in namespace `ca`. A faithful port of the Rust `argon2id`
crate.

Argon2 (the Password Hashing Competition winner) fills a large memory matrix
(`memory_cost` KiB) with a BLAKE2b-derived compression function and reads it back
so that an attacker cannot trade memory for speed. The *id* variant combines
Argon2i and Argon2d: the first two slices of the first pass use data-**independent**
addressing (an address stream), and everything after uses data-**dependent**
addressing (the previous block) — the **recommended** variant for password
hashing (RFC 9106 §4).

```
H0        = BLAKE2b(params || pass || salt || key || ad)
B[i][0/1] = H'(H0 || 0/1 || i)
B[i][j]   = G(B[i][j-1], B[l'][z'])       (XOR into place after pass 0)
tag       = H'(XOR of the last column across lanes)
```

Built on the sibling header-only [`blake2b`](../blake2b) package.

## API

Functions take and return `std::vector<std::uint8_t>`; invalid parameters throw
`std::invalid_argument`.

```cpp
#include "argon2id.hpp"

ca::Argon2idOptions opts;               // optional: key, associated_data, version
opts.key = {...};
auto tag = ca::argon2id(password, salt, /*time*/ 3, /*mem KiB*/ 32,
                       /*parallelism*/ 4, /*tag_length*/ 32, opts);
auto hex = ca::argon2id_hex(password, salt, 3, 32, 4, 32);
```

`memory_cost` (KiB) must be `>= 8*parallelism`; `parallelism` in `[1, 2^24-1]`;
`tag_length >= 4`; `time_cost >= 1`. `Argon2idOptions::version` defaults to 0x13.

## Portability

Pure ISO C++17 — compiles clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../../c/iso-harness). Standard library only.

## Development

```bash
# Compile and run the RFC 9106 §5.3 vector test under every C++ compiler.
sh BUILD
```
