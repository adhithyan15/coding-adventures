"""InMemorySchemaProvider — dict-backed schema for planner tests."""

from __future__ import annotations

import pytest

from sql_planner import InMemorySchemaProvider, SchemaProvider, UnknownTable


class TestProtocolConformance:
    def test_is_schema_provider(self) -> None:
        sp = InMemorySchemaProvider({"t": ["a", "b"]})
        assert isinstance(sp, SchemaProvider)


class TestColumns:
    def test_returns_columns(self) -> None:
        sp = InMemorySchemaProvider({"users": ["id", "name"]})
        assert sp.columns("users") == ["id", "name"]

    def test_unknown_table_raises(self) -> None:
        sp = InMemorySchemaProvider({"users": ["id"]})
        with pytest.raises(UnknownTable) as ei:
            sp.columns("nope")
        assert ei.value.table == "nope"

    def test_returns_copy_not_reference(self) -> None:
        sp = InMemorySchemaProvider({"t": ["a"]})
        got = sp.columns("t")
        got.append("mutated")
        assert sp.columns("t") == ["a"]


class TestDefensiveCopy:
    def test_external_mutation_does_not_affect_provider(self) -> None:
        original = {"t": ["a", "b"]}
        sp = InMemorySchemaProvider(original)
        original["t"].append("c")
        assert sp.columns("t") == ["a", "b"]


class TestTables:
    def test_lists_tables(self) -> None:
        sp = InMemorySchemaProvider({"a": [], "b": []})
        assert set(sp.tables()) == {"a", "b"}
