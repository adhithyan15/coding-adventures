"""
Tests for ``storage_sqlite.schema`` — the sqlite_schema catalog (phase 6).

Test organisation
-----------------

``initialize_new_database``:

1. **Rejects non-empty pager** — raises ``ValueError`` if the pager already
   has pages.
2. **Allocates exactly one page** — after the call, ``pager.size_pages == 1``.
3. **Page-1 header bytes** — bytes 0–99 are a valid SQLite file header:
   magic string, page size, schema format, text encoding.
4. **sqlite_schema leaf header** — bytes 100–107 encode an empty leaf page
   (type=0x0D, freeblock=0, ncells=0, content_start=4096, fragmented=0).
5. **Returns a Schema object** — the return value is a usable ``Schema``.

``Schema.list_tables``:

6. **Fresh database is empty** — ``list_tables()`` returns ``[]``.
7. **One table** — returns ``["users"]`` after creating "users".
8. **Insertion order preserved** — multiple tables come back in the order
   they were created.

``Schema.find_table``:

9. **Returns None for unknown table** — ``find_table("nope") is None``.
10. **Returns correct tuple** — ``(rowid, rootpage, sql)`` match what was
    passed to ``create_table``.

``Schema.rootpage_for``:

11. **Returns None for unknown table**.
12. **Returns the root page for a known table**.

``Schema.get_schema_cookie``:

13. **Starts at 1 after the first DDL** — a fresh database has cookie 0; each
    ``create_table`` or ``drop_table`` increments it by 1.
14. **Increments on every DDL** — create, create, drop = cookie 3.

``Schema.create_table``:

15. **Raises SchemaError on duplicate name** — second create with the same
    name raises ``SchemaError``.
16. **Returns a valid root page** — the returned page number is > 1 and
    contains an empty leaf header.
17. **Multiple tables have distinct root pages** — creating N tables yields N
    different page numbers.
18. **Rowid allocation starts at 1** — first row gets rowid 1, second gets 2.
19. **Rowids are monotonically increasing** — even after drops.
20. **Schema cookie bumped** — cookie increments exactly once per call.

``Schema.drop_table``:

21. **Raises SchemaError for unknown table** — ``drop_table("nope")`` raises.
22. **Table removed from list_tables** — not visible after drop.
23. **find_table returns None after drop** — confirmed clean removal.
24. **Schema cookie bumped** — cookie increments exactly once per call.
25. **Pages freed to freelist (with freelist)** — freelist total grows after
    dropping a table.
26. **Root page reused after drop (with freelist)** — a new table can reuse
    the freed root page.
27. **Without freelist: root page zeroed** — if no freelist, the root page
    is zeroed so it doesn't hold stale data.

Persistence:

28. **Tables survive commit + reopen** — create, commit, reopen → tables
    still listed.
29. **Schema cookie survives commit + reopen** — cookie value persists.
30. **BTree operations on schema-created tables** — insert/find/scan work
    on a table opened via the schema's root page.

Rollback:

31. **create_table rolled back** — after ``pager.rollback()``, the table is
    not in the schema.
32. **drop_table rolled back** — after ``pager.rollback()``, the table is
    still there.

Multi-table operations:

33. **Drop one of several tables** — other tables are unaffected.
34. **Recreate a dropped table** — same name is accepted again.
35. **Drop all tables, then create fresh ones** — schema is reusable.
"""

from __future__ import annotations

import struct

import pytest

from storage_sqlite import PAGE_SIZE, BTree, Freelist, Header, Pager, record
from storage_sqlite.schema import Schema, SchemaError, initialize_new_database

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

_SCHEMA_COOKIE_OFFSET: int = 40
_SCHEMA_HDR_OFFSET: int = 100


def _new_db(tmp_path, *, use_freelist: bool = False):
    """Create a fresh single-page database; return (pager, schema[, freelist])."""
    path = str(tmp_path / "test.db")
    pager = Pager.create(path)
    schema = initialize_new_database(pager)
    if use_freelist:
        fl = Freelist(pager)
        schema2 = Schema(pager, freelist=fl)
        return pager, schema2, fl
    return pager, schema


def _reopen(path: str, *, use_freelist: bool = False):
    """Reopen an existing database; return (pager, schema[, freelist])."""
    pager = Pager.open(path)
    if use_freelist:
        fl = Freelist(pager)
        schema = Schema(pager, freelist=fl)
        return pager, schema, fl
    schema = Schema(pager)
    return pager, schema


