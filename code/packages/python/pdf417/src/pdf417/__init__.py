"""PDF417 stacked linear barcode encoder — ISO/IEC 15438:2015 compliant.

PDF417 (Portable Data File 417) was invented by Ynjiun P. Wang at Symbol
Technologies in 1991.  The name encodes the format's geometry: each codeword
has exactly **4** bars and **4** spaces (8 elements), and every codeword
occupies exactly **17** modules of horizontal space.  "417" = 4 × 17.

Where PDF417 is deployed
------------------------
+------------------+-----------------------------------------------------------+
| Application      | Detail                                                    |
+==================+===========================================================+
| AAMVA            | North American driver's licences and government IDs       |
+------------------+-----------------------------------------------------------+
| IATA BCBP        | Airline boarding passes (the barcode on your phone)       |
+------------------+-----------------------------------------------------------+
| USPS             | Domestic shipping labels                                  |
+------------------+-----------------------------------------------------------+
| US immigration   | Form I-94, customs declarations                           |
+------------------+-----------------------------------------------------------+
| Healthcare       | Patient wristbands, medication labels                     |
+------------------+-----------------------------------------------------------+

Key properties
--------------
- **Stacked linear** (not a matrix code): each row is an independent strip
  readable by a single horizontal scan.
- **GF(929) Reed-Solomon**: ECC over the prime field GF(929), not GF(256).
- **Three clusters**: rows cycle through three codeword-to-pattern mappings,
  making each row's cluster identifiable even in isolation.
- **Variable dimensions**: 3–90 rows, 1–30 data columns.
- **ECC levels 0–8**: 2 to 512 ECC codewords.

Encoding pipeline
-----------------
.. code-block::

    input string
      → UTF-8 bytes
      → byte compaction (codeword 924 + 6-bytes-to-5-codewords base-900)
      → length descriptor (codeword 0 = total codewords in symbol)
      → GF(929) Reed-Solomon ECC (b=3 convention, α=3)
      → dimension selection (auto: roughly square symbol)
      → padding (codeword 900 fills unused slots)
      → row indicators (LRI + RRI per row encode R/C/ECC level)
      → cluster table lookup (codeword → 17-module bar/space pattern)
      → start/stop patterns (fixed 17/18 modules per row)
      → ModuleGrid (abstract boolean grid, True = dark)

Public API
----------
- :func:`encode` — encode a string to a ``ModuleGrid``.
- :func:`compute_lri` — compute Left Row Indicator for a given row.
- :func:`compute_rri` — compute Right Row Indicator for a given row.
- :func:`grid_to_string` — debug rendering as '0'/'1' string.
- :class:`PDF417Error` — base error class.
- :class:`InputTooLongError` — data exceeds symbol capacity.
- :class:`InvalidDimensionsError` — rows/columns out of valid range.
- :class:`InvalidECCLevelError` — ECC level not in 0–8.

v0.1.0 scope
------------
This release implements **byte compaction only** (codeword 924 latch).  All
inputs are encoded as raw bytes regardless of content.  Text and numeric
compaction (yielding denser codeword sequences for ASCII/digit inputs) are
planned for v0.2.0.
"""

from __future__ import annotations

from typing import Final

from barcode_2d import ModuleGrid

from .pdf417 import (
    InvalidDimensionsError,
    InvalidECCLevelError,
    InputTooLongError,
    PDF417Error,
    compute_lri,
    compute_rri,
    encode,
)

__version__: Final = "0.1.0"


def grid_to_string(grid: ModuleGrid) -> str:
    """Render a ``ModuleGrid`` as a multi-line '0' / '1' string.

    Useful for debugging, snapshot tests, and cross-language corpus
    comparison.  Each row is one line; rows are separated by newlines;
    no trailing newline.

    Parameters
    ----------
    grid
        A ``ModuleGrid`` returned by :func:`encode`.

    Returns
    -------
    str
        Multi-line string where '1' = dark module, '0' = light module.

    Examples
    --------
    ::

        grid = encode("A")
        s = grid_to_string(grid)
        lines = s.split("\\n")
        # Every row has the same width.
        assert all(len(line) == len(lines[0]) for line in lines)
    """
    return "\n".join(
        "".join("1" if grid.modules[r][c] else "0" for c in range(grid.cols))
        for r in range(grid.rows)
    )


__all__ = [
    # Version
    "__version__",
    # Core encoder
    "encode",
    # Row indicator helpers (exported for testing and cross-language verification)
    "compute_lri",
    "compute_rri",
    # Utilities
    "grid_to_string",
    # Error types
    "PDF417Error",
    "InputTooLongError",
    "InvalidDimensionsError",
    "InvalidECCLevelError",
]
