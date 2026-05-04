"""LEFT OUTER JOIN execution tests.

These tests build plans directly (no SQL parser) and exercise the full
codegen → VM pipeline so that JoinBeginRow / JoinSetMatched / JoinIfMatched
semantics are verified end-to-end.

Null-padding relies on the VM returning NULL for any LoadColumn when the
cursor has no current row (right cursor is closed after the inner scan).
"""

from __future__ import annotations

import pytest
from sql_backend.in_memory import InMemoryBackend
from sql_backend.schema import ColumnDef
from sql_codegen import compile
from sql_planner import (
    BinaryExpr,
    BinaryOp,
    Column,
    Filter,
    Join,
    JoinKind,
    Literal,
    Project,
    ProjectionItem,
    Scan,
)

from sql_vm import execute

# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture
def two_table_backend() -> InMemoryBackend:
    """Two related tables: customers (3 rows) and orders (2 rows).

    customers:  id=1 Alice, id=2 Bob, id=3 Carol
    orders:     customer_id=1 (alice_order), customer_id=2 (bob_order)

    A LEFT JOIN customers → orders should yield:
        (1, Alice, alice_order)
        (2, Bob,   bob_order)
        (3, Carol, NULL)        ← unmatched row
    """
    be = InMemoryBackend()
    be.create_table(
        "customers",
        [
            ColumnDef(name="id", type_name="INTEGER", primary_key=True),
            ColumnDef(name="name", type_name="TEXT"),
        ],
        False,
    )
    be.create_table(
        "orders",
        [
            ColumnDef(name="order_id", type_name="INTEGER", primary_key=True),
            ColumnDef(name="customer_id", type_name="INTEGER"),
            ColumnDef(name="product", type_name="TEXT"),
        ],
        False,
    )
    for row in [
        {"id": 1, "name": "Alice"},
        {"id": 2, "name": "Bob"},
        {"id": 3, "name": "Carol"},
    ]:
        be.insert("customers", row)
    for row in [
        {"order_id": 10, "customer_id": 1, "product": "alice_order"},
        {"order_id": 20, "customer_id": 2, "product": "bob_order"},
    ]:
        be.insert("orders", row)
    return be


def _left_join_plan(
    left_table: str,
    right_table: str,
    on_left_col: str,
    on_right_col: str,
    select_cols: list[tuple[str, str, str]],  # (alias, col, out_name)
    where_predicate: object = None,
) -> object:
    """Helper: build a Project(Filter?(Join(Scan,Scan,LEFT))) plan."""
    condition = BinaryExpr(
        op=BinaryOp.EQ,
        left=Column(left_table, on_left_col),
        right=Column(right_table, on_right_col),
    )
    join_node = Join(
        left=Scan(table=left_table, alias=left_table),
        right=Scan(table=right_table, alias=right_table),
        kind=JoinKind.LEFT,
        condition=condition,
    )
    source = (
        Filter(input=join_node, predicate=where_predicate)
        if where_predicate is not None
        else join_node
    )
    items = tuple(
        ProjectionItem(expr=Column(alias, col), alias=out_name)
        for alias, col, out_name in select_cols
    )
    return Project(input=source, items=items)


# ---------------------------------------------------------------------------
# Core semantics
# ---------------------------------------------------------------------------


def test_left_join_all_match(two_table_backend: InMemoryBackend) -> None:
    """When every left row has a matching right row the result equals INNER JOIN."""
    # Use only customers 1 and 2, both have matching orders.
    be = two_table_backend
    plan = _left_join_plan(
        "customers", "orders", "id", "customer_id",
        [("customers", "name", "name"), ("orders", "product", "product")],
    )
    result = execute(compile(plan), be)
    names = sorted(r[0] for r in result.rows)
    assert names == ["Alice", "Bob", "Carol"]
    # Carol has no order → product should be NULL for her row.
    carol_rows = [r for r in result.rows if r[0] == "Carol"]
    assert len(carol_rows) == 1
    assert carol_rows[0][1] is None