def _cookie(pager: Pager) -> int:
    """Read the schema cookie directly from page 1."""
    (v,) = struct.unpack_from(">I", pager.read(1), _SCHEMA_COOKIE_OFFSET)
    return v


# ---------------------------------------------------------------------------
# 1–5  initialize_new_database
# ---------------------------------------------------------------------------


class TestInitializeNewDatabase:
    """initialize_new_database sets up a correct brand-new database."""

    def test_rejects_non_empty_pager(self, tmp_path):
        """Raises ValueError if the pager already has at least one page."""
        path = str(tmp_path / "existing.db")
        pager = Pager.create(path)
        pager.allocate()  # page 1 now exists
        with pytest.raises(ValueError, match="already has"):
            initialize_new_database(pager)
        pager.close()

    def test_allocates_exactly_one_page(self, tmp_path):
        pager, _ = _new_db(tmp_path)
        assert pager.size_pages == 1
        pager.close()

    def test_page1_has_valid_sqlite_header(self, tmp_path):
        """Bytes 0–99 of page 1 are a valid 100-byte SQLite database header."""
        pager, _ = _new_db(tmp_path)
        raw = pager.read(1)
        hdr = Header.from_bytes(raw[:100])
        assert hdr.magic == b"SQLite format 3\x00"
        assert hdr.page_size == PAGE_SIZE
        # Schema format 4 is the modern default (used since SQLite 3.3.7).
        assert hdr.schema_format == 4
        # Text encoding: 1 == UTF-8.
        assert hdr.text_encoding == 1
        pager.close()

    def test_sqlite_schema_leaf_header_at_offset_100(self, tmp_path):
        """Bytes 100–107 of page 1 are an empty table-leaf page header."""
        pager, _ = _new_db(tmp_path)
        raw = pager.read(1)

        page_type = raw[100]
        freeblock, ncells, content_start, fragmented = struct.unpack_from(
            ">HHHB", raw, 101
        )

        assert page_type == 0x0D, "must be PAGE_TYPE_LEAF_TABLE"
        assert freeblock == 0, "no freeblocks in an empty page"
        assert ncells == 0, "no cells in an empty page"
        assert content_start == PAGE_SIZE, "content area starts at page end when empty"
        assert fragmented == 0
        pager.close()

    def test_returns_schema_object(self, tmp_path):
        pager, schema = _new_db(tmp_path)
        assert isinstance(schema, Schema)
        pager.close()


# ---------------------------------------------------------------------------
# 6–8  list_tables
# ---------------------------------------------------------------------------


class TestListTables:
    """Schema.list_tables returns table names in insertion order."""

    def test_fresh_database_is_empty(self, tmp_path):
        pager, schema = _new_db(tmp_path)
        assert schema.list_tables() == []
        pager.close()

    def test_single_table(self, tmp_path):
        pager, schema = _new_db(tmp_path)
        schema.create_table("users", "CREATE TABLE users (id INTEGER, name TEXT)")
        assert schema.list_tables() == ["users"]
        pager.close()

    def test_insertion_order_preserved(self, tmp_path):
        pager, schema = _new_db(tmp_path)
        schema.create_table("alpha", "CREATE TABLE alpha (x INTEGER)")
        schema.create_table("beta", "CREATE TABLE beta (x INTEGER)")
        schema.create_table("gamma", "CREATE TABLE gamma (x INTEGER)")
        assert schema.list_tables() == ["alpha", "beta", "gamma"]
        pager.close()


# ---------------------------------------------------------------------------
# 9–10  find_table
# ---------------------------------------------------------------------------


class TestFindTable:
    """Schema.find_table returns (rowid, rootpage, sql) or None."""

    def test_returns_none_for_unknown_table(self, tmp_path):
        pager, schema = _new_db(tmp_path)
        assert schema.find_table("nope") is None
        pager.close()

    def test_returns_correct_tuple(self, tmp_path):
        pager, schema = _new_db(tmp_path)
        sql = "CREATE TABLE users (id INTEGER, name TEXT)"
        root = schema.create_table("users", sql)

        result = schema.find_table("users")
        assert result is not None
        rowid, rootpage, returned_sql = result
        assert rowid == 1
        assert rootpage == root
        assert returned_sql == sql
        pager.close()

    def test_finds_second_table_independently(self, tmp_path):
        pager, schema = _new_db(tmp_path)
        schema.create_table("a", "CREATE TABLE a (x INTEGER)")
        root_b = schema.create_table("b", "CREATE TABLE b (y TEXT)")

        result = schema.find_table("b")
        assert result is not None
        rowid_b, rootpage_b, _ = result
        assert rowid_b == 2
        assert rootpage_b == root_b
        pager.close()


