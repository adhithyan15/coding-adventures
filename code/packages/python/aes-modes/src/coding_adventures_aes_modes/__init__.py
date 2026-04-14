"""
coding_adventures_aes_modes --- AES Modes of Operation (ECB, CBC, CTR, GCM)

Why Do We Need Modes?
---------------------

AES is a *block cipher*: it encrypts exactly 16 bytes (128 bits) at a time.
But real messages are rarely exactly 16 bytes. A "mode of operation" defines
how to use a block cipher to encrypt messages of arbitrary length.

The choice of mode is *critical* for security:

  - **ECB** (Electronic Codebook) --- encrypts each block independently.
    This is catastrophically insecure because identical plaintext blocks
    produce identical ciphertext blocks, revealing patterns. The famous
    "ECB penguin" demonstrates this: encrypting a bitmap image in ECB mode
    preserves the image structure in the ciphertext.

  - **CBC** (Cipher Block Chaining) --- chains blocks together by XOR-ing
    each plaintext block with the previous ciphertext block. This hides
    patterns but is vulnerable to padding oracle attacks (POODLE, Lucky 13).

  - **CTR** (Counter Mode) --- turns the block cipher into a *stream cipher*
    by encrypting a counter and XOR-ing the resulting keystream with the
    plaintext. No padding needed. Parallelizable. The modern standard.

  - **GCM** (Galois/Counter Mode) --- CTR mode plus a polynomial-based
    authentication tag (GHASH). Provides both confidentiality AND integrity.
    The gold standard for TLS 1.3. Detects if ciphertext was tampered with.

Security Hierarchy
------------------

    ECB  <  CBC  <  CTR  <  GCM
  (broken) (legacy) (good) (best)

ECB and CBC are implemented here purely for educational purposes.
In production, always use GCM (or another AEAD construction).

Dependencies
------------

This package wraps the AES block cipher from coding_adventures_aes,
which provides encrypt_block and decrypt_block operating on 16-byte blocks.
"""

from __future__ import annotations

from coding_adventures_aes import aes_decrypt_block, aes_encrypt_block

# =============================================================================
# PKCS#7 Padding
# =============================================================================
#
# Block ciphers in ECB and CBC mode require the plaintext to be an exact
# multiple of the block size (16 bytes for AES). PKCS#7 padding fills the
# gap by appending N bytes, each with value N.
#
# Example (block size = 16):
#   Plaintext is 11 bytes long -> need 5 more bytes -> append [05 05 05 05 05]
#   Plaintext is 16 bytes long -> append full block [10 10 10 ... 10] (16 bytes)
#
# The second case is important: even if the plaintext is already aligned,
# we MUST add a full block of padding. Otherwise, when unpadding, we could
# not distinguish "the last byte is 0x01 as data" from "the last byte is
# 0x01 as padding."

BLOCK_SIZE = 16


def pkcs7_pad(data: bytes) -> bytes:
    """Pad data to a multiple of 16 bytes using PKCS#7.

    Always adds at least 1 byte of padding (up to 16 bytes).
    Each padding byte has the value equal to the number of padding bytes added.

    Examples:
        >>> pkcs7_pad(b'hello')  # 5 bytes -> 11 padding bytes of value 0x0b
        b'hello\\x0b\\x0b\\x0b\\x0b\\x0b\\x0b\\x0b\\x0b\\x0b\\x0b\\x0b'
        >>> len(pkcs7_pad(b'sixteen_bytes!!!'))  # 16 bytes -> 32 bytes (full block of padding)
        32
    """
    pad_len = BLOCK_SIZE - (len(data) % BLOCK_SIZE)
    return data + bytes([pad_len] * pad_len)


