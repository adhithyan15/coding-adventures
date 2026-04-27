"""DDL planning — CREATE TABLE, DROP TABLE."""

from __future__ import annotations

from sql_backend.schema import ColumnDef

from sql_planner import (
    CreateTableStmt,
    DropTableStmt,
    InMemorySchemaProvider,
    plan,
)
from sql_planner.plan import CreateTable, DropTable


def schema() -> InMemorySchemaProvider:
    return InMemorySchemaProvider({})


class TestCreateTable:
    def test_create_table(self) -> None:
        cols = (
            ColumnDef(name="id", type_name="INTEGER", primary_key=True),
            ColumnDef(name="name", type_name="TEXT", not_null=True),
        )
        ast = CreateTableStmt(table="users", columns=cols)
        p = plan(ast, schema())
        assert isinstance(p, CreateTable)
        assert p.table == "users"
        assert p.columns == cols
        assert p.if_not_exists is False

    def test_create_table_if_not_exists(self) -> None:
        ast = CreateTableStmt(table="t", columns=(), if_not_exists=True)
        p = plan(ast, schema())
        assert isinstance(p, CreateTable)
        assert p.if_not_exists is True


class TestDropTable:
    def test_drop_table(self) -> None:
        ast = DropTableStmt(table="users")
        p = plan(ast, schema())
        assert isinstance(p, DropTable)
        assert p.table == "users"
        assert p.if_exists is False

    def test_drop_table_if_exists(self) -> None:
        ast = DropTableStmt(table="t", if_exists=True)
        p = plan(ast, schema())
        assert isinstance(p, DropTable)
        assert p.if_exists is True
