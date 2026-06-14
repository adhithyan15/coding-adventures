"""
Phase 9: CREATE / DROP TRIGGER.

Tests are organised in four classes:

  TestTriggerGrammar   — grammar parses CREATE/DROP TRIGGER correctly
  TestTriggerAdapter   — adapter maps AST → CreateTriggerStmt / DropTriggerStmt
  TestTriggerBackend   — InMemoryBackend stores, lists, and removes TriggerDefs
  TestTriggerIntegration — end-to-end correctness tests via mini-sqlite
"""

from __future__ import annotations

import pytest  # noqa: F401  (used for pytest.raises)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _make_conn() -> object:
    """Open an in-memory mini-sqlite connection."""
    from mini_sqlite import connect  # type: ignore[import]

    return connect(":memory:")


def _setup_log_table(conn: object) -> None:
    """Create a simple log table for trigger audit trails (no PK — triggers use it for appends)."""
    conn.execute("CREATE TABLE log (msg TEXT)")


def _rows(conn: object, sql: str) -> list[tuple]:
    """Return all rows from a query as a list of tuples."""
    cur = conn.execute(sql)
    return cur.fetchall()


# ---------------------------------------------------------------------------
# TestTriggerGrammar — pipeline unit tests
# ---------------------------------------------------------------------------


class TestTriggerGrammar:
    """Verify CREATE/DROP TRIGGER syntax parses without error."""

    def test_grammar_parses_create_after_insert_trigger(self) -> None:
        """AFTER INSERT trigger parses to a valid program ASTNode."""
        from sql_parser import parse_sql  # type: ignore[import]

        sql = (
            "CREATE TRIGGER log_insert AFTER INSERT ON items FOR EACH ROW "
            "BEGIN INSERT INTO log (msg) VALUES ('inserted') ; END"
        )
        tree = parse_sql(sql)
        assert tree is not None
        assert tree.rule_name == "program"

    def test_grammar_parses_create_before_insert_trigger(self) -> None:
        """BEFORE INSERT trigger parses without error."""
        from sql_parser import parse_sql  # type: ignore[import]

        sql = (
            "CREATE TRIGGER chk BEFORE INSERT ON items FOR EACH ROW "
            "BEGIN INSERT INTO log (msg) VALUES ('before') ; END"
        )
        tree = parse_sql(sql)
        assert tree is not None

    def test_grammar_parses_create_after_update_trigger(self) -> None:
        """AFTER UPDATE trigger parses without error."""
        from sql_parser import parse_sql  # type: ignore[import]

        sql = (
            "CREATE TRIGGER log_update AFTER UPDATE ON items FOR EACH ROW "
            "BEGIN INSERT INTO log (msg) VALUES ('updated') ; END"
        )
        tree = parse_sql(sql)
        assert tree is not None

    def test_grammar_parses_create_after_delete_trigger(self) -> None:
        """AFTER DELETE trigger parses without error."""
        from sql_parser import parse_sql  # type: ignore[import]

        sql = (
            "CREATE TRIGGER log_delete AFTER DELETE ON items FOR EACH ROW "
            "BEGIN INSERT INTO log (msg) VALUES ('deleted') ; END"
        )
        tree = parse_sql(sql)
        assert tree is not None

    def test_grammar_parses_trigger_with_new_reference(self) -> None:
        """Trigger body that references NEW parses without error."""
        from sql_parser import parse_sql  # type: ignore[import]

        sql = (
            "CREATE TRIGGER audit AFTER INSERT ON items FOR EACH ROW "
            "BEGIN INSERT INTO log (msg) VALUES (NEW.name) ; END"
        )
        tree = parse_sql(sql)
        assert tree is not None

    def test_grammar_parses_trigger_with_old_reference(self) -> None:
        """Trigger body that references OLD parses without error."""
        from sql_parser import parse_sql  # type: ignore[import]

        sql = (
            "CREATE TRIGGER audit AFTER DELETE ON items FOR EACH ROW "
            "BEGIN INSERT INTO log (msg) VALUES (OLD.name) ; END"
        )
        tree = parse_sql(sql)
        assert tree is not None

    def test_grammar_parses_trigger_with_multiple_body_stmts(self) -> None:
        """Trigger body with multiple statements parses without error."""
        from sql_parser import parse_sql  # type: ignore[import]

        sql = (
            "CREATE TRIGGER multi AFTER INSERT ON items FOR EACH ROW "
            "BEGIN "
            "INSERT INTO log (msg) VALUES ('a') ; "
            "INSERT INTO log (msg) VALUES ('b') ; "
            "END"
        )
        tree = parse_sql(sql)
        assert tree is not None

    def test_grammar_parses_drop_trigger(self) -> None:
        """DROP TRIGGER parses to a valid program ASTNode."""
        from sql_parser import parse_sql  # type: ignore[import]

        sql = "DROP TRIGGER log_insert"
        tree = parse_sql(sql)
        assert tree is not None
        assert tree.rule_name == "program"

    def test_grammar_parses_drop_trigger_if_exists(self) -> None:
        """DROP TRIGGER IF EXISTS parses without error."""
        from sql_parser import parse_sql  # type: ignore[import]

        sql = "DROP TRIGGER IF EXISTS log_insert"
        tree = parse_sql(sql)
        assert tree is not None