def pkcs7_unpad(data: bytes) -> bytes:
    """Remove PKCS#7 padding and return the original data.

    Validates that:
    1. Data is non-empty and a multiple of 16 bytes
    2. The padding length (last byte) is between 1 and 16
    3. All padding bytes have the same value

    Raises ValueError on invalid padding (this is what padding oracle attacks
    exploit --- the error message itself leaks information).
    """
    if len(data) == 0 or len(data) % BLOCK_SIZE != 0:
        msg = f"Data length {len(data)} is not a positive multiple of {BLOCK_SIZE}"
        raise ValueError(msg)

    pad_len = data[-1]

    if pad_len < 1 or pad_len > BLOCK_SIZE:
        raise ValueError("Invalid PKCS#7 padding")

    # Verify ALL padding bytes match using constant-time comparison to prevent
    # timing side-channels. We accumulate differences with OR rather than
    # returning early on the first mismatch — this ensures the loop always
    # takes the same time regardless of which byte (if any) is wrong.
    diff = 0
    for i in range(1, pad_len + 1):
        diff |= data[-i] ^ pad_len
    if diff != 0:
        raise ValueError("Invalid PKCS#7 padding")

    return data[:-pad_len]


# =============================================================================
# Utility: XOR two byte strings
# =============================================================================


def xor_bytes(a: bytes, b: bytes) -> bytes:
    """XOR two byte strings of equal length.

    XOR is the fundamental operation in symmetric cryptography:
    - It's its own inverse: (A XOR B) XOR B = A
    - It perfectly mixes bits: each output bit depends on both inputs
    - It's the cheapest operation in hardware (single gate per bit)
    """
    return bytes(x ^ y for x, y in zip(a, b))


# =============================================================================
# ECB Mode (Electronic Codebook) --- INSECURE
# =============================================================================
#
# ECB is the simplest possible mode. Each 16-byte block is encrypted
# independently with the same key:
#
#   Plaintext:   [  Block 1  ] [  Block 2  ] [  Block 3  ]
#                      |              |              |
#                  AES_encrypt    AES_encrypt    AES_encrypt
#                      |              |              |
#   Ciphertext:  [  Block 1  ] [  Block 2  ] [  Block 3  ]
#
# The fatal flaw: identical plaintext blocks produce identical ciphertext
# blocks. This means patterns in the plaintext are visible in the ciphertext.
# The famous "ECB penguin" image demonstrates this perfectly --- the outline
# of the penguin is clearly visible even after encryption.
#
# ECB is included here as an anti-pattern. Never use it for real encryption.


def ecb_encrypt(plaintext: bytes, key: bytes) -> bytes:
    """Encrypt plaintext using AES in ECB mode (INSECURE --- educational only).

    1. Pad the plaintext with PKCS#7
    2. Split into 16-byte blocks
    3. Encrypt each block independently

    Args:
        plaintext: Arbitrary-length data to encrypt
        key: 16, 24, or 32 bytes (AES-128/192/256)

    Returns:
        Ciphertext (always a multiple of 16 bytes, longer than plaintext due to padding)
    """
    padded = pkcs7_pad(plaintext)
    ciphertext = b""
    for i in range(0, len(padded), BLOCK_SIZE):
        block = padded[i : i + BLOCK_SIZE]
        ciphertext += aes_encrypt_block(block, key)
    return ciphertext


def ecb_decrypt(ciphertext: bytes, key: bytes) -> bytes:
    """Decrypt ciphertext that was encrypted with AES-ECB.

    1. Decrypt each 16-byte block independently
    2. Remove PKCS#7 padding

    Args:
        ciphertext: Must be a non-empty multiple of 16 bytes
        key: Same key used for encryption

    Returns:
        Original plaintext with padding removed
    """
    if len(ciphertext) == 0 or len(ciphertext) % BLOCK_SIZE != 0:
        msg = f"Ciphertext length {len(ciphertext)} is not a positive multiple of {BLOCK_SIZE}"
        raise ValueError(msg)

    plaintext = b""
    for i in range(0, len(ciphertext), BLOCK_SIZE):
        block = ciphertext[i : i + BLOCK_SIZE]
        plaintext += aes_decrypt_block(block, key)
    return pkcs7_unpad(plaintext)


# =============================================================================
# CBC Mode (Cipher Block Chaining) --- Legacy
# =============================================================================
#
# CBC fixes ECB's pattern-leaking problem by chaining blocks together.
# Each plaintext block is XOR'd with the *previous* ciphertext block
# before encryption:
#
#       IV ----+
#              |
#              v
#   P[0] --> XOR --> AES_encrypt --> C[0] ---+
#                                            |
#                                            v
#   P[1] -----------------> XOR --> AES_encrypt --> C[1] ---+
#                                                           |
#                                                           v
#   P[2] -----------------------------> XOR --> AES_encrypt --> C[2]
#
# The Initialization Vector (IV) "randomizes" the first block. With a
# random IV, encrypting the same plaintext twice produces different
# ciphertexts (unlike ECB).
#
# CBC's weakness: padding oracle attacks. If an attacker can tell whether
# decryption produced valid padding (e.g., via different error messages or
# timing), they can recover the plaintext byte by byte. This is why TLS
# moved from CBC to GCM.


