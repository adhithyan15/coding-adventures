"""Independent UUID oracle for the uuid-v1 shared fixture.

Pure Python standard library only (hashlib for MD5/SHA-1). This oracle is
written directly from RFC 4122 / RFC 9562 and the uuid.md algorithms — not
from any lane's implementation — so agreement between a lane and this oracle
is genuine differential evidence.

Conventions:
- UUIDs are 16-byte big-endian ``bytes`` objects.
- ``parse`` mirrors the reference leniency exactly: trim, strip one
  ``urn:uuid:`` prefix, strip one layer of braces, delete every hyphen,
  require 32 hex digits. Hyphen positions are NOT validated.
"""

from __future__ import annotations

import hashlib

NAMESPACES = {
    "dns": bytes.fromhex("6ba7b8109dad11d180b400c04fd430c8"),
    "url": bytes.fromhex("6ba7b8119dad11d180b400c04fd430c8"),
    "oid": bytes.fromhex("6ba7b8129dad11d180b400c04fd430c8"),
    "x500": bytes.fromhex("6ba7b8149dad11d180b400c04fd430c8"),
}

NIL = bytes(16)
MAX = bytes([0xFF] * 16)


class UuidOracleError(ValueError):
    """Raised when oracle input is not a valid UUID representation."""


def format_uuid(raw: bytes) -> str:
    """Format 16 bytes as canonical 8-4-4-4-12 lowercase."""
    assert len(raw) == 16
    h = raw.hex()
    return f"{h[0:8]}-{h[8:12]}-{h[12:16]}-{h[16:20]}-{h[20:32]}"


def parse(text: str) -> bytes:
    """Parse standard, compact, braced, or URN forms to 16 bytes."""
    s = text.strip()
    s = s.removeprefix("urn:uuid:")
    s = s.removeprefix("{")
    s = s.removesuffix("}")
    hexpart = s.replace("-", "")
    if len(hexpart) != 32:
        raise UuidOracleError(f"expected 32 hex digits, got {len(hexpart)}: {s!r}")
    try:
        return bytes.fromhex(hexpart)
    except ValueError:
        raise UuidOracleError(f"non-hex character in UUID string: {s!r}")


def version_of(raw: bytes) -> int:
    """High nibble of byte 6."""
    return (raw[6] >> 4) & 0xF


def variant_of(raw: bytes) -> str:
    """Variant string from the high bits of byte 8."""
    top = (raw[8] >> 6) & 0x3
    if top in (0, 1):
        return "ncs"
    if top == 2:
        return "rfc4122"
    return "microsoft" if ((raw[8] >> 5) & 1) == 0 else "reserved"


def stamp(raw: bytearray, version: int) -> bytes:
    """Set version nibble and RFC 4122 variant bits, returning bytes."""
    raw[6] = (raw[6] & 0x0F) | (version << 4)
    raw[8] = (raw[8] & 0x3F) | 0x80
    return bytes(raw)


def v3(namespace: bytes, name: str) -> bytes:
    """MD5 name-based UUID (deterministic)."""
    digest = hashlib.md5(namespace + name.encode("utf-8")).digest()
    return stamp(bytearray(digest), 3)


def v5(namespace: bytes, name: str) -> bytes:
    """SHA-1 name-based UUID (deterministic, first 16 of 20 digest bytes)."""
    digest = hashlib.sha1(namespace + name.encode("utf-8")).digest()[:16]
    return stamp(bytearray(digest), 5)
