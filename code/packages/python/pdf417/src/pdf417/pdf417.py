"""PDF417 stacked linear barcode encoder — core implementation.

PDF417 (Portable Data File 417) was invented by Ynjiun P. Wang at Symbol
Technologies in 1991.  The name encodes the format's geometry:

    4 bars + 4 spaces = 8 elements per codeword
    17 modules per codeword (element widths sum to 17)
    → "417"

This module implements the full encoding pipeline:

    input bytes
      → byte compaction (codeword 924 latch + 6-bytes-to-5-codewords)
      → length descriptor (codeword 0 = total codewords in symbol)
      → RS ECC (GF(929) Reed-Solomon, b=3 convention, α=3)
      → dimension selection (auto: roughly square symbol)
      → padding (codeword 900 fills unused slots)
      → row indicators (LRI + RRI per row, encode R/C/ECC level)
      → cluster table lookup (codeword → 17-module bar/space pattern)
      → start/stop patterns (fixed per row)
      → ModuleGrid (abstract boolean grid)

GF(929) — The Prime Field
--------------------------
PDF417 uses Reed-Solomon over GF(929), not GF(256).  Since 929 is prime,
GF(929) is simply the integers modulo 929:

    add(a, b) = (a + b) mod 929
    mul(a, b) computed via log/antilog tables for speed
    generator α = 3  (primitive root mod 929)

This is fundamentally different from QR Code's GF(256) which uses binary
polynomial arithmetic.  GF(929) uses ordinary integer arithmetic.

Why 929?  It equals the number of valid codeword values (0–928).  For
Reed-Solomon to work, the ECC must be computed in a field whose size equals
the alphabet size.  929 is prime → GF(929) = Z/929Z.

Three-Cluster Encoding
-----------------------
Each row uses one of three codeword-to-barcode mappings (clusters), cycling
as row % 3:

    row % 3 == 0  →  cluster 0
    row % 3 == 1  →  cluster 1 (ISO calls this "cluster 3")
    row % 3 == 2  →  cluster 2 (ISO calls this "cluster 6")

This lets a scanner identify which cluster a row belongs to, verify row
indicators, and reconstruct the full symbol even with partial reads.

Row Indicators
--------------
Each row carries a Left Row Indicator (LRI) and Right Row Indicator (RRI)
that together encode R (total rows), C (total data columns), and L (ECC
level).  A scanner that reads any three consecutive rows (one of each cluster)
can fully reconstruct R, C, and L — even without reading the whole symbol.
"""

from __future__ import annotations

import math
from typing import Final

from barcode_2d import ModuleGrid, make_module_grid, set_module

from ._cluster_tables import CLUSTER_TABLES, START_PATTERN, STOP_PATTERN

__all__ = [
    "encode",
    "compute_lri",
    "compute_rri",
    "PDF417Error",
    "InputTooLongError",
    "InvalidDimensionsError",
    "InvalidECCLevelError",
]

# ---------------------------------------------------------------------------
# Error types
# ---------------------------------------------------------------------------


class PDF417Error(Exception):
    """Base class for all PDF417 encoding errors.

    Catch ``PDF417Error`` to handle any encoder error regardless of subclass.
    """


class InputTooLongError(PDF417Error):
    """Input data is too long to fit in any valid PDF417 symbol.

    The largest PDF417 symbol is 90 rows × 30 columns = 2700 data slots.
    After subtracting ECC codewords, the maximum data capacity is ~2188
    bytes (at ECC level 0 with byte compaction).
    """


class InvalidDimensionsError(PDF417Error):
    """User-supplied rows or columns are outside the valid range.

    Rows must be 3–90; columns must be 1–30.
    """


class InvalidECCLevelError(PDF417Error):
    """ECC level is outside the valid range 0–8."""


# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

# GF(929) prime modulus.
_GF929_PRIME: Final = 929

# Generator element α = 3.  This is a primitive root modulo 929, meaning the
# powers 3^0, 3^1, ..., 3^{927} cycle through all 928 non-zero elements of
# GF(929) before repeating.  Specified in ISO/IEC 15438:2015, Annex A.4.
_GF929_ALPHA: Final = 3

# Multiplicative group order = PRIME - 1 = 928.
_GF929_ORDER: Final = 928

