"""hmac -- HMAC (Hash-based Message Authentication Code) — RFC 2104 / FIPS 198-1.

What Is HMAC?
=============
HMAC takes a secret key and a message and produces a fixed-size authentication
tag. It proves both the integrity of the message (it has not been altered) and
its authenticity (the sender knows the secret key).

HMAC is used in:
- TLS 1.2 handshakes (HMAC-SHA256 for the PRF)
- JWT signatures (HS256 = HMAC-SHA256, HS512 = HMAC-SHA512)
- WPA2 Wi-Fi (HMAC-SHA1 inside PBKDF2)
- TOTP / HOTP one-time passwords (RFC 6238 / RFC 4226)
- AWS Signature Version 4 request signing

Why Not hash(key || message)?
=============================
Naïvely prepending the key is vulnerable to a **length extension attack**.
Merkle-Damgård hash functions (MD5, SHA-1, SHA-256, SHA-512) produce a digest
that IS the hash function's internal state after the last block. If an attacker
knows the digest of `key || message`, they can compute the digest of
`key || message || padding || extra` without knowing `key` — they just resume
the hash from the known state and add more blocks.

HMAC defeats this with two nested hash calls under different keys:

  HMAC(K, M) = H((K' XOR opad) || H((K' XOR ipad) || M))

The outer hash wraps the inner result under a different padded key. An attacker
who can extend the inner hash cannot touch the outer hash without knowing
`opad_key`.

The HMAC Construction (RFC 2104 § 2)
=====================================

  Input: key K (any length), message M, hash H, block size B

  1. Normalize key to B bytes:
       len(K) > B  →  K' = H(K)          (hash long keys)
       len(K) < B  →  K' = K || 0x00…   (zero-pad short keys)
       len(K) = B  →  K' = K

  2. Derive padded keys:
       inner_key = K' XOR (0x36 * B)
       outer_key = K' XOR (0x5C * B)

  3. Compute nested hashes:
       inner = H(inner_key || M)
       result = H(outer_key || inner)

The constants 0x36 ("ipad") and 0x5C ("opad") were chosen by Hugo Krawczyk
because they differ in 4 of 8 bits (0x36 = 0011_0110, 0x5C = 0101_1100),
maximising the Hamming distance between the two derived keys.

Block Sizes and Digest Sizes
=============================
  Algorithm   Block (bytes)   Digest (bytes)   Used in
  ─────────   ────────────    ──────────────   ───────
  MD5         64              16               Legacy, UUID v3
  SHA-1       64              20               WPA2, older TLS
  SHA-256     64              32               TLS 1.3, JWT HS256
  SHA-512     128             64               JWT HS512, high-security

Note that SHA-512 uses a 128-byte block (it operates on 64-bit words, so
the 16-word block is 128 bytes). This means the ipad/opad byte strings are
128 bytes long for HMAC-SHA512, not 64.

RFC 4231 Test Vector (TC1, HMAC-SHA256)
========================================
  Key:  b'\\x0b' * 20
  Data: b'Hi There'
  Tag:  b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7
"""

from __future__ import annotations

import hmac as _hmac_stdlib
from collections.abc import Callable

from coding_adventures_md5 import md5
from coding_adventures_sha1 import sha1
from coding_adventures_sha256 import sha256
from coding_adventures_sha512 import sha512

__version__ = "0.1.0"

# Type alias: a hash function that takes bytes and returns bytes.
_HashFn = Callable[[bytes], bytes]

# ipad byte and opad byte (RFC 2104 § 2)
_IPAD: int = 0x36
_OPAD: int = 0x5C


# =============================================================================
# Generic HMAC
# =============================================================================


def hmac(hash_fn: _HashFn, block_size: int, key: bytes, message: bytes) -> bytes:
    """Compute HMAC using any hash function.

    This is the core RFC 2104 construction. All named variants delegate here.

    Parameters
    ----------
    hash_fn:
        A function ``bytes -> bytes`` implementing a Merkle-Damgård hash
        (e.g. ``sha256``, ``sha512``, ``md5``).
    block_size:
        The internal block size of *hash_fn* in bytes.
        MD5 / SHA-1 / SHA-256 → 64.  SHA-512 → 128.
    key:
        The secret key.  Any length is valid — long keys are hashed,
        short keys are zero-padded.
    message:
        The data to authenticate.  Any length is valid.

    Returns
    -------
    bytes
        The authentication tag.  Same length as ``hash_fn``'s output.

    Examples
    --------
    >>> from coding_adventures_sha256 import sha256
    >>> hmac(sha256, 64, b"key", b"message").hex()
    '6e9ef29b75fffc5b7abae527d58fdadb2fe42e7219011976917343065f58ed4a'

    """
    # Step 1 — normalize key to exactly block_size bytes
    key_prime = _normalize_key(hash_fn, block_size, key)

    # Step 2 — derive inner and outer padded keys
    # XOR each byte of the normalised key with the constant ipad / opad byte.
    inner_key = bytes(b ^ _IPAD for b in key_prime)
    outer_key = bytes(b ^ _OPAD for b in key_prime)

    # Step 3 — nested hash calls
    inner = hash_fn(inner_key + message)
    return hash_fn(outer_key + inner)


