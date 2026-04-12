# swift/scrypt

A pure-Swift implementation of the **scrypt** password-based key derivation
function (RFC 7914), built from first principles as part of the
coding-adventures educational computing stack.

## What Is scrypt?

scrypt was designed by Colin Percival (2009) to resist hardware brute-force
attacks. Unlike PBKDF2, scrypt is **memory-hard**: an attacker cannot
accelerate it by buying more CPUs. They must also buy proportionally more
memory, which remains physically expensive even as compute gets cheap.

scrypt is used in:
- **Litecoin, Dogecoin** — Proof-of-Work hashing
- **macOS FileVault 2** — disk encryption key derivation
- **1Password, Tarsnap, OpenBSD bioctl** — password storage
- **libsodium** — the most widely-used crypto library

## Stack Position

```
scrypt (RFC 7914)
  └── PBKDF2-HMAC-SHA256 (internal, no empty-key guard)
        └── HMAC (RFC 2104)  +  SHA-256 (FIPS 180-4)
              └── ... (md5, sha1, sha256, sha512 primitives)
```

scrypt sits at the top of the cryptographic key-derivation hierarchy in this
stack. It depends on `../hmac` (for the generic HMAC function) and `../sha256`
(for the SHA-256 hash function), but does NOT use the `../pbkdf2` package
directly — this is intentional: the PBKDF2 package rejects empty passwords as
a security policy, but RFC 7914 vector 1 uses an empty password.

## Usage

```swift
import Scrypt

// Derive a 32-byte key
let dk = try scrypt(
    password: Array("my passphrase".utf8),
    salt:     Array("unique random salt".utf8),
    n:        16384,   // 2^14 — interactive login cost (~16 MB, ~0.1s)
    r:        8,       // block size factor
    p:        1,       // parallelization (sequential here)
    dkLen:    32       // 32-byte output
)

// Hex convenience
let hex = try scryptHex(
    password: Array("my passphrase".utf8),
    salt:     Array("unique random salt".utf8),
    n: 16384, r: 8, p: 1, dkLen: 32
)
print(hex)  // → 64 hex characters
```

## Parameters

| Parameter | Meaning | Typical Value |
|-----------|---------|---------------|
| `n` | CPU/memory cost. Power of 2 (≥2, ≤2^20). Memory = N×128×r bytes. | 16384 (interactive), 1048576 (file encryption) |
| `r` | Block size factor. Each scrypt block is 128×r bytes. | 8 |
| `p` | Parallelization. Independent parallel ROMix instances. | 1 |
| `dkLen` | Output key length in bytes (≥1, ≤2^20). | 32 or 64 |

## Running Tests

```sh
swift test --enable-code-coverage --verbose
```

RFC 7914 §11 test vectors 1 and 2 are verified. Vector 3 (N=16384) is commented
out in the test file — enable it locally for full RFC compliance verification.

## Algorithm at a Glance

```
scrypt(P, S, N, r, p, dkLen):
  1. B = PBKDF2-HMAC-SHA256(P, S, c=1, bLen=p×128×r)
  2. For i = 0..p-1: B[i] = ROMix(r, B[i], N)
  3. DK = PBKDF2-HMAC-SHA256(P, B, c=1, dkLen)

ROMix(r, B, N):
  X = B
  For i = 0..N-1: V[i] = X; X = BlockMix(X)   ← build N-entry table
  For i = 0..N-1: j = Integerify(X) mod N       ← pseudo-random lookups
                  X = BlockMix(X XOR V[j])
  Return X

BlockMix(X):
  x = X[2r-1]                         ← start with last block
  For i = 0..2r-1: x = Salsa20/8(x XOR X[i]); y[i] = x
  Return [y_0, y_2, ..., y_{2r-2}, y_1, y_3, ..., y_{2r-1}]  ← interleaved
```

## Security Notes

- **Empty passwords** are supported (required by RFC 7914 §11 vector 1). In
  production systems you should reject empty passwords at the application layer.
- **N must be a power of 2** — the ROMix table lookup uses `j = integerify(X) mod N`,
  and a power of 2 allows a fast bitwise AND instead of division.
- **Memory usage** is O(N×r): for N=16384, r=8, that is 16 MB per thread.
  For N=1048576 (2^20) it is 1 GB — ensure your system has enough RAM.
- Do not use N < 2^14 for password storage in production (per OWASP 2023).
