# coding-adventures-pbkdf2

PBKDF2 (Password-Based Key Derivation Function 2) — RFC 8018 — implemented from scratch.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) educational computing stack.

## What It Does

Derives a fixed-length cryptographic key from a password by applying HMAC thousands of times. The iteration count makes brute-force attacks expensive — every guess requires the same number of hash operations as the original derivation.

## Algorithm

```
DK = T_1 || T_2 || ... (first dkLen bytes)

T_i = U_1 XOR U_2 XOR ... XOR U_c

U_1 = HMAC(Password, Salt || INT_32_BE(i))
U_j = HMAC(Password, U_{j-1})
```

## Usage

```python
from coding_adventures_pbkdf2 import pbkdf2_hmac_sha256

# Derive a 32-byte key from a password
dk = pbkdf2_hmac_sha256(
    password=b"correct horse battery staple",
    salt=b"\xde\xad\xbe\xef" * 4,   # 16 random bytes per user
    iterations=600_000,              # OWASP 2023 minimum for SHA-256
    key_length=32,
)
print(dk.hex())
```

## Supported PRFs

| Function                 | PRF        | Output (bytes) |
|--------------------------|------------|----------------|
| `pbkdf2_hmac_sha1`       | HMAC-SHA1  | variable       |
| `pbkdf2_hmac_sha256`     | HMAC-SHA256| variable       |
| `pbkdf2_hmac_sha512`     | HMAC-SHA512| variable       |
| `*_hex` variants         | same       | hex string     |

## Security Notes

- Minimum iterations: 600,000 for HMAC-SHA256 (OWASP 2023).
- Salt must be random, unique per credential, at least 16 bytes.
- For new systems consider Argon2id (memory-hard, resists GPU attacks).

## Dependencies

- `coding-adventures-hmac` (KD01 depends on HF05)

## Stack Position

KD01 in the cryptography layer. Builds on HF05 (HMAC), which builds on HF03/HF04 (SHA-256/SHA-512).
