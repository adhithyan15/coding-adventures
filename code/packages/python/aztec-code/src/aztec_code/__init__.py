"""Aztec Code encoder — ISO/IEC 24778:2008 compliant.

Aztec Code was invented by Andrew Longacre Jr. at Welch Allyn in 1995 and
published as a patent-free format. Unlike QR Code (which uses three square
finder patterns at three corners), Aztec Code places a single **bullseye
finder pattern at the center** of the symbol. The scanner finds the centre
first, then reads outward in a spiral — no large quiet zone is needed.

Where Aztec Code is used today
------------------------------

- **IATA boarding passes** — the barcode on every airline boarding pass.
- **Eurostar and Amtrak rail tickets** — printed and on-screen tickets.
- **PostNL, Deutsche Post, La Poste** — European postal routing.
- **US military ID cards.**

Symbol variants
---------------

.. code-block::

    Compact: 1-4 layers,  size = 11 + 4*layers   (15x15 to 27x27)
    Full:    1-32 layers, size = 15 + 4*layers   (19x19 to 143x143)

Encoding pipeline (v0.1.0 — byte-mode only)
-------------------------------------------

.. code-block::

    input string / bytes
      -> Binary-Shift codewords from Upper mode
      -> symbol size selection (smallest compact then full that fits at 23% ECC)
      -> pad to exact codeword count
      -> GF(256)/0x12D Reed-Solomon ECC (poly 0x12D, b=1 roots alpha^1..alpha^n)
      -> bit stuffing (insert complement after 4 consecutive identical bits)
      -> GF(16) mode message (layers + codeword count + 5 or 6 RS nibbles)
      -> ModuleGrid  (bullseye -> orientation marks -> mode msg -> data spiral)

v0.1.0 simplifications
----------------------

1. **Byte-mode only** — all input encoded via Binary-Shift from Upper mode.
   Multi-mode (Digit/Upper/Lower/Mixed/Punct) optimisation is v0.2.0.
2. **8-bit codewords** -> GF(256) RS (same polynomial as Data Matrix: 0x12D).
   GF(16) and GF(32) RS for 4-bit/5-bit codewords are v0.2.0.
3. **Default ECC = 23%.**
4. **Auto-select compact vs full** (force-compact option is v0.2.0).

Public API
----------

- :func:`encode` — encode a string or bytes into a :class:`ModuleGrid`.
- :func:`encode_and_layout` — encode + layout into a :class:`PaintScene`.
- :func:`explain` — encode and return an :class:`AnnotatedModuleGrid`.
- :class:`AztecOptions` — encoding options (currently ``min_ecc_percent``).
- :class:`AztecError` / :class:`InputTooLongError` — error types.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Final

from barcode_2d import (
    AnnotatedModuleGrid,
    Barcode2DLayoutConfig,
    ModuleGrid,
)
from barcode_2d import layout as _barcode_layout
from paint_instructions import PaintScene

__version__: Final = "0.1.0"

VERSION: Final = __version__
"""Convenience alias matching the TypeScript package's ``VERSION`` export."""


# ============================================================================
# Public types
# ============================================================================


@dataclass(frozen=True)
class AztecOptions:
    """Options for Aztec Code encoding.

    Attributes
    ----------
    min_ecc_percent:
        Minimum error-correction percentage.  Default: 23.  Range: 10–90.
        Higher values reserve more codewords for parity, which generally
        requires a larger symbol but recovers more damage.
    """

    min_ecc_percent: int = 23


# ============================================================================
# Error hierarchy
# ============================================================================


class AztecError(Exception):
    """Base class for all Aztec Code encoder errors.

    Catch ``AztecError`` to handle any encoder error regardless of subclass.
    """


class InputTooLongError(AztecError):
    """Raised when the input is too long for any Aztec symbol.

    The maximum capacity is roughly 1914 bytes in a 32-layer full symbol at
    23% ECC.  For larger payloads, split the data or use a different format.
    """