# Latch-to-byte-compaction codeword (used for any-length byte data).
# Codeword 924 is the "alternate" byte latch that works regardless of whether
# the byte count is divisible by 6.
_LATCH_BYTE: Final = 924

# Padding codeword.  Value 900 = latch-to-text.  When a scanner sees it after
# the data region, it just switches to text mode and produces no output — a
# safe neutral filler.
_PADDING_CW: Final = 900

_MIN_ROWS: Final = 3
_MAX_ROWS: Final = 90
_MIN_COLS: Final = 1
_MAX_COLS: Final = 30

# ---------------------------------------------------------------------------
# GF(929) arithmetic — log/antilog tables
# ---------------------------------------------------------------------------
#
# We precompute exp and log tables at import time (once).  This reduces each
# multiplication to two table lookups + one addition mod 928 — O(1).
#
# Table sizes: 929 × 2 = 1858 int entries ≈ 7 KB.  Negligible.
#
# Algorithm:
#   GF_EXP[i] = 3^i mod 929   (i = 0..927, then 928 = copy of 0)
#   GF_LOG[v]  = i  such that 3^i ≡ v (mod 929)  (v = 1..928)


def _build_gf929_tables() -> tuple[tuple[int, ...], tuple[int, ...]]:
    """Build exp and log tables for GF(929).

    ``GF_EXP[i]`` = α^i mod 929 for i in 0..928.
    ``GF_LOG[v]`` = discrete log base α of v, for v in 1..928.
    ``GF_LOG[0]`` = 0 (not mathematically defined, but zero-initialised).
    """
    exp_table = [0] * (_GF929_ORDER + 1)  # indices 0..928
    log_table = [0] * _GF929_PRIME         # indices 0..928
    val = 1
    for i in range(_GF929_ORDER):
        exp_table[i] = val
        log_table[val] = i
        val = (val * _GF929_ALPHA) % _GF929_PRIME
    # Duplicate entry at index 928 for wrap-around convenience in gf_mul.
    exp_table[_GF929_ORDER] = exp_table[0]
    return tuple(exp_table), tuple(log_table)


_GF_EXP: Final[tuple[int, ...]]
_GF_LOG: Final[tuple[int, ...]]
_GF_EXP, _GF_LOG = _build_gf929_tables()


def _gf_mul(a: int, b: int) -> int:
    """Multiply two GF(929) elements using log/antilog tables.

    For a, b ≠ 0:  ``a × b = α^{(log[a] + log[b]) mod 928}``.
    If either operand is 0, the product is 0 — zero absorbs multiplication
    in any field.

    Example:
        ``_gf_mul(3, 3)`` = α^{1+1} = α^2 = 9 mod 929 = 9.
        ``_gf_mul(400, 400)`` = α^{k+k} where k = log(400).
    """
    if a == 0 or b == 0:
        return 0
    return _GF_EXP[(_GF_LOG[a] + _GF_LOG[b]) % _GF929_ORDER]


def _gf_add(a: int, b: int) -> int:
    """Add two GF(929) elements.

    GF(929) has characteristic 929 (NOT 2), so addition is ordinary modular
    integer addition — NOT bitwise XOR.  This is the key difference from
    GF(256) used by QR Code, Data Matrix, and Aztec Code.

    Example: _gf_add(100, 900) = (100 + 900) mod 929 = 71.
    """
    return (a + b) % _GF929_PRIME


# ---------------------------------------------------------------------------
# Reed-Solomon generator polynomial
# ---------------------------------------------------------------------------
#
# For ECC level L, k = 2^(L+1) ECC codewords.
#
# The generator polynomial uses the b=3 convention (different from QR's b=0
# or Data Matrix's b=1):
#
#   g(x) = (x − α^3)(x − α^4) ··· (x − α^{k+2})
#
# We build g iteratively by multiplying in one linear factor (x − α^j) at a
# time.  This is O(k^2) but k ≤ 512, and we cache each level's generator.


