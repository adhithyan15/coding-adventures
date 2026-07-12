# argon2i (C++)

**Argon2i** — data-independent memory-hard password hashing (RFC 9106) — in pure
ISO C++17, header-only, in namespace `ca`. A faithful port of the Rust `argon2i`
crate.

Argon2 (the Password Hashing Competition winner) fills a large memory matrix
(`memory_cost` KiB) with a BLAKE2b-derived compression function and reads it back
so that an attacker cannot trade memory for speed. The *i* variant picks each
reference block from a deterministic pseudo-random stream that does **not** depend
on the password or memory contents — a constant memory-access pattern that defeats
side-channel observers, at the cost of being the easiest variant to parallelise.
**Prefer Argon2id for password hashing.**

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
#include "argon2i.hpp"

ca::Argon2iOptions opts;               // optional: key, associated_data, version
opts.key = {...};
auto tag = ca::argon2i(password, salt, /*time*/ 3, /*mem KiB*/ 32,
                       /*parallelism*/ 4, /*tag_length*/ 32, opts);
auto hex = ca::argon2i_hex(password, salt, 3, 32, 4, 32);
```

`memory_cost` (KiB) must be `>= 8*parallelism`; `parallelism` in `[1, 2^24-1]`;
`tag_length >= 4`; `time_cost >= 1`. `Argon2iOptions::version` defaults to 0x13.

## Portability

Pure ISO C++17 — compiles clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../../c/iso-harness). Standard library only.

## Development

```bash
# Compile and run the RFC 9106 §5.2 vector test under every C++ compiler.
sh BUILD
```
