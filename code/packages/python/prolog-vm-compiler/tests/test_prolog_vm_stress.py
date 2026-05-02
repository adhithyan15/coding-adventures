"""End-to-end stress tests for Prolog programs running through the Logic VM."""

from __future__ import annotations

from logic_engine import (
    Compound,
    Disequality,
    LogicVar,
    Number,
    atom,
    logic_list,
    num,
    string,
    term,
)

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

    def test_term_equality_predicates_run_through_vm(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            ?- X = box(tea),
               X == box(tea),
               X \\== box(cake),
               X \\= box(cake),
               Result = ok.
            """,
        )

        answers = run_compiled_prolog_query_answers(compiled)

        assert [answer.as_dict() for answer in answers] == [
            {"X": term("box", "tea"), "Result": atom("ok")},
        ]

    def test_term_equality_predicate_failures_run_through_vm(self) -> None:
        unifiable = compile_swi_prolog_source("?- X \\= box(tea).")
        identical = compile_swi_prolog_source("?- X = box(tea), X \\== box(tea).")
        equal = compile_swi_prolog_source("?- X = box(tea), X \\= box(tea).")

        assert run_compiled_prolog_query(unifiable) == []
        assert run_compiled_prolog_query(identical) == []
        assert run_compiled_prolog_query(equal) == []

    def test_term_variant_and_subsumes_predicates_run_through_vm(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            variant_ok :-
                pair(X, X) =@= pair(Y, Y),
                pair(X, X) \\=@= pair(Y, Z),
                subsumes_term(box(A), box(tea)).

            ?- variant_ok,
               Result = ok.
            """,
        )

        answers = run_compiled_prolog_query_answers(compiled)

        assert [answer.as_dict() for answer in answers] == [
            {"Result": atom("ok")},
        ]

    def test_acyclic_and_cyclic_term_predicates_run_through_vm(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            ?- Term = pair(X, box(Y, X), tea),
               acyclic_term(Term),
               \\+ cyclic_term(Term),
               Result = ok.
            """,
        )

        answers = run_compiled_prolog_query_answers(compiled)

        assert len(answers) == 1
        answer = answers[0].as_dict()
        assert answer["Term"] == term(
            "pair",
            answer["X"],
            term("box", answer["Y"], answer["X"]),
            "tea",
        )
        assert answer["Result"] == atom("ok")

    def test_unifiability_predicates_run_through_vm(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            ?- unifiable(pair(X, X), pair(tea, Y), Unifier),
               unify_with_occurs_check(Z, box(tea)),
               \\+ unify_with_occurs_check(Bad, box(Bad)).
            """,
        )

        answers = run_compiled_prolog_query_answers(compiled)

        assert len(answers) == 1
        answer = answers[0].as_dict()
        assert answer["Unifier"] == logic_list([
            term("=", answer["X"], atom("tea")),
            term("=", answer["Y"], atom("tea")),
        ])
        assert answer["Z"] == term("box", "tea")

    def test_term_variables_runs_through_vm(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            ?- Term = pair(X, box(Y, X), tea),
               Y = cake,
               term_variables(Term, Variables).
            """,
        )

        answers = run_compiled_prolog_query_answers(compiled)

        assert len(answers) == 1
        answer = answers[0].as_dict()
        assert answer["Term"] == term(
            "pair",
            answer["X"],
            term("box", "cake", answer["X"]),
            "tea",
        )
        assert answer["Variables"] == logic_list([answer["X"]])

    def test_term_hash_predicates_run_through_vm(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            ?- read_term_from_atom('pair(X, X)', VariantLeft, []),
               read_term_from_atom('pair(Y, Y)', VariantRight, []),
               read_term_from_atom('pair(X, Y)', Different, []),
               term_hash(VariantLeft, FirstHash),
               term_hash(VariantRight, SecondHash),
               term_hash(Different, DifferentHash),
               term_hash(box(tea), 2, 1000, BoundedHash).
            """,
        )

        answers = run_compiled_prolog_query_answers(compiled)

        assert len(answers) == 1
        answer = answers[0].as_dict()
        assert answer["FirstHash"] == answer["SecondHash"]
        assert answer["FirstHash"] != answer["DifferentHash"]
        bounded = answer["BoundedHash"]
        assert isinstance(bounded, Number)
        assert 0 <= bounded.value < 1000

    def test_compound_reflection_predicates_run_through_vm(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            ?- compound_name_arguments(box(tea, cake), Name, Arguments),
               compound_name_arguments(Built, box, [tea, cake]),
               compound_name_arity(pair(left, right), PairName, PairArity),
               compound_name_arity(Template, pair, 2).
            """,
        )

        answers = run_compiled_prolog_query_answers(compiled)

        assert len(answers) == 1
        answer = answers[0].as_dict()
        template = answer["Template"]
        assert answer["Name"] == atom("box")
        assert answer["Arguments"] == logic_list(["tea", "cake"])
        assert answer["Built"] == term("box", "tea", "cake")
        assert answer["PairName"] == atom("pair")
        assert answer["PairArity"] == num(2)
        assert isinstance(template, Compound)
        assert template.functor == atom("pair").symbol
        assert len(template.args) == 2
        assert all(isinstance(argument, LogicVar) for argument in template.args)
        assert template.args[0] != template.args[1]

    def test_text_conversion_predicates_run_through_vm(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            ?- atom_chars(tea, Chars),
               atom_codes(Atom, [116, 101, 97]),
               number_chars(Number, ['4', '2']),
               number_codes(Float, [51, 46, 53]),
               number_string(Parsed, "7"),
               atom_concat(tea, cup, Joined),
               atom_concat(Prefix, cup, teacup),
               atom_length(teacup, AtomLength),
               sub_atom(teacup, 3, 3, 0, SubAtom),
               atomic_list_concat([tea, 2, go], '-', AtomList),
               atomic_list_concat(Split, '-', 'tea-cup'),
               char_code(Char, 90),
               string_chars(String, [h, i]),
               string_length("hello", StringLength),
               sub_string("logic", 2, 2, 1, SubString),
               term_to_atom(pair(tea, [cup, cake]), RenderedTerm),
               atom_to_term('pair(X, tea)', ParsedTerm, Bindings),
               read_term_from_atom('pair(X, Y, X)', ReadTerm,
                   [variable_names(Names), variables(Vars)]),
               write_term_to_atom(pair(tea, [cup]), WrittenTerm,
                   [quoted(true), ignore_ops(false)]),
               read_term_from_atom('pair(X, box(Y), X)', Numbered, []),
               numbervars(Numbered, 0, NumberedEnd),
               write_term_to_atom(Numbered, NumberedText, [numbervars(true)]),
               string_codes("ok", Codes).
            """,
        )

        answers = run_compiled_prolog_query_answers(compiled)

        assert len(answers) == 1
        answer = answers[0].as_dict()
        parsed_term = answer["ParsedTerm"]
        read_term = answer["ReadTerm"]
        assert isinstance(parsed_term, Compound)
        assert isinstance(read_term, Compound)
        assert answer == {
            "Chars": logic_list(["t", "e", "a"]),
            "Atom": atom("tea"),
            "Number": num(42),
            "Float": num(3.5),
            "Parsed": num(7),
            "Joined": atom("teacup"),
            "Prefix": atom("tea"),
            "AtomLength": num(6),
            "SubAtom": atom("cup"),
            "AtomList": atom("tea-2-go"),
            "Split": logic_list(["tea", "cup"]),
            "Char": atom("Z"),
            "String": string("hi"),
            "StringLength": num(5),
            "SubString": string("gi"),
            "RenderedTerm": atom("pair(tea, [cup, cake])"),
            "ParsedTerm": term("pair", parsed_term.args[0], "tea"),
            "Bindings": logic_list([term("=", "X", parsed_term.args[0])]),
            "ReadTerm": term(
                "pair",
                read_term.args[0],
                read_term.args[1],
                read_term.args[0],
            ),
            "Names": logic_list(
                [
                    term("=", "X", read_term.args[0]),
                    term("=", "Y", read_term.args[1]),
                ],
            ),
            "Vars": logic_list([read_term.args[0], read_term.args[1]]),
            "WrittenTerm": atom("pair(tea, [cup])"),
            "Numbered": term(
                "pair",
                term("$VAR", 0),
                term("box", term("$VAR", 1)),
                term("$VAR", 0),
            ),
            "NumberedEnd": num(2),
            "NumberedText": atom("pair(A, box(B), A)"),
            "Codes": logic_list([111, 107]),
        }

    def test_current_prolog_flag_runs_through_vm(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            ?- current_prolog_flag(unknown, Unknown),
               current_prolog_flag(double_quotes, DoubleQuotes),
               current_prolog_flag(integer_rounding_function, Rounding).
            """,
        )

        answers = run_compiled_prolog_query_answers(compiled)

        assert [answer.as_dict() for answer in answers] == [
            {
                "Unknown": atom("fail"),
                "DoubleQuotes": atom("string"),
                "Rounding": atom("floor"),
            },
        ]

    def test_current_atom_runs_through_vm(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            parent(homer, bart).
            parent(marge, lisa).

            ?- current_atom(SourceAtom),
               SourceAtom = bart,
               current_atom(BuiltinAtom),
               BuiltinAtom = current_atomo.
            """,
        )

        answers = run_compiled_prolog_query_answers(compiled)

        assert [answer.as_dict() for answer in answers] == [
            {
                "SourceAtom": atom("bart"),
                "BuiltinAtom": atom("current_atomo"),
            },
        ]

    def test_current_functor_runs_through_vm(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            parent(homer, child(bart)).

            ?- current_functor(child, SourceArity),
               current_functor(current_functoro, BuiltinArity).
            """,
        )

        answers = run_compiled_prolog_query_answers(compiled)

        assert [answer.as_dict() for answer in answers] == [
            {
                "SourceArity": num(1),
                "BuiltinArity": num(2),
            },
        ]

    def test_set_prolog_flag_runs_through_vm_with_branch_scope(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            ?- ( set_prolog_flag(unknown, error)
               ; true
               ),
               current_prolog_flag(unknown, Unknown).
            """,
        )

        answers = run_compiled_prolog_query_answers(compiled)

        assert [answer.as_dict() for answer in answers] == [
            {"Unknown": atom("error")},
            {"Unknown": atom("fail")},
        ]

    def test_dif_delayed_disequality_runs_through_vm(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            ?- dif(X, tea),
               X = cake,
               dif(Left, Right),
               Left = box(tea),
               Right = box(cake).
            """,
        )
        failure = compile_swi_prolog_source("?- dif(X, tea), X = tea.")

        answers = run_compiled_prolog_query_answers(compiled)

        assert [answer.as_dict() for answer in answers] == [
            {
                "X": atom("cake"),
                "Left": term("box", "tea"),
                "Right": term("box", "cake"),
            },
        ]
        assert run_compiled_prolog_query(failure) == []

    def test_dif_residual_constraints_are_visible_on_named_answers(self) -> None:
        compiled = compile_swi_prolog_source("?- dif(X, tea).")

        answers = run_compiled_prolog_query_answers(compiled)

        assert len(answers) == 1
        binding = answers[0].as_dict()["X"]
        assert isinstance(binding, LogicVar)
        assert answers[0].residual_constraints == (
            Disequality(left=binding, right=atom("tea")),
        )

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

    def test_exception_control_runs_through_vm(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            risky(tea) :- throw(problem(tea)).
            safe(Value, Status) :-
                catch(risky(Value), problem(Caught), Status = recovered(Caught)).

            ?- safe(tea, Status),
               catch(_Result is _Missing + 1,
                     error(instantiation_error, _Context),
                     Arithmetic = recovered).
            """,
        )

        answers = run_compiled_prolog_query_answers(compiled)

        assert len(answers) == 1
        answer = answers[0].as_dict()
        assert answer["Status"] == term("recovered", "tea")
        assert answer["Arithmetic"] == atom("recovered")

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

    def test_negation_once_and_aggregation_control_run_through_vm(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            item(tea).
            item(cake).
            item(jam).
            blocked(cake).
            probe(first).
            probe(second).
            score(2).
            score(1).
            duplicate(jam).
            duplicate(tea).
            duplicate(jam).

            allowed(Item) :- item(Item), \\+ blocked(Item).
            single_probe(Probe) :- once(probe(Probe)).
            all_numbers_small :- forall(member(N, [1,2,3]), N < 4).
            all_allowed(Allowed) :- findall(Value, allowed(Value), Allowed).
            score_bag(Numbers) :- bagof(Number, score(Number), Numbers).
            unique_duplicates(Unique) :- setof(Name, duplicate(Name), Unique).

            ?- allowed(Item),
               single_probe(Probe),
               all_numbers_small,
               all_allowed(Allowed),
               score_bag(Numbers),
               unique_duplicates(Unique),
               \\+ allowed(cake).
            """,
        )
        failure = compile_swi_prolog_source(
            """
            item(tea).
            ?- \\+ item(tea).
            """,
        )

        answers = run_compiled_prolog_query_answers(compiled)

        assert [answer.as_dict() for answer in answers] == [
            {
                "Item": atom("tea"),
                "Probe": atom("first"),
                "Allowed": logic_list(["tea", "jam"]),
                "Numbers": logic_list([2, 1]),
                "Unique": logic_list(["jam", "tea"]),
            },
            {
                "Item": atom("jam"),
                "Probe": atom("first"),
                "Allowed": logic_list(["tea", "jam"]),
                "Numbers": logic_list([2, 1]),
                "Unique": logic_list(["jam", "tea"]),
            },
        ]
        assert run_compiled_prolog_query(failure) == []

    def test_grouped_bagof_setof_and_existentials_run_through_vm(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            parent(homer, bart).
            parent(homer, lisa).
            parent(marge, maggie).
            score(homer, 2).
            score(homer, 1).
            score(homer, 2).
            score(marge, 3).

            ?- bagof(Child, parent(Parent, Child), Children),
               setof(Score, score(Parent, Score), Scores),
               bagof(AnyChild, AnyParent^parent(AnyParent, AnyChild), AllChildren).
            """,
        )

        answers = run_compiled_prolog_query_answers(compiled)

        rows = [answer.as_dict() for answer in answers]
        assert [
            {
                "Parent": row["Parent"],
                "Children": row["Children"],
                "Scores": row["Scores"],
                "AllChildren": row["AllChildren"],
            }
            for row in rows
        ] == [
            {
                "Parent": atom("homer"),
                "Children": logic_list(["bart", "lisa"]),
                "Scores": logic_list([1, 2]),
                "AllChildren": logic_list(["bart", "lisa", "maggie"]),
            },
            {
                "Parent": atom("marge"),
                "Children": logic_list(["maggie"]),
                "Scores": logic_list([3]),
                "AllChildren": logic_list(["bart", "lisa", "maggie"]),
            },
        ]

    def test_higher_order_list_predicates_run_through_vm(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            small(1).
            small(2).
            increment(1, 2).
            increment(2, 3).
            increment(3, 4).
            push(Item, Acc, [Item|Acc]).

            ?- maplist(increment, [1,2,3], Ys),
               include(small, [1,2,3], Small),
               exclude(small, [1,2,3], Big),
               partition(small, [1,2,3], Yes, No),
               foldl(push, [a,b,c], [], Stack).
            """,
        )

        answers = run_compiled_prolog_query_answers(compiled)

        assert [answer.as_dict() for answer in answers] == [
            {
                "Ys": logic_list([2, 3, 4]),
                "Small": logic_list([1, 2]),
                "Big": logic_list([3]),
                "Yes": logic_list([1, 2]),
                "No": logic_list([3]),
                "Stack": logic_list(["c", "b", "a"]),
            },
        ]

    def test_apply_family_predicates_run_through_vm(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            join4(A, B, C, joined(A, B, C)).
            convert(1, one).
            convert(3, three).
            pair_push(Left, Right, Acc, [pair(Left, Right)|Acc]).
            push(Item, Acc, [Item|Acc]).

            ?- maplist(join4, [a,b], [x,y], [1,2], Joined),
               convlist(convert, [1,2,3], Converted),
               foldl(pair_push, [a,b], [x,y], [], Folded),
               scanl(push, [a,b], [], Scanned).
            """,
        )

        answers = run_compiled_prolog_query_answers(compiled)

        assert [answer.as_dict() for answer in answers] == [
            {
                "Joined": logic_list([
                    term("joined", "a", "x", 1),
                    term("joined", "b", "y", 2),
                ]),
                "Converted": logic_list(["one", "three"]),
                "Folded": logic_list([term("pair", "b", "y"), term("pair", "a", "x")]),
                "Scanned": logic_list([logic_list(["a"]), logic_list(["b", "a"])]),
            },
        ]

    def test_module_qualified_apply_family_closures_run_through_vm(self) -> None:
        compiled = compile_swi_prolog_project(
            """
            :- module(apply_helpers, [
                increment/2,
                convert/2,
                small/1,
                pair_push/4,
                push/3
            ]).
            increment(1, 2).
            increment(2, 3).
            convert(1, one).
            convert(3, three).
            small(1).
            small(2).
            pair_push(Left, Right, Acc, [pair(Left, Right)|Acc]).
            push(Item, Acc, [Item|Acc]).
            """,
            """
            :- module(app, []).
            :- use_module(apply_helpers, [
                increment/2,
                convert/2,
                small/1,
                pair_push/4,
                push/3
            ]).
            ?- maplist(increment, [1,2], Ys),
               convlist(convert, [1,2,3], Converted),
               include(small, [1,2,3], Small),
               foldl(pair_push, [a,b], [x,y], [], Folded),
               scanl(push, [a,b], [], Scanned).
            """,
        )

        answers = run_compiled_prolog_query_answers(compiled)

        assert [answer.as_dict() for answer in answers] == [
            {
                "Ys": logic_list([2, 3]),
                "Converted": logic_list(["one", "three"]),
                "Small": logic_list([1, 2]),
                "Folded": logic_list([term("pair", "b", "y"), term("pair", "a", "x")]),
                "Scanned": logic_list([logic_list(["a"]), logic_list(["b", "a"])]),
            },
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
