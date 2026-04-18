"""Tests for symbol-core.

The symbol layer is intentionally tiny, so the tests lean into behavior rather
than surface area: interning, validation, namespace handling, and friendly
representations.
"""

from __future__ import annotations

import pytest

from symbol_core import (
    Symbol,
    SymbolError,
    SymbolTable,
    __version__,
    is_symbol,
    sym,
)


class TestVersion:
    """Verify the package is importable and versioned."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.1.0"


class TestSymbolTable:
    """Exercise the interning guarantees of the symbol table."""

    def test_intern_returns_same_object_for_equal_names(self) -> None:
        table = SymbolTable()

        left = table.intern("parent")
        right = table.intern("parent")

        assert left == right
        assert left is right
        assert table.size() == 1

    def test_case_is_preserved_and_significant(self) -> None:
        table = SymbolTable()

        lower = table.intern("x")
        upper = table.intern("X")

        assert lower != upper
        assert table.size() == 2

    def test_namespaces_are_part_of_identity(self) -> None:
        table = SymbolTable()

        plain = table.intern("sin")
        math = table.intern("sin", namespace="math")

        assert plain != math
        assert str(plain) == "sin"
        assert str(math) == "math:sin"

    def test_contains_and_len_follow_interned_state(self) -> None:
        table = SymbolTable()

        assert table.contains("foo") is False
        assert len(table) == 0

        table.intern("foo")

        assert table.contains("foo") is True
        assert len(table) == 1

    def test_tables_are_isolated_from_each_other(self) -> None:
        first = SymbolTable()
        second = SymbolTable()

        left = first.intern("parent")
        right = second.intern("parent")

        assert left == right
        assert left is not right


class TestValidation:
    """Reject malformed symbol names instead of normalizing them silently."""

    @pytest.mark.parametrize(
        ("name", "namespace"),
        [
            ("", None),
            (" x", None),
            ("x ", None),
            ("x", ""),
            ("x", " math"),
            (123, None),
        ],
    )
    def test_invalid_parts_raise_symbol_error(
        self,
        name: object,
        namespace: object,
    ) -> None:
        table = SymbolTable()

        with pytest.raises(SymbolError):
            table.intern(name, namespace=namespace)


class TestHelpers:
    """Verify the convenience API exported from the package root."""

    def test_sym_uses_the_default_table(self) -> None:
        first = sym("ancestor")
        second = sym("ancestor")

        assert first is second

    def test_sym_can_target_an_explicit_table(self) -> None:
        table = SymbolTable()

        value = sym("edge", table=table)

        assert table.contains("edge") is True
        assert value is table.intern("edge")

    def test_is_symbol_identifies_symbol_instances(self) -> None:
        value = sym("node")

        assert is_symbol(value) is True
        assert is_symbol("node") is False

    def test_repr_is_readable(self) -> None:
        plain = Symbol(namespace=None, name="x")
        qualified = Symbol(namespace="math", name="sin")

        assert repr(plain) == "Symbol(name='x')"
        assert repr(qualified) == "Symbol(namespace='math', name='sin')"
