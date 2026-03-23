"""uuid — UUID v1/v3/v4/v5/v7 generation and parsing (RFC 4122 + RFC 9562).

What Is a UUID?
===============
A Universally Unique Identifier (UUID) is a 128-bit label used to identify
information without requiring a central authority. Two systems can each generate
a UUID independently and be practically guaranteed not to collide.

UUIDs are formatted as 32 lowercase hex digits separated by hyphens into five
groups: 8-4-4-4-12:

  550e8400-e29b-41d4-a716-446655440000
  ^^^^^^^^ ^^^^ ^    ^^^^  ^^^^^^^^^^^^
  32 hex   16   ver  clk   48 hex (node)
  chars    hex  (M)  seq   chars

The letter M (position 13) is the version digit. The first hex digit of the
4th group (N, position 17) encodes the variant — always 8, 9, a, or b for
standard RFC 4122 UUIDs (because those are 1000, 1001, 1010, 1011 in binary,
and the top two bits are always "10" indicating RFC 4122 variant).

UUID Versions
=============
- v1: Time-based. Encodes the current time (100-ns intervals since 1582-10-15)
      and a MAC address or random node ID. Unique across space and time.
- v3: Name-based (MD5). MD5(namespace_bytes || name_utf8). Deterministic.
- v4: Random. 122 bits of cryptographically random data. Most common.
- v5: Name-based (SHA-1). SHA-1(namespace_bytes || name_utf8)[:16]. Deterministic.
- v7: Time-ordered random (RFC 9562). 48-bit millisecond timestamp in the
      high bits for database index locality, rest is random.

Internal Representation
=======================
We store UUIDs as 16 raw bytes in network byte order (big-endian). This is the
most portable representation and matches the wire/disk format. All other views
(string, version, variant) are derived from these 16 bytes.

Dependencies
============
- ca_sha1: our scratch SHA-1 package (for v5)
- ca_md5:  our scratch MD5 package (for v3)
"""

from __future__ import annotations

import os
import re
import struct
import time
from typing import Union

import ca_md5 as _md5_mod
import ca_sha1 as _sha1_mod

__version__ = "0.1.0"

# ─── UUID Error ───────────────────────────────────────────────────────────────

class UUIDError(ValueError):
    """Raised when a UUID string cannot be parsed or is invalid."""


# ─── UUID Class ───────────────────────────────────────────────────────────────

class UUID:
    """A 128-bit universally unique identifier.

    Internally stored as 16 bytes in network byte order (big-endian).

    Construction:
      UUID(bytes_val)          — from 16 raw bytes
      UUID("8-4-4-4-12 str")  — parsed from string
      UUID(int_val)            — from a 128-bit integer

    All other attributes are derived lazily from the raw bytes.
    """

    __slots__ = ("_bytes",)

    def __init__(self, value: Union[str, bytes, int]) -> None:
        if isinstance(value, bytes):
            if len(value) != 16:
                raise UUIDError(f"UUID bytes must be exactly 16, got {len(value)}")
            self._bytes = value
        elif isinstance(value, str):
            self._bytes = _parse_str(value)
        elif isinstance(value, int):
            if not (0 <= value < (1 << 128)):
                raise UUIDError("UUID int must be in range [0, 2^128)")
            self._bytes = value.to_bytes(16, "big")
        else:
            raise UUIDError(f"Cannot construct UUID from {type(value)}")

    # ── Derived properties ──────────────────────────────────────────────────

    @property
    def bytes(self) -> bytes:
        """The raw 16-byte representation in network byte order."""
        return self._bytes

    @property
    def int(self) -> int:
        """The UUID as a 128-bit integer."""
        return int.from_bytes(self._bytes, "big")

    @property
    def version(self) -> int:
        """The version field (bits 48-51). Returns 0 for NIL/MAX."""
        return (self._bytes[6] >> 4) & 0xF

    @property
    def variant(self) -> str:
        """The variant field (top bits of byte 8).

        Returns one of: "rfc4122", "microsoft", "ncs", "reserved".

        The variant tells which UUID specification applies:
          0xxx = NCS backward compatibility (legacy, rare)
          10xx = RFC 4122 (standard — what we always generate)
          110x = Microsoft GUID (legacy Windows)
          1111 = Reserved
        """
        top = (self._bytes[8] >> 6) & 0x3
        if top == 0b00 or top == 0b01:
            return "ncs"
        elif top == 0b10:
            return "rfc4122"
        elif top == 0b11:
            # Distinguish microsoft (110x) from reserved (1111)
            if (self._bytes[8] >> 5) & 0x1 == 0:
                return "microsoft"
            else:
                return "reserved"
        return "unknown"

    @property
    def is_nil(self) -> bool:
        """True if this is the nil UUID (all zeros)."""
        return self._bytes == b"\x00" * 16

    @property
    def is_max(self) -> bool:
        """True if this is the max UUID (all ones)."""
        return self._bytes == b"\xff" * 16

    # ── String representation ──────────────────────────────────────────────

    def __str__(self) -> str:
        """Return the standard 8-4-4-4-12 hyphenated hex string."""
        h = self._bytes.hex()
        return f"{h[:8]}-{h[8:12]}-{h[12:16]}-{h[16:20]}-{h[20:]}"

    def __repr__(self) -> str:
        return f"UUID('{self}')"

    # ── Comparison ────────────────────────────────────────────────────────

    def __eq__(self, other: object) -> bool:
        if isinstance(other, UUID):
            return self._bytes == other._bytes
        return NotImplemented

    def __lt__(self, other: "UUID") -> bool:
        return self._bytes < other._bytes

    def __le__(self, other: "UUID") -> bool:
        return self._bytes <= other._bytes

    def __gt__(self, other: "UUID") -> bool:
        return self._bytes > other._bytes

    def __ge__(self, other: "UUID") -> bool:
        return self._bytes >= other._bytes

    def __hash__(self) -> int:
        return hash(self._bytes)


