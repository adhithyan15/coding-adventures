"""Tests for the UUID library.

Covers all five versions (v1, v3, v4, v5, v7), parsing in all accepted formats,
validation, comparison, and RFC-specified test vectors.
"""

from __future__ import annotations

import re
import time

import pytest

from ca_uuid import (
    MAX,
    NAMESPACE_DNS,
    NAMESPACE_OID,
    NAMESPACE_URL,
    NAMESPACE_X500,
    NIL,
    UUID,
    UUIDError,
    is_valid,
    parse,
    v1,
    v3,
    v4,
    v5,
    v7,
)


# ─── UUID Construction ────────────────────────────────────────────────────────


class TestUUIDConstruction:
    def test_from_string(self) -> None:
        u = UUID("550e8400-e29b-41d4-a716-446655440000")
        assert str(u) == "550e8400-e29b-41d4-a716-446655440000"

    def test_from_bytes(self) -> None:
        b = bytes.fromhex("550e8400e29b41d4a716446655440000")
        u = UUID(b)
        assert u.bytes == b

    def test_from_int(self) -> None:
        i = 0x550E8400E29B41D4A716446655440000
        u = UUID(i)
        assert u.int == i

    def test_bad_bytes_length(self) -> None:
        with pytest.raises(UUIDError):
            UUID(b"\x00" * 15)

    def test_bad_int_range(self) -> None:
        with pytest.raises(UUIDError):
            UUID(-1)
        with pytest.raises(UUIDError):
            UUID(1 << 128)

    def test_bad_type(self) -> None:
        with pytest.raises(UUIDError):
            UUID(3.14)  # type: ignore


# ─── UUID Properties ──────────────────────────────────────────────────────────


class TestUUIDProperties:
    def test_version_v4(self) -> None:
        u = UUID("550e8400-e29b-41d4-a716-446655440000")
        assert u.version == 4

    def test_version_v1(self) -> None:
        u = UUID("6ba7b810-9dad-11d1-80b4-00c04fd430c8")
        assert u.version == 1

    def test_version_v5(self) -> None:
        u = UUID("886313e1-3b8a-5372-9b90-0c9aee199e5d")
        assert u.version == 5

    def test_variant_rfc4122(self) -> None:
        u = UUID("550e8400-e29b-41d4-a716-446655440000")
        assert u.variant == "rfc4122"

    def test_variant_rfc4122_all_values(self) -> None:
        # Variant byte 8-9 can be 8,9,a,b (10xx in binary)
        for nibble in ["8", "9", "a", "b"]:
            u = UUID(f"550e8400-e29b-41d4-{nibble}716-446655440000")
            assert u.variant == "rfc4122"

    def test_is_nil(self) -> None:
        assert NIL.is_nil
        assert not v4().is_nil

    def test_is_max(self) -> None:
        assert MAX.is_max
        assert not v4().is_max

    def test_bytes_roundtrip(self) -> None:
        u = UUID("550e8400-e29b-41d4-a716-446655440000")
        assert UUID(u.bytes) == u

    def test_int_roundtrip(self) -> None:
        u = UUID("550e8400-e29b-41d4-a716-446655440000")
        assert UUID(u.int) == u


# ─── String Formatting ────────────────────────────────────────────────────────