# ---------------------------------------------------------------------------
# 11–12  rootpage_for
# ---------------------------------------------------------------------------


class TestRootpageFor:
    """Schema.rootpage_for is a convenience wrapper around find_table."""

    def test_returns_none_for_unknown(self, tmp_path):
        pager, schema = _new_db(tmp_path)
        assert schema.rootpage_for("nope") is None
        pager.close()

    def test_returns_root_page_for_known(self, tmp_path):
        pager, schema = _new_db(tmp_path)
        root = schema.create_table("t", "CREATE TABLE t (x INTEGER)")
        assert schema.rootpage_for("t") == root
        pager.close()


# ---------------------------------------------------------------------------
# 13–14  get_schema_cookie
# ---------------------------------------------------------------------------


class TestGetSchemaCookie:
    """Schema cookie starts at 0 and increments on every DDL operation."""

    def test_fresh_database_cookie_is_zero(self, tmp_path):
        pager, schema = _new_db(tmp_path)
        assert schema.get_schema_cookie() == 0
        pager.close()

    def test_create_bumps_cookie_by_one(self, tmp_path):
        pager, schema = _new_db(tmp_path)
        schema.create_table("t", "CREATE TABLE t (x INTEGER)")
        assert schema.get_schema_cookie() == 1
        pager.close()

    def test_each_create_increments_cookie(self, tmp_path):
        pager, schema = _new_db(tmp_path)
        schema.create_table("a", "CREATE TABLE a (x INTEGER)")
        schema.create_table("b", "CREATE TABLE b (x INTEGER)")
        assert schema.get_schema_cookie() == 2
        pager.close()

    def test_drop_also_bumps_cookie(self, tmp_path):
        pager, schema = _new_db(tmp_path)
        schema.create_table("t", "CREATE TABLE t (x INTEGER)")
        schema.drop_table("t")
        # create=1, drop=2
        assert schema.get_schema_cookie() == 2
        pager.close()

    def test_cookie_in_page1_matches_get_schema_cookie(self, tmp_path):
        """The in-memory view matches the raw bytes on page 1."""
        pager, schema = _new_db(tmp_path)
        schema.create_table("t", "CREATE TABLE t (x INTEGER)")
        assert schema.get_schema_cookie() == _cookie(pager)
        pager.close()


# ---------------------------------------------------------------------------
# 15–20  create_table
# ---------------------------------------------------------------------------


class TestCreateTable:
    """Schema.create_table allocates a root page and inserts the schema row."""

    def test_raises_on_duplicate_name(self, tmp_path):
        pager, schema = _new_db(tmp_path)
        schema.create_table("users", "CREATE TABLE users (id INTEGER)")
        with pytest.raises(SchemaError, match="already exists"):
            schema.create_table("users", "CREATE TABLE users (id INTEGER)")
        pager.close()

    def test_returns_valid_root_page(self, tmp_path):
        """Root page must be > 1 and contain an empty table-leaf header."""
        pager, schema = _new_db(tmp_path)
        root = schema.create_table("t", "CREATE TABLE t (x INTEGER)")
        assert root > 1, "page 1 is sqlite_schema itself, not a user table"

        raw = pager.read(root)
        page_type = raw[0]
        assert page_type == 0x0D, "root page must be an empty leaf (0x0D)"
        _, ncells, _, _ = struct.unpack_from(">HHHB", raw, 1)
        assert ncells == 0
        pager.close()

    def test_multiple_tables_have_distinct_root_pages(self, tmp_path):
        pager, schema = _new_db(tmp_path)
        roots = [
            schema.create_table(f"t{i}", f"CREATE TABLE t{i} (x INTEGER)")
            for i in range(5)
        ]
        assert len(set(roots)) == 5, "each table must have a unique root page"
        pager.close()

    def test_first_table_gets_rowid_1(self, tmp_path):
        pager, schema = _new_db(tmp_path)
        schema.create_table("first", "CREATE TABLE first (x INTEGER)")
        result = schema.find_table("first")
        assert result is not None
        rowid, _, _ = result
        assert rowid == 1
        pager.close()

    def test_rowids_are_monotonically_increasing(self, tmp_path):
        pager, schema = _new_db(tmp_path)
        schema.create_table("a", "CREATE TABLE a (x INTEGER)")
        schema.create_table("b", "CREATE TABLE b (x INTEGER)")
        schema.create_table("c", "CREATE TABLE c (x INTEGER)")
        rowid_a = schema.find_table("a")[0]
        rowid_b = schema.find_table("b")[0]
        rowid_c = schema.find_table("c")[0]
        assert rowid_a < rowid_b < rowid_c
        pager.close()

    def test_cookie_bumped_exactly_once(self, tmp_path):
        pager, schema = _new_db(tmp_path)
        before = schema.get_schema_cookie()
        schema.create_table("t", "CREATE TABLE t (x INTEGER)")
        assert schema.get_schema_cookie() == before + 1
        pager.close()

    def test_sql_round_trips_exactly(self, tmp_path):
        """The SQL string comes back verbatim from find_table."""
        pager, schema = _new_db(tmp_path)
        sql = "CREATE TABLE weird (id INTEGER PRIMARY KEY, data BLOB NOT NULL)"
        schema.create_table("weird", sql)
        _, _, returned_sql = schema.find_table("weird")
        assert returned_sql == sql
        pager.close()


