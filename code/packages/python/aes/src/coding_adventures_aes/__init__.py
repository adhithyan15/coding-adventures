"""
coding_adventures_aes — AES block cipher (FIPS 197)

AES (Advanced Encryption Standard) is the most widely deployed symmetric
encryption algorithm in the world. Published by NIST in 2001 as FIPS 197,
it replaced DES and is used in TLS/HTTPS, WPA2/WPA3 WiFi, disk encryption
(BitLocker, LUKS, FileVault), VPNs, and virtually every secure protocol.

Designed by Joan Daemen and Vincent Rijmen (Rijndael), AES is a
Substitution-Permutation Network (SPN) — a fundamentally different structure
from DES's Feistel network. All bytes of the state are transformed on every
round, not just half.

This package is educational. It prioritizes clarity and shows the GF(2^8)
mathematics that underlies the algorithm. Production code should use AES-NI
hardware instructions (which this implementation cannot).

Architecture
────────────

  plaintext (16 bytes)
       │
  AddRoundKey(state, round_key[0])       ← XOR with first key material
       │
  ┌── Nr-1 full rounds ──────────────────────────────────────────────┐
  │   SubBytes   — non-linear S-box substitution (GF(2^8) inverse)   │
  │   ShiftRows  — cyclic row shifts (diffusion across columns)       │
  │   MixColumns — GF(2^8) matrix multiply (diffusion across rows)   │
  │   AddRoundKey — XOR with round key                               │
  └───────────────────────────────────────────────────────────────────┘
       │
  SubBytes + ShiftRows + AddRoundKey     ← final round (no MixColumns)
       │
  ciphertext (16 bytes)

The state is a 4×4 matrix of bytes, indexed state[row][col].

GF(2^8) Connection
──────────────────

AES arithmetic lives in GF(2^8) with irreducible polynomial:
  p(x) = x^8 + x^4 + x^3 + x + 1  =  0x11B

This is a different polynomial from Reed-Solomon's 0x11D. We create an
AES-specific field instance:
  _AES_FIELD = GF256Field(0x11B)

The S-box maps each byte to its multiplicative inverse in GF(2^8), followed
by an affine transformation over GF(2). This is the only non-linear step.

Key Sizes and Round Counts
──────────────────────────

  Key size   Nk (words)   Nr (rounds)   Round keys
  128 bits      4             10          11 × 16 bytes
  192 bits      6             12          13 × 16 bytes
  256 bits      8             14          15 × 16 bytes

Public API
──────────

  aes_encrypt_block(block, key) → 16 bytes
  aes_decrypt_block(block, key) → 16 bytes
  expand_key(key)               → list of round-key arrays
  SBOX                          — 256-byte S-box constant
  INV_SBOX                      — 256-byte inverse S-box constant
"""

from __future__ import annotations

from gf256 import GF256Field

__version__ = "0.1.0"
__all__ = [
    "aes_encrypt_block",
    "aes_decrypt_block",
    "expand_key",
    "SBOX",
    "INV_SBOX",
]

# ─────────────────────────────────────────────────────────────────────────────
# AES GF(2^8) field — polynomial 0x11B = x^8 + x^4 + x^3 + x + 1
# This is distinct from the Reed-Solomon polynomial 0x11D used by gf256's
# top-level functions. We create a parameterised instance for AES.
# ─────────────────────────────────────────────────────────────────────────────

_AES_FIELD = GF256Field(0x11B)


# ─────────────────────────────────────────────────────────────────────────────
# S-box and inverse S-box generation
#
# SubBytes maps each byte b to:
#   1. inv = b^{-1} in GF(2^8)   (0 maps to 0)
#   2. affine transformation: s = inv XOR rot(inv,1) XOR rot(inv,2) XOR rot(inv,3) XOR rot(inv,4) XOR 0x63
#      where rot(x, n) is circular left rotation of the byte by n bits.
#
# This two-step design makes the S-box resistant to linear and differential
# cryptanalysis: the GF inverse ensures non-linearity; the affine transform
# eliminates fixed points (no byte maps to itself).
# ─────────────────────────────────────────────────────────────────────────────