def _build_generator(ecc_level: int) -> tuple[int, ...]:
    """Build the RS generator polynomial for ECC level ``ecc_level``.

    Returns k+1 coefficients [g_k, g_{k-1}, ..., g_1, g_0] where:
        k = 2^(ecc_level + 1)
        g_k = 1  (leading coefficient — monic polynomial)

    The roots are α^3, α^4, ..., α^{k+2}  (b=3 convention).

    Algorithm:
        Start with g = [1].
        For j in 3..k+2:
            root = α^j
            multiply g by (x − root): new_g[i] += g[i], new_g[i+1] += g[i]×(−root)
    """
    k = 1 << (ecc_level + 1)  # 2^(ecc_level + 1)
    g: list[int] = [1]

    for j in range(3, k + 3):  # j = 3, 4, ..., k+2
        root = _GF_EXP[j % _GF929_ORDER]      # α^j
        neg_root = (_GF929_PRIME - root) % _GF929_PRIME  # −α^j in GF(929)

        new_g = [0] * (len(g) + 1)
        for i, coeff in enumerate(g):
            new_g[i] = _gf_add(new_g[i], coeff)
            new_g[i + 1] = _gf_add(new_g[i + 1], _gf_mul(coeff, neg_root))
        g = new_g

    return tuple(g)


# Cache generator polynomials — there are only 9 ECC levels (0–8).
_GEN_CACHE: dict[int, tuple[int, ...]] = {}


def _get_generator(ecc_level: int) -> tuple[int, ...]:
    """Return the generator polynomial for ``ecc_level`` (cached)."""
    cached = _GEN_CACHE.get(ecc_level)
    if cached is not None:
        return cached
    gen = _build_generator(ecc_level)
    _GEN_CACHE[ecc_level] = gen
    return gen


# ---------------------------------------------------------------------------
# Reed-Solomon encoder
# ---------------------------------------------------------------------------


def _rs_encode(data: list[int], ecc_level: int) -> list[int]:
    """Compute k RS ECC codewords for ``data`` over GF(929) with b=3 convention.

    Uses the standard shift-register (LFSR) polynomial long-division method.
    No interleaving — all data codewords feed a single RS encoder (simpler
    than QR Code which uses multiple interleaved blocks).

    Algorithm (shift register / LFSR method):
        ecc = [0] * k
        for d in data:
            feedback = (d + ecc[0]) mod 929
            shift ecc left by one (drop ecc[0], append 0)
            for i in 0..k-1:
                ecc[i] = (ecc[i] + g[k-i] × feedback) mod 929

    Parameters
    ----------
    data
        List of codeword values 0–928.
    ecc_level
        ECC level 0–8; determines k = 2^(ecc_level+1) ECC codewords.

    Returns
    -------
    list[int]
        k ECC codewords (each 0–928).
    """
    g = _get_generator(ecc_level)
    k = len(g) - 1  # degree of generator = number of ECC codewords
    ecc: list[int] = [0] * k

    for d in data:
        feedback = _gf_add(d, ecc[0])
        # Shift the register left by one position (drop ecc[0], append 0).
        ecc = ecc[1:] + [0]
        # Add feedback × generator coefficient to each cell.
        for i in range(k):
            ecc[i] = _gf_add(ecc[i], _gf_mul(g[k - i], feedback))

    return ecc


# ---------------------------------------------------------------------------
# Byte compaction
# ---------------------------------------------------------------------------
#
# 6 bytes → 5 codewords: treat 6 bytes as a 48-bit big-endian integer and
# express it in base 900.  This is lossless because:
#
#   256^6 = 281,474,976,710,656
#   900^5 = 590,490,000,000,000
#   256^6 < 900^5  ✓  (all 6-byte groups fit in 5 base-900 digits)
#
# Python's arbitrary-precision integers handle the 48-bit arithmetic natively.


