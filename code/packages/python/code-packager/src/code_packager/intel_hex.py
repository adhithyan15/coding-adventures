"""IntelHexPackager: wraps native_bytes in Intel HEX format.

Intel HEX is the industry-standard format for programming EPROM chips and
loading firmware into embedded simulators.  Every record on a line starts
with ``:``, encodes a chunk of data at a specific address, and ends with a
two's-complement checksum byte.

Record format (ASCII)::

    :LLAAAATTDD...CC
     LL   = byte count (hex, 1 byte)
     AAAA = start address (hex, 2 bytes big-endian)
     TT   = record type: 00=data, 01=EOF
     DD   = data bytes
     CC   = checksum: two's complement of (LL + addr_hi + addr_lo + TT + sum(DD))

This packager delegates to ``intel_4004_packager.encode_hex``, which despite
its name produces standard Intel HEX for any binary.

Metadata keys
-------------
``origin`` (int, default 0)
    The ROM load address.  All data records are offset from this address.
    For Intel 4004 programs, conventionally 0.  For Intel 8008 programs,
    the usual start address is also 0.

Example
-------
::

    artifact = CodeArtifact(
        native_bytes=b"\\xD7\\x01\\xC0",
        entry_point=0,
        target=Target.intel_4004(),
    )
    packager = IntelHexPackager()
    hex_str_bytes = packager.pack(artifact)
    # Returns UTF-8–encoded Intel HEX text, e.g.:
    # b":03000000D701C038\\n:00000001FF\\n"
"""

from __future__ import annotations

from intel_4004_packager import encode_hex

from code_packager.artifact import CodeArtifact
from code_packager.errors import UnsupportedTargetError
from code_packager.target import Target


class IntelHexPackager:
    """Package native bytes as an Intel HEX ROM image.

    Accepted targets: any ``Target`` with ``binary_format="intel_hex"``.
    """

    supported_targets: frozenset[Target] = frozenset({
        Target.intel_4004(),
        Target.intel_8008(),
    })

    def pack(self, artifact: CodeArtifact) -> bytes:
        if artifact.target.binary_format != "intel_hex":
            raise UnsupportedTargetError(artifact.target)
        origin: int = int(artifact.metadata.get("origin", 0))
        hex_text = encode_hex(artifact.native_bytes, origin=origin)
        return hex_text.encode("ascii")

    def file_extension(self, target: Target) -> str:  # noqa: ARG002
        return ".hex"