def _affine_transform(b: int) -> int:
    """
    AES affine transformation over GF(2):
      s_i = b_i XOR b_{(i+4)%8} XOR b_{(i+5)%8} XOR b_{(i+6)%8} XOR b_{(i+7)%8} XOR c_i
    where c = 0x63 = 01100011.

    Equivalent matrix form (each row of the matrix applied to the bit vector):
      s = M·b XOR c,  M = circulant matrix with first row 11110001.

    Implemented here by rotating the byte and XORing.
    """
    result = 0
    for i in range(8):
        bit = (
            ((b >> i) & 1)
            ^ ((b >> ((i + 4) % 8)) & 1)
            ^ ((b >> ((i + 5) % 8)) & 1)
            ^ ((b >> ((i + 6) % 8)) & 1)
            ^ ((b >> ((i + 7) % 8)) & 1)
            ^ ((0x63 >> i) & 1)
        )
        result |= bit << i
    return result


def _build_sbox() -> tuple[list[int], list[int]]:
    """
    Build the AES S-box and its inverse at module load time.

    For each byte b (0..255):
      - Compute the multiplicative inverse in GF(2^8) with polynomial 0x11B
        (0 has no inverse; it maps to 0)
      - Apply the AES affine transformation

    The inverse S-box is built by inverting: INV_SBOX[SBOX[b]] = b.
    """
    sbox = [0] * 256
    for b in range(256):
        inv = 0 if b == 0 else _AES_FIELD.inverse(b)
        sbox[b] = _affine_transform(inv)
    inv_sbox = [0] * 256
    for b in range(256):
        inv_sbox[sbox[b]] = b
    return sbox, inv_sbox


SBOX: list[int]
INV_SBOX: list[int]
SBOX, INV_SBOX = _build_sbox()


# ─────────────────────────────────────────────────────────────────────────────
# Round constants (Rcon) for the key schedule
#
# Rcon[i] = 2^{i-1} in GF(2^8) for i = 1..10.
# These are the first byte of a 4-byte word [Rcon_i, 0, 0, 0].
# They break symmetry in the key schedule so that no two round keys are equal.
# ─────────────────────────────────────────────────────────────────────────────

# Precomputed: Rcon[1..10] = [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1B, 0x36]
_RCON = [0x00]  # index 0 unused; NIST is 1-indexed
_val = 1
for _ in range(10):
    _RCON.append(_val)
    _val = _AES_FIELD.multiply(_val, 0x02)


# ─────────────────────────────────────────────────────────────────────────────
# MixColumns constants
#
# Each column of the 4×4 state is treated as a polynomial in GF(2^8) and
# multiplied by the fixed AES MixColumns matrix:
#
#   [2 3 1 1]   [s0]
#   [1 2 3 1] × [s1]
#   [1 1 2 3]   [s2]
#   [3 1 1 2]   [s3]
#
# where multiplication is in GF(2^8) with polynomial 0x11B.
# Multiplication by 2 is xtime(b); by 3 is xtime(b) XOR b.
#
# InvMixColumns uses the inverse matrix:
#   [14  11  13   9]
#   [ 9  14  11  13]
#   [13   9  14  11]
#   [11  13   9  14]
# ─────────────────────────────────────────────────────────────────────────────

def _xtime(b: int) -> int:
    """
    Multiply b by x (= 2) in GF(2^8) with AES polynomial 0x11B.
    Equivalent to left-shift by 1, XOR 0x1B if bit 7 was set.
    This is the inner loop of Russian-peasant multiplication for AES.
    """
    return _AES_FIELD.multiply(b, 0x02)


def _mix_col(col: list[int]) -> list[int]:
    """Apply MixColumns to one 4-byte column."""
    s0, s1, s2, s3 = col
    # Each output byte is a GF(2^8) dot product of a matrix row and the column.
    # 2·x = xtime(x), 3·x = xtime(x) XOR x
    t0 = _xtime(s0) ^ (_xtime(s1) ^ s1) ^ s2 ^ s3
    t1 = s0 ^ _xtime(s1) ^ (_xtime(s2) ^ s2) ^ s3
    t2 = s0 ^ s1 ^ _xtime(s2) ^ (_xtime(s3) ^ s3)
    t3 = (_xtime(s0) ^ s0) ^ s1 ^ s2 ^ _xtime(s3)
    return [t0, t1, t2, t3]