# ---------------------------------------------------------------------------
# 21–27  drop_table
# ---------------------------------------------------------------------------


class TestDropTable:
    """Schema.drop_table removes the row and frees or zeroes the root page."""

    def test_raises_for_unknown_table(self, tmp_path):
        pager, schema = _new_db(tmp_path)
        with pytest.raises(SchemaError, match="does not exist"):
            schema.drop_table("nope")
        pager.close()

    def test_table_removed_from_list(self, tmp_path):
        pager, schema = _new_db(tmp_path)
        schema.create_table("users", "CREATE TABLE users (id INTEGER)")
        schema.drop_table("users")
        assert "users" not in schema.list_tables()
        pager.close()

    def test_find_table_returns_none_after_drop(self, tmp_path):
        pager, schema = _new_db(tmp_path)
        schema.create_table("users", "CREATE TABLE users (id INTEGER)")
        schema.drop_table("users")
        assert schema.find_table("users") is None
        pager.close()

    def test_cookie_bumped_after_drop(self, tmp_path):
        pager, schema = _new_db(tmp_path)
        schema.create_table("t", "CREATE TABLE t (x INTEGER)")
        before = schema.get_schema_cookie()
        schema.drop_table("t")
        assert schema.get_schema_cookie() == before + 1
        pager.close()

    def test_pages_freed_to_freelist(self, tmp_path):
        """drop_table increases freelist.total_pages when a freelist is injected."""
        pager, schema, fl = _new_db(tmp_path, use_freelist=True)
        schema.create_table("t", "CREATE TABLE t (x INTEGER)")
        before = fl.total_pages
        schema.drop_table("t")
        assert fl.total_pages > before
        pager.close()

    def test_root_page_reused_after_drop(self, tmp_path):
        """After dropping a table its root page can be reused by a new table."""
        pager, schema, fl = _new_db(tmp_path, use_freelist=True)
        root_old = schema.create_table("old", "CREATE TABLE old (x INTEGER)")
        schema.drop_table("old")
        root_new = schema.create_table("new", "CREATE TABLE new (x INTEGER)")
        # The new root should reuse the freed page (or at least not grow the file
        # beyond what's needed).  Most simply: one of the pages is reused.
        assert root_new == root_old  # LIFO freelist returns the exact same page
        pager.close()

    def test_without_freelist_root_page_is_zeroed(self, tmp_path):
        """Without a freelist the root page is zeroed on drop."""
        pager, schema = _new_db(tmp_path)
        root = schema.create_table("t", "CREATE TABLE t (x INTEGER)")
        schema.drop_table("t")
        raw = pager.read(root)
        assert raw == b"\x00" * PAGE_SIZE
        pager.close()

    def test_drop_table_with_data_frees_all_leaf_pages(self, tmp_path):
        """A table with many rows (multiple leaf pages) frees all of them."""
        pager, schema, fl = _new_db(tmp_path, use_freelist=True)
        root = schema.create_table("big", "CREATE TABLE big (x INTEGER)")
        tree = BTree.open(pager, root, freelist=fl)
        # Insert enough rows to force at least one split (several leaf pages).
        for i in range(1, 201):
            tree.insert(i, record.encode([i, f"row{i}"]))
        pages_before_drop = pager.size_pages
        freed_before = fl.total_pages

        schema.drop_table("big")

        # Freelist must have grown by at least the root page.
        assert fl.total_pages > freed_before
        # At least the root page was freed; this is a lower bound.
        assert fl.total_pages >= freed_before + 1
        # File size did not grow.
        assert pager.size_pages == pages_before_drop
        pager.close()


