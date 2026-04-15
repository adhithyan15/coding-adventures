"""hkdf -- HKDF (HMAC-based Extract-and-Expand Key Derivation Function) -- RFC 5869.

What Is HKDF?
=============
HKDF derives one or more cryptographically strong keys from a single piece of
input keying material (IKM). It is the standard key derivation function used in
TLS 1.3, Signal Protocol, WireGuard, and many other modern protocols.

HKDF was designed by Hugo Krawczyk (the same person behind HMAC) and published
as RFC 5869 in 2010. It is intentionally simple: just two calls to HMAC.

Why Do We Need Key Derivation?
==============================
Raw key material -- whether from a Diffie-Hellman exchange, a password, or a
random source -- is not always suitable for direct use as a cryptographic key.
It might have:
  - Non-uniform distribution (some bits more predictable than others)
  - Wrong length (a DH shared secret is 32 bytes, but you need a 16-byte
    AES key AND a 32-byte HMAC key AND a 12-byte IV)
  - Insufficient entropy concentration

HKDF solves all three problems through a two-phase approach:

Phase 1: Extract
================
Extract takes the raw IKM and "concentrates" its entropy into a fixed-length
pseudorandom key (PRK). Even if the IKM has a non-uniform distribution, the
PRK will be indistinguishable from random (assuming the IKM has sufficient
min-entropy).

    PRK = HMAC-Hash(salt, IKM)

    +------+     +------+
    | salt |---->|      |
    +------+     | HMAC |----> PRK (HashLen bytes)
    | IKM  |---->|      |
    +------+     +------+

The salt is optional. If not provided, HKDF uses a string of HashLen zero
bytes. A good salt acts as a "domain separator" -- even if two applications
use the same IKM, different salts produce completely different PRKs.

Note that the salt is the HMAC *key* and the IKM is the HMAC *message*.
This is not a typo -- RFC 5869 Section 2.2 is explicit about this ordering.
The reasoning: HMAC's security as a PRF (pseudorandom function) holds even
when the key (salt) is public and the message (IKM) has high entropy.

Phase 2: Expand
===============
Expand takes the PRK and produces as many output bytes as needed. It works
by chaining HMAC calls with a counter byte:

    T(0) = empty string
    T(1) = HMAC-Hash(PRK, T(0) || info || 0x01)
    T(2) = HMAC-Hash(PRK, T(1) || info || 0x02)
    ...
    T(N) = HMAC-Hash(PRK, T(N-1) || info || N)

    OKM = first L bytes of T(1) || T(2) || ... || T(N)

    where N = ceil(L / HashLen)

The `info` parameter is a context string that binds the derived key to its
intended purpose. For example, TLS 1.3 uses info strings like
"tls13 derived" and "tls13 finished" to derive different keys from the
same PRK.

    PRK ----+
            |     +------+
            +---->|      |
    T(0)--------->| HMAC |----> T(1) = first HashLen bytes of OKM
    info--------->|      |
    0x01--------->|      |
                  +------+
            |     +------+
            +---->|      |
    T(1)--------->| HMAC |----> T(2) = next HashLen bytes of OKM
    info--------->|      |
    0x02--------->|      |
                  +------+
            ...

The counter byte is a single octet (0x01 through 0xFF), so the maximum
output length is 255 * HashLen bytes:
  - SHA-256: 255 * 32 = 8160 bytes
  - SHA-512: 255 * 64 = 16320 bytes

Combined: Extract-then-Expand
=============================
The most common usage combines both phases:

    OKM = HKDF(salt, IKM, info, L) = HKDF-Expand(HKDF-Extract(salt, IKM), info, L)

Supported Hash Functions
========================
  Algorithm   HashLen (bytes)   Max OKM (bytes)
  ---------   ---------------   ---------------
  SHA-256     32                8160
  SHA-512     64                16320
"""

from __future__ import annotations

import math

from coding_adventures_hmac import hmac, hmac_sha256, hmac_sha512
from coding_adventures_sha256 import sha256
from coding_adventures_sha512 import sha512

