"""Lexer: layout tokens, literals, comments and the hostile-input limits."""

from __future__ import annotations

import pytest

from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.lexer import (
    MAX_SOURCE_BYTES,
    decode_source,
    split_int,
    tokenize,
)


def kinds(text: str) -> list[str]:
    return [t.kind for t in tokenize(decode_source(text.encode()))]


def texts(text: str) -> list[str]:
    return [t.text for t in tokenize(decode_source(text.encode())) if t.kind not in ("EOF",)]


def test_layout_tokens_for_a_nested_block():
    assert kinds("warrant m\n  a\n    b\n  c\n") == [
        "NAME", "NAME", "NEWLINE", "INDENT", "NAME", "NEWLINE", "INDENT", "NAME", "NEWLINE",
        "DEDENT", "NAME", "NEWLINE", "DEDENT", "EOF",
    ]


def test_blank_and_comment_lines_make_no_layout():
    assert kinds("a\n\n  -- only a comment\nb\n") == ["NAME", "NEWLINE", "NAME", "NEWLINE", "EOF"]


def test_brackets_suppress_layout_across_lines():
    assert "INDENT" not in kinds("f(a,\n        b)\n")


def test_comment_may_follow_code_and_double_dash_is_never_subtraction():
    assert texts("a - -b -- trailing")[:4] == ["a", "-", "-", "b"]
    assert texts("a--b")[:1] == ["a"]  # everything after -- is a comment


def test_dot_is_separator_only_before_whitespace():
    toks = tokenize("forall a: T. e\nE.CASE\n")
    assert [t.kind for t in toks if t.text == "."] == ["DOTSEP", "DOT"]


def test_integer_literals_and_suffixes():
    assert split_int("5") == (5, None)
    assert split_int("255u8") == (255, "u8")
    assert split_int("0x1du16") == (0x1D, "u16")
    assert split_int("18446744073709551615u64") == ((1 << 64) - 1, "u64")
    assert split_int("7i32") == (7, "i32")


def test_bytes_literal_token():
    toks = tokenize('x == b"00ff"\n')
    assert [t.text for t in toks if t.kind == "BYTES"] == ["00ff"]
    assert [t.text for t in tokenize('b""\n') if t.kind == "BYTES"] == [""]


@pytest.mark.parametrize(
    "text",
    [
        'b"0"\n',  # odd length
        'b"AB"\n',  # uppercase hex
        'b"zz"\n',  # not hex
        'b"00\n',  # unterminated
        "007\n",  # leading zero
        "5x\n",  # malformed literal
        "0x\n",  # hex literal with no digits
        "a ! b\n",  # stray character
        "a $ b\n",  # stray character
        "(a\n",  # unbalanced
        "a)\n",  # unbalanced closing
        "a\n\tb\n",  # tab indentation
        "a\n  b\n c\n",  # inconsistent dedent
    ],
)
def test_malformed_tokens_are_lex_errors(text):
    with pytest.raises(OracleError) as err:
        tokenize(text)
    assert err.value.code in ("lex",)


def test_tab_inside_a_line_is_whitespace():
    assert texts("a\tb")[:2] == ["a", "b"]


def test_giant_integer_literal_is_a_limit_error():
    with pytest.raises(OracleError) as err:
        tokenize("1" + "0" * 4096 + "\n")
    assert err.value.code == "limit"
    tokenize("9" * 4096 + "\n")  # exactly at the limit is fine


def test_giant_hex_literal_is_a_limit_error():
    with pytest.raises(OracleError) as err:
        tokenize("0x" + "f" * 4097 + "\n")
    assert err.value.code == "limit"


def test_long_identifier_is_a_limit_error():
    with pytest.raises(OracleError) as err:
        tokenize("a" * 129 + "\n")
    assert err.value.code == "limit"
    tokenize("a" * 128 + "\n")


def test_token_count_limit():
    with pytest.raises(OracleError) as err:
        tokenize("a " * 100_001)
    assert err.value.code == "limit"


def test_source_admission():
    with pytest.raises(OracleError):
        decode_source(b"\xef\xbb\xbfwarrant m\n")
    with pytest.raises(OracleError):
        decode_source(b"warrant\x00 m\n")
    with pytest.raises(OracleError):
        decode_source(b"\xff\xfe")
    with pytest.raises(OracleError):
        decode_source(b"a\rb")
    with pytest.raises(OracleError) as err:
        decode_source(b"a" * (MAX_SOURCE_BYTES + 1))
    assert err.value.code == "source"
    assert decode_source(b"a\r\nb") == "a\nb"
