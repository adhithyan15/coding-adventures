"""Tests for Prolog-inspired control and term builtins."""

from __future__ import annotations

import pytest
from logic_engine import (
    Compound,
    LogicVar,
    atom,
    conj,
    disj,
    eq,
    fact,
    fail,
    fresh,
    logic_list,
    num,
    program,
    relation,
    rule,
    solve_all,
    solve_n,
    string,
    term,
    var,
)

from logic_builtins import (
    __version__,
    abolisho,
    add,
    all_differento,
    argo,
    assertao,
    assertzo,
    atomico,
    atomo,
    bagofo,
    betweeno,
    callableo,
    callo,
    calltermo,
    clauseo,
    compare_termo,
    compoundo,
    copytermo,
    current_predicateo,
    cuto,
    div,
    dynamico,
    failo,
    fd_addo,
    fd_eqo,
    fd_geqo,
    fd_gto,
    fd_ino,
    fd_leqo,
    fd_lto,
    fd_mulo,
    fd_neqo,
    fd_subo,
    fd_sumo,
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
    labelingo,
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
    predicate_propertyo,
    retractallo,
    retracto,
    same_termo,
    setofo,
    stringo,
    sub,
    termo_geqo,
    termo_gto,
    termo_leqo,
    termo_lto,
    trueo,
    univo,
    varo,
)


