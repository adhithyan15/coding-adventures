# scrypt (C++)

**scrypt** — the sequential memory-hard password-based key derivation function
(RFC 7914) — in pure ISO C++17, header-only, in namespace `ca`. A faithful port
of the Rust `scrypt` crate.

PBKDF2 and bcrypt can be parallelised cheaply on GPUs / FPGAs. scrypt adds
*memory hardness*: it deliberately allocates a large random-access working set
(`N * 128 * r` bytes) and reads it in a data-dependent order, so an attacker
cannot trade memory for speed.

```
scrypt(P, S, N, r, p, dkLen):
  1. B    = PBKDF2-HMAC-SHA256(P, S, 1, p*128*r)   -- expand into p blocks
  2. B[i] = ROMix(B[i], N)   for each 128*r block  -- the memory-hard step
  3. DK   = PBKDF2-HMAC-SHA256(P, B, 1, dkLen)      -- extract the key
```

ROMix fills a table `V` of `N` snapshots (BlockMix run `N` times), then does `N`
more BlockMix steps each XORing in a data-chosen `V` entry. BlockMix mixes `2r`
64-byte blocks with the **Salsa20/8** core. Built on the sibling header-only
[`pbkdf2`](../pbkdf2) package.

## Parameters

`n` (CPU/memory cost, a power of two in [2, 2^20]), `r` (block-size multiplier
≥ 1), `p` (parallelisation ≥ 1), `dk_len` (output length ≤ 2^20). Memory usage:
`N * 128 * r` bytes (N=16384, r=8 → 16 MiB).

## API

Functions take and return `std::vector<std::uint8_t>`; invalid parameters throw
`std::invalid_argument`.

```cpp
#include "scrypt.hpp"

std::vector<std::uint8_t> pw{'p','w'}, salt{'N','a','C','l'};

auto dk  = ca::scrypt(pw, salt, 16384, 8, 1, 64);        // 64-byte key
auto hex = ca::scrypt_hex(pw, salt, 16384, 8, 1, 64);    // lowercase hex string
```

An empty password/salt is permitted (RFC 7914 vector 1). `N` above
`ca::scrypt_max_n` (2^20), `dk_len` above `ca::scrypt_max_dk_len` (2^20), a
non-power-of-two `N`, or `p*r`/`p*128*r` exceeding 2^30 all throw.

## Portability

Pure ISO C++17 — compiles clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../../c/iso-harness). Standard library only.

## Development

```bash
# Compile and run the RFC 7914 vector tests under every C++ compiler.
sh BUILD
```
