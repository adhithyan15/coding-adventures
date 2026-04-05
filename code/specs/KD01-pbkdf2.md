# KD01 — PBKDF2 (Password-Based Key Derivation Function 2)

## Overview

PBKDF2 (RFC 8018, originally RFC 2898) derives a cryptographic key from
a password by applying a pseudorandom function (typically HMAC) many
times. The iteration count makes brute-force attacks computationally
expensive — each password guess requires thousands or millions of hash
operations.

PBKDF2 is used in:
- WPA2 WiFi (4096 iterations of HMAC-SHA1)
- macOS keychain, iOS data protection
- Django default password hasher
- LUKS disk encryption
- Many PKCS#12 and PKCS#5 implementations

### Why Not Just hash(password + salt)?

A single hash is too fast — modern GPUs can compute billions of MD5
hashes per second. PBKDF2 with 100,000+ iterations makes each guess
take measurable time, turning a millisecond attack into hours or years.

## Algorithm

```
DK = PBKDF2(PRF, Password, Salt, c, dkLen)

For each block i = 1, 2, ..., ceil(dkLen / hLen):
  U_1 = PRF(Password, Salt || INT_32_BE(i))
  U_2 = PRF(Password, U_1)
  ...
  U_c = PRF(Password, U_{c-1})

  T_i = U_1 XOR U_2 XOR ... XOR U_c

DK = T_1 || T_2 || ... (truncated to dkLen bytes)
```

Where:
- PRF = pseudorandom function (HMAC-SHA256 recommended)
- c = iteration count (minimum 100,000 for HMAC-SHA256 per OWASP 2023)
- dkLen = desired key length in bytes
- hLen = output length of PRF

## Interface Contract

| Function | Signature | Description |
|----------|-----------|-------------|
| `pbkdf2_hmac_sha256` | `(password, salt, iterations, key_length) -> bytes` | PBKDF2 with HMAC-SHA256. |
| `pbkdf2_hmac_sha1` | `(password, salt, iterations, key_length) -> bytes` | PBKDF2 with HMAC-SHA1 (legacy, WPA2). |
| `pbkdf2` | `(prf, password, salt, iterations, key_length) -> bytes` | Generic with any PRF. |
| `*_hex` variants | Same + `-> string` | Hex-encoded versions. |

## Test Vectors (RFC 6070)

```
# PBKDF2-HMAC-SHA1
Password: "password"
Salt:     "salt"
c:        1
dkLen:    20
DK:       0c60c80f961f0e71f3a9b524af6012062fe037a6

Password: "password"
Salt:     "salt"
c:        4096
dkLen:    20
DK:       4b007901b765489abead49d926f721d065a429c1

# PBKDF2-HMAC-SHA256 (from RFC 7914 Appendix B)
Password: "passwd"
Salt:     "salt"
c:        1
dkLen:    64
DK:       55ac046e56e3089fec1691c22544b605f94185216dde0465e68b9d57c20dacbc
          49ca9cccf179b645991664b39d77ef317c71b845b1e30bd509112041d3a19783
```

## Security Notes

- **Minimum iterations:** 600,000 for HMAC-SHA256, 1,300,000 for
  HMAC-SHA1 (OWASP 2023 recommendations)
- **Salt:** Must be random, unique per user, at least 16 bytes
- **Comparison:** For new systems, prefer Argon2id over PBKDF2 (memory-hard,
  resists GPU/ASIC attacks). PBKDF2 is still acceptable when Argon2 is
  unavailable.

## Package Matrix

Same 9 languages, in `pbkdf2/` directories.

**Dependencies:** HF05 (HMAC), which depends on HF01-HF04 (hash functions).