def _inv_mix_col(col: list[int]) -> list[int]:
    """Apply InvMixColumns to one 4-byte column."""
    s0, s1, s2, s3 = col
    f = _AES_FIELD.multiply
    # Coefficients: 14=0x0e, 11=0x0b, 13=0x0d, 9=0x09
    t0 = f(0x0e, s0) ^ f(0x0b, s1) ^ f(0x0d, s2) ^ f(0x09, s3)
    t1 = f(0x09, s0) ^ f(0x0e, s1) ^ f(0x0b, s2) ^ f(0x0d, s3)
    t2 = f(0x0d, s0) ^ f(0x09, s1) ^ f(0x0e, s2) ^ f(0x0b, s3)
    t3 = f(0x0b, s0) ^ f(0x0d, s1) ^ f(0x09, s2) ^ f(0x0e, s3)
    return [t0, t1, t2, t3]


# ─────────────────────────────────────────────────────────────────────────────
# Key schedule: expand_key
# ─────────────────────────────────────────────────────────────────────────────

def expand_key(key: bytes) -> list[list[list[int]]]:
    """
    Expand a 16-, 24-, or 32-byte AES key into round keys.

    Returns a list of (Nr+1) round keys, each a 4×4 list of ints.
    The round key at index 0 is used in the initial AddRoundKey;
    round key Nr is used in the final AddRoundKey.

    Key schedule algorithm (FIPS 197 Section 5.2):
      - Nk = key length in 32-bit words (4, 6, or 8)
      - Nr = number of rounds (10, 12, or 14)
      - Total words needed = 4 × (Nr + 1)
      - W[i] = W[i-1] XOR W[i-Nk]  for i not a multiple of Nk
      - W[i] = SubWord(RotWord(W[i-1])) XOR Rcon[i/Nk] XOR W[i-Nk]  when i mod Nk == 0
      - W[i] = SubWord(W[i-1]) XOR W[i-Nk]  when Nk=8 and i mod Nk == 4
    """
    key_len = len(key)
    if key_len not in (16, 24, 32):
        raise ValueError(f"AES key must be 16, 24, or 32 bytes; got {key_len}")

    nk = key_len // 4
    nr = {4: 10, 6: 12, 8: 14}[nk]
    total_words = 4 * (nr + 1)

    # W is a flat list of 4-byte words (each word = list of 4 ints)
    w: list[list[int]] = []
    for i in range(nk):
        w.append(list(key[4 * i:4 * i + 4]))

    for i in range(nk, total_words):
        temp = list(w[i - 1])
        if i % nk == 0:
            # RotWord: left-rotate the 4 bytes
            temp = [temp[1], temp[2], temp[3], temp[0]]
            # SubWord: apply S-box to each byte
            temp = [SBOX[b] for b in temp]
            # XOR with round constant
            temp[0] ^= _RCON[i // nk]
        elif nk == 8 and i % nk == 4:
            # Extra SubWord for AES-256
            temp = [SBOX[b] for b in temp]
        w.append([w[i - nk][j] ^ temp[j] for j in range(4)])

    # Pack into (Nr+1) round keys, each a 4×4 state (column-major)
    round_keys = []
    for rk in range(nr + 1):
        # Each round key is 4 words; arrange as state[row][col]
        rk_words = w[4 * rk:4 * rk + 4]
        state = [[rk_words[col][row] for col in range(4)] for row in range(4)]
        round_keys.append(state)
    return round_keys


# ─────────────────────────────────────────────────────────────────────────────
# State manipulation helpers
# ─────────────────────────────────────────────────────────────────────────────

def _bytes_to_state(block: bytes) -> list[list[int]]:
    """
    Convert 16 bytes to AES state (4×4 column-major matrix).
    state[row][col] = block[row + 4*col].

    AES loads bytes column by column:
      block[0]  block[4]  block[8]  block[12]
      block[1]  block[5]  block[9]  block[13]
      block[2]  block[6]  block[10] block[14]
      block[3]  block[7]  block[11] block[15]
    """
    return [[block[row + 4 * col] for col in range(4)] for row in range(4)]


def _state_to_bytes(state: list[list[int]]) -> bytes:
    """Convert AES state back to 16 bytes (column-major)."""
    return bytes(state[row][col] for col in range(4) for row in range(4))


def _add_round_key(state: list[list[int]], round_key: list[list[int]]) -> list[list[int]]:
    """XOR state with round key (AddRoundKey step)."""
    return [[state[r][c] ^ round_key[r][c] for c in range(4)] for r in range(4)]


def _sub_bytes(state: list[list[int]]) -> list[list[int]]:
    """Replace each byte with its S-box value (SubBytes step)."""
    return [[SBOX[state[r][c]] for c in range(4)] for r in range(4)]


def _inv_sub_bytes(state: list[list[int]]) -> list[list[int]]:
    """Inverse SubBytes — apply inverse S-box."""
    return [[INV_SBOX[state[r][c]] for c in range(4)] for r in range(4)]


def _shift_rows(state: list[list[int]]) -> list[list[int]]:
    """
    Cyclically shift row i left by i positions (ShiftRows step).

    Row 0: no shift
    Row 1: shift left 1
    Row 2: shift left 2
    Row 3: shift left 3

    This ensures that after MixColumns, each output column is a function of
    all four input columns — providing diffusion across the full state.
    """
    return [state[r][r:] + state[r][:r] for r in range(4)]


def _inv_shift_rows(state: list[list[int]]) -> list[list[int]]:
    """Inverse ShiftRows — shift row i right by i positions."""
    return [state[r][-r:] + state[r][:-r] if r else state[r][:] for r in range(4)]


def _mix_columns(state: list[list[int]]) -> list[list[int]]:
    """Apply MixColumns to each of the 4 columns."""
    result = [[0] * 4 for _ in range(4)]
    for col in range(4):
        column = [state[row][col] for row in range(4)]
        mixed = _mix_col(column)
        for row in range(4):
            result[row][col] = mixed[row]
    return result


def _inv_mix_columns(state: list[list[int]]) -> list[list[int]]:
    """Apply InvMixColumns to each of the 4 columns."""
    result = [[0] * 4 for _ in range(4)]
    for col in range(4):
        column = [state[row][col] for row in range(4)]
        mixed = _inv_mix_col(column)
        for row in range(4):
            result[row][col] = mixed[row]
    return result


# ─────────────────────────────────────────────────────────────────────────────
# Core block cipher
# ─────────────────────────────────────────────────────────────────────────────

def aes_encrypt_block(block: bytes, key: bytes) -> bytes:
    """
    Encrypt a single 128-bit (16-byte) block with AES.

    Supports all three key sizes:
      - 16 bytes (AES-128): 10 rounds
      - 24 bytes (AES-192): 12 rounds
      - 32 bytes (AES-256): 14 rounds

    Args:
        block: 16 bytes of plaintext
        key:   16, 24, or 32 bytes

    Returns:
        16 bytes of ciphertext

    Algorithm (FIPS 197 Section 5.1):
      AddRoundKey(state, round_key[0])
      for round = 1 to Nr-1:
        SubBytes → ShiftRows → MixColumns → AddRoundKey
      SubBytes → ShiftRows → AddRoundKey  (final round: no MixColumns)
    """
    if len(block) != 16:
        raise ValueError(f"AES block must be 16 bytes, got {len(block)}")
    round_keys = expand_key(key)
    nr = len(round_keys) - 1

    state = _bytes_to_state(block)
    state = _add_round_key(state, round_keys[0])

    for rnd in range(1, nr):
        state = _sub_bytes(state)
        state = _shift_rows(state)
        state = _mix_columns(state)
        state = _add_round_key(state, round_keys[rnd])

    # Final round: no MixColumns
    state = _sub_bytes(state)
    state = _shift_rows(state)
    state = _add_round_key(state, round_keys[nr])

    return _state_to_bytes(state)


def aes_decrypt_block(block: bytes, key: bytes) -> bytes:
    """
    Decrypt a single 128-bit (16-byte) block with AES.

    Unlike DES (Feistel), AES decryption is not the same circuit as
    encryption — it uses the inverse of each operation, applied in reverse:
    InvShiftRows → InvSubBytes → AddRoundKey → InvMixColumns.

    (Note: AddRoundKey is its own inverse since XOR is self-inverse.)

    Args:
        block: 16 bytes of ciphertext
        key:   16, 24, or 32 bytes (same key used for encryption)

    Returns:
        16 bytes of plaintext
    """
    if len(block) != 16:
        raise ValueError(f"AES block must be 16 bytes, got {len(block)}")
    round_keys = expand_key(key)
    nr = len(round_keys) - 1

    state = _bytes_to_state(block)
    state = _add_round_key(state, round_keys[nr])

    for rnd in range(nr - 1, 0, -1):
        state = _inv_shift_rows(state)
        state = _inv_sub_bytes(state)
        state = _add_round_key(state, round_keys[rnd])
        state = _inv_mix_columns(state)

    # Final round
    state = _inv_shift_rows(state)
    state = _inv_sub_bytes(state)
    state = _add_round_key(state, round_keys[0])

    return _state_to_bytes(state)
