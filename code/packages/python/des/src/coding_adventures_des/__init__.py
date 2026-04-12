"""
coding_adventures_des — DES and 3DES block cipher (FIPS 46-3 / SP 800-67)

DES (Data Encryption Standard) was published by NIST in 1977 and was the
world's first openly standardized encryption algorithm. It is now completely
broken — a 56-bit key can be exhausted in under 24 hours on consumer hardware
— but it remains a vital historical and educational subject.

This package implements DES for educational purposes only. It should never
be used to protect real data. Its value is in understanding:

  1. Feistel networks — the structural innovation that lets encryption and
     decryption share the same circuit (just reverse the subkey order).
  2. S-boxes — the non-linear heart of DES; hardened by the NSA against
     differential cryptanalysis a decade before that attack was published.
  3. Key schedules — how a single 56-bit key expands into 16 round keys.
  4. Why 56 bits is not enough — the brute-force math that doomed DES.

Architecture
────────────

  plaintext (8 bytes)
       │
  IP (initial permutation)       ← scatters bits for 1970s bus alignment
       │
  ┌── 16 Feistel rounds ─────────────────────────────────────────────┐
  │   L_i = R_{i-1}                                                   │
  │   R_i = L_{i-1} XOR f(R_{i-1}, K_i)                             │
  │                                                                   │
  │   f(R, K):                                                        │
  │     E(R)          32→48 bits (expansion, border bits shared)      │
  │     XOR K_i       48-bit subkey                                   │
  │     S-boxes       8 × (6 bits → 4 bits) = 32 bits out            │
  │     P             32→32 bit permutation                           │
  └───────────────────────────────────────────────────────────────────┘
       │
  FP (final permutation = IP⁻¹)
       │
  ciphertext (8 bytes)

  Decryption is identical — just apply the 16 subkeys in reverse order
  (K16, K15, …, K1). The function f never needs to be inverted.

Public API
──────────

  expand_key(key)              → list of 16 subkeys (each 6 bytes)
  des_encrypt_block(block, key) → 8 bytes
  des_decrypt_block(block, key) → 8 bytes
  des_ecb_encrypt(plaintext, key) → bytes  (PKCS#7 padding)
  des_ecb_decrypt(ciphertext, key) → bytes
  tdea_encrypt_block(block, k1, k2, k3) → 8 bytes  (3DES EDE)
  tdea_decrypt_block(block, k1, k2, k3) → 8 bytes
"""

from __future__ import annotations

__version__ = "0.1.0"
__all__ = [
    "expand_key",
    "des_encrypt_block",
    "des_decrypt_block",
    "des_ecb_encrypt",
    "des_ecb_decrypt",
    "tdea_encrypt_block",
    "tdea_decrypt_block",
]

# ─────────────────────────────────────────────────────────────────────────────
# Permutation and selection tables
# All tables are 1-indexed in the DES standard; we subtract 1 here so they
# can be used directly as Python list indices.
# ─────────────────────────────────────────────────────────────────────────────

# IP — Initial Permutation
# Input bit 58 becomes output bit 1, bit 50 becomes bit 2, etc.
# This was designed for efficient loading on the 8-bit parallel buses of the
# 1970s — it has no cryptographic significance.
_IP = [
    58, 50, 42, 34, 26, 18, 10, 2,
    60, 52, 44, 36, 28, 20, 12, 4,
    62, 54, 46, 38, 30, 22, 14, 6,
    64, 56, 48, 40, 32, 24, 16, 8,
    57, 49, 41, 33, 25, 17,  9, 1,
    59, 51, 43, 35, 27, 19, 11, 3,
    61, 53, 45, 37, 29, 21, 13, 5,
    63, 55, 47, 39, 31, 23, 15, 7,
]

# FP — Final Permutation (IP⁻¹)
# Undoes the initial permutation. FP[IP[i]-1] = i+1.
_FP = [
    40, 8, 48, 16, 56, 24, 64, 32,
    39, 7, 47, 15, 55, 23, 63, 31,
    38, 6, 46, 14, 54, 22, 62, 30,
    37, 5, 45, 13, 53, 21, 61, 29,
    36, 4, 44, 12, 52, 20, 60, 28,
    35, 3, 43, 11, 51, 19, 59, 27,
    34, 2, 42, 10, 50, 18, 58, 26,
    33, 1, 41,  9, 49, 17, 57, 25,
]

