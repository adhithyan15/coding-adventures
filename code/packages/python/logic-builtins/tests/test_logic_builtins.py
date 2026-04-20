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
    add,
    argo,
    atomo,
    bagofo,
    callo,
    compoundo,
    div,
    failo,
    findallo,
    floordiv,
    forallo,
    functoro,
    geqo,
    groundo,
    gto,
    ifthenelseo,
    iftheno,
    iso,
    leqo,
    lto,
    mod,
    mul,
    neg,
    nonvaro,
    noto,
    numbero,
    numeqo,
    numneqo,
    onceo,
    setofo,
    stringo,
    sub,
    trueo,
    varo,
)


class TestVersion:
    """Verify the package is importable and versioned."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.4.0"


class TestAdvancedControlBuiltins:
    """Advanced control should stay honest about committed search behavior."""

    def test_trueo_succeeds_once_and_preserves_state(self) -> None:
        item = var("Item")

        assert solve_all(program(), item, conj(eq(item, "tea"), trueo())) == [
            atom("tea"),
        ]

    def test_failo_fails_without_successor_states(self) -> None:
        item = var("Item")

        assert solve_all(program(), item, conj(eq(item, "tea"), failo())) == []

    def test_iftheno_commits_to_first_condition_proof(self) -> None:
        guard = var("Guard")
        result = var("Result")

        assert solve_all(
            program(),
            result,
            iftheno(
                disj(eq(guard, "first"), eq(guard, "second")),
                eq(result, guard),
            ),
        ) == [atom("first")]

    def test_iftheno_keeps_then_branch_backtracking(self) -> None:
        guard = var("Guard")
        result = var("Result")

        assert solve_all(
            program(),
            result,
            iftheno(
                disj(eq(guard, "first"), eq(guard, "second")),
                disj(
                    eq(result, term("choice", guard, "a")),
                    eq(result, term("choice", guard, "b")),
                ),
            ),
        ) == [
            term("choice", "first", "a"),
            term("choice", "first", "b"),
        ]

    def test_iftheno_fails_when_condition_fails(self) -> None:
        result = var("Result")

        assert solve_all(program(), result, iftheno(fail(), eq(result, "then"))) == []

    def test_ifthenelseo_chooses_then_branch_when_condition_succeeds(self) -> None:
        guard = var("Guard")
        result = var("Result")

        assert solve_all(
            program(),
            result,
            ifthenelseo(
                disj(eq(guard, "first"), eq(guard, "second")),
                eq(result, term("then", guard)),
                eq(result, "else"),
            ),
        ) == [term("then", "first")]

    def test_ifthenelseo_chooses_else_from_original_state(self) -> None:
        item = var("Item")

        assert solve_all(
            program(),
            item,
            ifthenelseo(
                conj(eq(item, "condition-binding"), fail()),
                eq(item, "then"),
                eq(item, "else"),
            ),
        ) == [atom("else")]

    def test_forallo_succeeds_when_every_generated_proof_passes(self) -> None:
        marker = var("Marker")
        item = var("Item")

        assert solve_all(
            program(),
            marker,
            conj(
                eq(marker, "ok"),
                forallo(disj(eq(item, 1), eq(item, 2)), lto(item, 3)),
            ),
        ) == [atom("ok")]

    def test_forallo_fails_when_any_generated_proof_fails_test(self) -> None:
        marker = var("Marker")
        item = var("Item")

        assert solve_all(
            program(),
            marker,
            conj(
                eq(marker, "ok"),
                forallo(disj(eq(item, 1), eq(item, 4)), lto(item, 3)),
            ),
        ) == []

    def test_forallo_succeeds_vacuously_and_does_not_leak_bindings(self) -> None:
        item = var("Item")

        assert solve_all(program(), item, forallo(fail(), fail())) == [item]
        assert solve_all(
            program(),
            item,
            forallo(disj(eq(item, 1), eq(item, 2)), numbero(item)),
        ) == [item]

    def test_advanced_control_rejects_non_goals(self) -> None:
        with pytest.raises(TypeError):
            iftheno(object(), trueo())
        with pytest.raises(TypeError):
            iftheno(trueo(), object())
        with pytest.raises(TypeError):
            ifthenelseo(object(), trueo(), failo())
        with pytest.raises(TypeError):
            ifthenelseo(trueo(), object(), failo())
        with pytest.raises(TypeError):
            ifthenelseo(trueo(), failo(), object())
        with pytest.raises(TypeError):
            forallo(object(), trueo())
        with pytest.raises(TypeError):
            forallo(trueo(), object())


class TestCollectionBuiltins:
    """Collection helpers should turn streams of proofs into logic lists."""

    def test_findallo_collects_answers_in_proof_order(self) -> None:
        item = var("Item")
        results = var("Results")

        assert solve_all(
            program(),
            results,
            findallo(item, disj(eq(item, "tea"), eq(item, "cake")), results),
        ) == [logic_list(["tea", "cake"])]

    def test_findallo_preserves_duplicates(self) -> None:
        item = var("Item")
        results = var("Results")

        assert solve_all(
            program(),
            results,
            findallo(item, disj(eq(item, "tea"), eq(item, "tea")), results),
        ) == [logic_list(["tea", "tea"])]

    def test_findallo_succeeds_with_empty_list_when_goal_fails(self) -> None:
        item = var("Item")
        results = var("Results")

        assert solve_all(program(), results, findallo(item, fail(), results)) == [
            logic_list([]),
        ]

    def test_findallo_does_not_leak_inner_bindings_outside_results(self) -> None:
        item = var("Item")
        results = var("Results")

        assert solve_all(
            program(),
            (item, results),
            findallo(item, disj(eq(item, "tea"), eq(item, "cake")), results),
        ) == [(item, logic_list(["tea", "cake"]))]

    def test_bagofo_preserves_duplicates_and_fails_when_empty(self) -> None:
        item = var("Item")
        results = var("Results")

        assert solve_all(
            program(),
            results,
            bagofo(item, disj(eq(item, "tea"), eq(item, "tea")), results),
        ) == [logic_list(["tea", "tea"])]
        assert solve_all(program(), results, bagofo(item, fail(), results)) == []

    def test_setofo_removes_duplicates_and_sorts_terms(self) -> None:
        item = var("Item")
        results = var("Results")

        assert solve_all(
            program(),
            results,
            setofo(
                item,
                disj(
                    eq(item, "pear"),
                    eq(item, 2),
                    eq(item, "apple"),
                    eq(item, "pear"),
                ),
                results,
            ),
        ) == [logic_list([2, "apple", "pear"])]
        assert solve_all(program(), results, setofo(item, fail(), results)) == []

    def test_collectors_compose_with_arithmetic_predicates(self) -> None:
        raw = var("Raw")
        adjusted = var("Adjusted")
        results = var("Results")

        assert solve_all(
            program(),
            results,
            findallo(
                adjusted,
                conj(
                    disj(eq(raw, 2), eq(raw, 5)),
                    iso(adjusted, add(raw, 10)),
                ),
                results,
            ),
        ) == [logic_list([12, 15])]

    def test_collectors_compose_with_relation_search(self) -> None:
        parent = relation("parent", 2)
        child = var("Child")
        results = var("Results")
        family = program(
            fact(parent("homer", "bart")),
            fact(parent("homer", "lisa")),
            fact(parent("marge", "maggie")),
        )

        assert solve_all(
            family,
            results,
            findallo(child, parent("homer", child), results),
        ) == [logic_list(["bart", "lisa"])]

    def test_collectors_reject_non_goals(self) -> None:
        results = var("Results")

        with pytest.raises(TypeError):
            findallo("x", object(), results)
        with pytest.raises(TypeError):
            bagofo("x", object(), results)
        with pytest.raises(TypeError):
            setofo("x", object(), results)


class TestArithmeticBuiltins:
    """Arithmetic helpers should evaluate expressions without becoming syntax."""

    def test_arithmetic_constructors_return_ordinary_compound_terms(self) -> None:
        assert add(1, 2) == term("+", 1, 2)
        assert sub(4, 2) == term("-", 4, 2)
        assert mul(3, 5) == term("*", 3, 5)
        assert div(7, 2) == term("/", 7, 2)
        assert floordiv(7, 2) == term("//", 7, 2)
        assert mod(7, 2) == term("mod", 7, 2)
        assert neg(4) == term("-", 4)

    def test_iso_evaluates_integer_and_float_expressions(self) -> None:
        result = var("Result")

        assert solve_all(program(), result, iso(result, add(1, mul(2, 3)))) == [
            num(7),
        ]
        assert solve_all(program(), result, iso(result, div(7, 2))) == [num(3.5)]
        assert solve_all(program(), result, iso(result, floordiv(7, 2))) == [num(3)]
        assert solve_all(program(), result, iso(result, mod(7, 2))) == [num(1)]
        assert solve_all(program(), result, iso(result, neg(4))) == [num(-4)]

    def test_iso_uses_current_logic_variable_bindings(self) -> None:
        base = var("Base")
        doubled = var("Doubled")

        assert solve_all(
            program(),
            doubled,
            conj(eq(base, 4), iso(doubled, mul(base, 2))),
        ) == [num(8)]

    def test_iso_fails_when_expression_is_not_instantiated_enough(self) -> None:
        base = var("Base")
        result = var("Result")

        assert solve_all(program(), result, iso(result, add(base, 1))) == []

    def test_iso_fails_for_non_numeric_terms_and_division_by_zero(self) -> None:
        result = var("Result")

        assert solve_all(program(), result, iso(result, add("tea", 1))) == []
        assert solve_all(program(), result, iso(result, term("pow", 2, 3))) == []
        assert solve_all(program(), result, iso(result, div(7, 0))) == []
        assert solve_all(program(), result, iso(result, floordiv(7, 0))) == []
        assert solve_all(program(), result, iso(result, mod(7, 0))) == []

    def test_numeric_comparisons_evaluate_both_sides(self) -> None:
        marker = var("Marker")

        assert solve_all(
            program(),
            marker,
            conj(eq(marker, "ok"), numeqo(add(1, 2), 3)),
        ) == [atom("ok")]
        assert solve_all(
            program(),
            marker,
            conj(eq(marker, "ok"), numneqo(add(1, 2), 4)),
        ) == [atom("ok")]
        assert solve_all(program(), marker, conj(eq(marker, "ok"), lto(2, 3))) == [
            atom("ok"),
        ]
        assert solve_all(program(), marker, conj(eq(marker, "ok"), leqo(3, 3))) == [
            atom("ok"),
        ]
        assert solve_all(program(), marker, conj(eq(marker, "ok"), gto(4, 3))) == [
            atom("ok"),
        ]
        assert solve_all(program(), marker, conj(eq(marker, "ok"), geqo(4, 4))) == [
            atom("ok"),
        ]

    def test_numeric_comparisons_fail_for_false_or_open_expressions(self) -> None:
        marker = var("Marker")
        open_value = var("Open")

        assert solve_all(program(), marker, conj(eq(marker, "ok"), numeqo(1, 2))) == []
        assert solve_all(
            program(),
            marker,
            conj(eq(marker, "ok"), lto(add(open_value, 1), 3)),
        ) == []

    def test_arithmetic_goals_compose_with_relation_search(self) -> None:
        score = relation("score", 2)
        person = var("Person")
        raw_score = var("RawScore")
        adjusted_score = var("AdjustedScore")
        scores = program(
            fact(score("alice", 7)),
            fact(score("bob", 3)),
        )

        assert solve_all(
            scores,
            (person, adjusted_score),
            conj(
                score(person, raw_score),
                geqo(raw_score, 5),
                iso(adjusted_score, add(raw_score, 10)),
            ),
        ) == [(atom("alice"), num(17))]

    def test_arithmetic_goals_compose_with_control_builtins(self) -> None:
        value = var("Value")
        marker = var("Marker")

        assert solve_all(
            program(),
            value,
            onceo(disj(iso(value, add(1, 1)), iso(value, add(2, 2)))),
        ) == [num(2)]
        assert solve_all(
            program(),
            marker,
            conj(eq(marker, "ok"), noto(lto(add(2, 2), 3))),
        ) == [atom("ok")]


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
