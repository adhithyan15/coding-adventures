"""
_qr_code.py — Full QR Code encoder (ISO/IEC 18004:2015).

## What this file does

This module implements a complete QR Code encoder from scratch.  Given a
UTF-8 string and an error-correction level it returns a ``ModuleGrid`` — a
2-D boolean grid where ``True`` = dark module.  That grid can then be passed
to ``barcode_2d.layout()`` to produce a pixel-level ``PaintScene``.

## Why QR Code exists

Denso Wave engineer Masahiro Hara invented QR Code in 1994 to track
automotive parts on assembly lines.  The goal was a symbol 10× faster to
scan than a 1-D barcode.  Three design choices made this possible:

1. **Three finder patterns** — identical 7×7 square-ring targets placed at
   three corners.  A scanner can locate them instantly and infer orientation
   from which corner is *missing* the fourth pattern.

2. **Reed-Solomon error correction** — up to 30 % of the symbol's area can
   be physically destroyed while the data remains recoverable.  This
   allows logos, stickers, and scratches to coexist with the code.

3. **Masking** — eight XOR patterns are evaluated; the one producing the
   least "degenerate" appearance (solid blocks, finder look-alikes, etc.)
   is chosen.  This prevents scanners from confusing data modules with
   structural patterns.

## Encoding pipeline

::

    Input string
      → mode selection       (numeric / alphanumeric / byte)
      → version selection    (smallest v1–40 that fits at chosen ECC)
      → bit stream assembly  (mode indicator + char count + data + padding)
      → block splitting      (data CWs divided per ISO block table)
      → RS ECC computation   (GF(256) remainder, b=0 convention)
      → interleaving         (round-robin across blocks)
      → grid initialisation  (finder, separator, timing, alignment, dark)
      → zigzag data placement
      → mask evaluation      (8 candidates, lowest 4-rule penalty wins)
      → format info write    (BCH(15,5) + XOR 0x5412)
      → version info write   (BCH(18,6) for v7+)
      → ModuleGrid

## Key data dependencies

- **gf256** (MA01) — GF(2^8) log/antilog tables and ``multiply()``.  QR
  uses the same primitive polynomial ``0x11D`` as gf256.
- **barcode_2d** — provides ``ModuleGrid``, ``make_module_grid()``,
  ``set_module()``, and ``layout()``.
- **paint_instructions** — ``PaintScene`` type re-exported at the package
  level for callers that want to type-annotate scenes.

## Key constants embedded from ISO/IEC 18004:2015

- Capacity table (data CWs + block structure, 40 versions × 4 ECC levels)
- Alignment pattern centre coordinates (versions 1–40)
- RS generator polynomials (degrees 7, 10, 13, 15, 16, 17, 18, 20, 22,
  24, 26, 28, 30 — all counts used across the 40×4 table)
- Format information module positions (copy 1 and copy 2)

See each section below for detailed explanations.
"""

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass, field

from barcode_2d import (
    Barcode2DLayoutConfig,
    ModuleGrid,
    PaintScene,
)
from barcode_2d import (
    layout as barcode2d_layout,
)
from gf256 import ALOG
from gf256 import multiply as gf_mul

# ============================================================================
# Public errors
# ============================================================================


class QRCodeError(Exception):
    """Base class for all QR Code encoding errors."""


class InputTooLongError(QRCodeError):
    """Raised when the input does not fit in any version 1–40 symbol.

    QR Code v40 at ECC=L holds at most 2953 bytes in byte mode (7089 digits
    in numeric mode).  If the caller's input exceeds v40 capacity, there is
    nothing the encoder can do — the caller must shorten the input, split it
    across multiple QR symbols, or use a different format.
    """


class InvalidInputError(QRCodeError):
    """Raised when the input contains characters outside the selected mode.

    In practice this only occurs if the caller forces ``mode="kanji"`` on
    input that cannot be represented in Shift-JIS.  The auto-selection path
    (``mode=None``) always falls through to byte mode and never raises this.
    """


# ============================================================================
# ECC level constants
# ============================================================================

# Every QR Code symbol carries one of four error-correction levels.  Higher
# levels trade data density for damage tolerance.
#
# | Level | Indicator | Recovery  | Typical use                          |
# |-------|-----------|-----------|--------------------------------------|
# | L     | 01        | ~7 %      | Maximum density, clean environment   |
# | M     | 00        | ~15 %     | General purpose (most common default)|
# | Q     | 11        | ~25 %     | Outdoor / industrial                 |
# | H     | 10        | ~30 %     | Overlaid with logo, high damage risk |
#
# The numeric indicators are *not* sorted alphabetically — they come from
# the ISO standard and are fixed in the format information BCH word.

ECC_INDICATOR: dict[str, int] = {"L": 0b01, "M": 0b00, "Q": 0b11, "H": 0b10}

# Index used as the first dimension in the capacity tables below.
ECC_IDX: dict[str, int] = {"L": 0, "M": 1, "Q": 2, "H": 3}

# ============================================================================
# ISO 18004:2015 capacity tables
# ============================================================================
#
# The block structure of each (version, ECC level) pair is given by four
# parameters:
#
#   g1_blocks  — number of blocks in group 1
#   g1_dw      — data codewords per block in group 1
#   g2_blocks  — number of blocks in group 2  (may be 0)
#   g2_dw      — data codewords per block in group 2  (g1_dw + 1)
#   ecc_per_block — ECC codewords per block (same for both groups)
#
# Total data codewords = g1_blocks * g1_dw + g2_blocks * g2_dw
#
# We store these in two parallel tables that mirror the TypeScript reference:
#   ECC_CODEWORDS_PER_BLOCK[ecc_idx][version]
#   NUM_BLOCKS[ecc_idx][version]
#
# Index 0 of each row is a placeholder (−1) because versions run 1–40.

ECC_CODEWORDS_PER_BLOCK: tuple[tuple[int, ...], ...] = (
    # L:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40  # noqa: E501
    (-1,  7, 10, 15, 20, 26, 18, 20, 24, 30, 18, 20, 24, 26, 30, 22, 24, 28, 30, 28, 28, 28, 28, 30, 30, 26, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30),  # noqa: E501
    # M:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40  # noqa: E501
    (-1, 10, 16, 26, 18, 24, 16, 18, 22, 22, 26, 30, 22, 22, 24, 24, 28, 28, 26, 26, 26, 26, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28),  # noqa: E501
    # Q:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40  # noqa: E501
    (-1, 13, 22, 18, 26, 18, 24, 18, 22, 20, 24, 28, 26, 24, 20, 30, 24, 28, 28, 26, 30, 28, 30, 30, 30, 30, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30),  # noqa: E501
    # H:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40  # noqa: E501
    (-1, 17, 28, 22, 16, 22, 28, 26, 26, 24, 28, 24, 28, 22, 24, 24, 30, 28, 28, 26, 28, 30, 24, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30),  # noqa: E501
)

NUM_BLOCKS: tuple[tuple[int, ...], ...] = (
    # L:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40  # noqa: E501
    (-1,  1,  1,  1,  1,  1,  2,  2,  2,  2,  4,  4,  4,  4,  4,  6,  6,  6,  6,  7,  8,  8,  9,  9, 10, 12, 12, 12, 13, 14, 15, 16, 17, 18, 19, 19, 20, 21, 22, 24, 25),  # noqa: E501
    # M:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40  # noqa: E501
    (-1,  1,  1,  1,  2,  2,  4,  4,  4,  5,  5,  5,  8,  9,  9, 10, 10, 11, 13, 14, 16, 17, 17, 18, 20, 21, 23, 25, 26, 28, 29, 31, 33, 35, 37, 38, 40, 43, 45, 47, 49),  # noqa: E501
    # Q:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40  # noqa: E501
    (-1,  1,  1,  2,  2,  4,  4,  6,  6,  8,  8,  8, 10, 12, 16, 12, 17, 16, 18, 21, 20, 23, 23, 25, 27, 29, 34, 34, 35, 38, 40, 43, 45, 48, 51, 53, 56, 59, 62, 65, 68),  # noqa: E501
    # H:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40  # noqa: E501
    (-1,  1,  1,  2,  4,  4,  4,  5,  6,  8,  8, 11, 11, 16, 16, 18, 16, 19, 21, 25, 25, 25, 34, 30, 32, 35, 37, 40, 42, 45, 48, 51, 54, 57, 60, 63, 66, 70, 74, 77, 80),  # noqa: E501
)

