"""Adapter — ASTNode → Statement translation."""

import pytest
from sql_parser import parse_sql
from sql_planner import (
    AggFunc,
    AggregateExpr,
    Assignment,
    BinaryOp,
    Column,
    CreateTableStmt,
    DeleteStmt,
    DropTableStmt,
    InsertValuesStmt,
    JoinKind,
    Literal,
    SelectStmt,
    UpdateStmt,
)

import mini_sqlite
from mini_sqlite.adapter import to_statement


def adapt(sql):
    return to_statement(parse_sql(sql))


# ----------------------------------------------------------------------
# SELECT variants.
# ----------------------------------------------------------------------


def test_select_star():
    stmt = adapt("SELECT * FROM t")
    assert isinstance(stmt, SelectStmt)
    assert stmt.from_.table == "t"
    assert len(stmt.items) == 1


def test_select_columns_and_alias():
    stmt = adapt("SELECT a, b AS bee FROM t")
    assert stmt.items[0].expr == Column(table=None, col="a")
    assert stmt.items[1].alias == "bee"


def test_select_qualified_column():
    stmt = adapt("SELECT t.a FROM t")
    assert stmt.items[0].expr == Column(table="t", col="a")


def test_select_table_alias():
    stmt = adapt("SELECT * FROM t AS u")
    assert stmt.from_.alias == "u"


def test_select_distinct():
    assert adapt("SELECT DISTINCT a FROM t").distinct is True


def test_select_where():
    stmt = adapt("SELECT * FROM t WHERE a > 5")
    assert stmt.where is not None
    assert stmt.where.op == BinaryOp.GT


def test_select_order_by():
    stmt = adapt("SELECT * FROM t ORDER BY a DESC")
    assert stmt.order_by[0].descending is True


def test_select_limit_offset():
    stmt = adapt("SELECT * FROM t LIMIT 10 OFFSET 3")
    assert stmt.limit.count == 10
    assert stmt.limit.offset == 3


def test_select_group_by_having():
    stmt = adapt("SELECT dept, COUNT(*) FROM t GROUP BY dept HAVING COUNT(*) > 2")
    assert len(stmt.group_by) == 1
    assert stmt.having is not None


def test_select_aggregate_sum():
    stmt = adapt("SELECT SUM(x) FROM t")
    assert isinstance(stmt.items[0].expr, AggregateExpr)
    assert stmt.items[0].expr.func == AggFunc.SUM


def test_select_join_inner():
    stmt = adapt("SELECT * FROM a INNER JOIN b ON a.id = b.id")
    assert stmt.joins[0].kind == JoinKind.INNER


def test_select_join_cross():
    # The SQL grammar in this stack requires ``ON`` even for CROSS JOIN;
    # the adapter still recognises the kind. The predicate is preserved
    # but semantically ignored by the planner.
    stmt = adapt("SELECT * FROM a CROSS JOIN b ON 1 = 1")
    assert stmt.joins[0].kind == JoinKind.CROSS


# ----------------------------------------------------------------------
# Expression tower.
# ----------------------------------------------------------------------


def test_expr_between():
    stmt = adapt("SELECT * FROM t WHERE a BETWEEN 1 AND 10")
    assert stmt.where is not None


def test_expr_not_between():
    stmt = adapt("SELECT * FROM t WHERE a NOT BETWEEN 1 AND 10")
    assert stmt.where is not None


def test_expr_in_list():
    stmt = adapt("SELECT * FROM t WHERE a IN (1, 2, 3)")
    assert stmt.where is not None


def test_expr_not_in():
    stmt = adapt("SELECT * FROM t WHERE a NOT IN (1, 2)")
    assert stmt.where is not None


def test_expr_like():
    stmt = adapt("SELECT * FROM t WHERE a LIKE 'abc%'")
    assert stmt.where is not None


def test_expr_not_like():
    stmt = adapt("SELECT * FROM t WHERE a NOT LIKE 'abc%'")
    assert stmt.where is not None


def test_expr_is_null():
    stmt = adapt("SELECT * FROM t WHERE a IS NULL")
    assert stmt.where is not None


def test_expr_is_not_null():
    stmt = adapt("SELECT * FROM t WHERE a IS NOT NULL")
    assert stmt.where is not None


