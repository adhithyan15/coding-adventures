"""Tests for shared Prolog operator and directive model objects."""

from __future__ import annotations

import pytest
from logic_engine import atom, logic_list, program, relation, solve_all, term, var

from prolog_core import (
    __version__,
    apply_op_directive,
    apply_predicate_directive,
    directive,
    empty_operator_table,
    empty_predicate_registry,
    expand_dcg_clause,
    expand_dcg_phrase,
    goal_expansion_from_directive,
    iso_operator_table,
    module_import_from_directive,
    module_spec_from_directive,
    swi_operator_table,
    term_expansion_from_directive,
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
        assert iso.get("#=", "xfx") is None
        assert swi.get("#=", "xfx") is not None
        assert swi.get("in", "xfx") is not None
        assert swi.get("ins", "xfx") is not None
        assert swi.get("..", "xfx") is not None

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


class TestPredicateRegistry:
    """Predicate directive helpers should preserve frontend predicate metadata."""

    def test_apply_predicate_directive_tracks_properties_and_initialization(
        self,
    ) -> None:
        registry = empty_predicate_registry()
        registry = apply_predicate_directive(
            registry,
            directive(relation("dynamic", 1)(term("/", "parent", 2))),
        )
        registry = apply_predicate_directive(
            registry,
            directive(
                relation("multifile", 1)(
                    term(
                        ".",
                        term("/", "parent", 2),
                        term(".", term("/", "helper", 1), "[]"),
                    ),
                ),
            ),
        )
        registry = apply_predicate_directive(
            registry,
            directive(relation("initialization", 1)("main")),
        )

        parent = registry.get("parent", 2)
        helper = registry.get("helper", 1)

        assert parent is not None
        assert helper is not None
        assert parent.dynamic is True
        assert parent.multifile is True
        assert helper.multifile is True
        assert registry.initialization_directives[0].term == term(
            "initialization",
            "main",
        )

    def test_apply_predicate_directive_tracks_term_and_goal_expansions(
        self,
    ) -> None:
        registry = empty_predicate_registry()
        registry = apply_predicate_directive(
            registry,
            directive(
                relation("term_expansion", 2)(
                    term("edge", "X", "Y"),
                    term("link", "X", "Y"),
                ),
            ),
        )
        registry = apply_predicate_directive(
            registry,
            directive(relation("goal_expansion", 2)("run", "main")),
        )

        assert registry.term_expansions[0].pattern == term("edge", "X", "Y")
        assert registry.term_expansions[0].expansion == term("link", "X", "Y")
        assert registry.goal_expansions[0].pattern == atom("run")
        assert registry.goal_expansions[0].expansion == atom("main")

    def test_apply_predicate_directive_rejects_invalid_predicate_indicators(
        self,
    ) -> None:
        with pytest.raises(TypeError, match="predicate indicator or proper list"):
            apply_predicate_directive(
                empty_predicate_registry(),
                directive(relation("dynamic", 1)("parent")),
            )

        with pytest.raises(TypeError, match="contain predicate indicators"):
            apply_predicate_directive(
                empty_predicate_registry(),
                directive(
                    relation("dynamic", 1)(
                        term(".", term("/", "parent", 2), term(".", "oops", "[]")),
                    ),
                ),
            )


class TestDcgExpansion:
    """DCG helpers should lower grammar rules into ordinary executable clauses."""

    def test_expand_dcg_clause_supports_terminals(self) -> None:
        clause = expand_dcg_clause(
            term("letters"),
            term(",", logic_list(["a"]), logic_list(["b"])),
        )
        input_var = var("Input")

        assert solve_all(
            program(clause),
            input_var,
            relation("letters", 2)(input_var, atom("[]")),
        ) == [logic_list(["a", "b"])]

    def test_expand_dcg_clause_supports_braced_goals_and_disjunction(self) -> None:
        symbol = var("Symbol")
        clause = expand_dcg_clause(
            term("pick", symbol),
            term(
                ",",
                term(
                    "{}",
                    term(";", term("=", symbol, "a"), term("=", symbol, "b")),
                ),
                logic_list([symbol]),
            ),
        )
        result_var = var("Result")

        assert solve_all(
            program(clause),
            result_var,
            relation("pick", 3)(result_var, logic_list(["b"]), atom("[]")),
        ) == [atom("b")]

    def test_expand_dcg_phrase_appends_state_arguments(self) -> None:
        assert expand_dcg_phrase(term("letters"), logic_list(["a", "b"])) == term(
            "letters",
            logic_list(["a", "b"]),
            atom("[]"),
        )

        assert expand_dcg_phrase(
            term("letters"),
            logic_list(["a"]),
            logic_list(["b"]),
        ) == term("letters", logic_list(["a"]), logic_list(["b"]))


class TestModules:
    """Module and import directive helpers should preserve shared metadata."""

    def test_module_spec_from_directive_parses_exports(self) -> None:
        parsed = module_spec_from_directive(
            directive(
                relation("module", 2)(
                    "family",
                    term(
                        ".",
                        term("/", "parent", 2),
                        term(
                            ".",
                            term("op", 500, "yfx", "++"),
                            term(".", term("/", "ancestor", 2), "[]"),
                        ),
                    ),
                ),
            ),
        )

        assert parsed is not None
        assert parsed.name.name == "family"
        assert [str(export) for export in parsed.exports] == ["parent/2", "ancestor/2"]
        assert str(parsed.exported_operators[0].symbol) == "++"

    def test_module_import_from_directive_parses_import_lists(self) -> None:
        parsed = module_import_from_directive(
            directive(
                relation("use_module", 2)(
                    "family",
                    term(".", term("/", "ancestor", 2), "[]"),
                ),
            ),
        )

        assert parsed is not None
        assert parsed.module_name.name == "family"
        assert parsed.import_all is False
        assert [str(imported) for imported in parsed.imports] == ["ancestor/2"]

    def test_module_import_from_directive_supports_import_all(self) -> None:
        parsed = module_import_from_directive(
            directive(relation("use_module", 1)("family")),
        )

        assert parsed is not None
        assert parsed.module_name.name == "family"
        assert parsed.import_all is True

    def test_module_spec_rejects_invalid_exports(self) -> None:
        with pytest.raises(TypeError, match="export lists may only contain"):
            module_spec_from_directive(
                directive(
                    relation("module", 2)(
                        "family",
                        term(".", "oops", "[]"),
                    ),
                ),
            )


class TestExpansions:
    """Expansion helpers should retain structured pattern/replacement terms."""

    def test_term_expansion_from_directive_parses_pattern_and_replacement(self) -> None:
        parsed = term_expansion_from_directive(
            directive(
                relation("term_expansion", 2)(
                    term("edge", "X", "Y"),
                    term("link", "X", "Y"),
                ),
                {"X": var("X"), "Y": var("Y")},
            ),
        )

        assert parsed is not None
        assert parsed.pattern == term("edge", "X", "Y")
        assert parsed.expansion == term("link", "X", "Y")
        assert sorted(parsed.variables) == ["X", "Y"]

    def test_goal_expansion_from_directive_parses_pattern_and_replacement(self) -> None:
        parsed = goal_expansion_from_directive(
            directive(
                relation("goal_expansion", 2)(
                    term("once", "Goal"),
                    term("call", "Goal"),
                ),
                {"Goal": var("Goal")},
            ),
        )

        assert parsed is not None
        assert parsed.pattern == term("once", "Goal")
        assert parsed.expansion == term("call", "Goal")

    def test_term_expansion_from_directive_rejects_wrong_arity(self) -> None:
        with pytest.raises(ValueError, match=r"term_expansion/2 directives require"):
            term_expansion_from_directive(
                directive(relation("term_expansion", 1)("edge")),
            )
