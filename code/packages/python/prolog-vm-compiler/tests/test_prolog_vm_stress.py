"""End-to-end stress tests for Prolog programs running through the Logic VM."""

from __future__ import annotations

from logic_engine import atom, logic_list, num

from prolog_vm_compiler import (
    compile_swi_prolog_project,
    compile_swi_prolog_source,
    run_compiled_prolog_query,
    run_compiled_prolog_query_answers,
    run_initialized_compiled_prolog_query,
    run_initialized_compiled_prolog_query_answers,
)


class TestPrologVMStress:
    """Real Prolog source should survive parser -> loader -> compiler -> VM."""

    def test_recursive_path_search_enumerates_structured_answers(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            edge(a, b).
            edge(a, c).
            edge(b, d).
            edge(c, d).

            path(Start, End, [Start, End]) :- edge(Start, End).
            path(Start, End, [Start | Rest]) :-
                edge(Start, Mid),
                path(Mid, End, Rest).

            ?- path(a, d, Path).
            """,
        )

        assert run_compiled_prolog_query(compiled) == [
            logic_list(["a", "b", "d"]),
            logic_list(["a", "c", "d"]),
        ]

    def test_modules_dcg_arithmetic_and_collections_share_one_vm_path(self) -> None:
        compiled = compile_swi_prolog_project(
            """
            :- module(family, [parent/2, age/2, older_child/2]).
            parent(homer, bart).
            parent(homer, maggie).
            age(bart, 10).
            age(maggie, 1).
            older_child(Parent, Child) :-
                parent(Parent, Child),
                age(Child, Age),
                Age >= 10.
            """,
            """
            :- use_module(family, [older_child/2]).

            tokens --> [one], [two].

            ?- findall(Child, older_child(homer, Child), Older),
               findall(Number, (between(1, 4, Number), Number > 2), Numbers),
               phrase(tokens, [one, two], Rest),
               Score is 1 + 2 * 3,
               Score =:= 7.
            """,
        )

        answers = run_compiled_prolog_query_answers(compiled)

        assert len(answers) == 1
        answer = answers[0].as_dict()
        assert answer["Older"] == logic_list(["bart"])
        assert answer["Numbers"] == logic_list([3, 4])
        assert answer["Rest"] == logic_list([])
        assert answer["Score"] == num(7)

    def test_initialization_can_seed_dynamic_state_before_source_queries(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            :- initialization(dynamic(memo/1)).
            :- initialization(assertz(memo(booted))).

            ?- memo(Value).
            """,
        )

        assert run_compiled_prolog_query(compiled) == []
        assert run_initialized_compiled_prolog_query(compiled) == [atom("booted")]

    def test_named_answers_make_vm_results_usable_from_python(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            pick(first).
            pick(second).
            chosen(Value) :- pick(Value), !.

            ?- chosen(Chosen).
            """,
        )

        answers = run_compiled_prolog_query_answers(compiled)

        assert [answer.as_dict() for answer in answers] == [
            {"Chosen": atom("first")},
        ]

    def test_if_then_else_commits_to_first_condition_solution(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            candidate(first).
            candidate(second).

            chosen(Value) :-
                (candidate(Candidate) -> Value = Candidate ; Value = none).

            fallback(Value) :-
                (missing(Candidate) -> Value = Candidate ; Value = none).

            ?- chosen(Chosen), fallback(Fallback).
            """,
        )

        answers = run_compiled_prolog_query_answers(compiled)

        assert [answer.as_dict() for answer in answers] == [
            {"Chosen": atom("first"), "Fallback": atom("none")},
        ]

    def test_call_n_meta_calls_run_through_vm(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            pick(tea).
            pick(cake).
            pair(Name, Flavor) :-
                call(pick, Name),
                call(member, Flavor, [sweet, savory]).

            ?- call(pair, Name, Flavor).
            """,
        )

        answers = run_compiled_prolog_query_answers(compiled)

        assert [answer.as_dict() for answer in answers] == [
            {"Name": atom("tea"), "Flavor": atom("sweet")},
            {"Name": atom("tea"), "Flavor": atom("savory")},
            {"Name": atom("cake"), "Flavor": atom("sweet")},
            {"Name": atom("cake"), "Flavor": atom("savory")},
        ]

    def test_list_stdlib_predicates_run_through_vm(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            ?- member(Item, [tea, cake]),
               append([Item], [jam], Combined),
               reverse(Combined, Reversed),
               select(Item, [tea, cake, jam], Remainder),
               length(Reversed, Count),
               sort([Item, jam, Item], UniqueSorted),
               msort([Item, jam, Item], Sorted),
               nth0(1, Reversed, ZeroBased),
               nth1(2, Reversed, OneBased),
               nth0(1, Reversed, ZeroRestBased, ZeroRest),
               nth1(2, Reversed, OneRestBased, OneRest),
               length(Pair, 2),
               succ(Count, NextCount),
               integer(NextCount),
               Pair = [left, right].
            """,
        )

        answers = run_compiled_prolog_query_answers(compiled)

        assert [answer.as_dict() for answer in answers] == [
            {
                "Item": atom("tea"),
                "Combined": logic_list(["tea", "jam"]),
                "Reversed": logic_list(["jam", "tea"]),
                "Remainder": logic_list(["cake", "jam"]),
                "Count": num(2),
                "UniqueSorted": logic_list(["jam", "tea"]),
                "Sorted": logic_list(["jam", "tea", "tea"]),
                "ZeroBased": atom("tea"),
                "OneBased": atom("tea"),
                "ZeroRestBased": atom("tea"),
                "ZeroRest": logic_list(["jam"]),
                "OneRestBased": atom("tea"),
                "OneRest": logic_list(["jam"]),
                "NextCount": num(3),
                "Pair": logic_list(["left", "right"]),
            },
            {
                "Item": atom("cake"),
                "Combined": logic_list(["cake", "jam"]),
                "Reversed": logic_list(["jam", "cake"]),
                "Remainder": logic_list(["tea", "jam"]),
                "Count": num(2),
                "UniqueSorted": logic_list(["cake", "jam"]),
                "Sorted": logic_list(["cake", "cake", "jam"]),
                "ZeroBased": atom("cake"),
                "OneBased": atom("cake"),
                "ZeroRestBased": atom("cake"),
                "ZeroRest": logic_list(["jam"]),
                "OneRestBased": atom("cake"),
                "OneRest": logic_list(["jam"]),
                "NextCount": num(3),
                "Pair": logic_list(["left", "right"]),
            },
        ]

    def test_clpfd_callable_forms_run_through_vm(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            ?- ins([X,Y], [1,2,3]),
               in(Z, [1,2,3,4,5,6]),
               #<(X,Y),
               #=(Z, +(X,Y)),
               all_different([X,Y]),
               labeling([], [X,Y,Z]).
            """,
        )

        answers = run_compiled_prolog_query_answers(compiled)

        assert [answer.as_dict() for answer in answers] == [
            {"X": num(1), "Y": num(2), "Z": num(3)},
            {"X": num(1), "Y": num(3), "Z": num(4)},
            {"X": num(2), "Y": num(3), "Z": num(5)},
        ]

    def test_clpfd_infix_forms_run_through_vm(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            ?- [X,Y] ins 1..3,
               Z in 1..6,
               X #< Y,
               Z #= X + Y,
               all_different([X,Y]),
               labeling([], [X,Y,Z]).
            """,
        )

        answers = run_compiled_prolog_query_answers(compiled)

        assert [answer.as_dict() for answer in answers] == [
            {"X": num(1), "Y": num(2), "Z": num(3)},
            {"X": num(1), "Y": num(3), "Z": num(4)},
            {"X": num(2), "Y": num(3), "Z": num(5)},
        ]

    def test_clpfd_nested_sum_equality_runs_through_vm(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            ?- [X,Y] ins 1..3,
               Z in 4..6,
               X #< Y,
               Z #= X + Y + 1,
               labeling([], [X,Y,Z]).
            """,
        )

        answers = run_compiled_prolog_query_answers(compiled)

        assert [answer.as_dict() for answer in answers] == [
            {"X": num(1), "Y": num(2), "Z": num(4)},
            {"X": num(1), "Y": num(3), "Z": num(5)},
            {"X": num(2), "Y": num(3), "Z": num(6)},
        ]

    def test_clpfd_labeling_options_run_through_vm(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            ?- X in 1..3,
               labeling([down], [X]).
            """,
        )

        answers = run_compiled_prolog_query_answers(compiled)

        assert [answer.as_dict() for answer in answers] == [
            {"X": num(3)},
            {"X": num(2)},
            {"X": num(1)},
        ]

    def test_clpfd_sum_global_runs_through_vm(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            ?- [X,Y,Z] ins 1..4,
               sum([X,Y,Z], #=, 6),
               X #< Y,
               Y #< Z,
               labeling([], [X,Y,Z]).
            """,
        )

        answers = run_compiled_prolog_query_answers(compiled)

        assert [answer.as_dict() for answer in answers] == [
            {"X": num(1), "Y": num(2), "Z": num(3)},
        ]

    def test_clpfd_scalar_product_runs_through_vm(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            ?- [X,Y] ins 0..4,
               scalar_product([2,3], [X,Y], #=, 12),
               X #< Y,
               labeling([], [X,Y]).
            """,
        )

        answers = run_compiled_prolog_query_answers(compiled)

        assert [answer.as_dict() for answer in answers] == [
            {"X": num(0), "Y": num(4)},
        ]

    def test_clpfd_modeling_globals_run_through_vm(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            ?- [I,X,Y,Z] ins 1..4,
               I #= 2,
               element(I, [X,Y,Z], 4),
               sum([X,Y,Z], #=<, 8),
               scalar_product([2,1,1], [X,Y,Z], #>, 8),
               all_different([X,Y,Z]),
               labeling([], [I,X,Y,Z]).
            """,
        )

        answers = run_compiled_prolog_query_answers(compiled)

        assert [answer.as_dict() for answer in answers] == [
            {"I": num(2), "X": num(1), "Y": num(4), "Z": num(3)},
            {"I": num(2), "X": num(2), "Y": num(4), "Z": num(1)},
            {"I": num(2), "X": num(3), "Y": num(4), "Z": num(1)},
        ]

    def test_clpfd_reification_and_booleans_run_through_vm(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            ?- [X,Y,Z] ins 1..3,
               (X #< Y) #<==> A,
               (Y #< Z) #<==> B,
               (A #/\\ B) #<==> Chain,
               Chain #= 1,
               labeling([], [X,Y,Z,A,B,Chain]).
            """,
        )

        answers = run_compiled_prolog_query_answers(compiled)

        assert [answer.as_dict() for answer in answers] == [
            {
                "X": num(1),
                "Y": num(2),
                "Z": num(3),
                "A": num(1),
                "B": num(1),
                "Chain": num(1),
            },
        ]

    def test_initialized_named_answers_keep_runtime_assertions_visible(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            :- initialization(dynamic(seen/1)).
            :- initialization(assertz(seen(alpha))).
            :- initialization(assertz(seen(beta))).

            ?- seen(Name).
            """,
        )

        answers = run_initialized_compiled_prolog_query_answers(compiled)

        assert [answer.as_dict() for answer in answers] == [
            {"Name": atom("alpha")},
            {"Name": atom("beta")},
        ]

    def test_loader_expansions_are_preserved_before_vm_compilation(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            :- term_expansion(edge(X, Y), link(X, Y)).
            :- goal_expansion(run(X), link(homer, X)).

            edge(homer, bart).

            ?- run(Who).
            """,
        )

        assert run_compiled_prolog_query(compiled) == [atom("bart")]
