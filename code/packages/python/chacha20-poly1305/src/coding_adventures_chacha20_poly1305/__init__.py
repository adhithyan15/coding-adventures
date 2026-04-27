"""
ChaCha20-Poly1305: Authenticated Encryption with Associated Data (RFC 8439)
============================================================================

This module implements the ChaCha20-Poly1305 AEAD cipher suite from scratch,
using only basic arithmetic operations. It combines two primitives:

1. **ChaCha20** -- a stream cipher that generates a pseudorandom keystream
   using only Add, Rotate, and XOR (ARX) operations on 32-bit words.

2. **Poly1305** -- a one-time message authentication code (MAC) that produces
   a 16-byte tag using polynomial evaluation modulo a prime.

Together, they provide *authenticated encryption*: the ciphertext is both
confidential (only someone with the key can read it) and authentic (any
tampering is detected).

Why ChaCha20 instead of AES?
-----------------------------
AES relies on lookup tables (S-boxes) and Galois field arithmetic that are
complex and vulnerable to cache-timing side-channel attacks when implemented
in software. ChaCha20 uses only additions, rotations, and XORs -- operations
that run in constant time on all CPUs, making it naturally resistant to
timing attacks without any special effort.

Reference: RFC 8439 (https://www.rfc-editor.org/rfc/rfc8439)
"""

from __future__ import annotations

import struct

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

# The ChaCha20 state matrix begins with four constant words that spell out
# "expand 32-byte k" in ASCII. These constants ensure that even if an
# attacker controls the key, nonce, and counter, they cannot force the
# initial state into a degenerate configuration.
#
#   "expa" = 0x61707865
#   "nd 3" = 0x3320646e
#   "2-by" = 0x79622d32
#   "te k" = 0x6b206574

CHACHA20_CONSTANTS: tuple[int, int, int, int] = (
    0x61707865,
    0x3320646E,
    0x79622D32,
    0x6B206574,
)

# All 32-bit arithmetic must be masked to prevent Python's arbitrary-precision
# integers from growing beyond 32 bits.
MASK32 = 0xFFFFFFFF


# ---------------------------------------------------------------------------
# Utility: 32-bit left rotation
# ---------------------------------------------------------------------------

def _rotl32(value: int, shift: int) -> int:
    """Rotate a 32-bit integer left by *shift* bits.

    In hardware, a rotation is a single instruction. In Python, we emulate it
    by shifting left, shifting right the complement, and OR-ing the two halves
    together. The mask ensures we stay within 32 bits.

    Example (shift=16):
        0xAABBCCDD -> rotate left 16 -> 0xCCDDAABB

        High bits:  0xAABBCCDD << 16  = 0x0000AABB (after mask)
                                          Wait -- let's be precise:
        0xAABBCCDD << 16 = 0xAABBCCDD0000 & 0xFFFFFFFF = 0xCCDD0000
        0xAABBCCDD >> 16 = 0x0000AABB
        OR together       = 0xCCDDAABB  -- correct!
    """
    return ((value << shift) | (value >> (32 - shift))) & MASK32


# ---------------------------------------------------------------------------
# ChaCha20 Quarter Round
# ---------------------------------------------------------------------------

def _quarter_round(
    state: list[int], a: int, b: int, c: int, d: int,
) -> None:
    """Apply the ChaCha20 quarter round to four words in the state matrix.

    The quarter round is the core mixing function of ChaCha20. It takes four
    32-bit words and thoroughly mixes them using a sequence of ARX operations.
    Each step feeds the output of one operation into the next, creating a
    cascade of diffusion:

        a += b;  d ^= a;  d <<<= 16    Step 1: mix a,b into d
        c += d;  b ^= c;  b <<<= 12    Step 2: mix c,d into b
        a += b;  d ^= a;  d <<<= 8     Step 3: mix a,b into d again
        c += d;  b ^= c;  b <<<= 7     Step 4: mix c,d into b again

    The rotation amounts (16, 12, 8, 7) were chosen by Bernstein to maximize
    diffusion -- after 20 rounds, every output bit depends on every input bit.

    The function modifies *state* in place for efficiency.
    """
    # Step 1
    state[a] = (state[a] + state[b]) & MASK32
    state[d] ^= state[a]
    state[d] = _rotl32(state[d], 16)

    # Step 2
    state[c] = (state[c] + state[d]) & MASK32
    state[b] ^= state[c]
    state[b] = _rotl32(state[b], 12)

    # Step 3
    state[a] = (state[a] + state[b]) & MASK32
    state[d] ^= state[a]
    state[d] = _rotl32(state[d], 8)

    # Step 4
    state[c] = (state[c] + state[d]) & MASK32
    state[b] ^= state[c]
    state[b] = _rotl32(state[b], 7)