def cbc_encrypt(plaintext: bytes, key: bytes, iv: bytes) -> bytes:
    """Encrypt plaintext using AES in CBC mode.

    The IV (Initialization Vector) MUST be:
    - Exactly 16 bytes
    - Unpredictable (random) for each message
    - Never reused with the same key

    Args:
        plaintext: Arbitrary-length data to encrypt
        key: 16, 24, or 32 bytes (AES-128/192/256)
        iv: Exactly 16 random bytes (transmitted alongside ciphertext)

    Returns:
        Ciphertext (multiple of 16 bytes)
    """
    if len(iv) != BLOCK_SIZE:
        msg = f"IV must be {BLOCK_SIZE} bytes, got {len(iv)}"
        raise ValueError(msg)

    padded = pkcs7_pad(plaintext)
    ciphertext = b""
    prev = iv  # The "previous ciphertext block" starts as the IV

    for i in range(0, len(padded), BLOCK_SIZE):
        block = padded[i : i + BLOCK_SIZE]
        # XOR plaintext with previous ciphertext, then encrypt
        xored = xor_bytes(block, prev)
        encrypted = aes_encrypt_block(xored, key)
        ciphertext += encrypted
        prev = encrypted  # This block becomes "previous" for the next iteration

    return ciphertext


def cbc_decrypt(ciphertext: bytes, key: bytes, iv: bytes) -> bytes:
    """Decrypt ciphertext that was encrypted with AES-CBC.

    Decryption reverses the process:
      P[i] = AES_decrypt(C[i], key) XOR C[i-1]
    where C[-1] = IV.

    Args:
        ciphertext: Must be a non-empty multiple of 16 bytes
        key: Same key used for encryption
        iv: Same IV used for encryption (16 bytes)

    Returns:
        Original plaintext with padding removed
    """
    if len(iv) != BLOCK_SIZE:
        msg = f"IV must be {BLOCK_SIZE} bytes, got {len(iv)}"
        raise ValueError(msg)
    if len(ciphertext) == 0 or len(ciphertext) % BLOCK_SIZE != 0:
        msg = f"Ciphertext length {len(ciphertext)} is not a positive multiple of {BLOCK_SIZE}"
        raise ValueError(msg)

    plaintext = b""
    prev = iv

    for i in range(0, len(ciphertext), BLOCK_SIZE):
        block = ciphertext[i : i + BLOCK_SIZE]
        decrypted = aes_decrypt_block(block, key)
        plaintext += xor_bytes(decrypted, prev)
        prev = block  # Use the ciphertext block (not decrypted!) as prev

    return pkcs7_unpad(plaintext)


# =============================================================================
# CTR Mode (Counter Mode) --- Recommended
# =============================================================================
#
# CTR mode turns a block cipher into a *stream cipher*. Instead of encrypting
# the plaintext directly, we encrypt a *counter* and XOR the resulting
# "keystream" with the plaintext:
#
#   nonce || counter=1    nonce || counter=2    nonce || counter=3
#          |                      |                      |
#      AES_encrypt            AES_encrypt            AES_encrypt
#          |                      |                      |
#      keystream[1]           keystream[2]           keystream[3]
#          |                      |                      |
#   P[0] XOR              P[1] XOR              P[2] XOR
#          |                      |                      |
#   C[0]                   C[1]                   C[2]
#
# Key properties:
# - No padding needed (XOR the exact number of bytes)
# - Parallelizable (each block is independent)
# - Random access (can decrypt block N without blocks 0..N-1)
# - Encryption = Decryption (XOR is its own inverse)
#
# The counter block format used here:
#   [  12-byte nonce  ] [ 4-byte big-endian counter ]
#
# CRITICAL: Never reuse a nonce with the same key. If you do:
#   C1 = P1 XOR keystream
#   C2 = P2 XOR keystream  (same keystream!)
#   C1 XOR C2 = P1 XOR P2  (keystream cancels out!)
# An attacker gets the XOR of the two plaintexts, which is often enough
# to recover both.