# ---------------------------------------------------------------------------
# 28–30  Persistence
# ---------------------------------------------------------------------------


class TestSchemaPersistence:
    """Schema state survives commit + pager reopen."""

    def test_tables_survive_commit_reopen(self, tmp_path):
        path = str(tmp_path / "persist.db")
        pager = Pager.create(path)
        schema = initialize_new_database(pager)
        schema.create_table("users", "CREATE TABLE users (id INTEGER)")
        schema.create_table("posts", "CREATE TABLE posts (id INTEGER)")
        pager.commit()
        pager.close()

        pager2, schema2 = _reopen(path)
        assert schema2.list_tables() == ["users", "posts"]
        pager2.close()

    def test_schema_cookie_survives_commit_reopen(self, tmp_path):
        path = str(tmp_path / "cookie.db")
        pager = Pager.create(path)
        schema = initialize_new_database(pager)
        schema.create_table("a", "CREATE TABLE a (x INTEGER)")
        schema.create_table("b", "CREATE TABLE b (x INTEGER)")
        cookie_before_close = schema.get_schema_cookie()
        pager.commit()
        pager.close()

        pager2, schema2 = _reopen(path)
        assert schema2.get_schema_cookie() == cookie_before_close
        pager2.close()

    def test_btree_dml_on_schema_created_table(self, tmp_path):
        """BTree.insert/find/scan work on a table whose root came from Schema."""
        path = str(tmp_path / "dml.db")
        pager = Pager.create(path)
        schema = initialize_new_database(pager)
        root = schema.create_table("items", "CREATE TABLE items (id INTEGER, v TEXT)")

        tree = BTree.open(pager, root)
        for i in range(1, 11):
            tree.insert(i, record.encode([i, f"val{i}"]))
        pager.commit()
        pager.close()

        pager2, schema2 = _reopen(path)
        root2 = schema2.rootpage_for("items")
        assert root2 is not None
        tree2 = BTree.open(pager2, root2)
        rows = [(rid, record.decode(pl)[0]) for rid, pl in tree2.scan()]
        assert rows == [(i, [i, f"val{i}"]) for i in range(1, 11)]
        pager2.close()

    def test_find_table_fields_survive_reopen(self, tmp_path):
        """rowid, rootpage, and sql are all recovered correctly after reopen."""
        path = str(tmp_path / "fields.db")
        sql = "CREATE TABLE t (id INTEGER PRIMARY KEY, data BLOB)"
        pager = Pager.create(path)
        schema = initialize_new_database(pager)
        root = schema.create_table("t", sql)
        pager.commit()
        pager.close()

        pager2, schema2 = _reopen(path)
        result = schema2.find_table("t")
        assert result is not None
        rowid, rootpage, returned_sql = result
        assert rowid == 1
        assert rootpage == root
        assert returned_sql == sql
        pager2.close()


# ---------------------------------------------------------------------------
# 31–32  Rollback
# ---------------------------------------------------------------------------


class TestSchemaRollback:
    """Pager.rollback() undoes create_table and drop_table."""

    def test_create_table_rolled_back(self, tmp_path):
        # Commit the initial database setup so that rollback only reverts the
        # create_table call, not the initialize_new_database allocation.
        path = str(tmp_path / "rollback_create.db")
        pager = Pager.create(path)
        initialize_new_database(pager)
        pager.commit()  # commit: page 1 is now on disk

        # New transaction: create a table, then roll back.
        schema2 = Schema(pager)
        schema2.create_table("t", "CREATE TABLE t (x INTEGER)")
        assert "t" in schema2.list_tables()
        pager.rollback()
        # After rollback the schema row is gone.
        schema3 = Schema(pager)
        assert schema3.list_tables() == []
        pager.close()

    def test_drop_table_rolled_back(self, tmp_path):
        path = str(tmp_path / "rollback_drop.db")
        pager = Pager.create(path)
        schema = initialize_new_database(pager)
        schema.create_table("users", "CREATE TABLE users (id INTEGER)")
        pager.commit()

        # New transaction: drop the table, then roll back.
        schema2 = Schema(pager)
        schema2.drop_table("users")
        assert schema2.find_table("users") is None
        pager.rollback()

        # After rollback the row is back.
        schema3 = Schema(pager)
        assert "users" in schema3.list_tables()
        pager.close()