def test_expr_and_or_not():
    stmt = adapt("SELECT * FROM t WHERE NOT (a = 1 AND b = 2 OR c = 3)")
    assert stmt.where is not None


def test_expr_arithmetic():
    stmt = adapt("SELECT a + b * c - d / e FROM t")
    assert stmt.items[0].expr is not None


def test_expr_unary_neg():
    stmt = adapt("SELECT -a FROM t")
    assert stmt.items[0].expr is not None


def test_expr_literals():
    stmt = adapt("SELECT 1, 1.5, 'x', TRUE, FALSE, NULL FROM t")
    vals = [it.expr for it in stmt.items]
    assert vals[0] == Literal(value=1)
    assert vals[1] == Literal(value=1.5)
    assert vals[2] == Literal(value="x")
    assert vals[3] == Literal(value=True)
    assert vals[4] == Literal(value=False)
    assert vals[5] == Literal(value=None)


def test_expr_function_call():
    stmt = adapt("SELECT abs(x) FROM t")
    assert stmt.items[0].expr is not None


def test_expr_like_requires_string_literal():
    with pytest.raises(mini_sqlite.ProgrammingError):
        adapt("SELECT * FROM t WHERE a LIKE b")


def test_agg_count_star():
    stmt = adapt("SELECT COUNT(*) FROM t")
    assert isinstance(stmt.items[0].expr, AggregateExpr)
    assert stmt.items[0].expr.arg.star is True


def test_agg_wrong_arity():
    with pytest.raises(mini_sqlite.ProgrammingError):
        adapt("SELECT COUNT() FROM t")


# ----------------------------------------------------------------------
# INSERT / UPDATE / DELETE.
# ----------------------------------------------------------------------


def test_insert_with_columns():
    stmt = adapt("INSERT INTO t (a, b) VALUES (1, 2)")
    assert isinstance(stmt, InsertValuesStmt)
    assert stmt.columns == ("a", "b")
    assert len(stmt.rows) == 1


def test_insert_multiple_rows():
    stmt = adapt("INSERT INTO t (a) VALUES (1), (2), (3)")
    assert len(stmt.rows) == 3


def test_insert_without_columns():
    stmt = adapt("INSERT INTO t VALUES (1, 2)")
    assert stmt.columns is None


def test_update_single_assignment():
    stmt = adapt("UPDATE t SET a = 1")
    assert isinstance(stmt, UpdateStmt)
    assert stmt.assignments[0] == Assignment(column="a", value=Literal(value=1))


def test_update_multi_assignment_where():
    stmt = adapt("UPDATE t SET a = 1, b = 2 WHERE c = 3")
    assert len(stmt.assignments) == 2
    assert stmt.where is not None


def test_delete_all():
    stmt = adapt("DELETE FROM t")
    assert isinstance(stmt, DeleteStmt)
    assert stmt.where is None


def test_delete_where():
    stmt = adapt("DELETE FROM t WHERE id = 1")
    assert stmt.where is not None


# ----------------------------------------------------------------------
# DDL.
# ----------------------------------------------------------------------


def test_create_table_basic():
    stmt = adapt("CREATE TABLE t (a INTEGER, b TEXT)")
    assert isinstance(stmt, CreateTableStmt)
    assert stmt.table == "t"
    assert len(stmt.columns) == 2


def test_create_table_if_not_exists():
    stmt = adapt("CREATE TABLE IF NOT EXISTS t (a INTEGER)")
    assert stmt.if_not_exists is True


def test_create_table_primary_key_implies_not_null():
    stmt = adapt("CREATE TABLE t (id INTEGER PRIMARY KEY)")
    col = stmt.columns[0]
    assert col.primary_key is True
    assert col.not_null is True


def test_create_table_not_null_unique():
    stmt = adapt("CREATE TABLE t (a TEXT NOT NULL, b TEXT UNIQUE)")
    assert stmt.columns[0].not_null is True
    assert stmt.columns[1].unique is True


def test_drop_table():
    stmt = adapt("DROP TABLE t")
    assert isinstance(stmt, DropTableStmt)
    assert stmt.if_exists is False


def test_drop_table_if_exists():
    stmt = adapt("DROP TABLE IF EXISTS t")
    assert stmt.if_exists is True