def _byte_compact(data: bytes) -> list[int]:
    """Encode bytes using byte compaction (codeword 924 latch).

    Returns [924, c1, c2, ...] where c_i are byte-compacted codewords.
    Codeword 924 is the "any-length" byte compaction latch.

    Encoding steps:
    1. Emit codeword 924 (latch).
    2. Process full 6-byte groups: each group → 5 codewords via base-900.
    3. Remaining 1–5 bytes: 1 codeword per byte (direct value 0–255).

    The 6-bytes-to-5-codewords compression factor is 6/5 = 1.2 bytes per
    codeword.  The remainder uses 1 byte per codeword (no compression).
    """
    codewords: list[int] = [_LATCH_BYTE]

    i = 0
    n = len(data)

    # Process full 6-byte groups → 5 codewords each.
    while i + 6 <= n:
        # Treat the 6 bytes as a 48-bit big-endian integer.
        # Python ints are arbitrary precision, so no overflow risk.
        group_val = int.from_bytes(data[i : i + 6], "big")

        # Convert to base 900 (most-significant codeword first).
        group: list[int] = [0] * 5
        for j in range(4, -1, -1):
            group[j] = group_val % 900
            group_val //= 900

        codewords.extend(group)
        i += 6

    # Remaining 1–5 bytes: direct byte value (0–255) as a codeword each.
    while i < n:
        codewords.append(data[i])
        i += 1

    return codewords


# ---------------------------------------------------------------------------
# ECC level auto-selection
# ---------------------------------------------------------------------------


def _auto_ecc_level(data_cw_count: int) -> int:
    """Select the minimum recommended ECC level based on data codeword count.

    These thresholds come from the PDF417 best-practices guidance:
    higher data density → higher ECC level to maintain scan reliability.

    Table (data codeword count → minimum ECC level):
        ≤  40  →  level 2  ( 8 ECC codewords)
        ≤ 160  →  level 3  (16 ECC codewords)
        ≤ 320  →  level 4  (32 ECC codewords)
        ≤ 863  →  level 5  (64 ECC codewords)
        >  863  →  level 6 (128 ECC codewords)
    """
    if data_cw_count <= 40:
        return 2
    if data_cw_count <= 160:
        return 3
    if data_cw_count <= 320:
        return 4
    if data_cw_count <= 863:
        return 5
    return 6


# ---------------------------------------------------------------------------
# Dimension selection
# ---------------------------------------------------------------------------


def _choose_dimensions(total_codewords: int) -> tuple[int, int]:
    """Choose the number of data columns and rows for the symbol.

    Heuristic: target a roughly square symbol.  A PDF417 row is about
    3× wider than tall (due to 17-module codewords vs. ~3-module row height),
    so we scale accordingly.

    Algorithm:
        cols = ceil(sqrt(total / 3)), clamped to 1..30
        rows = ceil(total / cols), clamped to 3..90

    Returns
    -------
    tuple[int, int]
        (cols, rows) — number of data columns and number of rows.
    """
    cols = max(_MIN_COLS, min(_MAX_COLS, math.ceil(math.sqrt(total_codewords / 3))))
    rows = max(_MIN_ROWS, math.ceil(total_codewords / cols))

    # If rows clamped at MIN_ROWS, recompute cols so it actually fits.
    if rows < _MIN_ROWS:
        rows = _MIN_ROWS
        cols = max(_MIN_COLS, min(_MAX_COLS, math.ceil(total_codewords / rows)))
        rows = max(_MIN_ROWS, math.ceil(total_codewords / cols))

    rows = min(_MAX_ROWS, rows)
    return cols, rows


# ---------------------------------------------------------------------------
# Row indicator computation
# ---------------------------------------------------------------------------
#
# Each row carries two special codewords — Left Row Indicator (LRI) and Right
# Row Indicator (RRI) — that together encode three quantities:
#
#   R_info = (rows - 1) // 3       (encodes total row count)
#   C_info = cols - 1              (encodes column count)
#   L_info = 3 * ecc_level + (rows - 1) % 3  (encodes ECC level + row parity)
#
# These three values are distributed across the three clusters:
#
#   row % 3 == 0:  LRI = 30*(row//3) + R_info,  RRI = 30*(row//3) + C_info
#   row % 3 == 1:  LRI = 30*(row//3) + L_info,  RRI = 30*(row//3) + R_info
#   row % 3 == 2:  LRI = 30*(row//3) + C_info,  RRI = 30*(row//3) + L_info
#
# A scanner that reads three consecutive rows (one of each cluster) can
# reconstruct all three values — and from them R, C, and L — even without
# reading the full symbol.
#
# Note: the RRI formula (C_info, R_info, L_info for clusters 0, 1, 2) follows
# the Python pdf417 library convention which produces verified scannable symbols.