# PC-1 — Permuted Choice 1
# Drops the 8 parity bits (positions 8,16,24,32,40,48,56,64) and
# reorders the remaining 56 bits into two 28-bit halves C and D.
_PC1 = [
    57, 49, 41, 33, 25, 17,  9,
     1, 58, 50, 42, 34, 26, 18,
    10,  2, 59, 51, 43, 35, 27,
    19, 11,  3, 60, 52, 44, 36,
    63, 55, 47, 39, 31, 23, 15,
     7, 62, 54, 46, 38, 30, 22,
    14,  6, 61, 53, 45, 37, 29,
    21, 13,  5, 28, 20, 12,  4,
]

# PC-2 — Permuted Choice 2
# Selects 48 of the 56 key bits to form each round subkey.
# The 8 discarded positions (9,18,22,25,35,38,43,54 in the combined C∥D)
# act as a compression step.
_PC2 = [
    14, 17, 11, 24,  1,  5,
     3, 28, 15,  6, 21, 10,
    23, 19, 12,  4, 26,  8,
    16,  7, 27, 20, 13,  2,
    41, 52, 31, 37, 47, 55,
    30, 40, 51, 45, 33, 48,
    44, 49, 39, 56, 34, 53,
    46, 42, 50, 36, 29, 32,
]

# E — Expansion permutation
# Expands the 32-bit right half to 48 bits by copying border bits of each
# 4-bit group into the adjacent 6-bit group. This expansion is what allows
# the 48-bit subkey to mix into every bit position.
_E = [
    32,  1,  2,  3,  4,  5,
     4,  5,  6,  7,  8,  9,
     8,  9, 10, 11, 12, 13,
    12, 13, 14, 15, 16, 17,
    16, 17, 18, 19, 20, 21,
    20, 21, 22, 23, 24, 25,
    24, 25, 26, 27, 28, 29,
    28, 29, 30, 31, 32,  1,
]

# P — Post-S-box permutation
# Disperses the 32-bit S-box output across all bit positions so that each
# round affects every bit of the next round's input.
_P = [
    16,  7, 20, 21, 29, 12, 28, 17,
     1, 15, 23, 26,  5, 18, 31, 10,
     2,  8, 24, 14, 32, 27,  3,  9,
    19, 13, 30,  6, 22, 11,  4, 25,
]

# Left-rotation amounts for the key schedule halves C and D.
# Total across 16 rounds = 28 (one full rotation of a 28-bit register).
# Rounds 1, 2, 9, 16 rotate by 1; all others rotate by 2.
_SHIFTS = [1, 1, 2, 2, 2, 2, 2, 2, 1, 2, 2, 2, 2, 2, 2, 1]

