"""Stress parity tests for Prolog programs running through Logic bytecode."""

from __future__ import annotations

from logic_engine import Number, atom, logic_list, num, string, term

from prolog_vm_compiler import (
    PrologAnswer,
    compile_swi_prolog_project,
    compile_swi_prolog_source,
    run_compiled_prolog_bytecode_query,
    run_compiled_prolog_bytecode_query_answers,
    run_compiled_prolog_query,
    run_compiled_prolog_query_answers,
    run_initialized_compiled_prolog_bytecode_query_answers,
    run_initialized_compiled_prolog_query_answers,
)


def _answer_dicts(answers: list[PrologAnswer]) -> list[dict[str, object]]:
    return [answer.as_dict() for answer in answers]


def _project_answers(
    answers: list[PrologAnswer],
    *names: str,
) -> list[dict[str, object]]:
    return [
        {name: answer.as_dict()[name] for name in names}
        for answer in answers
    ]


class TestPrologBytecodeVMStress:
    """The bytecode VM should preserve mature structured VM behavior."""

    def test_recursive_path_search_matches_structured_vm(self) -> None:
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

        expected = [
            logic_list(["a", "b", "d"]),
            logic_list(["a", "c", "d"]),
        ]
        assert run_compiled_prolog_bytecode_query(compiled) == expected
        assert run_compiled_prolog_bytecode_query(compiled) == (
            run_compiled_prolog_query(compiled)
        )

    def test_modules_dcg_arithmetic_and_collections_match_structured_vm(
        self,
    ) -> None:
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

        answers = run_compiled_prolog_bytecode_query_answers(compiled)

        assert _answer_dicts(answers) == _answer_dicts(
            run_compiled_prolog_query_answers(compiled),
        )
        assert _project_answers(answers, "Older", "Numbers", "Rest", "Score") == [
            {
                "Older": logic_list(["bart"]),
                "Numbers": logic_list([3, 4]),
                "Rest": logic_list([]),
                "Score": num(7),
            },
        ]

    def test_dynamic_initialization_matches_structured_vm(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            :- initialization(dynamic(seen/1)).
            :- initialization(assertz(seen(alpha))).
            :- initialization(assertz(seen(beta))).

            ?- seen(Name).
            """,
        )

        answers = run_initialized_compiled_prolog_bytecode_query_answers(compiled)

        assert _answer_dicts(answers) == _answer_dicts(
            run_initialized_compiled_prolog_query_answers(compiled),
        )
        assert _answer_dicts(answers) == [
            {"Name": atom("alpha")},
            {"Name": atom("beta")},
        ]

    def test_exception_and_cleanup_control_match_structured_vm(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            :- dynamic(cleaned/1).

            ?- catch(throw(problem(fail)),
                     problem(Caught),
                     Value = recovered(Caught)),
               setup_call_cleanup(
                   true,
                   call_cleanup(true, assertz(cleaned(done))),
                   assertz(cleaned(setup))),
               cleaned(done),
               cleaned(setup).
            """,
        )

        answers = run_compiled_prolog_bytecode_query_answers(compiled)

        assert _answer_dicts(answers) == _answer_dicts(
            run_compiled_prolog_query_answers(compiled),
        )
        assert _answer_dicts(answers) == [
            {"Value": term("recovered", "fail"), "Caught": atom("fail")},
        ]

    def test_term_text_and_flag_builtins_match_structured_vm(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            ?- Term = pair(X, box(Y, X), tea),
               Y = cake,
               term_variables(Term, Variables),
               term_hash(Term, Hash),
               atom_chars(tea, Chars),
               number_string(42, NumberText),
               current_prolog_flag(unknown, Unknown).
            """,
        )

        answers = run_compiled_prolog_bytecode_query_answers(compiled)

        assert _answer_dicts(answers) == _answer_dicts(
            run_compiled_prolog_query_answers(compiled),
        )
        assert len(answers) == 1
        row = answers[0].as_dict()
        assert row["Term"] == term(
            "pair",
            row["X"],
            term("box", "cake", row["X"]),
            "tea",
        )
        assert row["Variables"] == logic_list([row["X"]])
        assert row["Chars"] == logic_list(["t", "e", "a"])
        assert row["NumberText"] == string("42")
        assert row["Unknown"] == atom("fail")
        assert isinstance(row["Hash"], Number)

    def test_grouped_collections_and_apply_predicates_match_structured_vm(
        self,
    ) -> None:
        compiled = compile_swi_prolog_source(
            """
            parent(homer, bart).
            parent(homer, lisa).
            parent(marge, maggie).
            small(1).
            small(2).
            increment(1, 2).
            increment(2, 3).
            increment(3, 4).

            ?- bagof(Child, parent(Parent, Child), Children),
               bagof(AnyChild, AnyParent^parent(AnyParent, AnyChild), AllChildren),
               maplist(increment, [1,2,3], Ys),
               include(small, [1,2,3], Small),
               exclude(small, [1,2,3], Big).
            """,
        )

        answers = run_compiled_prolog_bytecode_query_answers(compiled)

        assert _answer_dicts(answers) == _answer_dicts(
            run_compiled_prolog_query_answers(compiled),
        )
        assert _project_answers(
            answers,
            "Parent",
            "Children",
            "AllChildren",
            "Ys",
            "Small",
            "Big",
        ) == [
            {
                "Parent": atom("homer"),
                "Children": logic_list(["bart", "lisa"]),
                "AllChildren": logic_list(["bart", "lisa", "maggie"]),
                "Ys": logic_list([2, 3, 4]),
                "Small": logic_list([1, 2]),
                "Big": logic_list([3]),
            },
            {
                "Parent": atom("marge"),
                "Children": logic_list(["maggie"]),
                "AllChildren": logic_list(["bart", "lisa", "maggie"]),
                "Ys": logic_list([2, 3, 4]),
                "Small": logic_list([1, 2]),
                "Big": logic_list([3]),
            },
        ]

    def test_list_stdlib_predicates_match_structured_vm(self) -> None:
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
               succ(Count, NextCount),
               integer(NextCount).
            """,
        )

        assert _answer_dicts(
            run_compiled_prolog_bytecode_query_answers(compiled),
        ) == _answer_dicts(run_compiled_prolog_query_answers(compiled))

    def test_clpfd_modeling_globals_match_structured_vm(self) -> None:
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

        answers = run_compiled_prolog_bytecode_query_answers(compiled)

        assert _answer_dicts(answers) == _answer_dicts(
            run_compiled_prolog_query_answers(compiled),
        )
        assert _answer_dicts(answers) == [
            {"I": num(2), "X": num(1), "Y": num(4), "Z": num(3)},
            {"I": num(2), "X": num(2), "Y": num(4), "Z": num(1)},
            {"I": num(2), "X": num(3), "Y": num(4), "Z": num(1)},
        ]