def _build_counter_block(nonce: bytes, counter: int) -> bytes:
    """Build a 16-byte counter block: 12-byte nonce || 4-byte big-endian counter.

    The nonce identifies this particular message. The counter increments
    for each 16-byte block within the message. Together, they ensure every
    AES input is unique (as long as the nonce is unique per message).
    """
    return nonce + counter.to_bytes(4, "big")


def ctr_encrypt(plaintext: bytes, key: bytes, nonce: bytes) -> bytes:
    """Encrypt plaintext using AES in CTR mode.

    Args:
        plaintext: Arbitrary-length data (no padding needed)
        key: 16, 24, or 32 bytes
        nonce: Exactly 12 bytes. MUST be unique per message with the same key.

    Returns:
        Ciphertext (same length as plaintext)
    """
    if len(nonce) != 12:
        msg = f"Nonce must be 12 bytes, got {len(nonce)}"
        raise ValueError(msg)

    ciphertext = b""
    counter = 1  # Start at 1 (GCM reserves counter 0 for the tag)

    for i in range(0, len(plaintext), BLOCK_SIZE):
        # Build the counter block and encrypt it to produce keystream
        counter_block = _build_counter_block(nonce, counter)
        keystream = aes_encrypt_block(counter_block, key)

        # XOR the keystream with the plaintext chunk
        # The last chunk may be shorter than 16 bytes --- only XOR what we have
        chunk = plaintext[i : i + BLOCK_SIZE]
        ciphertext += xor_bytes(keystream[: len(chunk)], chunk)

        counter += 1

    return ciphertext


def ctr_decrypt(ciphertext: bytes, key: bytes, nonce: bytes) -> bytes:
    """Decrypt ciphertext that was encrypted with AES-CTR.

    CTR decryption is identical to encryption because XOR is its own inverse:
      encrypt: C = P XOR keystream
      decrypt: P = C XOR keystream  (same operation!)
    """
    return ctr_encrypt(ciphertext, key, nonce)


# =============================================================================
# GCM Mode (Galois/Counter Mode) --- Recommended with Authentication
# =============================================================================
#
# GCM combines CTR mode encryption with a polynomial hash (GHASH) to provide
# *authenticated encryption with associated data* (AEAD). This means:
#
#   1. Confidentiality: the plaintext is hidden (via CTR mode)
#   2. Integrity: any modification to the ciphertext is detected (via the tag)
#   3. Associated data: additional unencrypted data (headers, metadata) can
#      be authenticated without being encrypted
#
# Architecture:
#
#   H = AES_encrypt(0^128, key)    <- hash subkey (computed once per key)
#
#   J0 = IV || 0x00000001          <- initial counter value
#
#   Ciphertext = CTR_encrypt(plaintext, key, starting at counter=2)
#
#   Tag = GHASH(H, AAD, ciphertext) XOR AES_encrypt(J0, key)
#
# The GHASH function operates in GF(2^128) --- a finite field with 2^128
# elements, using the reducing polynomial:
#
#   R(x) = x^128 + x^7 + x^2 + x + 1
#
# This is a DIFFERENT field from GF(2^8) used inside AES itself. GF(2^128)
# operates on 128-bit (16-byte) values.


