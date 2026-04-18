"""Tests for logic-engine.

These tests focus on the layer above ``logic-core``: the parts that make it
possible to write real logic *programs* with named relations, facts, rules, and
recursive resolution.
"""

from __future__ import annotations

import pytest

from logic_engine import (
    Clause,
    DeferredExpr,
    Disequality,
    Program,
    State,
    __version__,
    all_different,
    atom,
    conj,
    defer,
    disj,
    eq,
    fact,
    fail,
    fresh,
    logic_list,
    neq,
    program,
    relation,
    rule,
    solve,
    solve_all,
    solve_n,
    term,
    var,
)


class TestVersion:
    """Verify the package is importable and versioned."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.3.0"


class TestRelationsAndClauses:
    """The public building blocks should validate their shape eagerly."""

    def test_relation_rejects_negative_arity(self) -> None:
        with pytest.raises(ValueError):
            relation("broken", -1)

    def test_relation_enforces_arity(self) -> None:
        parent = relation("parent", 2)

        with pytest.raises(ValueError):
            parent("homer")

    def test_fact_and_rule_require_relation_calls(self) -> None:
        parent = relation("parent", 2)
        x = var("X")
        y = var("Y")

        assert isinstance(fact(parent("homer", "bart")), Clause)
        assert isinstance(rule(parent(x, y), parent(x, y)), Clause)

        with pytest.raises(TypeError):
            fact(x)

        with pytest.raises(TypeError):
            rule(x, parent(x, y))

    def test_program_indexes_clauses_by_relation(self) -> None:
        parent = relation("parent", 2)
        family = program(
            fact(parent("homer", "bart")),
            fact(parent("homer", "lisa")),
        )

        assert isinstance(family, Program)
        assert len(family.clauses_for(parent)) == 2

    def test_program_rejects_non_clause_entries(self) -> None:
        with pytest.raises(TypeError):
            Program(clauses=("not-a-clause",))

    def test_clause_reports_whether_it_is_a_fact(self) -> None:
        parent = relation("parent", 2)
        x = var("X")
        y = var("Y")

        assert fact(parent("homer", "bart")).is_fact()
        assert not rule(parent(x, y), parent(x, y)).is_fact()

    def test_string_forms_are_human_readable(self) -> None:
        parent = relation("parent", 2)
        true_rel = relation("true", 0)

        assert str(parent) == "parent/2"
        assert str(parent("homer", "bart")) == "parent(homer, bart)"
        assert str(true_rel()) == "true"

    def test_term_coercion_rejects_ambiguous_or_unsupported_values(self) -> None:
        parent = relation("parent", 2)

        with pytest.raises(TypeError):
            parent(True, "bart")

        with pytest.raises(TypeError):
            parent(object(), "bart")


class TestSolvingFactsAndRules:
    """Facts and rules should solve end to end."""

    def test_fact_solves_directly(self) -> None:
        parent = relation("parent", 2)
        x = var("X")
        family = program(fact(parent("homer", "bart")))

        assert solve_all(family, x, parent("homer", x)) == [atom("bart")]

    def test_rule_solves_through_body(self) -> None:
        parent = relation("parent", 2)
        child = relation("child", 2)
        x = var("X")
        y = var("Y")
        family = program(
            fact(parent("homer", "bart")),
            rule(child(x, y), parent(y, x)),
        )

        assert solve_all(family, x, child(x, "homer")) == [atom("bart")]

    def test_missing_relation_yields_no_answers(self) -> None:
        parent = relation("parent", 2)
        ancestor = relation("ancestor", 2)
        x = var("X")
        family = program(fact(parent("homer", "bart")))

        assert solve_all(family, x, ancestor("homer", x)) == []


class TestRecursivePrograms:
    """Recursive relations are the real proof that the engine works."""

    def test_ancestor_recursion(self) -> None:
        parent = relation("parent", 2)
        ancestor = relation("ancestor", 2)

        x = var("X")
        y = var("Y")

        family = program(
            fact(parent("abe", "homer")),
            fact(parent("homer", "bart")),
            fact(parent("homer", "lisa")),
            rule(ancestor(x, y), parent(x, y)),
            rule(
                ancestor(x, y),
                fresh(1, lambda z: conj(parent(x, z), ancestor(z, y))),
            ),
        )

        assert solve_all(family, y, ancestor("abe", y)) == [
            atom("homer"),
            atom("bart"),
            atom("lisa"),
        ]

    def test_standardize_apart_prevents_recursive_variable_capture(self) -> None:
        edge = relation("edge", 2)
        path = relation("path", 2)

        x = var("X")
        y = var("Y")

        graph = program(
            fact(edge("a", "b")),
            fact(edge("b", "c")),
            fact(edge("c", "d")),
            rule(path(x, y), edge(x, y)),
            rule(
                path(x, y),
                fresh(1, lambda z: conj(edge(x, z), path(z, y))),
            ),
        )

        assert solve_all(graph, y, path("a", y)) == [
            atom("b"),
            atom("c"),
            atom("d"),
        ]


class TestQueryHelpers:
    """Query helpers should keep the library ergonomic."""

    def test_empty_and_singleton_goal_builders_behave_like_logic_identities(
        self,
    ) -> None:
        x = var("X")
        trivial = program()

        assert solve_all(trivial, x, conj()) == [x]
        assert solve_all(trivial, x, disj()) == []
        assert conj(eq(x, "bart")) == eq(x, "bart")
        assert disj(fail()) == fail()

    def test_tuple_queries_return_multiple_reified_values(self) -> None:
        parent = relation("parent", 2)
        x = var("X")
        y = var("Y")
        family = program(
            fact(parent("homer", "bart")),
            fact(parent("marge", "bart")),
        )

        assert solve_all(family, (x, y), parent(x, y)) == [
            (atom("homer"), atom("bart")),
            (atom("marge"), atom("bart")),
        ]

    def test_solve_n_truncates_answer_streams(self) -> None:
        parent = relation("parent", 2)
        x = var("X")
        family = program(
            fact(parent("homer", "bart")),
            fact(parent("homer", "lisa")),
            fact(parent("homer", "maggie")),
        )

        assert solve_n(family, 2, x, parent("homer", x)) == [
            atom("bart"),
            atom("lisa"),
        ]

    def test_solve_n_rejects_negative_limits(self) -> None:
        parent = relation("parent", 2)
        x = var("X")
        family = program(fact(parent("homer", "bart")))

        with pytest.raises(ValueError):
            solve_n(family, -1, x, parent("homer", x))

    def test_solve_n_supports_tuple_queries(self) -> None:
        parent = relation("parent", 2)
        x = var("X")
        y = var("Y")
        family = program(fact(parent("homer", "bart")))

        assert solve_n(family, 1, (x, y), parent(x, y)) == [
            (atom("homer"), atom("bart")),
        ]

    def test_fresh_requires_at_least_one_variable(self) -> None:
        with pytest.raises(ValueError):
            fresh(0, lambda: conj())

    def test_invalid_goal_expressions_are_rejected(self) -> None:
        with pytest.raises(TypeError):
            solve_all(program(), var("X"), object())

    def test_defer_creates_a_goal_expression(self) -> None:
        deferred = defer(eq, "tea", "tea")

        assert isinstance(deferred, DeferredExpr)


class TestDisequalityConstraints:
    """Disequality is the first real puzzle-oriented constraint."""

    def test_neq_can_live_inside_engine_goals(self) -> None:
        x = var("X")

        assert solve_all(program(), x, conj(neq(x, "homer"), eq(x, "marge"))) == [
            atom("marge"),
        ]

    def test_neq_blocks_later_equal_bindings(self) -> None:
        x = var("X")

        assert solve_all(program(), x, conj(neq(x, "homer"), eq(x, "homer"))) == []

    def test_solve_exposes_pending_constraints_on_states(self) -> None:
        x = var("X")
        states = list(solve(program(), neq(x, "homer")))

        assert len(states) == 1
        assert states[0].constraints == (
            Disequality(left=x, right=atom("homer")),
        )

    def test_all_different_solves_a_small_coloring_problem(self) -> None:
        color = relation("color", 1)
        wa = var("WA")
        nt = var("NT")
        sa = var("SA")

        palette = program(
            fact(color("red")),
            fact(color("green")),
            fact(color("blue")),
        )

        answers = solve_n(
            palette,
            3,
            (wa, nt, sa),
            conj(
                color(wa),
                color(nt),
                color(sa),
                all_different(wa, nt, sa),
            ),
        )

        assert answers == [
            (atom("red"), atom("green"), atom("blue")),
            (atom("red"), atom("blue"), atom("green")),
            (atom("green"), atom("red"), atom("blue")),
        ]


class TestDeferredRecursiveHelpers:
    """Deferred goal builders unlock recursive Python helper libraries."""

    def test_defer_supports_recursive_helper_goals(self) -> None:
        def member_like(item: object, items: object) -> object:
            return fresh(
                2,
                lambda head, tail: disj(
                    eq(items, term(".", item, tail)),
                    conj(
                        eq(items, term(".", head, tail)),
                        defer(member_like, item, tail),
                    ),
                ),
            )

        item = var("Item")

        assert solve_all(
            program(),
            item,
            member_like(item, logic_list(["tea", "cake", "jam"])),
        ) == [atom("tea"), atom("cake"), atom("jam")]


class TestListRelations:
    """List-shaped relations should work with LP00's canonical list terms."""

    def test_member_relation_over_logic_lists(self) -> None:
        member = relation("member", 2)
        x = var("X")
        head = var("Head")
        tail = var("Tail")

        members = program(
            rule(member(x, term(".", x, tail)), conj()),
            rule(member(x, term(".", head, tail)), member(x, tail)),
        )

        values = logic_list(["a", "b", "c"])
        assert solve_all(members, x, member(x, values)) == [
            atom("a"),
            atom("b"),
            atom("c"),
        ]


class TestSolveIterator:
    """The low-level iterator should still be available for advanced callers."""

    def test_solve_returns_states(self) -> None:
        parent = relation("parent", 2)
        x = var("X")
        family = program(fact(parent("homer", "bart")))

        states = list(solve(family, parent("homer", x)))

        assert len(states) == 1
        assert isinstance(states[0], State)
        assert states[0].substitution.reify(x) == atom("bart")
