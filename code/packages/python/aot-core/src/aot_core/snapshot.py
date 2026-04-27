"""AOT snapshot format: writer and reader for ``.aot`` binaries.

The ``.aot`` file is a self-contained binary that can be executed on the
target architecture.  Its layout:

::

    ┌─────────────────────────────────────────┐
    │ Header (26 bytes)                       │
    │   magic             4 bytes  "AOT\\0"   │
    │   version           2 bytes  0x01 0x00  │
    │   flags             4 bytes             │
    │   entry_point_offset 4 bytes            │
    │   vm_iir_table_offset 4 bytes           │
    │   vm_iir_table_size  4 bytes            │
    │   native_code_size  4 bytes             │
    ├─────────────────────────────────────────┤
    │ Code section                            │
    │   N bytes  native machine code          │
    ├─────────────────────────────────────────┤
    │ IIR table section (optional)            │
    │   M bytes  serialised IIR for dynamic   │
    └─────────────────────────────────────────┘

Flags
-----
bit 0 (``FLAG_VM_RUNTIME``)  — vm-runtime IIR table is present
bit 1 (``FLAG_DEBUG_INFO``)  — debug section present (future; always 0 now)

Endianness: little-endian throughout.

Example
-------
Write and round-trip a simple snapshot:

>>> code = b"\\xde\\xad\\xbe\\xef"
>>> raw = write(code, entry_point_offset=0)
>>> snap = read(raw)
>>> snap.native_code == code
True
>>> snap.iir_table is None
True
"""

from __future__ import annotations

import struct
from dataclasses import dataclass

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

MAGIC: bytes = b"AOT\x00"
VERSION: int = 0x0100  # major=1, minor=0 packed as big-endian uint16

FLAG_VM_RUNTIME: int = 0x01
FLAG_DEBUG_INFO: int = 0x02

# Header layout: magic(4) + version(2) + flags(4) + entry_point_offset(4)
#                + vm_iir_table_offset(4) + vm_iir_table_size(4)
#                + native_code_size(4)  = 26 bytes total
_HEADER_FORMAT: str = "<4sHIIIII"
HEADER_SIZE: int = struct.calcsize(_HEADER_FORMAT)  # 26


# ---------------------------------------------------------------------------
# Data types
# ---------------------------------------------------------------------------

@dataclass
class AOTSnapshot:
    """Parsed contents of a ``.aot`` binary.

    Attributes
    ----------
    version:
        Packed version word (``0x0100`` = v1.0).
    flags:
        Bitmask of flags (see ``FLAG_*`` constants).
    entry_point_offset:
        Byte offset within ``native_code`` where the entry-point function begins.
    native_code:
        Raw native machine code bytes (all compiled functions concatenated).
    iir_table:
        Serialised IIR bytes for uncompiled functions, or ``None`` if the
        vm-runtime section is absent.
    """

    version: int
    flags: int
    entry_point_offset: int
    native_code: bytes
    iir_table: bytes | None

    @property
    def has_vm_runtime(self) -> bool:
        """True if the IIR table section is present."""
        return bool(self.flags & FLAG_VM_RUNTIME)


# ---------------------------------------------------------------------------
# Writer
# ---------------------------------------------------------------------------

def write(
    native_code: bytes,
    iir_table: bytes | None = None,
    entry_point_offset: int = 0,
) -> bytes:
    """Serialise a compiled module to the ``.aot`` binary format.

    Parameters
    ----------
    native_code:
        Combined native binary produced by the backend (all functions
        concatenated, typically via ``link.link()``).
    iir_table:
        Serialised IIR bytes for functions that could not be compiled
        (produced by ``vm_runtime.serialise_iir_table()``).  Pass ``None``
        if all functions were compiled.
    entry_point_offset:
        Byte offset within ``native_code`` where the entry-point function
        (``main``) starts.

    Returns
    -------
    bytes
        Complete ``.aot`` binary ready for writing to disk or passing to a
        simulator.
    """
    flags = 0
    iir_bytes = iir_table or b""
    if iir_table is not None:
        flags |= FLAG_VM_RUNTIME

    native_code_size = len(native_code)
    vm_iir_table_size = len(iir_bytes)

    # IIR table lives immediately after the code section.
    code_section_start = HEADER_SIZE
    vm_iir_table_offset = (
        code_section_start + native_code_size if iir_table is not None else 0
    )

    header = struct.pack(
        _HEADER_FORMAT,
        MAGIC,
        VERSION,
        flags,
        entry_point_offset,
        vm_iir_table_offset,
        vm_iir_table_size,
        native_code_size,
    )
    return header + native_code + iir_bytes


# ---------------------------------------------------------------------------
# Reader
# ---------------------------------------------------------------------------

def read(data: bytes) -> AOTSnapshot:
    """Parse a ``.aot`` binary.

    Parameters
    ----------
    data:
        Raw bytes of the ``.aot`` file.

    Returns
    -------
    AOTSnapshot
        Parsed snapshot with all sections separated.

    Raises
    ------
    ValueError
        If the magic bytes are wrong, the data is too short, or the
        declared section offsets are out of bounds.
    """
    if len(data) < HEADER_SIZE:
        raise ValueError(f"data too short: {len(data)} < {HEADER_SIZE}")

    (
        magic,
        version,
        flags,
        entry_point_offset,
        vm_iir_table_offset,
        vm_iir_table_size,
        native_code_size,
    ) = struct.unpack_from(_HEADER_FORMAT, data, 0)

    if magic != MAGIC:
        raise ValueError(f"bad magic: {magic!r} (expected {MAGIC!r})")

    code_start = HEADER_SIZE
    code_end = code_start + native_code_size
    if code_end > len(data):
        raise ValueError(
            f"native_code section truncated: need {code_end} bytes, have {len(data)}"
        )
    native_code = data[code_start:code_end]

    iir_table: bytes | None = None
    if flags & FLAG_VM_RUNTIME:
        if vm_iir_table_size == 0:
            iir_table = b""
        else:
            expected_offset = HEADER_SIZE + native_code_size
            if vm_iir_table_offset < expected_offset:
                raise ValueError(
                    f"iir_table offset {vm_iir_table_offset} overlaps header"
                    f" or code section (expected >= {expected_offset})"
                )
            iir_end = vm_iir_table_offset + vm_iir_table_size
            if iir_end > len(data):
                raise ValueError(
                    f"iir_table section truncated: need {iir_end} bytes,"
                    f" have {len(data)}"
                )
            iir_table = data[vm_iir_table_offset:iir_end]

    return AOTSnapshot(
        version=version,
        flags=flags,
        entry_point_offset=entry_point_offset,
        native_code=native_code,
        iir_table=iir_table,
    )
