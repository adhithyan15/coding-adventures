"""
sqlite_schema — the SQLite catalog table (phase 6).

What is sqlite_schema?
----------------------

Every SQLite database has a built-in catalog table called ``sqlite_schema``
(formerly ``sqlite_master``). It is itself a normal table B-tree — no magic
except for its fixed location: it lives on **page 1**, with its B-tree page
header at byte offset 100 (bytes 0–99 of page 1 are the 100-byte database
header).

The table has exactly five columns::

    type     TEXT     -- 'table' | 'index' | 'view' | 'trigger'
    name     TEXT     -- the object's name (e.g. 'users')
    tbl_name TEXT     -- the owning table (same as name for 'table' rows)
    rootpage INTEGER  -- page number of the B-tree root for this object
    sql      TEXT     -- the CREATE statement as typed (for round-trip)

v1 supports only ``type = 'table'`` rows. Index, view, and trigger rows are
read back correctly (so a file written by the real ``sqlite3`` CLI that
happens to contain those is not corrupted) but cannot be created or dropped
through this module.

Schema cookie
-------------

The database header (page 1, bytes 0–99) stores a **schema cookie** at
offset 40 — a u32 that is incremented every time the schema changes (CREATE
or DROP TABLE). Any connection that caches schema information must re-read it
when the cookie changes. We increment the cookie on every DDL operation.

Lifecycle of CREATE TABLE
--------------------------

1. Verify the name is not already in use.
2. Create a fresh empty B-tree for the new table (``BTree.create``), which
   allocates a root page via the freelist or by extending the file.
3. Determine the next rowid for the ``sqlite_schema`` row (``max + 1``).
4. Encode the 5-column record and insert it into the page-1 B-tree.
5. Bump the schema cookie.

Lifecycle of DROP TABLE
------------------------

1. Find the ``sqlite_schema`` row for the table (by scanning + name match).
2. Call ``BTree.free_all(freelist)`` on the dropped table's B-tree to return
   every page — interior pages, leaf pages, overflow pages — to the freelist.
3. Delete the ``sqlite_schema`` row.
4. Bump the schema cookie.

Database initialisation
-----------------------

A brand-new database file has a single page (page 1) whose first 100 bytes
are the ``Header`` and whose remaining bytes (100–4095) form an empty leaf
page for the ``sqlite_schema`` tree. Use :func:`initialize_new_database` to
set up this structure, then create a :class:`Schema` to operate on it::

    with Pager.create("app.db") as pager:
        fl = Freelist(pager)
        schema = initialize_new_database(pager)
        root = schema.create_table("users", "CREATE TABLE users (id INTEGER, name TEXT)")
        pager.commit()

v1 limitations
--------------

* Only ``type = 'table'`` rows can be created or dropped. Other types are
  preserved on read but not actionable.
* ``AUTOINCREMENT`` (``sqlite_sequence``) is not maintained.
* The database header fields ``file_change_counter``, ``database_size_pages``,
  and ``version_valid_for`` are NOT updated here — those belong to the Backend
  adapter (phase 7) which wraps each user transaction.
"""

from __future__ import annotations

import struct
from typing import TYPE_CHECKING

from storage_sqlite import record
from storage_sqlite.btree import BTree
from storage_sqlite.errors import StorageError
from storage_sqlite.header import Header
from storage_sqlite.pager import PAGE_SIZE, Pager

if TYPE_CHECKING:
    from storage_sqlite.freelist import Freelist

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

_SCHEMA_ROOT_PAGE: int = 1
"""sqlite_schema is always rooted at page 1."""

_SCHEMA_HDR_OFFSET: int = 100
"""The B-tree page header for sqlite_schema starts at byte 100 of page 1,
right after the 100-byte database file header."""

_SCHEMA_COOKIE_OFFSET: int = 40
"""Byte offset of the schema cookie (u32 BE) inside the 100-byte database
header, which lives at the start of page 1."""


# ---------------------------------------------------------------------------
# Exceptions
# ---------------------------------------------------------------------------


class SchemaError(StorageError):
    """A schema-level error: duplicate table name, unknown table, etc."""


# ---------------------------------------------------------------------------
# Module-level helper
# ---------------------------------------------------------------------------