class TestVersion:
    """Verify the package is importable and versioned."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.14.0"


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

    def test_ifthenelseo_freshens_rule_local_variables(self) -> None:
        candidate = relation("candidate", 1)
        chosen = relation("chosen", 1)
        choice = var("Choice")
        result = var("Result")
        output = var("Output")

        prog = program(
            fact(candidate("first")),
            fact(candidate("second")),
            rule(
                chosen(result),
                ifthenelseo(
                    candidate(choice),
                    eq(result, choice),
                    eq(result, "none"),
                ),
            ),
        )

        assert solve_all(prog, output, chosen(output)) == [atom("first")]

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

    def test_betweeno_enumerates_inclusive_integer_ranges(self) -> None:
        value = var("Value")

        assert solve_all(program(), value, betweeno(2, 5, value)) == [
            num(2),
            num(3),
            num(4),
            num(5),
        ]

    def test_betweeno_validates_bound_values(self) -> None:
        marker = var("Marker")

        assert solve_all(
            program(),
            marker,
            conj(eq(marker, "ok"), betweeno(2, 5, 4)),
        ) == [atom("ok")]
        assert solve_all(
            program(),
            marker,
            conj(eq(marker, "ok"), betweeno(2, 5, 8)),
        ) == []

    def test_betweeno_rejects_non_integer_and_descending_ranges(self) -> None:
        value = var("Value")

        assert solve_all(program(), value, betweeno(5, 2, value)) == []
        assert solve_all(program(), value, betweeno(1.5, 3, value)) == []
        assert solve_all(program(), value, betweeno(1, 3, "tea")) == []

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


class TestFiniteDomainBuiltins:
    """Finite-domain constraints should narrow first and label explicitly."""

    def test_fd_ino_and_labelingo_enumerate_domains_in_order(self) -> None:
        value = var("Value")

        assert solve_all(
            program(),
            value,
            conj(fd_ino(value, range(1, 4)), labelingo([value])),
        ) == [num(1), num(2), num(3)]

    def test_fd_domains_can_be_logic_lists_or_inclusive_range_terms(self) -> None:
        left = var("Left")
        right = var("Right")

        assert solve_all(
            program(),
            (left, right),
            conj(
                fd_ino(left, logic_list([2, 4])),
                fd_ino(right, term("..", 3, 4)),
                labelingo([left, right]),
            ),
        ) == [(num(2), num(3)), (num(2), num(4)), (num(4), num(3)), (num(4), num(4))]

    def test_fd_constraints_narrow_before_labeling(self) -> None:
        value = var("Value")

        assert solve_all(
            program(),
            value,
            conj(fd_ino(value, range(1, 6)), fd_lto(value, 3), labelingo([value])),
        ) == [num(1), num(2)]

    def test_fd_constraints_are_order_independent(self) -> None:
        value = var("Value")

        assert solve_all(
            program(),
            value,
            conj(fd_lto(value, 3), fd_ino(value, range(1, 6)), labelingo([value])),
        ) == [num(1), num(2)]

    def test_fd_equality_intersects_aliased_domains(self) -> None:
        left = var("Left")
        right = var("Right")

        assert solve_all(
            program(),
            (left, right),
            conj(
                fd_ino(left, [1, 2]),
                fd_ino(right, [2, 3]),
                fd_eqo(left, right),
                labelingo([left, right]),
            ),
        ) == [(num(2), num(2))]

    def test_fd_neq_and_comparisons_prune_binary_domains(self) -> None:
        left = var("Left")
        right = var("Right")

        assert solve_all(
            program(),
            (left, right),
            conj(
                fd_ino(left, range(1, 4)),
                fd_ino(right, range(1, 4)),
                fd_neqo(left, right),
                fd_leqo(left, right),
                labelingo([left, right]),
            ),
        ) == [(num(1), num(2)), (num(1), num(3)), (num(2), num(3))]

    def test_fd_greater_comparisons_check_concrete_terms(self) -> None:
        marker = var("Marker")

        assert solve_all(
            program(),
            marker,
            conj(eq(marker, "ok"), fd_gto(4, 3), fd_geqo(4, 4)),
        ) == [atom("ok")]
        assert solve_all(program(), marker, conj(eq(marker, "ok"), fd_gto(3, 4))) == []

    def test_labelingo_accepts_logic_list_of_variables(self) -> None:
        left = var("Left")
        right = var("Right")

        assert solve_all(
            program(),
            (left, right),
            conj(
                fd_ino(left, [1]),
                fd_ino(right, [2]),
                labelingo(logic_list([left, right])),
            ),
        ) == [(num(1), num(2))]

    def test_fd_store_rolls_back_across_disjunction_branches(self) -> None:
        value = var("Value")

        assert solve_all(
            program(),
            value,
            disj(
                conj(fd_ino(value, [1]), labelingo([value])),
                labelingo([value]),
            ),
        ) == [num(1)]

    def test_fd_ino_without_labeling_preserves_open_variables(self) -> None:
        value = var("Value")

        assert solve_all(program(), value, fd_ino(value, [1, 2])) == [value]

    def test_fd_domain_validation_is_eager(self) -> None:
        value = var("Value")

        with pytest.raises(TypeError):
            fd_ino(value, [1, 2.5])
        with pytest.raises(ValueError):
            fd_ino(value, range(0, 10_001))

    def test_fd_addo_prunes_addend_domains_before_labeling(self) -> None:
        left = var("Left")
        right = var("Right")

        assert solve_all(
            program(),
            (left, right),
            conj(
                fd_ino(left, range(1, 6)),
                fd_ino(right, range(1, 6)),
                fd_addo(left, right, 6),
                fd_lto(left, right),
                labelingo([left, right]),
            ),
        ) == [(num(1), num(5)), (num(2), num(4))]

    def test_fd_arithmetic_constraints_are_order_independent(self) -> None:
        left = var("Left")
        right = var("Right")

        assert solve_all(
            program(),
            (left, right),
            conj(
                fd_addo(left, right, 4),
                fd_ino(left, range(1, 5)),
                fd_ino(right, range(1, 5)),
                fd_lto(left, right),
                labelingo([left, right]),
            ),
        ) == [(num(1), num(3))]

    def test_fd_subo_solves_difference_constraints(self) -> None:
        left = var("Left")
        right = var("Right")

        assert solve_all(
            program(),
            (left, right),
            conj(
                fd_ino(left, range(1, 6)),
                fd_ino(right, range(1, 6)),
                fd_subo(left, right, 2),
                labelingo([left, right]),
            ),
        ) == [(num(3), num(1)), (num(4), num(2)), (num(5), num(3))]

    def test_fd_mulo_solves_product_constraints(self) -> None:
        left = var("Left")
        right = var("Right")

        assert solve_all(
            program(),
            (left, right),
            conj(
                fd_ino(left, range(1, 7)),
                fd_ino(right, range(1, 7)),
                fd_mulo(left, right, 12),
                fd_leqo(left, right),
                labelingo([left, right]),
            ),
        ) == [(num(2), num(6)), (num(3), num(4))]

    def test_fd_sumo_solves_n_ary_sum_constraints(self) -> None:
        left = var("Left")
        middle = var("Middle")
        right = var("Right")
        values = (left, middle, right)

        assert solve_all(
            program(),
            values,
            conj(
                *(fd_ino(value, range(1, 5)) for value in values),
                fd_sumo(values, 6),
                all_differento(values),
                labelingo(values),
            ),
        ) == [
            (num(1), num(2), num(3)),
            (num(1), num(3), num(2)),
            (num(2), num(1), num(3)),
            (num(2), num(3), num(1)),
            (num(3), num(1), num(2)),
            (num(3), num(2), num(1)),
        ]

    def test_fd_sumo_accepts_logic_lists_and_result_variables(self) -> None:
        left = var("Left")
        right = var("Right")
        total = var("Total")

        assert solve_all(
            program(),
            (left, right, total),
            conj(
                fd_ino(left, range(1, 4)),
                fd_ino(right, range(1, 4)),
                fd_ino(total, [4]),
                fd_sumo(logic_list([left, right]), total),
                fd_lto(left, right),
                labelingo([left, right, total]),
            ),
        ) == [(num(1), num(3), num(4))]

    def test_fd_sumo_prunes_result_domain_before_labeling(self) -> None:
        total = var("Total")

        assert solve_all(
            program(),
            total,
            conj(
                fd_ino(total, range(0, 10)),
                fd_sumo([1, 2, 3], total),
                labelingo([total]),
            ),
        ) == [num(6)]

    def test_fd_sumo_handles_empty_sums(self) -> None:
        total = var("Total")
        marker = var("Marker")

        assert solve_all(
            program(),
            total,
            conj(fd_ino(total, range(0, 3)), fd_sumo([], total), labelingo([total])),
        ) == [num(0)]
        assert solve_all(
            program(),
            marker,
            conj(eq(marker, "ok"), fd_sumo([], 1)),
        ) == []

    def test_all_differento_prunes_singleton_assignments(self) -> None:
        left = var("Left")
        middle = var("Middle")
        right = var("Right")

        assert solve_all(
            program(),
            (left, middle, right),
            conj(
                fd_ino(left, [1]),
                fd_ino(middle, range(1, 4)),
                fd_ino(right, range(1, 4)),
                all_differento([left, middle, right]),
                labelingo([left, middle, right]),
            ),
        ) == [(num(1), num(2), num(3)), (num(1), num(3), num(2))]

    def test_all_differento_rejects_duplicate_concrete_values(self) -> None:
        marker = var("Marker")

        assert solve_all(
            program(),
            marker,
            conj(eq(marker, "ok"), all_differento([1, 1, marker])),
        ) == []

    def test_all_differento_solves_tiny_latin_square(self) -> None:
        top_left = var("TopLeft")
        top_right = var("TopRight")
        bottom_left = var("BottomLeft")
        bottom_right = var("BottomRight")
        cells = (top_left, top_right, bottom_left, bottom_right)

        assert solve_all(
            program(),
            cells,
            conj(
                *(fd_ino(cell, range(1, 3)) for cell in cells),
                all_differento([top_left, top_right]),
                all_differento([bottom_left, bottom_right]),
                all_differento(logic_list([top_left, bottom_left])),
                all_differento(logic_list([top_right, bottom_right])),
                labelingo(cells),
            ),
        ) == [
            (num(1), num(2), num(2), num(1)),
            (num(2), num(1), num(1), num(2)),
        ]

    def test_labelingo_uses_smallest_domain_first_with_stable_ties(self) -> None:
        wide = var("Wide")
        narrow = var("Narrow")

        assert solve_all(
            program(),
            (wide, narrow),
            conj(
                fd_ino(wide, range(1, 4)),
                fd_ino(narrow, [10, 20]),
                labelingo([wide, narrow]),
            ),
        ) == [
            (num(1), num(10)),
            (num(2), num(10)),
            (num(3), num(10)),
            (num(1), num(20)),
            (num(2), num(20)),
            (num(3), num(20)),
        ]

    def test_fd_constraints_solve_australia_map_coloring(self) -> None:
        wa = var("WA")
        nt = var("NT")
        sa = var("SA")
        q = var("Q")
        nsw = var("NSW")
        v = var("V")
        t = var("T")
        regions = (wa, nt, sa, q, nsw, v, t)
        borders = (
            (wa, nt),
            (wa, sa),
            (nt, sa),
            (nt, q),
            (sa, q),
            (sa, nsw),
            (sa, v),
            (q, nsw),
            (nsw, v),
        )

        assert solve_n(
            program(),
            1,
            regions,
            conj(
                *(fd_ino(region, range(1, 4)) for region in regions),
                fd_ino(wa, [1]),
                *(fd_neqo(left, right) for left, right in borders),
                labelingo(regions),
            ),
        ) == [(num(1), num(2), num(3), num(1), num(2), num(1), num(1))]

    def test_fd_constraints_solve_four_by_four_latin_square(self) -> None:
        cells = tuple(var(f"Cell{index}") for index in range(16))
        rows = tuple(cells[index : index + 4] for index in range(0, 16, 4))
        columns = tuple(tuple(row[index] for row in rows) for index in range(4))
        givens = {
            cells[0]: 1,
            cells[1]: 2,
            cells[2]: 3,
            cells[3]: 4,
            cells[4]: 2,
            cells[8]: 3,
            cells[12]: 4,
        }

        answers = solve_n(
            program(),
            1,
            cells,
            conj(
                *(fd_ino(cell, range(1, 5)) for cell in cells),
                *(fd_ino(cell, [value]) for cell, value in givens.items()),
                *(all_differento(row) for row in rows),
                *(all_differento(column) for column in columns),
                labelingo(cells),
            ),
        )

        assert len(answers) == 1
        square = tuple(
            answers[0][index : index + 4]
            for index in range(0, len(answers[0]), 4)
        )
        latin_values = {num(1), num(2), num(3), num(4)}
        assert square[0] == (num(1), num(2), num(3), num(4))
        assert tuple(row[0] for row in square) == (num(1), num(2), num(3), num(4))
        assert all(set(row) == latin_values for row in square)
        assert all(
            {row[index] for row in square} == latin_values
            for index in range(4)
        )

    def test_fd_constraints_solve_simple_task_schedule(self) -> None:
        design = var("Design")
        build = var("Build")
        test = var("Test")
        design_done = var("DesignDone")
        build_done = var("BuildDone")
        starts = (design, build, test)
        all_times = (*starts, design_done, build_done)

        assert solve_n(
            program(),
            1,
            starts,
            conj(
                *(fd_ino(time, range(0, 7)) for time in all_times),
                fd_ino(test, range(0, 5)),
                fd_sumo([design, 1], design_done),
                fd_leqo(design_done, build),
                fd_sumo([build, 2], build_done),
                fd_leqo(build_done, test),
                labelingo(starts),
            ),
        ) == [(num(0), num(1), num(3))]

    def test_fd_sumo_models_resource_allocation(self) -> None:
        design_hours = var("DesignHours")
        build_hours = var("BuildHours")
        test_hours = var("TestHours")
        hours = (design_hours, build_hours, test_hours)

        assert solve_n(
            program(),
            3,
            hours,
            conj(
                fd_ino(design_hours, range(1, 4)),
                fd_ino(build_hours, range(2, 5)),
                fd_ino(test_hours, range(1, 3)),
                fd_sumo(hours, 7),
                fd_geqo(build_hours, design_hours),
                labelingo(hours),
            ),
        ) == [
            (num(2), num(4), num(1)),
            (num(3), num(3), num(1)),
            (num(1), num(4), num(2)),
        ]


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

    def test_cuto_commits_surrounding_search(self) -> None:
        item = var("Item")

        assert solve_all(
            program(),
            item,
            conj(disj(eq(item, "first"), eq(item, "second")), cuto()),
        ) == [atom("first")]

    def test_cuto_is_not_onceo(self) -> None:
        outer = var("Outer")
        inner = var("Inner")

        assert solve_all(
            program(),
            (outer, inner),
            conj(
                disj(eq(outer, "left"), eq(outer, "right")),
                onceo(disj(eq(inner, "one"), eq(inner, "two"))),
            ),
        ) == [(atom("left"), atom("one")), (atom("right"), atom("one"))]
        assert solve_all(
            program(),
            outer,
            conj(disj(eq(outer, "left"), eq(outer, "right")), cuto()),
        ) == [atom("left")]

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


class TestCallableTermBuiltins:
    """Callable term helpers should execute Prolog-shaped goal data."""

    def test_calltermo_executes_reified_relation_and_control_terms(self) -> None:
        parent = relation("parent", 2)
        child = var("Child")
        family = program(
            fact(parent("homer", "bart")),
            fact(parent("homer", "lisa")),
        )

        assert solve_all(
            family,
            child,
            calltermo(term("parent", "homer", child)),
        ) == [atom("bart"), atom("lisa")]
        assert solve_all(
            family,
            child,
            calltermo(
                term(
                    ",",
                    term("parent", "homer", child),
                    term("\\=", child, "lisa"),
                ),
            ),
        ) == [atom("bart")]

    def test_calltermo_executes_clauseo_body_round_trips(self) -> None:
        parent = relation("parent", 2)
        child = relation("child", 2)
        x = var("X")
        y = var("Y")
        body = var("Body")
        family = program(
            fact(parent("homer", "bart")),
            rule(child(x, y), parent(y, x)),
        )

        assert solve_all(
            family,
            body,
            conj(clauseo(child("bart", "homer"), body), calltermo(body)),
        ) == [term("parent", "homer", "bart")]

    def test_calltermo_fails_for_open_or_non_callable_terms(self) -> None:
        goal = var("Goal")
        marker = var("Marker")

        assert solve_all(
            program(),
            marker,
            conj(eq(marker, "ok"), calltermo(goal)),
        ) == []
        assert solve_all(
            program(),
            marker,
            conj(eq(marker, "ok"), calltermo(string("nope"))),
        ) == []


class TestTermMetaprogrammingBuiltins:
    """Term metaprogramming should expose Prolog-style term-as-data tools."""

    def test_univo_decomposes_compounds_in_functor_first_order(self) -> None:
        parts = var("Parts")

        assert solve_all(
            program(),
            parts,
            univo(term("box", "tea", "cake"), parts),
        ) == [logic_list(["box", "tea", "cake"])]

    def test_univo_decomposes_atomic_terms_to_singleton_lists(self) -> None:
        parts = var("Parts")

        assert solve_all(program(), parts, univo("tea", parts)) == [
            logic_list(["tea"]),
        ]
        assert solve_all(program(), parts, univo(3, parts)) == [logic_list([3])]
        assert solve_all(program(), parts, univo(string("tea"), parts)) == [
            logic_list([string("tea")]),
        ]

    def test_univo_constructs_compounds_and_atomic_terms(self) -> None:
        value = var("Value")

        assert solve_all(
            program(),
            value,
            univo(value, logic_list(["box", "tea", "cake"])),
        ) == [term("box", "tea", "cake")]
        assert solve_all(program(), value, univo(value, logic_list([3]))) == [num(3)]

    def test_univo_fails_for_invalid_construction_parts(self) -> None:
        value = var("Value")

        assert solve_all(program(), value, univo(value, logic_list([]))) == []
        assert solve_all(program(), value, univo(value, logic_list([1, "tea"]))) == []
        assert solve_all(program(), value, univo(value, term(".", "box", "open"))) == []

    def test_functoro_inspects_compound_and_atomic_terms(self) -> None:
        name = var("Name")
        arity = var("Arity")

        assert solve_all(
            program(),
            (name, arity),
            functoro(term("box", "tea", "cake"), name, arity),
        ) == [(atom("box"), num(2))]
        assert solve_all(program(), (name, arity), functoro("tea", name, arity)) == [
            (atom("tea"), num(0)),
        ]
        assert solve_all(program(), (name, arity), functoro(3, name, arity)) == [
            (num(3), num(0)),
        ]

    def test_functoro_constructs_atoms_and_compounds(self) -> None:
        value = var("Value")

        assert solve_all(program(), value, functoro(value, "tea", 0)) == [atom("tea")]

        constructed = solve_all(program(), value, functoro(value, "box", 2))
        assert len(constructed) == 1
        [box] = constructed
        assert isinstance(box, Compound)
        assert box.functor == atom("box").symbol
        assert len(box.args) == 2
        assert all(isinstance(argument, LogicVar) for argument in box.args)
        assert box.args[0] != box.args[1]

    def test_functoro_fails_for_invalid_construction_inputs(self) -> None:
        value = var("Value")
        name = var("Name")

        assert solve_all(program(), value, functoro(value, name, 1)) == []
        assert solve_all(program(), value, functoro(value, "box", -1)) == []
        assert solve_all(program(), value, functoro(value, "box", 1.5)) == []
        assert solve_all(program(), value, functoro(value, 3, 1)) == []

    def test_copytermo_copies_ground_terms_and_refreshes_variables(self) -> None:
        copy = var("Copy")
        original = var("Original")

        assert solve_all(program(), copy, copytermo(term("box", "tea"), copy)) == [
            term("box", "tea"),
        ]

        [copied] = solve_all(
            program(),
            copy,
            copytermo(term("pair", original, original), copy),
        )
        assert isinstance(copied, Compound)
        assert copied.functor == atom("pair").symbol
        assert copied.args[0] == copied.args[1]
        assert copied.args[0] != original

    def test_copytermo_respects_existing_bindings_before_copying(self) -> None:
        source = var("Source")
        inner = var("Inner")
        copy = var("Copy")

        assert solve_all(
            program(),
            copy,
            conj(
                eq(source, term("box", inner)),
                eq(inner, "tea"),
                copytermo(source, copy),
            ),
        ) == [term("box", "tea")]

    def test_same_termo_checks_strict_identity_without_unifying(self) -> None:
        left = var("Left")
        right = var("Right")

        assert solve_all(program(), left, same_termo(left, left)) == [left]
        assert solve_all(program(), (left, right), same_termo(left, right)) == []
        assert solve_all(
            program(),
            (left, right),
            conj(eq(left, right), same_termo(left, right)),
        ) == [(right, right)]

    def test_atomico_and_callableo_classify_reified_terms(self) -> None:
        value = var("Value")

        assert solve_all(program(), value, conj(eq(value, "tea"), atomico(value))) == [
            atom("tea"),
        ]
        assert solve_all(program(), value, conj(eq(value, 3), atomico(value))) == [
            num(3),
        ]
        assert solve_all(
            program(),
            value,
            conj(eq(value, term("box", "tea")), atomico(value)),
        ) == []
        assert solve_all(
            program(),
            value,
            conj(eq(value, term("box", "tea")), callableo(value)),
        ) == [term("box", "tea")]
        assert solve_all(
            program(),
            value,
            conj(eq(value, string("tea")), callableo(value)),
        ) == []

    def test_standard_term_order_predicates_compare_without_binding(self) -> None:
        order = var("Order")
        open_var = var("Open")
        marker = var("Marker")

        assert solve_all(program(), order, compare_termo(order, open_var, 7)) == [
            atom("<"),
        ]
        assert solve_all(
            program(),
            order,
            compare_termo(order, term("pair", "a"), term("pair", "a")),
        ) == [atom("=")]
        assert solve_all(
            program(),
            marker,
            conj(eq(marker, "ok"), termo_lto(open_var, 7)),
        ) == [atom("ok")]
        assert solve_all(
            program(),
            marker,
            conj(eq(marker, "ok"), termo_gto(term("box", "tea"), "z")),
        ) == [atom("ok")]

    def test_standard_term_order_respects_compound_arity_before_functor(self) -> None:
        marker = var("Marker")

        assert solve_all(
            program(),
            marker,
            conj(eq(marker, "ok"), termo_lto(term("z"), term("a", "value"))),
        ) == [atom("ok")]
        assert solve_all(
            program(),
            marker,
            conj(eq(marker, "ok"), termo_leqo(term("same"), term("same"))),
        ) == [atom("ok")]
        assert solve_all(
            program(),
            marker,
            conj(eq(marker, "ok"), termo_geqo("atom", 42)),
        ) == [atom("ok")]


class TestClauseIntrospectionBuiltins:
    """Program clauses should be queryable as Prolog-style data."""

    def test_clauseo_enumerates_facts_in_source_order(self) -> None:
        parent = relation("parent", 2)
        head = var("Head")
        body = var("Body")
        family = program(
            fact(parent("homer", "bart")),
            fact(parent("homer", "lisa")),
        )

        assert solve_all(family, (head, body), clauseo(head, body)) == [
            (term("parent", "homer", "bart"), atom("true")),
            (term("parent", "homer", "lisa"), atom("true")),
        ]

    def test_clauseo_returns_instantiated_rule_body_after_head_matching(self) -> None:
        parent = relation("parent", 2)
        child = relation("child", 2)
        x = var("X")
        y = var("Y")
        body = var("Body")
        family = program(rule(child(x, y), parent(y, x)))

        assert solve_all(family, body, clauseo(child("bart", "homer"), body)) == [
            term("parent", "homer", "bart"),
        ]

    def test_clauseo_filters_by_head_pattern(self) -> None:
        parent = relation("parent", 2)
        child = var("Child")
        family = program(
            fact(parent("homer", "bart")),
            fact(parent("homer", "lisa")),
            fact(parent("marge", "maggie")),
        )

        assert solve_all(
            family,
            child,
            clauseo(term("parent", "homer", child), atom("true")),
        ) == [atom("bart"), atom("lisa")]

    def test_clauseo_filters_by_body_pattern(self) -> None:
        parent = relation("parent", 2)
        child = relation("child", 2)
        x = var("X")
        y = var("Y")
        marker = var("Marker")
        family = program(rule(child(x, y), parent(y, x)))

        assert solve_all(
            family,
            marker,
            conj(
                eq(marker, "ok"),
                clauseo(child("bart", "homer"), term("parent", "homer", "bart")),
            ),
        ) == [atom("ok")]
        assert solve_all(
            family,
            marker,
            conj(
                eq(marker, "ok"),
                clauseo(child("bart", "homer"), term("parent", "marge", "bart")),
            ),
        ) == []

    def test_clauseo_standardizes_returned_variables_apart(self) -> None:
        parent = relation("parent", 2)
        child = relation("child", 2)
        source_x = var("X")
        source_y = var("Y")
        head = var("Head")
        body = var("Body")
        family = program(rule(child(source_x, source_y), parent(source_y, source_x)))

        [(observed_head, observed_body)] = solve_all(
            family,
            (head, body),
            clauseo(head, body),
        )

        assert isinstance(observed_head, Compound)
        assert isinstance(observed_body, Compound)
        observed_x, observed_y = observed_head.args
        assert isinstance(observed_x, LogicVar)
        assert isinstance(observed_y, LogicVar)
        assert observed_x != source_x
        assert observed_y != source_y
        assert observed_body.args == (observed_y, observed_x)

    def test_clauseo_skips_host_only_bodies(self) -> None:
        predicate = relation("p", 1)
        body = var("Body")
        facts_and_rules = program(
            rule(predicate("x"), fresh(1, lambda inner: eq(inner, "x"))),
            fact(predicate("y")),
        )

        assert solve_all(facts_and_rules, body, clauseo(predicate("x"), body)) == []
        assert solve_all(facts_and_rules, body, clauseo(predicate("y"), body)) == [
            atom("true"),
        ]


class TestPredicateMetadataBuiltins:
    """Predicate metadata should expose source and builtin predicates."""

    def test_current_predicateo_enumerates_source_predicates(self) -> None:
        parent = relation("parent", 2)
        child = relation("child", 2)
        arity = var("Arity")
        family = program(
            fact(parent("homer", "bart")),
            fact(parent("homer", "lisa")),
            rule(child("bart", "homer"), parent("homer", "bart")),
        )

        assert solve_all(family, arity, current_predicateo("parent", arity)) == [
            num(2),
        ]
        assert solve_all(family, arity, current_predicateo("child", arity)) == [
            num(2),
        ]

    def test_current_predicateo_enumerates_builtin_predicates(self) -> None:
        arity = var("Arity")

        assert solve_all(program(), arity, current_predicateo("calltermo", arity)) == [
            num(1),
        ]

    def test_predicate_propertyo_reports_source_properties(self) -> None:
        parent = relation("parent", 2)
        prop = var("Property")
        family = program(
            fact(parent("homer", "bart")),
            fact(parent("homer", "lisa")),
        )

        properties = set(
            solve_all(family, prop, predicate_propertyo("parent", 2, prop)),
        )

        assert atom("defined") in properties
        assert atom("static") in properties
        assert atom("built_in") not in properties
        assert term("number_of_clauses", 2) in properties

    def test_predicate_propertyo_reports_builtin_properties(self) -> None:
        prop = var("Property")

        properties = set(
            solve_all(program(), prop, predicate_propertyo("calltermo", 1, prop)),
        )

        assert atom("defined") in properties
        assert atom("built_in") in properties
        assert atom("static") not in properties
        assert term("number_of_clauses", 0) in properties


class TestDynamicRuntimeDatabaseBuiltins:
    """Runtime database builtins should be scoped to active proof branches."""

    def test_asserta_and_assertz_update_answer_order_in_branch(self) -> None:
        item = relation("item", 1)
        value = var("Value")

        assert solve_all(
            program(),
            value,
            conj(
                dynamico("item", 1),
                assertzo(item("last")),
                assertao(item("first")),
                item(value),
            ),
        ) == [atom("first"), atom("last")]

    def test_dynamic_database_rolls_back_across_disjunction_branches(self) -> None:
        seen = relation("seen", 1)
        value = var("Value")

        assert solve_all(
            program(),
            value,
            disj(
                conj(dynamico("seen", 1), assertzo(seen("left")), seen(value)),
                seen(value),
            ),
        ) == [atom("left")]

    def test_retracto_binds_pattern_and_removes_first_match(self) -> None:
        todo = relation("todo", 1)
        removed = var("Removed")
        remaining = var("Remaining")

        assert solve_all(
            program(),
            (removed, remaining),
            conj(
                dynamico("todo", 1),
                assertzo(todo("tea")),
                assertzo(todo("cake")),
                retracto(todo(removed)),
                todo(remaining),
            ),
        ) == [(atom("tea"), atom("cake"))]

    def test_retractallo_removes_all_matching_dynamic_heads(self) -> None:
        parent = relation("parent", 2)
        parent_name = var("Parent")
        child_name = var("Child")
        any_child = var("AnyChild")

        assert solve_all(
            program(),
            (parent_name, child_name),
            conj(
                dynamico("parent", 2),
                assertzo(parent("homer", "bart")),
                assertzo(parent("homer", "lisa")),
                assertzo(parent("marge", "bart")),
                retractallo(parent("homer", any_child)),
                parent(parent_name, child_name),
            ),
        ) == [(atom("marge"), atom("bart"))]

    def test_abolisho_removes_dynamic_predicate_for_later_goals(self) -> None:
        scratch = relation("scratch", 1)
        value = var("Value")

        assert solve_all(
            program(),
            value,
            conj(
                dynamico("scratch", 1),
                assertzo(scratch("temp")),
                abolisho("scratch", 1),
                scratch(value),
            ),
        ) == []

    def test_static_source_predicates_cannot_be_modified_by_runtime_builtins(
        self,
    ) -> None:
        parent = relation("parent", 2)
        child = var("Child")
        family = program(fact(parent("homer", "bart")))

        assert solve_all(
            family,
            child,
            conj(assertzo(parent("homer", "lisa")), parent("homer", child)),
        ) == []
        assert solve_all(
            family,
            child,
            conj(dynamico("parent", 2), parent("homer", child)),
        ) == []

    def test_program_dynamic_source_clauses_can_be_retracted(self) -> None:
        parent = relation("parent", 2)
        removed = var("Removed")
        remaining = var("Remaining")
        family = program(
            fact(parent("homer", "bart")),
            fact(parent("homer", "lisa")),
            dynamic_relations=(parent,),
        )

        assert solve_all(
            family,
            (removed, remaining),
            conj(retracto(parent("homer", removed)), parent("homer", remaining)),
        ) == [(atom("bart"), atom("lisa"))]

    def test_dynamic_predicates_are_visible_to_metadata_and_clauseo(self) -> None:
        memo = relation("memo", 1)
        prop = var("Property")
        body = var("Body")

        properties = solve_all(
            program(),
            prop,
            conj(
                dynamico("memo", 1),
                assertzo(memo("cached")),
                predicate_propertyo("memo", 1, prop),
            ),
        )

        assert atom("defined") in properties
        assert atom("dynamic") in properties
        assert atom("static") not in properties
        assert term("number_of_clauses", 1) in properties
        assert solve_all(
            program(),
            body,
            conj(
                dynamico("memo", 1),
                assertzo(memo("cached")),
                clauseo(memo("cached"), body),
            ),
        ) == [atom("true")]


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

    def test_functoro_fails_for_unbound_terms_without_construction_inputs(
        self,
    ) -> None:
        target = var("Target")
        name = var("Name")
        arity = var("Arity")

        assert solve_all(program(), (name, arity), functoro(target, name, arity)) == []

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
