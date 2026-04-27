"""Tests for backend_as_schema_provider and SchemaProvider wiring."""

from __future__ import annotations

from sql_backend.backend import backend_as_schema_provider
from sql_backend.conformance import make_in_memory_users


class TestSchemaProviderAdapter:
    def test_returns_column_names(self) -> None:
        b = make_in_memory_users()
        sp = backend_as_schema_provider(b)
        assert sp.columns("users") == ["id", "name", "age", "email"]

    def test_propagates_table_not_found(self) -> None:
        import pytest

        from sql_backend.errors import TableNotFound

        b = make_in_memory_users()
        sp = backend_as_schema_provider(b)
        with pytest.raises(TableNotFound):
            sp.columns("missing")