# ─── Parsing ─────────────────────────────────────────────────────────────────
#
# A UUID string can appear in several forms. We handle all of them:
#
#   Standard (hyphenated):  "550e8400-e29b-41d4-a716-446655440000"
#   Uppercase:              "550E8400-E29B-41D4-A716-446655440000"
#   Compact (no hyphens):   "550e8400e29b41d4a716446655440000"
#   Braces (Windows GUID):  "{550e8400-e29b-41d4-a716-446655440000}"
#   URN form:               "urn:uuid:550e8400-e29b-41d4-a716-446655440000"
#
# All are case-insensitive. We strip to just the 32 hex digits and convert.
#
# NOTE: _HEX_RE and _parse_str are defined here — before the NAMESPACE_*
# constants below — because those constants call UUID("...") at import time,
# which triggers _parse_str.  Python executes module-level code top-to-bottom,
# so the function must be defined before it is called.

_HEX_RE = re.compile(
    r"^\s*(?:urn:uuid:)?\{?"
    r"([0-9a-f]{8})-?([0-9a-f]{4})-?([0-9a-f]{4})-?([0-9a-f]{4})-?([0-9a-f]{12})"
    r"\}?\s*$",
    re.IGNORECASE,
)


def _parse_str(text: str) -> bytes:
    m = _HEX_RE.match(text.strip())
    if not m:
        raise UUIDError(f"Invalid UUID string: {text!r}")
    hex32 = "".join(m.groups())
    return bytes.fromhex(hex32)


# ─── Namespace Constants (RFC 4122 §4.3) ─────────────────────────────────────
#
# These four well-known namespace UUIDs are defined in RFC 4122 Appendix C.
# They are fixed values — every UUID implementation in every language must use
# exactly these bytes to produce interoperable v3 and v5 UUIDs.
#
# When you compute v5(NAMESPACE_DNS, "python.org"), the result is the same in
# Python, Go, Ruby, TypeScript, Elixir, and Rust because we all start from the
# same namespace bytes.

NAMESPACE_DNS  = UUID("6ba7b810-9dad-11d1-80b4-00c04fd430c8")
NAMESPACE_URL  = UUID("6ba7b811-9dad-11d1-80b4-00c04fd430c8")
NAMESPACE_OID  = UUID("6ba7b812-9dad-11d1-80b4-00c04fd430c8")
NAMESPACE_X500 = UUID("6ba7b814-9dad-11d1-80b4-00c04fd430c8")

NIL = UUID("00000000-0000-0000-0000-000000000000")  # all-zero UUID
MAX = UUID("ffffffff-ffff-ffff-ffff-ffffffffffff")  # all-one UUID


def parse(text: str) -> UUID:
    """Parse a UUID from its string representation.

    Accepts standard (hyphenated), compact (no hyphens), braced, and URN forms.
    Case-insensitive.

    Raises UUIDError if the string is not a valid UUID.

    >>> parse("550e8400-e29b-41d4-a716-446655440000").version
    4
    """
    return UUID(_parse_str(text))


def is_valid(text: str) -> bool:
    """Return True if the string is a valid UUID representation.

    >>> is_valid("not-a-uuid")
    False
    >>> is_valid("550e8400-e29b-41d4-a716-446655440000")
    True
    """
    return bool(_HEX_RE.match(text.strip()))


