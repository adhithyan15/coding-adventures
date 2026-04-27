"""Tests for the error hierarchy."""

from __future__ import annotations

import pytest

from sql_backend.errors import (
    BackendError,
    ColumnNotFound,
    ConstraintViolation,
    Internal,
    TableAlreadyExists,
    TableNotFound,
    Unsupported,
)


class TestErrorHierarchy:
    @pytest.mark.parametrize(
        "error",
        [
            TableNotFound(table="t"),
            TableAlreadyExists(table="t"),
            ColumnNotFound(table="t", column="c"),
            ConstraintViolation(table="t", column="c", message="boom"),
            Unsupported(operation="op"),
            Internal(message="boom"),
        ],
    )
    def test_all_subclass_backend_error(self, error: BackendError) -> None:
        assert isinstance(error, BackendError)

    def test_all_subclass_exception(self) -> None:
        # Can be raised and caught as normal exceptions.
        with pytest.raises(BackendError):
            raise TableNotFound(table="x")


class TestEquality:
    def test_equal_dataclasses(self) -> None:
        assert TableNotFound(table="u") == TableNotFound(table="u")
        assert TableNotFound(table="u") != TableNotFound(table="v")

    def test_constraint_equality(self) -> None:
        a = ConstraintViolation(table="t", column="c", message="m")
        b = ConstraintViolation(table="t", column="c", message="m")
        assert a == b


class TestStringFormat:
    def test_table_not_found_str(self) -> None:
        assert "users" in str(TableNotFound(table="users"))

    def test_column_not_found_str(self) -> None:
        s = str(ColumnNotFound(table="users", column="age"))
        assert "users" in s and "age" in s

    def test_constraint_str_is_message(self) -> None:
        assert str(ConstraintViolation(table="t", column="c", message="NOT NULL")) == "NOT NULL"

    def test_unsupported_str(self) -> None:
        assert "transactions" in str(Unsupported(operation="transactions"))

    def test_table_exists_str(self) -> None:
        assert "widgets" in str(TableAlreadyExists(table="widgets"))

    def test_internal_str(self) -> None:
        assert str(Internal(message="disk full")) == "disk full"