# ---------------------------------------------------------------------------
# TestTriggerAdapter — adapter unit tests
# ---------------------------------------------------------------------------


class TestTriggerAdapter:
    """Verify the adapter maps AST nodes to the correct planner Statement types."""

    def _adapt(self, sql: str) -> object:
        from sql_parser import parse_sql  # type: ignore[import]

        from mini_sqlite.adapter import to_statement  # type: ignore[import]

        return to_statement(parse_sql(sql))

    def test_adapter_create_after_insert(self) -> None:
        """CREATE TRIGGER … AFTER INSERT produces CreateTriggerStmt with correct fields."""
        from sql_planner import CreateTriggerStmt  # type: ignore[import]

        stmt = self._adapt(
            "CREATE TRIGGER t1 AFTER INSERT ON orders FOR EACH ROW "
            "BEGIN INSERT INTO log (msg) VALUES ('inserted') ; END"
        )
        assert isinstance(stmt, CreateTriggerStmt)
        assert stmt.name == "t1"
        assert stmt.timing == "AFTER"
        assert stmt.event == "INSERT"
        assert stmt.table == "orders"
        assert "INSERT" in stmt.body_sql.upper()

    def test_adapter_create_before_insert(self) -> None:
        """BEFORE INSERT produces CreateTriggerStmt with timing='BEFORE'."""
        from sql_planner import CreateTriggerStmt  # type: ignore[import]

        stmt = self._adapt(
            "CREATE TRIGGER t2 BEFORE INSERT ON orders FOR EACH ROW "
            "BEGIN INSERT INTO log (msg) VALUES ('before') ; END"
        )
        assert isinstance(stmt, CreateTriggerStmt)
        assert stmt.timing == "BEFORE"
        assert stmt.event == "INSERT"

    def test_adapter_create_after_update(self) -> None:
        """AFTER UPDATE produces CreateTriggerStmt with event='UPDATE'."""
        from sql_planner import CreateTriggerStmt  # type: ignore[import]

        stmt = self._adapt(
            "CREATE TRIGGER t3 AFTER UPDATE ON orders FOR EACH ROW "
            "BEGIN INSERT INTO log (msg) VALUES ('updated') ; END"
        )
        assert isinstance(stmt, CreateTriggerStmt)
        assert stmt.event == "UPDATE"

    def test_adapter_create_after_delete(self) -> None:
        """AFTER DELETE produces CreateTriggerStmt with event='DELETE'."""
        from sql_planner import CreateTriggerStmt  # type: ignore[import]

        stmt = self._adapt(
            "CREATE TRIGGER t4 AFTER DELETE ON orders FOR EACH ROW "
            "BEGIN INSERT INTO log (msg) VALUES ('deleted') ; END"
        )
        assert isinstance(stmt, CreateTriggerStmt)
        assert stmt.event == "DELETE"

    def test_adapter_new_normalised_uppercase(self) -> None:
        """body_sql contains 'NEW' (uppercase) even when written as 'new' in SQL."""
        from sql_planner import CreateTriggerStmt  # type: ignore[import]

        # Grammar produces NAME tokens for new/old; adapter normalises to uppercase.
        stmt = self._adapt(
            "CREATE TRIGGER t5 AFTER INSERT ON items FOR EACH ROW "
            "BEGIN INSERT INTO log (msg) VALUES (NEW.name) ; END"
        )
        assert isinstance(stmt, CreateTriggerStmt)
        assert "NEW" in stmt.body_sql

    def test_adapter_old_normalised_uppercase(self) -> None:
        """body_sql contains 'OLD' (uppercase) for OLD pseudo-table references."""
        from sql_planner import CreateTriggerStmt  # type: ignore[import]

        stmt = self._adapt(
            "CREATE TRIGGER t6 AFTER DELETE ON items FOR EACH ROW "
            "BEGIN INSERT INTO log (msg) VALUES (OLD.name) ; END"
        )
        assert isinstance(stmt, CreateTriggerStmt)
        assert "OLD" in stmt.body_sql

    def test_adapter_multiple_body_statements(self) -> None:
        """Multiple body statements are joined with ' ; ' in body_sql."""
        from sql_planner import CreateTriggerStmt  # type: ignore[import]

        stmt = self._adapt(
            "CREATE TRIGGER t7 AFTER INSERT ON items FOR EACH ROW "
            "BEGIN "
            "INSERT INTO log (msg) VALUES ('a') ; "
            "INSERT INTO log (msg) VALUES ('b') ; "
            "END"
        )
        assert isinstance(stmt, CreateTriggerStmt)
        assert " ; " in stmt.body_sql

    def test_adapter_drop_trigger(self) -> None:
        """DROP TRIGGER name produces DropTriggerStmt(name=..., if_exists=False)."""
        from sql_planner import DropTriggerStmt  # type: ignore[import]

        stmt = self._adapt("DROP TRIGGER t1")
        assert isinstance(stmt, DropTriggerStmt)
        assert stmt.name == "t1"
        assert stmt.if_exists is False

    def test_adapter_drop_trigger_if_exists(self) -> None:
        """DROP TRIGGER IF EXISTS produces DropTriggerStmt with if_exists=True."""
        from sql_planner import DropTriggerStmt  # type: ignore[import]

        stmt = self._adapt("DROP TRIGGER IF EXISTS t1")
        assert isinstance(stmt, DropTriggerStmt)
        assert stmt.if_exists is True


