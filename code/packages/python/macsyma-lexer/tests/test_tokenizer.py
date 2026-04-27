"""MACSYMA lexer tests.

These tests exercise every token type in the MACSYMA grammar and verify
edge cases: percent-prefixed constants, the distinction between ``:``
(assignment), ``:=`` (definition), and ``=`` (equation), and the two
statement terminators ``;`` and ``$``.
"""

from __future__ import annotations

from macsyma_lexer import tokenize_macsyma


def types_of(source: str) -> list[str]:
    """Return the list of token type names for a given source string.

    The ``tokenize_macsyma`` function returns ``Token`` objects whose
    ``type`` is either an enum or a raw string. This helper normalizes
    them to strings (stripping the trailing ``EOF``) for cleaner
    assertions.
    """
    tokens = tokenize_macsyma(source)
    names = []
    for tok in tokens:
        name = tok.type if isinstance(tok.type, str) else tok.type.name
        if name == "EOF":
            continue
        names.append(name)
    return names


def values_of(source: str) -> list[str]:
    tokens = tokenize_macsyma(source)
    return [t.value for t in tokens if (t.type if isinstance(t.type, str) else t.type.name) != "EOF"]


# ---------------------------------------------------------------------------
# Basic tokens
# ---------------------------------------------------------------------------


def test_integer_number() -> None:
    assert types_of("42") == ["NUMBER"]
    assert values_of("42") == ["42"]


def test_float_number() -> None:
    assert types_of("3.14") == ["NUMBER"]
    assert values_of("3.14") == ["3.14"]


def test_scientific_notation() -> None:
    assert types_of("1.5e10") == ["NUMBER"]


def test_simple_name() -> None:
    assert types_of("x") == ["NAME"]
    assert values_of("x") == ["x"]


def test_percent_prefixed_name() -> None:
    # %pi, %e, %i — MACSYMA's system-defined constants.
    assert types_of("%pi") == ["NAME"]
    assert values_of("%pi") == ["%pi"]


def test_string_literal() -> None:
    assert types_of('"hello"') == ["STRING"]


# ---------------------------------------------------------------------------
# Operators
# ---------------------------------------------------------------------------


def test_arithmetic_operators() -> None:
    assert types_of("1 + 2 - 3 * 4 / 5 ^ 6") == [
        "NUMBER",
        "PLUS",
        "NUMBER",
        "MINUS",
        "NUMBER",
        "STAR",
        "NUMBER",
        "SLASH",
        "NUMBER",
        "CARET",
        "NUMBER",
    ]


def test_double_star_power() -> None:
    # MACSYMA accepts both `^` and `**` for exponentiation.
    assert types_of("x ** 2") == ["NAME", "STAREQ", "NUMBER"]


def test_assignment_vs_definition() -> None:
    # `:` is single-char; `:=` must be lexed as one two-char token.
    assert types_of("a : 5") == ["NAME", "COLON", "NUMBER"]
    assert types_of("f(x) := x") == [
        "NAME",
        "LPAREN",
        "NAME",
        "RPAREN",
        "COLONEQ",
        "NAME",
    ]


def test_equation_operator() -> None:
    # `=` is the equation operator, distinct from `:` assignment.
    assert types_of("x = 4") == ["NAME", "EQ", "NUMBER"]


def test_not_equal_hash() -> None:
    # MACSYMA uses `#` for not-equal. Unusual but correct.
    assert types_of("a # b") == ["NAME", "HASH", "NAME"]


def test_comparison_operators() -> None:
    assert types_of("a <= b >= c < d > e") == [
        "NAME",
        "LEQ",
        "NAME",
        "GEQ",
        "NAME",
        "LT",
        "NAME",
        "GT",
        "NAME",
    ]


def test_arrow_operator() -> None:
    # `->` for rule arrows. Must be tried before `-`.
    assert types_of("x -> y") == ["NAME", "ARROW", "NAME"]


# ---------------------------------------------------------------------------
# Delimiters
# ---------------------------------------------------------------------------


def test_parens_brackets_braces() -> None:
    assert types_of("( [ { } ] )") == [
        "LPAREN",
        "LBRACKET",
        "LBRACE",
        "RBRACE",
        "RBRACKET",
        "RPAREN",
    ]


def test_statement_terminators() -> None:
    # `;` displays the result; `$` suppresses it. Both are valid
    # terminators and are lexed as distinct tokens.
    assert types_of("x;") == ["NAME", "SEMI"]
    assert types_of("x$") == ["NAME", "DOLLAR"]


# ---------------------------------------------------------------------------
# Keywords
# ---------------------------------------------------------------------------


def test_boolean_keywords() -> None:
    # Keywords come through as type KEYWORD with their value in .value.
    tokens = tokenize_macsyma("x and y or not z")
    # Filter out EOF.
    non_eof = [t for t in tokens if (t.type if isinstance(t.type, str) else t.type.name) != "EOF"]
    # The shape we expect: NAME KEYWORD(and) NAME KEYWORD(or) KEYWORD(not) NAME
    types = [t.type if isinstance(t.type, str) else t.type.name for t in non_eof]
    values = [t.value for t in non_eof]
    assert types == ["NAME", "KEYWORD", "NAME", "KEYWORD", "KEYWORD", "NAME"]
    assert values == ["x", "and", "y", "or", "not", "z"]


def test_true_false_keywords() -> None:
    tokens = tokenize_macsyma("true false")
    non_eof = [t for t in tokens if (t.type if isinstance(t.type, str) else t.type.name) != "EOF"]
    values = [t.value for t in non_eof]
    assert values == ["true", "false"]


# ---------------------------------------------------------------------------
# Comments and whitespace
# ---------------------------------------------------------------------------


def test_comments_are_skipped() -> None:
    # Comments should not produce any tokens.
    assert types_of("/* this is ignored */ x") == ["NAME"]


def test_multiline_comment() -> None:
    source = """
    /* Multi-line
       comment that
       spans lines */
    42
    """
    assert types_of(source) == ["NUMBER"]


def test_whitespace_is_skipped() -> None:
    assert types_of("   x   +   y   ") == ["NAME", "PLUS", "NAME"]


def test_newlines_are_skipped() -> None:
    # MACSYMA doesn't care about newlines — statements end with `;` or `$`.
    assert types_of("x\n+\ny") == ["NAME", "PLUS", "NAME"]


# ---------------------------------------------------------------------------
# End-to-end realistic MACSYMA
# ---------------------------------------------------------------------------


def test_function_definition_and_use() -> None:
    source = "f(x) := x^2 + 1; diff(f(x), x);"
    assert types_of(source) == [
        # f ( x ) := x ^ 2 + 1 ;
        "NAME",
        "LPAREN",
        "NAME",
        "RPAREN",
        "COLONEQ",
        "NAME",
        "CARET",
        "NUMBER",
        "PLUS",
        "NUMBER",
        "SEMI",
        # diff ( f ( x ) , x ) ;
        "NAME",
        "LPAREN",
        "NAME",
        "LPAREN",
        "NAME",
        "RPAREN",
        "COMMA",
        "NAME",
        "RPAREN",
        "SEMI",
    ]


def test_list_literal() -> None:
    assert types_of("[1, 2, 3]") == [
        "LBRACKET",
        "NUMBER",
        "COMMA",
        "NUMBER",
        "COMMA",
        "NUMBER",
        "RBRACKET",
    ]