def _gf128_mul(x: bytes, y: bytes) -> bytes:
    """Multiply two elements in GF(2^128) with the GCM reducing polynomial.

    GCM uses the polynomial R = x^128 + x^7 + x^2 + x + 1, which in the
    "bit-reflected" representation corresponds to the constant 0xE1 << 120.

    The algorithm is "shift-and-add" (schoolbook multiplication in GF(2)):

    1. Convert x and y to 128-bit integers
    2. For each bit of y (from MSB to LSB):
       a. If the bit is 1, XOR the current value of x into the result
       b. Check if the LSB of x is 1 (will overflow on shift)
       c. Right-shift x by 1
       d. If overflow, XOR with the reducing polynomial

    Note on bit ordering: GCM uses a "reflected" bit convention where
    bit 0 is the MSB. This means we process bits from MSB to LSB and
    shift right (not left). The reducing polynomial in this convention
    is 0xE1000000000000000000000000000000.
    """
    # Convert bytes to 128-bit integers (big-endian)
    x_int = int.from_bytes(x, "big")
    y_int = int.from_bytes(y, "big")

    # The reducing polynomial: x^128 + x^7 + x^2 + x + 1
    # In the reflected representation: 0xE1 << 120
    r = 0xE1000000000000000000000000000000

    z = 0  # Accumulator (result)
    v = x_int  # Shifting copy of x

    for i in range(128):
        # Check bit i of y (MSB first, which is bit 127-i in normal representation)
        if y_int & (1 << (127 - i)):
            z ^= v

        # Check if LSB of v is set (will "overflow" on right shift)
        carry = v & 1

        # Right-shift v by 1
        v >>= 1

        # If there was a carry, XOR with the reducing polynomial
        if carry:
            v ^= r

    return z.to_bytes(16, "big")


def _ghash(h: bytes, *data_blocks: bytes) -> bytes:
    """Compute GHASH over one or more data sequences.

    GHASH is a keyed hash function using multiplication in GF(2^128).
    It processes 16-byte blocks sequentially:

      Y_0 = 0^128
      Y_i = (Y_{i-1} XOR X_i) * H    (in GF(2^128))

    The hash subkey H = AES_encrypt(0^128, key) is derived from the key.

    For GCM, GHASH is called with:
      GHASH(H,  AAD_padded || ciphertext_padded || len(AAD) || len(C))

    The padding here is zero-padding to a 16-byte boundary, and the lengths
    are 64-bit big-endian bit counts.

    Args:
        h: The hash subkey (16 bytes)
        data_blocks: One or more byte strings to hash (concatenated and
                     processed as consecutive 16-byte blocks)

    Returns:
        16-byte GHASH digest
    """
    data = b"".join(data_blocks)
    y = b"\x00" * 16

    # Process each 16-byte block
    for i in range(0, len(data), 16):
        block = data[i : i + 16]
        # Pad the last block with zeros if needed
        if len(block) < 16:
            block = block + b"\x00" * (16 - len(block))
        y = _gf128_mul(xor_bytes(y, block), h)

    return y


def _pad_to_16(data: bytes) -> bytes:
    """Zero-pad data to a multiple of 16 bytes.

    Unlike PKCS#7, this just appends zero bytes. It's used for GHASH input
    alignment, not for encryption padding.
    """
    remainder = len(data) % 16
    if remainder == 0:
        return data
    return data + b"\x00" * (16 - remainder)


def gcm_encrypt(
    plaintext: bytes,
    key: bytes,
    iv: bytes,
    aad: bytes = b"",
) -> tuple[bytes, bytes]:
    """Encrypt and authenticate using AES-GCM.

    GCM provides authenticated encryption: if any bit of the ciphertext,
    AAD, or IV is modified, decryption will detect the tampering and fail.

    Algorithm:
      1. Compute hash subkey: H = AES(0^128, key)
      2. Build initial counter J0 = IV || 0x00000001  (for 12-byte IV)
      3. Encrypt plaintext with CTR starting at counter=2
      4. Compute authentication tag via GHASH

    Args:
        plaintext: Data to encrypt (arbitrary length)
        key: 16, 24, or 32 bytes
        iv: Exactly 12 bytes. MUST be unique per message.
        aad: Additional Authenticated Data --- integrity-protected but NOT
             encrypted. Used for headers, metadata, etc.

    Returns:
        (ciphertext, tag) where tag is 16 bytes
    """
    if len(iv) != 12:
        msg = f"IV must be 12 bytes, got {len(iv)}"
        raise ValueError(msg)

    # Step 1: Compute the hash subkey H = AES_encrypt(0^128, key)
    # H is used for GHASH polynomial multiplication
    h = aes_encrypt_block(b"\x00" * 16, key)

    # Step 2: Build the initial counter block J0
    # For a 12-byte IV: J0 = IV || 0x00000001
    j0 = iv + b"\x00\x00\x00\x01"

    # Step 3: Encrypt plaintext using CTR mode starting at counter value 2
    # Counter value 1 (J0) is reserved for computing the authentication tag
    ciphertext = b""
    counter = 2

    for i in range(0, max(len(plaintext), 1), BLOCK_SIZE):
        if i >= len(plaintext):
            break
        counter_block = iv + counter.to_bytes(4, "big")
        keystream = aes_encrypt_block(counter_block, key)
        chunk = plaintext[i : i + BLOCK_SIZE]
        ciphertext += xor_bytes(keystream[: len(chunk)], chunk)
        counter += 1

    # Step 4: Compute the authentication tag
    #
    # The GHASH input is:
    #   pad(AAD) || pad(ciphertext) || len_aad_bits || len_ct_bits
    #
    # where lengths are 64-bit big-endian bit counts.
    len_block = (len(aad) * 8).to_bytes(8, "big") + (len(ciphertext) * 8).to_bytes(
        8, "big"
    )

    ghash_input = _pad_to_16(aad) + _pad_to_16(ciphertext) + len_block
    s = _ghash(h, ghash_input)

    # Tag = GHASH_result XOR AES_encrypt(J0, key)
    # Encrypting J0 ensures the tag depends on the key even if H is compromised
    tag = xor_bytes(s, aes_encrypt_block(j0, key))

    return ciphertext, tag