# ─────────────────────────────────────────────────────────────────────────────
# S-Boxes: the core non-linearity of DES
#
# Eight substitution boxes, each mapping 6 bits → 4 bits.
# Without S-boxes, DES would be linear and solvable with Gaussian elimination.
#
# Reading an S-box with 6 input bits b₁b₂b₃b₄b₅b₆:
#   row = 2·b₁ + b₆            (outer bits, range 0–3)
#   col = 8·b₂ + 4·b₃ + 2·b₄ + b₅  (inner bits, range 0–15)
#   output = SBOX[box][row][col]
#
# These S-boxes were redesigned by the NSA from IBM's originals. In 1990,
# Biham and Shamir proved they resist differential cryptanalysis — a technique
# the NSA knew about in 1974 but kept classified. The S-boxes were hardened,
# not backdoored.
# ─────────────────────────────────────────────────────────────────────────────
_SBOXES = [
    # S1
    [
        [14,  4, 13,  1,  2, 15, 11,  8,  3, 10,  6, 12,  5,  9,  0,  7],
        [ 0, 15,  7,  4, 14,  2, 13,  1, 10,  6, 12, 11,  9,  5,  3,  8],
        [ 4,  1, 14,  8, 13,  6,  2, 11, 15, 12,  9,  7,  3, 10,  5,  0],
        [15, 12,  8,  2,  4,  9,  1,  7,  5, 11,  3, 14, 10,  0,  6, 13],
    ],
    # S2
    [
        [15,  1,  8, 14,  6, 11,  3,  4,  9,  7,  2, 13, 12,  0,  5, 10],
        [ 3, 13,  4,  7, 15,  2,  8, 14, 12,  0,  1, 10,  6,  9, 11,  5],
        [ 0, 14,  7, 11, 10,  4, 13,  1,  5,  8, 12,  6,  9,  3,  2, 15],
        [13,  8, 10,  1,  3, 15,  4,  2, 11,  6,  7, 12,  0,  5, 14,  9],
    ],
    # S3
    [
        [10,  0,  9, 14,  6,  3, 15,  5,  1, 13, 12,  7, 11,  4,  2,  8],
        [13,  7,  0,  9,  3,  4,  6, 10,  2,  8,  5, 14, 12, 11, 15,  1],
        [13,  6,  4,  9,  8, 15,  3,  0, 11,  1,  2, 12,  5, 10, 14,  7],
        [ 1, 10, 13,  0,  6,  9,  8,  7,  4, 15, 14,  3, 11,  5,  2, 12],
    ],
    # S4
    [
        [ 7, 13, 14,  3,  0,  6,  9, 10,  1,  2,  8,  5, 11, 12,  4, 15],
        [13,  8, 11,  5,  6, 15,  0,  3,  4,  7,  2, 12,  1, 10, 14,  9],
        [10,  6,  9,  0, 12, 11,  7, 13, 15,  1,  3, 14,  5,  2,  8,  4],
        [ 3, 15,  0,  6, 10,  1, 13,  8,  9,  4,  5, 11, 12,  7,  2, 14],
    ],
    # S5
    [
        [ 2, 12,  4,  1,  7, 10, 11,  6,  8,  5,  3, 15, 13,  0, 14,  9],
        [14, 11,  2, 12,  4,  7, 13,  1,  5,  0, 15, 10,  3,  9,  8,  6],
        [ 4,  2,  1, 11, 10, 13,  7,  8, 15,  9, 12,  5,  6,  3,  0, 14],
        [11,  8, 12,  7,  1, 14,  2, 13,  6, 15,  0,  9, 10,  4,  5,  3],
    ],
    # S6
    [
        [12,  1, 10, 15,  9,  2,  6,  8,  0, 13,  3,  4, 14,  7,  5, 11],
        [10, 15,  4,  2,  7, 12,  9,  5,  6,  1, 13, 14,  0, 11,  3,  8],
        [ 9, 14, 15,  5,  2,  8, 12,  3,  7,  0,  4, 10,  1, 13, 11,  6],
        [ 4,  3,  2, 12,  9,  5, 15, 10, 11, 14,  1,  7,  6,  0,  8, 13],
    ],
    # S7
    [
        [ 4, 11,  2, 14, 15,  0,  8, 13,  3, 12,  9,  7,  5, 10,  6,  1],
        [13,  0, 11,  7,  4,  9,  1, 10, 14,  3,  5, 12,  2, 15,  8,  6],
        [ 1,  4, 11, 13, 12,  3,  7, 14, 10, 15,  6,  8,  0,  5,  9,  2],
        [ 6, 11, 13,  8,  1,  4, 10,  7,  9,  5,  0, 15, 14,  2,  3, 12],
    ],
    # S8
    [
        [13,  2,  8,  4,  6, 15, 11,  1, 10,  9,  3, 14,  5,  0, 12,  7],
        [ 1, 15, 13,  8, 10,  3,  7,  4, 12,  5,  6, 11,  0, 14,  9,  2],
        [ 7, 11,  4,  1,  9, 12, 14,  2,  0,  6, 10, 13, 15,  3,  5,  8],
        [ 2,  1, 14,  7,  4, 10,  8, 13, 15, 12,  9,  0,  3,  5,  6, 11],
    ],
]


# ─────────────────────────────────────────────────────────────────────────────
# Bit manipulation helpers
# ─────────────────────────────────────────────────────────────────────────────