# ---------------------------------------------------------------------------
# ChaCha20 Block Function
# ---------------------------------------------------------------------------

def _chacha20_block(key: bytes, counter: int, nonce: bytes) -> bytes:
    """Generate one 64-byte keystream block.

    The ChaCha20 state is a 4x4 matrix of 32-bit words laid out as:

        +----------+----------+----------+----------+
        | const[0] | const[1] | const[2] | const[3] |   <- "expand 32-byte k"
        +----------+----------+----------+----------+
        |  key[0]  |  key[1]  |  key[2]  |  key[3]  |   <- first half of key
        +----------+----------+----------+----------+
        |  key[4]  |  key[5]  |  key[6]  |  key[7]  |   <- second half of key
        +----------+----------+----------+----------+
        | counter  | nonce[0] | nonce[1] | nonce[2] |   <- counter + nonce
        +----------+----------+----------+----------+

    The block function:
    1. Initializes this state from the inputs
    2. Runs 20 rounds (10 iterations of column + diagonal quarter rounds)
    3. Adds the original state back to the mixed state
    4. Serializes the result as 64 little-endian bytes

    Adding the original state back (step 3) is crucial: without it, the
    mixing rounds would be invertible, and an attacker could recover the key
    from any keystream block. The addition makes the function one-way.
    """
    # --- Initialize state ---
    # Unpack key as 8 little-endian 32-bit words
    key_words = struct.unpack("<8I", key)
    # Unpack nonce as 3 little-endian 32-bit words
    nonce_words = struct.unpack("<3I", nonce)

    state: list[int] = [
        CHACHA20_CONSTANTS[0], CHACHA20_CONSTANTS[1],
        CHACHA20_CONSTANTS[2], CHACHA20_CONSTANTS[3],
        key_words[0], key_words[1], key_words[2], key_words[3],
        key_words[4], key_words[5], key_words[6], key_words[7],
        counter & MASK32, nonce_words[0], nonce_words[1], nonce_words[2],
    ]

    # Save original state for the final addition
    initial_state = list(state)

    # --- 20 rounds = 10 double-rounds ---
    # Each double-round consists of:
    #   4 column quarter rounds (operating on columns of the 4x4 matrix)
    #   4 diagonal quarter rounds (operating on diagonals)
    #
    # Column indices:        Diagonal indices:
    #   (0,4,8,12)            (0,5,10,15)  <- main diagonal
    #   (1,5,9,13)            (1,6,11,12)  <- shifted by 1
    #   (2,6,10,14)           (2,7,8,13)   <- shifted by 2
    #   (3,7,11,15)           (3,4,9,14)   <- shifted by 3

    for _round in range(10):
        # Column rounds
        _quarter_round(state, 0, 4, 8, 12)
        _quarter_round(state, 1, 5, 9, 13)
        _quarter_round(state, 2, 6, 10, 14)
        _quarter_round(state, 3, 7, 11, 15)
        # Diagonal rounds
        _quarter_round(state, 0, 5, 10, 15)
        _quarter_round(state, 1, 6, 11, 12)
        _quarter_round(state, 2, 7, 8, 13)
        _quarter_round(state, 3, 4, 9, 14)

    # --- Add original state ---
    # This step converts the permutation into a PRF (pseudorandom function).
    for i in range(16):
        state[i] = (state[i] + initial_state[i]) & MASK32

    # --- Serialize as little-endian bytes ---
    return struct.pack("<16I", *state)


