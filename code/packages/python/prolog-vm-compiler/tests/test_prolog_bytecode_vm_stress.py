"""Stress parity tests for Prolog programs running through Logic bytecode."""

from __future__ import annotations

from pathlib import Path

from logic_engine import Compound, Number, atom, logic_list, num, string, term

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

    def test_file_text_io_matches_structured_vm(self, tmp_path: Path) -> None:
        source_path = tmp_path / "story.pltxt"
        source_path.write_text("bytecode\ntea", encoding="utf-8")
        path_atom = str(source_path).replace("\\", "\\\\").replace("'", "\\'")
        compiled = compile_swi_prolog_source(
            f"""
            ?- exists_file('{path_atom}'),
               read_file_to_string('{path_atom}', Text),
               read_file_to_codes('{path_atom}', Codes).
            """,
        )

        answers = run_compiled_prolog_bytecode_query_answers(compiled)

        assert _answer_dicts(answers) == _answer_dicts(
            run_compiled_prolog_query_answers(compiled),
        )
        assert _answer_dicts(answers) == [
            {
                "Text": string("bytecode\ntea"),
                "Codes": logic_list([
                    num(ord(character)) for character in "bytecode\ntea"
                ]),
            },
        ]

    def test_file_stream_io_matches_structured_vm(self, tmp_path: Path) -> None:
        source_path = tmp_path / "stream.pltxt"
        path_atom = str(source_path).replace("\\", "\\\\").replace("'", "\\'")
        compiled = compile_swi_prolog_source(
            f"""
            ?- open('{path_atom}', write, Out),
               write(Out, "bytecode"),
               nl(Out),
               write(Out, tea),
               close(Out),
               open('{path_atom}', read, In),
               read_line_to_string(In, Line),
               get_char(In, Char),
               read_string(In, 2, Tail),
               at_end_of_stream(In),
               close(In).
            """,
        )

        answers = run_compiled_prolog_bytecode_query_answers(compiled)

        assert _project_answers(answers, "Line", "Char", "Tail") == _project_answers(
            run_compiled_prolog_query_answers(compiled),
            "Line",
            "Char",
            "Tail",
        )
        assert _project_answers(answers, "Line", "Char", "Tail") == [
            {
                "Line": string("bytecode"),
                "Char": atom("t"),
                "Tail": string("ea"),
            },
        ]

    def test_stream_options_and_properties_match_structured_vm(
        self,
        tmp_path: Path,
    ) -> None:
        source_path = tmp_path / "stream-options.pltxt"
        path_atom = str(source_path).replace("\\", "\\\\").replace("'", "\\'")
        compiled = compile_swi_prolog_source(
            f"""
            ?- open('{path_atom}', write, Out,
                    [alias(bytecode_report), encoding(utf8), type(text)]),
               write(bytecode_report, "tea"),
               flush_output(bytecode_report),
               stream_property(bytecode_report, alias(Alias)),
               current_stream(Path, Mode, Out),
               close(bytecode_report).
            """,
        )

        answers = run_compiled_prolog_bytecode_query_answers(compiled)

        assert _project_answers(answers, "Alias", "Path", "Mode") == _project_answers(
            run_compiled_prolog_query_answers(compiled),
            "Alias",
            "Path",
            "Mode",
        )
        assert _project_answers(answers, "Alias", "Path", "Mode") == [
            {
                "Alias": atom("bytecode_report"),
                "Path": atom(str(source_path)),
                "Mode": atom("write"),
            },
        ]

    def test_stream_positioning_matches_structured_vm(
        self,
        tmp_path: Path,
    ) -> None:
        source_path = tmp_path / "stream-position.pltxt"
        source_path.write_text("abcdef", encoding="utf-8")
        path_atom = str(source_path).replace("\\", "\\\\").replace("'", "\\'")
        compiled = compile_swi_prolog_source(
            f"""
            ?- open('{path_atom}', read, In, [alias(bytecode_position_stream)]),
               read_string(In, 2, Prefix),
               stream_property(In, position(Saved)),
               set_stream_position(bytecode_position_stream, 0),
               read_string(In, 2, Replay),
               seek(In, -1, eof, Seeked),
               stream_property(In, position(Current)),
               read_string(In, 1, Suffix),
               at_end_of_stream(In),
               close(bytecode_position_stream).
            """,
        )

        answers = run_compiled_prolog_bytecode_query_answers(compiled)

        assert _project_answers(
            answers,
            "Prefix",
            "Saved",
            "Replay",
            "Seeked",
            "Current",
            "Suffix",
        ) == _project_answers(
            run_compiled_prolog_query_answers(compiled),
            "Prefix",
            "Saved",
            "Replay",
            "Seeked",
            "Current",
            "Suffix",
        )
        assert _project_answers(
            answers,
            "Prefix",
            "Saved",
            "Replay",
            "Seeked",
            "Current",
            "Suffix",
        ) == [
            {
                "Prefix": string("ab"),
                "Saved": num(2),
                "Replay": string("ab"),
                "Seeked": num(5),
                "Current": num(5),
                "Suffix": string("f"),
            },
        ]

    def test_current_stream_facade_matches_structured_vm(
        self,
        tmp_path: Path,
    ) -> None:
        input_path = tmp_path / "current-input.pltxt"
        output_path = tmp_path / "current-output.pltxt"
        input_path.write_text("abcdef", encoding="utf-8")
        input_atom = str(input_path).replace("\\", "\\\\").replace("'", "\\'")
        output_atom = str(output_path).replace("\\", "\\\\").replace("'", "\\'")
        compiled = compile_swi_prolog_source(
            f"""
            ?- open('{input_atom}', read, In, [alias(bytecode_current_input)]),
               open('{output_atom}', write, Out, [alias(bytecode_current_output)]),
               set_input(bytecode_current_input),
               set_output(bytecode_current_output),
               current_input(CurrentIn),
               current_output(CurrentOut),
               get_char(First),
               read_string(2, Chunk),
               read_line_to_string(Line),
               at_end_of_stream,
               write("tea"),
               nl,
               write(cake(slice)),
               flush_output,
               stream_property(In, current_input),
               stream_property(Out, current_output),
               close(In),
               close(Out).
            """,
        )

        answers = run_compiled_prolog_bytecode_query_answers(compiled)

        assert _project_answers(
            answers,
            "First",
            "Chunk",
            "Line",
        ) == _project_answers(
            run_compiled_prolog_query_answers(compiled),
            "First",
            "Chunk",
            "Line",
        )
        assert len(answers) == 1
        answer = answers[0].as_dict()
        assert answer["CurrentIn"] != answer["CurrentOut"]
        assert {
            "First": answer["First"],
            "Chunk": answer["Chunk"],
            "Line": answer["Line"],
        } == {
            "First": atom("a"),
            "Chunk": string("bc"),
            "Line": string("def"),
        }
        assert output_path.read_text(encoding="utf-8") == "tea\ncake(slice)"

    def test_stream_term_io_matches_structured_vm(self, tmp_path: Path) -> None:
        input_path = tmp_path / "bytecode-terms.pltxt"
        output_path = tmp_path / "bytecode-written-terms.pltxt"
        input_path.write_text(
            "% leading layout is skipped\n"
            "box(cake).\n"
            "/* block comments are layout */\n"
            "pair(tea, X).\n",
            encoding="utf-8",
        )
        input_atom = str(input_path).replace("\\", "\\\\").replace("'", "\\'")
        output_atom = str(output_path).replace("\\", "\\\\").replace("'", "\\'")
        compiled = compile_swi_prolog_source(
            f"""
            ?- open('{input_atom}', read, In, [alias(bytecode_term_input)]),
               read(In, First),
               read_term(In, Second, [variable_names(Names), variables(Vars)]),
               read(In, Eof),
               close(In),
               open('{input_atom}', read, CurrentIn,
                    [alias(bytecode_current_term_input)]),
               open('{output_atom}', write, Out,
                    [alias(bytecode_term_output)]),
               set_input(CurrentIn),
               set_output(Out),
               read(CurrentFirst),
               read_term(CurrentSecond, []),
               write_term(Out, First, []),
               write(Out, '.'),
               nl,
               write_term(CurrentFirst, []),
               write('.'),
               nl,
               write_term(CurrentSecond, []),
               close(CurrentIn),
               close(Out).
            """,
        )

        answers = run_compiled_prolog_bytecode_query_answers(compiled)

        assert _project_answers(
            answers,
            "First",
            "Eof",
            "CurrentFirst",
        ) == _project_answers(
            run_compiled_prolog_query_answers(compiled),
            "First",
            "Eof",
            "CurrentFirst",
        )
        assert len(answers) == 1
        answer = answers[0].as_dict()
        assert answer["First"] == term("box", "cake")
        second = answer["Second"]
        assert isinstance(second, Compound)
        assert second == term("pair", "tea", second.args[1])
        assert answer["Names"] == logic_list([term("=", "X", second.args[1])])
        assert answer["Vars"] == logic_list([second.args[1]])
        assert answer["Eof"] == atom("end_of_file")
        assert answer["CurrentFirst"] == answer["First"]
        assert output_path.read_text(encoding="utf-8") == (
            "box(cake).\nbox(cake).\npair(tea, X)"
        )

    def test_term_writer_conveniences_match_structured_vm(
        self,
        tmp_path: Path,
    ) -> None:
        output_path = tmp_path / "bytecode-writer-conveniences.pltxt"
        output_atom = str(output_path).replace("\\", "\\\\").replace("'", "\\'")
        compiled = compile_swi_prolog_source(
            f"""
            ?- read_term_from_atom('pair(X, Y, X, Z)', Term,
                   [singletons(Singletons)]),
               open('{output_atom}', write, Out,
                    [alias(bytecode_writer_output)]),
               writeq(Out, 'two words'),
               nl(Out),
               write_canonical(Out, '$VAR'(0)),
               nl(Out),
               writeln(Out, line(one)),
               set_output(bytecode_writer_output),
               portray_clause(fact('$VAR'(1))),
               close(Out).
            """,
        )

        answers = run_compiled_prolog_bytecode_query_answers(compiled)

        assert len(run_compiled_prolog_query_answers(compiled)) == len(answers)
        assert len(answers) == 1
        answer = answers[0].as_dict()
        parsed_term = answer["Term"]
        assert isinstance(parsed_term, Compound)
        assert answer["Singletons"] == logic_list(
            [
                term("=", "Y", parsed_term.args[1]),
                term("=", "Z", parsed_term.args[3]),
            ],
        )
        assert output_path.read_text(encoding="utf-8") == (
            "'two words'\nA\nline(one)\nfact(B).\n"
        )