def _bytes_to_bits(data: bytes) -> list[int]:
    """Convert bytes to a list of bits (MSB first within each byte)."""
    bits = []
    for byte in data:
        for i in range(7, -1, -1):
            bits.append((byte >> i) & 1)
    return bits


def _bits_to_bytes(bits: list[int]) -> bytes:
    """Convert a list of bits (MSB first) back to bytes."""
    result = bytearray()
    for i in range(0, len(bits), 8):
        byte = 0
        for j in range(8):
            byte = (byte << 1) | bits[i + j]
        result.append(byte)
    return bytes(result)


def _permute(bits: list[int], table: list[int]) -> list[int]:
    """Apply a permutation table (1-indexed positions) to a bit list."""
    return [bits[pos - 1] for pos in table]


def _left_rotate(half: list[int], n: int) -> list[int]:
    """Left-rotate a 28-bit half of the key register by n positions."""
    return half[n:] + half[:n]


# ─────────────────────────────────────────────────────────────────────────────
# Key schedule: expand_key
# ─────────────────────────────────────────────────────────────────────────────

def expand_key(key: bytes) -> list[bytes]:
    """
    Derive the 16 DES round subkeys from an 8-byte key.

    The DES key is 64 bits wide but only 56 bits are key material — bits at
    positions 8, 16, 24, 32, 40, 48, 56, 64 are parity bits and are dropped
    by PC-1. This function accepts any 8-byte key and ignores parity.

    Returns a list of 16 subkeys, each 6 bytes (48 bits), suitable for use
    with des_encrypt_block / des_decrypt_block.

    Key schedule algorithm:
      1. PC-1: 64-bit key → 56 bits (drop parity), split into C₀ (28) and D₀ (28)
      2. For each round i = 1..16:
           C_i = LeftRotate(C_{i-1}, SHIFTS[i])
           D_i = LeftRotate(D_{i-1}, SHIFTS[i])
           K_i = PC-2(C_i ∥ D_i)   (56 → 48 bits)
    """
    if len(key) != 8:
        raise ValueError(f"DES key must be exactly 8 bytes, got {len(key)}")

    key_bits = _bytes_to_bits(key)
    permuted = _permute(key_bits, _PC1)   # 64 → 56 bits
    c, d = permuted[:28], permuted[28:]   # split into two 28-bit halves

    subkeys: list[bytes] = []
    for shift in _SHIFTS:
        c = _left_rotate(c, shift)
        d = _left_rotate(d, shift)
        subkey_bits = _permute(c + d, _PC2)   # 56 → 48 bits
        subkeys.append(_bits_to_bytes(subkey_bits + [0] * 8))  # pad to 6 bytes (48 bits → 6 bytes, no padding needed since 48 is divisible by 8)

    # Each subkey is exactly 48 bits = 6 bytes — no padding needed.
    # Recompute correctly: 48 bits / 8 = 6 bytes exactly.
    subkeys_clean: list[bytes] = []
    c, d = permuted[:28], permuted[28:]
    for shift in _SHIFTS:
        c = _left_rotate(c, shift)
        d = _left_rotate(d, shift)
        subkey_bits = _permute(c + d, _PC2)   # 48 bits
        subkeys_clean.append(_bits_to_bytes(subkey_bits))
    return subkeys_clean


# ─────────────────────────────────────────────────────────────────────────────
# Round function f(R, K)
# ─────────────────────────────────────────────────────────────────────────────