def initialize_new_database(pager: Pager) -> Schema:
    """Set up a brand-new database and return a :class:`Schema` for it.

    *pager* must be freshly created (``size_pages == 0``). The function:

    1. Allocates page 1.
    2. Writes the 100-byte database header at bytes 0–99.
    3. Initialises an empty ``sqlite_schema`` leaf page at bytes 100–107.
    4. Returns a :class:`Schema` attached to the pager.

    The caller must call ``pager.commit()`` when ready to persist.

    Example::

        with Pager.create("app.db") as pager:
            schema = initialize_new_database(pager)
            schema.create_table("t", "CREATE TABLE t (x INTEGER)")
            pager.commit()
    """
    if pager.size_pages != 0:
        raise ValueError(
            f"pager already has {pager.size_pages} pages; "
            "initialize_new_database requires a fresh pager"
        )

    pager.allocate()  # claims page 1

    # Build the initial page 1: 100-byte header + empty sqlite_schema leaf.
    buf = bytearray(PAGE_SIZE)
    buf[:100] = Header.new_database().to_bytes()

    # Empty leaf page header at offset 100.
    # Format: page_type(1) freeblock(2) ncells(2) content_start(2) fragmented(1)
    # content_start = PAGE_SIZE when the content area is empty.
    buf[100] = 0x0D  # PAGE_TYPE_LEAF_TABLE
    struct.pack_into(">HHHB", buf, 101, 0, 0, PAGE_SIZE, 0)

    pager.write(1, bytes(buf))
    return Schema(pager)


# ---------------------------------------------------------------------------
# Schema class
# ---------------------------------------------------------------------------