__version__ = "0.1.0"

# ─── Constants ────────────────────────────────────────────────────────────────

# Map from hash name to (hash_function, block_size, hash_length).
#
# block_size: the internal block size of the hash function, needed by the
#     generic HMAC implementation. SHA-256 uses 64-byte blocks; SHA-512
#     uses 128-byte blocks.
#
# hash_length: the output size of the hash function in bytes. This
#     determines the PRK length, the size of each T(i) block, and the
#     default salt when none is provided.
_HASH_PARAMS: dict[str, tuple[object, int, int]] = {
    "sha256": (sha256, 64, 32),
    "sha512": (sha512, 128, 64),
}


# ─── Extract ──────────────────────────────────────────────────────────────────


def hkdf_extract(
    salt: bytes,
    ikm: bytes,
    hash: str = "sha256",  # noqa: A002 — shadowing builtin is intentional for API clarity
) -> bytes:
    """HKDF-Extract: concentrate entropy from IKM into a pseudorandom key.

    Implements RFC 5869 Section 2.2:

        PRK = HMAC-Hash(salt, IKM)

    Parameters
    ----------
    salt:
        Optional salt value. If empty (``b""``), a string of HashLen zero
        bytes is used. A good salt improves security but is not required
        for basic correctness.
    ikm:
        Input keying material. Must contain sufficient entropy for the
        intended security level.
    hash:
        Hash algorithm name: ``"sha256"`` (default) or ``"sha512"``.

    Returns
    -------
    bytes
        The pseudorandom key (PRK), exactly HashLen bytes long.

    Examples
    --------
    >>> ikm = bytes.fromhex("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
    >>> salt = bytes.fromhex("000102030405060708090a0b0c")
    >>> hkdf_extract(salt, ikm).hex()
    '077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5'

    """
    hash_fn, block_size, hash_len = _get_hash_params(hash)

    # RFC 5869 Section 2.2: "if not provided, [salt] is set to a string
    # of HashLen zeros."
    #
    # Why HashLen zeros? Because HMAC normalizes keys shorter than the
    # block size by zero-padding to block_size anyway. Using HashLen zeros
    # as the explicit salt means the PRK is still well-defined and
    # deterministic even when the caller omits the salt entirely.
    if not salt:
        salt = b"\x00" * hash_len

    # Note the parameter order: salt is the HMAC *key*, IKM is the *message*.
    # This follows RFC 5869 exactly. The generic hmac() function accepts
    # (hash_fn, block_size, key, message).
    return hmac(hash_fn, block_size, salt, ikm)


# ─── Expand ───────────────────────────────────────────────────────────────────


