"""Tests for logic-engine.

These tests focus on the layer above ``logic-core``: the parts that make it
possible to write real logic *programs* with named relations, facts, rules, and
recursive resolution.
"""

from __future__ import annotations

from collections.abc import Iterator

import pytest

from logic_engine import (
    Clause,
    DeferredExpr,
    Disequality,
    LogicVar,
    Program,
    State,
    Term,
    __version__,
    abolish,
    all_different,
    asserta,
    assertz,
    atom,
    clause_as_term,
    clause_body,
    clause_from_term,
    clauses_matching,
    conj,
    cut,
    declare_dynamic,
    defer,
    disj,
    eq,
    fact,
    fail,
    fresh,
    freshen_clause,
    goal_as_term,
    goal_from_term,
    logic_list,
    native_goal,
    neq,
    num,
    program,
    relation,
    retract_all,
    retract_first,
    rule,
    runtime_assertz,
    runtime_declare_dynamic,
    runtime_retract_first,
    solve,
    solve_all,
    solve_from,
    solve_n,
    string,
    succeed,
    term,
    var,
    visible_clauses_for,
)


class TestVersion:
    """Verify the package is importable and versioned."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.10.0"


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


class TestPersistentClauseDatabase:
    """Database helpers should update immutable programs in Prolog-like ways."""

    def test_assertz_appends_clauses_in_source_order(self) -> None:
        parent = relation("parent", 2)
        x = var("X")
        family = program(fact(parent("homer", "bart")))

        updated = assertz(family, fact(parent("homer", "lisa")))

        assert solve_all(updated, x, parent("homer", x)) == [
            atom("bart"),
            atom("lisa"),
        ]

    def test_asserta_prepends_clauses_and_affects_answer_order(self) -> None:
        parent = relation("parent", 2)
        x = var("X")
        family = program(
            fact(parent("homer", "bart")),
            fact(parent("homer", "lisa")),
        )

        updated = asserta(family, fact(parent("homer", "maggie")))

        assert solve_all(updated, x, parent("homer", x)) == [
            atom("maggie"),
            atom("bart"),
            atom("lisa"),
        ]

    def test_assertion_helpers_reject_invalid_inputs(self) -> None:
        parent = relation("parent", 2)

        with pytest.raises(TypeError):
            asserta(program(), object())

        with pytest.raises(TypeError):
            assertz(object(), fact(parent("homer", "bart")))

    def test_clauses_matching_uses_head_unification_in_source_order(self) -> None:
        parent = relation("parent", 2)
        child = relation("child", 2)
        x = var("X")
        homer_bart = fact(parent("homer", "bart"))
        homer_lisa = fact(parent("homer", "lisa"))
        marge_bart = fact(parent("marge", "bart"))
        inverse = rule(child(x, "homer"), parent("homer", x))
        family = program(homer_bart, homer_lisa, marge_bart, inverse)

        assert clauses_matching(family, parent("homer", x)) == (
            homer_bart,
            homer_lisa,
        )

    def test_matching_ignores_unrelated_relation_symbols_and_arities(self) -> None:
        parent2 = relation("parent", 2)
        parent1 = relation("parent", 1)
        x = var("X")
        expected = fact(parent2("homer", "bart"))
        family = program(expected, fact(parent1("homer")))

        assert clauses_matching(family, parent2("homer", x)) == (expected,)

    def test_retract_first_removes_only_the_first_matching_clause(self) -> None:
        parent = relation("parent", 2)
        x = var("X")
        family = program(
            fact(parent("homer", "bart")),
            fact(parent("homer", "lisa")),
            fact(parent("marge", "bart")),
        )

        updated = retract_first(family, parent("homer", x))

        assert updated is not None
        assert solve_all(updated, x, parent("homer", x)) == [atom("lisa")]
        assert solve_all(updated, x, parent("marge", x)) == [atom("bart")]

    def test_retract_first_returns_none_when_no_clause_matches(self) -> None:
        parent = relation("parent", 2)
        x = var("X")
        family = program(fact(parent("homer", "bart")))

        assert retract_first(family, parent("marge", x)) is None

    def test_retract_all_removes_every_matching_clause(self) -> None:
        parent = relation("parent", 2)
        x = var("X")
        family = program(
            fact(parent("homer", "bart")),
            fact(parent("homer", "lisa")),
            fact(parent("marge", "bart")),
        )

        updated = retract_all(family, parent("homer", x))

        assert solve_all(updated, x, parent("homer", x)) == []
        assert solve_all(updated, x, parent("marge", x)) == [atom("bart")]

    def test_abolish_removes_a_relation_and_keeps_other_relations(self) -> None:
        parent = relation("parent", 2)
        sibling = relation("sibling", 2)
        x = var("X")
        family = program(
            fact(parent("homer", "bart")),
            fact(parent("homer", "lisa")),
            fact(sibling("bart", "lisa")),
        )

        updated = abolish(family, parent)

        assert solve_all(updated, x, parent("homer", x)) == []
        assert solve_all(updated, x, sibling("bart", x)) == [atom("lisa")]


class TestRuntimeDynamicDatabase:
    """Runtime database overlays should be branch-local and Prolog-shaped."""

    def test_program_can_declare_dynamic_relations(self) -> None:
        parent = relation("parent", 2)
        family = declare_dynamic(program(fact(parent("homer", "bart"))), parent)

        assert family.dynamic_relations == frozenset({parent.key()})

    def test_clause_terms_round_trip_back_into_clauses(self) -> None:
        parent = relation("parent", 2)
        child = relation("child", 1)
        x = var("X")

        fact_clause = clause_from_term(term("parent", "homer", "bart"))
        rule_clause = clause_from_term(
            term(":-", term("child", x), term("parent", x, "homer")),
        )

        assert fact_clause == fact(parent("homer", "bart"))
        assert rule_clause == rule(child(x), parent(x, "homer"))

    def test_runtime_assertion_is_visible_from_later_goals(self) -> None:
        edge = relation("edge", 2)
        x = var("X")
        initial = State()

        dynamic_state = runtime_declare_dynamic(program(), initial, edge)
        assert dynamic_state is not None
        asserted_state = runtime_assertz(program(), dynamic_state, fact(edge("a", "b")))
        assert asserted_state is not None

        assert solve_all(program(), x, edge("a", x)) == []
        assert [
            state.substitution.reify(x)
            for state in solve_from(program(), edge("a", x), asserted_state)
        ] == [atom("b")]

    def test_runtime_retract_can_remove_program_dynamic_clauses(self) -> None:
        parent = relation("parent", 2)
        x = var("X")
        y = var("Y")
        family = program(
            fact(parent("homer", "bart")),
            fact(parent("homer", "lisa")),
            dynamic_relations=(parent,),
        )

        states = list(
            runtime_retract_first(family, State(), fact(parent("homer", x))),
        )

        assert len(states) == 1
        assert states[0].substitution.reify(x) == atom("bart")
        assert [
            state.substitution.reify(y)
            for state in solve_from(family, parent("homer", y), states[0])
        ] == [atom("lisa")]

    def test_visible_clauses_include_dynamic_overlay_order(self) -> None:
        item = relation("item", 1)
        base = program(fact(item("base")), dynamic_relations=(item,))
        state = State()
        first = runtime_assertz(base, state, fact(item("last")))
        assert first is not None
        second = runtime_assertz(base, first, fact(item("tail")))
        assert second is not None

        observed = [
            clause.head.args[0]
            for clause in visible_clauses_for(base, item, second)
        ]
        assert observed == [
            atom("base"),
            atom("last"),
            atom("tail"),
        ]

    def test_database_updates_do_not_mutate_the_original_program(self) -> None:
        parent = relation("parent", 2)
        x = var("X")
        original = program(fact(parent("homer", "bart")))
        asserted = assertz(original, fact(parent("homer", "lisa")))
        retracted = retract_all(asserted, parent("homer", x))

        assert solve_all(original, x, parent("homer", x)) == [atom("bart")]
        assert solve_all(asserted, x, parent("homer", x)) == [
            atom("bart"),
            atom("lisa"),
        ]
        assert solve_all(retracted, x, parent("homer", x)) == []

    def test_database_helpers_validate_programs_and_patterns(self) -> None:
        parent = relation("parent", 2)

        with pytest.raises(TypeError):
            clauses_matching(object(), parent("homer", "bart"))

        with pytest.raises(TypeError):
            clauses_matching(program(), object())

        with pytest.raises(TypeError):
            retract_all(program(), object())

        with pytest.raises(TypeError):
            abolish(program(), object())


class TestClauseTermIntrospection:
    """Clauses and goals should be inspectable as first-order term data."""

    def test_clause_body_uses_truth_for_facts(self) -> None:
        parent = relation("parent", 2)
        x = var("X")
        y = var("Y")

        assert clause_body(fact(parent("homer", "bart"))) == succeed()
        assert clause_body(rule(parent(x, y), parent(y, x))) == parent(y, x)

    def test_goal_as_term_encodes_representable_goals(self) -> None:
        parent = relation("parent", 2)
        x = var("X")
        y = var("Y")

        assert goal_as_term(parent(x, y)) == term("parent", x, y)
        assert goal_as_term(succeed()) == atom("true")
        assert goal_as_term(fail()) == atom("fail")
        assert goal_as_term(cut()) == atom("!")
        assert goal_as_term(eq(x, "homer")) == term("=", x, "homer")
        assert goal_as_term(neq(x, "homer")) == term("\\=", x, "homer")
        encoded_conjunction = goal_as_term(
            conj(parent(x, y), eq(x, "homer"), neq(y, "bart")),
        )

        assert encoded_conjunction == term(
            ",",
            term("parent", x, y),
            term(",", term("=", x, "homer"), term("\\=", y, "bart")),
        )
        assert goal_as_term(disj(eq(x, "homer"), fail())) == term(
            ";",
            term("=", x, "homer"),
            atom("fail"),
        )

    def test_goal_as_term_rejects_host_only_goals(self) -> None:
        x = var("X")

        def passthrough(
            _program: Program,
            state: State,
            _args: tuple[Term, ...],
        ) -> Iterator[State]:
            yield state

        with pytest.raises(TypeError):
            goal_as_term(fresh(1, lambda inner: eq(inner, "tea")))

        with pytest.raises(TypeError):
            goal_as_term(defer(eq, "tea", "tea"))

        with pytest.raises(TypeError):
            goal_as_term(native_goal(passthrough, x))

    def test_goal_from_term_lowers_representable_goal_terms(self) -> None:
        parent = relation("parent", 2)
        child = var("Child")
        family = program(
            fact(parent("homer", "bart")),
            fact(parent("homer", "lisa")),
        )

        assert solve_all(
            family,
            child,
            goal_from_term(term("parent", "homer", child)),
        ) == [atom("bart"), atom("lisa")]
        assert solve_all(
            program(),
            child,
            goal_from_term(term("=", child, "tea")),
        ) == [atom("tea")]
        assert solve_all(
            program(),
            child,
            goal_from_term(
                term(",", term("=", child, "tea"), term("\\=", child, "cake")),
            ),
        ) == [atom("tea")]
        assert solve_all(
            program(),
            child,
            goal_from_term(
                term(";", term("=", child, "tea"), term("=", child, "cake")),
            ),
        ) == [atom("tea"), atom("cake")]
        assert solve_all(
            program(),
            child,
            conj(
                goal_from_term(
                    term(";", term("=", child, "tea"), term("=", child, "cake")),
                ),
                goal_from_term(atom("!")),
            ),
        ) == [atom("tea")]

    def test_goal_from_term_lowers_callable_atoms_to_zero_arity_relations(self) -> None:
        ready = relation("ready", 0)

        assert list(solve(program(fact(ready())), goal_from_term(atom("ready"))))
        assert list(solve(program(), goal_from_term(atom("true"))))
        assert not list(solve(program(), goal_from_term(atom("fail"))))

    def test_goal_from_term_rejects_non_callable_or_malformed_terms(self) -> None:
        with pytest.raises(TypeError):
            goal_from_term(num(1))
        with pytest.raises(TypeError):
            goal_from_term(string("not-callable"))
        with pytest.raises(TypeError):
            goal_from_term(var("Goal"))
        with pytest.raises(TypeError):
            goal_from_term(term(",", "true"))

    def test_clause_as_term_encodes_facts_and_rules(self) -> None:
        parent = relation("parent", 2)
        child = relation("child", 2)
        x = var("X")
        y = var("Y")

        assert clause_as_term(fact(parent("homer", "bart"))) == term(
            ":-",
            term("parent", "homer", "bart"),
            atom("true"),
        )
        assert clause_as_term(rule(child(x, y), parent(y, x))) == term(
            ":-",
            term("child", x, y),
            term("parent", y, x),
        )

    def test_freshen_clause_standardizes_apart_and_preserves_aliasing(self) -> None:
        parent = relation("parent", 2)
        x = var("X")
        source = rule(parent(x, x), parent(x, "bart"))

        freshened, next_var_id = freshen_clause(source, 3)

        assert next_var_id == 4
        first, second = freshened.head.args
        assert isinstance(first, LogicVar)
        assert first == second
        assert first != x
        assert first.id == 3
        assert freshened.body == parent(first, "bart")

    def test_freshen_clause_validates_next_var_id(self) -> None:
        parent = relation("parent", 2)
        source = fact(parent("homer", "bart"))

        with pytest.raises(TypeError):
            freshen_clause(source, 1.5)  # type: ignore[arg-type]

        with pytest.raises(TypeError):
            freshen_clause(source, True)  # type: ignore[arg-type]

        with pytest.raises(ValueError):
            freshen_clause(source, -1)

        with pytest.raises(TypeError):
            freshen_clause(object(), 0)


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


class TestSearchControl:
    """Cut should prune scoped choicepoints without replacing ordinary search."""

    def test_cut_prunes_prior_query_choicepoints(self) -> None:
        item = var("Item")

        assert solve_all(
            program(),
            item,
            conj(disj(eq(item, "first"), eq(item, "second")), cut()),
        ) == [atom("first")]

    def test_cut_keeps_choices_created_after_it(self) -> None:
        item = var("Item")

        assert solve_all(
            program(),
            item,
            conj(cut(), disj(eq(item, "first"), eq(item, "second"))),
        ) == [atom("first"), atom("second")]

    def test_cut_inside_rule_prunes_earlier_body_choices_and_later_clauses(
        self,
    ) -> None:
        pick = relation("pick", 1)
        x = var("X")
        options = program(
            rule(pick(x), conj(disj(eq(x, "first"), eq(x, "second")), cut())),
            fact(pick("fallback")),
        )

        assert solve_all(options, x, pick(x)) == [atom("first")]

    def test_cut_still_prunes_when_later_goal_fails(self) -> None:
        pick = relation("pick", 1)
        x = var("X")
        options = program(
            rule(pick("blocked"), conj(cut(), fail())),
            fact(pick("fallback")),
        )

        assert solve_all(options, x, pick(x)) == []

    def test_cut_inside_relation_does_not_prune_caller_choicepoints(self) -> None:
        committed = relation("committed", 1)
        side = var("Side")
        value = var("Value")
        x = var("X")
        options = program(
            rule(
                committed(x),
                conj(disj(eq(x, "inner-first"), eq(x, "inner-second")), cut()),
            ),
            fact(committed("later-clause")),
        )

        assert solve_all(
            options,
            (side, value),
            disj(
                conj(eq(side, "left"), committed(value)),
                conj(eq(side, "right"), eq(value, "outside")),
            ),
        ) == [(atom("left"), atom("inner-first")), (atom("right"), atom("outside"))]

    def test_cut_inside_recursive_predicate_commits_one_recursive_frame(
        self,
    ) -> None:
        member_once = relation("member_once", 2)
        item = var("Item")
        head = var("Head")
        tail = var("Tail")
        marker = var("Marker")
        members = program(
            rule(member_once(item, term(".", item, tail)), cut()),
            rule(
                member_once(item, term(".", head, tail)),
                member_once(item, tail),
            ),
        )

        assert solve_all(
            members,
            marker,
            conj(
                member_once("b", logic_list(["a", "b", "b"])),
                eq(marker, "hit"),
            ),
        ) == [atom("hit")]


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

    def test_native_goal_can_inspect_and_extend_current_state(self) -> None:
        x = var("X")
        trivial = program()

        def bind_bart(
            _program: Program,
            state: State,
            args: tuple[Term, ...],
        ) -> Iterator[State]:
            (target,) = args
            yield from solve_from(_program, eq(target, "bart"), state)

        assert solve_all(trivial, x, native_goal(bind_bart, x)) == [atom("bart")]

    def test_native_goal_arguments_are_renamed_inside_fresh_scopes(self) -> None:
        trivial = program()

        def bind_bart(
            _program: Program,
            state: State,
            args: tuple[Term, ...],
        ) -> Iterator[State]:
            (target,) = args
            yield from solve_from(_program, eq(target, "bart"), state)

        assert solve_all(
            trivial,
            logic_list([]),
            fresh(1, lambda x: native_goal(bind_bart, x)),
        ) == [logic_list([])]

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
