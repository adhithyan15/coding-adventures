"""intel_4004_packager — Intel HEX ROM packager for the Nib compiler pipeline.

This is the final stage of the Nib → Intel 4004 compiler toolchain.  It
takes raw binary machine code produced by the assembler and wraps it in
the Intel HEX format used by EPROM programmers.

It also exposes the ``Intel4004Packager`` class, which orchestrates the
*entire* pipeline in a single ``pack_source()`` call:

  Nib source → parse → type-check → IR compile → optimize
             → backend → assemble → Intel HEX

Quick Start
-----------

::

    from intel_4004_packager import Intel4004Packager, encode_hex, decode_hex

    # Full pipeline: Nib source → Intel HEX
    packager = Intel4004Packager()
    result = packager.pack_source('''
        fn main() -> u4 {
            let x: u4 = 7
            return x
        }
    ''')
    print(result.hex_text)

    # Lower level: binary → Intel HEX
    raw_bytes = bytes([0xD7, 0x01])   # LDM 7, HLT
    hex_text = encode_hex(raw_bytes)

    # Round-trip: Intel HEX → binary
    origin, binary = decode_hex(hex_text)
    assert binary == raw_bytes

Public API
----------

- ``Intel4004Packager``  — full-pipeline orchestrator
- ``PackageResult``      — all artifacts from ``pack_source()``
- ``PackageError``       — raised on pipeline failure
- ``encode_hex(binary, origin=0) -> str``  — binary → Intel HEX
- ``decode_hex(hex_text) -> (origin, bytes)``  — Intel HEX → binary
"""

from __future__ import annotations

from intel_4004_packager.hex_encoder import decode_hex, encode_hex
from intel_4004_packager.packager import Intel4004Packager, PackageError, PackageResult

__all__ = [
    "Intel4004Packager",
    "PackageError",
    "PackageResult",
    "decode_hex",
    "encode_hex",
]