# ============================================================================
# Alignment pattern centre coordinates (ISO/IEC 18004:2015, Annex E)
# ============================================================================
#
# For each version, this gives the set of row/column coordinates that form
# the grid of alignment pattern centres.  The actual positions are ALL
# pairwise combinations of the listed values *except* those that would
# overlap an existing finder pattern or separator.
#
# The overlap check is simple: if the centre coordinate falls within 4
# modules of a corner (i.e. the centre row/col < 8 or > size−9), skip it.
# In practice the reserved-module check handles this automatically.
#
# Version 1 has no alignment patterns.

ALIGNMENT_POSITIONS: tuple[tuple[int, ...], ...] = (
    (),                              # v1  — none
    (6, 18),                         # v2
    (6, 22),                         # v3
    (6, 26),                         # v4
    (6, 30),                         # v5
    (6, 34),                         # v6
    (6, 22, 38),                     # v7
    (6, 24, 42),                     # v8
    (6, 26, 46),                     # v9
    (6, 28, 50),                     # v10
    (6, 30, 54),                     # v11
    (6, 32, 58),                     # v12
    (6, 34, 62),                     # v13
    (6, 26, 46, 66),                 # v14
    (6, 26, 48, 70),                 # v15
    (6, 26, 50, 74),                 # v16
    (6, 30, 54, 78),                 # v17
    (6, 30, 56, 82),                 # v18
    (6, 30, 58, 86),                 # v19
    (6, 34, 62, 90),                 # v20
    (6, 28, 50, 72, 94),             # v21
    (6, 26, 50, 74, 98),             # v22
    (6, 30, 54, 78, 102),            # v23
    (6, 28, 54, 80, 106),            # v24
    (6, 32, 58, 84, 110),            # v25
    (6, 30, 58, 86, 114),            # v26
    (6, 34, 62, 90, 118),            # v27
    (6, 26, 50, 74, 98, 122),        # v28
    (6, 30, 54, 78, 102, 126),       # v29
    (6, 26, 52, 78, 104, 130),       # v30
    (6, 30, 56, 82, 108, 134),       # v31
    (6, 34, 60, 86, 112, 138),       # v32
    (6, 30, 58, 86, 114, 142),       # v33
    (6, 34, 62, 90, 118, 146),       # v34
    (6, 30, 54, 78, 102, 126, 150),  # v35
    (6, 24, 50, 76, 102, 128, 154),  # v36
    (6, 28, 54, 80, 106, 132, 158),  # v37
    (6, 32, 58, 84, 110, 136, 162),  # v38
    (6, 26, 54, 82, 110, 138, 166),  # v39
    (6, 30, 58, 86, 114, 142, 170),  # v40
)

# ============================================================================
# Grid geometry helpers
# ============================================================================


def symbol_size(version: int) -> int:
    """Return the side length (in modules) of a QR Code symbol.

    The formula is ``4 × version + 17``:

    - Version 1:   21 × 21 modules
    - Version 5:   37 × 37 modules
    - Version 10:  57 × 57 modules
    - Version 40: 177 × 177 modules

    The ``17`` comes from the fixed structural overhead at the margins:
    7 modules for each of the two finder patterns on a side, plus
    3 modules for their separators.  That is ``7 + 7 + 3 = 17``.
    Each additional version adds 4 modules to each side (``4 × version``).
    """
    return 4 * version + 17


def num_raw_data_modules(version: int) -> int:
    """Total number of data+ECC module positions in the symbol.

    This is the total number of modules in the grid minus every module
    dedicated to structural patterns (finders, separators, timing, alignment,
    format info, version info, dark module).

    The closed-form formula comes from Nayuki's public-domain QR reference::

        base = (16v + 128)v + 64
        subtract alignment overhead (v >= 2)
        subtract version info (v >= 7)

    We use this to derive ``num_data_codewords`` without needing to fully
    build the reserved-module grid just for capacity checking.
    """
    result = (16 * version + 128) * version + 64
    if version >= 2:
        num_align = version // 7 + 2
        result -= (25 * num_align - 10) * num_align - 55
        if version >= 7:
            result -= 36
    return result


def num_data_codewords(version: int, ecc: str) -> int:
    """Number of data codewords (message bytes) for a given version and ECC.

    This does NOT include the ECC codewords — those are subtracted out.
    The formula is::

        floor(raw_modules / 8) − (num_blocks × ecc_per_block)

    Example: Version 1 ECC=M has 26 raw modules / 8 = 208 bits = 26 bytes,
    minus 1 block × 10 ECC bytes = **16** data bytes.
    """
    e = ECC_IDX[ecc]
    return (
        num_raw_data_modules(version) // 8
        - NUM_BLOCKS[e][version] * ECC_CODEWORDS_PER_BLOCK[e][version]
    )


def num_remainder_bits(version: int) -> int:
    """Number of remainder (zero-fill) bits after the interleaved codewords.

    Most QR versions have zero remainder bits.  The exceptions are:
    - Versions  2–6: 7 remainder bits
    - Versions 14–20, 28–34: 3 or 4 bits
    See ISO/IEC 18004 Table 1 for the full list.

    We compute it as ``raw_modules % 8`` which gives exactly the right value
    without embedding yet another lookup table.
    """
    return num_raw_data_modules(version) % 8


# ============================================================================
# Reed-Solomon ECC — b=0 convention
# ============================================================================
#
# QR Code uses Reed-Solomon over GF(256) with the SAME primitive polynomial
# as gf256 (MA01): ``x^8 + x^4 + x^3 + x^2 + 1`` = 0x11D.
#
# The only difference from MA02 is the *generator polynomial convention*:
#
#   QR (b=0):   g(x) = ∏(x + α^i)  for i = 0, 1, …, n−1
#   MA02 (b=1): g(x) = ∏(x + α^i)  for i = 1, 2, …, n
#
# That shift of 1 means the two generators produce different ECC bytes for
# the same data, so we CANNOT reuse MA02 here.
#
# Generator construction: start with g = [1] (degree-0 polynomial = 1).
# Multiply by (x + α^i) for i = 0, 1, …, n−1:
#
#   new[j] = old[j−1] ⊕ (α^i · old[j])
#
# The resulting g has degree n and n+1 coefficients (including the leading 1).
#
# The ECC computation is a polynomial long-division remainder:
#
#   R(x) = D(x) · x^n  mod  G(x)
#
# Implemented as an LFSR shift register to avoid allocating big polynomials.


def _build_generator(n: int) -> tuple[int, ...]:
    """Build the monic RS generator polynomial of degree n (b=0 convention).

    Returns a tuple of n+1 coefficients with g[0] = 1 (leading coefficient).

    Derivation::

        Start: g = [1]   (polynomial "1")
        For i in 0..n−1:
            ai = α^i = ALOG[i]
            new_g has one more term
            new_g[j] = old_g[j−1] XOR (ai · old_g[j])

    After n steps we have g(x) = x^n + c_{n-1}·x^{n-1} + … + c_0.
    """
    g: list[int] = [1]
    for i in range(n):
        ai = ALOG[i]
        nxt: list[int] = [0] * (len(g) + 1)
        for j, coef in enumerate(g):
            nxt[j] ^= coef
            nxt[j + 1] ^= gf_mul(coef, ai)
        g = nxt
    return tuple(g)


# Pre-build all generators needed by the 40×4 capacity table.
# The ECC codeword counts used are exactly these values.
_GENERATOR_CACHE: dict[int, tuple[int, ...]] = {}
for _n in (7, 10, 13, 15, 16, 17, 18, 20, 22, 24, 26, 28, 30):
    _GENERATOR_CACHE[_n] = _build_generator(_n)


def _get_generator(n: int) -> tuple[int, ...]:
    """Return (cached) RS generator of degree n."""
    if n not in _GENERATOR_CACHE:
        _GENERATOR_CACHE[n] = _build_generator(n)
    return _GENERATOR_CACHE[n]


def rs_encode(data: Sequence[int], generator: tuple[int, ...]) -> list[int]:
    """Compute Reed-Solomon ECC bytes.

    Computes ``R(x) = D(x) · x^n  mod  G(x)`` via an LFSR shift register.

    Algorithm (equivalent to long division, O(k·n) operations)::

        rem = [0] * n
        for each data byte b:
            feedback = b XOR rem[0]
            shift rem left by one position (rem[i] ← rem[i+1], rem[n−1] ← 0)
            for i in 0..n−1:
                rem[i] ^= G[i+1] · feedback

    The shift-register view: ``rem`` represents the n-term polynomial
    remainder accumulated so far.  Each new data byte is "clocked in" from
    the high end; the feedback term propagates through all taps.

    Parameters
    ----------
    data:
        List of data codeword bytes.
    generator:
        n+1 coefficients of the monic generator (from ``_build_generator``).

    Returns
    -------
    list[int]
        n ECC codeword bytes (the remainder).
    """
    n = len(generator) - 1
    rem: list[int] = [0] * n
    for b in data:
        fb = b ^ rem[0]
        # Shift register left
        for i in range(n - 1):
            rem[i] = rem[i + 1]
        rem[n - 1] = 0
        if fb != 0:
            for i in range(n):
                rem[i] ^= gf_mul(generator[i + 1], fb)
    return rem