# ---------------------------------------------------------------------------
# ChaCha20 Stream Cipher
# ---------------------------------------------------------------------------

def chacha20_encrypt(
    plaintext: bytes, key: bytes, nonce: bytes, counter: int = 0,
) -> bytes:
    """Encrypt (or decrypt) data using the ChaCha20 stream cipher.

    ChaCha20 is a *stream cipher*: it generates a pseudorandom keystream and
    XORs it with the plaintext to produce ciphertext. Because XOR is its own
    inverse, the same function encrypts and decrypts:

        ciphertext = plaintext XOR keystream
        plaintext  = ciphertext XOR keystream

    The keystream is produced in 64-byte blocks. Each block uses a different
    counter value, so up to 2^32 * 64 = 256 GiB can be encrypted with one
    key/nonce pair.

    Parameters
    ----------
    plaintext : bytes
        Data to encrypt (or ciphertext to decrypt).
    key : bytes
        256-bit (32-byte) secret key.
    nonce : bytes
        96-bit (12-byte) nonce. Must be unique per key.
    counter : int
        Starting block counter (default 0). RFC 8439 AEAD uses counter=1
        for encryption, reserving counter=0 for Poly1305 key generation.

    Returns
    -------
    bytes
        The XOR of the input with the ChaCha20 keystream.
    """
    if len(key) != 32:
        raise ValueError(f"Key must be 32 bytes, got {len(key)}")
    if len(nonce) != 12:
        raise ValueError(f"Nonce must be 12 bytes, got {len(nonce)}")

    result = bytearray()
    offset = 0

    while offset < len(plaintext):
        # Generate one 64-byte keystream block
        keystream = _chacha20_block(key, counter, nonce)

        # XOR plaintext with keystream (last block may be partial)
        chunk = plaintext[offset : offset + 64]
        for i, byte in enumerate(chunk):
            result.append(byte ^ keystream[i])

        offset += 64
        counter += 1

    return bytes(result)


# ---------------------------------------------------------------------------
# Poly1305 Message Authentication Code
# ---------------------------------------------------------------------------

def poly1305_mac(message: bytes, key: bytes) -> bytes:
    """Compute a Poly1305 one-time MAC tag.

    Poly1305 evaluates a polynomial over a prime field to produce a 16-byte
    authentication tag. It's blazingly fast and provably secure -- but only
    when each key is used *exactly once*. Reusing a Poly1305 key allows an
    attacker to forge tags.

    How it works:
    1. Split the 32-byte key into two 16-byte halves: r and s.
    2. "Clamp" r by clearing certain bits (this ensures r has a specific
       algebraic structure that makes the security proof work).
    3. Process the message in 16-byte chunks. For each chunk:
       a. Interpret the chunk as a little-endian integer
       b. Append a 0x01 byte (to distinguish from zero padding)
       c. Add it to the accumulator
       d. Multiply the accumulator by r
       e. Reduce modulo p = 2^130 - 5
    4. Add s to the final accumulator, modulo 2^128.

    The prime 2^130 - 5 was chosen because:
    - It's a Mersenne-like prime, enabling fast modular reduction
    - It's just barely larger than 128 bits, so each block fits naturally
    - Arithmetic modulo this prime is very efficient

    Parameters
    ----------
    message : bytes
        The message to authenticate.
    key : bytes
        32-byte one-time key (first 16 bytes = r, last 16 bytes = s).

    Returns
    -------
    bytes
        16-byte authentication tag.
    """
    if len(key) != 32:
        raise ValueError(f"Poly1305 key must be 32 bytes, got {len(key)}")

    # --- Split key into r and s ---
    r_bytes = bytearray(key[:16])
    s_bytes = key[16:]

    # --- Clamp r ---
    # The clamping operation clears specific bits in r to ensure it has a
    # particular algebraic structure. Without clamping, the security proof
    # doesn't hold. The cleared bits are:
    #   - Top 4 bits of bytes 3, 7, 11, 15 (mask with 0x0f)
    #   - Bottom 2 bits of bytes 4, 8, 12 (mask with 0xfc)
    r_bytes[3] &= 0x0F
    r_bytes[7] &= 0x0F
    r_bytes[11] &= 0x0F
    r_bytes[15] &= 0x0F
    r_bytes[4] &= 0xFC
    r_bytes[8] &= 0xFC
    r_bytes[12] &= 0xFC

    # Convert r and s to integers (little-endian)
    r = int.from_bytes(r_bytes, "little")
    s = int.from_bytes(s_bytes, "little")

    # The prime modulus: p = 2^130 - 5
    # This is close to a power of 2, making modular arithmetic efficient.
    p = (1 << 130) - 5

    # --- Process message in 16-byte blocks ---
    accumulator = 0

    for i in range(0, len(message), 16):
        chunk = message[i : i + 16]

        # Convert chunk to a little-endian integer, then set bit 8*len(chunk)
        # (equivalent to appending a 0x01 byte). This "sentinel bit" ensures
        # that trailing zero bytes in a chunk are distinguishable from padding,
        # preventing a length-extension-style forgery.
        n = int.from_bytes(chunk, "little")
        n |= 1 << (8 * len(chunk))

        # Accumulate: acc = ((acc + chunk) * r) mod p
        accumulator = ((accumulator + n) * r) % p

    # --- Finalize ---
    # Add s and take modulo 2^128. The mod 2^128 truncation is important:
    # it hides the internal state of the polynomial evaluation, preventing
    # an attacker from recovering r.
    tag_int = (accumulator + s) & ((1 << 128) - 1)

    return tag_int.to_bytes(16, "little")