# ---------------------------------------------------------------------------
# TestTriggerBackend — InMemoryBackend unit tests
# ---------------------------------------------------------------------------


class TestTriggerBackend:
    """Verify InMemoryBackend correctly stores and retrieves TriggerDefs."""

    def _backend(self) -> object:
        from sql_backend import InMemoryBackend  # type: ignore[import]

        return InMemoryBackend()

    def test_create_trigger_stores_def(self) -> None:
        """create_trigger stores the TriggerDef and list_triggers returns it."""
        from sql_backend import InMemoryBackend  # type: ignore[import]
        from sql_backend.schema import TriggerDef  # type: ignore[import]

        b = InMemoryBackend()
        d = TriggerDef(name="t1", table="orders", timing="AFTER", event="INSERT", body="SELECT 1")
        b.create_trigger(d)
        assert b.list_triggers("orders") == [d]

    def test_list_triggers_empty_for_unknown_table(self) -> None:
        """list_triggers returns [] for a table with no triggers."""
        from sql_backend import InMemoryBackend  # type: ignore[import]

        b = InMemoryBackend()
        assert b.list_triggers("no_such_table") == []

    def test_list_triggers_returns_in_creation_order(self) -> None:
        """Multiple triggers on the same table are returned in creation order."""
        from sql_backend import InMemoryBackend  # type: ignore[import]
        from sql_backend.schema import TriggerDef  # type: ignore[import]

        b = InMemoryBackend()
        d1 = TriggerDef(name="t1", table="orders", timing="AFTER", event="INSERT", body="SELECT 1")
        d2 = TriggerDef(name="t2", table="orders", timing="AFTER", event="INSERT", body="SELECT 2")
        b.create_trigger(d1)
        b.create_trigger(d2)
        assert b.list_triggers("orders") == [d1, d2]

    def test_create_trigger_already_exists_raises(self) -> None:
        """Creating a trigger with a duplicate name raises TriggerAlreadyExists."""
        from sql_backend import InMemoryBackend  # type: ignore[import]
        from sql_backend.errors import TriggerAlreadyExists  # type: ignore[import]
        from sql_backend.schema import TriggerDef  # type: ignore[import]

        b = InMemoryBackend()
        d = TriggerDef(name="t1", table="orders", timing="AFTER", event="INSERT", body="SELECT 1")
        b.create_trigger(d)
        with pytest.raises(TriggerAlreadyExists):
            b.create_trigger(d)

    def test_drop_trigger_removes_from_list(self) -> None:
        """drop_trigger removes the trigger from list_triggers."""
        from sql_backend import InMemoryBackend  # type: ignore[import]
        from sql_backend.schema import TriggerDef  # type: ignore[import]

        b = InMemoryBackend()
        d = TriggerDef(name="t1", table="orders", timing="AFTER", event="INSERT", body="SELECT 1")
        b.create_trigger(d)
        b.drop_trigger("t1")
        assert b.list_triggers("orders") == []

    def test_drop_trigger_not_found_raises(self) -> None:
        """drop_trigger on a non-existent name raises TriggerNotFound."""
        from sql_backend import InMemoryBackend  # type: ignore[import]
        from sql_backend.errors import TriggerNotFound  # type: ignore[import]

        b = InMemoryBackend()
        with pytest.raises(TriggerNotFound):
            b.drop_trigger("ghost")

    def test_drop_trigger_if_exists_no_error(self) -> None:
        """drop_trigger(if_exists=True) on a non-existent name is silent."""
        from sql_backend import InMemoryBackend  # type: ignore[import]

        b = InMemoryBackend()
        b.drop_trigger("ghost", if_exists=True)  # must not raise

    def test_list_triggers_only_returns_matching_table(self) -> None:
        """list_triggers for table A doesn't return triggers registered for table B."""
        from sql_backend import InMemoryBackend  # type: ignore[import]
        from sql_backend.schema import TriggerDef  # type: ignore[import]

        b = InMemoryBackend()
        da = TriggerDef(name="ta", table="a", timing="AFTER", event="INSERT", body="SELECT 1")
        db = TriggerDef(name="tb", table="b", timing="AFTER", event="INSERT", body="SELECT 1")
        b.create_trigger(da)
        b.create_trigger(db)
        assert b.list_triggers("a") == [da]
        assert b.list_triggers("b") == [db]


