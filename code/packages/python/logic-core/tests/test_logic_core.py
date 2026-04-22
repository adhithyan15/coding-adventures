"""Tests for logic-core.

The package is intentionally small, so the tests focus on the essential
relational operations: term construction, unification, substitutions, goals,
and search.
"""

from __future__ import annotations

import pytest
from symbol_core import sym

from logic_core import (
    Atom,
    Compound,
    Disequality,
    LogicVar,
    Number,
    State,
    String,
    Substitution,
    __version__,
    atom,
    conj,
    disj,
    eq,
    fail,
    fresh,
    logic_list,
    neq,
    num,
    reify,
    run_all,
    run_n,
    string,
    succeed,
    term,
    unify,
    var,
)


class TestVersion:
    """Verify the package is importable and versioned."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.4.0"


class TestTermConstruction:
    """Term constructors should produce predictable structural values."""

    def test_atom_uses_symbol_backing(self) -> None:
        value = atom("homer")

        assert isinstance(value, Atom)
        assert value.symbol is sym("homer")

    def test_scalar_helpers_construct_distinct_term_kinds(self) -> None:
        assert isinstance(num(7), Number)
        assert isinstance(string("hello"), String)

    def test_term_builds_compounds(self) -> None:
        value = term("parent", atom("homer"), atom("bart"))

        assert isinstance(value, Compound)
        assert value.functor is sym("parent")
        assert value.args == (atom("homer"), atom("bart"))

    def test_logic_list_uses_canonical_cons_representation(self) -> None:
        value = logic_list([atom("a"), atom("b")])

        assert value == term(".", atom("a"), term(".", atom("b"), atom("[]")))

    def test_vars_with_same_name_are_still_distinct(self) -> None:
        left = var("X")
        right = var("X")

        assert isinstance(left, LogicVar)
        assert isinstance(right, LogicVar)
        assert left != right
        assert str(left) == "X"
        assert str(right) == "X"


class TestSubstitutionAndUnification:
    """Unification and substitution are the semantic heart of the package."""

    def test_walk_follows_variable_chains(self) -> None:
        x = var("X")
        y = var("Y")
        substitution = Substitution().extend(x, y).extend(y, atom("homer"))

        assert substitution.walk(x) == atom("homer")

    def test_unify_simple_atom_binding(self) -> None:
        x = var("X")
        result = unify(x, atom("homer"))

        assert result is not None
        assert result.walk(x) == atom("homer")

    def test_unify_compounds_threads_bindings(self) -> None:
        x = var("X")
        goal_left = term("parent", x, atom("bart"))
        goal_right = term("parent", atom("homer"), atom("bart"))

        result = unify(goal_left, goal_right)

        assert result is not None
        assert result.walk(x) == atom("homer")

    def test_unify_rejects_functor_mismatches(self) -> None:
        result = unify(term("f", atom("x")), term("g", atom("x")))

        assert result is None

    def test_unify_rejects_arity_mismatches(self) -> None:
        result = unify(term("f", atom("x")), term("f", atom("x"), atom("y")))

        assert result is None

    def test_occurs_check_rejects_cycles(self) -> None:
        x = var("X")

        assert unify(x, term("f", x)) is None

    def test_reify_recursively_resolves_nested_terms(self) -> None:
        x = var("X")
        y = var("Y")
        substitution = (
            Substitution()
            .extend(x, term("parent", atom("homer"), y))
            .extend(y, atom("bart"))
        )

        assert reify(x, substitution) == term(
            "parent",
            atom("homer"),
            atom("bart"),
        )


class TestGoalsAndSearch:
    """Goals should produce zero, one, or many states through search."""

    def test_succeed_and_fail_are_identity_and_empty_goals(self) -> None:
        x = var("X")

        assert run_all(x, succeed()) == [x]
        assert run_all(x, fail()) == []

    def test_eq_goal_emits_a_bound_answer(self) -> None:
        x = var("X")

        assert run_all(x, eq(x, atom("homer"))) == [atom("homer")]

    def test_neq_fails_on_equal_terms(self) -> None:
        assert run_all(var("X"), neq(atom("homer"), atom("homer"))) == []

    def test_neq_succeeds_immediately_on_obviously_different_terms(self) -> None:
        x = var("X")

        assert run_all(x, neq(atom("homer"), atom("marge"))) == [x]

    def test_neq_records_delayed_constraints_for_open_terms(self) -> None:
        x = var("X")
        states = list(neq(x, atom("homer"))(State()))

        assert len(states) == 1
        assert states[0].substitution == Substitution()
        assert states[0].constraints == (
            Disequality(left=x, right=atom("homer")),
        )

    def test_eq_fails_when_it_would_violate_a_stored_disequality(self) -> None:
        x = var("X")
        goal = conj(
            neq(x, atom("homer")),
            eq(x, atom("homer")),
        )

        assert run_all(x, goal) == []

    def test_eq_satisfies_and_drops_disequality_constraints(self) -> None:
        x = var("X")
        states = list(conj(neq(x, atom("homer")), eq(x, atom("marge")))(State()))

        assert len(states) == 1
        assert states[0].constraints == ()
        assert states[0].substitution.reify(x) == atom("marge")

    def test_disjunction_emits_multiple_answers(self) -> None:
        x = var("X")

        answers = run_all(
            x,
            disj(eq(x, atom("homer")), eq(x, atom("marge"))),
        )

        assert answers == [atom("homer"), atom("marge")]

    def test_conjunction_threads_substitutions(self) -> None:
        x = var("X")
        y = var("Y")
        goal = conj(
            eq(term("pair", x, y), term("pair", atom("homer"), atom("bart"))),
            eq(y, atom("bart")),
        )

        assert run_all((x, y), goal) == [(atom("homer"), atom("bart"))]

    def test_fresh_allocates_search_local_variables(self) -> None:
        x = var("X")
        goal = fresh(
            1,
            lambda y: conj(
                eq(y, atom("marge")),
                eq(x, y),
            ),
        )

        assert run_all(x, goal) == [atom("marge")]

    def test_fresh_advances_the_state_counter(self) -> None:
        initial = State()
        seen_ids: list[int] = []

        def recorder(inner: LogicVar) -> object:
            seen_ids.append(inner.id)
            return succeed()

        list(fresh(1, recorder)(initial))

        assert seen_ids == [0]

    def test_goals_preserve_runtime_extension_slots(self) -> None:
        x = var("X")
        initial = State(
            database={"dynamic": "snapshot"},
            fd_store={"domains": "snapshot"},
        )

        [state] = list(eq(x, atom("tea"))(initial))
        [fresh_state] = list(fresh(1, lambda _inner: succeed())(state))

        assert state.database == {"dynamic": "snapshot"}
        assert state.fd_store == {"domains": "snapshot"}
        assert fresh_state.database == {"dynamic": "snapshot"}
        assert fresh_state.fd_store == {"domains": "snapshot"}

    def test_run_n_truncates_without_consuming_every_answer(self) -> None:
        x = var("X")
        goal = disj(
            eq(x, atom("a")),
            eq(x, atom("b")),
            eq(x, atom("c")),
        )

        assert run_n(2, x, goal) == [atom("a"), atom("b")]

    def test_run_n_rejects_negative_limits(self) -> None:
        x = var("X")

        with pytest.raises(ValueError):
            run_n(-1, x, eq(x, atom("a")))

    def test_fresh_requires_at_least_one_variable(self) -> None:
        with pytest.raises(ValueError):
            fresh(0, lambda: succeed())

    def test_queries_can_return_multiple_reified_variables(self) -> None:
        x = var("X")
        y = var("Y")
        goal = eq(term("edge", x, y), term("edge", atom("a"), atom("b")))

        assert run_all((x, y), goal) == [(atom("a"), atom("b"))]

    def test_coercion_allows_host_language_atoms_in_goals(self) -> None:
        x = var("X")

        goal = eq(
            term("parent", x, "bart"),
            term("parent", "homer", "bart"),
        )

        assert run_all(x, goal) == [
            atom("homer"),
        ]
