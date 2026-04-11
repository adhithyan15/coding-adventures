"""pbkdf2 -- PBKDF2 (Password-Based Key Derivation Function 2) — RFC 8018.

What Is PBKDF2?
===============
PBKDF2 derives a cryptographic key from a password by applying a pseudorandom
function (typically HMAC) many thousands of times. The iteration count makes
brute-force attacks computationally expensive — an attacker must run the same
number of iterations for every password guess.

PBKDF2 is used in:
- WPA2 Wi-Fi (4096 iterations of HMAC-SHA1)
- macOS keychain, iOS data protection
- Django's default password hasher (PBKDF2-SHA256)
- LUKS disk encryption
- PKCS#12 / PKCS#5 key wrapping

Why Not Just hash(password + salt)?
====================================
A single hash is too fast — modern GPUs compute billions of SHA-256 hashes per
second. PBKDF2 with 100,000 iterations forces each guess to spend real CPU time,
turning a millisecond attack into hours or years.

The PBKDF2 Construction (RFC 8018 § 5.2)
==========================================

  DK = PBKDF2(PRF, Password, Salt, c, dkLen)

  For each block index i = 1, 2, ..., ceil(dkLen / hLen):
    U_1 = PRF(Password, Salt || INT_32_BE(i))   ← first HMAC call
    U_2 = PRF(Password, U_1)                    ← second HMAC call
    ...
    U_c = PRF(Password, U_{c-1})                ← c-th HMAC call

    T_i = U_1 XOR U_2 XOR ... XOR U_c          ← XOR all c outputs together

  DK = T_1 || T_2 || ... (first dkLen bytes)

Diagram — one block:

  Password ──┬──────────────────────────────────┐
             │                                  │
  Salt||i ──►│ HMAC → U_1                       │
             │          │                        │
             │          └──────────────────┐     │
             │                            ▼     ▼
             │                     HMAC → U_2   XOR
             │                              │    │
             │            (repeat c times)  └──►XOR──► T_i
             │
  (same Password for all HMAC calls in this block)

Choosing Iteration Count
=========================
OWASP 2023 recommendations (these rise as CPUs get faster):
  HMAC-SHA256:  600,000 iterations minimum
  HMAC-SHA1:  1,300,000 iterations minimum

Block Index Encoding — INT_32_BE
==================================
The block number is appended to the salt as a 4-byte big-endian integer:

  block 1 → b'\\x00\\x00\\x00\\x01'
  block 2 → b'\\x00\\x00\\x00\\x02'

This makes each block's first U value unique even if the salt repeats.

RFC 6070 Test Vector (PBKDF2-HMAC-SHA1)
=========================================
  Password:   "password"
  Salt:       "salt"
  Iterations: 1
  dkLen:      20
  DK:         0c60c80f961f0e71f3a9b524af6012062fe037a6
"""

from __future__ import annotations

import struct
from collections.abc import Callable

from coding_adventures_hmac import (
    hmac_sha1,
    hmac_sha256,
    hmac_sha512,
)

# A PRF takes (key: bytes, message: bytes) and returns bytes.
# In PBKDF2 the password is the HMAC key and the iterated data is the message.
_PRF = Callable[[bytes, bytes], bytes]


