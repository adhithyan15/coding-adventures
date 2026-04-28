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

v2 additions
-------------

* ``create_index(name, table, sql)`` — allocates a fresh index B-tree root
  page and inserts a ``type = 'index'`` row into ``sqlite_schema``.
* ``drop_index(name)`` — frees the index B-tree pages and deletes the schema row.
* ``find_index(name)`` — lookup by index name.
* ``list_indexes(table=None)`` — list all index rows, optionally filtered to
  a single table.

v1 limitations still apply
---------------------------

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
from storage_sqlite.pager import Pager

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
    ps = pager.page_size
    buf = bytearray(ps)
    buf[:100] = Header.new_database(page_size=ps).to_bytes()

    # Empty leaf page header at offset 100.
    # Format: page_type(1) freeblock(2) ncells(2) content_start(2) fragmented(1)
    # content_start = page_size when the content area is empty.
    buf[100] = 0x0D  # PAGE_TYPE_LEAF_TABLE
    struct.pack_into(">HHHB", buf, 101, 0, 0, ps, 0)

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
            self._pager.write(root_pgno, b"\x00" * self._pager.page_size)

        # Remove the schema row.
        self._btree.delete(row_rowid)

        # Schema changed — bump the cookie.
        self._bump_schema_cookie()

    # ── Index read operations ─────────────────────────────────────────────────

    def find_index(self, name: str) -> tuple[int, int, str | None] | None:
        """Return ``(rowid, rootpage, sql)`` for the index named *name*, or ``None``.

        Performs a full scan of ``sqlite_schema`` looking for entries with
        ``type = 'index'`` and ``name = name``.  Returns ``None`` when no
        such index exists.

        The ``sql`` element is the ``CREATE INDEX`` statement (or ``None``
        for auto-created indexes stored with a NULL sql field).

        Example::

            result = schema.find_index("auto_orders_user_id")
            if result is not None:
                row_rowid, root_pgno, sql = result
        """
        for rowid, payload in self._btree.scan():
            cols, _ = record.decode(payload)
            if cols[0] == "index" and cols[1] == name:
                sql_val = cols[4]
                return rowid, int(cols[3]), (str(sql_val) if sql_val is not None else None)
        return None

    def list_indexes(
        self, table: str | None = None
    ) -> list[tuple[str, str, int, str | None]]:
        """Return ``[(name, tbl_name, rootpage, sql)]`` for all indexes.

        Each tuple describes one ``type = 'index'`` row in ``sqlite_schema``.
        If *table* is provided, only indexes whose ``tbl_name`` matches are
        returned.  Results are in insertion (ascending rowid) order.

        The ``sql`` element is the ``CREATE INDEX`` statement or ``None`` for
        auto-created indexes stored with a NULL sql field.

        Example::

            rows = schema.list_indexes("orders")
            # [("auto_orders_user_id", "orders", 5, "CREATE INDEX ...")]
        """
        result: list[tuple[str, str, int, str | None]] = []
        for _, payload in self._btree.scan():
            cols, _ = record.decode(payload)
            if cols[0] != "index":
                continue
            tbl_name = str(cols[2])
            if table is not None and tbl_name != table:
                continue
            sql_val = cols[4]
            sql_str = str(sql_val) if sql_val is not None else None
            result.append((str(cols[1]), tbl_name, int(cols[3]), sql_str))
        return result

    # ── Index write operations ────────────────────────────────────────────────

    def create_index(self, name: str, table: str, sql: str | None) -> int:
        """Create a new index and return its root page number.

        Allocates a fresh empty index B-tree root page (type ``0x0A``
        leaf page) and inserts a ``type = 'index'`` row into
        ``sqlite_schema``.

        The stored row columns are::

            ('index', name, table, rootpage, sql)

        where *sql* is either the ``CREATE INDEX`` statement (for
        user-created indexes) or ``None`` (for auto-created indexes that
        follow the ``auto_{table}_{col}`` naming convention).

        Raises :class:`SchemaError` if a schema object with *name* already
        exists (either a table or an index).

        Returns the allocated root page number.

        Example::

            root = schema.create_index(
                "auto_orders_user_id", "orders",
                "CREATE INDEX auto_orders_user_id ON orders (user_id)"
            )
            # root is now the page number of the empty index B-tree leaf
        """
        # Guard against collisions with both tables and other indexes.
        if self.find_table(name) is not None or self.find_index(name) is not None:
            raise SchemaError(f"schema object {name!r} already exists")

        # Import here to avoid a circular-import at module load time —
        # schema.py and index_tree.py are siblings in the same package.
        from storage_sqlite.index_tree import IndexTree  # noqa: PLC0415

        new_tree = IndexTree.create(self._pager, freelist=self._freelist)
        root_pgno = new_tree.root_page

        rowid = self._next_rowid()
        payload = record.encode(["index", name, table, root_pgno, sql])
        self._btree.insert(rowid, payload)
        self._bump_schema_cookie()

        return root_pgno

    # ── Trigger read operations ───────────────────────────────────────────────

    def find_trigger(self, name: str) -> tuple[int, str, str | None] | None:
        """Return ``(rowid, tbl_name, sql)`` for the trigger named *name*, or ``None``.

        Performs a full scan of ``sqlite_schema`` looking for entries with
        ``type = 'trigger'`` and ``name = name``.  Returns ``None`` when no
        such trigger exists.

        The ``sql`` element is the full ``CREATE TRIGGER`` statement stored
        verbatim, or ``None`` for rows written without a SQL field.

        Example::

            result = schema.find_trigger("trg_orders_after_insert")
            if result is not None:
                row_rowid, tbl_name, sql = result
        """
        for rowid, payload in self._btree.scan():
            cols, _ = record.decode(payload)
            if cols[0] == "trigger" and cols[1] == name:
                sql_val = cols[4]
                return rowid, str(cols[2]), (str(sql_val) if sql_val is not None else None)
        return None

    def list_triggers(
        self, table: str | None = None
    ) -> list[tuple[str, str, str | None]]:
        """Return ``[(name, tbl_name, sql)]`` for all triggers.

        Each tuple describes one ``type = 'trigger'`` row in ``sqlite_schema``.
        If *table* is provided, only triggers whose ``tbl_name`` matches are
        returned.  Results are in insertion (ascending rowid) order.

        The ``sql`` element is the full ``CREATE TRIGGER`` statement or
        ``None`` for trigger rows written without a SQL field.

        Example::

            rows = schema.list_triggers("orders")
            # [("trg_after_insert", "orders", 5, "CREATE TRIGGER ...")]
        """
        result: list[tuple[str, str, str | None]] = []
        for _, payload in self._btree.scan():
            cols, _ = record.decode(payload)
            if cols[0] != "trigger":
                continue
            tbl_name = str(cols[2])
            if table is not None and tbl_name != table:
                continue
            sql_val = cols[4]
            sql_str = str(sql_val) if sql_val is not None else None
            result.append((str(cols[1]), tbl_name, sql_str))
        return result

    # ── Trigger write operations ──────────────────────────────────────────────

    def create_trigger(self, name: str, table: str, sql: str) -> None:
        """Insert a ``type = 'trigger'`` row into ``sqlite_schema``.

        Triggers do not own a B-tree, so ``rootpage`` is stored as ``0`` —
        the same convention used by the real ``sqlite3`` CLI for trigger rows.
        The full ``CREATE TRIGGER`` statement is stored in ``sql`` for
        round-trip fidelity.

        Raises :class:`SchemaError` if a trigger with *name* already exists.

        Example::

            schema.create_trigger(
                "trg_after_insert", "orders",
                "CREATE TRIGGER trg_after_insert AFTER INSERT ON orders BEGIN ... END"
            )
        """
        if self.find_trigger(name) is not None:
            raise SchemaError(f"trigger {name!r} already exists")

        rowid = self._next_rowid()
        payload = record.encode(["trigger", name, table, 0, sql])
        self._btree.insert(rowid, payload)
        self._bump_schema_cookie()

    def drop_trigger(self, name: str) -> None:
        """Drop a trigger: delete its ``sqlite_schema`` row, bump cookie.

        Raises :class:`SchemaError` if no trigger named *name* exists.

        Example::

            schema.drop_trigger("trg_after_insert")
            assert schema.find_trigger("trg_after_insert") is None
        """
        result = self.find_trigger(name)
        if result is None:
            raise SchemaError(f"trigger {name!r} does not exist")

        row_rowid, _, _ = result
        self._btree.delete(row_rowid)
        self._bump_schema_cookie()

    # ── Table-level mutations ─────────────────────────────────────────────────

    def update_table_sql(self, name: str, new_sql: str) -> None:
        """Rewrite the ``sql`` column of the ``sqlite_schema`` row for *name*.

        Used by ALTER TABLE ADD COLUMN to update the stored ``CREATE TABLE``
        statement without allocating a new root page.  The ``rootpage`` field
        is preserved unchanged.

        Raises :class:`SchemaError` if the table does not exist.

        Example::

            schema.update_table_sql(
                "users",
                "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, age INTEGER)"
            )
        """
        result = self.find_table(name)
        if result is None:
            raise SchemaError(f"table {name!r} does not exist")

        row_rowid, rootpage, _ = result
        payload = record.encode(["table", name, name, rootpage, new_sql])
        self._btree.update(row_rowid, payload)
        self._bump_schema_cookie()

    def drop_index(self, name: str) -> None:
        """Drop an index: free its B-tree pages, delete schema row, bump cookie.

        Steps:

        1. Locate the ``sqlite_schema`` row for *name* (must be ``type =
           'index'``).
        2. Free every page in the index's B-tree via
           :meth:`~storage_sqlite.index_tree.IndexTree.free_all`.
        3. Delete the schema row.
        4. Bump the schema cookie.

        Raises :class:`SchemaError` if no index named *name* exists.

        Example::

            schema.drop_index("auto_orders_user_id")
            assert schema.find_index("auto_orders_user_id") is None
        """
        result = self.find_index(name)
        if result is None:
            raise SchemaError(f"index {name!r} does not exist")

        row_rowid, root_pgno, _ = result

        # Free all pages in the index B-tree.
        from storage_sqlite.index_tree import IndexTree  # noqa: PLC0415

        if self._freelist is not None:
            idx_tree = IndexTree.open(self._pager, root_pgno, freelist=self._freelist)
            idx_tree.free_all(self._freelist)
        else:
            # No freelist: zero the root page so it does not hold stale data.
            self._pager.write(root_pgno, b"\x00" * self._pager.page_size)

        self._btree.delete(row_rowid)
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
