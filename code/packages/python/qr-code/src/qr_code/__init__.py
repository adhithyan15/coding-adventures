"""qr_code — ISO/IEC 18004:2015 compliant QR Code encoder.

This package encodes any UTF-8 string into a QR Code ``ModuleGrid`` —
an abstract 2-D boolean grid (``True`` = dark module).  The grid can then
be converted to a pixel-level ``PaintScene`` via ``barcode_2d.layout()``.

## Quick start

::

    from qr_code import encode, encode_to_scene

    # Encode to abstract module grid
    grid = encode("Hello, World!", level="M")
    print(f"Symbol size: {grid.rows} × {grid.cols}")

    # Encode and convert to PaintScene for rendering
    scene = encode_to_scene("Hello, World!", level="M")

## Error correction levels

+-------+-----------+------------------------------------------+
| Level | Recovery  | Use case                                 |
+=======+===========+==========================================+
| L     | ~7 %      | Maximum data density, clean environment  |
+-------+-----------+------------------------------------------+
| M     | ~15 %     | General purpose (default)                |
+-------+-----------+------------------------------------------+
| Q     | ~25 %     | Outdoor / industrial                     |
+-------+-----------+------------------------------------------+
| H     | ~30 %     | Logo overlay, high damage risk           |
+-------+-----------+------------------------------------------+

## Pipeline overview

::

    Input string
      → mode selection       (numeric / alphanumeric / byte)
      → version selection    (smallest v1–40 that fits)
      → bit stream assembly  (mode indicator + char count + data + padding)
      → block splitting      (ISO block table)
      → RS ECC computation   (GF(256), b=0 convention)
      → interleaving         (round-robin across blocks)
      → grid init            (finder, separator, timing, alignment, dark)
      → zigzag data placement
      → mask evaluation      (8 candidates, lowest 4-rule penalty wins)
      → format/version info write
      → ModuleGrid
"""

from __future__ import annotations

from qr_code._qr_code import (
    ALIGNMENT_POSITIONS,
    ALPHANUM_CHARS,
    ECC_CODEWORDS_PER_BLOCK,
    ECC_IDX,
    ECC_INDICATOR,
    MODE_INDICATOR,
    NUM_BLOCKS,
    Barcode2DLayoutConfig,
    InputTooLongError,
    InvalidInputError,
    ModuleGrid,
    PaintScene,
    QRCodeError,
    apply_mask,
    build_data_codewords,
    compute_blocks,
    compute_format_bits,
    compute_penalty,
    compute_version_bits,
    encode,
    encode_to_scene,
    interleave_blocks,
    num_data_codewords,
    num_raw_data_modules,
    num_remainder_bits,
    rs_encode,
    select_mode,
    select_version,
    symbol_size,
    write_format_info,
)

__version__ = "0.1.0"

__all__ = [
    # Public API
    "encode",
    "encode_to_scene",
    # Errors
    "QRCodeError",
    "InputTooLongError",
    "InvalidInputError",
    # Low-level helpers (educational / testing)
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
    # Version
    "__version__",
]