# ---------------------------------------------------------------------------
# TestTriggerIntegration — end-to-end tests via mini-sqlite
# ---------------------------------------------------------------------------


class TestTriggerIntegration:
    """End-to-end trigger tests via mini_sqlite.connect(':memory:')."""

    # ----------------------------------------------------------------
    # AFTER INSERT
    # ----------------------------------------------------------------

    def test_after_insert_trigger_fires(self) -> None:
        """AFTER INSERT trigger inserts a row into the log table."""
        conn = _make_conn()
        conn.execute("CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT)")
        _setup_log_table(conn)
        conn.execute(
            "CREATE TRIGGER log_ins AFTER INSERT ON items FOR EACH ROW "
            "BEGIN INSERT INTO log (msg) VALUES ('inserted') ; END"
        )
        conn.execute("INSERT INTO items VALUES (1, 'apple')")
        rows = _rows(conn, "SELECT msg FROM log")
        assert rows == [("inserted",)]

    def test_after_insert_trigger_fires_for_each_row(self) -> None:
        """AFTER INSERT trigger fires once per inserted row."""
        conn = _make_conn()
        conn.execute("CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT)")
        _setup_log_table(conn)
        conn.execute(
            "CREATE TRIGGER log_ins AFTER INSERT ON items FOR EACH ROW "
            "BEGIN INSERT INTO log (msg) VALUES ('inserted') ; END"
        )
        conn.execute("INSERT INTO items VALUES (1, 'a')")
        conn.execute("INSERT INTO items VALUES (2, 'b')")
        conn.execute("INSERT INTO items VALUES (3, 'c')")
        assert len(_rows(conn, "SELECT msg FROM log")) == 3

    # ----------------------------------------------------------------
    # AFTER UPDATE
    # ----------------------------------------------------------------

    def test_after_update_trigger_fires(self) -> None:
        """AFTER UPDATE trigger fires when a row is updated."""
        conn = _make_conn()
        conn.execute("CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT)")
        _setup_log_table(conn)
        conn.execute("INSERT INTO items VALUES (1, 'apple')")
        conn.execute(
            "CREATE TRIGGER log_upd AFTER UPDATE ON items FOR EACH ROW "
            "BEGIN INSERT INTO log (msg) VALUES ('updated') ; END"
        )
        conn.execute("UPDATE items SET name = 'banana' WHERE id = 1")
        rows = _rows(conn, "SELECT msg FROM log")
        assert rows == [("updated",)]

    def test_after_update_trigger_not_fires_without_match(self) -> None:
        """AFTER UPDATE trigger does NOT fire when no rows are matched."""
        conn = _make_conn()
        conn.execute("CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT)")
        _setup_log_table(conn)
        conn.execute("INSERT INTO items VALUES (1, 'apple')")
        conn.execute(
            "CREATE TRIGGER log_upd AFTER UPDATE ON items FOR EACH ROW "
            "BEGIN INSERT INTO log (msg) VALUES ('updated') ; END"
        )
        conn.execute("UPDATE items SET name = 'banana' WHERE id = 999")
        assert len(_rows(conn, "SELECT msg FROM log")) == 0

    # ----------------------------------------------------------------
    # AFTER DELETE
    # ----------------------------------------------------------------

    def test_after_delete_trigger_fires(self) -> None:
        """AFTER DELETE trigger fires when a row is deleted."""
        conn = _make_conn()
        conn.execute("CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT)")
        _setup_log_table(conn)
        conn.execute("INSERT INTO items VALUES (1, 'apple')")
        conn.execute(
            "CREATE TRIGGER log_del AFTER DELETE ON items FOR EACH ROW "
            "BEGIN INSERT INTO log (msg) VALUES ('deleted') ; END"
        )
        conn.execute("DELETE FROM items WHERE id = 1")
        rows = _rows(conn, "SELECT msg FROM log")
        assert rows == [("deleted",)]

    # ----------------------------------------------------------------
    # BEFORE INSERT
    # ----------------------------------------------------------------

    def test_before_insert_trigger_fires_before_row_exists(self) -> None:
        """BEFORE INSERT trigger fires before the row is written to the table."""
        conn = _make_conn()
        conn.execute("CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT)")
        _setup_log_table(conn)
        conn.execute(
            "CREATE TRIGGER pre_ins BEFORE INSERT ON items FOR EACH ROW "
            "BEGIN INSERT INTO log (msg) VALUES ('before') ; END"
        )
        conn.execute("INSERT INTO items VALUES (1, 'apple')")
        # The row should exist in items (trigger didn't abort it)
        assert _rows(conn, "SELECT count(*) FROM items")[0][0] == 1
        # The log entry should exist too
        assert _rows(conn, "SELECT msg FROM log")[0][0] == "before"

    # ----------------------------------------------------------------
    # NEW pseudo-table
    # ----------------------------------------------------------------

    def test_new_pseudo_table_readable_in_insert_trigger(self) -> None:
        """NEW.col is accessible in an AFTER INSERT trigger body."""
        conn = _make_conn()
        conn.execute("CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT)")
        _setup_log_table(conn)
        conn.execute(
            "CREATE TRIGGER capture_new AFTER INSERT ON items FOR EACH ROW "
            "BEGIN INSERT INTO log (msg) VALUES (NEW.name) ; END"
        )
        conn.execute("INSERT INTO items VALUES (1, 'pear')")
        rows = _rows(conn, "SELECT msg FROM log")
        assert rows == [("pear",)]

    def test_new_pseudo_table_readable_in_update_trigger(self) -> None:
        """NEW.col reflects the post-update value in an AFTER UPDATE trigger."""
        conn = _make_conn()
        conn.execute("CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT)")
        _setup_log_table(conn)
        conn.execute("INSERT INTO items VALUES (1, 'apple')")
        conn.execute(
            "CREATE TRIGGER capture_new AFTER UPDATE ON items FOR EACH ROW "
            "BEGIN INSERT INTO log (msg) VALUES (NEW.name) ; END"
        )
        conn.execute("UPDATE items SET name = 'grape' WHERE id = 1")
        rows = _rows(conn, "SELECT msg FROM log")
        assert rows == [("grape",)]

    # ----------------------------------------------------------------
    # OLD pseudo-table
    # ----------------------------------------------------------------

    def test_old_pseudo_table_readable_in_delete_trigger(self) -> None:
        """OLD.col contains the deleted row's value in an AFTER DELETE trigger."""
        conn = _make_conn()
        conn.execute("CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT)")
        _setup_log_table(conn)
        conn.execute("INSERT INTO items VALUES (1, 'mango')")
        conn.execute(
            "CREATE TRIGGER capture_old AFTER DELETE ON items FOR EACH ROW "
            "BEGIN INSERT INTO log (msg) VALUES (OLD.name) ; END"
        )
        conn.execute("DELETE FROM items WHERE id = 1")
        rows = _rows(conn, "SELECT msg FROM log")
        assert rows == [("mango",)]

    def test_old_pseudo_table_readable_in_update_trigger(self) -> None:
        """OLD.col reflects the pre-update value in an AFTER UPDATE trigger."""
        conn = _make_conn()
        conn.execute("CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT)")
        _setup_log_table(conn)
        conn.execute("INSERT INTO items VALUES (1, 'apple')")
        conn.execute(
            "CREATE TRIGGER capture_old AFTER UPDATE ON items FOR EACH ROW "
            "BEGIN INSERT INTO log (msg) VALUES (OLD.name) ; END"
        )
        conn.execute("UPDATE items SET name = 'grape' WHERE id = 1")
        rows = _rows(conn, "SELECT msg FROM log")
        assert rows == [("apple",)]

    # ----------------------------------------------------------------
    # Multiple triggers in creation order
    # ----------------------------------------------------------------

    def test_multiple_triggers_fire_in_creation_order(self) -> None:
        """Multiple triggers on the same table fire in the order they were created."""
        conn = _make_conn()
        conn.execute("CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT)")
        _setup_log_table(conn)
        conn.execute(
            "CREATE TRIGGER first AFTER INSERT ON items FOR EACH ROW "
            "BEGIN INSERT INTO log (msg) VALUES ('first') ; END"
        )
        conn.execute(
            "CREATE TRIGGER second AFTER INSERT ON items FOR EACH ROW "
            "BEGIN INSERT INTO log (msg) VALUES ('second') ; END"
        )
        conn.execute("INSERT INTO items VALUES (1, 'x')")
        rows = _rows(conn, "SELECT msg FROM log")
        assert [r[0] for r in rows] == ["first", "second"]

    # ----------------------------------------------------------------
    # Multiple body statements
    # ----------------------------------------------------------------

    def test_trigger_multiple_body_statements_all_execute(self) -> None:
        """All statements in a multi-statement trigger body execute."""
        conn = _make_conn()
        conn.execute("CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT)")
        _setup_log_table(conn)
        conn.execute(
            "CREATE TRIGGER multi AFTER INSERT ON items FOR EACH ROW "
            "BEGIN "
            "INSERT INTO log (msg) VALUES ('one') ; "
            "INSERT INTO log (msg) VALUES ('two') ; "
            "END"
        )
        conn.execute("INSERT INTO items VALUES (1, 'x')")
        rows = _rows(conn, "SELECT msg FROM log")
        assert [r[0] for r in rows] == ["one", "two"]

    # ----------------------------------------------------------------
    # DROP TRIGGER
    # ----------------------------------------------------------------

    def test_drop_trigger_stops_it_from_firing(self) -> None:
        """After DROP TRIGGER, the trigger no longer fires on DML."""
        conn = _make_conn()
        conn.execute("CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT)")
        _setup_log_table(conn)
        conn.execute(
            "CREATE TRIGGER log_ins AFTER INSERT ON items FOR EACH ROW "
            "BEGIN INSERT INTO log (msg) VALUES ('fired') ; END"
        )
        conn.execute("INSERT INTO items VALUES (1, 'a')")
        assert len(_rows(conn, "SELECT msg FROM log")) == 1

        conn.execute("DROP TRIGGER log_ins")
        conn.execute("INSERT INTO items VALUES (2, 'b')")
        # Still only 1 log row — trigger didn't fire again
        assert len(_rows(conn, "SELECT msg FROM log")) == 1

    def test_drop_trigger_if_exists_no_error_when_absent(self) -> None:
        """DROP TRIGGER IF EXISTS is silent when the trigger doesn't exist."""
        conn = _make_conn()
        conn.execute("DROP TRIGGER IF EXISTS no_such_trigger")  # must not raise

    def test_drop_trigger_not_exists_raises(self) -> None:
        """DROP TRIGGER on a non-existent trigger raises an error."""
        from mini_sqlite.errors import InternalError  # type: ignore[import]

        conn = _make_conn()
        with pytest.raises(InternalError):
            conn.execute("DROP TRIGGER no_such_trigger")

    # ----------------------------------------------------------------
    # Trigger depth / recursion guard
    # ----------------------------------------------------------------

    def test_trigger_recursion_depth_error(self) -> None:
        """A trigger that causes itself to fire recursively raises TriggerDepthError."""
        conn = _make_conn()
        conn.execute("CREATE TABLE counter (n INTEGER)")
        # This trigger inserts into counter, which fires the same trigger → infinite loop
        conn.execute(
            "CREATE TRIGGER recurse AFTER INSERT ON counter FOR EACH ROW "
            "BEGIN INSERT INTO counter VALUES (1) ; END"
        )
        with pytest.raises(Exception, match="depth"):
            conn.execute("INSERT INTO counter VALUES (1)")

    # ----------------------------------------------------------------
    # Trigger does not fire on different tables
    # ----------------------------------------------------------------

    def test_trigger_only_fires_on_registered_table(self) -> None:
        """A trigger on table A does not fire when table B is modified."""
        conn = _make_conn()
        conn.execute("CREATE TABLE a (id INTEGER PRIMARY KEY)")
        conn.execute("CREATE TABLE b (id INTEGER PRIMARY KEY)")
        _setup_log_table(conn)
        conn.execute(
            "CREATE TRIGGER log_a AFTER INSERT ON a FOR EACH ROW "
            "BEGIN INSERT INTO log (msg) VALUES ('a fired') ; END"
        )
        conn.execute("INSERT INTO b VALUES (1)")
        assert len(_rows(conn, "SELECT msg FROM log")) == 0
        conn.execute("INSERT INTO a VALUES (1)")
        assert len(_rows(conn, "SELECT msg FROM log")) == 1

    # ----------------------------------------------------------------
    # Trigger + transaction rollback
    # ----------------------------------------------------------------

    def test_trigger_effects_rolled_back_with_transaction(self) -> None:
        """Trigger side-effects inside a rolled-back transaction are undone."""
        conn = _make_conn()
        conn.execute("CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT)")
        _setup_log_table(conn)
        conn.execute(
            "CREATE TRIGGER log_ins AFTER INSERT ON items FOR EACH ROW "
            "BEGIN INSERT INTO log (msg) VALUES ('inserted') ; END"
        )
        conn.execute("BEGIN")
        conn.execute("INSERT INTO items VALUES (1, 'x')")
        conn.execute("ROLLBACK")
        # Both items and log should be empty after rollback
        assert len(_rows(conn, "SELECT id FROM items")) == 0
        assert len(_rows(conn, "SELECT msg FROM log")) == 0