# ============================================================================
# GF(16) arithmetic — for the mode message Reed-Solomon
# ============================================================================
#
# GF(16) is the finite field with 16 elements, built from the primitive
# polynomial:
#
#   p(x) = x^4 + x + 1   (binary: 10011 = 0x13)
#
# Every non-zero element can be written as a power of the primitive
# element alpha.  alpha is the root of p(x), so alpha^4 = alpha + 1.
#
# The log table maps a field element (1..15) to its discrete log (0..14).
# The antilog (exponentiation) table maps a log value to its element.
#
# alpha^0=1, alpha^1=2, alpha^2=4, alpha^3=8,
# alpha^4=3, alpha^5=6, alpha^6=12, alpha^7=11,
# alpha^8=5, alpha^9=10, alpha^10=7, alpha^11=14,
# alpha^12=15, alpha^13=13, alpha^14=9, alpha^15=1 (period=15)

_LOG16: Final[tuple[int, ...]] = (
    -1,  # log(0) = undefined
    0,   # log(1) = 0
    1,   # log(2) = 1
    4,   # log(3) = 4
    2,   # log(4) = 2
    8,   # log(5) = 8
    5,   # log(6) = 5
    10,  # log(7) = 10
    3,   # log(8) = 3
    14,  # log(9) = 14
    9,   # log(10) = 9
    7,   # log(11) = 7
    6,   # log(12) = 6
    13,  # log(13) = 13
    11,  # log(14) = 11
    12,  # log(15) = 12
)
"""GF(16) discrete logarithm: ``_LOG16[e] = i`` means ``alpha**i = e``."""

_ALOG16: Final[tuple[int, ...]] = (
    1, 2, 4, 8, 3, 6, 12, 11, 5, 10, 7, 14, 15, 13, 9, 1,
)
"""GF(16) antilogarithm: ``_ALOG16[i] = alpha**i`` (period 15, so index 15 = 1)."""


def _gf16_mul(a: int, b: int) -> int:
    """Multiply two GF(16) elements.

    Uses log/antilog: ``a * b = ALOG16[(LOG16[a] + LOG16[b]) mod 15]``.
    Returns 0 if either operand is 0.
    """
    if a == 0 or b == 0:
        return 0
    return _ALOG16[(_LOG16[a] + _LOG16[b]) % 15]


def _build_gf16_generator(n: int) -> list[int]:
    """Build the GF(16) RS generator polynomial with roots ``alpha^1..alpha^n``.

    Returns ``[g_0, g_1, ..., g_n]`` where ``g_n = 1`` (monic).

    The polynomial is built incrementally by multiplying ``(x - alpha^i)``
    one factor at a time, using XOR for addition since GF(16) is
    characteristic-2.
    """
    g: list[int] = [1]
    for i in range(1, n + 1):
        ai = _ALOG16[i % 15]
        nxt: list[int] = [0] * (len(g) + 1)
        for j in range(len(g)):
            nxt[j + 1] ^= g[j]
            nxt[j] ^= _gf16_mul(ai, g[j])
        g = nxt
    return g


def _gf16_rs_encode(data: list[int], n: int) -> list[int]:
    """Compute ``n`` GF(16) RS check nibbles for the given data nibbles.

    Uses the LFSR polynomial-division algorithm — the same shape as the
    classic CRC computation but in GF(16) arithmetic.
    """
    g = _build_gf16_generator(n)
    rem: list[int] = [0] * n
    for nibble in data:
        fb = nibble ^ rem[0]
        for i in range(n - 1):
            rem[i] = rem[i + 1] ^ _gf16_mul(g[i + 1], fb)
        rem[n - 1] = _gf16_mul(g[n], fb)
    return rem


# ============================================================================
# GF(256)/0x12D arithmetic — for 8-bit data codewords
# ============================================================================
#
# Aztec Code uses GF(256) with primitive polynomial:
#   p(x) = x^8 + x^5 + x^4 + x^2 + x + 1  =  0x12D
#
# This is the SAME polynomial as Data Matrix ECC200, but DIFFERENT from
# QR Code (0x11D).  The repo's ``gf256`` helper package targets QR's 0x11D,
# so we build the 0x12D tables inline here.
#
# Generator convention: b=1, roots alpha^1..alpha^n (the MA02 / Aztec style).

_GF256_POLY: Final = 0x12D


