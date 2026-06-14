"""Data Matrix ECC200 encoder — ISO/IEC 16022:2006 compliant.

Data Matrix is a two-dimensional matrix barcode invented in 1989 (originally
"DataCode") and standardised as ISO/IEC 16022:2006.  ECC200 is the modern
variant — using Reed-Solomon over GF(256) — that has displaced the older
ECC000–ECC140 lineage.

Where Data Matrix is used
--------------------------
- **PCBs** — every modern board carries a tiny Data Matrix etched on the
  substrate for traceability through automated assembly lines.
- **Pharmaceuticals** — the US FDA DSCSA mandates Data Matrix on unit-dose
  packages.
- **Aerospace parts** — etched / dot-peened marks survive decades of heat
  and abrasion that would destroy ink-printed labels.
- **Medical devices** — GS1 DataMatrix on surgical instruments and implants.
- **Postage** — USPS registered mail and customs forms.

Key differences from QR Code
----------------------------
+----------------+--------------------+-----------------------+
| Property       | QR Code            | Data Matrix ECC200    |
+================+====================+=======================+
| GF(256) poly   | 0x11D              | 0x12D                 |
+----------------+--------------------+-----------------------+
| RS root start  | b = 0 (α⁰..)       | b = 1 (α¹..)          |
+----------------+--------------------+-----------------------+
| Finder         | three corner       | one L-shape           |
|                | squares            | (left + bottom)       |
+----------------+--------------------+-----------------------+
| Placement      | column zigzag      | "Utah" diagonal       |
+----------------+--------------------+-----------------------+
| Masking        | 8 patterns,        | NONE                  |
|                | penalty-scored     |                       |
+----------------+--------------------+-----------------------+
| Sizes          | 40 versions        | 30 square + 6 rect    |
+----------------+--------------------+-----------------------+

Encoding pipeline
-----------------
.. code-block::

    input string
      → ASCII encoding      (chars+1; digit pairs packed into one codeword)
      → symbol selection    (smallest symbol whose capacity ≥ codeword count)
      → pad to capacity     (scrambled-pad codewords fill unused slots)
      → RS blocks + ECC     (GF(256)/0x12D, b=1 convention)
      → interleave blocks   (data round-robin then ECC round-robin)
      → grid init           (L-finder + timing border + alignment borders)
      → Utah placement      (diagonal codeword placement, NO masking)
      → ModuleGrid          (abstract boolean grid, true = dark)

Public API
----------
- :func:`encode` — encode a string to a ``ModuleGrid`` (auto-selects size).
- :func:`encode_at` — encode to a specific symbol size.
- :func:`layout_grid` — ``ModuleGrid`` → ``PaintScene`` via barcode-2d.
- :func:`encode_and_layout` — encode + layout in one call.
- :class:`SymbolShape` — ``Square``, ``Rectangular``, ``Any`` selection mode.
- :class:`DataMatrixError` (and :class:`InputTooLongError`) — error types.
- :func:`grid_to_string` — debug rendering as '0' / '1' lines.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Final, Optional

from barcode_2d import (
    Barcode2DLayoutConfig,
    ModuleGrid,
    layout as _barcode_layout,
    make_module_grid,
    set_module,
)
from gf256 import GF256Field
from paint_instructions import PaintScene

__version__: Final = "0.1.0"

# ============================================================================
# Public types — SymbolShape
# ============================================================================


class SymbolShape:
    """Controls which symbol shapes are considered during selection.

    ``Square`` selects only from the 24 square symbols (10×10 … 144×144).
    Squares are the most common Data Matrix variant and the default.

    ``Rectangular`` selects only from the 6 rectangular symbols (8×18 …
    16×48).  Rectangles are useful when the print area's aspect ratio is
    constrained — for example a long thin label on a cable wrap.

    ``Any`` considers both shapes and picks whichever fits the input with
    the smallest module count (ties broken by area).
    """

    Square: Final = "square"
    Rectangular: Final = "rectangular"
    Any: Final = "any"


# ============================================================================
# Error hierarchy
# ============================================================================


class DataMatrixError(Exception):
    """Base class for all Data Matrix encoder errors.

    Catch ``DataMatrixError`` to handle any encoder error regardless of type.
    """


class InputTooLongError(DataMatrixError):
    """Input encodes to more codewords than the largest symbol can hold.

    The largest Data Matrix ECC200 symbol is 144×144, which holds at most
    1558 data codewords.  Inputs that exceed this capacity cannot be
    encoded — consider splitting the data across multiple symbols
    (Structured Append) or switching to a different barcode format.
    """


class InvalidSymbolError(DataMatrixError):
    """The requested explicit symbol size does not exist.

    Raised by :func:`encode_at` when ``rows`` × ``cols`` does not match any
    of the 30 ECC200 symbol sizes defined in ISO/IEC 16022:2006.
    """


# ============================================================================
# Symbol size table — ISO/IEC 16022:2006 Table 7
# ============================================================================
#
# Every Data Matrix symbol decomposes as:
#
#   symbol = outer_border + (region_rows × region_cols) data regions
#
# Each data region is (data_region_height × data_region_width) modules of
# pure data.  Regions are separated by 2-module alignment borders, and the
# whole symbol is wrapped in a 1-module finder/timing border.  So:
#
#   symbol_rows = 2 + region_rows × (data_region_height + 2 × (region_rows>1))
#               (approximately — the exact formula simplifies to the table values)
#
# The Utah placement algorithm scans the *logical* grid — the concatenation
# of all data region interiors — then we map back to physical coordinates.


@dataclass(frozen=True)
class _SymbolEntry:
    """One Data Matrix ECC200 symbol size and its capacity parameters.

    Attributes
    ----------
    symbol_rows / symbol_cols : int
        Total symbol size in modules, including the outer border.
    region_rows / region_cols : int
        Number of data region rows / cols (rr × rc).  Single-region symbols
        have ``1, 1``; the largest 144×144 has ``6, 6``.
    data_region_height / data_region_width : int
        Interior data size per region (excludes alignment borders).
    data_cw : int
        Total data codeword capacity for this symbol size.
    ecc_cw : int
        Total ECC codewords appended after data (sum across all blocks).
    num_blocks : int
        Number of interleaved Reed-Solomon blocks.  Larger symbols use more
        blocks to keep each block within RS error-correction capacity.
    ecc_per_block : int
        ECC codewords per block (identical for all blocks in one symbol).
    """

    symbol_rows: int
    symbol_cols: int
    region_rows: int
    region_cols: int
    data_region_height: int
    data_region_width: int
    data_cw: int
    ecc_cw: int
    num_blocks: int
    ecc_per_block: int


# The 24 square symbol sizes from ISO/IEC 16022:2006, Table 7, in ascending
# capacity order.  The fields match :class:`_SymbolEntry` in declaration order.
_SQUARE_SIZES: Final[tuple[_SymbolEntry, ...]] = (
    _SymbolEntry(10, 10, 1, 1, 8, 8, 3, 5, 1, 5),
    _SymbolEntry(12, 12, 1, 1, 10, 10, 5, 7, 1, 7),
    _SymbolEntry(14, 14, 1, 1, 12, 12, 8, 10, 1, 10),
    _SymbolEntry(16, 16, 1, 1, 14, 14, 12, 12, 1, 12),
    _SymbolEntry(18, 18, 1, 1, 16, 16, 18, 14, 1, 14),
    _SymbolEntry(20, 20, 1, 1, 18, 18, 22, 18, 1, 18),
    _SymbolEntry(22, 22, 1, 1, 20, 20, 30, 20, 1, 20),
    _SymbolEntry(24, 24, 1, 1, 22, 22, 36, 24, 1, 24),
    _SymbolEntry(26, 26, 1, 1, 24, 24, 44, 28, 1, 28),
    _SymbolEntry(32, 32, 2, 2, 14, 14, 62, 36, 2, 18),
    _SymbolEntry(36, 36, 2, 2, 16, 16, 86, 42, 2, 21),
    _SymbolEntry(40, 40, 2, 2, 18, 18, 114, 48, 2, 24),
    _SymbolEntry(44, 44, 2, 2, 20, 20, 144, 56, 4, 14),
    _SymbolEntry(48, 48, 2, 2, 22, 22, 174, 68, 4, 17),
    _SymbolEntry(52, 52, 2, 2, 24, 24, 204, 84, 4, 21),
    _SymbolEntry(64, 64, 4, 4, 14, 14, 280, 112, 4, 28),
    _SymbolEntry(72, 72, 4, 4, 16, 16, 368, 144, 4, 36),
    _SymbolEntry(80, 80, 4, 4, 18, 18, 456, 192, 4, 48),
    _SymbolEntry(88, 88, 4, 4, 20, 20, 576, 224, 4, 56),
    _SymbolEntry(96, 96, 4, 4, 22, 22, 696, 272, 4, 68),
    _SymbolEntry(104, 104, 4, 4, 24, 24, 816, 336, 6, 56),
    _SymbolEntry(120, 120, 6, 6, 18, 18, 1050, 408, 6, 68),
    _SymbolEntry(132, 132, 6, 6, 20, 20, 1304, 496, 8, 62),
    _SymbolEntry(144, 144, 6, 6, 22, 22, 1558, 620, 10, 62),
)

# The 6 rectangular symbol sizes from ISO/IEC 16022:2006, Table 7.
_RECT_SIZES: Final[tuple[_SymbolEntry, ...]] = (
    _SymbolEntry(8, 18, 1, 1, 6, 16, 5, 7, 1, 7),
    _SymbolEntry(8, 32, 1, 2, 6, 14, 10, 11, 1, 11),
    _SymbolEntry(12, 26, 1, 1, 10, 24, 16, 14, 1, 14),
    _SymbolEntry(12, 36, 1, 2, 10, 16, 22, 18, 1, 18),
    _SymbolEntry(16, 36, 1, 2, 14, 16, 32, 24, 1, 24),
    _SymbolEntry(16, 48, 1, 2, 14, 22, 49, 28, 1, 28),
)

# Largest data codeword capacity across all symbols (used for error reporting).
_MAX_DATA_CW: Final = 1558


# ============================================================================
# GF(256) over 0x12D — Data Matrix field
# ============================================================================
#
# Data Matrix uses GF(256) with primitive polynomial 0x12D:
#
#     p(x) = x^8 + x^5 + x^4 + x^2 + x + 1  =  0x12D  =  301
#
# IMPORTANT: this is DIFFERENT from QR Code's 0x11D polynomial.  Both are
# degree-8 irreducible polynomials over GF(2), but the resulting fields are
# non-isomorphic — never mix tables between QR and Data Matrix.
#
# We use ``GF256Field(0x12D)`` for parameterised arithmetic, but for
# performance we precompute exp / log tables here to avoid Russian-peasant
# multiplication on every Reed-Solomon step.
#
# The generator g = 2 (polynomial x) is primitive — it produces all 255
# non-zero elements when raised to powers 0..254.

_DM_FIELD: Final = GF256Field(0x12D)


def _build_dm_tables() -> tuple[tuple[int, ...], tuple[int, ...]]:
    """Build (exp, log) tables for GF(256)/0x12D.

    Algorithm
    ---------
    Start with ``val = 1`` (= α⁰).  Each step left-shifts ``val`` by one bit
    (which is multiplication by α = x in polynomial form).  If bit 8 is set
    (val ≥ 256), XOR with 0x12D to reduce modulo the primitive polynomial.

    After 255 steps every non-zero field element has been visited exactly
    once — this is what "primitive" means for the generator α = 2.

    Returns
    -------
    tuple of two tuples
        ``exp[i] = α^i`` for ``i`` in 0..254 (255 wraps to 1).
        ``log[v] = k`` such that ``α^k = v`` (``log[0]`` is unused).
    """
    exp_table = [0] * 256
    log_table = [0] * 256
    val = 1
    for i in range(255):
        exp_table[i] = val
        log_table[val] = i
        val <<= 1
        if val & 0x100:
            val ^= 0x12D
    # α^255 = α^0 = 1 — the multiplicative group has order 255.
    exp_table[255] = exp_table[0]
    return tuple(exp_table), tuple(log_table)


_DM_EXP: Final[tuple[int, ...]]
_DM_LOG: Final[tuple[int, ...]]
_DM_EXP, _DM_LOG = _build_dm_tables()


def _gf_mul(a: int, b: int) -> int:
    """Multiply two GF(256)/0x12D field elements via log/antilog tables.

    For a, b ≠ 0:  ``a × b = α^{(log[a] + log[b]) mod 255}``.
    If either operand is 0, the product is 0 (zero absorbs multiplication).

    This turns a polynomial multiplication + reduction into two table lookups
    and an addition modulo 255 — effectively O(1).
    """
    if a == 0 or b == 0:
        return 0
    return _DM_EXP[(_DM_LOG[a] + _DM_LOG[b]) % 255]


# ============================================================================
# RS generator polynomials (GF(256)/0x12D, b=1 convention)
# ============================================================================
#
# Data Matrix uses the b=1 convention: the RS generator's roots are α¹, α²,
# …, α^n (not α⁰, α¹, …, α^{n-1} like QR).  This shifts every coefficient
# in the generator polynomial.
#
# We build generators on demand and cache them.  The set of distinct ECC
# block sizes across all 30 symbols is small ({5, 7, 10, 12, 14, 17, 18,
# 20, 21, 24, 28, 36, 42, 48, 56, 62, 68}), so the cache stays compact.


def _build_generator(n_ecc: int) -> tuple[int, ...]:
    """Build the RS generator polynomial for ``n_ecc`` ECC bytes.

    The generator is::

        g(x) = (x + α¹)(x + α²) ··· (x + α^{n_ecc})

    Algorithm — start with ``g = [1]``, then for each ``i`` from 1 to
    ``n_ecc``, multiply ``g`` by the linear factor ``(x + α^i)``::

        for j, coeff in enumerate(g):
            new_g[j]   ^= coeff           (coeff × x term)
            new_g[j+1] ^= coeff × α^i    (coeff × constant term)

    Format: highest-degree coefficient first, length = ``n_ecc + 1``.
    """
    g: list[int] = [1]
    for i in range(1, n_ecc + 1):
        ai = _DM_EXP[i]  # α^i
        new_g = [0] * (len(g) + 1)
        for j, coeff in enumerate(g):
            new_g[j] ^= coeff
            new_g[j + 1] ^= _gf_mul(coeff, ai)
        g = new_g
    return tuple(g)


_GEN_CACHE: dict[int, tuple[int, ...]] = {}


def _get_generator(n_ecc: int) -> tuple[int, ...]:
    """Return the generator polynomial for ``n_ecc`` ECC bytes (cached)."""
    cached = _GEN_CACHE.get(n_ecc)
    if cached is not None:
        return cached
    gen = _build_generator(n_ecc)
    _GEN_CACHE[n_ecc] = gen
    return gen


# ============================================================================
# Reed-Solomon encoding
# ============================================================================


def _rs_encode_block(data: list[int], generator: tuple[int, ...]) -> list[int]:
    """Compute ECC bytes for a data block using the LFSR shift-register method.

    Computes ``R(x) = D(x) · x^{n_ecc} mod G(x)`` over GF(256)/0x12D.

    For each input byte ``d``::

        feedback = d XOR rem[0]
        shift rem left by one position (drop rem[0], append 0)
        for i in 0..n_ecc-1:
            rem[i] ^= gen[i+1] × feedback

    This is the standard systematic Reed-Solomon encoding approach —
    equivalent to polynomial long-division but implemented as a streaming
    shift register.  Output length = ``len(generator) - 1``.
    """
    n_ecc = len(generator) - 1
    rem = [0] * n_ecc
    for d in data:
        fb = d ^ rem[0]
        # Shift register left by one position.
        rem = rem[1:] + [0]
        if fb != 0:
            for i in range(n_ecc):
                rem[i] ^= _gf_mul(generator[i + 1], fb)
    return rem


# ============================================================================
# ASCII data encoding
# ============================================================================


def _encode_ascii(input_bytes: bytes) -> list[int]:
    """Encode input bytes in Data Matrix ASCII mode.

    ASCII mode rules (ISO/IEC 16022:2006 §5.2.4)
    --------------------------------------------
    - Two consecutive ASCII digits (0x30–0x39) → one codeword =
      ``130 + (d1 × 10 + d2)``.  This digit-pair optimization halves the
      codeword budget for numeric strings — critical for manufacturing lot
      codes, serial numbers, and barcodes that are mostly digits.

    - Single ASCII char (0–127) → one codeword = ``ASCII_value + 1``.
      So ``'A'`` (65) → 66, space (32) → 33.  The +1 shift exists because
      codeword 0 is reserved as "end of data".

    - Extended ASCII (128–255) → two codewords: ``235`` (UPPER_SHIFT), then
      ``ASCII_value - 127``.  This enables Latin-1 / Windows-1252 chars
      but is rare in practice (most Data Matrix content is plain ASCII).

    Examples
    --------
    +------------+--------------------+-----------------------------------+
    | Input      | Codewords          | Why                               |
    +============+====================+===================================+
    | ``"A"``    | ``[66]``           | 65 + 1                            |
    +------------+--------------------+-----------------------------------+
    | ``" "``    | ``[33]``           | 32 + 1                            |
    +------------+--------------------+-----------------------------------+
    | ``"12"``   | ``[142]``          | 130 + 12 (digit pair)             |
    +------------+--------------------+-----------------------------------+
    | ``"1234"`` | ``[142, 174]``     | two digit pairs                   |
    +------------+--------------------+-----------------------------------+
    | ``"1A"``   | ``[50, 66]``       | '1' alone (next not a digit)      |
    +------------+--------------------+-----------------------------------+
    | ``"00"``   | ``[130]``          | 130 + 0                           |
    +------------+--------------------+-----------------------------------+
    | ``"99"``   | ``[229]``          | 130 + 99                          |
    +------------+--------------------+-----------------------------------+
    """
    codewords: list[int] = []
    i = 0
    n = len(input_bytes)
    while i < n:
        c = input_bytes[i]
        # Digit pair: both current and next bytes are ASCII digits.
        if (
            0x30 <= c <= 0x39
            and i + 1 < n
            and 0x30 <= input_bytes[i + 1] <= 0x39
        ):
            d1 = c - 0x30
            d2 = input_bytes[i + 1] - 0x30
            codewords.append(130 + d1 * 10 + d2)
            i += 2
        elif c <= 127:
            # Standard single ASCII character: value + 1.
            codewords.append(c + 1)
            i += 1
        else:
            # Extended ASCII (128–255): UPPER_SHIFT (235) then (value - 127).
            codewords.append(235)
            codewords.append(c - 127)
            i += 1
    return codewords


# ============================================================================
# Pad codewords (ISO/IEC 16022:2006 §5.2.3)
# ============================================================================


def _pad_codewords(codewords: list[int], data_cw: int) -> list[int]:
    """Pad ``codewords`` to exactly ``data_cw`` bytes using the ECC200 rule.

    Padding rules
    -------------
    1. The first pad codeword is always the literal value ``129`` (often
       called "End of Message" or EOM in the spec).

    2. Subsequent pads use a *scrambled* value that depends on their
       1-indexed position ``k`` within the full codeword stream::

           scrambled = 129 + (149 × k) mod 253 + 1
           if scrambled > 254: scrambled -= 254

       The scrambling prevents a run of "129 129 129 …" from creating a
       degenerate placement pattern in the Utah algorithm — long identical
       runs would cluster related modules and bias the error-correction
       structure.

    Worked example
    --------------
    Encoding "A" (codewords ``[66]``) into a 10×10 symbol (``data_cw = 3``):

    - ``k=2``: 129                              (first pad — always literal)
    - ``k=3``: ``129 + (149×3 mod 253) + 1`` =
      ``129 + 194 + 1`` = 324; ``324 - 254`` = 70
    - Result: ``[66, 129, 70]``
    """
    padded = list(codewords)
    is_first = True
    k = len(codewords) + 1  # 1-indexed position of the first pad byte
    while len(padded) < data_cw:
        if is_first:
            padded.append(129)
            is_first = False
        else:
            scrambled = 129 + (149 * k) % 253 + 1
            if scrambled > 254:
                scrambled -= 254
            padded.append(scrambled)
        k += 1
    return padded


# ============================================================================
# Symbol selection
# ============================================================================


def _select_symbol(codeword_count: int, shape: str) -> _SymbolEntry:
    """Find the smallest symbol entry that can hold ``codeword_count`` codewords.

    Iterates all candidates (filtered by ``shape``) sorted by ``data_cw``
    ascending — ties broken by total module area — and returns the first
    whose ``data_cw ≥ codeword_count``.

    Raises :class:`InputTooLongError` if no symbol is large enough.
    """
    if shape == SymbolShape.Square:
        candidates = list(_SQUARE_SIZES)
    elif shape == SymbolShape.Rectangular:
        candidates = list(_RECT_SIZES)
    else:  # SymbolShape.Any (or any other value — we treat as union)
        candidates = list(_SQUARE_SIZES) + list(_RECT_SIZES)

    # Sort by capacity ascending, ties broken by area (rows × cols).
    candidates.sort(key=lambda e: (e.data_cw, e.symbol_rows * e.symbol_cols))

    for e in candidates:
        if e.data_cw >= codeword_count:
            return e

    raise InputTooLongError(
        f"data-matrix: input too long — encoded {codeword_count} codewords, "
        f"maximum is {_MAX_DATA_CW} (144×144 symbol)."
    )


# ============================================================================
# Block splitting, ECC computation, and interleaving
# ============================================================================


def _compute_interleaved(data: list[int], entry: _SymbolEntry) -> list[int]:
    """Split ``data`` across RS blocks, compute ECC per block, and interleave.

    Block splitting
    ---------------
    ::

        base_len     = data_cw / num_blocks      (integer division)
        extra_blocks = data_cw mod num_blocks
        Blocks 0..extra_blocks-1   get base_len + 1 data codewords.
        Blocks extra_blocks..end-1 get base_len     data codewords.

    Earlier blocks get one extra if the total is not evenly divisible — the
    standard ISO interleaving convention.

    Interleaving
    ------------
    Data round-robin first, then ECC round-robin::

        for pos in 0..max_data_per_block:
            for blk in 0..num_blocks:
                emit data[blk][pos] if pos < len(data[blk])
        for pos in 0..ecc_per_block:
            for blk in 0..num_blocks:
                emit ecc[blk][pos]

    Interleaving distributes burst errors: a physical scratch destroying N
    contiguous modules affects at most ``ceil(N / num_blocks)`` codewords
    per block — far more likely to be within each block's correction
    capacity than the same scratch hitting one block in full.
    """
    num_blocks = entry.num_blocks
    ecc_per_block = entry.ecc_per_block
    data_cw = entry.data_cw
    gen = _get_generator(ecc_per_block)

    # ── Split data into blocks ───────────────────────────────────────────────
    base_len = data_cw // num_blocks
    extra_blocks = data_cw % num_blocks

    data_blocks: list[list[int]] = []
    offset = 0
    for b in range(num_blocks):
        l = base_len + 1 if b < extra_blocks else base_len
        data_blocks.append(data[offset : offset + l])
        offset += l

    # ── Compute ECC for each block ───────────────────────────────────────────
    ecc_blocks: list[list[int]] = [
        _rs_encode_block(blk, gen) for blk in data_blocks
    ]

    # ── Interleave data round-robin ──────────────────────────────────────────
    interleaved: list[int] = []
    max_data_len = max((len(blk) for blk in data_blocks), default=0)
    for pos in range(max_data_len):
        for b in range(num_blocks):
            if pos < len(data_blocks[b]):
                interleaved.append(data_blocks[b][pos])

    # ── Interleave ECC round-robin ───────────────────────────────────────────
    for pos in range(ecc_per_block):
        for b in range(num_blocks):
            interleaved.append(ecc_blocks[b][pos])

    return interleaved


# ============================================================================
# Grid initialization (border + alignment borders)
# ============================================================================


def _init_grid(entry: _SymbolEntry) -> list[list[bool]]:
    """Allocate the physical grid and fill in all fixed structural elements.

    Outer "finder + clock" border
    -----------------------------
    - Top row (row 0): alternating dark/light starting dark at col 0
      (timing clock for the top edge).
    - Right col (col C-1): alternating dark/light starting dark at row 0
      (timing clock for the right edge).
    - Left col (col 0): all dark (vertical leg of the L-finder).
    - Bottom row (row R-1): all dark (horizontal leg of the L-finder).

    The L-shaped solid bar tells a scanner where the symbol starts and
    which orientation it has.  The alternating timing on the opposite two
    edges distinguishes all four 90° rotations.

    Alignment borders (multi-region symbols)
    ----------------------------------------
    For symbols with ``region_rows × region_cols > 1``, alignment borders
    separate adjacent data regions.  Each is two modules wide:

    - The first row/col is **all dark**.
    - The second row/col is **alternating** (starts dark at col/row 0).

    Writing order
    -------------
    Alignment borders FIRST, then top + right timing, then left + bottom
    L-finder.  The L-finder bottom row is written LAST so it always wins
    at intersections.  This precise order matters for the corner pixels:
    e.g. the top-right corner ``(0, C-1)`` ends up as the right column's
    "row 0 = dark" rather than the top row's value at the rightmost col.
    """
    R, C = entry.symbol_rows, entry.symbol_cols

    grid: list[list[bool]] = [[False] * C for _ in range(R)]

    # ── Alignment borders (multi-region symbols only) ───────────────────────
    # Written FIRST so the outer border can override at intersections.
    for rr in range(entry.region_rows - 1):
        # Physical row of first AB row after data region rr+1:
        # outer border (1) + (rr+1) * data_region_height + rr * 2 (prev ABs)
        ab_row0 = 1 + (rr + 1) * entry.data_region_height + rr * 2
        ab_row1 = ab_row0 + 1
        for c in range(C):
            grid[ab_row0][c] = True            # all dark
            grid[ab_row1][c] = (c % 2 == 0)    # alternating, starts dark

    for rc in range(entry.region_cols - 1):
        ab_col0 = 1 + (rc + 1) * entry.data_region_width + rc * 2
        ab_col1 = ab_col0 + 1
        for r in range(R):
            grid[r][ab_col0] = True            # all dark
            grid[r][ab_col1] = (r % 2 == 0)    # alternating, starts dark

    # ── Top row: timing clock — alternating dark/light starting dark ────────
    for c in range(C):
        grid[0][c] = (c % 2 == 0)

    # ── Right column: timing clock — alternating, starts dark ───────────────
    for r in range(R):
        grid[r][C - 1] = (r % 2 == 0)

    # ── Left column: L-finder left leg — all dark ───────────────────────────
    # Written after timing to override the timing value at (0, 0).
    for r in range(R):
        grid[r][0] = True

    # ── Bottom row: L-finder bottom leg — all dark ──────────────────────────
    # Written LAST: overrides alignment borders, right-column timing, etc.
    for c in range(C):
        grid[R - 1][c] = True

    return grid


# ============================================================================
# Utah placement algorithm
# ============================================================================
#
# The Utah placement algorithm is the most distinctive part of Data Matrix
# encoding.  Its name comes from the 8-module codeword shape, which resembles
# the outline of the US state of Utah — a rectangle with a notch cut from
# the top-left corner.
#
# The algorithm scans the *logical* grid (all data region interiors
# concatenated) in a diagonal zigzag.  For each codeword, 8 bits are placed
# at 8 fixed offsets relative to the current reference position ``(row,
# col)``.  After each codeword the reference moves diagonally:
# ``row -= 2, col += 2`` for the upward-right leg; ``row += 2, col -= 2``
# for the downward-left leg.
#
# Four special "corner" patterns handle positions where the standard Utah
# shape would extend outside the grid boundary.
#
# There is **no masking step** after placement.  The diagonal traversal
# naturally distributes bits across the symbol without the degenerate
# clustering that would otherwise require masking (as in QR Code).


def _apply_wrap(row: int, col: int, n_rows: int, n_cols: int) -> tuple[int, int]:
    """Apply the boundary wrap rules from ISO/IEC 16022:2006 Annex F.

    When the standard Utah shape extends beyond the logical grid edge,
    these rules fold the coordinates back into the valid range.

    The four wrap rules (applied in order):

    1. ``row < 0`` AND ``col == 0``        → ``(1, 3)``    top-left singularity
    2. ``row < 0`` AND ``col == n_cols``   → ``(0, col-2)`` wrapped past right
    3. ``row < 0``                          → ``(row+n_rows, col-4)`` wrap top→bot
    4. ``col < 0``                          → ``(row-4, col+n_cols)`` wrap left→rt
    """
    if row < 0 and col == 0:
        return 1, 3
    if row < 0 and col == n_cols:
        return 0, col - 2
    if row < 0:
        return row + n_rows, col - 4
    if col < 0:
        return row - 4, col + n_cols
    return row, col


def _place_utah(
    cw: int,
    row: int,
    col: int,
    n_rows: int,
    n_cols: int,
    grid: list[list[bool]],
    used: list[list[bool]],
) -> None:
    """Place one codeword using the standard "Utah" 8-module pattern.

    The Utah shape at reference position ``(row, col)``::

                 col-2  col-1   col

        row-2 :    .   [bit1]  [bit2]
        row-1 :  [bit3] [bit4] [bit5]
        row   :  [bit6] [bit7] [bit8]

    Bits 1–8 are extracted from the codeword with **bit 8 = MSB** (placed at
    ``(row, col)``) and **bit 1 = LSB** (placed at ``(row-2, col-1)``).
    """
    # (raw_row, raw_col, bit_shift)  where bit_shift 7 = MSB, 0 = LSB.
    placements = (
        (row,     col,     7),  # bit 8 (MSB)
        (row,     col - 1, 6),  # bit 7
        (row,     col - 2, 5),  # bit 6
        (row - 1, col,     4),  # bit 5
        (row - 1, col - 1, 3),  # bit 4
        (row - 1, col - 2, 2),  # bit 3
        (row - 2, col,     1),  # bit 2
        (row - 2, col - 1, 0),  # bit 1 (LSB)
    )
    for raw_r, raw_c, bit in placements:
        r, c = _apply_wrap(raw_r, raw_c, n_rows, n_cols)
        if 0 <= r < n_rows and 0 <= c < n_cols and not used[r][c]:
            grid[r][c] = ((cw >> bit) & 1) == 1
            used[r][c] = True


def _place_with_positions(
    cw: int,
    positions: tuple[tuple[int, int, int], ...],
    n_rows: int,
    n_cols: int,
    grid: list[list[bool]],
    used: list[list[bool]],
) -> None:
    """Helper: place a codeword's 8 bits at explicit ``(row, col, bit)`` positions."""
    for r, c, bit in positions:
        if 0 <= r < n_rows and 0 <= c < n_cols and not used[r][c]:
            grid[r][c] = ((cw >> bit) & 1) == 1
            used[r][c] = True