def _pbkdf2(
    prf: _PRF,
    h_len: int,
    password: bytes,
    salt: bytes,
    iterations: int,
    key_length: int,
) -> bytes:
    """Core PBKDF2 loop — not exported; call the concrete variants below.

    Parameters
    ----------
    prf:        Pseudorandom function: (key, msg) → bytes of length h_len.
    h_len:      Output byte length of prf (20 for SHA-1, 32 for SHA-256, …).
    password:   The secret being stretched — becomes the HMAC key.
    salt:       Random value unique per credential (≥16 bytes recommended).
    iterations: Number of PRF calls per block (higher = slower = harder to
                brute-force).
    key_length: Number of bytes to produce.
    """
    if not password:
        raise ValueError("PBKDF2 password must not be empty")
    if iterations <= 0:
        raise ValueError(f"PBKDF2 iterations must be positive, got {iterations}")
    if key_length <= 0:
        raise ValueError(f"PBKDF2 key_length must be positive, got {key_length}")

    import math

    num_blocks = math.ceil(key_length / h_len)
    dk = bytearray()

    for i in range(1, num_blocks + 1):
        # First call seeds U_1 from the salt and the big-endian block index.
        # struct.pack(">I", i) encodes i as a 4-byte big-endian unsigned int.
        u = prf(password, salt + struct.pack(">I", i))

        # t accumulates the XOR of all U values for this block.
        t = bytearray(u)

        for _ in range(iterations - 1):
            u = prf(password, u)
            # XOR each byte of the new U into the accumulator.
            for k in range(h_len):
                t[k] ^= u[k]

        dk.extend(t)

    return bytes(dk[:key_length])


# ──────────────────────────────────────────────────────────────────────────────
# Public API — concrete PRF variants
# ──────────────────────────────────────────────────────────────────────────────

def pbkdf2_hmac_sha1(
    password: bytes,
    salt: bytes,
    iterations: int,
    key_length: int,
) -> bytes:
    """PBKDF2 with HMAC-SHA1.

    HMAC-SHA1 is used in WPA2 (4096 iterations) and older PKCS#12 files.
    For new systems, prefer PBKDF2-HMAC-SHA256 or Argon2id.

    Digest size h_len = 20 bytes (160-bit SHA-1 output).

    Example — RFC 6070 test vector:
    >>> pbkdf2_hmac_sha1(b"password", b"salt", 1, 20).hex()
    '0c60c80f961f0e71f3a9b524af6012062fe037a6'
    """
    return _pbkdf2(hmac_sha1, 20, password, salt, iterations, key_length)


def pbkdf2_hmac_sha256(
    password: bytes,
    salt: bytes,
    iterations: int,
    key_length: int,
) -> bytes:
    """PBKDF2 with HMAC-SHA256.

    The recommended choice for new systems as of OWASP 2023.
    Use at least 600,000 iterations.

    Digest size h_len = 32 bytes (256-bit SHA-256 output).

    Example — RFC 7914 Appendix B:
    >>> dk = pbkdf2_hmac_sha256(b"passwd", b"salt", 1, 64)
    >>> dk.hex()[:32]
    '55ac046e56e3089fec1691c22544b605'
    """
    return _pbkdf2(hmac_sha256, 32, password, salt, iterations, key_length)


def pbkdf2_hmac_sha512(
    password: bytes,
    salt: bytes,
    iterations: int,
    key_length: int,
) -> bytes:
    """PBKDF2 with HMAC-SHA512.

    Suitable for high-security applications where 512-bit output is needed.
    Digest size h_len = 64 bytes (512-bit SHA-512 output).
    """
    return _pbkdf2(hmac_sha512, 64, password, salt, iterations, key_length)


# ──────────────────────────────────────────────────────────────────────────────
# Hex-string convenience variants
# ──────────────────────────────────────────────────────────────────────────────

def pbkdf2_hmac_sha1_hex(
    password: bytes, salt: bytes, iterations: int, key_length: int
) -> str:
    """Like pbkdf2_hmac_sha1 but returns a lowercase hex string."""
    return pbkdf2_hmac_sha1(password, salt, iterations, key_length).hex()


def pbkdf2_hmac_sha256_hex(
    password: bytes, salt: bytes, iterations: int, key_length: int
) -> str:
    """Like pbkdf2_hmac_sha256 but returns a lowercase hex string."""
    return pbkdf2_hmac_sha256(password, salt, iterations, key_length).hex()


def pbkdf2_hmac_sha512_hex(
    password: bytes, salt: bytes, iterations: int, key_length: int
) -> str:
    """Like pbkdf2_hmac_sha512 but returns a lowercase hex string."""
    return pbkdf2_hmac_sha512(password, salt, iterations, key_length).hex()
