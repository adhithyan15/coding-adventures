"""Tests for parsing ISO/Core Prolog syntax into executable logic programs."""

from __future__ import annotations

import pytest
from logic_engine import atom, goal_as_term, solve_all
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

    def test_rejects_dcg_rules_for_now(self) -> None:
        with pytest.raises(PrologParseError, match="DCG rules"):
            parse_iso_source("digits --> digit.\n")