def _place_corner1(
    cw: int, n_rows: int, n_cols: int,
    grid: list[list[bool]], used: list[list[bool]],
) -> None:
    """Corner pattern 1 — triggered at the top-left boundary."""
    positions = (
        (0,           n_cols - 2, 7),
        (0,           n_cols - 1, 6),
        (1,           0,          5),
        (2,           0,          4),
        (n_rows - 2,  0,          3),
        (n_rows - 1,  0,          2),
        (n_rows - 1,  1,          1),
        (n_rows - 1,  2,          0),
    )
    _place_with_positions(cw, positions, n_rows, n_cols, grid, used)


def _place_corner2(
    cw: int, n_rows: int, n_cols: int,
    grid: list[list[bool]], used: list[list[bool]],
) -> None:
    """Corner pattern 2 — triggered at the top-right boundary."""
    positions = (
        (0,           n_cols - 2, 7),
        (0,           n_cols - 1, 6),
        (1,           n_cols - 1, 5),
        (2,           n_cols - 1, 4),
        (n_rows - 1,  0,          3),
        (n_rows - 1,  1,          2),
        (n_rows - 1,  2,          1),
        (n_rows - 1,  3,          0),
    )
    _place_with_positions(cw, positions, n_rows, n_cols, grid, used)


def _place_corner3(
    cw: int, n_rows: int, n_cols: int,
    grid: list[list[bool]], used: list[list[bool]],
) -> None:
    """Corner pattern 3 — triggered at the bottom-left boundary."""
    positions = (
        (0,           n_cols - 1, 7),
        (1,           0,          6),
        (2,           0,          5),
        (n_rows - 2,  0,          4),
        (n_rows - 1,  0,          3),
        (n_rows - 1,  1,          2),
        (n_rows - 1,  2,          1),
        (n_rows - 1,  3,          0),
    )
    _place_with_positions(cw, positions, n_rows, n_cols, grid, used)