def _build_gf256_tables() -> tuple[tuple[int, ...], tuple[int, ...]]:
    """Build the doubled antilog table and the discrete-log table for GF(256)/0x12D.

    The doubled antilog table (length 512) lets multiplication skip the
    modulo: ``alpha^(i + j) = exp[i + j]`` for any ``i, j in [0, 254]``.

    The primitive element is ``alpha = 2``.  Repeatedly doubling and
    reducing modulo the primitive polynomial enumerates all 255 non-zero
    elements before cycling back to 1.
    """
    exp_table: list[int] = [0] * 512
    log_table: list[int] = [0] * 256
    x = 1
    for i in range(255):
        exp_table[i] = x
        exp_table[i + 255] = x
        log_table[x] = i
        x <<= 1
        if x & 0x100:
            x ^= _GF256_POLY
        x &= 0xFF
    exp_table[255] = 1
    return tuple(exp_table), tuple(log_table)


_EXP_12D: Final[tuple[int, ...]]
_LOG_12D: Final[tuple[int, ...]]
_EXP_12D, _LOG_12D = _build_gf256_tables()


def _gf256_mul(a: int, b: int) -> int:
    """Multiply two GF(256)/0x12D elements via log/antilog lookup."""
    if a == 0 or b == 0:
        return 0
    return _EXP_12D[_LOG_12D[a] + _LOG_12D[b]]


def _build_gf256_generator(n: int) -> list[int]:
    """Build the GF(256)/0x12D RS generator polynomial.

    Roots are ``alpha^1..alpha^n``.  Returns big-endian coefficients
    (highest degree first) — this matches the iteration order used in
    :func:`_gf256_rs_encode` below.
    """
    g: list[int] = [1]
    for i in range(1, n + 1):
        ai = _EXP_12D[i]
        nxt: list[int] = [0] * (len(g) + 1)
        for j in range(len(g)):
            nxt[j] ^= g[j]
            nxt[j + 1] ^= _gf256_mul(g[j], ai)
        g = nxt
    return g


def _gf256_rs_encode(data: list[int], n_check: int) -> list[int]:
    """Compute ``n_check`` GF(256)/0x12D RS check bytes for the given data bytes.

    Standard LFSR polynomial division.  ``rem`` holds the partial remainder
    coefficients in big-endian order.
    """
    g = _build_gf256_generator(n_check)
    n = len(g) - 1
    rem: list[int] = [0] * n
    for byte in data:
        fb = byte ^ rem[0]
        for i in range(n - 1):
            rem[i] = rem[i + 1] ^ _gf256_mul(g[i + 1], fb)
        rem[n - 1] = _gf256_mul(g[n], fb)
    return rem


# ============================================================================
# Aztec Code capacity tables
# ============================================================================
#
# Derived from ISO/IEC 24778:2008 Table 1.
# Each entry: (total_bits, max_bytes8) where:
#   - total_bits:  total data+ECC bit positions in the symbol layers
#   - max_bytes8:  number of 8-bit codeword slots (data + ECC combined)


@dataclass(frozen=True)
class _Capacity:
    """One row of the Aztec capacity table."""

    total_bits: int
    """Total data+ECC bit positions in the symbol layers."""

    max_bytes8: int
    """Number of 8-bit codeword slots (data + ECC combined)."""


_COMPACT_CAPACITY: Final[tuple[_Capacity, ...]] = (
    _Capacity(0, 0),         # index 0 unused
    _Capacity(72, 9),        # 1 layer, 15x15
    _Capacity(200, 25),      # 2 layers, 19x19
    _Capacity(392, 49),      # 3 layers, 23x23
    _Capacity(648, 81),      # 4 layers, 27x27
)

_FULL_CAPACITY: Final[tuple[_Capacity, ...]] = (
    _Capacity(0, 0),         # index 0 unused
    _Capacity(88, 11),       #  1 layer
    _Capacity(216, 27),      #  2 layers
    _Capacity(360, 45),      #  3 layers
    _Capacity(520, 65),      #  4 layers
    _Capacity(696, 87),      #  5 layers
    _Capacity(888, 111),     #  6 layers
    _Capacity(1096, 137),    #  7 layers
    _Capacity(1320, 165),    #  8 layers
    _Capacity(1560, 195),    #  9 layers
    _Capacity(1816, 227),    # 10 layers
    _Capacity(2088, 261),    # 11 layers
    _Capacity(2376, 297),    # 12 layers
    _Capacity(2680, 335),    # 13 layers
    _Capacity(3000, 375),    # 14 layers
    _Capacity(3336, 417),    # 15 layers
    _Capacity(3688, 461),    # 16 layers
    _Capacity(4056, 507),    # 17 layers
    _Capacity(4440, 555),    # 18 layers
    _Capacity(4840, 605),    # 19 layers
    _Capacity(5256, 657),    # 20 layers
    _Capacity(5688, 711),    # 21 layers
    _Capacity(6136, 767),    # 22 layers
    _Capacity(6600, 825),    # 23 layers
    _Capacity(7080, 885),    # 24 layers
    _Capacity(7576, 947),    # 25 layers
    _Capacity(8088, 1011),   # 26 layers
    _Capacity(8616, 1077),   # 27 layers
    _Capacity(9160, 1145),   # 28 layers
    _Capacity(9720, 1215),   # 29 layers
    _Capacity(10296, 1287),  # 30 layers
    _Capacity(10888, 1361),  # 31 layers
    _Capacity(11496, 1437),  # 32 layers
)