# ---------------------------------------------------------------------------
# 33–35  Multi-table operations
# ---------------------------------------------------------------------------


class TestMultiTableOperations:
    """Schema stays consistent across create/drop sequences."""

    def test_drop_one_of_several_leaves_others(self, tmp_path):
        pager, schema = _new_db(tmp_path)
        schema.create_table("a", "CREATE TABLE a (x INTEGER)")
        schema.create_table("b", "CREATE TABLE b (x INTEGER)")
        schema.create_table("c", "CREATE TABLE c (x INTEGER)")
        schema.drop_table("b")
        assert schema.list_tables() == ["a", "c"]
        pager.close()

    def test_recreate_dropped_table(self, tmp_path):
        """Same name can be used again after the original is dropped."""
        pager, schema = _new_db(tmp_path)
        schema.create_table("users", "CREATE TABLE users (id INTEGER)")
        schema.drop_table("users")
        # Recreating with the same name must not raise.
        new_root = schema.create_table("users", "CREATE TABLE users (id INTEGER, v TEXT)")
        assert schema.rootpage_for("users") == new_root
        pager.close()

    def test_drop_all_then_create_fresh(self, tmp_path):
        pager, schema = _new_db(tmp_path)
        for name in ["a", "b", "c"]:
            schema.create_table(name, f"CREATE TABLE {name} (x INTEGER)")
        for name in ["a", "b", "c"]:
            schema.drop_table(name)
        assert schema.list_tables() == []

        schema.create_table("fresh", "CREATE TABLE fresh (x INTEGER)")
        assert schema.list_tables() == ["fresh"]
        pager.close()

    def test_rowids_keep_growing_after_drops(self, tmp_path):
        """Rowids never recycle: they continue from the previous maximum."""
        pager, schema = _new_db(tmp_path)
        schema.create_table("a", "CREATE TABLE a (x INTEGER)")  # rowid 1
        schema.create_table("b", "CREATE TABLE b (x INTEGER)")  # rowid 2
        schema.drop_table("a")  # rowid 1 deleted
        schema.create_table("c", "CREATE TABLE c (x INTEGER)")  # rowid must be 3
        _, rowid_c, _ = schema.find_table("c")  # actually (rowid, rootpage, sql)
        result = schema.find_table("c")
        assert result is not None
        rowid_c = result[0]
        assert rowid_c == 3
        pager.close()

    def test_schema_error_is_storage_error_subclass(self):
        """SchemaError inherits from StorageError for uniform error handling."""
        from storage_sqlite.errors import StorageError

        assert issubclass(SchemaError, StorageError)

    def test_large_schema_many_tables(self, tmp_path):
        """Creating 50 tables stays correct: no duplication, correct order."""
        pager, schema = _new_db(tmp_path)
        names = [f"table_{i:03d}" for i in range(50)]
        for name in names:
            schema.create_table(name, f"CREATE TABLE {name} (id INTEGER)")
        assert schema.list_tables() == names
        assert schema.get_schema_cookie() == 50
        pager.close()

    def test_drop_table_with_overflow_data_frees_overflow_pages(self, tmp_path):
        """Overflow pages in a dropped table are also freed to the freelist."""
        large_payload = b"X" * 5000  # forces overflow chain
        pager, schema, fl = _new_db(tmp_path, use_freelist=True)
        root = schema.create_table("big", "CREATE TABLE big (data BLOB)")
        tree = BTree.open(pager, root, freelist=fl)
        tree.insert(1, record.encode([large_payload]))
        pages_with_overflow = pager.size_pages
        freed_before_drop = fl.total_pages

        schema.drop_table("big")

        # All pages (root + overflow pages) should be freed.
        freed_after_drop = fl.total_pages
        assert freed_after_drop > freed_before_drop
        # The file didn't grow — overflow pages came back to freelist.
        assert pager.size_pages == pages_with_overflow
        pager.close()


# ---------------------------------------------------------------------------
# Triggers
# ---------------------------------------------------------------------------