# ─── Version and Variant Bit Manipulation ────────────────────────────────────
#
# Both v3/v5 (name-based) and v4/v7 (random/time-ordered) need to stamp two
# fields into the raw bytes:
#
#   Version nibble: byte 6, high 4 bits. Set to the version number.
#   Variant bits:   byte 8, high 2 bits. Set to 0b10 (binary 10xxxxxx).
#
# The operation is:
#   byte[6] = (byte[6] & 0x0F) | (version << 4)   — clear high nibble, set version
#   byte[8] = (byte[8] & 0x3F) | 0x80              — force top 2 bits to 10

def _set_version_variant(raw: bytearray, version: int) -> bytes:
    """Set the version nibble and RFC 4122 variant bits in-place, return bytes."""
    raw[6] = (raw[6] & 0x0F) | (version << 4)
    raw[8] = (raw[8] & 0x3F) | 0x80
    return bytes(raw)


# ─── UUID v4: Random ─────────────────────────────────────────────────────────
#
# The simplest version: 16 bytes from the OS CSPRNG, with two fields set.
#
# os.urandom(16) is the Python way to get cryptographically secure random bytes.
# It uses /dev/urandom on Linux/Mac and CryptGenRandom on Windows.

def v4() -> UUID:
    """Generate a UUID v4 (random).

    Uses os.urandom(16) for cryptographically secure randomness.
    Sets the version nibble to 4 and variant bits to RFC 4122.

    >>> u = v4()
    >>> u.version
    4
    >>> u.variant
    'rfc4122'
    """
    raw = bytearray(os.urandom(16))
    return UUID(_set_version_variant(raw, 4))


# ─── UUID v5: Name-Based (SHA-1) ─────────────────────────────────────────────
#
# Algorithm:
#   1. Concatenate namespace.bytes (16 bytes) + name encoded as UTF-8.
#   2. Compute SHA-1 of the concatenation → 20 bytes.
#   3. Take the first 16 bytes (SHA-1 output is 20 bytes; we discard the last 4).
#   4. Set version nibble to 5.
#   5. Set variant bits to 10.
#
# The result is always the same for the same (namespace, name) pair — v5 is
# deterministic. This makes it ideal for creating stable IDs from natural keys
# (e.g., a URL, a domain name, a database table name).
#
# We use our own sha1 package (not hashlib) to keep the dep graph educational.

def v5(namespace: UUID, name: str) -> UUID:
    """Generate a UUID v5 (name-based, SHA-1).

    Deterministic: same (namespace, name) always yields the same UUID.

    >>> v5(NAMESPACE_DNS, "python.org")
    UUID('886313e1-3b8a-5372-9b90-0c9aee199e5d')
    """
    data = namespace.bytes + name.encode("utf-8")
    digest = _sha1_mod.sha1(data)  # 20 bytes
    raw = bytearray(digest[:16])
    return UUID(_set_version_variant(raw, 5))


# ─── UUID v3: Name-Based (MD5) ───────────────────────────────────────────────
#
# Same concept as v5, but using MD5 instead of SHA-1. MD5 produces 16 bytes
# directly, so there's no truncation step.
#
# Use v3 only for compatibility with existing systems. For new code, prefer v5.

def v3(namespace: UUID, name: str) -> UUID:
    """Generate a UUID v3 (name-based, MD5).

    Deterministic: same (namespace, name) always yields the same UUID.
    Prefer v5() for new code.

    RFC test vector:
    >>> v3(NAMESPACE_DNS, "python.org")
    UUID('6fa459ea-ee8a-3ca4-894e-db77e160355e')
    """
    data = namespace.bytes + name.encode("utf-8")
    digest = _md5_mod.md5(data)  # 16 bytes
    raw = bytearray(digest)
    return UUID(_set_version_variant(raw, 3))


# ─── UUID v1: Time-Based ─────────────────────────────────────────────────────
#
# UUID v1 encodes the current time as a 60-bit count of 100-nanosecond intervals
# since the Gregorian epoch (1582-10-15 00:00:00 UTC).
#
# Why the Gregorian epoch? The RFC authors wanted a date before any computer
# system existed, to guarantee monotonically increasing timestamps.
#
# GREGORIAN_OFFSET: the number of 100-ns intervals between 1582-10-15 and
# 1970-01-01 (Unix epoch). This is a fixed constant:
#   122,192,928,000,000,000 intervals = 12219292800 seconds * 10,000,000 intervals/s

