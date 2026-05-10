"""VM-level upsert tests — InsertRow / InsertFromResult + UpsertSpec.

These tests drive the VM directly via compiled LogicalPlan trees so that
every branch inside ``_do_upsert`` and ``_upsert_apply`` is exercised
without routing through the full mini-sqlite adapter stack.

Coverage targets (vm.py):
  - DO NOTHING fast path (conflict → skip)
  - DO NOTHING with no conflict (plain insert)
  - DO UPDATE with explicit conflict_target
  - DO UPDATE bare column reference in SET (existing row value)
  - DO UPDATE EXCLUDED.col in SET (would-be-inserted value)
  - DO UPDATE arithmetic: existing.col + EXCLUDED.col
  - DO UPDATE multiple columns
  - DO UPDATE with empty conflict_target (schema-discovered PK)
  - DO UPDATE no match found (conflict_target names non-existent row)
  - InsertFromResult + DO NOTHING (INSERT…SELECT path)
  - InsertFromResult + DO UPDATE (INSERT…SELECT path)
  - Counter accumulation: repeated upserts with n = n + 1
"""

from __future__ import annotations

from sql_backend.in_memory import InMemoryBackend
from sql_backend.schema import ColumnDef as BackendColumnDef
from sql_codegen import compile
from sql_planner import (
    BinaryExpr,
    BinaryOp,
    Column,
    ExcludedColumn,
    Insert,
    InsertSource,
    Literal,
    UpsertAction,
    UpsertAssignment,
)

from sql_vm import execute

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _make_table(
    name: str,
    cols: list[tuple[str, str, bool]],  # (name, type, primary_key)
    backend: InMemoryBackend,
) -> None:
    """Create *name* in *backend* with the given columns."""
    backend.create_table(
        name,
        [
            BackendColumnDef(name=n, type_name=t, primary_key=pk)
            for n, t, pk in cols
        ],
        if_not_exists=False,
    )


def _scan(backend: InMemoryBackend, table: str) -> list[dict]:
    cur = backend.scan(table)
    rows: list[dict] = []
    while True:
        r = cur.next()
        if r is None:
            break
        rows.append(dict(r))
    cur.close()
    return rows


def _insert_row(
    backend: InMemoryBackend,
    table: str,
    cols: tuple[str, ...],
    values: tuple,
    *,
    upsert: UpsertAction | None = None,
) -> None:
    """Compile and execute a single-row VALUES INSERT into *backend*."""
    plan = Insert(
        table=table,
        columns=cols,
        source=InsertSource(values=(tuple(Literal(value=v) for v in values),)),
        upsert=upsert,
    )
    execute(compile(plan), backend)


# ---------------------------------------------------------------------------
# DO NOTHING
# ---------------------------------------------------------------------------


class TestUpsertDoNothing:
    """ON CONFLICT DO NOTHING — conflict is silently skipped."""

    def test_do_nothing_conflict_skipped(self) -> None:
        """A conflicting row is silently dropped; the original row survives."""
        be = InMemoryBackend()
        _make_table("t", [("id", "INTEGER", True), ("val", "TEXT", False)], be)
        # Seed the table.
        _insert_row(be, "t", ("id", "val"), (1, "original"))
        # Second insert conflicts → DO NOTHING.
        _insert_row(
            be,
            "t",
            ("id", "val"),
            (1, "new"),
            upsert=UpsertAction(conflict_target=(), do_nothing=True),
        )
        rows = _scan(be, "t")
        assert rows == [{"id": 1, "val": "original"}]

    def test_do_nothing_no_conflict_inserts(self) -> None:
        """When there is no conflict, DO NOTHING still inserts the row."""
        be = InMemoryBackend()
        _make_table("t", [("id", "INTEGER", True), ("val", "TEXT", False)], be)
        _insert_row(be, "t", ("id", "val"), (1, "first"))
        _insert_row(
            be,
            "t",
            ("id", "val"),
            (2, "second"),
            upsert=UpsertAction(conflict_target=(), do_nothing=True),
        )
        rows = _scan(be, "t")
        assert len(rows) == 2
        assert {"id": 2, "val": "second"} in rows

    def test_do_nothing_with_explicit_conflict_target(self) -> None:
        """ON CONFLICT (id) DO NOTHING respects the named conflict target."""
        be = InMemoryBackend()
        _make_table("t", [("id", "INTEGER", True), ("val", "TEXT", False)], be)
        _insert_row(be, "t", ("id", "val"), (42, "keep"))
        _insert_row(
            be,
            "t",
            ("id", "val"),
            (42, "drop"),
            upsert=UpsertAction(conflict_target=("id",), do_nothing=True),
        )
        rows = _scan(be, "t")
        assert rows == [{"id": 42, "val": "keep"}]