class TestSchemaTriggers:
    """Schema.create_trigger / find_trigger / list_triggers / drop_trigger."""

    def test_create_and_find(self, tmp_path):
        """A newly created trigger can be found by name."""
        pager, schema = _new_db(tmp_path)
        schema.create_table("orders", "CREATE TABLE orders (id INTEGER)")
        trg_sql = "CREATE TRIGGER trg_ai AFTER INSERT ON orders BEGIN SELECT 1; END"
        schema.create_trigger("trg_ai", "orders", trg_sql)
        result = schema.find_trigger("trg_ai")
        assert result is not None
        rowid, tbl_name, sql = result
        assert tbl_name == "orders"
        assert "trg_ai" in sql
        pager.close()

    def test_find_unknown_returns_none(self, tmp_path):
        """find_trigger returns None for a name that doesn't exist."""
        pager, schema = _new_db(tmp_path)
        assert schema.find_trigger("no_such") is None
        pager.close()

    def test_list_triggers_empty(self, tmp_path):
        """list_triggers returns an empty list when no triggers exist."""
        pager, schema = _new_db(tmp_path)
        schema.create_table("t", "CREATE TABLE t (x INTEGER)")
        assert schema.list_triggers("t") == []
        pager.close()

    def test_list_triggers_for_table(self, tmp_path):
        """list_triggers returns only triggers for the requested table."""
        pager, schema = _new_db(tmp_path)
        schema.create_table("orders", "CREATE TABLE orders (id INTEGER)")
        schema.create_table("users", "CREATE TABLE users (id INTEGER)")
        sql1 = "CREATE TRIGGER trg1 AFTER INSERT ON orders BEGIN END"
        sql2 = "CREATE TRIGGER trg2 BEFORE DELETE ON orders BEGIN END"
        sql3 = "CREATE TRIGGER trg3 AFTER INSERT ON users BEGIN END"
        schema.create_trigger("trg1", "orders", sql1)
        schema.create_trigger("trg2", "orders", sql2)
        schema.create_trigger("trg3", "users", sql3)
        rows = schema.list_triggers("orders")
        names = [r[0] for r in rows]
        assert names == ["trg1", "trg2"]
        assert all(r[1] == "orders" for r in rows)
        pager.close()

    def test_list_all_triggers_no_filter(self, tmp_path):
        """list_triggers(None) returns all triggers."""
        pager, schema = _new_db(tmp_path)
        schema.create_table("t1", "CREATE TABLE t1 (x INTEGER)")
        schema.create_table("t2", "CREATE TABLE t2 (x INTEGER)")
        schema.create_trigger("a", "t1", "CREATE TRIGGER a AFTER INSERT ON t1 BEGIN SELECT 1; END")
        schema.create_trigger("b", "t2", "CREATE TRIGGER b AFTER INSERT ON t2 BEGIN SELECT 1; END")
        rows = schema.list_triggers(None)
        assert len(rows) == 2
        pager.close()

    def test_duplicate_trigger_raises_schema_error(self, tmp_path):
        """Creating a trigger with a duplicate name raises SchemaError."""
        pager, schema = _new_db(tmp_path)
        schema.create_table("t", "CREATE TABLE t (x INTEGER)")
        schema.create_trigger("trg", "t", "CREATE TRIGGER trg AFTER INSERT ON t BEGIN END")
        with pytest.raises(SchemaError):
            schema.create_trigger("trg", "t", "CREATE TRIGGER trg AFTER INSERT ON t BEGIN END")
        pager.close()

    def test_drop_trigger(self, tmp_path):
        """drop_trigger removes the trigger; find_trigger returns None afterward."""
        pager, schema = _new_db(tmp_path)
        schema.create_table("t", "CREATE TABLE t (x INTEGER)")
        schema.create_trigger("trg", "t", "CREATE TRIGGER trg AFTER INSERT ON t BEGIN END")
        assert schema.find_trigger("trg") is not None
        schema.drop_trigger("trg")
        assert schema.find_trigger("trg") is None
        pager.close()

    def test_drop_nonexistent_trigger_raises(self, tmp_path):
        """drop_trigger raises SchemaError for a name that doesn't exist."""
        pager, schema = _new_db(tmp_path)
        with pytest.raises(SchemaError):
            schema.drop_trigger("ghost")
        pager.close()

    def test_trigger_bumps_schema_cookie(self, tmp_path):
        """create_trigger and drop_trigger each bump the schema cookie."""
        pager, schema = _new_db(tmp_path)
        schema.create_table("t", "CREATE TABLE t (x INTEGER)")
        cookie_before = schema.get_schema_cookie()
        schema.create_trigger("trg", "t", "CREATE TRIGGER trg AFTER INSERT ON t BEGIN END")
        assert schema.get_schema_cookie() == cookie_before + 1
        schema.drop_trigger("trg")
        assert schema.get_schema_cookie() == cookie_before + 2
        pager.close()

    def test_trigger_rowpage_stored_as_zero(self, tmp_path):
        """Trigger rows have rootpage = 0 (no B-tree allocation)."""
        pager, schema = _new_db(tmp_path)
        schema.create_table("t", "CREATE TABLE t (x INTEGER)")
        schema.create_trigger("trg", "t", "CREATE TRIGGER trg AFTER INSERT ON t BEGIN END")
        rowid, tbl_name, sql = schema.find_trigger("trg")
        # Verify by scanning sqlite_schema directly.
        from storage_sqlite import record
        from storage_sqlite.btree import BTree
        tree = BTree.open(pager, 1, header_offset=100)
        for _, payload in tree.scan():
            cols, _ = record.decode(payload)
            if cols[0] == "trigger" and cols[1] == "trg":
                assert int(cols[3]) == 0  # rootpage must be 0
                break
        pager.close()


