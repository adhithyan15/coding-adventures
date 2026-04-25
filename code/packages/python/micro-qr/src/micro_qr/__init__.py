"""Micro QR Code encoder — ISO/IEC 18004:2015 Annex E compliant.

Micro QR Code is the compact variant of QR Code, designed for applications
where even the smallest standard QR (21×21 at version 1) is too large.
Common use cases include surface-mount component labels, circuit board
markings, and miniature industrial tags.

Symbol sizes
------------
.. code-block::

    M1: 11×11   M2: 13×13   M3: 15×15   M4: 17×17
    formula: size = 2 × version_number + 9

Key differences from regular QR Code
--------------------------------------
- **Single finder pattern** at top-left only (one 7×7 square, not three).
- **Timing at row 0 / col 0** (not row 6 / col 6).
- **Only 4 mask patterns** (not 8).
- **Format XOR mask 0x4445** (not 0x5412).
- **Single copy of format info** (not two).
- **2-module quiet zone** (not 4).
- **Narrower mode indicators** (0–3 bits instead of 4).
- **Single block** (no interleaving).

Encoding pipeline
-----------------
.. code-block::

    input string
      → auto-select smallest symbol (M1..M4) and mode
      → build bit stream (mode indicator + char count + data + terminator + padding)
      → Reed-Solomon ECC (GF(256)/0x11D, b=0, single block)
      → initialize grid (finder, L-shaped separator, timing at row0/col0,
        format reserved)
      → zigzag data placement (two-column snake from bottom-right)
      → evaluate 4 mask patterns, pick lowest penalty
      → write format information (15 bits, single copy, XOR 0x4445)
      → ModuleGrid

Public API
----------
- :func:`encode` — auto-selects the smallest symbol that fits the input.
- :func:`encode_at` — encode at a specific version + ECC level.
- :func:`layout_grid` — ``ModuleGrid`` → ``PaintScene`` (delegates to barcode-2d).
- :func:`encode_and_layout` — encode + layout in one call.
- :class:`MicroQRVersion` — ``M1``, ``M2``, ``M3``, ``M4``.
- :class:`MicroQREccLevel` — ``Detection``, ``L``, ``M``, ``Q``.
- :class:`MicroQRError` (and subclasses) — error types.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Final

import gf256 as gf
from barcode_2d import (
    Barcode2DLayoutConfig,
    ModuleGrid,
    make_module_grid,
    set_module,
)
from barcode_2d import layout as _barcode_layout
from paint_instructions import PaintScene

__version__: Final = "0.1.0"

# ============================================================================
# Public types — MicroQRVersion and MicroQREccLevel
# ============================================================================


class MicroQRVersion:
    """Micro QR symbol designator.

    Each step up adds two rows and columns (size = 2 × version_number + 9):
    M1 = 11×11, M2 = 13×13, M3 = 15×15, M4 = 17×17.
    """

    M1: Final = "M1"
    M2: Final = "M2"
    M3: Final = "M3"
    M4: Final = "M4"


class MicroQREccLevel:
    """Error correction level for Micro QR.

    +------------+--------------+-----------------------------------+
    | Level      | Available in | Recovery capability               |
    +============+==============+===================================+
    | Detection  | M1 only      | Detects errors only (no recovery) |
    +------------+--------------+-----------------------------------+
    | L          | M2, M3, M4   | ~7% of codewords recoverable      |
    +------------+--------------+-----------------------------------+
    | M          | M2, M3, M4   | ~15% of codewords recoverable     |
    +------------+--------------+-----------------------------------+
    | Q          | M4 only      | ~25% of codewords recoverable     |
    +------------+--------------+-----------------------------------+

    Level H is not available in any Micro QR symbol — the symbols are
    simply too small to spare 30% of their codewords for redundancy.
    """

    Detection: Final = "Detection"
    L: Final = "L"
    M: Final = "M"
    Q: Final = "Q"


# ============================================================================
# Error hierarchy
# ============================================================================


class MicroQRError(Exception):
    """Base class for all Micro QR encoder errors.

    Catch ``MicroQRError`` to handle any encoder error regardless of type.
    """


class InputTooLongError(MicroQRError):
    """Input is too long to fit in any M1–M4 symbol at any ECC level.

    The maximum capacity is 35 numeric characters in M4-L.
    Consider using regular QR Code for longer data.
    """


class ECCNotAvailableError(MicroQRError):
    """The requested ECC level is not available for the chosen symbol.

    For example: ECC level Q is only available in M4.
    ECC level H is not available in any Micro QR symbol.
    """


class UnsupportedModeError(MicroQRError):
    """The requested encoding mode is not available for the chosen symbol.

    For example: byte mode requires M3 or M4.
    Alphanumeric mode requires M2 or higher.
    """


class InvalidCharacterError(MicroQRError):
    """A character cannot be encoded in the selected mode.

    For example: lowercase letters cannot be encoded in alphanumeric mode.
    """


# ============================================================================
# Symbol configuration table
# ============================================================================

# All 8 valid (version, ECC) combinations with their compile-time constants.
# This replaces a large if-elif chain and lets us iterate over candidates
# in order when auto-selecting the smallest symbol.


@dataclass(frozen=True)
class _SymbolConfig:
    """Compile-time constants for one (version, ECC) combination.

    There are exactly 8 valid combinations:
    M1/Detection, M2/L, M2/M, M3/L, M3/M, M4/L, M4/M, M4/Q.
    """

    version: str
    """MicroQRVersion constant: "M1", "M2", "M3", or "M4"."""

    ecc: str
    """MicroQREccLevel constant."""

    symbol_indicator: int
    """3-bit value placed in format information (0..7)."""

    size: int
    """Symbol side length in modules (11, 13, 15, or 17)."""

    data_cw: int
    """Number of data codewords (full bytes, except M1 which uses 2.5 bytes)."""

    ecc_cw: int
    """Number of ECC codewords appended after data."""

    numeric_cap: int
    """Maximum numeric characters (0 = mode not supported)."""

    alpha_cap: int
    """Maximum alphanumeric characters (0 = mode not supported)."""

    byte_cap: int
    """Maximum byte characters (0 = mode not supported)."""

    terminator_bits: int
    """Length of the terminator in bits (3/5/7/9 depending on version)."""

    mode_indicator_bits: int
    """Width of mode indicator field in bits (0=M1, 1=M2, 2=M3, 3=M4)."""

    cc_bits_numeric: int
    """Bit width of the character count field for numeric mode."""

    cc_bits_alpha: int
    """Bit width of the character count field for alphanumeric mode.

    0 means the mode is not supported in this symbol version.
    """

    cc_bits_byte: int
    """Bit width of the character count field for byte mode (0 = unsupported)."""

    m1_half_cw: bool
    """True only for M1: last data 'codeword' is 4 bits; total = 20 data bits."""


# The 8 configurations from ISO 18004:2015 Annex E, in ascending size order.
# This ordering is critical for auto-selection: the first config that fits wins.
_SYMBOL_CONFIGS: Final[tuple[_SymbolConfig, ...]] = (
    # ── M1 / Detection ────────────────────────────────────────────────────────
    # M1 is the smallest Micro QR: 11×11 modules. It supports only numeric
    # encoding and provides error detection only (no correction). The unusual
    # 20-bit data capacity (3 codewords where the last is only a 4-bit nibble)
    # is a consequence of the limited grid space.
    _SymbolConfig(
        version=MicroQRVersion.M1, ecc=MicroQREccLevel.Detection,
        symbol_indicator=0, size=11,
        data_cw=3, ecc_cw=2,
        numeric_cap=5, alpha_cap=0, byte_cap=0,
        terminator_bits=3, mode_indicator_bits=0,
        cc_bits_numeric=3, cc_bits_alpha=0, cc_bits_byte=0,
        m1_half_cw=True,
    ),
    # ── M2 / L ────────────────────────────────────────────────────────────────
    # M2 adds alphanumeric and byte support. ECC-L uses only 5 of its 10
    # codewords for error correction, leaving 5 for data.
    _SymbolConfig(
        version=MicroQRVersion.M2, ecc=MicroQREccLevel.L,
        symbol_indicator=1, size=13,
        data_cw=5, ecc_cw=5,
        numeric_cap=10, alpha_cap=6, byte_cap=4,
        terminator_bits=5, mode_indicator_bits=1,
        cc_bits_numeric=4, cc_bits_alpha=3, cc_bits_byte=4,
        m1_half_cw=False,
    ),
    # ── M2 / M ────────────────────────────────────────────────────────────────
    # M2-M trades 1 data codeword for 1 more ECC codeword vs. M2-L.
    _SymbolConfig(
        version=MicroQRVersion.M2, ecc=MicroQREccLevel.M,
        symbol_indicator=2, size=13,
        data_cw=4, ecc_cw=6,
        numeric_cap=8, alpha_cap=5, byte_cap=3,
        terminator_bits=5, mode_indicator_bits=1,
        cc_bits_numeric=4, cc_bits_alpha=3, cc_bits_byte=4,
        m1_half_cw=False,
    ),
    # ── M3 / L ────────────────────────────────────────────────────────────────
    # M3 doubles the data capacity vs. M2 (15×15 vs. 13×13). Its 17 total
    # codewords split 11/6 for data/ECC at level L.
    _SymbolConfig(
        version=MicroQRVersion.M3, ecc=MicroQREccLevel.L,
        symbol_indicator=3, size=15,
        data_cw=11, ecc_cw=6,
        numeric_cap=23, alpha_cap=14, byte_cap=9,
        terminator_bits=7, mode_indicator_bits=2,
        cc_bits_numeric=5, cc_bits_alpha=4, cc_bits_byte=4,
        m1_half_cw=False,
    ),
    # ── M3 / M ────────────────────────────────────────────────────────────────
    _SymbolConfig(
        version=MicroQRVersion.M3, ecc=MicroQREccLevel.M,
        symbol_indicator=4, size=15,
        data_cw=9, ecc_cw=8,
        numeric_cap=18, alpha_cap=11, byte_cap=7,
        terminator_bits=7, mode_indicator_bits=2,
        cc_bits_numeric=5, cc_bits_alpha=4, cc_bits_byte=4,
        m1_half_cw=False,
    ),
    # ── M4 / L ────────────────────────────────────────────────────────────────
    # M4 is the largest Micro QR: 17×17 modules, 24 total codewords.
    # At L it can hold up to 35 numeric, 21 alphanumeric, or 15 byte chars.
    _SymbolConfig(
        version=MicroQRVersion.M4, ecc=MicroQREccLevel.L,
        symbol_indicator=5, size=17,
        data_cw=16, ecc_cw=8,
        numeric_cap=35, alpha_cap=21, byte_cap=15,
        terminator_bits=9, mode_indicator_bits=3,
        cc_bits_numeric=6, cc_bits_alpha=5, cc_bits_byte=5,
        m1_half_cw=False,
    ),
    # ── M4 / M ────────────────────────────────────────────────────────────────
    _SymbolConfig(
        version=MicroQRVersion.M4, ecc=MicroQREccLevel.M,
        symbol_indicator=6, size=17,
        data_cw=14, ecc_cw=10,
        numeric_cap=30, alpha_cap=18, byte_cap=13,
        terminator_bits=9, mode_indicator_bits=3,
        cc_bits_numeric=6, cc_bits_alpha=5, cc_bits_byte=5,
        m1_half_cw=False,
    ),
    # ── M4 / Q ────────────────────────────────────────────────────────────────
    # Q is the highest ECC level in Micro QR — only M4 is large enough to
    # afford it. At Q, 14 of 24 codewords are ECC, leaving only 10 for data.
    _SymbolConfig(
        version=MicroQRVersion.M4, ecc=MicroQREccLevel.Q,
        symbol_indicator=7, size=17,
        data_cw=10, ecc_cw=14,
        numeric_cap=21, alpha_cap=13, byte_cap=9,
        terminator_bits=9, mode_indicator_bits=3,
        cc_bits_numeric=6, cc_bits_alpha=5, cc_bits_byte=5,
        m1_half_cw=False,
    ),
)

# ============================================================================
# RS generator polynomials (compile-time constants)
# ============================================================================
#
# Monic RS generator polynomials for GF(256)/0x11D with b=0 convention.
# g(x) = (x + α^0)(x + α^1) ··· (x + α^{n-1}).
#
# Only the ECC codeword counts {2, 5, 6, 8, 10, 14} are needed for Micro QR.
# These are identical to the polynomials used in regular QR Code for the same
# block sizes — the field and generator convention is shared.
#
# Each entry is the full polynomial including the leading 1 (monic):
# length = ecc_cw + 1, highest-degree coefficient first.

_RS_GENERATORS: Final[dict[int, tuple[int, ...]]] = {
    # g(x) = (x + α^0)(x + α^1) = x^2 + 3x + 2
    2:  (0x01, 0x03, 0x02),
    # g(x) = product of (x + α^i) for i=0..4
    5:  (0x01, 0x1f, 0xf6, 0x44, 0xd9, 0x68),
    # g(x) = product of (x + α^i) for i=0..5
    6:  (0x01, 0x3f, 0x4e, 0x17, 0x9b, 0x05, 0x37),
    # g(x) = product of (x + α^i) for i=0..7
    8:  (0x01, 0x63, 0x0d, 0x60, 0x6d, 0x5b, 0x10, 0xa2, 0xa3),
    # g(x) = product of (x + α^i) for i=0..9
    10: (0x01, 0xf6, 0x75, 0xa8, 0xd0, 0xc3, 0xe3, 0x36, 0xe1, 0x3c, 0x45),
    # g(x) = product of (x + α^i) for i=0..13
    14: (0x01, 0xf6, 0x9a, 0x60, 0x97, 0x8a, 0xf1, 0xa4, 0xa1, 0x8e,
         0xfc, 0x7a, 0x52, 0xad, 0xac),
}

# ============================================================================
# Pre-computed format information lookup table
# ============================================================================
#
# All 32 format words (8 symbol_indicators × 4 mask patterns), pre-computed
# and XOR-masked with 0x4445 as required by Micro QR.
#
# Format word structure (15 bits):
#   [symbol_indicator (3b)] [mask_pattern (2b)] [BCH-10 remainder]
# then XOR with 0x4445 (Micro QR-specific, NOT 0x5412 like regular QR).
#
# The XOR ensures a Micro QR symbol cannot be confused with a regular QR
# symbol by a scanner — the format bits look distinct.
#
# Indexed as _FORMAT_TABLE[symbol_indicator][mask_pattern].

_FORMAT_TABLE: Final[tuple[tuple[int, ...], ...]] = (
    (0x4445, 0x4172, 0x4E2B, 0x4B1C),  # M1 (symbol_indicator=0)
    (0x5528, 0x501F, 0x5F46, 0x5A71),  # M2-L (symbol_indicator=1)
    (0x6649, 0x637E, 0x6C27, 0x6910),  # M2-M (symbol_indicator=2)
    (0x7764, 0x7253, 0x7D0A, 0x783D),  # M3-L (symbol_indicator=3)
    (0x06DE, 0x03E9, 0x0CB0, 0x0987),  # M3-M (symbol_indicator=4)
    (0x17F3, 0x12C4, 0x1D9D, 0x18AA),  # M4-L (symbol_indicator=5)
    (0x24B2, 0x2185, 0x2EDC, 0x2BEB),  # M4-M (symbol_indicator=6)
    (0x359F, 0x30A8, 0x3FF1, 0x3AC6),  # M4-Q (symbol_indicator=7)
)

# ============================================================================
# Encoding mode constants and helpers
# ============================================================================

# The 45-character set used by alphanumeric mode.
# Same as regular QR Code. Pairs are packed into 11 bits; singles use 6.
_ALPHANUM_CHARS: Final = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:"

_MODE_NUMERIC: Final = "numeric"
_MODE_ALPHA: Final = "alphanumeric"
_MODE_BYTE: Final = "byte"


def _select_mode(input_str: str, cfg: _SymbolConfig) -> str:
    """Select the most compact encoding mode for the input.

    Priority: numeric > alphanumeric > byte.

    We try each mode in order from most compact (numeric) to least compact
    (byte). The first mode that:
    1. Can represent all characters in the input, AND
    2. Is supported by the given symbol version

    is returned.

    This greedy single-mode selection is sufficient for the Micro QR use
    case — mixed-mode segments are future work.

    Parameters
    ----------
    input_str : str
        The raw input string.
    cfg : _SymbolConfig
        Symbol configuration to check mode availability against.

    Returns
    -------
    str
        One of ``_MODE_NUMERIC``, ``_MODE_ALPHA``, ``_MODE_BYTE``.

    Raises
    ------
    UnsupportedModeError
        If no mode is available for this input in this symbol version.
    """
    # Numeric: every character must be an ASCII digit.
    is_numeric = all(c.isdigit() for c in input_str) if input_str else True
    if is_numeric and cfg.cc_bits_numeric > 0:
        return _MODE_NUMERIC

    # Alphanumeric: every character must be in the 45-char set.
    is_alpha = all(c in _ALPHANUM_CHARS for c in input_str)
    if is_alpha and cfg.alpha_cap > 0:
        return _MODE_ALPHA

    # Byte: raw UTF-8 bytes. Always encodable if the symbol supports byte mode.
    if cfg.byte_cap > 0:
        return _MODE_BYTE

    raise UnsupportedModeError(
        f"No encoding mode available for input in {cfg.version}-{cfg.ecc}. "
        f"Input requires byte mode (has non-alphanumeric characters) but "
        f"{cfg.version} only supports "
        + ("numeric only." if cfg.alpha_cap == 0 else "numeric and alphanumeric.")
    )


def _mode_indicator_value(mode: str, cfg: _SymbolConfig) -> int:
    """Return the mode indicator bits for the given mode and symbol version.

    Micro QR uses fewer mode indicator bits than regular QR:
    - M1: 0 bits (implicit numeric — no indicator needed, there's only one mode)
    - M2: 1 bit  (0=numeric, 1=alphanumeric)
    - M3: 2 bits (00=numeric, 01=alphanumeric, 10=byte)
    - M4: 3 bits (000=numeric, 001=alphanumeric, 010=byte, 011=kanji)
    """
    bits = cfg.mode_indicator_bits
    if bits == 0:
        return 0  # M1: no indicator
    if bits == 1:
        return 0 if mode == _MODE_NUMERIC else 1
    if bits == 2:
        return {_MODE_NUMERIC: 0b00, _MODE_ALPHA: 0b01, _MODE_BYTE: 0b10}[mode]
    # bits == 3 (M4)
    return {_MODE_NUMERIC: 0b000, _MODE_ALPHA: 0b001, _MODE_BYTE: 0b010}[mode]


def _char_count_bits(mode: str, cfg: _SymbolConfig) -> int:
    """Return the width of the character count field in bits."""
    if mode == _MODE_NUMERIC:
        return cfg.cc_bits_numeric
    if mode == _MODE_ALPHA:
        return cfg.cc_bits_alpha
    return cfg.cc_bits_byte  # _MODE_BYTE


# ============================================================================
# Bit writer — accumulates bits MSB-first
# ============================================================================


class _BitWriter:
    """Accumulate bits MSB-first, then flush to a byte sequence.

    QR and Micro QR use big-endian bit ordering within each codeword.
    ``write(value, count)`` appends the ``count`` least-significant bits
    of ``value``, MSB first.

    Example::

        w = _BitWriter()
        w.write(0b101, 3)    # appends bits [1, 0, 1]
        w.write(0b11, 2)     # appends bits [1, 1]
        assert w.to_bytes() == [0b10111000]  # 0xB8 padded to byte boundary
    """

    def __init__(self) -> None:
        self._bits: list[int] = []  # each element is 0 or 1

    def write(self, value: int, count: int) -> None:
        """Append the ``count`` least-significant bits of ``value``, MSB first."""
        for i in range(count - 1, -1, -1):
            self._bits.append((value >> i) & 1)

    def bit_len(self) -> int:
        """Return the number of bits written so far."""
        return len(self._bits)

    def to_bytes(self) -> list[int]:
        """Flush bits to a byte list, MSB-first, zero-padded to byte boundary."""
        result: list[int] = []
        i = 0
        while i < len(self._bits):
            byte = 0
            for j in range(8):
                bit = self._bits[i + j] if i + j < len(self._bits) else 0
                byte = (byte << 1) | bit
            result.append(byte)
            i += 8
        return result

    def to_bit_list(self) -> list[int]:
        """Return a copy of the raw bit list."""
        return list(self._bits)


# ============================================================================
# Data encoding helpers
# ============================================================================


def _encode_numeric(input_str: str, w: _BitWriter) -> None:
    """Encode a string of digits in numeric mode.

    Groups of 3 digits → 10 bits (decimal value 000–999).
    Remaining pair → 7 bits (decimal value 00–99).
    Single remaining digit → 4 bits (decimal value 0–9).

    This is identical to regular QR Code numeric mode. The packing is
    efficient because 2^10 = 1024 > 999, so 10 bits can represent any
    3-digit group; similarly 2^7 = 128 > 99 for pairs and 2^4 = 16 > 9
    for single digits.

    Example::

        "12345" → "123" + "45" → 10+7 = 17 bits
        "12"    → "12"          → 7 bits
        "1"     → "1"           → 4 bits
    """
    digits = [ord(c) - ord("0") for c in input_str]
    i = 0
    while i + 2 < len(digits):
        # Three-digit group: pack into 10 bits.
        w.write(digits[i] * 100 + digits[i + 1] * 10 + digits[i + 2], 10)
        i += 3
    if i + 1 < len(digits):
        # Two-digit pair: pack into 7 bits.
        w.write(digits[i] * 10 + digits[i + 1], 7)
        i += 2
    if i < len(digits):
        # Single digit: 4 bits.
        w.write(digits[i], 4)


def _encode_alphanumeric(input_str: str, w: _BitWriter) -> None:
    """Encode a string in alphanumeric mode.

    Pairs of characters → 11 bits: first_index * 45 + second_index.
    Single trailing character → 6 bits: its index in the 45-char set.

    The 45-character set (0-indexed):
        0–9  → 0–9   (digits)
        A–Z  → 10–35 (uppercase letters)
        SP   → 36    (space)
        $%*+-./:  → 37–44

    Packing two characters into 11 bits: 45 * 45 = 2025 ≤ 2048 = 2^11.
    Packing one character into 6 bits: 44 ≤ 63 = 2^6 − 1.

    Example::

        "AC-3" → pair "AC" = 10*45+12=462 → 11 bits;
                  pair "-3" = 41*45+3=1848 → 11 bits.
    """
    indices = [_ALPHANUM_CHARS.index(c) for c in input_str]
    i = 0
    while i + 1 < len(indices):
        w.write(indices[i] * 45 + indices[i + 1], 11)
        i += 2
    if i < len(indices):
        w.write(indices[i], 6)


def _encode_byte_mode(input_str: str, w: _BitWriter) -> None:
    """Encode a string in byte mode.

    Each UTF-8 byte of the string is encoded as an 8-bit value.
    Multi-byte UTF-8 characters contribute multiple bytes to the stream
    (and each byte counts separately in the character count field).

    ISO 18004 calls this mode "8-bit byte" or "ISO-8859-1 mode", but in
    practice UTF-8 encoding is accepted by all modern QR scanners.
    """
    for b in input_str.encode("utf-8"):
        w.write(b, 8)


# ============================================================================
# RS encoder
# ============================================================================


def _rs_encode(data: list[int], ecc_count: int) -> list[int]:
    """Compute Reed-Solomon ECC bytes over GF(256)/0x11D.

    Uses the LFSR (shift-register) polynomial division algorithm:

    .. code-block::

        ecc = [0] × n      ← n = ecc_count, the register
        for each data byte b:
            feedback = b XOR ecc[0]
            shift ecc left by one (drop ecc[0], append 0)
            for i in 0..n-1:
                ecc[i] XOR= gf_multiply(generator[i+1], feedback)
        result = ecc

    This computes the remainder of D(x) · x^n mod G(x) over GF(256).
    The b=0 convention means the first root is α^0 = 1 (same as regular QR).

    Parameters
    ----------
    data : list[int]
        Data codeword bytes.
    ecc_count : int
        Number of ECC codewords to produce (2, 5, 6, 8, 10, or 14).

    Returns
    -------
    list[int]
        The ECC byte sequence of length ``ecc_count``.
    """
    generator = _RS_GENERATORS[ecc_count]
    n = len(generator) - 1  # degree = ecc_count
    rem = [0] * n

    for b in data:
        feedback = b ^ rem[0]
        # Shift the register left by one position.
        rem = rem[1:] + [0]
        if feedback != 0:
            for i in range(n):
                rem[i] ^= gf.multiply(generator[i + 1], feedback)

    return rem


# ============================================================================
# Symbol configuration selector
# ============================================================================


def _select_config(
    input_str: str,
    version: str | None,
    ecc: str | None,
) -> _SymbolConfig:
    """Find the smallest symbol configuration that can hold the input.

    Iterates through ``_SYMBOL_CONFIGS`` in order (M1 → M4) and returns
    the first config where:
    1. The version matches (if specified).
    2. The ECC level matches (if specified).
    3. A supported encoding mode exists for the input.
    4. The input length does not exceed the mode capacity.

    Parameters
    ----------
    input_str : str
        The raw input string.
    version : str or None
        Force a specific symbol version, or ``None`` to auto-select.
    ecc : str or None
        Force a specific ECC level, or ``None`` to auto-select.

    Returns
    -------
    _SymbolConfig
        The matching configuration.

    Raises
    ------
    ECCNotAvailableError
        If no config matches the requested version+ECC combination.
    InputTooLongError
        If the input does not fit in any matching config.
    """
    candidates = [
        cfg for cfg in _SYMBOL_CONFIGS
        if (version is None or cfg.version == version)
        and (ecc is None or cfg.ecc == ecc)
    ]

    if not candidates:
        raise ECCNotAvailableError(
            f"No Micro QR symbol supports version={version}, ecc={ecc}. "
            f"Valid combinations: M1/Detection, M2/L, M2/M, M3/L, M3/M, "
            f"M4/L, M4/M, M4/Q."
        )

    for cfg in candidates:
        try:
            mode = _select_mode(input_str, cfg)
        except UnsupportedModeError:
            continue

        # Byte mode counts UTF-8 bytes, not Unicode characters.
        if mode == _MODE_BYTE:
            length = len(input_str.encode("utf-8"))
        else:
            length = len(input_str)
        cap = {
            _MODE_NUMERIC: cfg.numeric_cap,
            _MODE_ALPHA: cfg.alpha_cap,
            _MODE_BYTE: cfg.byte_cap,
        }[mode]

        if cap > 0 and length <= cap:
            return cfg

    raise InputTooLongError(
        f"Input (length {len(input_str)}) does not fit in any Micro QR symbol "
        f"(version={version}, ecc={ecc}). "
        f"Maximum is 35 numeric characters in M4-L. "
        f"Consider using regular QR Code for longer inputs."
    )


# ============================================================================
# Data codeword assembly
# ============================================================================


def _build_data_codewords(
    input_str: str,
    cfg: _SymbolConfig,
    mode: str,
) -> list[int]:
    """Build the complete data codeword byte sequence.

    For all symbols except M1:
        [mode indicator] [char count] [data bits]
        [terminator] [byte-align] [0xEC/0x11 fill]
        → exactly ``cfg.data_cw`` bytes.

    For M1 (m1_half_cw = True):
        Total capacity = 20 bits = 2 full bytes + 4-bit nibble.
        The RS encoder receives 3 bytes where byte[2] has data in the upper
        4 bits and forced zeros in the lower 4 bits.

    Terminator
    ----------
    A terminator of ``cfg.terminator_bits`` zero bits is appended after the
    encoded data. It is truncated if the remaining capacity is already less
    than the full terminator width. This allows the encoder to fill the
    symbol exactly without overflow.

    Padding codewords
    -----------------
    After the terminator and byte-alignment zero bits, the remaining data
    codewords are filled with the alternating pattern 0xEC, 0x11, 0xEC, 0x11…
    This is identical to regular QR Code padding.

    Parameters
    ----------
    input_str : str
        Raw input string.
    cfg : _SymbolConfig
        Symbol configuration (determines field widths, capacity, etc.).
    mode : str
        Selected encoding mode (``_MODE_NUMERIC``, etc.).

    Returns
    -------
    list[int]
        Data codeword bytes (length = ``cfg.data_cw``).
    """
    # M1 special case: total data capacity is 20 bits (not 24).
    total_bits = cfg.data_cw * 8 - 4 if cfg.m1_half_cw else cfg.data_cw * 8

    w = _BitWriter()

    # Mode indicator (0/1/2/3 bits depending on symbol version).
    if cfg.mode_indicator_bits > 0:
        w.write(_mode_indicator_value(mode, cfg), cfg.mode_indicator_bits)

    # Character count: number of characters (or bytes for byte mode).
    char_count = (
        len(input_str.encode("utf-8")) if mode == _MODE_BYTE else len(input_str)
    )
    w.write(char_count, _char_count_bits(mode, cfg))

    # Encoded data bits.
    if mode == _MODE_NUMERIC:
        _encode_numeric(input_str, w)
    elif mode == _MODE_ALPHA:
        _encode_alphanumeric(input_str, w)
    else:
        _encode_byte_mode(input_str, w)

    # Terminator: up to terminator_bits zero bits, truncated if capacity exhausted.
    remaining = total_bits - w.bit_len()
    if remaining > 0:
        term = min(cfg.terminator_bits, remaining)
        w.write(0, term)

    # ── M1 special packing ───────────────────────────────────────────────────
    # M1 uses 20 data bits (not 24). The third "codeword" is only 4 bits.
    # We pack into exactly 20 bits, then split as: byte0, byte1, nibble_byte.
    # The nibble_byte has the 4 data bits in its high nibble and zeros in the
    # low nibble. This ensures the RS encoder sees a valid byte stream.
    if cfg.m1_half_cw:
        bits = w.to_bit_list()
        bits = bits[:20]               # truncate to 20 bits
        bits += [0] * (20 - len(bits)) # zero-pad to exactly 20 bits
        b0 = (
            (bits[0]  << 7) | (bits[1]  << 6) | (bits[2]  << 5) | (bits[3]  << 4)
            | (bits[4]  << 3) | (bits[5]  << 2) | (bits[6]  << 1) | bits[7]
        )
        b1 = (
            (bits[8]  << 7) | (bits[9]  << 6) | (bits[10] << 5) | (bits[11] << 4)
            | (bits[12] << 3) | (bits[13] << 2) | (bits[14] << 1) | bits[15]
        )
        # Upper nibble = data bits 16-19; lower nibble = 0 (forced zeros).
        b2 = (
            (bits[16] << 7) | (bits[17] << 6) | (bits[18] << 5) | (bits[19] << 4)
        )
        return [b0, b1, b2]

    # ── Standard packing (M2, M3, M4) ────────────────────────────────────────
    # Pad to next byte boundary with zero bits.
    rem = w.bit_len() % 8
    if rem != 0:
        w.write(0, 8 - rem)

    codewords = w.to_bytes()

    # Fill remaining data codewords with alternating 0xEC / 0x11.
    # This padding pattern is specified by ISO 18004 and is common to both
    # regular QR and Micro QR. The alternation is arbitrary but prevents
    # long runs of identical bytes that could degrade penalty scoring.
    pad = 0xEC
    while len(codewords) < cfg.data_cw:
        codewords.append(pad)
        pad = 0x11 if pad == 0xEC else 0xEC

    return codewords[:cfg.data_cw]


# ============================================================================
# Working grid — mutable 2D grid for building the symbol
# ============================================================================


class _WorkGrid:
    """Mutable 2D module grid with a parallel reservation map.

    The final ``ModuleGrid`` is immutable (frozen dataclass), but building
    the symbol requires placing and overwriting modules many times. This
    class provides a mutable intermediate representation.

    The ``reserved`` map marks structural modules (finder pattern,
    separators, timing strips, format information) that must not be
    overwritten by data placement or masking.

    Parameters
    ----------
    size : int
        Symbol side length in modules.
    """

    def __init__(self, size: int) -> None:
        self.size = size
        # modules[row][col]: True = dark, False = light
        self.modules: list[list[bool]] = [[False] * size for _ in range(size)]
        # reserved[row][col]: True = structural (must not be overwritten)
        self.reserved: list[list[bool]] = [[False] * size for _ in range(size)]

    def set(self, row: int, col: int, dark: bool, *, reserve: bool = False) -> None:
        """Set a module value and optionally mark it as reserved."""
        self.modules[row][col] = dark
        if reserve:
            self.reserved[row][col] = True

    def to_module_grid(self) -> ModuleGrid:
        """Convert to an immutable ``ModuleGrid``."""
        rows = len(self.modules)
        cols = len(self.modules[0]) if rows > 0 else 0
        grid = make_module_grid(rows, cols)
        for r in range(rows):
            for c in range(cols):
                if self.modules[r][c]:
                    grid = set_module(grid, r, c, True)
        return grid


# ============================================================================
# Grid structural placement helpers
# ============================================================================


def _place_finder(g: _WorkGrid) -> None:
    """Place the 7×7 finder pattern at the top-left corner.

    The finder pattern is the same as regular QR Code — a 7×7 square with
    a 3×3 dark core, a ring of light modules, and a dark outer border.
    Scanners use this pattern's distinctive 1:1:3:1:1 dark-light-dark ratio
    to locate the symbol and determine scale.

    In Micro QR there is only ONE finder pattern (at the top-left), not
    three. This saves space at the cost of some orientation robustness —
    the single-corner placement is itself unambiguous.

    Pattern (■ = dark, □ = light):

    .. code-block::

        ■ ■ ■ ■ ■ ■ ■
        ■ □ □ □ □ □ ■
        ■ □ ■ ■ ■ □ ■
        ■ □ ■ ■ ■ □ ■
        ■ □ ■ ■ ■ □ ■
        ■ □ □ □ □ □ ■
        ■ ■ ■ ■ ■ ■ ■
    """
    for dr in range(7):
        for dc in range(7):
            on_border = dr in (0, 6) or dc in (0, 6)
            in_core = 2 <= dr <= 4 and 2 <= dc <= 4
            g.set(dr, dc, on_border or in_core, reserve=True)


def _place_separator(g: _WorkGrid) -> None:
    """Place the L-shaped separator around the finder pattern.

    Unlike regular QR Code which surrounds all three finder patterns with
    a full border, Micro QR's single finder only needs separation on its
    BOTTOM (row 7, cols 0–7) and RIGHT (col 7, rows 0–7) sides. The top
    and left sides of the finder ARE the symbol boundary.

    All separator modules are always light (False).
    """
    for i in range(8):
        g.set(7, i, False, reserve=True)  # bottom row of separator
        g.set(i, 7, False, reserve=True)  # right column of separator


def _place_timing(g: _WorkGrid) -> None:
    """Place timing pattern extensions along row 0 and col 0.

    Timing patterns alternate dark/light starting with dark at position 0.
    In Micro QR, timing runs along the OUTER edges (row 0 and col 0),
    unlike regular QR where timing is at row 6 and col 6.

    Positions 0–6 on each strip are already covered by the finder pattern.
    Position 7 is the separator (always light). We place timing starting
    at position 8 outward, where "even index → dark" holds consistently.

    For M4 (17×17), the extended timing positions are:
    - Row 0: cols 8–16  → dark,light,dark,light,dark,light,dark,light,dark
    - Col 0: rows 8–16  → same pattern
    """
    sz = g.size
    for c in range(8, sz):
        g.set(0, c, c % 2 == 0, reserve=True)
    for r in range(8, sz):
        g.set(r, 0, r % 2 == 0, reserve=True)


def _reserve_format_info(g: _WorkGrid) -> None:
    """Reserve the 15 format information module positions.

    Format information occupies an L-shaped strip adjacent to the separator:
    - Row 8, cols 1–8  → 8 modules (bits f14 down to f7, MSB first)
    - Col 8, rows 1–7  → 7 modules (bits f6 down to f0, f6 at row 7)

    These modules are reserved now (set to light as placeholder) and written
    with actual format bits AFTER mask selection.
    """
    for c in range(1, 9):
        g.set(8, c, False, reserve=True)
    for r in range(1, 8):
        g.set(r, 8, False, reserve=True)


def _build_empty_grid(cfg: _SymbolConfig) -> _WorkGrid:
    """Initialize a grid with all structural modules placed."""
    g = _WorkGrid(cfg.size)
    _place_finder(g)
    _place_separator(g)
    _place_timing(g)
    _reserve_format_info(g)
    return g


# ============================================================================
# Data placement — two-column zigzag
# ============================================================================


def _place_bits(g: _WorkGrid, bits: list[bool]) -> None:
    """Place data and ECC bits into the grid via two-column zigzag.

    The zigzag scans from the **bottom-right** corner of the symbol, moving
    two columns left at a time, alternating upward and downward sweeps.
    Reserved modules (finder, separator, timing, format info) are skipped.

    Unlike regular QR Code, there is NO timing column at col 6 to hop over.
    Micro QR's timing is at col 0, which is always reserved and therefore
    skipped automatically by the reserved-module check.

    The scan terminates when ``col < 1`` because the leftmost two-column
    strip covers col 1 and col 0; col 0 is entirely reserved (timing), so
    all data bits that would go there are simply skipped.

    Zigzag walk for M4 (17×17), showing column-pair ordering::

        col-pair 16,15  → upward   (rows 16 down to 0)
        col-pair 14,13  → downward (rows 0 up to 16)
        col-pair 12,11  → upward
        ...
        col-pair 2,1    → depends on direction at that point

    Parameters
    ----------
    g : _WorkGrid
        The working grid (already has structural modules placed).
    bits : list[bool]
        Bit stream to place (True = dark, False = light).
    """
    sz = g.size
    bit_idx = 0
    going_up = True

    col = sz - 1
    while col >= 1:
        for vi in range(sz):
            row = (sz - 1 - vi) if going_up else vi
            # Visit right column of the pair first, then left.
            for dc in range(2):
                c = col - dc
                if g.reserved[row][c]:
                    continue
                g.modules[row][c] = bits[bit_idx] if bit_idx < len(bits) else False
                bit_idx += 1
        going_up = not going_up
        col -= 2


# ============================================================================
# Masking
# ============================================================================


def _mask_condition(mask_idx: int, row: int, col: int) -> bool:
    """Test whether mask pattern ``mask_idx`` applies to module (row, col).

    Micro QR uses only 4 mask patterns (vs. 8 in regular QR):

    +-------+---------------------------------+
    | Index | Condition (flip if true)        |
    +=======+=================================+
    | 0     | (row + col) mod 2 == 0          |
    +-------+---------------------------------+
    | 1     | row mod 2 == 0                  |
    +-------+---------------------------------+
    | 2     | col mod 3 == 0                  |
    +-------+---------------------------------+
    | 3     | (row + col) mod 3 == 0          |
    +-------+---------------------------------+

    These are the first four of regular QR's eight patterns.  The more
    complex patterns (4–7) are absent because the smaller symbol size
    means simpler patterns suffice to break up degenerate sequences.
    """
    if mask_idx == 0:
        return (row + col) % 2 == 0
    if mask_idx == 1:
        return row % 2 == 0
    if mask_idx == 2:
        return col % 3 == 0
    # mask_idx == 3
    return (row + col) % 3 == 0


def _apply_mask(
    modules: list[list[bool]],
    reserved: list[list[bool]],
    sz: int,
    mask_idx: int,
) -> list[list[bool]]:
    """Apply a mask pattern to non-reserved modules. Returns a new grid."""
    result = [list(row) for row in modules]
    for r in range(sz):
        for c in range(sz):
            if not reserved[r][c]:
                result[r][c] = modules[r][c] != _mask_condition(mask_idx, r, c)
    return result


def _write_format_info_into(modules: list[list[bool]], fmt: int) -> None:
    """Write a 15-bit format word into the format information positions.

    Placement (MSB at row 8, col 1):
    - Row 8, cols 1–8: bits f14 (MSB) down to f7
    - Col 8, rows 7 down to 1: bits f6 down to f0 (LSB)

    The "upward" direction on col 8 (row 7 = f6, row 1 = f0) means the
    LSB is nearest to the finder corner — this is a Micro QR convention.

    Note: Micro QR has only ONE copy of the format information (unlike
    regular QR which places two copies). This simplifies placement but
    means there is no redundancy if format modules are damaged.
    """
    # Row 8, cols 1–8: bits f14 down to f7 (8 bits, MSB first)
    for i in range(8):
        modules[8][1 + i] = ((fmt >> (14 - i)) & 1) == 1
    # Col 8, rows 7 down to 1: bits f6 down to f0 (7 bits)
    for i in range(7):
        modules[7 - i][8] = ((fmt >> (6 - i)) & 1) == 1


# ============================================================================
# Penalty scoring (4 rules, same as regular QR)
# ============================================================================


def _compute_penalty(modules: list[list[bool]], sz: int) -> int:
    """Compute the 4-rule penalty score for a masked module grid.

    The same four rules as regular QR Code:

    Rule 1 — Adjacent run penalty
        Scan each row and column. For runs of ≥ 5 consecutive same-colour
        modules, add (run_length − 2) to the penalty.
        Run of 5 → +3, run of 6 → +4, etc.

    Rule 2 — 2×2 block penalty
        For each 2×2 square with all four modules the same colour, add 3.

    Rule 3 — Finder-pattern-like sequences
        Scan all rows and columns for the 11-module sequence
        ``1 0 1 1 1 0 1 0 0 0 0`` or its reverse. Each occurrence adds 40.

    Rule 4 — Dark-module proportion
        ``dark_pct = dark_count * 100 / total`` (integer division).
        ``prev5 = largest multiple of 5 ≤ dark_pct``
        ``penalty += min(|prev5 − 50|, |prev5 + 5 − 50|) / 5 × 10``

    The mask with the lowest total penalty is selected.

    Parameters
    ----------
    modules : list[list[bool]]
        The masked module grid (2D list of booleans).
    sz : int
        Symbol side length.

    Returns
    -------
    int
        Total penalty score.
    """
    penalty = 0

    # ── Rule 1: runs of ≥ 5 same-colour modules ──────────────────────────────
    for a in range(sz):
        for horizontal in (True, False):
            # Scan one row (if horizontal) or one column (if vertical).
            run = 1
            prev = modules[a][0] if horizontal else modules[0][a]
            for i in range(1, sz):
                cur = modules[a][i] if horizontal else modules[i][a]
                if cur == prev:
                    run += 1
                else:
                    if run >= 5:
                        penalty += run - 2
                    run = 1
                    prev = cur
            if run >= 5:
                penalty += run - 2

    # ── Rule 2: 2×2 same-colour blocks ───────────────────────────────────────
    for r in range(sz - 1):
        for c in range(sz - 1):
            d = modules[r][c]
            if (d == modules[r][c + 1]
                    and d == modules[r + 1][c]
                    and d == modules[r + 1][c + 1]):
                penalty += 3

    # ── Rule 3: finder-pattern-like 11-module sequences ──────────────────────
    # These two patterns are the regular and reversed finder-locator sequences.
    # Their appearance in the data region confuses scanners.
    p1 = (1, 0, 1, 1, 1, 0, 1, 0, 0, 0, 0)
    p2 = (0, 0, 0, 0, 1, 0, 1, 1, 1, 0, 1)
    limit = sz - 11  # last starting position for an 11-module window
    if sz >= 11:
        for a in range(sz):
            for b in range(limit + 1):
                mh1 = mh2 = mv1 = mv2 = True
                for k in range(11):
                    bh = 1 if modules[a][b + k] else 0
                    bv = 1 if modules[b + k][a] else 0
                    if bh != p1[k]:
                        mh1 = False
                    if bh != p2[k]:
                        mh2 = False
                    if bv != p1[k]:
                        mv1 = False
                    if bv != p2[k]:
                        mv2 = False
                if mh1:
                    penalty += 40
                if mh2:
                    penalty += 40
                if mv1:
                    penalty += 40
                if mv2:
                    penalty += 40

    # ── Rule 4: dark proportion deviation from 50% ───────────────────────────
    # A balanced symbol (50% dark) is easiest for scanners to process.
    # Deviation from 50% is penalized in steps of 5 percentage points.
    dark = sum(1 for row in modules for d in row if d)
    total = sz * sz
    dark_pct = (dark * 100) // total
    prev5 = (dark_pct // 5) * 5
    next5 = prev5 + 5
    r4 = min(abs(prev5 - 50), abs(next5 - 50))
    penalty += (r4 // 5) * 10

    return penalty


# ============================================================================
# Core encode function
# ============================================================================


def encode(
    input_str: str,
    version: str | None = None,
    ecc: str | None = None,
) -> ModuleGrid:
    """Encode a string to a Micro QR Code ``ModuleGrid``.

    Automatically selects the smallest symbol (M1..M4) and ECC level that
    can hold the input. Pass ``version`` and/or ``ecc`` to override.

    The full pipeline:

    1. Select the smallest (version, ECC) configuration.
    2. Select the most compact encoding mode (numeric > alphanumeric > byte).
    3. Build the data codeword byte sequence.
    4. Compute Reed-Solomon ECC using GF(256)/0x11D.
    5. Flatten to a bit stream (MSB-first per codeword).
    6. Initialize grid with finder, separator, timing, and reserved format info.
    7. Place bits via two-column zigzag from bottom-right.
    8. Evaluate all 4 mask patterns, compute penalty for each.
    9. Apply the best mask (lowest penalty, ties broken by lower index).
    10. Write the 15-bit format information into the reserved positions.
    11. Return the final immutable ``ModuleGrid``.

    Parameters
    ----------
    input_str : str
        The string to encode.
    version : str or None
        One of ``MicroQRVersion.M1`` / ``"M1"`` .. ``"M4"``.  ``None``
        auto-selects the smallest version that fits.
    ecc : str or None
        One of ``MicroQREccLevel.Detection`` / ``"L"`` / ``"M"`` / ``"Q"``.
        ``None`` auto-selects.  Note: ``Detection`` is only valid for M1;
        ``Q`` is only valid for M4.

    Returns
    -------
    ModuleGrid
        A ``rows × cols`` boolean grid where ``True`` = dark module.

    Raises
    ------
    InputTooLongError
        If the input exceeds M4 capacity (35 numeric chars).
    ECCNotAvailableError
        If the requested version + ECC combination does not exist.
    UnsupportedModeError
        If the input requires a mode not available in the chosen symbol.

    Examples
    --------
    Minimal numeric symbol::

        grid = encode("1")
        assert grid.rows == 11  # M1 = 11×11

    Auto-selecting M2 for alphanumeric::

        grid = encode("HELLO")
        assert grid.rows == 13  # M2 = 13×13

    Forcing M4-L for a URL::

        grid = encode("https://a.b", version="M4", ecc="L")
        assert grid.rows == 17
    """
    cfg = _select_config(input_str, version, ecc)
    mode = _select_mode(input_str, cfg)

    # ── Step 1: Build data codewords ─────────────────────────────────────────
    data_cw = _build_data_codewords(input_str, cfg, mode)

    # ── Step 2: Compute RS ECC ────────────────────────────────────────────────
    ecc_cw = _rs_encode(data_cw, cfg.ecc_cw)

    # ── Step 3: Flatten to bit stream ────────────────────────────────────────
    # Concatenate data + ECC codewords, then extract bits MSB-first.
    # M1 special case: the last data codeword contributes only 4 bits
    # (the upper nibble — the lower nibble is forced to zero and is NOT
    # placed in the grid).
    final_cw = data_cw + ecc_cw
    bits: list[bool] = []
    for cw_idx, cw in enumerate(final_cw):
        # M1: last DATA codeword (index data_cw-1) contributes only 4 bits.
        # All ECC codewords contribute 8 bits even in M1.
        is_m1_half = cfg.m1_half_cw and cw_idx == cfg.data_cw - 1
        bits_in_cw = 4 if is_m1_half else 8
        # Extract bits MSB-first from the appropriate bit positions.
        shift_start = 8 - bits_in_cw  # = 4 for half-codeword, 0 for full
        for b in range(bits_in_cw - 1, -1, -1):
            bits.append(((cw >> (b + shift_start)) & 1) == 1)

    # ── Step 4: Initialize grid with structural modules ───────────────────────
    grid = _build_empty_grid(cfg)

    # ── Step 5: Place data bits ───────────────────────────────────────────────
    _place_bits(grid, bits)

    # ── Steps 6–9: Evaluate 4 masks, pick best ───────────────────────────────
    best_mask = 0
    best_penalty = 10 ** 9  # sentinel: very large

    for m in range(4):
        # Apply this mask to non-reserved modules.
        masked = _apply_mask(grid.modules, grid.reserved, cfg.size, m)

        # Write format information for this candidate mask.
        fmt = _FORMAT_TABLE[cfg.symbol_indicator][m]
        _write_format_info_into(masked, fmt)

        # Compute penalty and track the best.
        p = _compute_penalty(masked, cfg.size)
        if p < best_penalty:
            best_penalty = p
            best_mask = m

    # ── Step 10: Finalize with the best mask ─────────────────────────────────
    final_modules = _apply_mask(grid.modules, grid.reserved, cfg.size, best_mask)
    final_fmt = _FORMAT_TABLE[cfg.symbol_indicator][best_mask]
    _write_format_info_into(final_modules, final_fmt)

    # ── Step 11: Build immutable ModuleGrid ───────────────────────────────────
    result = make_module_grid(cfg.size, cfg.size)
    for r in range(cfg.size):
        for c in range(cfg.size):
            if final_modules[r][c]:
                result = set_module(result, r, c, True)

    return result


# ============================================================================
# encode_at — force a specific version and ECC level
# ============================================================================


def encode_at(
    input_str: str,
    version: str,
    ecc: str,
) -> ModuleGrid:
    """Encode to a specific symbol version and ECC level.

    Equivalent to ``encode(input_str, version=version, ecc=ecc)`` with both
    arguments required. Raises ``InputTooLongError`` if the input does not
    fit in the requested version/ECC combination.

    Parameters
    ----------
    input_str : str
        The string to encode.
    version : str
        One of ``"M1"``, ``"M2"``, ``"M3"``, ``"M4"``.
    ecc : str
        One of ``"Detection"``, ``"L"``, ``"M"``, ``"Q"``.

    Returns
    -------
    ModuleGrid
        The encoded symbol as a boolean grid.

    Examples
    --------
    ::

        grid = encode_at("HELLO", "M2", "L")
        assert grid.rows == 13
    """
    return encode(input_str, version=version, ecc=ecc)


# ============================================================================
# layout_grid — ModuleGrid → PaintScene
# ============================================================================


def layout_grid(
    grid: ModuleGrid,
    config: Barcode2DLayoutConfig | None = None,
) -> PaintScene:
    """Convert a ``ModuleGrid`` to a ``PaintScene`` via barcode-2d ``layout()``.

    Defaults to a quiet zone of 2 modules (Micro QR minimum, half of regular
    QR's 4-module requirement). Override with a custom config if needed.

    This function is a thin wrapper around ``barcode_2d.layout()``. Micro QR
    does not implement rendering — it delegates to the shared ``barcode-2d``
    pipeline.

    Parameters
    ----------
    grid : ModuleGrid
        A grid returned by :func:`encode` or :func:`encode_at`.
    config : Barcode2DLayoutConfig or None
        Optional layout configuration.  If ``None``, uses
        ``module_size_px=10``, ``quiet_zone_modules=2``.

    Returns
    -------
    PaintScene
        A ``PaintScene`` ready for the PaintVM backend.
    """
    cfg = config if config is not None else Barcode2DLayoutConfig(
        quiet_zone_modules=2,  # Micro QR minimum (half of regular QR's 4)
        module_size_px=10,
    )
    return _barcode_layout(grid, cfg)


# ============================================================================
# encode_and_layout — convenience: encode + layout in one call
# ============================================================================


def encode_and_layout(
    input_str: str,
    ecc: str | None = None,
    config: Barcode2DLayoutConfig | None = None,
) -> PaintScene:
    """Encode a string and convert the result to a ``PaintScene``.

    Equivalent to ``layout_grid(encode(input_str, ecc=ecc), config=config)``.

    Parameters
    ----------
    input_str : str
        The string to encode.
    ecc : str or None
        ECC level override.  ``None`` auto-selects.
    config : Barcode2DLayoutConfig or None
        Layout configuration.  ``None`` uses the Micro QR default (quiet
        zone = 2).

    Returns
    -------
    PaintScene
        A ``PaintScene`` ready for the PaintVM backend.
    """
    grid = encode(input_str, ecc=ecc)
    return layout_grid(grid, config)


# ============================================================================
# compute_format_word — utility for testing and verification
# ============================================================================


def compute_format_word(symbol_indicator: int, mask_pattern: int) -> int:
    """Return the 15-bit Micro QR format word for the given symbol and mask.

    The format word is looked up from the pre-computed ``_FORMAT_TABLE``, which
    contains the ISO/IEC 18004:2015 Annex E BCH-protected values XOR-masked
    with 0x4445 (the Micro QR format XOR constant).

    Format word structure (15 bits, MSB first):
    ``[symbol_indicator (3b)][mask_pattern (2b)][BCH-10 remainder][XOR 0x4445]``

    The table is the canonical reference — it was independently verified against
    the Rust micro-qr implementation and the ISO spec appendix values.

    Parameters
    ----------
    symbol_indicator : int
        3-bit value (0–7). Maps to the 8 valid (version, ECC) combinations:
        0 = M1/Detection, 1 = M2/L, 2 = M2/M, 3 = M3/L, 4 = M3/M,
        5 = M4/L, 6 = M4/M, 7 = M4/Q.
    mask_pattern : int
        2-bit value (0–3). One of the 4 Micro QR mask patterns.

    Returns
    -------
    int
        The 15-bit format word (after XOR masking with 0x4445).

    Raises
    ------
    ValueError
        If ``symbol_indicator`` is not in 0–7 or ``mask_pattern`` is not in 0–3.
    """
    if not (0 <= symbol_indicator <= 7):
        raise ValueError(f"symbol_indicator must be 0–7, got {symbol_indicator}")
    if not (0 <= mask_pattern <= 3):
        raise ValueError(f"mask_pattern must be 0–3, got {mask_pattern}")
    return _FORMAT_TABLE[symbol_indicator][mask_pattern]


# ============================================================================
# grid_to_string — debugging utility
# ============================================================================


def grid_to_string(grid: ModuleGrid) -> str:
    """Render a ``ModuleGrid`` as a multi-line string of '1' and '0' characters.

    Useful for debugging, snapshot tests, and cross-language comparison.
    Each row is one line; rows are separated by newlines; no trailing newline.

    Example::

        grid = encode("1")
        print(grid_to_string(grid))
        # 11111111100
        # 10000001...
        # ...

    Parameters
    ----------
    grid : ModuleGrid
        The grid to render.

    Returns
    -------
    str
        Multi-line string representation.
    """
    return "\n".join(
        "".join("1" if grid.modules[r][c] else "0" for c in range(grid.cols))
        for r in range(grid.rows)
    )


# ============================================================================
# Public API exports
# ============================================================================

__all__ = [
    # Version
    "__version__",
    # Types
    "MicroQRVersion",
    "MicroQREccLevel",
    # Errors
    "MicroQRError",
    "InputTooLongError",
    "ECCNotAvailableError",
    "UnsupportedModeError",
    "InvalidCharacterError",
    # Encoding functions
    "encode",
    "encode_at",
    "layout_grid",
    "encode_and_layout",
    # Utilities
    "compute_format_word",
    "grid_to_string",
]
