"""Tests for parsing ISO/Core Prolog syntax into executable logic programs."""

from __future__ import annotations

import pytest
from logic_engine import atom, goal_as_term, logic_list, relation, solve_all, term
from prolog_core import iso_operator_table

from iso_prolog_parser import (
    ISO_PROLOG_GRAMMAR_PATH,
    PrologParseError,
    __version__,
    create_iso_prolog_parser,
    parse_iso_ast,
    parse_iso_program,
    parse_iso_query,
    parse_iso_source,
)


class TestVersion:
    """Verify the package is importable and versioned."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.1.0"


class TestIsoParser:
    """The ISO/Core parser should own its grammar and execute through lowering."""

    def test_uses_iso_parser_grammar_path(self) -> None:
        assert ISO_PROLOG_GRAMMAR_PATH.name == "iso.grammar"
        assert ISO_PROLOG_GRAMMAR_PATH.parent.name == "prolog"

    def test_create_iso_prolog_parser(self) -> None:
        ast = create_iso_prolog_parser("parent(homer, bart).\n").parse()

        assert ast.rule_name == "program"

    def test_parse_iso_ast(self) -> None:
        ast = parse_iso_ast("parent(homer, bart).\n?- parent(homer, Who).\n")

        assert ast.rule_name == "program"
        assert len(ast.children) == 2

    def test_parse_iso_ast_accepts_dcg_rules(self) -> None:
        ast = parse_iso_ast("digits --> [a], [b].\n")

        assert ast.rule_name == "program"
        assert len(ast.children) == 1

    def test_parse_iso_source_executes_recursive_program(self) -> None:
        parsed = parse_iso_source(
            """
            parent(homer, bart).
            parent(bart, lisa).
            ancestor(X, Y) :- parent(X, Y).
            ancestor(X, Y) :- parent(X, Z), ancestor(Z, Y).
            ?- ancestor(homer, Who).
            """,
        )

        query = parsed.queries[0]
        assert parsed.directives == ()
        assert parsed.operator_table.get(":-", "xfx") is not None
        assert solve_all(
            parsed.program,
            query.variables["Who"],
            query.goal,
        ) == [atom("bart"), atom("lisa")]

    def test_parse_iso_program_rejects_queries(self) -> None:
        with pytest.raises(PrologParseError, match="expected only clauses"):
            parse_iso_program("?- true.\n")

    def test_parse_iso_query_rejects_clauses(self) -> None:
        with pytest.raises(PrologParseError, match="expected only a query"):
            parse_iso_query("parent(homer, bart).\n")

    def test_parse_iso_query_helper(self) -> None:
        query = parse_iso_query("?- parent(homer, Who).\n")
        program = parse_iso_program("parent(homer, bart).\n")

        assert solve_all(
            program,
            query.variables["Who"],
            query.goal,
        ) == [atom("bart")]

    def test_parse_iso_query_understands_operator_terms(self) -> None:
        query = parse_iso_query("?- X is 1 + 2 * 3.\n")

        assert str(goal_as_term(query.goal)) == "is(X, +(1, *(2, 3)))"

    def test_parse_iso_query_accepts_custom_operator_table(self) -> None:
        table = iso_operator_table().define(500, "yfx", "++")
        query = parse_iso_query("?- Result = a ++ b ++ c.\n", operator_table=table)

        assert str(goal_as_term(query.goal)) == "=(Result, ++(++(a, b), c))"

    def test_parse_iso_source_applies_op_directives_file_locally(self) -> None:
        parsed = parse_iso_source(
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

    def test_parse_iso_source_tracks_predicate_registry_metadata(self) -> None:
        parsed = parse_iso_source(
            """
            :- dynamic(parent/2).
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
        assert parent.multifile is True
        assert helper.multifile is True
        assert parsed.program.dynamic_relations == frozenset(
            {relation("parent", 2).key()},
        )
        assert parsed.predicate_registry.initialization_directives[0].term == term(
            "initialization",
            "main",
        )

    def test_parse_iso_source_executes_dcg_rules(self) -> None:
        parsed = parse_iso_source(
            """
            digits --> [a], [b].
            ?- digits(Input, []).
            """,
        )

        query = parsed.queries[0]

        assert solve_all(parsed.program, query.variables["Input"], query.goal) == [
            logic_list(["a", "b"]),
        ]