def gcm_decrypt(
    ciphertext: bytes,
    key: bytes,
    iv: bytes,
    aad: bytes = b"",
    tag: bytes = b"",
) -> bytes:
    """Decrypt and verify using AES-GCM.

    IMPORTANT: This verifies the authentication tag BEFORE returning the
    plaintext. If the tag does not match (indicating tampering), a ValueError
    is raised and NO plaintext is returned.

    In a real implementation, the tag comparison must be constant-time to
    prevent timing attacks. Here we use simple equality for clarity.

    Args:
        ciphertext: Data to decrypt
        key: Same key used for encryption
        iv: Same IV used for encryption (12 bytes)
        aad: Same AAD used for encryption
        tag: The 16-byte authentication tag from encryption

    Returns:
        Decrypted plaintext

    Raises:
        ValueError: If the tag does not match (ciphertext was tampered with)
    """
    if len(iv) != 12:
        msg = f"IV must be 12 bytes, got {len(iv)}"
        raise ValueError(msg)
    if len(tag) != 16:
        msg = f"Tag must be 16 bytes, got {len(tag)}"
        raise ValueError(msg)

    # Recompute the hash subkey and tag
    h = aes_encrypt_block(b"\x00" * 16, key)
    j0 = iv + b"\x00\x00\x00\x01"

    # Compute what the tag SHOULD be for this ciphertext
    len_block = (len(aad) * 8).to_bytes(8, "big") + (len(ciphertext) * 8).to_bytes(
        8, "big"
    )
    ghash_input = _pad_to_16(aad) + _pad_to_16(ciphertext) + len_block
    s = _ghash(h, ghash_input)
    expected_tag = xor_bytes(s, aes_encrypt_block(j0, key))

    # Verify the tag using constant-time comparison to prevent timing attacks.
    # We OR together all byte differences — if any byte differs, diff will be
    # non-zero, but an attacker cannot tell WHICH byte from the timing.
    diff = 0
    for a_byte, b_byte in zip(expected_tag, tag):
        diff |= a_byte ^ b_byte
    if diff != 0:
        msg = "Authentication tag mismatch --- ciphertext may have been tampered with"
        raise ValueError(msg)

    # Tag is valid --- decrypt using CTR mode starting at counter=2
    plaintext = b""
    counter = 2

    for i in range(0, max(len(ciphertext), 1), BLOCK_SIZE):
        if i >= len(ciphertext):
            break
        counter_block = iv + counter.to_bytes(4, "big")
        keystream = aes_encrypt_block(counter_block, key)
        chunk = ciphertext[i : i + BLOCK_SIZE]
        plaintext += xor_bytes(keystream[: len(chunk)], chunk)
        counter += 1

    return plaintext


# =============================================================================
# Public API
# =============================================================================

__all__ = [
    "ecb_encrypt",
    "ecb_decrypt",
    "cbc_encrypt",
    "cbc_decrypt",
    "ctr_encrypt",
    "ctr_decrypt",
    "gcm_encrypt",
    "gcm_decrypt",
    "pkcs7_pad",
    "pkcs7_unpad",
]
