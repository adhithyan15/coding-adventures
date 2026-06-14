"""Lexer tests for Twig.

These tests check that the grammar-driven ``GrammarLexer``,
configured by ``code/grammars/twig.tokens``, recognises every
documented token shape and discards comments / whitespace.
"""

from __future__ import annotations

import pytest

from twig.lexer import tokenize_twig


def _types(source: str) -> list[str]:
    """Return the bare type names of every non-EOF token, as strings.

    Token types come back as ``TokenType.LPAREN`` enum members for
    the standard punctuation but as plain strings for our custom
    tokens (``INTEGER``, ``BOOL_TRUE``, etc.).  We normalise via
    ``.name`` if available.
    """
    return [
        getattr(t.type, "name", None) or str(t.type)
        for t in tokenize_twig(source)
        if str(t.type) != "EOF" and getattr(t.type, "name", "") != "EOF"
    ]


def _values(source: str) -> list:
    return [t.value for t in tokenize_twig(source) if str(t.type) != "EOF"
            and getattr(t.type, "name", "") != "EOF"]


def test_empty_source_produces_only_eof() -> None:
    toks = tokenize_twig("")
    assert all(
        getattr(t.type, "name", "") == "EOF" or str(t.type) == "EOF"
        for t in toks
    )


def test_punctuation() -> None:
    assert _types("(())") == ["LPAREN", "LPAREN", "RPAREN", "RPAREN"]


def test_quote_char() -> None:
    assert _types("'foo") == ["QUOTE", "NAME"]


def test_integer_positive() -> None:
    assert _values("42") == ["42"]


def test_integer_negative() -> None:
    assert _values("-7") == ["-7"]


def test_negative_then_name() -> None:
    """``-foo`` tokenises as a NAME (the subtraction op + foo would be
    two tokens, but the lexer's longest-match rule picks NAME because
    INTEGER's regex requires at least one digit after ``-``)."""
    types = _types("-foo")
    # Could be a single NAME, or LPAREN-style separator behaviour.
    # We just assert it lexes without an exception.
    assert types == ["NAME"]


def test_boolean_literals() -> None:
    assert _types("#t #f") == ["BOOL_TRUE", "BOOL_FALSE"]


def test_keyword_promotion() -> None:
    """``define`` / ``lambda`` / ``let`` / ``if`` / ``begin`` /
    ``quote`` / ``nil`` are promoted to KEYWORD tokens."""
    src = "define lambda let if begin quote nil"
    types = _types(src)
    assert types == ["KEYWORD"] * 7


def test_keyword_value_preserved() -> None:
    toks = tokenize_twig("define")
    kw = [t for t in toks if str(t.type) != "EOF"
          and getattr(t.type, "name", "") != "EOF"][0]
    assert kw.value == "define"


def test_identifiers_with_punctuation() -> None:
    types = _types("null? pair? + - * / = <= >=")
    assert all(t == "NAME" for t in types)


def test_comment_is_skipped() -> None:
    src = "+ ; this is a comment\n42"
    types = _types(src)
    assert types == ["NAME", "INTEGER"]


def test_whitespace_is_skipped() -> None:
    src = "(\t\n  +\n\n42  \r\n)"
    types = _types(src)
    assert types == ["LPAREN", "NAME", "INTEGER", "RPAREN"]


def test_unknown_character_raises() -> None:
    # ``@`` is not in any token grammar rule — the lexer should fail.
    with pytest.raises(Exception):  # noqa: B017 — wrapped error from generic lexer
        tokenize_twig("(foo @ bar)")