def _feistel_f(right: list[int], subkey: bytes) -> list[int]:
    """
    DES round function f(R, K):

      1. E(R)   — expand 32-bit right half to 48 bits
      2. XOR    — mix in the 48-bit round subkey
      3. S      — 8 S-boxes, each 6 bits → 4 bits, total 48 → 32 bits
      4. P      — final 32-bit permutation

    The S-boxes are the only non-linear step. Without them, DES would be
    entirely linear and solvable with a system of linear equations over GF(2).
    """
    # Step 1: Expand R from 32 → 48 bits
    expanded = _permute(right, _E)

    # Step 2: XOR with subkey (48 bits)
    subkey_bits = _bytes_to_bits(subkey)
    xored = [expanded[i] ^ subkey_bits[i] for i in range(48)]

    # Step 3: Apply S-boxes (8 × 6-bit → 4-bit substitutions)
    sbox_out: list[int] = []
    for box_idx in range(8):
        chunk = xored[box_idx * 6:(box_idx + 1) * 6]
        # Row = outer bits (first and last)
        row = (chunk[0] << 1) | chunk[5]
        # Col = inner 4 bits
        col = (chunk[1] << 3) | (chunk[2] << 2) | (chunk[3] << 1) | chunk[4]
        val = _SBOXES[box_idx][row][col]
        # Convert 4-bit value to bits (MSB first)
        for bit_pos in range(3, -1, -1):
            sbox_out.append((val >> bit_pos) & 1)

    # Step 4: P permutation
    return _permute(sbox_out, _P)


# ─────────────────────────────────────────────────────────────────────────────
# Core block cipher
# ─────────────────────────────────────────────────────────────────────────────

def _des_block(block: bytes, subkeys: list[bytes]) -> bytes:
    """
    Encrypt or decrypt a single 8-byte block using the provided subkey list.

    Encryption: pass subkeys in order (K1..K16)
    Decryption: pass subkeys in reverse order (K16..K1)

    This is the beauty of the Feistel structure — decryption requires no
    inverse round function, just reversed subkeys. The same hardware
    handles both directions.
    """
    if len(block) != 8:
        raise ValueError(f"DES block must be exactly 8 bytes, got {len(block)}")

    bits = _bytes_to_bits(block)

    # Initial permutation
    bits = _permute(bits, _IP)

    # Split into L₀ and R₀
    left, right = bits[:32], bits[32:]

    # 16 Feistel rounds
    for subkey in subkeys:
        f_out = _feistel_f(right, subkey)
        new_right = [left[i] ^ f_out[i] for i in range(32)]
        left = right
        right = new_right

    # Swap halves before final permutation (standard DES step)
    combined = right + left

    # Final permutation (IP⁻¹)
    return _bits_to_bytes(_permute(combined, _FP))


def des_encrypt_block(block: bytes, key: bytes) -> bytes:
    """
    Encrypt a single 64-bit (8-byte) block using DES.

    Args:
        block: 8 bytes of plaintext
        key:   8 bytes (64 bits, of which 56 are key material; 8 are parity)

    Returns:
        8 bytes of ciphertext

    Note: This is the raw block cipher. For variable-length data, use
    des_ecb_encrypt (ECB mode with PKCS#7 padding) — but ECB mode is
    insecure for most purposes. See SE02 for proper modes (CBC, CTR, GCM).
    """
    subkeys = expand_key(key)
    return _des_block(block, subkeys)


def des_decrypt_block(block: bytes, key: bytes) -> bytes:
    """
    Decrypt a single 64-bit (8-byte) block using DES.

    Decryption is encryption with the subkeys in reverse order — a direct
    consequence of the Feistel structure's self-inverse property.

    Args:
        block: 8 bytes of ciphertext
        key:   8 bytes (same key used for encryption)

    Returns:
        8 bytes of plaintext
    """
    subkeys = expand_key(key)
    return _des_block(block, list(reversed(subkeys)))


# ─────────────────────────────────────────────────────────────────────────────
# ECB mode (educational only)
# ─────────────────────────────────────────────────────────────────────────────

def _pkcs7_pad(data: bytes, block_size: int) -> bytes:
    """
    PKCS#7 padding: append N bytes each with value N, where N is the number
    of bytes needed to reach the next block boundary (1 ≤ N ≤ block_size).

    If the data is already block-aligned, a full padding block is added so
    that unpadding is always unambiguous.

    Example: 5 bytes, block_size=8 → append 3 bytes of value 0x03.
    """
    pad_len = block_size - (len(data) % block_size)
    return data + bytes([pad_len] * pad_len)


