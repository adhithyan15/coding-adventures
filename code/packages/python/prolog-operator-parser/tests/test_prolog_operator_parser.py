"""Tests for the operator-aware token-level Prolog parser."""

from __future__ import annotations

import pytest
from logic_engine import (
    Compound,
    atom,
    goal_as_term,
    logic_list,
    relation,
    solve_all,
    term,
)
from prolog_core import iso_operator_table
from prolog_lexer import tokenize_prolog
from prolog_parser import PrologParseError

from prolog_operator_parser import (
    __version__,
    parse_operator_goal_tokens,
    parse_operator_named_term_tokens,
    parse_operator_program_tokens,
    parse_operator_query_tokens,
    parse_operator_source_tokens,
    parse_operator_term_tokens,
)


class TestVersion:
    """Verify the package is importable and versioned."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.1.0"


class TestOperatorParsing:
    """Operator-aware parsing should honor precedence tables and source shapes."""

    def test_parse_term_respects_operator_precedence(self) -> None:
        parsed = parse_operator_term_tokens(
            tokenize_prolog("1 + 2 * 3"),
            iso_operator_table(),
        )

        assert parsed == term("+", 1, term("*", 2, 3))

    def test_parse_term_supports_lists_prefix_and_postfix(self) -> None:
        table = iso_operator_table().define(300, "fy", "~").define(350, "yf", "after")

        parsed = parse_operator_term_tokens(
            tokenize_prolog("[head, tail | Rest] = ~ value after"),
            table,
        )

        assert str(parsed) == "=(.(head, .(tail, Rest)), after(~(value)))"

    def test_parse_term_keeps_grouping_parentheses(self) -> None:
        parsed = parse_operator_term_tokens(
            tokenize_prolog("(1 + 2) * 3"),
            iso_operator_table(),
        )

        assert parsed == term("*", term("+", 1, 2), 3)

    def test_parse_named_term_returns_variables(self) -> None:
        parsed = parse_operator_named_term_tokens(
            tokenize_prolog("pair(X, Y, X, _)"),
            iso_operator_table(),
        )

        assert isinstance(parsed.term, Compound)
        assert parsed.term == term(
            "pair",
            parsed.variables["X"],
            parsed.variables["Y"],
            parsed.variables["X"],
            parsed.term.args[3],
        )
        assert list(parsed.variables) == ["X", "Y"]

    def test_parse_goal_lowers_control_and_relational_operators(self) -> None:
        parsed = parse_operator_goal_tokens(
            tokenize_prolog("X is 1 + 2 * 3, Y = done"),
            iso_operator_table(),
        )

        assert str(goal_as_term(parsed.goal)) == ",(is(X, +(1, *(2, 3))), =(Y, done))"

    def test_parse_query_handles_custom_operator_table(self) -> None:
        table = iso_operator_table().define(500, "yfx", "++")
        parsed = parse_operator_query_tokens(
            tokenize_prolog("?- Result = a ++ b ++ c.\n"),
            table,
        )

        assert str(goal_as_term(parsed.goal)) == "=(Result, ++(++(a, b), c))"

    def test_parse_source_collects_directives_queries_and_clauses(self) -> None:
        parsed = parse_operator_source_tokens(
            tokenize_prolog(
                ":- setup(a + b).\n"
                "parent(homer, bart).\n"
                "parent(bart, lisa).\n"
                "ancestor(X, Y) :- parent(X, Y).\n"
                "ancestor(X, Y) :- parent(X, Z), ancestor(Z, Y).\n"
                "?- ancestor(homer, Who).\n"
            ),
            iso_operator_table(),
            allow_directives=True,
        )

        query = parsed.queries[0]
        assert len(parsed.directives) == 1
        assert str(parsed.directives[0].term) == "setup(+(a, b))"
        assert parsed.operator_table.get("+", "yfx") is not None
        assert len(parsed.clauses) == 4
        assert solve_all(parsed.program, query.variables["Who"], query.goal) == [
            atom("bart"),
            atom("lisa"),
        ]

    def test_parse_source_applies_op_directives_to_following_clauses(self) -> None:
        parsed = parse_operator_source_tokens(
            tokenize_prolog(
                ":- op(500, yfx, ++).\n"
                "value(Result) :- Result = a ++ b ++ c.\n"
                "?- value(Result).\n"
            ),
            iso_operator_table(),
            allow_directives=True,
        )

        query = parsed.queries[0]
        assert parsed.operator_table.get("++", "yfx") is not None
        assert str(goal_as_term(query.goal)) == "value(Result)"
        assert solve_all(parsed.program, query.variables["Result"], query.goal) == [
            term("++", term("++", "a", "b"), "c"),
        ]

    def test_parse_source_tracks_predicate_properties_and_initialization(self) -> None:
        parsed = parse_operator_source_tokens(
            tokenize_prolog(
                ":- dynamic(parent/2).\n"
                ":- multifile([parent/2, helper/1]).\n"
                ":- discontiguous(parent/2).\n"
                ":- initialization(main).\n"
                "parent(homer, bart).\n"
                "?- parent(homer, Who).\n"
            ),
            iso_operator_table(),
            allow_directives=True,
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

    def test_parse_source_expands_dcg_rules_into_executable_clauses(self) -> None:
        parsed = parse_operator_source_tokens(
            tokenize_prolog(
                "letters --> [a], [b].\n"
                "?- letters(Input, []).\n"
            ),
            iso_operator_table(),
            allow_directives=True,
        )

        query = parsed.queries[0]

        assert solve_all(parsed.program, query.variables["Input"], query.goal) == [
            logic_list(["a", "b"]),
        ]

    def test_parse_source_supports_braced_dcg_goals(self) -> None:
        parsed = parse_operator_source_tokens(
            tokenize_prolog(
                "pick(X) --> { X = a ; X = b }, [X].\n"
                "?- pick(Result, [b], []).\n"
            ),
            iso_operator_table(),
            allow_directives=True,
        )

        query = parsed.queries[0]

        assert solve_all(parsed.program, query.variables["Result"], query.goal) == [
            atom("b"),
        ]

    def test_parse_source_removes_operators_after_op_zero(self) -> None:
        with pytest.raises(PrologParseError, match="expected RPAREN"):
            parse_operator_source_tokens(
                tokenize_prolog(
                    ":- op(500, yfx, ++).\n"
                    "value(a ++ b).\n"
                    ":- op(0, yfx, ++).\n"
                    "broken(a ++ b).\n"
                ),
                iso_operator_table(),
                allow_directives=True,
            )

    def test_parse_program_rejects_queries(self) -> None:
        with pytest.raises(PrologParseError, match="expected only clauses"):
            parse_operator_program_tokens(
                tokenize_prolog("parent(homer, bart).\n?- parent(homer, Who).\n"),
                iso_operator_table(),
            )

    def test_parse_query_rejects_non_query_sources(self) -> None:
        with pytest.raises(PrologParseError, match="top-level query"):
            parse_operator_query_tokens(
                tokenize_prolog("parent(homer, bart).\n"),
                iso_operator_table(),
            )

    def test_parse_goal_rejects_non_callable_terms(self) -> None:
        with pytest.raises(
            PrologParseError,
            match="cannot lower Number into a callable goal",
        ):
            parse_operator_goal_tokens(tokenize_prolog("42"), iso_operator_table())

    def test_parse_term_rejects_trailing_tokens(self) -> None:
        with pytest.raises(PrologParseError, match="unexpected tokens after term"):
            parse_operator_term_tokens(
                tokenize_prolog("1 + 2.\n"),
                iso_operator_table(),
            )

    def test_parse_source_rejects_non_callable_clause_heads(self) -> None:
        with pytest.raises(PrologParseError, match="clause head must be callable"):
            parse_operator_source_tokens(
                tokenize_prolog("42 :- true.\n"),
                iso_operator_table(),
            )

    def test_parse_source_rejects_non_callable_dcg_heads(self) -> None:
        with pytest.raises(PrologParseError, match="DCG head must be callable"):
            parse_operator_source_tokens(
                tokenize_prolog("42 --> [a].\n"),
                iso_operator_table(),
            )

    def test_parse_term_rejects_empty_stream(self) -> None:
        with pytest.raises(PrologParseError, match="expected term"):
            parse_operator_term_tokens([], iso_operator_table())