def compute_lri(r: int, rows: int, cols: int, ecc_level: int) -> int:
    """Compute the Left Row Indicator codeword value for row ``r``.

    Parameters
    ----------
    r
        0-indexed row number.
    rows
        Total number of rows in the symbol (R).
    cols
        Number of data columns (C).
    ecc_level
        Reed-Solomon ECC level (0–8).

    Returns
    -------
    int
        LRI codeword value (0–928).

    Examples
    --------
    For a 10-row, 3-column, ECC level 2 symbol:
        R_info = (10-1)//3 = 3
        C_info = 3-1 = 2
        L_info = 3×2 + (10-1)%3 = 6+0 = 6

        Row 0 (cluster 0): LRI = 30×0 + 3 = 3
        Row 1 (cluster 1): LRI = 30×0 + 6 = 6
        Row 2 (cluster 2): LRI = 30×0 + 2 = 2
        Row 3 (cluster 0): LRI = 30×1 + 3 = 33
    """
    r_info = (rows - 1) // 3
    c_info = cols - 1
    l_info = 3 * ecc_level + (rows - 1) % 3
    row_group = r // 3
    cluster = r % 3

    if cluster == 0:
        return 30 * row_group + r_info
    if cluster == 1:
        return 30 * row_group + l_info
    return 30 * row_group + c_info  # cluster == 2


def compute_rri(r: int, rows: int, cols: int, ecc_level: int) -> int:
    """Compute the Right Row Indicator codeword value for row ``r``.

    The RRI for the same row encodes complementary information to the LRI.
    Given the LRI and RRI from a single row, plus the cluster index (from
    the bar/space patterns themselves), a scanner can uniquely determine the
    cluster and recover at least one of R_info, C_info, L_info.

    Parameters
    ----------
    r
        0-indexed row number.
    rows
        Total number of rows in the symbol (R).
    cols
        Number of data columns (C).
    ecc_level
        Reed-Solomon ECC level (0–8).

    Returns
    -------
    int
        RRI codeword value (0–928).
    """
    r_info = (rows - 1) // 3
    c_info = cols - 1
    l_info = 3 * ecc_level + (rows - 1) % 3
    row_group = r // 3
    cluster = r % 3

    if cluster == 0:
        return 30 * row_group + c_info
    if cluster == 1:
        return 30 * row_group + r_info
    return 30 * row_group + l_info  # cluster == 2


# ---------------------------------------------------------------------------
# Pattern expansion helpers
# ---------------------------------------------------------------------------


def _expand_pattern(packed: int) -> list[bool]:
    """Expand a packed 32-bit bar/space pattern into 17 boolean module values.

    The pattern packs 8 element widths (4 bars + 4 spaces) as 4 bits each:
        bits 31..28 = b1 (first bar width)
        bits 27..24 = s1 (first space width)
        bits 23..20 = b2
        bits 19..16 = s2
        bits 15..12 = b3
        bits 11..8  = s3
        bits 7..4   = b4
        bits 3..0   = s4

    Modules alternate: bar (dark=True), space (dark=False), bar, space, …
    The sum b1+s1+b2+s2+b3+s3+b4+s4 = 17 for all valid patterns.

    Returns
    -------
    list[bool]
        17 boolean values where True = dark module.
    """
    b1 = (packed >> 28) & 0xF
    s1 = (packed >> 24) & 0xF
    b2 = (packed >> 20) & 0xF
    s2 = (packed >> 16) & 0xF
    b3 = (packed >> 12) & 0xF
    s3 = (packed >>  8) & 0xF
    b4 = (packed >>  4) & 0xF
    s4 =  packed        & 0xF

    modules: list[bool] = []
    modules.extend([True]  * b1)
    modules.extend([False] * s1)
    modules.extend([True]  * b2)
    modules.extend([False] * s2)
    modules.extend([True]  * b3)
    modules.extend([False] * s3)
    modules.extend([True]  * b4)
    modules.extend([False] * s4)
    return modules


def _expand_widths(widths: tuple[int, ...]) -> list[bool]:
    """Expand a bar/space width sequence into boolean module values.

    The first element is always a bar (dark=True).  Each subsequent element
    alternates between space (False) and bar (True).

    Used for the start pattern (17 modules) and stop pattern (18 modules),
    which are the same for every row regardless of cluster.

    Parameters
    ----------
    widths
        Sequence of element widths (e.g. START_PATTERN = (8, 1, 1, 1, 1, 1, 1, 3)).

    Returns
    -------
    list[bool]
        Boolean module values.
    """
    modules: list[bool] = []
    dark = True
    for w in widths:
        modules.extend([dark] * w)
        dark = not dark
    return modules


