# go/scrypt

A Go implementation of the **scrypt** password-based key derivation function
(RFC 7914, Colin Percival, 2009).

---

## What Is scrypt?

scrypt is a deliberately **memory-hard** KDF.  Where PBKDF2 only requires a
few kilobytes of state per iteration (making it easy to parallelise on GPUs),
scrypt builds a large random-access table (V) that must be held in memory
throughout the computation.  Cutting memory forces more I/O, making GPU/ASIC
attacks disproportionately expensive.

**Used in:**  Litecoin and other cryptocurrencies, the tarsnap backup tool,
various password managers and OS credential stores.

---

## Algorithm Overview

```
DK = scrypt(Password, Salt, N, r, p, dkLen)

1. B = PBKDF2-HMAC-SHA256(Password, Salt, c=1, p×128r bytes)
2. For each of p lanes in B:
       B[i] = ROMix(B[i], N, r)
3. DK = PBKDF2-HMAC-SHA256(Password, B, c=1, dkLen bytes)
```

### ROMix (the memory-hard core)

```
Phase 1 — Fill:
    V[0] = X
    V[i] = BlockMix(V[i-1])   for i = 1..N-1
    X    = BlockMix(V[N-1])

Phase 2 — Mix:
    for i = 0..N-1:
        j = Integerify(X) mod N
        X = BlockMix(X XOR V[j])

return X
```

Memory cost: N × 128 × r bytes per lane.

---

## Parameters

| Parameter | Description                             | Typical Value  |
|-----------|-----------------------------------------|----------------|
| `N`       | CPU/memory cost (power of 2, ≥ 2)       | 32768 or 16384 |
| `r`       | Block size multiplier (≥ 1)             | 8              |
| `p`       | Parallelisation factor (≥ 1)            | 1              |
| `dkLen`   | Derived key length in bytes (1 to 2²⁰)  | 32 or 64       |

Memory usage: `N × 128 × r` bytes.  For N=32768, r=8: **32 MiB**.

---

## Usage

```go
import scryptpkg "github.com/adhithyan15/coding-adventures/code/packages/go/scrypt"

// Derive a 32-byte key for a new user password.
salt := []byte("random-16-bytes!") // store this alongside the hash
dk, err := scryptpkg.Scrypt([]byte("hunter2"), salt, 32768, 8, 1, 32)
if err != nil {
    log.Fatal(err)
}

// Or get the result as a hex string:
hex, err := scryptpkg.ScryptHex([]byte("hunter2"), salt, 32768, 8, 1, 32)
```

---

## RFC 7914 Test Vectors

Verified against Python `hashlib.scrypt`, `golang.org/x/crypto/scrypt`, and OpenSSL.

```
scrypt("", "", 16, 1, 1, 64)
→ 77d6576238657b203b19ca42c18a0497f16b4844e3074ae8dfdffa3fede21442
  fcd0069ded0948f8326a753a0fc81f17e8d3e0fb2e0d3628cf35e20c38d18906

scrypt("password", "NaCl", 1024, 8, 16, 64)
→ fdbabe1c9d3472007856e7190d01e9fe7c6ad7cbc8237830e77376634b373162
  2eaf30d92e22a3886ff109279d9830dac727afb94a83ee6d8360cbdfa2cc0640
```

---

## Error Reference

| Error                 | Cause                                       |
|-----------------------|---------------------------------------------|
| `ErrInvalidN`         | N is not a power of 2, or N < 2             |
| `ErrNTooLarge`        | N > 2²⁰                                     |
| `ErrInvalidR`         | r < 1                                       |
| `ErrInvalidP`         | p < 1                                       |
| `ErrInvalidKeyLength` | dkLen < 1                                   |
| `ErrKeyLengthTooLarge`| dkLen > 2²⁰                                 |
| `ErrPRTooLarge`       | p × r > 2³⁰                                 |

---

## Stack Position

```
scrypt
  └── hmac (HMAC-SHA256, via internal PBKDF2)
        └── sha256
```

The internal `pbkdf2Sha256` calls `hmacpkg.HMAC` directly (bypassing the
empty-key restriction in `hmacpkg.HmacSHA256`) to support RFC 7914 vector 1's
empty password.

---

## Running Tests

```
go test ./... -v -cover
```

---

## Security Guidance

- Always use a **random, unique salt** (16–32 bytes from `crypto/rand`).
- Choose N so that derivation takes **≥ 100 ms** on your slowest target hardware.
- For interactive login at r=8, p=1: **N=32768** (32 MiB, ~100 ms on modern CPUs).
- For offline encryption or key wrapping: **N=1048576** (1 GiB) for much stronger resistance.
- Never store the password; store salt + derived key only.