def hkdf_expand(
    prk: bytes,
    info: bytes,
    length: int,
    hash: str = "sha256",  # noqa: A002
) -> bytes:
    """HKDF-Expand: derive output keying material from a pseudorandom key.

    Implements RFC 5869 Section 2.3:

        N = ceil(L / HashLen)
        T(0) = empty
        T(i) = HMAC-Hash(PRK, T(i-1) || info || i)   for i = 1..N
        OKM  = first L bytes of T(1) || T(2) || ... || T(N)

    Parameters
    ----------
    prk:
        Pseudorandom key, typically output of ``hkdf_extract``.
        Must be at least HashLen bytes for full security.
    info:
        Context and application-specific information. Can be empty.
        This binds the derived key to its intended purpose.
    length:
        Desired output length in bytes. Must satisfy
        ``1 <= length <= 255 * HashLen``.
    hash:
        Hash algorithm name: ``"sha256"`` (default) or ``"sha512"``.

    Returns
    -------
    bytes
        Output keying material (OKM), exactly ``length`` bytes.

    Raises
    ------
    ValueError
        If ``length`` is out of the valid range.

    Examples
    --------
    >>> prk = bytes.fromhex(
    ...     "077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5"
    ... )
    >>> info = bytes.fromhex("f0f1f2f3f4f5f6f7f8f9")
    >>> hkdf_expand(prk, info, 42).hex()
    '3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865'

    """
    hash_fn, block_size, hash_len = _get_hash_params(hash)

    # Validate output length.
    # The counter is a single byte (0x01..0xFF), so the maximum number of
    # HMAC blocks is 255, giving a maximum output of 255 * HashLen bytes.
    max_length = 255 * hash_len
    if length <= 0:
        msg = f"HKDF output length must be positive, got {length}"
        raise ValueError(msg)
    if length > max_length:
        msg = (
            f"HKDF output length {length} exceeds maximum "
            f"{max_length} (255 * {hash_len}) for {hash}"
        )
        raise ValueError(msg)

    # Number of HMAC blocks needed.
    n = math.ceil(length / hash_len)

    # Build the output keying material block by block.
    #
    # Each block T(i) is computed as:
    #     T(i) = HMAC-Hash(PRK, T(i-1) || info || i)
    #
    # where PRK is the HMAC key and the concatenation is the HMAC message.
    # T(0) is the empty string, so T(1) = HMAC-Hash(PRK, "" || info || 0x01).
    okm = bytearray()
    t_prev = b""  # T(0) = empty

    for i in range(1, n + 1):
        # Build the HMAC message: T(i-1) || info || counter_byte
        # The counter is a single byte, 1-indexed.
        message = t_prev + info + bytes([i])

        # Compute T(i) using the generic HMAC function.
        # We use the generic hmac() rather than hmac_sha256/hmac_sha512
        # because the PRK is always valid (non-empty), and the generic
        # function gives us hash-agnostic code.
        t_prev = hmac(hash_fn, block_size, prk, message)
        okm.extend(t_prev)

    # Truncate to exactly the requested length.
    # If length is not a multiple of HashLen, the last block contributes
    # only a partial set of bytes.
    return bytes(okm[:length])


# ─── Combined: Extract-then-Expand ───────────────────────────────────────────


def hkdf(
    salt: bytes,
    ikm: bytes,
    info: bytes,
    length: int,
    hash: str = "sha256",  # noqa: A002
) -> bytes:
    """HKDF: derive keying material from input keying material.

    This is the standard "extract-then-expand" usage from RFC 5869 Section 2.

        OKM = HKDF-Expand(HKDF-Extract(salt, IKM), info, L)

    Parameters
    ----------
    salt:
        Optional salt value. If empty, HashLen zero bytes are used.
    ikm:
        Input keying material (e.g., a Diffie-Hellman shared secret).
    info:
        Context string binding the derived key to its purpose.
    length:
        Desired output length in bytes.
    hash:
        Hash algorithm: ``"sha256"`` (default) or ``"sha512"``.

    Returns
    -------
    bytes
        Output keying material (OKM), exactly ``length`` bytes.

    Examples
    --------
    >>> # RFC 5869 Test Case 1
    >>> ikm = bytes.fromhex("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
    >>> salt = bytes.fromhex("000102030405060708090a0b0c")
    >>> info = bytes.fromhex("f0f1f2f3f4f5f6f7f8f9")
    >>> hkdf(salt, ikm, info, 42).hex()
    '3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865'

    """
    prk = hkdf_extract(salt, ikm, hash)
    return hkdf_expand(prk, info, length, hash)


# ─── Private Helpers ──────────────────────────────────────────────────────────


def _get_hash_params(hash_name: str) -> tuple[object, int, int]:
    """Look up (hash_fn, block_size, hash_len) for a named hash algorithm.

    Raises
    ------
    ValueError
        If the hash algorithm is not supported.
    """
    if hash_name not in _HASH_PARAMS:
        supported = ", ".join(sorted(_HASH_PARAMS))
        msg = (
            f"Unsupported hash algorithm: {hash_name!r}. "
            f"Supported: {supported}"
        )
        raise ValueError(msg)
    return _HASH_PARAMS[hash_name]