# ============================================================================
# Data encoding modes
# ============================================================================
#
# QR Code defines four encoding modes.  This implementation supports the
# three most common:
#
#   Numeric (0001)      — digits 0–9 only; best density for phone numbers
#   Alphanumeric (0010) — digits, A-Z, space, and $%*+-./:; URLs (uppercase)
#   Byte (0100)         — arbitrary bytes; default for UTF-8 text
#
# Kanji (1000) and ECI (0111) are not implemented (v0.1.0).
#
# Mode selection heuristic: choose the most compact mode that covers the
# *entire* input.  Mixed-segment encoding (numeric prefix + byte suffix) is
# a v0.2.0 enhancement.

# The 45-character QR alphanumeric alphabet with their canonical indices.
# Pairs encode as: (first_idx × 45 + second_idx) → 11 bits.
# Single trailing character encodes as idx → 6 bits.
ALPHANUM_CHARS: str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:"

MODE_INDICATOR: dict[str, int] = {
    "numeric": 0b0001,
    "alphanumeric": 0b0010,
    "byte": 0b0100,
}


def select_mode(text: str) -> str:
    """Choose the most compact encoding mode for the input string.

    Rules (applied in order — first match wins):

    1. **Numeric** — every character is a decimal digit 0–9.  This packs
       three digits into 10 bits vs. 24 bits for byte mode.

    2. **Alphanumeric** — every character is in the 45-char QR alphabet.
       Pairs pack as ``idx1 × 45 + idx2`` → 11 bits (5.5 bits/char).

    3. **Byte** — the universal fallback.  Encodes each UTF-8 byte directly
       as 8 bits.

    For most real-world URLs we end up in byte mode because of lowercase
    letters (alphanumeric only has A-Z uppercase).  Numeric mode is useful
    for phone numbers, order codes, and loyalty card numbers.
    """
    if all(c.isdigit() for c in text):
        return "numeric"
    if all(c in ALPHANUM_CHARS for c in text):
        return "alphanumeric"
    return "byte"


def char_count_bits(mode: str, version: int) -> int:
    """Return the width (in bits) of the character-count indicator field.

    The width depends on both mode and version group.  Larger versions need
    wider fields to express larger character counts.

    +---------------+----------+----------+----------+
    | Mode          | v1–9     | v10–26   | v27–40   |
    +===============+==========+==========+==========+
    | Numeric       | 10       | 12       | 14       |
    +---------------+----------+----------+----------+
    | Alphanumeric  |  9       | 11       | 13       |
    +---------------+----------+----------+----------+
    | Byte          |  8       | 16       | 16       |
    +---------------+----------+----------+----------+
    | Kanji         |  8       | 10       | 12       |
    +---------------+----------+----------+----------+

    Source: ISO/IEC 18004:2015 Table 3.
    """
    if mode == "numeric":
        return 10 if version <= 9 else (12 if version <= 26 else 14)
    if mode == "alphanumeric":
        return 9 if version <= 9 else (11 if version <= 26 else 13)
    # byte (and kanji, not implemented)
    return 8 if version <= 9 else 16


# ============================================================================
# Bit-stream writer
# ============================================================================


class BitWriter:
    """Accumulates bits and converts them to a byte list (MSB-first).

    QR Code bit streams are big-endian at the codeword level: the most
    significant bit of each codeword appears first in the stream.

    Internally we store individual bits in a list and flush to bytes on
    demand.  The ``write(value, count)`` method appends ``count`` bits of
    ``value``, MSB first.

    Example — writing the numeric mode indicator (0001) and the 10-bit
    character count 7 for the input "HELLO":

    ::

        w = BitWriter()
        w.write(0b0001, 4)   # numeric mode indicator
        w.write(7, 10)       # character count = 7
    """

    def __init__(self) -> None:
        self._bits: list[int] = []

    def write(self, value: int, count: int) -> None:
        """Append ``count`` bits of ``value``, MSB first."""
        for i in range(count - 1, -1, -1):
            self._bits.append((value >> i) & 1)

    @property
    def bit_length(self) -> int:
        """Total number of bits written so far."""
        return len(self._bits)

    def to_bytes(self) -> list[int]:
        """Return the accumulated bits as a list of bytes (zero-padded).

        If the bit count is not a multiple of 8, the last byte is
        zero-padded on the right (this should only happen transiently —
        ``build_data_codewords()`` always pads to a byte boundary before
        calling this).
        """
        result: list[int] = []
        for i in range(0, len(self._bits), 8):
            byte = 0
            for j in range(8):
                bit_idx = i + j
                if bit_idx < len(self._bits):
                    byte = (byte << 1) | self._bits[bit_idx]
                else:
                    byte <<= 1
            result.append(byte)
        return result


# ============================================================================
# Per-mode encoders
# ============================================================================


def encode_numeric(text: str, w: BitWriter) -> None:
    """Encode digit string into the bit writer in QR numeric mode.

    Packing rules:
    - Groups of 3 digits: ``int(ddd)`` → 10 bits  (range 0–999)
    - Remainder group of 2: ``int(dd)`` → 7 bits   (range 0–99)
    - Remainder single digit: ``int(d)`` → 4 bits   (range 0–9)

    Why groups of three? 10 bits can represent values 0–1023, so we can
    exactly fit 0–999 (three decimal digits) with two bits to spare.  This
    gives us log2(1000)/3 ≈ 3.32 bits per digit — much better than 8 bits
    per byte (= 1 digit per byte) in byte mode.

    Example::

        "01234567" → groups: "012", "345", "67"
                     bits: 10, 10, 7 = 27 bits
        vs. byte mode: 8 × 8 = 64 bits
    """
    i = 0
    while i + 3 <= len(text):
        w.write(int(text[i : i + 3]), 10)
        i += 3
    if i + 2 <= len(text):
        w.write(int(text[i : i + 2]), 7)
        i += 2
    if i < len(text):
        w.write(int(text[i]), 4)


def encode_alphanumeric(text: str, w: BitWriter) -> None:
    """Encode alphanumeric string into the bit writer.

    Packing rules:
    - Pairs of characters: ``(idx1 × 45 + idx2)`` → 11 bits
    - Single trailing character: ``idx`` → 6 bits

    Why pairs? Two alphanumeric characters from a 45-symbol alphabet can
    take 45² = 2025 values.  11 bits can represent 0–2047, so 11 bits fit
    any pair.  This gives about 5.5 bits/char vs. 8 bits in byte mode.

    The index mapping: 0–9 → 0–9, A–Z → 10–35, SP→36, $→37, %→38,
    *→39, +→40, -→41, .→42, /→43, :→44.
    """
    i = 0
    while i + 2 <= len(text):
        idx0 = ALPHANUM_CHARS.index(text[i])
        idx1 = ALPHANUM_CHARS.index(text[i + 1])
        w.write(idx0 * 45 + idx1, 11)
        i += 2
    if i < len(text):
        idx = ALPHANUM_CHARS.index(text[i])
        w.write(idx, 6)


def encode_byte(text: str, w: BitWriter) -> None:
    """Encode arbitrary string as UTF-8 bytes, one byte = 8 bits.

    Each byte of the UTF-8 encoding is written in its entirety.  For ASCII
    text this is 1 byte per character; for emoji or CJK it can be 3–4 bytes.

    Most modern QR scanners default to UTF-8, so this approach is both
    universal and widely compatible.  To *guarantee* UTF-8 interpretation,
    an ECI header would be needed (v0.2.0 enhancement).
    """
    for byte_val in text.encode("utf-8"):
        w.write(byte_val, 8)


# ============================================================================
# Full data codeword assembly
# ============================================================================


