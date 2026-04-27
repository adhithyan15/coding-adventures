"""
The 100-byte SQLite database header.

Every SQLite database file begins with a 100-byte header at byte offset 0.
The header lives on page 1 — after it, the rest of page 1 (bytes 100 through
``page_size - 1``) is the beginning of the ``sqlite_schema`` B-tree. That
overlap makes page 1 special: every *other* page is a full ``page_size``
bytes of one thing (B-tree page, overflow page, freelist page, etc.), but
page 1's first 100 bytes are this fixed metadata block.

We model the header as a frozen dataclass with one field per header slot.
``from_bytes`` parses, ``to_bytes`` serialises, and ``new_database``
constructs a fresh header for writing a new file.

Byte layout (all multi-byte integers big-endian):

::

    offset  size  field
       0    16    magic                  b"SQLite format 3\\0"
      16     2    page_size              u16 (4096 in v1)
      18     1    file_format_write_version   1 (legacy) / 2 (WAL)
      19     1    file_format_read_version    1 (legacy) / 2 (WAL)
      20     1    reserved_per_page           0
      21     1    max_embedded_payload_fraction  64
      22     1    min_embedded_payload_fraction  32
      23     1    leaf_payload_fraction          32
      24     4    file_change_counter    u32 (bumped on every write txn)
      28     4    database_size_pages    u32 (size in pages per header)
      32     4    first_freelist_trunk   u32 (0 if none)
      36     4    freelist_page_count    u32
      40     4    schema_cookie          u32 (bumped when sqlite_schema changes)
      44     4    schema_format          u32 (4 — supports everything we need)
      48     4    default_cache_size     u32
      52     4    largest_root_btree     u32 (incremental vacuum — 0 in v1)
      56     4    text_encoding          u32 (1 = UTF-8)
      60     4    user_version           u32
      64     4    incremental_vacuum     u32
      68     4    application_id         u32
      72    20    reserved               20 zero bytes
      92     4    version_valid_for      u32 (== file_change_counter)
      96     4    sqlite_version_number  u32 (arbitrary; we pick 3046002)

Notes:

- ``max_embedded_payload_fraction`` and ``min_embedded_payload_fraction``
  have fixed values in every SQLite database ever written (64 and 32). We
  accept only those values on parse. Same for ``leaf_payload_fraction``.
- Page size 1 is a legal SQLite encoding that *means* 65536 (the format
  could not fit 65536 in a u16). v1 only uses 4096, but we accept the
  encoded form for forward-compat on parse — storing ``page_size = 65536``
  after decoding.

References: `SQLite file format §1.3 <https://www.sqlite.org/fileformat2.html>`_.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass, field
from typing import ClassVar

from storage_sqlite.errors import CorruptDatabaseError

# The first 16 bytes of every SQLite database: ASCII "SQLite format 3" plus
# a trailing NUL. If a file starts with anything else, it's not ours.
MAGIC: bytes = b"SQLite format 3\x00"

# The header is *exactly* 100 bytes, always. The format has no variable-
# length fields in this region.
HEADER_SIZE: int = 100

# Valid SQLite page sizes are powers of two from 512 to 32768 inclusive,
# plus the special value 65536 (encoded as 1 in the u16 slot). v1 pins on
# 4096 — the default and by far the most common size.
_VALID_PAGE_SIZES: frozenset[int] = frozenset({512, 1024, 2048, 4096, 8192, 16384, 32768, 65536})

# Text encodings — only UTF-8 is supported in v1. Reading a file that
# declares UTF-16 (2 or 3) is a "not yet supported" error, not corruption.
_UTF8: int = 1

# ``struct`` format string that packs/unpacks every fixed-size field from
# offset 16 onward. The 16-byte magic and the 20-byte reserved block are
# handled separately because ``struct`` doesn't have a clean way to express
# "these bytes are fixed literals and we validate them".
#
#   >   big-endian
#   H   page_size                    (u16)
#   B   file_format_write_version    (u8)
#   B   file_format_read_version     (u8)
#   B   reserved_per_page            (u8)
#   B   max_embedded_payload_fraction(u8)
#   B   min_embedded_payload_fraction(u8)
#   B   leaf_payload_fraction        (u8)
#   I   file_change_counter          (u32)
#   I   database_size_pages          (u32)
#   I   first_freelist_trunk         (u32)
#   I   freelist_page_count          (u32)
#   I   schema_cookie                (u32)
#   I   schema_format                (u32)
#   I   default_cache_size           (u32)
#   I   largest_root_btree           (u32)
#   I   text_encoding                (u32)
#   I   user_version                 (u32)
#   I   incremental_vacuum           (u32)
#   I   application_id               (u32)
_AFTER_MAGIC_FMT: str = ">HBBBBBBIIIIIIIIIIII"

# Fields at the tail: version_valid_for (u32) then sqlite_version_number
# (u32). Separated from the middle block by the 20-byte reserved zero
# region at offset 72.
_TAIL_FMT: str = ">II"


@dataclass(frozen=True, slots=True)
class Header:
    """The 100-byte SQLite database header.

    Use :meth:`new_database` to construct a fresh one, :meth:`from_bytes` to
    parse an existing file's first 100 bytes, and :meth:`to_bytes` to
    serialise.
    """

    magic: bytes = MAGIC
    page_size: int = 4096
    file_format_write_version: int = 1
    file_format_read_version: int = 1
    reserved_per_page: int = 0
    max_embedded_payload_fraction: int = 64
    min_embedded_payload_fraction: int = 32
    leaf_payload_fraction: int = 32
    file_change_counter: int = 0
    database_size_pages: int = 1
    first_freelist_trunk: int = 0
    freelist_page_count: int = 0
    schema_cookie: int = 0
    schema_format: int = 4
    default_cache_size: int = 0
    largest_root_btree: int = 0
    text_encoding: int = _UTF8
    user_version: int = 0
    incremental_vacuum: int = 0
    application_id: int = 0
    reserved: bytes = field(default=b"\x00" * 20)
    version_valid_for: int = 0
    # A fixed SQLite version tag so tools like ``sqlite3 .dbinfo`` show
    # something plausible. The value is arbitrary from our perspective —
    # SQLite ignores it on read. 3.46.2 → 3 046 002.
    sqlite_version_number: int = 3_046_002

    SIZE: ClassVar[int] = HEADER_SIZE

    # ------------------------------------------------------------------
    # Construction.
    # ------------------------------------------------------------------

    @classmethod
    def new_database(cls, *, page_size: int = 4096) -> Header:
        """Return a brand-new header for an empty database.

        ``database_size_pages`` defaults to 1 because a freshly-created file
        has exactly one page — page 1, which contains just this header.
        """
        if page_size not in _VALID_PAGE_SIZES:
            raise ValueError(
                f"page_size must be one of {sorted(_VALID_PAGE_SIZES)}, got {page_size}"
            )
        return cls(page_size=page_size)

    # ------------------------------------------------------------------
    # Parsing.
    # ------------------------------------------------------------------

    @classmethod
    def from_bytes(cls, data: bytes) -> Header:
        """Parse 100 bytes into a :class:`Header`.

        Raises :class:`CorruptDatabaseError` if the magic string is wrong,
        the page size is invalid, or a field has an impossible value.
        """
        if len(data) < HEADER_SIZE:
            raise CorruptDatabaseError(
                f"header must be at least {HEADER_SIZE} bytes, got {len(data)}"
            )

        magic = bytes(data[:16])
        if magic != MAGIC:
            raise CorruptDatabaseError(f"not a SQLite database: bad magic {magic!r}")

        # Unpack every fixed-size field from offset 16..72 in one go.
        (
            encoded_page_size,
            write_v,
            read_v,
            reserved_pp,
            max_payload,
            min_payload,
            leaf_payload,
            change_counter,
            db_size_pages,
            first_freelist,
            freelist_count,
            schema_cookie,
            schema_format,
            default_cache,
            largest_root,
            text_encoding,
            user_version,
            incremental_vacuum,
            application_id,
        ) = struct.unpack_from(_AFTER_MAGIC_FMT, data, 16)

        # The u16 at offset 16 encodes 65536 as 1 — that's the only page
        # size that doesn't fit in 16 bits.
        page_size = 65536 if encoded_page_size == 1 else encoded_page_size
        if page_size not in _VALID_PAGE_SIZES:
            raise CorruptDatabaseError(f"invalid page_size {page_size}")

        # These three are *always* 64/32/32 in valid SQLite databases. If
        # they differ, the file was produced by something broken or
        # intentionally malformed — refuse rather than guess.
        if (max_payload, min_payload, leaf_payload) != (64, 32, 32):
            raise CorruptDatabaseError(
                "invalid payload fractions: expected (64, 32, 32), got "
                f"({max_payload}, {min_payload}, {leaf_payload})"
            )

        if text_encoding != _UTF8:
            raise CorruptDatabaseError(
                f"unsupported text encoding {text_encoding}: v1 supports only UTF-8 (1)"
            )

        # The 20-byte reserved block at offset 72 must be all zeros per spec.
        reserved = bytes(data[72:92])
        if reserved != b"\x00" * 20:
            raise CorruptDatabaseError("reserved bytes at offset 72..91 are non-zero")

        version_valid_for, sqlite_version_number = struct.unpack_from(_TAIL_FMT, data, 92)

        return cls(
            magic=magic,
            page_size=page_size,
            file_format_write_version=write_v,
            file_format_read_version=read_v,
            reserved_per_page=reserved_pp,
            max_embedded_payload_fraction=max_payload,
            min_embedded_payload_fraction=min_payload,
            leaf_payload_fraction=leaf_payload,
            file_change_counter=change_counter,
            database_size_pages=db_size_pages,
            first_freelist_trunk=first_freelist,
            freelist_page_count=freelist_count,
            schema_cookie=schema_cookie,
            schema_format=schema_format,
            default_cache_size=default_cache,
            largest_root_btree=largest_root,
            text_encoding=text_encoding,
            user_version=user_version,
            incremental_vacuum=incremental_vacuum,
            application_id=application_id,
            reserved=reserved,
            version_valid_for=version_valid_for,
            sqlite_version_number=sqlite_version_number,
        )

    # ------------------------------------------------------------------
    # Serialisation.
    # ------------------------------------------------------------------

    def to_bytes(self) -> bytes:
        """Serialise to exactly 100 bytes, ready to splice into page 1."""
        # 65536 is encoded as 1 because it doesn't fit in the u16 slot.
        encoded_page_size = 1 if self.page_size == 65536 else self.page_size

        buf = bytearray(HEADER_SIZE)
        buf[:16] = self.magic
        struct.pack_into(
            _AFTER_MAGIC_FMT,
            buf,
            16,
            encoded_page_size,
            self.file_format_write_version,
            self.file_format_read_version,
            self.reserved_per_page,
            self.max_embedded_payload_fraction,
            self.min_embedded_payload_fraction,
            self.leaf_payload_fraction,
            self.file_change_counter,
            self.database_size_pages,
            self.first_freelist_trunk,
            self.freelist_page_count,
            self.schema_cookie,
            self.schema_format,
            self.default_cache_size,
            self.largest_root_btree,
            self.text_encoding,
            self.user_version,
            self.incremental_vacuum,
            self.application_id,
        )
        buf[72:92] = self.reserved
        struct.pack_into(_TAIL_FMT, buf, 92, self.version_valid_for, self.sqlite_version_number)
        return bytes(buf)