# =============================================================================
# Named variants — MD5, SHA-1, SHA-256, SHA-512
# =============================================================================


def hmac_md5(key: bytes, message: bytes) -> bytes:
    """HMAC-MD5: 16-byte (128-bit) authentication tag.

    HMAC-MD5 remains secure as a MAC even though MD5 is broken for collision
    resistance — MAC security is a different property.  It appears in legacy
    protocols (older TLS cipher suites) and is still widely deployed.

    Examples
    --------
    >>> hmac_md5(b"Jefe", b"what do ya want for nothing?").hex()
    '750c783e6ab0b503eaa86e310a5db738'

    """
    if not key:
        raise ValueError("HMAC key must not be empty")
    return hmac(md5, 64, key, message)


def hmac_sha1(key: bytes, message: bytes) -> bytes:
    """HMAC-SHA1: 20-byte (160-bit) authentication tag.

    Used in WPA2 (PBKDF2-HMAC-SHA1 for PSK derivation), older SSH and TLS
    versions, and TOTP/HOTP one-time password algorithms.

    Examples
    --------
    >>> hmac_sha1(b"Jefe", b"what do ya want for nothing?").hex()
    'effcdf6ae5eb2fa2d27416d5f184df9c259a7c79'

    """
    if not key:
        raise ValueError("HMAC key must not be empty")
    return hmac(sha1, 64, key, message)


def hmac_sha256(key: bytes, message: bytes) -> bytes:
    """HMAC-SHA256: 32-byte (256-bit) authentication tag.

    The modern default.  Used in TLS 1.3, JWT HS256, AWS Signature V4,
    PBKDF2-HMAC-SHA256, and most newly designed protocols.

    Examples
    --------
    >>> hmac_sha256(b"\\x0b" * 20, b"Hi There").hex()
    'b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7'

    """
    if not key:
        raise ValueError("HMAC key must not be empty")
    return hmac(sha256, 64, key, message)


def hmac_sha512(key: bytes, message: bytes) -> bytes:
    """HMAC-SHA512: 64-byte (512-bit) authentication tag.

    Used in JWT HS512 and high-security configurations.  Note that SHA-512
    has a 128-byte block size, so the ipad/opad strings are 128 bytes long.

    Examples
    --------
    >>> key = b"\\x0b" * 20
    >>> hmac_sha512(key, b"Hi There").hex()  # doctest: +ELLIPSIS
    '87aa7cdea5ef619d4ff0b4241a1d6cb0...'

    """
    if not key:
        raise ValueError("HMAC key must not be empty")
    return hmac(sha512, 128, key, message)


# =============================================================================
# Hex-string variants
# =============================================================================


def hmac_md5_hex(key: bytes, message: bytes) -> str:
    """HMAC-MD5 as a lowercase hex string (32 characters)."""
    return hmac_md5(key, message).hex()


def hmac_sha1_hex(key: bytes, message: bytes) -> str:
    """HMAC-SHA1 as a lowercase hex string (40 characters)."""
    return hmac_sha1(key, message).hex()


def hmac_sha256_hex(key: bytes, message: bytes) -> str:
    """HMAC-SHA256 as a lowercase hex string (64 characters)."""
    return hmac_sha256(key, message).hex()


def hmac_sha512_hex(key: bytes, message: bytes) -> str:
    """HMAC-SHA512 as a lowercase hex string (128 characters)."""
    return hmac_sha512(key, message).hex()


# =============================================================================
# Constant-time tag verification
# =============================================================================


def verify(expected: bytes, actual: bytes) -> bool:
    """Compare two HMAC tags in constant time.

    Use this instead of ``==`` when checking whether a received tag matches an
    expected tag.  The ``==`` operator short-circuits on the first differing
    byte, leaking timing information about *how many bytes* match.  Over many
    requests an attacker can exploit these timing differences to reconstruct
    the expected tag byte by byte — a **timing attack**.

    ``verify`` delegates to :func:`hmac.compare_digest` from the standard
    library, which is implemented in C and guaranteed to run in constant time.

    Parameters
    ----------
    expected:
        The tag produced locally using the secret key.
    actual:
        The tag received from an untrusted source.

    Returns
    -------
    bool
        ``True`` iff ``expected`` and ``actual`` are identical.

    Examples
    --------
    >>> tag = hmac_sha256(b"secret", b"message")
    >>> verify(tag, tag)
    True
    >>> verify(tag, b"wrong")
    False

    """
    return _hmac_stdlib.compare_digest(expected, actual)


# =============================================================================
# Private helpers
# =============================================================================


def _normalize_key(hash_fn: _HashFn, block_size: int, key: bytes) -> bytes:
    """Bring *key* to exactly *block_size* bytes.

    RFC 2104 § 2:
    - If len(key) > block_size: key = hash(key)
    - Then zero-pad to block_size if shorter.

    Parameters
    ----------
    hash_fn:
        The hash function (used only when key is longer than block_size).
    block_size:
        Target length in bytes.
    key:
        The original key, any length.

    Returns
    -------
    bytes
        Exactly *block_size* bytes.

    """
    if len(key) > block_size:
        key = hash_fn(key)
    # Zero-pad to block_size (works whether key was hashed or already short)
    return key.ljust(block_size, b"\x00")