class TestFormatting:
    def test_str_format(self) -> None:
        u = UUID("550e8400-e29b-41d4-a716-446655440000")
        s = str(u)
        assert re.match(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$", s)

    def test_str_lowercase(self) -> None:
        u = UUID("550E8400-E29B-41D4-A716-446655440000")
        assert str(u) == "550e8400-e29b-41d4-a716-446655440000"

    def test_repr(self) -> None:
        u = UUID("550e8400-e29b-41d4-a716-446655440000")
        assert "550e8400-e29b-41d4-a716-446655440000" in repr(u)


# ─── Parsing ──────────────────────────────────────────────────────────────────


class TestParsing:
    def test_standard_format(self) -> None:
        u = parse("550e8400-e29b-41d4-a716-446655440000")
        assert u.version == 4

    def test_uppercase(self) -> None:
        u = parse("550E8400-E29B-41D4-A716-446655440000")
        assert str(u) == "550e8400-e29b-41d4-a716-446655440000"

    def test_compact_no_hyphens(self) -> None:
        u = parse("550e8400e29b41d4a716446655440000")
        assert str(u) == "550e8400-e29b-41d4-a716-446655440000"

    def test_braces(self) -> None:
        u = parse("{550e8400-e29b-41d4-a716-446655440000}")
        assert str(u) == "550e8400-e29b-41d4-a716-446655440000"

    def test_urn_form(self) -> None:
        u = parse("urn:uuid:550e8400-e29b-41d4-a716-446655440000")
        assert str(u) == "550e8400-e29b-41d4-a716-446655440000"

    def test_leading_trailing_whitespace(self) -> None:
        u = parse("  550e8400-e29b-41d4-a716-446655440000  ")
        assert str(u) == "550e8400-e29b-41d4-a716-446655440000"

    def test_invalid_raises(self) -> None:
        with pytest.raises(UUIDError):
            parse("not-a-uuid")

    def test_too_short_raises(self) -> None:
        with pytest.raises(UUIDError):
            parse("550e8400-e29b-41d4-a716")

    def test_nil_parses(self) -> None:
        u = parse("00000000-0000-0000-0000-000000000000")
        assert u.is_nil

    def test_max_parses(self) -> None:
        u = parse("ffffffff-ffff-ffff-ffff-ffffffffffff")
        assert u.is_max


# ─── Validation ───────────────────────────────────────────────────────────────


class TestValidation:
    def test_valid_standard(self) -> None:
        assert is_valid("550e8400-e29b-41d4-a716-446655440000")

    def test_valid_uppercase(self) -> None:
        assert is_valid("550E8400-E29B-41D4-A716-446655440000")

    def test_valid_compact(self) -> None:
        assert is_valid("550e8400e29b41d4a716446655440000")

    def test_valid_braces(self) -> None:
        assert is_valid("{550e8400-e29b-41d4-a716-446655440000}")

    def test_valid_urn(self) -> None:
        assert is_valid("urn:uuid:550e8400-e29b-41d4-a716-446655440000")

    def test_invalid_too_short(self) -> None:
        assert not is_valid("550e8400-e29b")

    def test_invalid_not_hex(self) -> None:
        assert not is_valid("gggggggg-gggg-gggg-gggg-gggggggggggg")

    def test_invalid_random_string(self) -> None:
        assert not is_valid("not-a-uuid-at-all")

    def test_nil_is_valid(self) -> None:
        assert is_valid("00000000-0000-0000-0000-000000000000")

    def test_max_is_valid(self) -> None:
        assert is_valid("ffffffff-ffff-ffff-ffff-ffffffffffff")


# ─── Comparison ───────────────────────────────────────────────────────────────


class TestComparison:
    def test_equality(self) -> None:
        u1 = UUID("550e8400-e29b-41d4-a716-446655440000")
        u2 = UUID("550e8400-e29b-41d4-a716-446655440000")
        assert u1 == u2

    def test_inequality(self) -> None:
        u1 = UUID("550e8400-e29b-41d4-a716-446655440000")
        u2 = NIL
        assert u1 != u2

    def test_nil_less_than_max(self) -> None:
        assert NIL < MAX

    def test_ordering(self) -> None:
        u1 = UUID("00000000-0000-0000-0000-000000000001")
        u2 = UUID("00000000-0000-0000-0000-000000000002")
        assert u1 < u2
        assert u2 > u1
        assert u1 <= u2
        assert u2 >= u1

    def test_hashable(self) -> None:
        s = {NIL, MAX, NIL}
        assert len(s) == 2


# ─── Namespace Constants ──────────────────────────────────────────────────────


class TestNamespaces:
    def test_dns_namespace(self) -> None:
        assert str(NAMESPACE_DNS) == "6ba7b810-9dad-11d1-80b4-00c04fd430c8"

    def test_url_namespace(self) -> None:
        assert str(NAMESPACE_URL) == "6ba7b811-9dad-11d1-80b4-00c04fd430c8"

    def test_oid_namespace(self) -> None:
        assert str(NAMESPACE_OID) == "6ba7b812-9dad-11d1-80b4-00c04fd430c8"

    def test_x500_namespace(self) -> None:
        assert str(NAMESPACE_X500) == "6ba7b814-9dad-11d1-80b4-00c04fd430c8"

    def test_namespaces_are_v1(self) -> None:
        for ns in [NAMESPACE_DNS, NAMESPACE_URL, NAMESPACE_OID, NAMESPACE_X500]:
            assert ns.version == 1


# ─── UUID v4 ──────────────────────────────────────────────────────────────────


class TestV4:
    def test_version(self) -> None:
        assert v4().version == 4

    def test_variant(self) -> None:
        assert v4().variant == "rfc4122"

    def test_random_unique(self) -> None:
        uuids = {v4() for _ in range(100)}
        assert len(uuids) == 100

    def test_16_bytes(self) -> None:
        assert len(v4().bytes) == 16

    def test_string_format(self) -> None:
        s = str(v4())
        assert re.match(r"^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$", s)

    def test_not_nil(self) -> None:
        assert not v4().is_nil


# ─── UUID v5 ──────────────────────────────────────────────────────────────────


class TestV5:
    def test_rfc_vector_dns_python_org(self) -> None:
        """RFC-specified test vector."""
        assert str(v5(NAMESPACE_DNS, "python.org")) == "886313e1-3b8a-5372-9b90-0c9aee199e5d"

    def test_rfc_vector_url(self) -> None:
        assert str(v5(NAMESPACE_URL, "http://www.python.org/")) == "c2a8cbf8-d0f1-5ef4-9740-c3faec8ab1a0"

    def test_version(self) -> None:
        assert v5(NAMESPACE_DNS, "example.com").version == 5

    def test_variant(self) -> None:
        assert v5(NAMESPACE_DNS, "example.com").variant == "rfc4122"

    def test_deterministic(self) -> None:
        u1 = v5(NAMESPACE_DNS, "example.com")
        u2 = v5(NAMESPACE_DNS, "example.com")
        assert u1 == u2

    def test_different_names_differ(self) -> None:
        u1 = v5(NAMESPACE_DNS, "example.com")
        u2 = v5(NAMESPACE_DNS, "example.org")
        assert u1 != u2

    def test_different_namespaces_differ(self) -> None:
        u1 = v5(NAMESPACE_DNS, "example.com")
        u2 = v5(NAMESPACE_URL, "example.com")
        assert u1 != u2

    def test_utf8_name(self) -> None:
        u = v5(NAMESPACE_DNS, "example.com")
        assert u.version == 5

    def test_unicode_name(self) -> None:
        u = v5(NAMESPACE_URL, "https://例え.jp/")
        assert u.version == 5
        assert u.variant == "rfc4122"

    def test_empty_name(self) -> None:
        u = v5(NAMESPACE_DNS, "")
        assert u.version == 5


# ─── UUID v3 ──────────────────────────────────────────────────────────────────


class TestV3:
    def test_rfc_vector_dns_python_org(self) -> None:
        """RFC-specified test vector."""
        assert str(v3(NAMESPACE_DNS, "python.org")) == "6fa459ea-ee8a-3ca4-894e-db77e160355e"

    def test_version(self) -> None:
        assert v3(NAMESPACE_DNS, "example.com").version == 3

    def test_variant(self) -> None:
        assert v3(NAMESPACE_DNS, "example.com").variant == "rfc4122"

    def test_deterministic(self) -> None:
        u1 = v3(NAMESPACE_DNS, "example.com")
        u2 = v3(NAMESPACE_DNS, "example.com")
        assert u1 == u2

    def test_different_names_differ(self) -> None:
        u1 = v3(NAMESPACE_DNS, "example.com")
        u2 = v3(NAMESPACE_DNS, "example.org")
        assert u1 != u2

    def test_v3_and_v5_differ(self) -> None:
        """v3 (MD5) and v5 (SHA-1) of the same input should differ."""
        u3 = v3(NAMESPACE_DNS, "python.org")
        u5 = v5(NAMESPACE_DNS, "python.org")
        assert u3 != u5


# ─── UUID v1 ──────────────────────────────────────────────────────────────────


class TestV1:
    def test_version(self) -> None:
        assert v1().version == 1

    def test_variant(self) -> None:
        assert v1().variant == "rfc4122"

    def test_unique(self) -> None:
        uuids = {v1() for _ in range(20)}
        assert len(uuids) == 20

    def test_16_bytes(self) -> None:
        assert len(v1().bytes) == 16

    def test_string_format(self) -> None:
        s = str(v1())
        assert re.match(r"^[0-9a-f]{8}-[0-9a-f]{4}-1[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$", s)

    def test_node_multicast_bit_set(self) -> None:
        """RFC 4122 §4.5: random node IDs must have the multicast bit set."""
        u = v1()
        # Node is the last 6 bytes; the first byte of the node must have bit 0 set
        node_first_byte = u.bytes[10]
        assert node_first_byte & 0x01, "multicast bit should be set for random node IDs"

    def test_time_ordered(self) -> None:
        """v1 UUIDs generated in sequence should have increasing timestamps."""
        u1 = v1()
        time.sleep(0.001)  # ensure time advances
        u2 = v1()
        # Extract the 60-bit timestamp from both
        def timestamp_of(u: UUID) -> int:
            b = u.bytes
            time_low = int.from_bytes(b[0:4], "big")
            time_mid = int.from_bytes(b[4:6], "big")
            time_hi  = int.from_bytes(b[6:8], "big") & 0x0FFF
            return (time_hi << 48) | (time_mid << 32) | time_low
        assert timestamp_of(u1) <= timestamp_of(u2)


# ─── UUID v7 ──────────────────────────────────────────────────────────────────


class TestV7:
    def test_version(self) -> None:
        assert v7().version == 7

    def test_variant(self) -> None:
        assert v7().variant == "rfc4122"

    def test_unique(self) -> None:
        uuids = {v7() for _ in range(100)}
        assert len(uuids) == 100

    def test_16_bytes(self) -> None:
        assert len(v7().bytes) == 16

    def test_string_format(self) -> None:
        s = str(v7())
        assert re.match(r"^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$", s)

    def test_time_ordered(self) -> None:
        """v7 UUIDs from different milliseconds must sort correctly."""
        u1 = v7()
        time.sleep(0.002)  # ensure different millisecond
        u2 = v7()
        assert u1 < u2, "v7 UUIDs should be lexicographically ordered by time"

    def test_timestamp_is_recent(self) -> None:
        """The 48-bit timestamp should be close to the current time."""
        before_ms = time.time_ns() // 1_000_000
        u = v7()
        after_ms = time.time_ns() // 1_000_000
        # Extract timestamp from bytes 0-5
        ts_ms = int.from_bytes(u.bytes[:6], "big")
        assert before_ms <= ts_ms <= after_ms + 1

    def test_many_in_sequence(self) -> None:
        """Multiple v7 UUIDs in one millisecond are still unique."""
        uuids = [v7() for _ in range(50)]
        assert len(set(uuids)) == 50


# ─── NIL and MAX ─────────────────────────────────────────────────────────────


class TestNilMax:
    def test_nil_str(self) -> None:
        assert str(NIL) == "00000000-0000-0000-0000-000000000000"

    def test_max_str(self) -> None:
        assert str(MAX) == "ffffffff-ffff-ffff-ffff-ffffffffffff"

    def test_nil_is_smallest(self) -> None:
        assert NIL < v4()

    def test_max_is_largest(self) -> None:
        assert MAX > v4()

    def test_nil_bytes_all_zero(self) -> None:
        assert NIL.bytes == b"\x00" * 16

    def test_max_bytes_all_ff(self) -> None:
        assert MAX.bytes == b"\xff" * 16