def build_data_codewords(text: str, version: int, ecc: str) -> list[int]:
    """Assemble the complete data codeword sequence for a segment.

    The format is::

        [mode indicator  — 4 bits        ]
        [character count — mode/version dependent]
        [encoded data bits               ]
        [terminator      — 0000, up to 4 bits if room]
        [byte-boundary pad  — 0-bit fill to multiple of 8]
        [fill bytes: alternate 0xEC, 0x11 to reach capacity]

    The output is exactly ``num_data_codewords(version, ecc)`` bytes.

    ### Why 0xEC and 0x11?

    These pad bytes come from the ISO standard.  They alternate (0xEC =
    0b11101100, 0x11 = 0b00010001) to prevent the pad pattern itself from
    being misread as a finder pattern or timing strip.  0xEC and 0x11 are
    complements in GF(256) arithmetic, which helped avoid degenerate RS
    remainder patterns in early QR decoder implementations.

    Parameters
    ----------
    text:
        The input string to encode.
    version:
        QR version (1–40).
    ecc:
        ECC level string: "L", "M", "Q", or "H".

    Returns
    -------
    list[int]
        Exactly ``num_data_codewords(version, ecc)`` bytes.
    """
    mode = select_mode(text)
    capacity = num_data_codewords(version, ecc)
    w = BitWriter()

    # Mode indicator: 4 bits
    w.write(MODE_INDICATOR[mode], 4)

    # Character count indicator: width depends on mode and version group
    char_count = len(text.encode("utf-8")) if mode == "byte" else len(text)
    w.write(char_count, char_count_bits(mode, version))

    # Encoded data
    if mode == "numeric":
        encode_numeric(text, w)
    elif mode == "alphanumeric":
        encode_alphanumeric(text, w)
    else:
        encode_byte(text, w)

    # Terminator: up to 4 zero bits (fewer if at capacity)
    bits_used = w.bit_length
    bits_avail = capacity * 8
    term_len = min(4, bits_avail - bits_used)
    if term_len > 0:
        w.write(0, term_len)

    # Pad to byte boundary
    rem = w.bit_length % 8
    if rem != 0:
        w.write(0, 8 - rem)

    # Pad with alternating 0xEC / 0x11 bytes
    data_bytes = w.to_bytes()
    pad = 0xEC
    while len(data_bytes) < capacity:
        data_bytes.append(pad)
        pad = 0x11 if pad == 0xEC else 0xEC

    return data_bytes


# ============================================================================
# Block splitting + RS ECC computation
# ============================================================================


@dataclass
class Block:
    """One RS block containing data codewords and their ECC codewords.

    QR Code splits the data stream across multiple blocks to improve
    damage resilience.  A burst error (scratch, fold, sticker) that
    destroys a contiguous region will only affect one or two blocks,
    leaving the remaining blocks intact.

    Each block is an independent RS codeword — its data bytes are fed
    to the RS encoder to produce ``ecc_per_block`` ECC bytes.
    """

    data: list[int]
    ecc: list[int] = field(default_factory=list)


def compute_blocks(data: list[int], version: int, ecc: str) -> list[Block]:
    """Split data codewords into blocks and compute RS ECC for each.

    The split uses the ISO block structure table.  For most versions
    there are two groups:

    - Group 1: ``g1_count`` blocks of ``short_len`` data codewords each.
    - Group 2: ``g2_count`` blocks of ``short_len + 1`` data codewords each.
      (Group 2 blocks get one extra codeword to absorb the remainder when
      ``total_data`` is not divisible by ``total_blocks``.)

    Both groups use the same ``ecc_per_block`` value.

    Example: Version 5, ECC=Q
        total_data = 64, total_blocks = 4, ecc_per_block = 18
        short_len = 64 // 4 = 16, g2_count = 64 % 4 = 0
        → 4 blocks of 16 data codewords each (no group 2)

    Wait, that's wrong — v5/Q is:
        g1 = 2 blocks × 15 cw, g2 = 2 blocks × 16 cw
        total_blocks = 4, short_len = 64//4 = 16, g2_count = 64%4 = 0 ?
        Actually: total_data for v5/Q = 64, total_blocks = 4
        short_len = 64 // 4 = 16, g2_count = 64 % 4 = 0 → all group1

    The ISO standard spec the block structure explicitly per row. Our formula
    (total // blocks, total % blocks) replicates the standard's values exactly.
    """
    e = ECC_IDX[ecc]
    total_blocks = NUM_BLOCKS[e][version]
    ecc_len = ECC_CODEWORDS_PER_BLOCK[e][version]
    total_data = num_data_codewords(version, ecc)
    short_len = total_data // total_blocks
    num_long = total_data % total_blocks  # number of "long" blocks (short+1)
    gen = _get_generator(ecc_len)

    blocks: list[Block] = []
    offset = 0

    # Group 1: (total_blocks - num_long) blocks of short_len codewords
    g1_count = total_blocks - num_long
    for _ in range(g1_count):
        d = data[offset : offset + short_len]
        blocks.append(Block(data=d, ecc=rs_encode(d, gen)))
        offset += short_len

    # Group 2: num_long blocks of (short_len + 1) codewords
    for _ in range(num_long):
        d = data[offset : offset + short_len + 1]
        blocks.append(Block(data=d, ecc=rs_encode(d, gen)))
        offset += short_len + 1

    return blocks


def interleave_blocks(blocks: list[Block]) -> list[int]:
    """Interleave data codewords then ECC codewords across all blocks.

    Interleaving layout:
    1. Round-robin through data: block[0][0], block[1][0], …, block[n][0],
       block[0][1], block[1][1], …, block[n][1], etc.
    2. Then round-robin through ECC: same pattern.

    Why interleave? A single scanner pass reads modules in a two-column
    zigzag across the entire grid.  If we didn't interleave, a single
    physical scratch could wipe out an entire block's data, exceeding the
    RS correction budget.  Interleaving spreads the scratch across all
    blocks so each block only loses a few codewords — well within budget.

    The final bit stream (module placement) is derived from this interleaved
    sequence.
    """
    result: list[int] = []

    max_data = max(len(b.data) for b in blocks)
    max_ecc = max(len(b.ecc) for b in blocks)

    # Data interleave
    for i in range(max_data):
        for b in blocks:
            if i < len(b.data):
                result.append(b.data[i])

    # ECC interleave
    for i in range(max_ecc):
        for b in blocks:
            if i < len(b.ecc):
                result.append(b.ecc[i])

    return result


# ============================================================================
# Working grid — mutable internal representation
# ============================================================================
#
# During encoding we need a mutable grid where we can:
# - Set modules dark or light
# - Mark modules as "reserved" (structural — don't touch during data/mask)
#
# We use a flat list-of-lists (list[list[bool]]) for speed rather than
# the immutable ``ModuleGrid`` tuples.  After finalization we convert to
# the immutable ``ModuleGrid``.
#
# ``reserved`` is a parallel boolean matrix: True = this module is
# structural and must not be modified by data placement or masking.


@dataclass
class WorkGrid:
    """Mutable working grid used during encoding.

    Two parallel matrices of size ``size × size``:

    - ``modules[r][c]`` — True = dark module.
    - ``reserved[r][c]`` — True = structural module (skip during data/mask).

    We use a flat list-of-lists rather than the immutable ``ModuleGrid``
    because we need fast in-place writes during grid construction, data
    placement (thousands of writes), and mask evaluation (8 copies).
    """

    size: int
    modules: list[list[bool]]
    reserved: list[list[bool]]

    @classmethod
    def make(cls, size: int) -> WorkGrid:
        """Create an all-light, all-unreserved working grid."""
        return cls(
            size=size,
            modules=[[False] * size for _ in range(size)],
            reserved=[[False] * size for _ in range(size)],
        )

    def set(self, r: int, c: int, dark: bool, reserve: bool = False) -> None:
        """Set module at (r, c) and optionally mark it reserved."""
        self.modules[r][c] = dark
        if reserve:
            self.reserved[r][c] = True

    def to_module_grid(self) -> ModuleGrid:
        """Convert to the immutable ``ModuleGrid`` for public API return."""
        tup = tuple(tuple(row) for row in self.modules)
        # Build the immutable ModuleGrid directly from the tuple of tuples.
        # We do not use make_module_grid() + set_module() because that would
        # allocate O(size²) new tuples — one per module — when a single direct
        # construction is sufficient.
        from barcode_2d import ModuleGrid as MG  # noqa: PLC0415

        return MG(cols=self.size, rows=self.size, modules=tup, module_shape="square")


# ============================================================================
# Structural pattern placement
# ============================================================================