def _place_corner4(
    cw: int, n_rows: int, n_cols: int,
    grid: list[list[bool]], used: list[list[bool]],
) -> None:
    """Corner pattern 4 — triggered for ``n_cols mod 8 == 0``."""
    positions = (
        (n_rows - 3, n_cols - 1, 7),
        (n_rows - 2, n_cols - 1, 6),
        (n_rows - 1, n_cols - 3, 5),
        (n_rows - 1, n_cols - 2, 4),
        (n_rows - 1, n_cols - 1, 3),
        (0,          0,          2),
        (1,          0,          1),
        (2,          0,          0),
    )
    _place_with_positions(cw, positions, n_rows, n_cols, grid, used)


def _utah_placement(
    codewords: list[int], n_rows: int, n_cols: int
) -> list[list[bool]]:
    """Run the Utah diagonal placement algorithm on the logical data matrix.

    The reference position ``(row, col)`` starts at ``(4, 0)`` and zigzags
    diagonally across the logical grid.  Each iteration of the outer loop
    has two legs:

    1. **Upward-right leg**: place codewords at ``(row, col)``, then move
       ``row -= 2, col += 2`` until out of bounds.  Then step to the next
       diagonal start: ``row += 1, col += 3``.

    2. **Downward-left leg**: place codewords at ``(row, col)``, then move
       ``row += 2, col -= 2`` until out of bounds.  Then step to the next
       diagonal start: ``row += 3, col += 1``.

    Between legs, four corner patterns fire when the reference position
    matches specific trigger conditions (described per-function).

    Termination — when both ``row >= n_rows`` and ``col >= n_cols``, all
    codewords have been visited.  Any unvisited modules at the end receive
    the "fill" pattern ``(r + c) mod 2 == 1`` (dark), matching the ISO
    right-and-bottom fill rule.
    """
    grid: list[list[bool]] = [[False] * n_cols for _ in range(n_rows)]
    used: list[list[bool]] = [[False] * n_cols for _ in range(n_rows)]

    cw_idx = 0
    row = 4
    col = 0

    while True:
        # ── Corner special cases ─────────────────────────────────────────────
        # Corner 1: reference at (n_rows, 0) when n_rows or n_cols ≡ 0 (mod 4).
        if row == n_rows and col == 0 and (n_rows % 4 == 0 or n_cols % 4 == 0):
            if cw_idx < len(codewords):
                _place_corner1(codewords[cw_idx], n_rows, n_cols, grid, used)
                cw_idx += 1
        # Corner 2: reference at (n_rows-2, 0) when n_cols mod 4 ≠ 0.
        if row == n_rows - 2 and col == 0 and n_cols % 4 != 0:
            if cw_idx < len(codewords):
                _place_corner2(codewords[cw_idx], n_rows, n_cols, grid, used)
                cw_idx += 1
        # Corner 3: reference at (n_rows-2, 0) when n_cols mod 8 == 4.
        if row == n_rows - 2 and col == 0 and n_cols % 8 == 4:
            if cw_idx < len(codewords):
                _place_corner3(codewords[cw_idx], n_rows, n_cols, grid, used)
                cw_idx += 1
        # Corner 4: reference at (n_rows+4, 2) when n_cols mod 8 == 0.
        if row == n_rows + 4 and col == 2 and n_cols % 8 == 0:
            if cw_idx < len(codewords):
                _place_corner4(codewords[cw_idx], n_rows, n_cols, grid, used)
                cw_idx += 1

        # ── Upward-right diagonal leg (row -= 2, col += 2) ──────────────────
        while True:
            if (
                0 <= row < n_rows
                and 0 <= col < n_cols
                and not used[row][col]
                and cw_idx < len(codewords)
            ):
                _place_utah(
                    codewords[cw_idx], row, col, n_rows, n_cols, grid, used
                )
                cw_idx += 1
            row -= 2
            col += 2
            if row < 0 or col >= n_cols:
                break

        # Step to next diagonal start.
        row += 1
        col += 3

        # ── Downward-left diagonal leg (row += 2, col -= 2) ─────────────────
        while True:
            if (
                0 <= row < n_rows
                and 0 <= col < n_cols
                and not used[row][col]
                and cw_idx < len(codewords)
            ):
                _place_utah(
                    codewords[cw_idx], row, col, n_rows, n_cols, grid, used
                )
                cw_idx += 1
            row += 2
            col -= 2
            if row >= n_rows or col < 0:
                break

        # Step to next diagonal start.
        row += 3
        col += 1

        # ── Termination check ────────────────────────────────────────────────
        if row >= n_rows and col >= n_cols:
            break
        if cw_idx >= len(codewords):
            break

    # ── Fill remaining unset modules (ISO right-and-bottom fill rule) ────────
    # Some symbol sizes have residual modules the diagonal walk does not reach.
    # ISO/IEC 16022 §10 specifies these receive (r+c) mod 2 == 1 (dark).
    for r in range(n_rows):
        for c in range(n_cols):
            if not used[r][c]:
                grid[r][c] = (r + c) % 2 == 1

    return grid