def _pkcs7_unpad(data: bytes) -> bytes:
    """
    Remove PKCS#7 padding. Raises ValueError if padding is invalid.
    """
    if not data:
        raise ValueError("Cannot unpad empty data")
    pad_len = data[-1]
    if pad_len == 0 or pad_len > 8:
        raise ValueError(f"Invalid PKCS#7 padding byte: {pad_len}")
    if len(data) < pad_len:
        raise ValueError("Padding length exceeds data length")
    if data[-pad_len:] != bytes([pad_len] * pad_len):
        raise ValueError("Invalid PKCS#7 padding (bytes do not match)")
    return data[:-pad_len]


def des_ecb_encrypt(plaintext: bytes, key: bytes) -> bytes:
    """
    Encrypt variable-length plaintext with DES in ECB mode (PKCS#7 padding).

    WARNING: ECB mode is insecure for most purposes. Identical 8-byte
    plaintext blocks always produce identical ciphertext blocks, leaking
    data patterns. The canonical demonstration is the "ECB penguin": encrypt
    a bitmap in ECB mode and the image structure remains visible in the
    ciphertext. Use CBC or CTR mode (SE02) for real data.

    This function exists for:
      - Compatibility with historical data
      - Educational demonstration of ECB's weakness
      - As a building block to understand why modes of operation exist
    """
    subkeys = expand_key(key)
    padded = _pkcs7_pad(plaintext, 8)
    result = bytearray()
    for i in range(0, len(padded), 8):
        result.extend(_des_block(padded[i:i + 8], subkeys))
    return bytes(result)


def des_ecb_decrypt(ciphertext: bytes, key: bytes) -> bytes:
    """
    Decrypt variable-length ciphertext with DES in ECB mode.

    Args:
        ciphertext: bytes (must be a multiple of 8 bytes)
        key: 8 bytes

    Returns:
        Plaintext with PKCS#7 padding removed
    """
    if len(ciphertext) % 8 != 0:
        raise ValueError("DES ECB ciphertext length must be a multiple of 8 bytes")
    if len(ciphertext) == 0:
        raise ValueError("Ciphertext must not be empty")
    subkeys = list(reversed(expand_key(key)))
    result = bytearray()
    for i in range(0, len(ciphertext), 8):
        result.extend(_des_block(ciphertext[i:i + 8], subkeys))
    return _pkcs7_unpad(bytes(result))


# ─────────────────────────────────────────────────────────────────────────────
# Triple DES (3DES / TDEA)
# ─────────────────────────────────────────────────────────────────────────────

def tdea_encrypt_block(block: bytes, k1: bytes, k2: bytes, k3: bytes) -> bytes:
    """
    Encrypt one 8-byte block with Triple DES (3TDEA / EDE mode).

    Algorithm (NIST SP 800-67): C = E_K1(D_K2(E_K3(P)))

    Applied right-to-left to plaintext:
      1. Encrypt with K3
      2. Decrypt with K2
      3. Encrypt with K1

    The EDE (Encrypt-Decrypt-Encrypt) structure provides backward
    compatibility: if K1 = K2 = K3 = K, then 3DES reduces to single DES:
      E(K, D(K, E(K, P))) = E(K, P)    since D(K, E(K, x)) = x.

    Effective security: ~112 bits (168-bit key reduced by meet-in-the-middle).

    NIST deprecated 3DES for new applications in 2017 and disallowed
    it entirely in 2023 due to the SWEET32 attack on 64-bit block sizes.
    """
    step1 = des_encrypt_block(block, k3)   # E_K3(P)
    step2 = des_decrypt_block(step1, k2)   # D_K2(E_K3(P))
    return des_encrypt_block(step2, k1)    # E_K1(D_K2(E_K3(P)))


def tdea_decrypt_block(block: bytes, k1: bytes, k2: bytes, k3: bytes) -> bytes:
    """
    Decrypt one 8-byte block with Triple DES (3TDEA / EDE mode).

    Algorithm (NIST SP 800-67): P = D_K3(E_K2(D_K1(C)))

    Applied right-to-left to ciphertext:
      1. Decrypt with K1
      2. Encrypt with K2
      3. Decrypt with K3
    """
    step1 = des_decrypt_block(block, k1)   # D_K1(C)
    step2 = des_encrypt_block(step1, k2)   # E_K2(D_K1(C))
    return des_decrypt_block(step2, k3)    # D_K3(E_K2(D_K1(C)))