def test_left_join_no_match_right_empty(two_table_backend: InMemoryBackend) -> None:
    """Right table is empty: every left row should appear with NULL right cols."""
    be = InMemoryBackend()
    be.create_table("a", [ColumnDef(name="x", type_name="INTEGER")], False)
    be.create_table(
        "b",
        [ColumnDef(name="x", type_name="INTEGER"), ColumnDef(name="y", type_name="TEXT")],
        False,
    )
    be.insert("a", {"x": 1})
    be.insert("a", {"x": 2})
    # b is empty
    plan = _left_join_plan("a", "b", "x", "x", [("a", "x", "ax"), ("b", "y", "by")])
    result = execute(compile(plan), be)
    assert len(result.rows) == 2
    for row in result.rows:
        assert row[1] is None  # b.y is NULL for all unmatched rows


def test_left_join_partial_match(two_table_backend: InMemoryBackend) -> None:
    """Some left rows match, some don't.  Unmatched rows have NULL right cols."""
    result = execute(
        compile(
            _left_join_plan(
                "customers", "orders", "id", "customer_id",
                [("customers", "id", "cid"), ("orders", "product", "product")],
            )
        ),
        two_table_backend,
    )
    rows_by_cid = {r[0]: r[1] for r in result.rows}
    assert rows_by_cid[1] == "alice_order"
    assert rows_by_cid[2] == "bob_order"
    assert rows_by_cid[3] is None     # Carol has no order


def test_left_join_multiple_right_matches(two_table_backend: InMemoryBackend) -> None:
    """A left row matching multiple right rows must appear once per match (not null-padded)."""
    be = two_table_backend
    be.insert("orders", {"order_id": 30, "customer_id": 1, "product": "alice_order_2"})
    result = execute(
        compile(
            _left_join_plan(
                "customers", "orders", "id", "customer_id",
                [("customers", "name", "name"), ("orders", "product", "product")],
            )
        ),
        be,
    )
    alice_rows = sorted(r[1] for r in result.rows if r[0] == "Alice")
    assert alice_rows == ["alice_order", "alice_order_2"]
    # Carol still has NULL.
    carol_rows = [r for r in result.rows if r[0] == "Carol"]
    assert len(carol_rows) == 1
    assert carol_rows[0][1] is None


# ---------------------------------------------------------------------------
# WHERE predicate on the join result
# ---------------------------------------------------------------------------


def test_left_join_where_on_right_null(two_table_backend: InMemoryBackend) -> None:
    """WHERE product IS NULL selects only rows with no right match."""
    from sql_planner import IsNull

    plan = _left_join_plan(
        "customers", "orders", "id", "customer_id",
        [("customers", "name", "name"), ("orders", "product", "product")],
        where_predicate=IsNull(operand=Column("orders", "product")),
    )
    result = execute(compile(plan), two_table_backend)
    assert len(result.rows) == 1
    assert result.rows[0][0] == "Carol"
    assert result.rows[0][1] is None


def test_left_join_where_on_left_col(two_table_backend: InMemoryBackend) -> None:
    """WHERE on a left-side column filters before AND after join match."""
    plan = _left_join_plan(
        "customers", "orders", "id", "customer_id",
        [("customers", "name", "name"), ("orders", "product", "product")],
        where_predicate=BinaryExpr(
            op=BinaryOp.EQ,
            left=Column("customers", "name"),
            right=Literal("Alice"),
        ),
    )
    result = execute(compile(plan), two_table_backend)
    assert len(result.rows) == 1
    assert result.rows[0][0] == "Alice"
    assert result.rows[0][1] == "alice_order"


# ---------------------------------------------------------------------------
# Nested LEFT JOINs (three tables)
# ---------------------------------------------------------------------------