# ============================================================================
# Logical → physical coordinate mapping
# ============================================================================


def _logical_to_physical(r: int, c: int, entry: _SymbolEntry) -> tuple[int, int]:
    """Map a logical data-matrix coordinate to its physical symbol coordinate.

    The logical data matrix is the concatenation of all data region
    interiors treated as one flat grid.  Utah placement works in this
    logical space.  After placement we map back to the physical grid, which
    adds:

    - 1-module outer border (finder + timing) on all four sides.
    - 2-module alignment border between adjacent data regions.

    For a symbol with ``region_rows × region_cols`` data regions, each of
    size ``(rh × rw)``::

        phys_row = floor(r / rh) × (rh + 2) + (r mod rh) + 1
        phys_col = floor(c / rw) × (rw + 2) + (c mod rw) + 1

    The "+ 2" accounts for the 2-module alignment border between regions.
    The "+ 1" accounts for the 1-module outer border.

    For single-region symbols (1 × 1), this simplifies to
    ``phys_row = r + 1, phys_col = c + 1``.
    """
    rh = entry.data_region_height
    rw = entry.data_region_width
    phys_row = (r // rh) * (rh + 2) + (r % rh) + 1
    phys_col = (c // rw) * (rw + 2) + (c % rw) + 1
    return phys_row, phys_col


# ============================================================================
# Core encode function
# ============================================================================


def encode(
    data: str,
    size: Optional[tuple[int, int]] = None,
    *,
    shape: str = SymbolShape.Square,
) -> ModuleGrid:
    """Encode a string into a Data Matrix ECC200 :class:`ModuleGrid`.

    The smallest symbol that can hold the encoded data is selected
    automatically when ``size`` is ``None``.  Pass ``size=(rows, cols)`` to
    force a specific size — :class:`InvalidSymbolError` is raised if the
    size is not one of the 30 ECC200 sizes; :class:`InputTooLongError` is
    raised if the input does not fit.

    Pipeline
    --------
    1. ASCII-encode the input (with digit-pair compression).
    2. Select the smallest fitting symbol (or use ``size`` if provided).
    3. Pad to data capacity with the ECC200 scrambled-pad sequence.
    4. Compute Reed-Solomon ECC for each block over GF(256)/0x12D.
    5. Interleave data + ECC blocks round-robin.
    6. Initialize the physical grid (finder + timing + alignment borders).
    7. Run Utah diagonal placement on the logical data matrix.
    8. Map logical → physical coordinates.
    9. Return the immutable :class:`ModuleGrid`.

    Parameters
    ----------
    data
        The string to encode.  Encoded as UTF-8; bytes 128–255 use the
        UPPER_SHIFT mechanism and consume two codewords each.
    size
        Optional ``(rows, cols)`` tuple to force a specific symbol size.
        Must match one of the 30 ECC200 symbol sizes (see :data:`SQUARE_SIZES`
        and :data:`RECT_SIZES`).  When ``None`` the smallest fitting symbol
        is selected.
    shape
        One of :data:`SymbolShape.Square` (default), :data:`SymbolShape.Rectangular`,
        or :data:`SymbolShape.Any`.  Ignored when ``size`` is provided.

    Returns
    -------
    ModuleGrid
        A frozen ``rows × cols`` boolean grid where ``True`` = dark module.

    Raises
    ------
    InputTooLongError
        If the input encodes to more codewords than the largest fitting
        symbol can hold.
    InvalidSymbolError
        If ``size`` is provided and does not match any ECC200 symbol size.

    Examples
    --------
    Auto-selected smallest square symbol::

        grid = encode("A")
        assert grid.rows == 10 and grid.cols == 10

    Forced larger symbol::

        grid = encode("Hello", size=(18, 18))
        assert grid.rows == 18 and grid.cols == 18

    Allow rectangular symbols too::

        grid = encode("Hi", shape=SymbolShape.Any)
    """
    # Step 1: ASCII encode (uses UTF-8 bytes for inputs containing non-ASCII).
    codewords = _encode_ascii(data.encode("utf-8"))

    # Step 2: Select symbol — explicit size or auto-pick smallest.
    if size is not None:
        entry = _find_entry_by_size(size)
        if len(codewords) > entry.data_cw:
            raise InputTooLongError(
                f"data-matrix: input encodes to {len(codewords)} codewords "
                f"but {entry.symbol_rows}×{entry.symbol_cols} symbol holds "
                f"only {entry.data_cw}."
            )
    else:
        entry = _select_symbol(len(codewords), shape)

    # Step 3: Pad to data capacity.
    padded = _pad_codewords(codewords, entry.data_cw)

    # Steps 4–5: Compute ECC and interleave.
    interleaved = _compute_interleaved(padded, entry)

    # Step 6: Initialize physical grid with finder + timing + alignment borders.
    phys_grid = _init_grid(entry)

    # Step 7: Run Utah placement on the logical data matrix.
    n_rows = entry.region_rows * entry.data_region_height
    n_cols = entry.region_cols * entry.data_region_width
    logical_grid = _utah_placement(interleaved, n_rows, n_cols)

    # Step 8: Map logical → physical coordinates.
    for r in range(n_rows):
        for c in range(n_cols):
            pr, pc = _logical_to_physical(r, c, entry)
            phys_grid[pr][pc] = logical_grid[r][c]

    # Step 9: Build immutable ModuleGrid.
    grid = make_module_grid(entry.symbol_rows, entry.symbol_cols)
    for r in range(entry.symbol_rows):
        for c in range(entry.symbol_cols):
            if phys_grid[r][c]:
                grid = set_module(grid, r, c, True)

    return grid


def _find_entry_by_size(size: tuple[int, int]) -> _SymbolEntry:
    """Return the symbol entry matching ``(rows, cols)``, or raise."""
    rows, cols = size
    for entry in _SQUARE_SIZES:
        if entry.symbol_rows == rows and entry.symbol_cols == cols:
            return entry
    for entry in _RECT_SIZES:
        if entry.symbol_rows == rows and entry.symbol_cols == cols:
            return entry
    raise InvalidSymbolError(
        f"data-matrix: {rows}×{cols} is not a valid ECC200 symbol size. "
        f"Square sizes: 10×10, 12×12, …, 144×144. "
        f"Rect sizes: 8×18, 8×32, 12×26, 12×36, 16×36, 16×48."
    )


# ============================================================================
# encode_at — force a specific symbol size
# ============================================================================


def encode_at(data: str, rows: int, cols: int) -> ModuleGrid:
    """Encode ``data`` to a specific symbol size.

    Equivalent to ``encode(data, size=(rows, cols))``.  Raises
    :class:`InvalidSymbolError` if the size is not one of the 30 ECC200
    sizes; raises :class:`InputTooLongError` if the input does not fit.

    Examples
    --------
    ::

        grid = encode_at("HELLO", 18, 18)
        assert grid.rows == 18 and grid.cols == 18
    """
    return encode(data, size=(rows, cols))


# ============================================================================
# layout_grid — ModuleGrid → PaintScene
# ============================================================================


def layout_grid(
    grid: ModuleGrid,
    config: Barcode2DLayoutConfig | None = None,
) -> PaintScene:
    """Convert a :class:`ModuleGrid` to a :class:`PaintScene` via barcode-2d.

    Defaults to a 1-module quiet zone (Data Matrix minimum, narrower than
    QR's 4-module requirement — the L-finder is inherently self-delimiting).

    Parameters
    ----------
    grid
        A grid returned by :func:`encode` or :func:`encode_at`.
    config
        Optional layout configuration.  ``None`` uses ``module_size_px=10``
        and ``quiet_zone_modules=1``.

    Returns
    -------
    PaintScene
        Ready for any paint-vm backend (SVG, Metal, Canvas, terminal).
    """
    cfg = config if config is not None else Barcode2DLayoutConfig(
        quiet_zone_modules=1,
        module_size_px=10,
    )
    return _barcode_layout(grid, cfg)


# ============================================================================
# encode_and_layout — convenience: encode + layout in one call
# ============================================================================


def encode_and_layout(
    data: str,
    config: Barcode2DLayoutConfig | None = None,
    *,
    size: Optional[tuple[int, int]] = None,
    shape: str = SymbolShape.Square,
) -> PaintScene:
    """Encode and convert to :class:`PaintScene` in a single call.

    Equivalent to ``layout_grid(encode(data, size=size, shape=shape), config)``.
    """
    grid = encode(data, size=size, shape=shape)
    return layout_grid(grid, config)


# ============================================================================
# grid_to_string — debugging utility
# ============================================================================


def grid_to_string(grid: ModuleGrid) -> str:
    """Render a :class:`ModuleGrid` as a multi-line '0' / '1' string.

    Useful for debugging, snapshot tests, and cross-language corpus
    comparison.  Each row is one line; rows are separated by newlines;
    no trailing newline.
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
    "SymbolShape",
    # Errors
    "DataMatrixError",
    "InputTooLongError",
    "InvalidSymbolError",
    # Encoding functions
    "encode",
    "encode_at",
    "layout_grid",
    "encode_and_layout",
    # Utilities
    "grid_to_string",
]
