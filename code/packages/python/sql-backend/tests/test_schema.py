"""Tests for ColumnDef and the NO_DEFAULT sentinel."""

from __future__ import annotations

from sql_backend.schema import NO_DEFAULT, ColumnDef


class TestNoDefaultSentinel:
    def test_singleton(self) -> None:
        # The sentinel is supposed to be a unique identity — comparisons use ``is``.
        from sql_backend.schema import _NoDefault

        assert NO_DEFAULT is _NoDefault()

    def test_falsy(self) -> None:
        # Lets callers write ``if col.default:`` as a shortcut — but code
        # that cares about NULL-as-default must use ``has_default`` instead.
        assert not NO_DEFAULT

    def test_repr(self) -> None:
        assert repr(NO_DEFAULT) == "NO_DEFAULT"


class TestColumnDef:
    def test_defaults(self) -> None:
        c = ColumnDef(name="x", type_name="INTEGER")
        assert not c.not_null
        assert not c.primary_key
        assert not c.unique
        assert not c.has_default()

    def test_explicit_default_none_is_still_a_default(self) -> None:
        # Writing ``default=None`` means "DEFAULT NULL" — a real SQL default.
        c = ColumnDef(name="x", type_name="INTEGER", default=None)
        assert c.has_default()
        assert c.default is None

    def test_explicit_default_value(self) -> None:
        c = ColumnDef(name="x", type_name="INTEGER", default=7)
        assert c.has_default()
        assert c.default == 7

    def test_primary_key_implies_not_null(self) -> None:
        c = ColumnDef(name="id", type_name="INTEGER", primary_key=True)
        assert c.effective_not_null()
        assert c.effective_unique()

    def test_explicit_not_null(self) -> None:
        c = ColumnDef(name="x", type_name="TEXT", not_null=True)
        assert c.effective_not_null()
        assert not c.effective_unique()

    def test_explicit_unique(self) -> None:
        c = ColumnDef(name="email", type_name="TEXT", unique=True)
        assert c.effective_unique()
        assert not c.effective_not_null()

    def test_equality(self) -> None:
        a = ColumnDef(name="x", type_name="INTEGER")
        b = ColumnDef(name="x", type_name="INTEGER")
        assert a == b