_GREGORIAN_OFFSET = 122192928000000000
_clock_seq = int.from_bytes(os.urandom(2), "big") & 0x3FFF  # random 14-bit start


def v1() -> UUID:
    """Generate a UUID v1 (time-based).

    Uses the current UTC time encoded as 100-ns intervals since 1582-10-15.
    Uses a random node ID (48 bits with the multicast bit set) because getting
    the actual MAC address is unreliable and a privacy risk in modern contexts.

    >>> u = v1()
    >>> u.version
    1
    """
    # time.time_ns() gives Unix time in nanoseconds.
    # Convert: ns → 100-ns intervals → add Gregorian offset.
    timestamp = time.time_ns() // 100 + _GREGORIAN_OFFSET

    # The 60-bit timestamp is split across three fields:
    #   time_low:     bits 0-31  (32 bits, the least significant part)
    #   time_mid:     bits 32-47 (16 bits)
    #   time_hi:      bits 48-59 (12 bits, most significant)
    time_low = timestamp & 0xFFFFFFFF
    time_mid = (timestamp >> 32) & 0xFFFF
    time_hi  = (timestamp >> 48) & 0x0FFF

    # version nibble is stamped into time_hi_and_version:
    #   time_hi_and_version = (1 << 12) | time_hi
    time_hi_and_version = 0x1000 | time_hi

    # Clock sequence: 14-bit random value to handle clock resets
    clock_seq_hi = 0x80 | (_clock_seq >> 8)   # top 2 bits set to 10 (variant)
    clock_seq_low = _clock_seq & 0xFF

    # Node: 48 random bits with multicast bit set (RFC 4122 §4.5)
    node_bytes = bytearray(os.urandom(6))
    node_bytes[0] |= 0x01  # set multicast bit to indicate random node

    raw = struct.pack(
        ">IHHBB6s",
        time_low,
        time_mid,
        time_hi_and_version,
        clock_seq_hi,
        clock_seq_low,
        bytes(node_bytes),
    )
    return UUID(raw)


# ─── UUID v7: Time-Ordered Random ────────────────────────────────────────────
#
# UUID v7 (RFC 9562) combines a millisecond timestamp in the high bits with
# random data in the low bits. This makes v7 UUIDs sortable by creation time,
# which dramatically improves database index locality compared to random v4 UUIDs.
#
# Bit layout:
#   Bits  0-47:  timestamp_ms (48-bit Unix timestamp in milliseconds)
#   Bits 48-51:  version = 7
#   Bits 52-63:  rand_a (12 random bits)
#   Bits 64-65:  variant = 10
#   Bits 66-127: rand_b (62 random bits)
#
# Why milliseconds? Microseconds would need 54 bits (overflows 48 bits after
# 2556 AD). Milliseconds fit in 48 bits until year 10889 AD.

def v7() -> UUID:
    """Generate a UUID v7 (time-ordered random, RFC 9562).

    The first 48 bits are the current Unix timestamp in milliseconds,
    ensuring lexicographic sort order matches creation order.

    >>> u1 = v7()
    >>> u2 = v7()
    >>> u1 <= u2  # usually True (same ms possible)
    True
    >>> u1.version
    7
    """
    ts_ms = time.time_ns() // 1_000_000  # milliseconds

    # 10 bytes of random data for rand_a (12 bits) + rand_b (62 bits)
    rand_bytes = bytearray(os.urandom(10))

    # Build the 128-bit UUID:
    #   byte 0-5:  timestamp_ms (48 bits, big-endian)
    #   byte 6:    version nibble (4) | rand_a high 4 bits
    #   byte 7:    rand_a low 8 bits
    #   byte 8:    variant (10xx) | rand_b high 6 bits
    #   byte 9-15: rand_b remaining 56 bits

    raw = bytearray(16)
    # Timestamp: 48 bits = 6 bytes, big-endian
    raw[0] = (ts_ms >> 40) & 0xFF
    raw[1] = (ts_ms >> 32) & 0xFF
    raw[2] = (ts_ms >> 24) & 0xFF
    raw[3] = (ts_ms >> 16) & 0xFF
    raw[4] = (ts_ms >> 8) & 0xFF
    raw[5] = ts_ms & 0xFF
    # Version nibble + rand_a (12 bits)
    raw[6] = 0x70 | (rand_bytes[0] & 0x0F)
    raw[7] = rand_bytes[1]
    # Variant (10xx) + rand_b (62 bits)
    raw[8] = 0x80 | (rand_bytes[2] & 0x3F)
    raw[9:16] = rand_bytes[3:10]

    return UUID(bytes(raw))