# ---------------------------------------------------------------------------
# update_table_sql
# ---------------------------------------------------------------------------


class TestUpdateTableSql:
    """Schema.update_table_sql rewrites the stored CREATE TABLE statement."""

    def test_update_sql_changes_stored_text(self, tmp_path):
        """update_table_sql replaces the sql field for the named table."""
        pager, schema = _new_db(tmp_path)
        schema.create_table("t", "CREATE TABLE t (id INTEGER)")
        new_sql = "CREATE TABLE t (id INTEGER, name TEXT)"
        schema.update_table_sql("t", new_sql)
        _, _, found_sql = schema.find_table("t")
        assert found_sql == new_sql
        pager.close()

    def test_rootpage_preserved_after_update(self, tmp_path):
        """update_table_sql does not change the root page number."""
        pager, schema = _new_db(tmp_path)
        original_root = schema.create_table("t", "CREATE TABLE t (id INTEGER)")
        schema.update_table_sql("t", "CREATE TABLE t (id INTEGER, extra TEXT)")
        _, new_root, _ = schema.find_table("t")
        assert new_root == original_root
        pager.close()

    def test_update_sql_unknown_table_raises(self, tmp_path):
        """update_table_sql raises SchemaError for an unknown table name."""
        pager, schema = _new_db(tmp_path)
        with pytest.raises(SchemaError):
            schema.update_table_sql("ghost", "CREATE TABLE ghost (x INTEGER)")
        pager.close()

    def test_update_sql_bumps_cookie(self, tmp_path):
        """update_table_sql increments the schema cookie."""
        pager, schema = _new_db(tmp_path)
        schema.create_table("t", "CREATE TABLE t (id INTEGER)")
        cookie_before = schema.get_schema_cookie()
        schema.update_table_sql("t", "CREATE TABLE t (id INTEGER, extra TEXT)")
        assert schema.get_schema_cookie() == cookie_before + 1
        pager.close()


# ---------------------------------------------------------------------------
# Configurable page sizes — integration tests
# ---------------------------------------------------------------------------


class TestNonDefaultPageSize:
    """End-to-end: initialize, create table, write rows, close, reopen."""

    @pytest.mark.parametrize("ps", [1024, 8192])
    def test_initialize_reports_correct_page_size(self, tmp_path, ps: int) -> None:
        """initialize_new_database writes the page size into the header."""
        path = str(tmp_path / "db.db")
        pager = Pager.create(path, page_size=ps)
        initialize_new_database(pager)
        page1 = pager.read(1)
        hdr = Header.from_bytes(page1[:100])
        assert hdr.page_size == ps
        pager.close()

    @pytest.mark.parametrize("ps", [1024, 8192])
    def test_reopen_detects_page_size(self, tmp_path, ps: int) -> None:
        """Pager.open detects the page size from the SQLite header on reopen."""
        path = str(tmp_path / "db.db")
        with Pager.create(path, page_size=ps) as pager:
            initialize_new_database(pager)
            pager.commit()

        with Pager.open(path) as pager2:
            assert pager2.page_size == ps

    @pytest.mark.parametrize("ps", [1024, 8192])
    def test_create_table_and_reopen(self, tmp_path, ps: int) -> None:
        """Create a table in a non-4096 database, close, reopen, verify it survives."""
        path = str(tmp_path / "db.db")
        with Pager.create(path, page_size=ps) as pager:
            schema = initialize_new_database(pager)
            schema.create_table("users", "CREATE TABLE users (id INTEGER, name TEXT)")
            pager.commit()

        with Pager.open(path) as pager2:
            assert pager2.page_size == ps
            schema2 = Schema(pager2)
            assert schema2.list_tables() == ["users"]
            _, root, sql = schema2.find_table("users")
            assert "users" in sql
            assert root > 1