# ---------------------------------------------------------------------------
# Pad16 Helper
# ---------------------------------------------------------------------------

def _pad16(data: bytes) -> bytes:
    """Return zero-padding bytes to make len(data) a multiple of 16.

    The AEAD construction requires that both AAD and ciphertext be padded
    to 16-byte boundaries before computing the Poly1305 tag. This ensures
    that the MAC input is unambiguous regardless of the original lengths.

    If the data is already a multiple of 16 bytes, no padding is added.
    """
    remainder = len(data) % 16
    if remainder == 0:
        return b""
    return b"\x00" * (16 - remainder)


# ---------------------------------------------------------------------------
# AEAD Encryption (RFC 8439 Section 2.8)
# ---------------------------------------------------------------------------

def aead_encrypt(
    plaintext: bytes, key: bytes, nonce: bytes, aad: bytes = b"",
) -> tuple[bytes, bytes]:
    """Encrypt and authenticate data using ChaCha20-Poly1305 AEAD.

    This combines ChaCha20 encryption with Poly1305 authentication following
    the construction specified in RFC 8439 Section 2.8:

    1. **Generate Poly1305 key**: Use ChaCha20 with counter=0 to generate
       a one-time Poly1305 key (first 32 bytes of the keystream block).

    2. **Encrypt**: Use ChaCha20 with counter=1 to encrypt the plaintext.
       Counter=0 is reserved for key generation, so encryption starts at 1.

    3. **Authenticate**: Compute a Poly1305 tag over a carefully constructed
       input that includes both the AAD and ciphertext, along with their
       lengths. This binds the authentication to all the data.

    The MAC input is constructed as:
        AAD || pad16(AAD) || ciphertext || pad16(ciphertext) ||
        le64(len(AAD)) || le64(len(ciphertext))

    Parameters
    ----------
    plaintext : bytes
        Data to encrypt.
    key : bytes
        256-bit (32-byte) secret key.
    nonce : bytes
        96-bit (12-byte) nonce. Must be unique per key.
    aad : bytes
        Additional authenticated data (not encrypted, but authenticated).

    Returns
    -------
    tuple[bytes, bytes]
        (ciphertext, tag) where tag is a 16-byte authentication tag.
    """
    if len(key) != 32:
        raise ValueError(f"Key must be 32 bytes, got {len(key)}")
    if len(nonce) != 12:
        raise ValueError(f"Nonce must be 12 bytes, got {len(nonce)}")

    # Step 1: Generate one-time Poly1305 key from ChaCha20 block 0
    poly_key_block = _chacha20_block(key, 0, nonce)
    poly_key = poly_key_block[:32]

    # Step 2: Encrypt plaintext starting at counter=1
    ciphertext = chacha20_encrypt(plaintext, key, nonce, counter=1)

    # Step 3: Construct the MAC input and compute the tag
    #
    # The MAC input binds together:
    #   - The AAD (padded to 16 bytes)
    #   - The ciphertext (padded to 16 bytes)
    #   - The lengths of both (as 64-bit little-endian integers)
    #
    # Including the lengths prevents an attacker from shifting bytes
    # between the AAD and ciphertext while keeping the total the same.
    mac_data = (
        aad + _pad16(aad)
        + ciphertext + _pad16(ciphertext)
        + struct.pack("<Q", len(aad))
        + struct.pack("<Q", len(ciphertext))
    )
    tag = poly1305_mac(mac_data, poly_key)

    return ciphertext, tag