# ---------------------------------------------------------------------------
# Rasterization
# ---------------------------------------------------------------------------


def _rasterize(
    sequence: list[int],
    rows: int,
    cols: int,
    ecc_level: int,
    row_height: int,
) -> ModuleGrid:
    """Convert the flat codeword sequence into a ``ModuleGrid``.

    Each logical row in the symbol produces ``row_height`` identical module
    rows in the grid.  Within each logical row, modules are laid out left to
    right:

        [start(17)] [LRI(17)] [data×cols(17 each)] [RRI(17)] [stop(18)]

    Total module columns = 17 + 17 + cols×17 + 17 + 18 = 69 + 17×cols.

    Parameters
    ----------
    sequence
        Full codeword sequence of length rows×cols, in row-major order.
    rows
        Number of logical PDF417 rows (3–90).
    cols
        Number of data columns (1–30).
    ecc_level
        ECC level used (affects row indicator computation).
    row_height
        Number of module rows per logical row (≥ 1).

    Returns
    -------
    ModuleGrid
        The assembled symbol as an abstract boolean grid.
    """
    module_width = 69 + 17 * cols
    module_height = rows * row_height

    grid = make_module_grid(module_height, module_width, "square")

    # Precompute the start and stop module sequences — these are identical for
    # every row, so we only compute them once.
    start_modules = _expand_widths(START_PATTERN)   # 17 modules
    stop_modules  = _expand_widths(STOP_PATTERN)    # 18 modules

    for r in range(rows):
        cluster_idx = r % 3
        cluster_table = CLUSTER_TABLES[cluster_idx]

        row_modules: list[bool] = []

        # 1. Start pattern — 17 modules, fixed.
        row_modules.extend(start_modules)

        # 2. Left Row Indicator — 17 modules, depends on row metadata.
        lri = compute_lri(r, rows, cols, ecc_level)
        row_modules.extend(_expand_pattern(cluster_table[lri]))

        # 3. Data codewords — cols×17 modules.
        for j in range(cols):
            cw = sequence[r * cols + j]
            row_modules.extend(_expand_pattern(cluster_table[cw]))

        # 4. Right Row Indicator — 17 modules.
        rri = compute_rri(r, rows, cols, ecc_level)
        row_modules.extend(_expand_pattern(cluster_table[rri]))

        # 5. Stop pattern — 18 modules, fixed.
        row_modules.extend(stop_modules)

        # Sanity check: every row must produce exactly module_width modules.
        assert len(row_modules) == module_width, (
            f"Row {r}: expected {module_width} modules, got {len(row_modules)}"
        )

        # Write this module row into the grid, repeated row_height times.
        module_row_base = r * row_height
        for h in range(row_height):
            module_row = module_row_base + h
            for col_idx in range(module_width):
                if row_modules[col_idx]:
                    grid = set_module(grid, module_row, col_idx, True)

    return grid


# ---------------------------------------------------------------------------
# Main encode function
# ---------------------------------------------------------------------------


