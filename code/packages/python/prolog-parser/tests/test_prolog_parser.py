"""Tests for parsing Prolog syntax into executable logic-engine objects."""

from __future__ import annotations

import pytest
from lang_parser import ASTNode
from logic_engine import (
    atom,
    conj,
    eq,
    logic_list,
    num,
    relation,
    solve_all,
    string,
    var,
)

from prolog_parser import (
    PrologParseError,
    __version__,
    lower_ast,
    lower_goal_ast,
    parse_ast,
    parse_program,
    parse_query,
    parse_source,
)


class TestVersion:
    """Verify the package is importable and versioned."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.1.0"


class TestGrammarDrivenParser:
    """The parser should use the shared Prolog grammar as its syntax source."""

    def test_parse_ast_uses_prolog_grammar(self) -> None:
        ast = parse_ast("parent(homer, bart).\n?- parent(homer, Who).\n")

        assert ast.rule_name == "program"
        assert [child.rule_name for child in ast.children] == [
            "statement",
            "statement",
        ]

    def test_lower_ast_reuses_parser_lowering(self) -> None:
        parsed = lower_ast(parse_ast("parent(homer, bart).\n"))

        assert len(parsed.clauses) == 1

    def test_lower_goal_ast_reuses_goal_lowering(self) -> None:
        ast = parse_ast("?- parent(homer, Who).\n")
        statement = next(
            child for child in ast.children if isinstance(child, ASTNode)
        )
        query_statement = next(
            child for child in statement.children if isinstance(child, ASTNode)
        )
        goal_node = next(
            child
            for child in query_statement.children
            if isinstance(child, ASTNode) and child.rule_name == "goal"
        )

        parsed = lower_goal_ast(goal_node)

        assert "Who" in parsed.variables


class TestClausesAndQueries:
    """Parsed facts, rules, and queries should execute through logic-engine."""

    def test_parse_fact_program(self) -> None:
        parsed = parse_source("parent(homer, bart).\nparent(homer, lisa).\n")
        parent = relation("parent", 2)
        child = var("Child")

        assert len(parsed.clauses) == 2
        assert parsed.queries == ()
        assert solve_all(parsed.program, child, parent("homer", child)) == [
            atom("bart"),
            atom("lisa"),
        ]

    def test_parse_recursive_rules_and_query(self) -> None:
        parsed = parse_source(
            """
            parent(homer, bart).
            parent(bart, lisa).
            ancestor(X, Y) :- parent(X, Y).
            ancestor(X, Y) :- parent(X, Z), ancestor(Z, Y).
            ?- ancestor(homer, Who).
            """,
        )
        query = parsed.queries[0]

        assert solve_all(
            parsed.program,
            query.variables["Who"],
            query.goal,
        ) == [atom("bart"), atom("lisa")]

    def test_parse_zero_arity_fact_and_query(self) -> None:
        parsed = parse_source("sunny.\n?- sunny.\n")
        marker = var("Marker")

        assert solve_all(
            parsed.program,
            marker,
            conj(parsed.queries[0].goal, eq(marker, "ok")),
        ) == [atom("ok")]

    def test_parse_query_helper(self) -> None:
        query = parse_query("?- X = homer, X \\= marge.\n")

        assert solve_all(parse_program(""), query.variables["X"], query.goal) == [
            atom("homer"),
        ]


class TestTermsAndGoals:
    """Terms, lists, grouping, disjunction, and cut should lower correctly."""

    def test_lists_lower_to_logic_lists(self) -> None:
        parsed = parse_source("value([a, b | tail]).\n")
        value = relation("value", 1)
        observed = var("Observed")

        assert solve_all(parsed.program, observed, value(observed)) == [
            logic_list(["a", "b"], tail="tail"),
        ]

    def test_named_variables_share_identity_inside_one_query(self) -> None:
        parsed = parse_source("same(a, a).\nsame(a, b).\n?- same(X, X).\n")
        query = parsed.queries[0]

        assert solve_all(parsed.program, query.variables["X"], query.goal) == [
            atom("a"),
        ]

    def test_anonymous_variables_do_not_share_identity(self) -> None:
        parsed = parse_source("pair(a, b).\n?- pair(_, _).\n")
        marker = var("Marker")

        assert solve_all(
            parsed.program,
            marker,
            conj(parsed.queries[0].goal, eq(marker, "ok")),
        ) == [atom("ok")]

    def test_grouped_disjunction_query(self) -> None:
        parsed = parse_source(
            """
            parent(homer, bart).
            parent(marge, lisa).
            ?- (parent(homer, Child); parent(marge, Child)).
            """,
        )
        query = parsed.queries[0]

        assert solve_all(
            parsed.program,
            query.variables["Child"],
            query.goal,
        ) == [atom("bart"), atom("lisa")]

    def test_cut_prunes_choices_when_parsed(self) -> None:
        parsed = parse_source(
            """
            pick(first).
            pick(second).
            ?- pick(X), !.
            """,
        )
        query = parsed.queries[0]

        assert solve_all(
            parsed.program,
            query.variables["X"],
            query.goal,
        ) == [atom("first")]

    def test_strings_numbers_and_quoted_atoms(self) -> None:
        parsed = parse_source("sample('Hello world', \"tea\", 42, 3.5).\n")
        sample = relation("sample", 4)
        first = var("First")
        second = var("Second")
        third = var("Third")
        fourth = var("Fourth")

        assert solve_all(
            parsed.program,
            (first, second, third, fourth),
            sample(first, second, third, fourth),
        ) == [(atom("Hello world"), string("tea"), num(42), num(3.5))]


class TestErrors:
    """Unsupported or malformed syntax should fail with source locations."""

    def test_rejects_dcg_rules_for_now(self) -> None:
        with pytest.raises(PrologParseError, match="DCG rules"):
            parse_source("digits --> digit.\n")

    def test_rejects_missing_clause_terminator(self) -> None:
        with pytest.raises(PrologParseError, match="DOT|Unexpected"):
            parse_source("parent(homer, bart)")

    def test_parse_program_rejects_queries(self) -> None:
        with pytest.raises(PrologParseError, match="expected only clauses"):
            parse_program("?- true.\n")

    def test_parse_query_rejects_clauses(self) -> None:
        with pytest.raises(PrologParseError, match="expected only a query"):
            parse_query("parent(homer, bart).\n")