# ---------------------------------------------------------------------------
# DO UPDATE — EXCLUDED.col and bare column references
# ---------------------------------------------------------------------------


class TestUpsertDoUpdate:
    """ON CONFLICT DO UPDATE SET — in-place update via positioned cursor."""

    def test_do_update_excluded_col(self) -> None:
        """EXCLUDED.col provides the would-be-inserted value."""
        be = InMemoryBackend()
        _make_table("t", [("id", "INTEGER", True), ("val", "TEXT", False)], be)
        _insert_row(be, "t", ("id", "val"), (1, "old"))
        _insert_row(
            be,
            "t",
            ("id", "val"),
            (1, "new"),
            upsert=UpsertAction(
                conflict_target=("id",),
                assignments=(
                    UpsertAssignment(column="val", value=ExcludedColumn(col="val")),
                ),
            ),
        )
        rows = _scan(be, "t")
        assert rows == [{"id": 1, "val": "new"}]

    def test_do_update_literal_set(self) -> None:
        """SET clause can use a literal value instead of EXCLUDED."""
        be = InMemoryBackend()
        _make_table("t", [("id", "INTEGER", True), ("val", "TEXT", False)], be)
        _insert_row(be, "t", ("id", "val"), (1, "old"))
        _insert_row(
            be,
            "t",
            ("id", "val"),
            (1, "ignored"),
            upsert=UpsertAction(
                conflict_target=("id",),
                assignments=(
                    UpsertAssignment(column="val", value=Literal(value="hardcoded")),
                ),
            ),
        )
        rows = _scan(be, "t")
        assert rows == [{"id": 1, "val": "hardcoded"}]

    def test_do_update_bare_column_ref(self) -> None:
        """A bare column ref in SET reads from the *existing* row."""
        be = InMemoryBackend()
        _make_table("t", [("id", "INTEGER", True), ("n", "INTEGER", False)], be)
        _insert_row(be, "t", ("id", "n"), (1, 10))
        # SET n = n (existing row's n)
        _insert_row(
            be,
            "t",
            ("id", "n"),
            (1, 99),
            upsert=UpsertAction(
                conflict_target=("id",),
                assignments=(
                    UpsertAssignment(
                        column="n",
                        value=Column(table=None, col="n"),
                    ),
                ),
            ),
        )
        rows = _scan(be, "t")
        # n stays as 10 (we set it to the existing value of n)
        assert rows == [{"id": 1, "n": 10}]

    def test_do_update_arithmetic_existing_plus_excluded(self) -> None:
        """qty = qty + EXCLUDED.qty accumulates: existing + new-incoming."""
        be = InMemoryBackend()
        _make_table("inv", [("id", "INTEGER", True), ("qty", "INTEGER", False)], be)
        _insert_row(be, "inv", ("id", "qty"), (1, 10))
        _insert_row(
            be,
            "inv",
            ("id", "qty"),
            (1, 5),
            upsert=UpsertAction(
                conflict_target=("id",),
                assignments=(
                    UpsertAssignment(
                        column="qty",
                        value=BinaryExpr(
                            op=BinaryOp.ADD,
                            left=Column(table=None, col="qty"),
                            right=ExcludedColumn(col="qty"),
                        ),
                    ),
                ),
            ),
        )
        rows = _scan(be, "inv")
        assert rows == [{"id": 1, "qty": 15}]

    def test_do_update_multiple_columns(self) -> None:
        """Multiple SET assignments in one upsert all take effect."""
        be = InMemoryBackend()
        _make_table(
            "t",
            [("id", "INTEGER", True), ("a", "TEXT", False), ("b", "TEXT", False)],
            be,
        )
        _insert_row(be, "t", ("id", "a", "b"), (1, "a1", "b1"))
        _insert_row(
            be,
            "t",
            ("id", "a", "b"),
            (1, "a2", "b2"),
            upsert=UpsertAction(
                conflict_target=("id",),
                assignments=(
                    UpsertAssignment(column="a", value=ExcludedColumn(col="a")),
                    UpsertAssignment(column="b", value=ExcludedColumn(col="b")),
                ),
            ),
        )
        rows = _scan(be, "t")
        assert rows == [{"id": 1, "a": "a2", "b": "b2"}]

    def test_do_update_preserves_non_updated_columns(self) -> None:
        """Columns absent from SET keep their existing values."""
        be = InMemoryBackend()
        _make_table(
            "t",
            [("id", "INTEGER", True), ("a", "TEXT", False), ("b", "TEXT", False)],
            be,
        )
        _insert_row(be, "t", ("id", "a", "b"), (1, "a1", "b1"))
        # Only update 'a'; 'b' should stay as 'b1'.
        _insert_row(
            be,
            "t",
            ("id", "a", "b"),
            (1, "a2", "b2"),
            upsert=UpsertAction(
                conflict_target=("id",),
                assignments=(
                    UpsertAssignment(column="a", value=ExcludedColumn(col="a")),
                ),
            ),
        )
        rows = _scan(be, "t")
        assert rows == [{"id": 1, "a": "a2", "b": "b1"}]

    def test_do_update_no_conflict_plain_insert(self) -> None:
        """When there is no conflict, DO UPDATE inserts as a normal row."""
        be = InMemoryBackend()
        _make_table("t", [("id", "INTEGER", True), ("val", "TEXT", False)], be)
        _insert_row(be, "t", ("id", "val"), (1, "existing"))
        _insert_row(
            be,
            "t",
            ("id", "val"),
            (2, "new"),
            upsert=UpsertAction(
                conflict_target=("id",),
                assignments=(
                    UpsertAssignment(column="val", value=ExcludedColumn(col="val")),
                ),
            ),
        )
        rows = sorted(_scan(be, "t"), key=lambda r: r["id"])
        assert rows == [{"id": 1, "val": "existing"}, {"id": 2, "val": "new"}]

    def test_do_update_empty_conflict_target_discovers_pk(self) -> None:
        """With no explicit target, the PK column is auto-discovered."""
        be = InMemoryBackend()
        _make_table("t", [("id", "INTEGER", True), ("val", "TEXT", False)], be)
        _insert_row(be, "t", ("id", "val"), (7, "old"))
        # Empty conflict_target: VM discovers the PK via backend.columns().
        _insert_row(
            be,
            "t",
            ("id", "val"),
            (7, "new"),
            upsert=UpsertAction(
                conflict_target=(),
                assignments=(
                    UpsertAssignment(column="val", value=ExcludedColumn(col="val")),
                ),
            ),
        )
        rows = _scan(be, "t")
        assert rows == [{"id": 7, "val": "new"}]

    def test_do_update_counter_accumulation(self) -> None:
        """Canonical counter upsert: repeated inserts increment the count."""
        be = InMemoryBackend()
        _make_table("hits", [("evt", "TEXT", True), ("n", "INTEGER", False)], be)
        upsert = UpsertAction(
            conflict_target=("evt",),
            assignments=(
                UpsertAssignment(
                    column="n",
                    value=BinaryExpr(
                        op=BinaryOp.ADD,
                        left=Column(table=None, col="n"),
                        right=Literal(value=1),
                    ),
                ),
            ),
        )
        for _ in range(4):
            _insert_row(be, "hits", ("evt", "n"), ("click", 1), upsert=upsert)
        rows = _scan(be, "hits")
        # First insert: n=1; three subsequent: n=2, 3, 4.
        assert rows == [{"evt": "click", "n": 4}]
