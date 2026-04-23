"""Tests for the 100-byte SQLite database header."""

from __future__ import annotations

import struct

import pytest

from storage_sqlite.errors import CorruptDatabaseError
from storage_sqlite.header import HEADER_SIZE, MAGIC, Header


def test_new_database_defaults() -> None:
    h = Header.new_database()
    assert h.magic == MAGIC
    assert h.page_size == 4096
    assert h.text_encoding == 1
    assert h.schema_format == 4
    assert h.database_size_pages == 1
    assert h.max_embedded_payload_fraction == 64
    assert h.min_embedded_payload_fraction == 32
    assert h.leaf_payload_fraction == 32


@pytest.mark.parametrize("ps", [512, 1024, 2048, 4096, 8192, 16384, 32768, 65536])
def test_new_database_accepts_all_valid_page_sizes(ps: int) -> None:
    h = Header.new_database(page_size=ps)
    assert h.page_size == ps


def test_new_database_rejects_invalid_page_size() -> None:
    with pytest.raises(ValueError, match="page_size must be one of"):
        Header.new_database(page_size=3000)


def test_to_bytes_is_exactly_100_bytes() -> None:
    h = Header.new_database()
    assert len(h.to_bytes()) == HEADER_SIZE


def test_round_trip_default() -> None:
    h = Header.new_database()
    reparsed = Header.from_bytes(h.to_bytes())
    assert reparsed == h


def test_round_trip_with_all_fields_set() -> None:
    h = Header(
        page_size=8192,
        file_format_write_version=1,
        file_format_read_version=1,
        reserved_per_page=0,
        file_change_counter=42,
        database_size_pages=17,
        first_freelist_trunk=5,
        freelist_page_count=3,
        schema_cookie=99,
        schema_format=4,
        default_cache_size=2000,
        largest_root_btree=0,
        text_encoding=1,
        user_version=7,
        incremental_vacuum=0,
        application_id=0x12345678,
        version_valid_for=42,
        sqlite_version_number=3_046_002,
    )
    assert Header.from_bytes(h.to_bytes()) == h


def test_round_trip_page_size_65536() -> None:
    h = Header.new_database(page_size=65536)
    raw = h.to_bytes()
    # 65536 encodes as 1 in the u16 slot.
    assert struct.unpack_from(">H", raw, 16)[0] == 1
    assert Header.from_bytes(raw).page_size == 65536


def test_from_bytes_rejects_short_input() -> None:
    with pytest.raises(CorruptDatabaseError, match="at least"):
        Header.from_bytes(b"\x00" * 50)


def test_from_bytes_rejects_bad_magic() -> None:
    raw = bytearray(Header.new_database().to_bytes())
    raw[:16] = b"NotSQLite\x00\x00\x00\x00\x00\x00\x00"
    with pytest.raises(CorruptDatabaseError, match="bad magic"):
        Header.from_bytes(bytes(raw))


def test_from_bytes_rejects_invalid_page_size() -> None:
    raw = bytearray(Header.new_database().to_bytes())
    # 3000 isn't in the valid set.
    struct.pack_into(">H", raw, 16, 3000)
    with pytest.raises(CorruptDatabaseError, match="invalid page_size"):
        Header.from_bytes(bytes(raw))


def test_from_bytes_rejects_bad_payload_fractions() -> None:
    raw = bytearray(Header.new_database().to_bytes())
    raw[21] = 60  # max_embedded_payload_fraction should be 64
    with pytest.raises(CorruptDatabaseError, match="payload fractions"):
        Header.from_bytes(bytes(raw))


def test_from_bytes_rejects_non_utf8_encoding() -> None:
    raw = bytearray(Header.new_database().to_bytes())
    struct.pack_into(">I", raw, 56, 2)  # UTF-16le
    with pytest.raises(CorruptDatabaseError, match="text encoding"):
        Header.from_bytes(bytes(raw))


def test_from_bytes_rejects_nonzero_reserved() -> None:
    raw = bytearray(Header.new_database().to_bytes())
    raw[80] = 0x01
    with pytest.raises(CorruptDatabaseError, match="reserved bytes"):
        Header.from_bytes(bytes(raw))


def test_from_bytes_accepts_encoded_65536() -> None:
    # Build a valid header and flip page_size to the encoded 65536 form.
    raw = bytearray(Header.new_database(page_size=65536).to_bytes())
    h = Header.from_bytes(bytes(raw))
    assert h.page_size == 65536