def encode(
    data: str,
    *,
    ecc_level: int | None = None,
    columns: int | None = None,
    row_height: int = 3,
) -> ModuleGrid:
    """Encode a string as a PDF417 symbol and return the ``ModuleGrid``.

    The full encoding pipeline is:

    1. **Byte compact** the UTF-8 bytes (codeword 924 latch).
    2. **Auto-select ECC level** (or use ``ecc_level`` if provided).
    3. **Compute length descriptor** (codeword 0 = total codewords in symbol).
    4. **Compute RS ECC** over GF(929) with b=3 convention.
    5. **Choose dimensions** (auto: ``cols = ceil(√(total/3))``, clamped).
    6. **Pad** unused slots with codeword 900.
    7. **Rasterize**: per-row, emit start + LRI + data codewords + RRI + stop.

    Parameters
    ----------
    data
        The string to encode.  Encoded as UTF-8 bytes, then byte-compacted.
    ecc_level
        Reed-Solomon ECC level 0–8.  ``None`` → auto-select.
    columns
        Number of data columns (1–30).  ``None`` → auto-select.
    row_height
        Number of module-rows per logical PDF417 row (≥ 1, default 3).
        Larger values make the symbol taller — useful for low-resolution
        printing where the scanner needs vertical margin to find the row.

    Returns
    -------
    ModuleGrid
        The PDF417 symbol as an abstract boolean grid (True = dark module).

    Raises
    ------
    InvalidECCLevelError
        If ``ecc_level`` is not in 0–8.
    InvalidDimensionsError
        If ``columns`` is not in 1–30.
    InputTooLongError
        If the data cannot fit in any valid PDF417 symbol with the given
        parameters.

    Examples
    --------
    Encode a short string::

        grid = encode("HELLO")
        assert grid.rows == grid.rows  # check dimensions

    Override ECC level and column count::

        grid = encode("HELLO WORLD", ecc_level=3, columns=5)

    Notes
    -----
    v0.1.0 implements byte compaction only.  All input (even pure ASCII or
    digits) is encoded via the byte path.  Text and numeric compaction
    (which yield denser codeword sequences for ASCII/digit inputs) are
    planned for v0.2.0.
    """
    # ── Validate options ─────────────────────────────────────────────────────
    if ecc_level is not None and not (0 <= ecc_level <= 8):
        raise InvalidECCLevelError(
            f"ECC level must be 0–8, got {ecc_level}"
        )
    if columns is not None and not (_MIN_COLS <= columns <= _MAX_COLS):
        raise InvalidDimensionsError(
            f"columns must be {_MIN_COLS}–{_MAX_COLS}, got {columns}"
        )

    # ── Step 1: byte compaction ───────────────────────────────────────────────
    raw_bytes = data.encode("utf-8")
    data_cwords = _byte_compact(raw_bytes)

    # ── Step 2: auto-select ECC level ────────────────────────────────────────
    # length descriptor is +1, so total data before ECC is len(data_cwords)+1
    effective_ecc = ecc_level if ecc_level is not None else _auto_ecc_level(
        len(data_cwords) + 1
    )
    ecc_count = 1 << (effective_ecc + 1)  # 2^(ecc_level+1)

    # ── Step 3: length descriptor ─────────────────────────────────────────────
    # The length descriptor is the very first codeword in the symbol.  Its
    # value = (number of codewords it covers) = 1 (itself) + data + ECC.
    # Padding is NOT included in the length descriptor.
    length_desc = 1 + len(data_cwords) + ecc_count
    full_data: list[int] = [length_desc] + data_cwords

    # ── Step 4: RS ECC ────────────────────────────────────────────────────────
    ecc_cwords = _rs_encode(full_data, effective_ecc)

    # ── Step 5: choose dimensions ─────────────────────────────────────────────
    total_cwords = len(full_data) + len(ecc_cwords)

    if columns is not None:
        cols = columns
        raw_rows = math.ceil(total_cwords / cols)
        if raw_rows > _MAX_ROWS:
            raise InputTooLongError(
                f"Data requires {raw_rows} rows (max {_MAX_ROWS}) with "
                f"{cols} columns."
            )
        rows = max(_MIN_ROWS, raw_rows)
    else:
        cols, rows = _choose_dimensions(total_cwords)

    # Verify capacity.
    if cols * rows < total_cwords:
        raise InputTooLongError(
            f"Cannot fit {total_cwords} codewords in {rows}×{cols} grid."
        )

    # ── Step 6: pad ───────────────────────────────────────────────────────────
    # Padding goes between the data codewords and the ECC codewords.
    # full_sequence = [length_desc, ...data_cwords, ...padding, ...ecc_cwords]
    padding_count = cols * rows - total_cwords
    padded_data: list[int] = full_data + [_PADDING_CW] * padding_count
    full_sequence: list[int] = padded_data + ecc_cwords

    assert len(full_sequence) == cols * rows, (
        f"Expected {cols*rows} codewords, got {len(full_sequence)}"
    )

    # ── Step 7: rasterize ─────────────────────────────────────────────────────
    effective_row_height = max(1, row_height)
    return _rasterize(full_sequence, rows, cols, effective_ecc, effective_row_height)
