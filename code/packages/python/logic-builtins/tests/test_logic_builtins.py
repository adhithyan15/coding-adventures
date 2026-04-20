"""Tests for Prolog-inspired control and term builtins."""

from __future__ import annotations

import pytest
from logic_engine import (
    atom,
    conj,
    disj,
    eq,
    fact,
    fail,
    logic_list,
    num,
    program,
    relation,
    solve_all,
    solve_n,
    string,
    term,
    var,
)

from logic_builtins import (
    __version__,
    argo,
    atomo,
    callo,
    compoundo,
    functoro,
    groundo,
    nonvaro,
    noto,
    numbero,
    onceo,
    stringo,
    varo,
)


class TestVersion:
    """Verify the package is importable and versioned."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.1.0"


class TestControlBuiltins:
    """Control helpers should compose with ordinary engine goals."""

    def test_callo_runs_a_supplied_goal(self) -> None:
        item = var("Item")

        assert solve_all(program(), item, callo(eq(item, "tea"))) == [atom("tea")]

    def test_onceo_keeps_only_the_first_solution(self) -> None:
        item = var("Item")

        answers = solve_all(
            program(),
            item,
            onceo(disj(eq(item, "first"), eq(item, "second"))),
        )

        assert answers == [atom("first")]

    def test_noto_succeeds_when_goal_fails_and_fails_when_goal_succeeds(self) -> None:
        marker = var("Marker")

        assert solve_all(program(), marker, noto(fail())) == [marker]
        assert solve_all(program(), marker, noto(eq("same", "same"))) == []

    def test_noto_respects_the_current_binding_state(self) -> None:
        item = var("Item")

        assert solve_all(
            program(),
            item,
            conj(eq(item, "tea"), noto(eq(item, "cake"))),
        ) == [atom("tea")]
        assert solve_all(
            program(),
            item,
            conj(eq(item, "tea"), noto(eq(item, "tea"))),
        ) == []

    def test_control_builtins_reject_non_goals(self) -> None:
        with pytest.raises(TypeError):
            callo(object())
        with pytest.raises(TypeError):
            onceo(object())
        with pytest.raises(TypeError):
            noto(object())


class TestTermStateBuiltins:
    """Term predicates should observe the current substitution."""

    def test_groundo_distinguishes_ground_terms_from_open_terms(self) -> None:
        item = var("Item")
        open_item = var("Open")

        assert solve_all(
            program(),
            item,
            conj(eq(item, term("box", "tea")), groundo(item)),
        ) == [term("box", "tea")]
        assert solve_all(program(), open_item, groundo(term("box", open_item))) == []

    def test_varo_and_nonvaro_observe_bindings(self) -> None:
        item = var("Item")

        assert solve_all(program(), item, varo(item)) == [item]
        assert solve_all(program(), item, conj(eq(item, "tea"), varo(item))) == []
        assert solve_all(program(), item, conj(eq(item, "tea"), nonvaro(item))) == [
            atom("tea"),
        ]

    def test_type_checks_observe_reified_values(self) -> None:
        item = var("Item")

        assert solve_all(program(), item, conj(eq(item, "tea"), atomo(item))) == [
            atom("tea"),
        ]
        assert solve_all(program(), item, conj(eq(item, 3), numbero(item))) == [num(3)]
        assert solve_all(
            program(),
            item,
            conj(eq(item, string("tea")), stringo(item)),
        ) == [string("tea")]
        assert solve_all(
            program(),
            item,
            conj(eq(item, term("box", "tea")), compoundo(item)),
        ) == [term("box", "tea")]

        assert solve_all(program(), item, atomo(item)) == []
        assert solve_all(program(), item, numbero(atom("tea"))) == []

    def test_functoro_extracts_compound_functor_and_arity(self) -> None:
        name = var("Name")
        arity = var("Arity")

        assert solve_all(
            program(),
            (name, arity),
            functoro(term("box", "tea", "cake"), name, arity),
        ) == [(atom("box"), num(2))]

    def test_functoro_fails_for_non_compound_terms(self) -> None:
        name = var("Name")
        arity = var("Arity")

        assert solve_all(program(), (name, arity), functoro("tea", name, arity)) == []

    def test_argo_extracts_one_based_compound_arguments(self) -> None:
        value = var("Value")

        assert solve_all(
            program(),
            value,
            argo(2, term("box", "tea", "cake"), value),
        ) == [atom("cake")]

    def test_argo_fails_for_invalid_indexes_or_non_compounds(self) -> None:
        value = var("Value")

        assert solve_all(program(), value, argo(0, term("box", "tea"), value)) == []
        assert solve_all(program(), value, argo(2, term("box", "tea"), value)) == []
        assert solve_all(program(), value, argo(1, "tea", value)) == []

    def test_builtins_compose_with_relation_search(self) -> None:
        parent = relation("parent", 2)
        child = var("Child")
        family = program(fact(parent("homer", "bart")))

        assert solve_all(
            family,
            child,
            conj(parent("homer", child), groundo(child), noto(eq(child, "lisa"))),
        ) == [atom("bart")]

    def test_builtins_compose_with_structural_terms(self) -> None:
        value = var("Value")

        assert solve_n(
            program(),
            1,
            value,
            conj(
                eq(value, logic_list(["tea"])),
                compoundo(value),
                groundo(value),
            ),
        ) == [logic_list(["tea"])]
