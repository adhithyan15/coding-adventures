# HKDF — HMAC-based Extract-and-Expand Key Derivation Function

## Overview

HKDF (RFC 5869) is a key derivation function based on HMAC. It was
designed by Hugo Kravczyk in 2010 and adopted into TLS 1.3, Signal
Protocol, WireGuard, and Noise Framework. Unlike password-based KDFs
(PBKDF2, scrypt, Argon2), HKDF is not designed to be slow — it's
designed to be *correct*. Its job is to take key material that is
"good enough" (random but possibly non-uniform) and produce
cryptographically strong derived keys.

### Why Two Steps?

Real-world key material comes from messy sources: Diffie-Hellman
shared secrets, concatenated passwords and salts, hardware RNG output.
This material may be non-uniform (biased bits) or longer/shorter than
needed. HKDF addresses this with a clean two-phase design:

1. **Extract:** Compress arbitrary-length, non-uniform input into a
   fixed-length pseudorandom key (PRK). This "distills" the entropy.
2. **Expand:** Stretch the PRK into as many output bytes as needed,
   bound by the hash output length × 255.

The separation is not just cosmetic. It lets you extract once and
expand multiple times with different `info` strings — deriving
independent keys for encryption, authentication, and IVs from a
single shared secret.

### When to Use HKDF vs Other KDFs

- **HKDF:** Deriving keys from a Diffie-Hellman exchange, a master
  secret, or any high-entropy source. Fast, deterministic.
- **PBKDF2/scrypt/Argon2:** Deriving keys from passwords (low-entropy).
  Must be deliberately slow to resist brute force.

## Algorithm

### Extract

```
HKDF-Extract(salt, IKM) -> PRK

PRK = HMAC-Hash(salt, IKM)
```

- `salt`: Optional (can be zero-length or absent, defaulting to a
  string of `HashLen` zero bytes). Acts as a domain separator.
- `IKM`: Input Keying Material — the raw secret bits.
- `PRK`: Pseudorandom Key — exactly `HashLen` bytes.

The salt is used as the HMAC *key*, and the IKM as the HMAC *message*.
This is intentional: HMAC's key is the more "trusted" input, and the
salt serves as a public randomizer that strengthens extraction even
when IKM has structure.

### Expand

```
HKDF-Expand(PRK, info, L) -> OKM

N = ceil(L / HashLen)
T(0) = empty string
T(1) = HMAC-Hash(PRK, T(0) || info || 0x01)
T(2) = HMAC-Hash(PRK, T(1) || info || 0x02)
...
T(N) = HMAC-Hash(PRK, T(N-1) || info || N)
OKM = first L bytes of T(1) || T(2) || ... || T(N)
```

- `PRK`: The extracted key (must be at least `HashLen` bytes).
- `info`: Context and application-specific information (can be empty).
  This is what differentiates derived keys — e.g., "tls13 key" vs
  "tls13 iv".
- `L`: Desired output length in bytes. Must be <= 255 × `HashLen`.
- The counter byte `i` is a single octet (1-indexed, max 255).

### Combined

```
HKDF(salt, IKM, info, L) -> OKM

PRK = HKDF-Extract(salt, IKM)
OKM = HKDF-Expand(PRK, info, L)
```

### Visual Flow

```
Input Keying Material (IKM)
         │
         ▼
┌──────────────────┐
│  HMAC-Hash(salt,  │  ◄── salt (public randomizer)
│       IKM)        │
└────────┬─────────┘
         │
    PRK (HashLen bytes)
         │
    ┌────┴─────────────────────────────────┐
    ▼              ▼              ▼         │
 T(1)=HMAC     T(2)=HMAC     T(N)=HMAC    │
 (PRK,         (PRK,         (PRK,         │
  ""||info     T(1)||info    T(N-1)||info   │
  ||0x01)      ||0x02)       ||N)          │
    │              │              │         │
    └──────────────┴──────────────┘         │
         │                                  │
    first L bytes = OKM                     │
                                  info ─────┘
                              (context string)
```

## Interface Contract

| Function | Signature | Description |
|----------|-----------|-------------|
| `hkdf_extract` | `(salt, ikm, hash_func) -> prk` | Extract step: compress IKM into PRK. |
| `hkdf_expand` | `(prk, info, length, hash_func) -> okm` | Expand step: derive `length` bytes from PRK. |
| `hkdf` | `(salt, ikm, info, length, hash_func) -> okm` | Combined extract-and-expand. |

Constraints:
- `length` must be <= 255 × `HashLen` (e.g., 8160 bytes for SHA-256).
- If `salt` is not provided, it defaults to a string of `HashLen`
  zero bytes.
- `info` can be empty (zero-length).
- `hash_func` selects the underlying hash (SHA-256, SHA-1, etc.).

## Test Vectors (RFC 5869, Appendix A)

### Test Case 1: SHA-256, Basic

```
Hash:  SHA-256
IKM:   0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b (22 bytes)
salt:  000102030405060708090a0b0c (13 bytes)
info:  f0f1f2f3f4f5f6f7f8f9 (10 bytes)
L:     42

PRK:   077709362c2e32df0ddc3f0dc47bba63
       90b6c73bb50f9c3122ec844ad7c2b3e5

OKM:   3cb25f25faacd57a90434f64d0362f2a
       2d2d0a90cf1a5a4c5db02d56ecc4c5bf
       34007208d5b887185865
```

### Test Case 2: SHA-256, Longer Inputs/Outputs

```
Hash:  SHA-256
IKM:   000102030405060708090a0b0c0d0e0f
       101112131415161718191a1b1c1d1e1f
       202122232425262728292a2b2c2d2e2f
       303132333435363738393a3b3c3d3e3f
       404142434445464748494a4b4c4d4e4f (80 bytes)
salt:  606162636465666768696a6b6c6d6e6f
       707172737475767778797a7b7c7d7e7f
       808182838485868788898a8b8c8d8e8f
       909192939495969798999a9b9c9d9e9f
       a0a1a2a3a4a5a6a7a8a9aaabacadaeaf (80 bytes)
info:  b0b1b2b3b4b5b6b7b8b9babbbcbdbebf
       c0c1c2c3c4c5c6c7c8c9cacbcccdcecf
       d0d1d2d3d4d5d6d7d8d9dadbdcdddedf
       e0e1e2e3e4e5e6e7e8e9eaebecedeeef
       f0f1f2f3f4f5f6f7f8f9fafbfcfdfeff (80 bytes)
L:     82

PRK:   06a6b88c5853361a06104c9ceb35b45c
       ef760014904671014a193f40c15fc244

OKM:   b11e398dc80327a1c8e7f78c596a4934
       4f012eda2d4efad8a050cc4c19afa97c
       59045a99cac7827271cb41c65e590e09
       da3275600c2f09b8367793a9aca3db71
       cc30c58179ec3e87c14c01d5c1f3434f
       1d87
```

### Test Case 3: SHA-256, Zero-Length Salt and Info

```
Hash:  SHA-256
IKM:   0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b (22 bytes)
salt:  (empty)
info:  (empty)
L:     42

PRK:   19ef24a32c717b167f33a91d6f648bdf
       96596776afdb6377ac434c1c293ccb04

OKM:   8da4e775a563c18f715f802a063c5a31
       b8a11f5c5ee1879ec3454e5f3c738d2d
       9d201395faa4b61a96c8
```

## Package Matrix

Same 9 languages, in `hkdf/` directories.

**Dependencies:** HF05 (HMAC), which transitively depends on a hash
function (HF03 SHA-256 or HF04 SHA-512 depending on the chosen hash).