# ============================================================================
# Data encoding — Binary-Shift from Upper mode (v0.1.0 byte-mode path)
# ============================================================================
#
# All input is wrapped in a single Binary-Shift block from Upper mode:
#
#   1. Emit 5 bits = 0b11111  (Binary-Shift escape in Upper mode)
#   2. If len <= 31: 5 bits for length
#      If len > 31:  5 bits = 0b00000, then 11 bits for length
#   3. Each byte as 8 bits, MSB first


def _encode_bytes_as_bits(data: bytes) -> list[int]:
    """Encode input bytes as a flat bit list using the Binary-Shift escape.

    Returns a list of 0/1 values, MSB first.  This is the entire data
    bit stream consumed by the symbol-selection step.
    """
    bits: list[int] = []

    def write_bits(value: int, count: int) -> None:
        for i in range(count - 1, -1, -1):
            bits.append((value >> i) & 1)

    length = len(data)

    # Binary-Shift escape (5 bits of all ones in Upper mode).
    write_bits(0b11111, 5)

    # Length field: short form (5 bits) for 1–31, long form (5+11 bits) otherwise.
    if length <= 31:
        write_bits(length, 5)
    else:
        write_bits(0, 5)
        write_bits(length, 11)

    for byte in data:
        write_bits(byte, 8)

    return bits


# ============================================================================
# Symbol size selection
# ============================================================================


@dataclass(frozen=True)
class _SymbolSpec:
    """Resolved symbol parameters chosen by :func:`_select_symbol`."""

    compact: bool
    layers: int
    data_cw_count: int
    ecc_cw_count: int
    total_bits: int


