"""Tests for the SqlValue helpers."""

from __future__ import annotations

import pytest

from sql_backend.values import is_sql_value, sql_type_name


class TestSqlTypeName:
    def test_null(self) -> None:
        assert sql_type_name(None) == "NULL"

    def test_bool_before_int(self) -> None:
        # Critical: bool is a subclass of int in Python, so the check must
        # happen in the right order.
        assert sql_type_name(True) == "BOOLEAN"
        assert sql_type_name(False) == "BOOLEAN"

    def test_int(self) -> None:
        assert sql_type_name(42) == "INTEGER"
        assert sql_type_name(-1) == "INTEGER"
        assert sql_type_name(0) == "INTEGER"

    def test_float(self) -> None:
        assert sql_type_name(1.5) == "REAL"
        assert sql_type_name(0.0) == "REAL"

    def test_text(self) -> None:
        assert sql_type_name("hello") == "TEXT"
        assert sql_type_name("") == "TEXT"

    def test_rejects_non_sql_types(self) -> None:
        with pytest.raises(TypeError):
            sql_type_name([1, 2, 3])  # type: ignore[arg-type]
        with pytest.raises(TypeError):
            sql_type_name({"a": 1})  # type: ignore[arg-type]


class TestIsSqlValue:
    def test_accepts_all_variants(self) -> None:
        assert is_sql_value(None)
        assert is_sql_value(True)
        assert is_sql_value(1)
        assert is_sql_value(1.5)
        assert is_sql_value("text")

    def test_rejects_others(self) -> None:
        assert not is_sql_value([1])
        assert not is_sql_value({"k": 1})
        assert not is_sql_value(object())
