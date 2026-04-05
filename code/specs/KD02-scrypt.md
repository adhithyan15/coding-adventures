# KD02 — scrypt

## Overview

scrypt (RFC 7914) is a memory-hard key derivation function designed by
Colin Percival in 2009. Unlike PBKDF2 which is purely CPU-intensive,
scrypt requires large amounts of memory, making it resistant to GPU and
ASIC attacks where memory is the bottleneck.

Used in: Litecoin, Dogecoin, Tarsnap backup, various password hashing
schemes.

### Why Memory-Hardness Matters

GPUs have thousands of cores but limited per-core memory. A function
that requires 128 MB of RAM per instance can only run a few instances
in parallel on a GPU, dramatically reducing the attacker's advantage
over defenders using CPUs.

## Algorithm

scrypt builds on three primitives:

### 1. Salsa20/8 Core
A reduced-round (8 instead of 20) Salsa20 stream cipher applied as a
mixing function. Takes 64 bytes, returns 64 bytes.

### 2. BlockMix(B, r)
Applies Salsa20/8 to 2r × 64-byte blocks in a specific pattern.

### 3. ROMix(B, N)
The memory-hard core:
```
1. V[0] = B
2. For i = 1 to N-1: V[i] = BlockMix(V[i-1])  // Fill memory
3. X = V[N-1]
4. For i = 0 to N-1:                             // Random lookups
   j = Integerify(X) mod N
   X = BlockMix(X XOR V[j])                     // Memory-dependent
5. Return X
```

### Full scrypt
```
scrypt(Password, Salt, N, r, p, dkLen):
1. B = PBKDF2-HMAC-SHA256(Password, Salt, 1, p × 128 × r)
2. For i = 0 to p-1:
   B_i = ROMix(B_i, N)
3. DK = PBKDF2-HMAC-SHA256(Password, B, 1, dkLen)
```

Parameters: N = CPU/memory cost, r = block size, p = parallelism.

## Interface Contract

| Function | Signature | Description |
|----------|-----------|-------------|
| `scrypt` | `(password, salt, N, r, p, dk_len) -> bytes` | Full scrypt KDF. |
| `scrypt_hex` | Same + `-> string` | Hex variant. |

Typical parameters: N=2^14 (interactive), N=2^20 (sensitive), r=8, p=1.

## Test Vectors (RFC 7914 Section 12)

```
Password: ""
Salt:     ""
N=16, r=1, p=1, dkLen=64
DK: 77d6576238657b203b19ca42c18a0497f9b3fc3d0d863d2c4dfcbff56b8b6e4c
    0e3e7e6cf64cf7d6d58a28b8faecbc77c7c0d68eb6c36fc4c83c6c24e1ac1d24

Password: "password"
Salt:     "NaCl"
N=1024, r=8, p=16, dkLen=64
DK: fdbabe1c9d3472007856e7190d01e72cc0044298eb1c1127886d3a3f48f2f7b0
    d9a34e8c2ded1f83e8eb37e9677f83a38ae1d0a8da30dd1d4bc4ca26a96e38db
```

## Package Matrix

Same 9 languages, in `scrypt/` directories.

**Dependencies:** KD01 (PBKDF2), HF05 (HMAC), HF03 (SHA-256).
Also depends on a Salsa20 core (can be inlined or separate package).