# ---------------------------------------------------------------------------
# AEAD Decryption (RFC 8439 Section 2.8)
# ---------------------------------------------------------------------------

def aead_decrypt(
    ciphertext: bytes,
    key: bytes,
    nonce: bytes,
    aad: bytes,
    tag: bytes,
) -> bytes:
    """Decrypt and verify data using ChaCha20-Poly1305 AEAD.

    This reverses the AEAD encryption:
    1. Recompute the Poly1305 key and expected tag
    2. Compare tags in constant time (well, as constant as Python allows)
    3. Only if the tag matches, decrypt and return the plaintext

    If the tag doesn't match, the ciphertext has been tampered with (or the
    wrong key/nonce/AAD was used), and we raise an error without returning
    any decrypted data. This is essential for security: returning partial
    plaintext from a tampered ciphertext enables chosen-ciphertext attacks.

    Parameters
    ----------
    ciphertext : bytes
        Encrypted data.
    key : bytes
        256-bit (32-byte) secret key.
    nonce : bytes
        96-bit (12-byte) nonce (same one used for encryption).
    aad : bytes
        Additional authenticated data (same as used for encryption).
    tag : bytes
        16-byte authentication tag from encryption.

    Returns
    -------
    bytes
        Decrypted plaintext.

    Raises
    ------
    ValueError
        If the authentication tag doesn't match (data was tampered with).
    """
    if len(key) != 32:
        raise ValueError(f"Key must be 32 bytes, got {len(key)}")
    if len(nonce) != 12:
        raise ValueError(f"Nonce must be 12 bytes, got {len(nonce)}")
    if len(tag) != 16:
        raise ValueError(f"Tag must be 16 bytes, got {len(tag)}")

    # Step 1: Generate one-time Poly1305 key
    poly_key_block = _chacha20_block(key, 0, nonce)
    poly_key = poly_key_block[:32]

    # Step 2: Recompute the expected tag
    mac_data = (
        aad + _pad16(aad)
        + ciphertext + _pad16(ciphertext)
        + struct.pack("<Q", len(aad))
        + struct.pack("<Q", len(ciphertext))
    )
    expected_tag = poly1305_mac(mac_data, poly_key)

    # Step 3: Compare tags
    # We use constant-time comparison to avoid timing side channels.
    # An attacker who can measure how long the comparison takes could
    # learn which bytes of the tag are correct, enabling a byte-by-byte
    # forgery attack. By always comparing all 16 bytes, we leak nothing.
    if not _constant_time_compare(expected_tag, tag):
        raise ValueError("Authentication failed: tag mismatch")

    # Step 4: Decrypt (ChaCha20 with counter=1, same as encryption)
    return chacha20_encrypt(ciphertext, key, nonce, counter=1)


def _constant_time_compare(a: bytes, b: bytes) -> bool:
    """Compare two byte strings in constant time.

    A naive comparison like `a == b` short-circuits on the first differing
    byte, leaking timing information. This function always examines every
    byte, accumulating differences with XOR and OR. The result is True only
    if all bytes are identical.
    """
    if len(a) != len(b):
        return False
    result = 0
    for x, y in zip(a, b, strict=True):
        result |= x ^ y
    return result == 0


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------

__all__ = [
    "chacha20_encrypt",
    "poly1305_mac",
    "aead_encrypt",
    "aead_decrypt",
]