def _select_symbol(data_bit_count: int, min_ecc_pct: int) -> _SymbolSpec:
    """Select the smallest symbol that fits ``data_bit_count`` bits.

    Tries compact 1-4 first, then full 1-32.  We add a 20% conservative
    overhead to account for bit stuffing (which inserts a complement bit
    after every 4 consecutive identical bits — worst case, every 5th bit
    is overhead, so 20% is a safe upper bound).

    Raises
    ------
    InputTooLongError
        If no symbol — even the largest 32-layer full — can fit the data.
    """
    # Stuffing inserts at most 1 bit per 4 input bits, so 20% overhead is safe.
    stuffed_bit_count = -(-data_bit_count * 12 // 10)  # ceil(x * 1.2)

    for layers in range(1, 5):
        cap = _COMPACT_CAPACITY[layers]
        total_bytes = cap.max_bytes8
        ecc_cw_count = -(-(min_ecc_pct * total_bytes) // 100)  # ceil
        data_cw_count = total_bytes - ecc_cw_count
        if data_cw_count <= 0:
            continue
        # ceil(stuffed_bit_count / 8) <= data_cw_count
        if -(-stuffed_bit_count // 8) <= data_cw_count:
            return _SymbolSpec(
                compact=True,
                layers=layers,
                data_cw_count=data_cw_count,
                ecc_cw_count=ecc_cw_count,
                total_bits=cap.total_bits,
            )

    for layers in range(1, 33):
        cap = _FULL_CAPACITY[layers]
        total_bytes = cap.max_bytes8
        ecc_cw_count = -(-(min_ecc_pct * total_bytes) // 100)
        data_cw_count = total_bytes - ecc_cw_count
        if data_cw_count <= 0:
            continue
        if -(-stuffed_bit_count // 8) <= data_cw_count:
            return _SymbolSpec(
                compact=False,
                layers=layers,
                data_cw_count=data_cw_count,
                ecc_cw_count=ecc_cw_count,
                total_bits=cap.total_bits,
            )

    raise InputTooLongError(
        f"Input is too long to fit in any Aztec Code symbol "
        f"({data_bit_count} bits needed)"
    )


# ============================================================================
# Padding
# ============================================================================


def _pad_to_bytes(bits: list[int], target_bytes: int) -> list[int]:
    """Pad the bit stream up to exactly ``target_bytes * 8`` bits with zeroes.

    First pad to the next byte boundary (so the bit stream is whole bytes),
    then keep appending zero bits until reaching ``target_bytes * 8`` bits.
    Truncates if already longer (should never happen in practice — the
    selector already verified capacity).
    """
    out = list(bits)
    while len(out) % 8 != 0:
        out.append(0)
    while len(out) < target_bytes * 8:
        out.append(0)
    return out[: target_bytes * 8]


# ============================================================================
# Bit stuffing
# ============================================================================
#
# After every 4 consecutive identical bits (all 0 or all 1), insert one
# complement bit.  Applies only to the data+ECC bit stream.
#
# Example::
#
#   Input:  1 1 1 1 0 0 0 0
#   After 4 ones: insert 0  -> [1,1,1,1,0]
#   After 4 zeros: insert 1 -> [1,1,1,1,0, 0,0,0,1,0]
#
# This rule prevents long runs of identical bits, which the scanner needs to
# distinguish from the bullseye/orientation patterns.


def _stuff_bits(bits: list[int]) -> list[int]:
    """Apply Aztec bit stuffing to the data+ECC bit stream.

    Inserts a complement bit after every run of 4 identical bits.  After
    inserting, the run resets so the inserted complement counts as the new
    run-of-1.
    """
    stuffed: list[int] = []
    run_val = -1
    run_len = 0

    for bit in bits:
        if bit == run_val:
            run_len += 1
        else:
            run_val = bit
            run_len = 1

        stuffed.append(bit)

        if run_len == 4:
            stuff_bit = 1 - bit
            stuffed.append(stuff_bit)
            run_val = stuff_bit
            run_len = 1

    return stuffed


# ============================================================================
# Mode message encoding
# ============================================================================
#
# The mode message encodes the layer count and data codeword count,
# protected by a GF(16) Reed-Solomon code.
#
# Compact (28 bits = 7 nibbles):
#   m = ((layers-1) << 6) | (data_cw_count-1)
#   2 data nibbles + 5 ECC nibbles
#
# Full (40 bits = 10 nibbles):
#   m = ((layers-1) << 11) | (data_cw_count-1)
#   4 data nibbles + 6 ECC nibbles
#
# Note: data nibbles are emitted little-endian (LSB nibble first), but bits
# within each nibble are MSB-first.  This matches the wire-format convention
# in ISO/IEC 24778:2008 Annex G.


def _encode_mode_message(
    compact: bool,
    layers: int,
    data_cw_count: int,
) -> list[int]:
    """Encode the mode message as a flat bit list.

    Returns 28 bits for compact symbols, 40 bits for full symbols.
    """
    if compact:
        m = ((layers - 1) << 6) | (data_cw_count - 1)
        data_nibbles = [m & 0xF, (m >> 4) & 0xF]
        num_ecc = 5
    else:
        m = ((layers - 1) << 11) | (data_cw_count - 1)
        data_nibbles = [
            m & 0xF,
            (m >> 4) & 0xF,
            (m >> 8) & 0xF,
            (m >> 12) & 0xF,
        ]
        num_ecc = 6

    ecc_nibbles = _gf16_rs_encode(data_nibbles, num_ecc)
    all_nibbles = data_nibbles + ecc_nibbles

    bits: list[int] = []
    for nibble in all_nibbles:
        for i in range(3, -1, -1):
            bits.append((nibble >> i) & 1)

    return bits


# ============================================================================
# Grid construction
# ============================================================================


def _symbol_size(compact: bool, layers: int) -> int:
    """Side length in modules: compact = 11+4*layers, full = 15+4*layers."""
    return (11 + 4 * layers) if compact else (15 + 4 * layers)


def _bullseye_radius(compact: bool) -> int:
    """Bullseye Chebyshev radius: 5 for compact, 7 for full."""
    return 5 if compact else 7


def _draw_bullseye(
    modules: list[list[bool]],
    reserved: list[list[bool]],
    cx: int,
    cy: int,
    compact: bool,
) -> None:
    """Paint the bullseye finder pattern at the centre.

    Module colour at Chebyshev distance ``d`` from centre:

    - ``d <= 1`` -> DARK  (the solid 3x3 inner core)
    - ``d > 1, d even`` -> LIGHT
    - ``d > 1, d odd`` -> DARK

    Also marks every painted cell as reserved so the data spiral skips them.
    """
    br = _bullseye_radius(compact)
    for row in range(cy - br, cy + br + 1):
        for col in range(cx - br, cx + br + 1):
            d = max(abs(col - cx), abs(row - cy))
            dark = (d <= 1) or (d % 2 == 1)
            modules[row][col] = dark
            reserved[row][col] = True


def _draw_reference_grid(
    modules: list[list[bool]],
    reserved: list[list[bool]],
    cx: int,
    cy: int,
    size: int,
) -> None:
    """Paint the reference grid for full Aztec symbols.

    Reference grid lines lie at rows/cols whose offset from the centre is a
    multiple of 16.  The module value alternates dark/light along each line
    based on the offset's parity from the centre.

    This grid is later partially overwritten by the bullseye and mode-message
    rings near the centre, which is fine — those reservations are written by
    the subsequent draw calls.
    """
    for row in range(size):
        for col in range(size):
            on_h = (cy - row) % 16 == 0
            on_v = (cx - col) % 16 == 0
            if not on_h and not on_v:
                continue

            if on_h and on_v:
                dark = True
            elif on_h:
                dark = (cx - col) % 2 == 0
            else:
                dark = (cy - row) % 2 == 0

            modules[row][col] = dark
            reserved[row][col] = True


def _draw_orientation_and_mode_message(
    modules: list[list[bool]],
    reserved: list[list[bool]],
    cx: int,
    cy: int,
    compact: bool,
    mode_message_bits: list[int],
) -> list[tuple[int, int]]:
    """Paint orientation marks and place mode-message bits around the centre.

    The mode-message ring is the perimeter at Chebyshev radius
    ``bullseye_radius + 1``.  The 4 corners of that ring are orientation
    marks (always DARK).  The remaining non-corner positions carry the
    mode-message bits clockwise starting from the top edge (top-left + 1).

    Returns
    -------
    list[tuple[int, int]]
        The remaining ring positions (after the mode-message bits) as
        ``(col, row)`` pairs.  The caller fills these from the data spiral.
    """
    r = _bullseye_radius(compact) + 1

    # Enumerate non-corner perimeter positions clockwise, starting from the
    # cell immediately right of the top-left corner.
    non_corner: list[tuple[int, int]] = []

    # Top edge (left to right, skipping both corners).
    for col in range(cx - r + 1, cx + r):
        non_corner.append((col, cy - r))
    # Right edge (top to bottom, skipping both corners).
    for row in range(cy - r + 1, cy + r):
        non_corner.append((cx + r, row))
    # Bottom edge (right to left, skipping both corners).
    for col in range(cx + r - 1, cx - r, -1):
        non_corner.append((col, cy + r))
    # Left edge (bottom to top, skipping both corners).
    for row in range(cy + r - 1, cy - r, -1):
        non_corner.append((cx - r, row))

    # Place the 4 orientation-mark corners as DARK.
    corners: list[tuple[int, int]] = [
        (cx - r, cy - r),
        (cx + r, cy - r),
        (cx + r, cy + r),
        (cx - r, cy + r),
    ]
    for col, row in corners:
        modules[row][col] = True
        reserved[row][col] = True

    # Place mode-message bits.
    for i in range(min(len(mode_message_bits), len(non_corner))):
        col, row = non_corner[i]
        modules[row][col] = mode_message_bits[i] == 1
        reserved[row][col] = True

    # Return the leftover ring positions for the data spiral to consume first.
    return non_corner[len(mode_message_bits):]


# ============================================================================
# Data layer spiral placement
# ============================================================================
#
# Bits are placed in a clockwise spiral starting from the innermost data
# layer.  Each layer band is 2 modules wide.  Within a band, pairs of cells
# are written outer-row/col first, then inner.
#
# For compact: d_inner of the first layer = bullseye_radius + 2 = 7.
# For full:    d_inner of the first layer = bullseye_radius + 2 = 9.


def _place_data_bits(
    modules: list[list[bool]],
    reserved: list[list[bool]],
    bits: list[int],
    cx: int,
    cy: int,
    compact: bool,
    layers: int,
    mode_ring_remaining_positions: list[tuple[int, int]],
) -> None:
    """Place all data bits using the clockwise layer spiral.

    Fills the leftover mode-ring positions first, then spirals outward
    layer by layer.  Cells already reserved by the bullseye, orientation
    marks, mode message, or reference grid are silently skipped.
    """
    size = len(modules)
    bit_index = 0

    def place_bit(col: int, row: int) -> None:
        nonlocal bit_index
        if row < 0 or row >= size or col < 0 or col >= size:
            return
        if not reserved[row][col]:
            value = bits[bit_index] if bit_index < len(bits) else 0
            modules[row][col] = value == 1
            bit_index += 1

    # Fill remaining mode ring positions first (these aren't reserved yet).
    for col, row in mode_ring_remaining_positions:
        value = bits[bit_index] if bit_index < len(bits) else 0
        modules[row][col] = value == 1
        bit_index += 1

    # Spiral through data layers.
    br = _bullseye_radius(compact)
    d_start = br + 2  # mode msg ring sits at br+1; first data layer at br+2

    for layer in range(layers):
        d_i = d_start + 2 * layer  # inner radius of the layer band
        d_o = d_i + 1               # outer radius of the layer band

        # Top edge: left to right.
        for col in range(cx - d_i + 1, cx + d_i + 1):
            place_bit(col, cy - d_o)
            place_bit(col, cy - d_i)
        # Right edge: top to bottom.
        for row in range(cy - d_i + 1, cy + d_i + 1):
            place_bit(cx + d_o, row)
            place_bit(cx + d_i, row)
        # Bottom edge: right to left.
        for col in range(cx + d_i, cx - d_i, -1):
            place_bit(col, cy + d_o)
            place_bit(col, cy + d_i)
        # Left edge: bottom to top.
        for row in range(cy + d_i, cy - d_i, -1):
            place_bit(cx - d_o, row)
            place_bit(cx - d_i, row)


# ============================================================================
# Main encode function
# ============================================================================


def encode(
    data: str | bytes,
    options: AztecOptions | None = None,
) -> ModuleGrid:
    """Encode ``data`` as an Aztec Code symbol.

    Returns a :class:`ModuleGrid` where ``modules[row][col]`` is ``True`` for
    a dark module.  The grid origin ``(0, 0)`` is the top-left corner.

    Encoding steps:

    1. Encode the input via Binary-Shift from Upper mode.
    2. Select the smallest symbol satisfying the requested ECC level.
    3. Pad the data codeword sequence.
    4. Compute GF(256)/0x12D Reed-Solomon ECC.
    5. Apply bit stuffing to the combined data+ECC bit stream.
    6. Compute the GF(16) mode message.
    7. Initialise the grid with structural patterns
       (reference grid for full symbols, then bullseye, then orientation +
       mode message).
    8. Place data+ECC bits in the clockwise layer spiral.

    Parameters
    ----------
    data:
        Input string (encoded as UTF-8) or raw byte sequence.
    options:
        Optional :class:`AztecOptions`.  Defaults to ``min_ecc_percent=23``.

    Raises
    ------
    InputTooLongError
        If the data exceeds the maximum symbol capacity (~1914 bytes at
        23% ECC in a 32-layer full symbol).
    """
    opts = options if options is not None else AztecOptions()
    min_ecc_pct = opts.min_ecc_percent

    input_bytes = (
        data.encode("utf-8") if isinstance(data, str) else bytes(data)
    )

    # Step 1: encode data into a bit stream.
    data_bits = _encode_bytes_as_bits(input_bytes)

    # Step 2: pick the smallest symbol that fits.
    spec = _select_symbol(len(data_bits), min_ecc_pct)
    compact = spec.compact
    layers = spec.layers
    data_cw_count = spec.data_cw_count
    ecc_cw_count = spec.ecc_cw_count

    # Step 3: pad the bit stream up to data_cw_count whole bytes.
    padded_bits = _pad_to_bytes(data_bits, data_cw_count)

    data_bytes: list[int] = []
    for i in range(data_cw_count):
        byte = 0
        for b in range(8):
            byte = (byte << 1) | padded_bits[i * 8 + b]
        # All-zero codeword avoidance: per ISO/IEC 24778:2008 §7.3.1.1 the
        # last data codeword cannot be 0x00 (it would collide with the RS
        # padding sentinel).  Substitute 0xFF in that case.
        if byte == 0 and i == data_cw_count - 1:
            byte = 0xFF
        data_bytes.append(byte)

    # Step 4: compute Reed-Solomon ECC bytes.
    ecc_bytes = _gf256_rs_encode(data_bytes, ecc_cw_count)

    # Step 5: build the combined bit stream and apply bit stuffing.
    all_bytes = data_bytes + ecc_bytes
    raw_bits: list[int] = []
    for byte in all_bytes:
        for i in range(7, -1, -1):
            raw_bits.append((byte >> i) & 1)
    stuffed_bits = _stuff_bits(raw_bits)

    # Step 6: build the mode message.
    mode_msg = _encode_mode_message(compact, layers, data_cw_count)

    # Step 7: initialise the grid.
    size = _symbol_size(compact, layers)
    cx = size // 2
    cy = size // 2

    modules: list[list[bool]] = [[False] * size for _ in range(size)]
    reserved: list[list[bool]] = [[False] * size for _ in range(size)]

    # Reference grid first (full symbols only) — the bullseye paints over the
    # central section afterwards, but the outer reference modules survive.
    if not compact:
        _draw_reference_grid(modules, reserved, cx, cy, size)
    _draw_bullseye(modules, reserved, cx, cy, compact)

    mode_ring_remaining = _draw_orientation_and_mode_message(
        modules, reserved, cx, cy, compact, mode_msg
    )

    # Step 8: place the data spiral.
    _place_data_bits(
        modules, reserved, stuffed_bits, cx, cy, compact, layers, mode_ring_remaining
    )

    # Convert to the immutable ModuleGrid representation.
    return ModuleGrid(
        cols=size,
        rows=size,
        modules=tuple(tuple(row) for row in modules),
        module_shape="square",
    )


# ============================================================================
# Convenience wrappers — layout, scene rendering, annotated grid
# ============================================================================


def layout_grid(
    grid: ModuleGrid,
    config: Barcode2DLayoutConfig | None = None,
) -> PaintScene:
    """Convert a :class:`ModuleGrid` to a :class:`PaintScene` via ``barcode-2d``.

    Aztec needs no large quiet zone (the bullseye serves as a self-contained
    locator), but a small 2-module quiet zone improves scanner ergonomics.
    The default :class:`Barcode2DLayoutConfig` uses a 4-module quiet zone;
    callers wanting tighter symbols can pass a custom config.
    """
    return _barcode_layout(grid, config)


def encode_and_layout(
    data: str | bytes,
    options: AztecOptions | None = None,
    config: Barcode2DLayoutConfig | None = None,
) -> PaintScene:
    """Encode ``data`` and convert the resulting grid to a :class:`PaintScene`."""
    grid = encode(data, options)
    return layout_grid(grid, config)


def explain(
    data: str | bytes,
    options: AztecOptions | None = None,
) -> AnnotatedModuleGrid:
    """Encode ``data`` and return an :class:`AnnotatedModuleGrid`.

    In v0.1.0 annotations are not yet populated; the function returns the
    grid wrapped in an annotated container with no per-module roles.  Full
    annotation support is planned for v0.2.0.
    """
    grid = encode(data, options)
    empty_row: tuple[None, ...] = tuple(None for _ in range(grid.cols))
    annotations = tuple(empty_row for _ in range(grid.rows))
    return AnnotatedModuleGrid(
        cols=grid.cols,
        rows=grid.rows,
        modules=grid.modules,
        module_shape=grid.module_shape,
        annotations=annotations,
    )


__all__ = [
    "VERSION",
    "__version__",
    "AztecOptions",
    "AztecError",
    "InputTooLongError",
    "ModuleGrid",
    "AnnotatedModuleGrid",
    "Barcode2DLayoutConfig",
    "PaintScene",
    "encode",
    "encode_and_layout",
    "explain",
    "layout_grid",
]
