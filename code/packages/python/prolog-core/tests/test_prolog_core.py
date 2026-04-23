"""Tests for shared Prolog operator and directive model objects."""

from __future__ import annotations

import pytest
from logic_engine import relation, term, var

from prolog_core import (
    __version__,
    apply_op_directive,
    directive,
    empty_operator_table,
    iso_operator_table,
    swi_operator_table,
)


class TestVersion:
    """Verify the package is importable and versioned."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.1.0"


class TestOperatorTable:
    """Operator tables should support define, lookup, and dialect defaults."""

    def test_define_and_remove_operator(self) -> None:
        table = empty_operator_table().define(500, "yfx", "+", "-")

        assert table.get("+", "yfx") is not None
        assert table.get("-", "yfx") is not None

        removed = table.define(0, "yfx", "+")

        assert removed.get("+", "yfx") is None
        assert removed.get("-", "yfx") is not None

    def test_iso_and_swi_defaults_differ(self) -> None:
        iso = iso_operator_table()
        swi = swi_operator_table()

        assert iso.get(":-", "xfx") is not None
        assert iso.get(":", "xfy") is None
        assert swi.get(":", "xfy") is not None

    def test_apply_op_directive_adds_and_removes_operator(self) -> None:
        table = empty_operator_table()

        with_operator = apply_op_directive(table, term("op", 500, "yfx", "++"))
        removed = apply_op_directive(with_operator, term("op", 0, "yfx", "++"))

        assert with_operator.get("++", "yfx") is not None
        assert removed.get("++", "yfx") is None

    def test_apply_op_directive_accepts_lists_of_names(self) -> None:
        updated = apply_op_directive(
            empty_operator_table(),
            term("op", 400, "yfx", term(".", "+", term(".", "-", "[]"))),
        )

        assert updated.get("+", "yfx") is not None
        assert updated.get("-", "yfx") is not None

    def test_apply_op_directive_rejects_invalid_arguments(self) -> None:
        with pytest.raises(TypeError, match="associativity must be an atom"):
            apply_op_directive(empty_operator_table(), term("op", 500, 1, "++"))

        with pytest.raises(TypeError, match="contain atoms"):
            apply_op_directive(
                empty_operator_table(),
                term("op", 500, "yfx", term(".", 1, "[]")),
            )


class TestDirective:
    """Directive helpers should retain both goal and term views."""

    def test_directive_carries_goal_term_and_relation(self) -> None:
        main = relation("initialization", 1)("main")
        parsed = directive(main, {"X": var("X")})

        assert str(parsed.term) == "initialization(main)"
        assert parsed.relation is not None
        assert str(parsed.relation) == "initialization/1"
        assert "X" in parsed.variables
