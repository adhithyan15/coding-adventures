"""Tests for parsing SWI-Prolog syntax into executable logic programs."""

from __future__ import annotations

import pytest
from logic_engine import atom, goal_as_term, logic_list, relation, solve_all, term

from swi_prolog_parser import (
    SWI_PROLOG_GRAMMAR_PATH,
    PrologParseError,
    __version__,
    create_swi_prolog_parser,
    parse_swi_ast,
    parse_swi_program,
    parse_swi_query,
    parse_swi_source,
    parse_swi_term,
)


class TestVersion:
    """Verify the package is importable and versioned."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.1.0"


class TestSwiParser:
    """The SWI parser should own its grammar and execute through lowering."""

    def test_uses_swi_parser_grammar_path(self) -> None:
        assert SWI_PROLOG_GRAMMAR_PATH.name == "swi.grammar"
        assert SWI_PROLOG_GRAMMAR_PATH.parent.name == "prolog"

    def test_create_swi_prolog_parser(self) -> None:
        ast = create_swi_prolog_parser("parent(homer, bart).\n").parse()

        assert ast.rule_name == "program"

    def test_parse_swi_ast(self) -> None:
        ast = parse_swi_ast(":- initialization(main).\n?- parent(homer, Who).\n")

        assert ast.rule_name == "program"
        assert len(ast.children) == 2

    def test_parse_swi_ast_accepts_dcg_rules(self) -> None:
        ast = parse_swi_ast("digits --> [a], [b].\n")

        assert ast.rule_name == "program"
        assert len(ast.children) == 1

    def test_parse_swi_source_collects_directives_and_executes_program(self) -> None:
        parsed = parse_swi_source(
            """
            :- initialization(main).
            parent(homer, bart).
            parent(bart, lisa).
            ancestor(X, Y) :- parent(X, Y).
            ancestor(X, Y) :- parent(X, Z), ancestor(Z, Y).
            ?- ancestor(homer, Who).
            """,
        )

        query = parsed.queries[0]
        assert len(parsed.directives) == 1
        assert parsed.operator_table.get(":", "xfy") is not None
        assert parsed.directives[0].term == term("initialization", "main")
        assert str(parsed.directives[0].relation) == "initialization/1"
        assert solve_all(
            parsed.program,
            query.variables["Who"],
            query.goal,
        ) == [atom("bart"), atom("lisa")]

    def test_parse_swi_source_skips_swi_comments(self) -> None:
        parsed = parse_swi_source(
            """
            % line comment
            parent(homer, bart). /* block comment */
            ?- parent(homer, Who).
            """,
        )
        query = parsed.queries[0]

        assert solve_all(
            parsed.program,
            query.variables["Who"],
            query.goal,
        ) == [atom("bart")]

    def test_parse_swi_program_allows_directives(self) -> None:
        program = parse_swi_program(":- initialization(main).\nparent(homer, bart).\n")
        query = parse_swi_query("?- parent(homer, Who).\n")

        assert solve_all(
            program,
            query.variables["Who"],
            query.goal,
        ) == [atom("bart")]

    def test_parse_swi_source_parses_operator_directive_terms(self) -> None:
        parsed = parse_swi_source(":- initialization(main + extra).\n")

        assert str(parsed.directives[0].term) == "initialization(+(main, extra))"

    def test_parse_swi_source_applies_op_directives_file_locally(self) -> None:
        parsed = parse_swi_source(
            """
            :- op(500, yfx, ++).
            value(Result) :- Result = a ++ b ++ c.
            ?- value(Result).
            """,
        )

        query = parsed.queries[0]
        assert len(parsed.directives) == 1
        assert str(parsed.directives[0].term) == "op(500, yfx, ++)"
        assert parsed.operator_table.get("++", "yfx") is not None
        assert solve_all(
            parsed.program,
            query.variables["Result"],
            query.goal,
        ) == [term("++", term("++", "a", "b"), "c")]

    def test_parse_swi_source_tracks_predicate_registry_metadata(self) -> None:
        parsed = parse_swi_source(
            """
            :- dynamic(parent/2).
            :- discontiguous(parent/2).
            :- multifile([parent/2, helper/1]).
            :- initialization(main).
            parent(homer, bart).
            ?- parent(homer, Who).
            """,
        )

        parent = parsed.predicate_registry.get("parent", 2)
        helper = parsed.predicate_registry.get("helper", 1)

        assert parent is not None
        assert helper is not None
        assert parent.dynamic is True
        assert parent.discontiguous is True
        assert parent.multifile is True
        assert helper.multifile is True
        assert parsed.program.dynamic_relations == frozenset(
            {relation("parent", 2).key()},
        )
        assert parsed.predicate_registry.initialization_directives[0].term == term(
            "initialization",
            "main",
        )

    def test_parse_swi_query_understands_operator_terms(self) -> None:
        query = parse_swi_query("?- X is 1 + 2 * 3.\n")

        assert str(goal_as_term(query.goal)) == "is(X, +(1, *(2, 3)))"

    def test_parse_swi_term_returns_named_variables(self) -> None:
        parsed = parse_swi_term("pair(X, Y, X)")

        assert parsed.term == term(
            "pair",
            parsed.variables["X"],
            parsed.variables["Y"],
            parsed.variables["X"],
        )
        assert list(parsed.variables) == ["X", "Y"]

    def test_parse_swi_query_understands_clpfd_infix_forms(self) -> None:
        query = parse_swi_query("?- [X,Y] ins 1..4, Z #= X + Y, X #< Y.\n")

        assert str(goal_as_term(query.goal)) == (
            ",(ins(.(X, .(Y, [])), ..(1, 4)), ,(#=(Z, +(X, Y)), #<(X, Y)))"
        )

    def test_parse_swi_ast_accepts_clpfd_infix_forms(self) -> None:
        ast = parse_swi_ast("?- X in 1..4, Y #= X + 1, Y #=< 4.\n")

        assert ast.rule_name == "program"
        assert len(ast.children) == 1

    def test_parse_swi_program_rejects_queries(self) -> None:
        with pytest.raises(PrologParseError, match="expected only clauses"):
            parse_swi_program("?- true.\n")

    def test_parse_swi_query_rejects_directives(self) -> None:
        with pytest.raises(PrologParseError, match="directive"):
            parse_swi_query(":- initialization(main).\n")

    def test_parse_swi_query_rejects_clauses(self) -> None:
        with pytest.raises(PrologParseError, match="expected only a query"):
            parse_swi_query("parent(homer, bart).\n")

    def test_parse_swi_source_executes_dcg_rules(self) -> None:
        parsed = parse_swi_source(
            """
            digits --> [a], [b].
            ?- digits(Input, []).
            """,
        )

        query = parsed.queries[0]

        assert solve_all(parsed.program, query.variables["Input"], query.goal) == [
            logic_list(["a", "b"]),
        ]