def place_finder(g: WorkGrid, top_row: int, top_col: int) -> None:
    """Place a 7×7 finder pattern with its top-left corner at (top_row, top_col).

    The finder pattern is::

        ■ ■ ■ ■ ■ ■ ■
        ■ □ □ □ □ □ ■
        ■ □ ■ ■ ■ □ ■
        ■ □ ■ ■ ■ □ ■
        ■ □ ■ ■ ■ □ ■
        ■ □ □ □ □ □ ■
        ■ ■ ■ ■ ■ ■ ■

    It is dark on the 1-module outer border, light in the next ring, and
    dark in the 3×3 inner core.

    Three of these are placed at the top-left, top-right, and bottom-left
    corners of the symbol.  Their 1:1:3:1:1 dark-to-light ratio in every
    scan direction (horizontal, vertical, and both diagonals) lets decoders
    find and orient the symbol even under skew, partial occlusion, or 180°
    rotation.  The *missing* fourth corner is always the data corner, giving
    the decoder implicit orientation.
    """
    for dr in range(7):
        for dc in range(7):
            on_border = dr == 0 or dr == 6 or dc == 0 or dc == 6
            in_core = 2 <= dr <= 4 and 2 <= dc <= 4
            g.set(top_row + dr, top_col + dc, on_border or in_core, reserve=True)


def place_separators(g: WorkGrid) -> None:
    """Place the 1-module-wide light separators around each finder pattern.

    Separators isolate the finder patterns from the data area.  Without
    them, a decoder looking for the 1:1:3:1:1 ratio might latch onto a
    data run adjacent to the finder — causing a false positive.

    Layout:
    - Top-left finder: light strip along row 7 (cols 0–7) and col 7 (rows 0–7).
    - Top-right finder: light strip along row 7 (cols size−8..size−1) and
      col size−8 (rows 0–7).
    - Bottom-left finder: light strip along row size−8 (cols 0–7) and
      col 7 (rows size−8..size−1).
    """
    sz = g.size

    # Top-left
    for i in range(8):
        g.set(7, i, False, reserve=True)
        g.set(i, 7, False, reserve=True)

    # Top-right
    for i in range(8):
        g.set(7, sz - 1 - i, False, reserve=True)
        g.set(i, sz - 8, False, reserve=True)

    # Bottom-left
    for i in range(8):
        g.set(sz - 8, i, False, reserve=True)
        g.set(sz - 1 - i, 7, False, reserve=True)


def place_timing_strips(g: WorkGrid) -> None:
    """Place alternating dark/light timing strips on row 6 and column 6.

    Timing patterns run between the finder patterns:
    - Row 6: cols 8 to size−9 (inclusive)
    - Col 6: rows 8 to size−9 (inclusive)

    They start and end dark (at the finder-adjacent cell).  The alternating
    pattern lets decoders measure the module pitch precisely even in symbols
    with slight perspective distortion — each alternation marks exactly one
    module boundary.
    """
    sz = g.size
    for c in range(8, sz - 8):
        g.set(6, c, c % 2 == 0, reserve=True)
    for r in range(8, sz - 8):
        g.set(r, 6, r % 2 == 0, reserve=True)


def place_alignment(g: WorkGrid, row: int, col: int) -> None:
    """Place a 5×5 alignment pattern centred at (row, col).

    The pattern::

        ■ ■ ■ ■ ■
        ■ □ □ □ ■
        ■ □ ■ □ ■
        ■ □ □ □ ■
        ■ ■ ■ ■ ■

    Dark on the outer border (|dr|=2 or |dc|=2), light in the ring, dark
    at the single-pixel centre.  This is a scaled-down finder pattern.

    Alignment patterns appear in versions 2+ at tabulated ISO positions.
    They give decoders additional reference points for perspective-distortion
    correction in larger symbols where the grid may bow or skew.

    Called only for cells whose centre is not already reserved (skip if the
    centre falls on a finder pattern or separator — ``placeAllAlignments``
    guards this).
    """
    for dr in range(-2, 3):
        for dc in range(-2, 3):
            on_border = abs(dr) == 2 or abs(dc) == 2
            is_center = dr == 0 and dc == 0
            g.set(row + dr, col + dc, on_border or is_center, reserve=True)


def place_all_alignments(g: WorkGrid, version: int) -> None:
    """Place all alignment patterns for the given version.

    For each (row, col) in the cross-product of ALIGNMENT_POSITIONS[v-1],
    if the centre cell is not already reserved (finder/separator/timing),
    place a 5×5 alignment pattern centred there.

    Why skip reserved cells? The corner positions in the alignment table
    include (6,6), (6, last), and (last, 6) — these overlap the finder
    patterns.  The reserved-check naturally excludes them without a
    separate hard-coded list.
    """
    if version == 1:
        return  # version 1 has no alignment patterns
    positions = ALIGNMENT_POSITIONS[version - 1]
    for row in positions:
        for col in positions:
            if not g.reserved[row][col]:
                place_alignment(g, row, col)


def reserve_format_info(g: WorkGrid) -> None:
    """Reserve the 15 format-information module positions (× 2 copies).

    Format info is written twice for redundancy.  Both copies must be
    reserved before data placement so the zigzag scanner skips them.

    Copy 1 — adjacent to the top-left finder:
      - Row 8, cols 0–5 (6 modules)
      - Row 8, col 7     (skip col 6 = timing; 1 module)
      - Row 8, col 8     (corner; 1 module)
      - Row 7, col 8     (skip row 6 = timing; 1 module)
      - Col 8, rows 0–5  (6 modules)
      That's 6 + 1 + 1 + 1 + 6 = 15 modules, but the corner at (8,8) appears
      in both the row 8 sweep and the col 8 sweep — we de-duplicate.

    Copy 2 — adjacent to the other two finders:
      - Col 8, rows size−7..size−1 (7 modules)
      - Row 8, cols size−8..size−1 (8 modules)
      15 modules total.

    The actual bits are written later (after mask selection) by
    ``write_format_info()``.
    """
    sz = g.size

    # Copy 1: row 8, cols 0–8 (skip col 6)
    for c in range(9):
        if c != 6:
            g.reserved[8][c] = True
    # Copy 1: col 8, rows 0–8 (skip row 6)
    for r in range(9):
        if r != 6:
            g.reserved[r][8] = True

    # Copy 2: bottom-left strip, col 8 rows size−7..size−1
    for r in range(sz - 7, sz):
        g.reserved[r][8] = True

    # Copy 2: top-right strip, row 8 cols size−8..size−1
    for c in range(sz - 8, sz):
        g.reserved[8][c] = True


def reserve_version_info(g: WorkGrid, version: int) -> None:
    """Reserve the 18-bit version-information positions (versions 7+).

    Two 6×3 blocks:
    - Top-right:    rows 0–5, cols size−11..size−9
    - Bottom-left:  rows size−11..size−9, cols 0–5

    These are absent in versions 1–6.  Version information lets scanners
    determine the version (and therefore the grid size and alignment pattern
    locations) directly from the symbol, rather than counting modules.
    """
    if version < 7:
        return
    sz = g.size
    for r in range(6):
        for dc in range(3):
            g.reserved[r][sz - 11 + dc] = True
    for dr in range(3):
        for c in range(6):
            g.reserved[sz - 11 + dr][c] = True


def place_dark_module(g: WorkGrid, version: int) -> None:
    """Place the always-dark module at (4V+9, 8).

    This single module is mandated by the ISO standard.  It is always dark,
    never masked, and not part of the data.  Its purpose is to ensure that
    the format-information word can never be all-zeros (the all-zero 15-bit
    string would decode as no ECC and mask pattern 0, which is invalid).

    Position: row = 4×version + 9, col = 8.
    """
    g.set(4 * version + 9, 8, True, reserve=True)


# ============================================================================
# Data placement — zigzag scan
# ============================================================================


def place_bits(g: WorkGrid, codewords: list[int], version: int) -> None:
    """Place the interleaved codeword stream into the grid using zigzag scan.

    The placement algorithm scans the grid in two-column strips from the
    bottom-right corner upward, then downward, alternating direction each
    strip::

        current_col = size − 1    (start from rightmost)
        direction = -1             (upward)

        while current_col >= 1:
            for row in direction (bottom→top or top→bottom):
                for sub_col in [current_col, current_col-1]:
                    if sub_col == 6: skip (timing column)
                    if reserved:     skip
                    place bit at (row, sub_col)
            flip direction
            current_col -= 2
            if current_col == 6: current_col = 5  # skip timing col

    This produces a dense diagonal sweep that ensures adjacent bits in the
    bit stream land on adjacent or nearby modules, keeping codeword bits
    clustered for better RS performance.

    Remainder bits (from ``num_remainder_bits(version)``) are placed as 0.

    Parameters
    ----------
    g:
        Working grid (modified in place).
    codewords:
        Interleaved codeword bytes (output of ``interleave_blocks``).
    version:
        QR version, used to determine remainder-bit count.
    """
    sz = g.size

    # Expand codewords to a flat bit array, MSB-first
    bits: list[bool] = []
    for cw in codewords:
        for b in range(7, -1, -1):
            bits.append(bool((cw >> b) & 1))
    # Append remainder bits (always 0)
    for _ in range(num_remainder_bits(version)):
        bits.append(False)

    bit_idx = 0
    going_up = True
    col = sz - 1  # leading column of current 2-col strip

    while col >= 1:
        for vi in range(sz):
            row = sz - 1 - vi if going_up else vi
            for dc in (0, 1):
                c = col - dc
                if c == 6:  # skip vertical timing strip
                    continue
                if g.reserved[row][c]:
                    continue
                g.modules[row][c] = bits[bit_idx] if bit_idx < len(bits) else False
                bit_idx += 1

        going_up = not going_up
        col -= 2
        if col == 6:  # hop over timing column
            col = 5