class Schema:
    """Access layer for the ``sqlite_schema`` catalog table.

    Construct via :func:`initialize_new_database` (new database) or
    ``Schema(pager)`` (existing database). All mutations stage writes
    through the pager's dirty-page table; call ``pager.commit()`` to
    persist.

    Parameters
    ----------
    pager:
        The page-I/O layer for the database.
    freelist:
        Optional :class:`~storage_sqlite.freelist.Freelist`. When
        provided, new table root pages are taken from the freelist, and
        pages freed by :meth:`drop_table` are returned to it. When
        ``None``, new pages always extend the file.
    """

    __slots__ = ("_btree", "_freelist", "_pager")

    def __init__(self, pager: Pager, freelist: Freelist | None = None) -> None:
        self._pager: Pager = pager
        self._freelist: Freelist | None = freelist
        self._btree: BTree = BTree.open(
            pager,
            _SCHEMA_ROOT_PAGE,
            header_offset=_SCHEMA_HDR_OFFSET,
            freelist=freelist,
        )

    # ── Read operations ───────────────────────────────────────────────────────

    def list_tables(self) -> list[str]:
        """Return the names of all tables in the database.

        Scans ``sqlite_schema`` and returns every row where ``type = 'table'``
        in the order they were inserted (ascending rowid).

        Example::

            schema.create_table("users", "CREATE TABLE users (id INTEGER)")
            schema.create_table("posts", "CREATE TABLE posts (id INTEGER)")
            assert schema.list_tables() == ["users", "posts"]
        """
        result: list[str] = []
        for _, payload in self._btree.scan():
            cols, _ = record.decode(payload)
            if cols[0] == "table":
                result.append(str(cols[1]))
        return result

    def find_table(self, name: str) -> tuple[int, int, str] | None:
        """Return ``(rowid, rootpage, sql)`` for *name*, or ``None``.

        Performs a full scan of ``sqlite_schema``; the schema table is
        small enough that this is fine for v1.

        The returned ``rootpage`` is the 1-based page number of the table's
        B-tree root. ``sql`` is the CREATE TABLE statement as originally
        written.
        """
        for rowid, payload in self._btree.scan():
            cols, _ = record.decode(payload)
            if cols[0] == "table" and cols[1] == name:
                return rowid, int(cols[3]), str(cols[4])
        return None

    def rootpage_for(self, name: str) -> int | None:
        """Return the root page number for table *name*, or ``None``.

        Convenience wrapper around :meth:`find_table` when only the root
        page is needed (e.g. to open the table's B-tree for DML).
        """
        result = self.find_table(name)
        return result[1] if result is not None else None

    def get_schema_cookie(self) -> int:
        """Read the current schema cookie from the page-1 header.

        The cookie is a monotonically increasing u32 at byte offset 40
        of the database header. It is bumped on every CREATE or DROP TABLE.
        """
        page1 = self._pager.read(1)
        (cookie,) = struct.unpack_from(">I", page1, _SCHEMA_COOKIE_OFFSET)
        return cookie

    # ── Write operations ──────────────────────────────────────────────────────

    def create_table(self, name: str, sql: str) -> int:
        """Create a new table and return its root page number.

        Steps:

        1. Verify *name* is not already in ``sqlite_schema``.
        2. Allocate a fresh root page via :meth:`BTree.create` (uses
           freelist or extends the file).
        3. Compute the next rowid (``max_existing_rowid + 1``).
        4. Encode ``('table', name, name, rootpage, sql)`` and insert it
           into the page-1 B-tree.
        5. Bump the schema cookie.

        Returns the root page number allocated for the new table.

        Raises :class:`SchemaError` if a table with *name* already exists.

        Example::

            root = schema.create_table("users", "CREATE TABLE users (id INTEGER)")
            tree = BTree.open(pager, root)
        """
        if self.find_table(name) is not None:
            raise SchemaError(f"table {name!r} already exists")

        # Allocate and initialise a fresh root page for the new table.
        new_tree = BTree.create(self._pager, freelist=self._freelist)
        root_pgno = new_tree.root_page

        # Determine the rowid for this new schema row.
        rowid = self._next_rowid()

        # Encode and insert the 5-column schema record.
        payload = record.encode(["table", name, name, root_pgno, sql])
        self._btree.insert(rowid, payload)

        # Schema changed — bump the cookie.
        self._bump_schema_cookie()

        return root_pgno

    def drop_table(self, name: str) -> None:
        """Drop a table: free its pages, delete the schema row, bump cookie.

        Steps:

        1. Find the ``sqlite_schema`` row for *name*.
        2. Free every page in the table's B-tree (via
           :meth:`BTree.free_all`): interior pages, leaf pages, and all
           overflow chains.
        3. Delete the ``sqlite_schema`` row.
        4. Bump the schema cookie.

        Raises :class:`SchemaError` if *name* does not exist in the schema.

        Note: if no freelist was injected, the table's pages are zeroed but
        not returned to any freelist (they stay allocated). Pass a
        :class:`~storage_sqlite.freelist.Freelist` for proper page reuse.
        """
        result = self.find_table(name)
        if result is None:
            raise SchemaError(f"table {name!r} does not exist")

        row_rowid, root_pgno, _ = result

        # Free all pages in the dropped table's B-tree.
        if self._freelist is not None:
            table_tree = BTree.open(self._pager, root_pgno, freelist=self._freelist)
            table_tree.free_all(self._freelist)
        else:
            # No freelist: at least zero the root page so it doesn't hold
            # stale data. A full traversal to zero every page would be
            # expensive with no benefit — callers should use a Freelist.
            self._pager.write(root_pgno, b"\x00" * PAGE_SIZE)

        # Remove the schema row.
        self._btree.delete(row_rowid)

        # Schema changed — bump the cookie.
        self._bump_schema_cookie()

    # ── Internal helpers ──────────────────────────────────────────────────────

    def _next_rowid(self) -> int:
        """Return the next rowid to use for a new ``sqlite_schema`` row.

        Scans the entire schema B-tree to find the current maximum rowid,
        then returns ``max + 1``. Returns 1 when the table is empty (no
        existing entries).

        SQLite's own rowid allocation for ``sqlite_schema`` follows the
        same ``max + 1`` pattern, so our files are byte-compatible with
        files produced by ``sqlite3`` for the same sequence of DDL
        statements.
        """
        max_rowid: int = 0
        for rowid, _ in self._btree.scan():
            if rowid > max_rowid:
                max_rowid = rowid
        return max_rowid + 1

    def _bump_schema_cookie(self) -> None:
        """Increment the schema cookie in the page-1 header.

        Reads the current cookie at offset 40, increments it, and writes
        the modified page-1 bytes back through the pager. Only the 4 bytes
        at offset 40–43 are changed; all other bytes (database header
        fields, B-tree data) are preserved.
        """
        buf = bytearray(self._pager.read(1))
        (cookie,) = struct.unpack_from(">I", buf, _SCHEMA_COOKIE_OFFSET)
        struct.pack_into(">I", buf, _SCHEMA_COOKIE_OFFSET, (cookie + 1) & 0xFFFFFFFF)
        self._pager.write(1, bytes(buf))