def test_nested_left_join(two_table_backend: InMemoryBackend) -> None:
    """Three-table chained LEFT JOIN: join_match_stack nesting must work correctly.

    customers LEFT JOIN orders LEFT JOIN shipments
    Carol has no order → (Carol, NULL, NULL)
    Alice's order has no shipment → (Alice, alice_order, NULL)
    Bob's order has a shipment → (Bob, bob_order, shipped)
    """
    be = two_table_backend
    be.create_table(
        "shipments",
        [
            ColumnDef(name="ship_id", type_name="INTEGER", primary_key=True),
            ColumnDef(name="order_id", type_name="INTEGER"),
            ColumnDef(name="status", type_name="TEXT"),
        ],
        False,
    )
    be.insert("shipments", {"ship_id": 100, "order_id": 20, "status": "shipped"})
    # orders: 10 → Alice, 20 → Bob; shipments: 100 → order 20 (Bob)

    # Plan: (customers LEFT JOIN orders) LEFT JOIN shipments
    inner_join = Join(
        left=Scan(table="customers", alias="customers"),
        right=Scan(table="orders", alias="orders"),
        kind=JoinKind.LEFT,
        condition=BinaryExpr(
            op=BinaryOp.EQ,
            left=Column("customers", "id"),
            right=Column("orders", "customer_id"),
        ),
    )
    outer_join = Join(
        left=inner_join,
        right=Scan(table="shipments", alias="shipments"),
        kind=JoinKind.LEFT,
        condition=BinaryExpr(
            op=BinaryOp.EQ,
            left=Column("orders", "order_id"),
            right=Column("shipments", "order_id"),
        ),
    )
    plan = Project(
        input=outer_join,
        items=(
            ProjectionItem(expr=Column("customers", "name"), alias="name"),
            ProjectionItem(expr=Column("orders", "product"), alias="product"),
            ProjectionItem(expr=Column("shipments", "status"), alias="status"),
        ),
    )
    result = execute(compile(plan), be)
    by_name = {r[0]: r for r in result.rows}
    assert by_name["Alice"] == ("Alice", "alice_order", None)
    assert by_name["Bob"] == ("Bob", "bob_order", "shipped")
    assert by_name["Carol"] == ("Carol", None, None)


# ---------------------------------------------------------------------------
# RIGHT OUTER JOIN
# ---------------------------------------------------------------------------


def test_right_join_partial_match(two_table_backend: InMemoryBackend) -> None:
    """RIGHT JOIN: all right rows appear; left columns are NULL for unmatched.

    customers: 1 Alice, 2 Bob, 3 Carol
    orders: customer_id=1 alice_order, customer_id=2 bob_order

    RIGHT JOIN orders → customers:
        alice_order (1, Alice)   — matched
        bob_order   (2, Bob)     — matched

    If we add an order with customer_id=99 (no such customer):
        mystery_order (NULL, NULL) — unmatched right row
    """
    be = two_table_backend
    be.insert("orders", {"order_id": 30, "customer_id": 99, "product": "mystery_order"})

    plan = Project(
        input=Join(
            left=Scan(table="customers", alias="customers"),
            right=Scan(table="orders", alias="orders"),
            kind=JoinKind.RIGHT,
            condition=BinaryExpr(
                op=BinaryOp.EQ,
                left=Column("customers", "id"),
                right=Column("orders", "customer_id"),
            ),
        ),
        items=(
            ProjectionItem(expr=Column("customers", "name"), alias="name"),
            ProjectionItem(expr=Column("orders", "product"), alias="product"),
        ),
    )
    result = execute(compile(plan), be)
    by_product = {r[1]: r[0] for r in result.rows}
    assert by_product["alice_order"] == "Alice"
    assert by_product["bob_order"] == "Bob"
    assert by_product["mystery_order"] is None   # no matching customer


def test_right_join_left_empty(two_table_backend: InMemoryBackend) -> None:
    """RIGHT JOIN with empty left table: all right rows appear with NULL left cols."""
    be = InMemoryBackend()
    be.create_table("a", [ColumnDef(name="x", type_name="INTEGER")], False)
    be.create_table(
        "b",
        [ColumnDef(name="x", type_name="INTEGER"), ColumnDef(name="y", type_name="TEXT")],
        False,
    )
    be.insert("b", {"x": 10, "y": "hello"})
    be.insert("b", {"x": 20, "y": "world"})
    # a is empty

    plan = Project(
        input=Join(
            left=Scan(table="a", alias="a"),
            right=Scan(table="b", alias="b"),
            kind=JoinKind.RIGHT,
            condition=BinaryExpr(
                op=BinaryOp.EQ,
                left=Column("a", "x"),
                right=Column("b", "x"),
            ),
        ),
        items=(
            ProjectionItem(expr=Column("a", "x"), alias="ax"),
            ProjectionItem(expr=Column("b", "y"), alias="by"),
        ),
    )
    result = execute(compile(plan), be)
    assert len(result.rows) == 2
    for row in result.rows:
        assert row[0] is None   # a.x is NULL for all unmatched rows
    by_values = {r[1] for r in result.rows}
    assert by_values == {"hello", "world"}