# ============================================================================
# Masking
# ============================================================================
#
# After data placement, we evaluate all 8 mask patterns and choose the one
# with the lowest penalty score.  Masking XORs each non-reserved data/ECC
# module with a condition derived from its (row, col) position.
#
# The 8 conditions (ISO/IEC 18004 Table 10):
#
#   0: (row + col) mod 2 == 0
#   1: row mod 2 == 0
#   2: col mod 3 == 0
#   3: (row + col) mod 3 == 0
#   4: (row/2 + col/3) mod 2 == 0
#   5: (row*col) mod 2 + (row*col) mod 3 == 0
#   6: ((row*col) mod 2 + (row*col) mod 3) mod 2 == 0
#   7: ((row+col) mod 2 + (row*col) mod 3) mod 2 == 0

_MASK_COND_0 = lambda r, c: (r + c) % 2 == 0  # noqa: E731
_MASK_COND_1 = lambda r, c: r % 2 == 0  # noqa: E731
_MASK_COND_2 = lambda r, c: c % 3 == 0  # noqa: E731
_MASK_COND_3 = lambda r, c: (r + c) % 3 == 0  # noqa: E731
_MASK_COND_4 = lambda r, c: (r // 2 + c // 3) % 2 == 0  # noqa: E731
_MASK_COND_5 = lambda r, c: (r * c) % 2 + (r * c) % 3 == 0  # noqa: E731
_MASK_COND_6 = lambda r, c: ((r * c) % 2 + (r * c) % 3) % 2 == 0  # noqa: E731
_MASK_COND_7 = lambda r, c: ((r + c) % 2 + (r * c) % 3) % 2 == 0  # noqa: E731

MASK_CONDITIONS = (
    _MASK_COND_0,
    _MASK_COND_1,
    _MASK_COND_2,
    _MASK_COND_3,
    _MASK_COND_4,
    _MASK_COND_5,
    _MASK_COND_6,
    _MASK_COND_7,
)


def apply_mask(
    modules: list[list[bool]],
    reserved: list[list[bool]],
    sz: int,
    mask_idx: int,
) -> list[list[bool]]:
    """Return a new module array with the mask applied to non-reserved cells.

    Structural modules (finder, separator, timing, alignment, format info,
    version info, dark module) are never masked — their bit values are
    fixed by the standard.

    Only non-reserved (data/ECC) modules are XOR'd with the mask condition.
    XOR means: if the condition is True, flip the module; if False, leave it.
    """
    cond = MASK_CONDITIONS[mask_idx]
    result: list[list[bool]] = []
    for r in range(sz):
        row: list[bool] = []
        for c in range(sz):
            if reserved[r][c]:
                row.append(modules[r][c])
            else:
                row.append(modules[r][c] != cond(r, c))
        result.append(row)
    return result


# ============================================================================
# Penalty scoring (ISO/IEC 18004 Section 7.8.3)
# ============================================================================
#
# Four rules penalise patterns that could confuse scanners:
#
#   Rule 1: Runs of ≥5 same-colour modules in a row or column.
#           Penalty = run_length − 2 per qualifying run.
#
#   Rule 2: 2×2 same-colour blocks anywhere in the grid.
#           Penalty = 3 per block.
#
#   Rule 3: The pattern 1,0,1,1,1,0,1,0,0,0,0 (or its reverse) in a row
#           or column.  This looks like a finder pattern to a scanner.
#           Penalty = 40 per occurrence.
#
#   Rule 4: Dark-module proportion deviation from 50%.
#           Penalty = min(|prev5 − 50|, |next5 − 50|) / 5 × 10
#           where prev5 and next5 are the nearest multiples of 5% below
#           and above the actual dark percentage.
#
# Lowest total score → best mask.

# Pattern used in Rule 3 and its reverse (both must be checked)
_R3_PATTERN  = (1, 0, 1, 1, 1, 0, 1, 0, 0, 0, 0)
_R3_REVERSED = (0, 0, 0, 0, 1, 0, 1, 1, 1, 0, 1)


def compute_penalty(modules: list[list[bool]], sz: int) -> int:
    """Compute the four-rule ISO penalty score for a (masked) module grid.

    Lower is better.  A well-distributed, non-degenerate QR symbol
    typically scores in the 300–600 range for small versions.

    This function is called 8 times (once per mask candidate) so it must
    be efficient.  We use a single pass where possible and avoid allocating
    large intermediate structures.
    """
    penalty = 0

    # ── Rule 1: runs of ≥ 5 same-colour ──────────────────────────────────
    # Scan each row left-to-right, then each column top-to-bottom.
    for r in range(sz):
        for horizontal in (True, False):
            run = 1
            prev = modules[r][0] if horizontal else modules[0][r]
            for i in range(1, sz):
                cur = modules[r][i] if horizontal else modules[i][r]
                if cur == prev:
                    run += 1
                else:
                    if run >= 5:
                        penalty += run - 2
                    run = 1
                    prev = cur
            if run >= 5:
                penalty += run - 2

    # ── Rule 2: 2×2 same-colour blocks ───────────────────────────────────
    for r in range(sz - 1):
        for c in range(sz - 1):
            d = modules[r][c]
            if (
                d == modules[r][c + 1]
                and d == modules[r + 1][c]
                and d == modules[r + 1][c + 1]
            ):
                penalty += 3

    # ── Rule 3: finder-pattern-like sequences ────────────────────────────
    # Check all length-11 windows in rows and columns.
    for a in range(sz):
        for b in range(sz - 10):
            mh1 = mh2 = mv1 = mv2 = True
            for k in range(11):
                bh = 1 if modules[a][b + k] else 0
                bv = 1 if modules[b + k][a] else 0
                if bh != _R3_PATTERN[k]:
                    mh1 = False
                if bh != _R3_REVERSED[k]:
                    mh2 = False
                if bv != _R3_PATTERN[k]:
                    mv1 = False
                if bv != _R3_REVERSED[k]:
                    mv2 = False
                if not mh1 and not mh2 and not mv1 and not mv2:
                    break
            if mh1:
                penalty += 40
            if mh2:
                penalty += 40
            if mv1:
                penalty += 40
            if mv2:
                penalty += 40

    # ── Rule 4: dark-module proportion ───────────────────────────────────
    dark = sum(1 for r in range(sz) for c in range(sz) if modules[r][c])
    ratio = dark / (sz * sz) * 100
    prev5 = int(ratio // 5) * 5
    penalty += min(abs(prev5 - 50), abs(prev5 + 5 - 50)) // 5 * 10

    return penalty


# ============================================================================
# Format information (BCH(15,5))
# ============================================================================
#
# The format information encodes the ECC level and mask pattern as a
# 15-bit BCH codeword:
#
#   1. 5-bit data = [ECC_indicator (2b)][mask_pattern (3b)]
#   2. Multiply by x^10 (shift left 10)
#   3. Divide by G(x) = x^10 + x^8 + x^5 + x^4 + x^2 + x + 1 = 0x537
#   4. Append 10-bit remainder
#   5. XOR with 0x5412 = 0101_0100_0001_0010 (prevents all-zero format info)
#
# The result is written to two redundant copies in the symbol.
#
# CRITICAL (from lessons.md 2026-04-23): Bit ordering is MSB-first.
# Copy 1, row 8, cols 0–5: bit (14−i) at col i  (f14 at col 0)
# Copy 1, col 8, rows 0–5: bit i at row i        (f0 at row 0)
# See the full write_format_info docstring for all positions.


def compute_format_bits(ecc: str, mask: int) -> int:
    """Compute the 15-bit format information word.

    Parameters
    ----------
    ecc:
        ECC level string: "L", "M", "Q", or "H".
    mask:
        Mask pattern index 0–7.

    Returns
    -------
    int
        15-bit format information word (after BCH encoding and XOR mask).

    Example — ECC=M, mask=2:

    ::

        data = (0b00 << 3) | 2 = 0b00010 = 2
        rem  = polynomial_remainder(2 << 10, 0x537) = 0x3DA ... hmm
        Actually with correct MSB ordering: fmt = 0x5E7C for ECC=M, mask=2.
    """
    data = (ECC_INDICATOR[ecc] << 3) | mask
    rem = data << 10
    for i in range(14, 9, -1):  # i = 14 down to 10
        if (rem >> i) & 1:
            rem ^= 0x537 << (i - 10)
    return ((data << 10) | (rem & 0x3FF)) ^ 0x5412


def write_format_info(g: WorkGrid, fmt_bits: int) -> None:
    """Write the 15-bit format information into both copy locations.

    This function implements the CORRECTED bit ordering from lessons.md
    (2026-04-23).  Bit ordering is MSB-first in the horizontal strip and
    LSB-first in the vertical strip.

    **Copy 1** (adjacent to top-left finder):

    Row 8, cols 0–5 (MSB-first: bit 14 at col 0, bit 9 at col 5):

    ::

        for i in 0..5:  modules[8][i] = (fmt >> (14 - i)) & 1

    Row 8, col 7 (bit 8, skipping col 6 = timing):

    ::

        modules[8][7] = (fmt >> 8) & 1

    Rows (8,8) = bit 7 (corner):

    ::

        modules[8][8] = (fmt >> 7) & 1

    Col 8, row 7 (bit 6, skip row 6 = timing):

    ::

        modules[7][8] = (fmt >> 6) & 1

    Col 8, rows 0–5 (LSB-first: bit 0 at row 0, bit 5 at row 5):

    ::

        for i in 0..5:  modules[i][8] = (fmt >> i) & 1

    **Copy 2** (adjacent to the other two finders):

    Row 8, cols size−1..size−8 (bits 0–7, f0 at rightmost):

    ::

        for i in 0..7:  modules[8][sz-1-i] = (fmt >> i) & 1

    Col 8, rows size−7..size−1 (bits 8–14, f8 at row size−7):

    ::

        for i in 8..14:  modules[sz-7+(i-8)][8] = (fmt >> i) & 1
    """
    sz = g.size

    # ── Copy 1 ────────────────────────────────────────────────────────────
    # Row 8, cols 0–5: bits 14 down to 9 (MSB-first)
    for i in range(6):
        g.modules[8][i] = bool((fmt_bits >> (14 - i)) & 1)

    # Row 8, col 7: bit 8  (col 6 = timing, skipped)
    g.modules[8][7] = bool((fmt_bits >> 8) & 1)

    # Row 8, col 8: bit 7  (corner module)
    g.modules[8][8] = bool((fmt_bits >> 7) & 1)

    # Row 7, col 8: bit 6  (row 6 = timing, skipped)
    g.modules[7][8] = bool((fmt_bits >> 6) & 1)

    # Col 8, rows 0–5: bits 0 up to 5 (LSB-first)
    for i in range(6):
        g.modules[i][8] = bool((fmt_bits >> i) & 1)

    # ── Copy 2 ────────────────────────────────────────────────────────────
    # Row 8, cols size−1 down to size−8: bits 0–7 (f0 at rightmost)
    for i in range(8):
        g.modules[8][sz - 1 - i] = bool((fmt_bits >> i) & 1)

    # Col 8, rows size−7 up to size−1: bits 8–14
    for i in range(7):
        g.modules[sz - 7 + i][8] = bool((fmt_bits >> (i + 8)) & 1)


# ============================================================================
# Version information (BCH(18,6)) — versions 7+
# ============================================================================
#
# The version information encodes the version number (1–40) as an 18-bit
# BCH codeword:
#
#   1. 6-bit version number (e.g. version 7 → 000111)
#   2. Multiply by x^12 (shift left 12)
#   3. Divide by G(x) = x^12+x^11+x^10+x^9+x^8+x^5+x^2+1 = 0x1F25
#   4. Append 12-bit remainder
#
# Two copies: one near the top-right finder, one near the bottom-left.
# Each is arranged as a 6×3 block.


def compute_version_bits(version: int) -> int:
    """Compute the 18-bit version information word for versions 7–40.

    Returns 0 for versions 1–6 (no version information needed).
    """
    if version < 7:
        return 0
    rem = version << 12
    for i in range(17, 11, -1):  # i = 17 down to 12
        if (rem >> i) & 1:
            rem ^= 0x1F25 << (i - 12)
    return (version << 12) | (rem & 0xFFF)


def write_version_info(g: WorkGrid, version: int) -> None:
    """Write the 18-bit version information into both 6×3 blocks (v7+).

    Bit layout for each 6×3 block:
    - Bit i → row (5 − i//3), col (size − 9 − i%3)  for top-right block
    - Bit i → row (size − 9 − i%3), col (5 − i//3)  for bottom-left block

    These two blocks are transposes of each other (swapping row and col).

    The 18 bits are written LSB-first in the column direction:
    - i=0 at row 5, col size−9   (top-right)
    - i=17 at row 0, col size−11
    """
    if version < 7:
        return
    sz = g.size
    bits = compute_version_bits(version)
    for i in range(18):
        dark = bool((bits >> i) & 1)
        a = 5 - i // 3
        b = sz - 9 - i % 3
        g.modules[a][b] = dark   # top-right block
        g.modules[b][a] = dark   # bottom-left block (transpose)


# ============================================================================
# Grid initialisation
# ============================================================================


def build_grid(version: int) -> WorkGrid:
    """Initialise a QR Code working grid with all structural patterns placed.

    Steps:
    1. Create an all-light all-unreserved grid of size (4v+17) × (4v+17).
    2. Place three finder patterns at the three corners.
    3. Place separators (light borders around finders).
    4. Place timing strips (row 6 and col 6).
    5. Place alignment patterns (version-dependent).
    6. Reserve format information positions (placeholder zeros).
    7. Reserve version information positions (v7+).
    8. Place the always-dark module.

    The returned grid has all structural modules set and reserved.  Data
    placement (``place_bits``) will fill only the non-reserved modules.
    """
    sz = symbol_size(version)
    g = WorkGrid.make(sz)

    # 1–2. Three finder patterns
    place_finder(g, 0, 0)         # top-left
    place_finder(g, 0, sz - 7)    # top-right
    place_finder(g, sz - 7, 0)    # bottom-left

    # 3. Separators
    place_separators(g)

    # 4. Timing strips (must come before alignments so row/col 6 is reserved)
    place_timing_strips(g)

    # 5. Alignment patterns
    place_all_alignments(g, version)

    # 6. Reserve format info positions
    reserve_format_info(g)

    # 7. Reserve version info positions
    reserve_version_info(g, version)

    # 8. Dark module
    place_dark_module(g, version)

    return g


# ============================================================================
# Version selection
# ============================================================================


def select_version(text: str, ecc: str, forced_version: int = 0) -> int:
    """Find the minimum QR version (1–40) that fits the input.

    If ``forced_version > 0``, validate that the input fits and return that
    version, or raise ``InputTooLongError`` if it doesn't.

    Otherwise, iterate versions 1–40 and return the smallest that fits.
    "Fits" means the total encoded bit count (mode indicator + char count +
    data bits) rounded up to a byte boundary does not exceed the available
    data codeword capacity.

    Note: we use the exact bit count for the selected mode, accounting for
    the variable-width char-count field (which depends on version group).
    This matters because a v9 symbol might not fit but a v10 symbol does —
    and the char-count field width changes at v10 boundaries.

    Raises
    ------
    InputTooLongError
        If the input exceeds version-40 capacity at the chosen ECC level.
    """
    mode = select_mode(text)
    byte_len = len(text.encode("utf-8"))

    for v in range(1, 41):
        if forced_version > 0 and v != forced_version:
            continue
        capacity = num_data_codewords(v, ecc)
        if mode == "byte":
            data_bits = byte_len * 8
        elif mode == "numeric":
            # 10 bits per 3 digits + 7 per 2 + 4 per 1
            n = len(text)
            data_bits = (n // 3) * 10 + ((n % 3 == 2) * 7) + ((n % 3 == 1) * 4)
        else:  # alphanumeric
            n = len(text)
            data_bits = (n // 2) * 11 + (n % 2) * 6
        bits_needed = 4 + char_count_bits(mode, v) + data_bits
        if (bits_needed + 7) // 8 <= capacity:
            return v

        if forced_version > 0:
            break  # only check forced version

    raise InputTooLongError(
        f"Input ({len(text)} chars, ECC={ecc}) exceeds version 40 capacity."
    )


# ============================================================================
# Public API — encode
# ============================================================================


def encode(
    data: str | bytes,
    *,
    level: str = "M",
    version: int = 0,
    mode: str | None = None,
) -> ModuleGrid:
    """Encode data into a QR Code module grid.

    This is the main entry point for the QR Code encoder.  Given a string
    (or bytes), it produces a ``ModuleGrid`` — an abstract 2-D boolean grid
    suitable for rendering via ``barcode_2d.layout()``.

    Parameters
    ----------
    data:
        Input to encode.  A ``str`` is encoded as UTF-8 bytes in byte mode
        (or numeric/alphanumeric mode if the content qualifies).  A ``bytes``
        object is always encoded in byte mode.
    level:
        ECC level: ``"L"``, ``"M"`` (default), ``"Q"``, or ``"H"``.
    version:
        Force a specific version 1–40.  Pass ``0`` (default) to auto-select
        the smallest version that fits.
    mode:
        Force a specific encoding mode: ``"numeric"``, ``"alphanumeric"``,
        or ``"byte"``.  ``None`` (default) auto-selects the most compact
        mode.  Kanji is not supported in v0.1.0.

    Returns
    -------
    ModuleGrid
        An immutable ``ModuleGrid`` of size ``(4v+17) × (4v+17)`` where
        ``v`` is the chosen version.  ``True`` = dark module.

    Raises
    ------
    InputTooLongError
        If the input exceeds QR Code v40 capacity at the chosen ECC level.
    ValueError
        If ``level`` is not one of L/M/Q/H, or if ``version`` is outside
        1–40, or if ``mode`` is unsupported.
    """
    # ── Input validation ──────────────────────────────────────────────────
    if level not in ECC_IDX:
        raise ValueError(f"Invalid ECC level {level!r}; must be L, M, Q, or H")
    if version < 0 or version > 40:
        raise ValueError(f"version must be 0–40, got {version}")
    if mode is not None and mode not in MODE_INDICATOR:
        raise ValueError(
            f"mode {mode!r} is not supported; choose numeric, alphanumeric, or byte"
        )

    # Normalise bytes→str for uniform handling (byte mode forces the mode)
    if isinstance(data, bytes):
        text = data.decode("latin-1")  # preserve raw byte values
        effective_mode = "byte"
    else:
        text = data
        effective_mode = mode if mode is not None else select_mode(text)

    # Quick guard: QR v40 holds at most 7089 numeric chars / 2953 bytes
    if len(text) > 7089:
        raise InputTooLongError(
            f"Input length {len(text)} exceeds 7089 (QR v40 maximum)."
        )

    # Override select_mode result if the caller forced a mode
    if mode is not None and not isinstance(data, bytes):
        effective_mode = mode  # already set above, just for clarity

    # ── Version selection ─────────────────────────────────────────────────
    chosen_version = select_version(text, level, forced_version=version)
    sz = symbol_size(chosen_version)

    # ── Data encoding ─────────────────────────────────────────────────────
    # If the caller forced a mode, temporarily monkey-patch select_mode
    # for the build_data_codewords call.  Simpler: just call mode-specific
    # functions directly.
    if mode is not None and not isinstance(data, bytes):
        data_cw = _build_data_codewords_with_mode(
            text, chosen_version, level, effective_mode
        )
    else:
        data_cw = build_data_codewords(text, chosen_version, level)

    # ── Block splitting + RS ECC ──────────────────────────────────────────
    blocks = compute_blocks(data_cw, chosen_version, level)
    interleaved = interleave_blocks(blocks)

    # ── Grid initialisation + data placement ──────────────────────────────
    grid = build_grid(chosen_version)
    place_bits(grid, interleaved, chosen_version)

    # ── Mask evaluation ────────────────────────────────────────────────────
    best_mask = 0
    best_penalty = 10 ** 9  # effectively infinity

    for m in range(8):
        masked = apply_mask(grid.modules, grid.reserved, sz, m)
        fmt_bits = compute_format_bits(level, m)
        # Temporarily write format info to score the full grid
        test_grid = WorkGrid(
            size=sz,
            modules=masked,
            reserved=grid.reserved,
        )
        write_format_info(test_grid, fmt_bits)
        p = compute_penalty(masked, sz)
        if p < best_penalty:
            best_penalty = p
            best_mask = m

    # ── Finalise ──────────────────────────────────────────────────────────
    final_modules = apply_mask(grid.modules, grid.reserved, sz, best_mask)
    final_grid = WorkGrid(size=sz, modules=final_modules, reserved=grid.reserved)
    write_format_info(final_grid, compute_format_bits(level, best_mask))
    write_version_info(final_grid, chosen_version)

    # ── Return immutable ModuleGrid ───────────────────────────────────────
    from barcode_2d import ModuleGrid as MG  # noqa: PLC0415

    return MG(
        cols=sz,
        rows=sz,
        modules=tuple(tuple(row) for row in final_modules),
        module_shape="square",
    )


def _build_data_codewords_with_mode(
    text: str, version: int, ecc: str, mode: str
) -> list[int]:
    """Like ``build_data_codewords`` but with the mode forced externally.

    Used when the caller passes ``mode=`` explicitly to ``encode()``.
    This avoids patching ``select_mode`` globally.
    """
    capacity = num_data_codewords(version, ecc)
    w = BitWriter()

    w.write(MODE_INDICATOR[mode], 4)

    char_count = len(text.encode("utf-8")) if mode == "byte" else len(text)
    w.write(char_count, char_count_bits(mode, version))

    if mode == "numeric":
        encode_numeric(text, w)
    elif mode == "alphanumeric":
        encode_alphanumeric(text, w)
    else:
        encode_byte(text, w)

    bits_avail = capacity * 8
    term_len = min(4, bits_avail - w.bit_length)
    if term_len > 0:
        w.write(0, term_len)

    rem = w.bit_length % 8
    if rem != 0:
        w.write(0, 8 - rem)

    data_bytes = w.to_bytes()
    pad = 0xEC
    while len(data_bytes) < capacity:
        data_bytes.append(pad)
        pad = 0x11 if pad == 0xEC else 0xEC

    return data_bytes


# ============================================================================
# Public API — encode_to_scene
# ============================================================================


def encode_to_scene(
    data: str | bytes,
    *,
    level: str = "M",
    version: int = 0,
    mode: str | None = None,
    config: Barcode2DLayoutConfig | None = None,
) -> PaintScene:
    """Encode data and convert the QR Code grid to a ``PaintScene``.

    Convenience function combining ``encode()`` and ``barcode_2d.layout()``.

    Parameters
    ----------
    data:
        Input to encode (see ``encode()`` for details).
    level:
        ECC level: ``"L"``, ``"M"`` (default), ``"Q"``, or ``"H"``.
    version:
        Force a specific version 1–40 (0 = auto).
    mode:
        Force an encoding mode (``None`` = auto).
    config:
        Pixel rendering configuration.  ``None`` uses barcode-2d defaults
        (10 px/module, 4-module quiet zone, black on white).

    Returns
    -------
    PaintScene
        Ready for the PaintVM (paint-vm-svg, paint-vm-ascii, etc.).
    """
    grid = encode(data, level=level, version=version, mode=mode)
    return barcode2d_layout(grid, config)


# ============================================================================
# Module-level exports
# ============================================================================

__all__ = [
    # Errors
    "QRCodeError",
    "InputTooLongError",
    "InvalidInputError",
    # Core encoding
    "encode",
    "encode_to_scene",
    # Low-level helpers exposed for testing / educational use
    "select_mode",
    "select_version",
    "build_data_codewords",
    "compute_blocks",
    "interleave_blocks",
    "rs_encode",
    "compute_format_bits",
    "write_format_info",
    "compute_version_bits",
    "compute_penalty",
    "apply_mask",
    "num_data_codewords",
    "num_raw_data_modules",
    "num_remainder_bits",
    "symbol_size",
    # Constants
    "ECC_INDICATOR",
    "ECC_IDX",
    "ALIGNMENT_POSITIONS",
    "ECC_CODEWORDS_PER_BLOCK",
    "NUM_BLOCKS",
    "ALPHANUM_CHARS",
    "MODE_INDICATOR",
    # Re-exported types
    "ModuleGrid",
    "Barcode2DLayoutConfig",
    "PaintScene",
]
