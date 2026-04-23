"""Tests for tetrad_lexer.tokenize().

Coverage targets: ≥95% — every token type, every error path, edge cases.

Organisation
------------
Each test function targets a specific category of input.  The helper
``tok`` returns the first non-EOF token so that single-token assertions
stay one-liners.
"""

from __future__ import annotations

import pytest

from tetrad_lexer import (
    LexError,
    Token,
    TokenType,
    tokenize,
)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def tok(src: str) -> Token:
    """Tokenize *src* and return the first (non-EOF) token."""
    tokens = tokenize(src)
    assert tokens[-1].type is TokenType.EOF
    return tokens[0]


def types(src: str) -> list[TokenType]:
    """Return the TokenType sequence (excluding trailing EOF)."""
    return [t.type for t in tokenize(src)[:-1]]


def values(src: str) -> list[object]:
    """Return the .value sequence (excluding trailing EOF)."""
    return [t.value for t in tokenize(src)[:-1]]


# ---------------------------------------------------------------------------
# 1. Empty / whitespace-only input
# ---------------------------------------------------------------------------


def test_empty_source() -> None:
    tokens = tokenize("")
    assert len(tokens) == 1
    assert tokens[0].type is TokenType.EOF
    assert tokens[0].value is None


def test_whitespace_only() -> None:
    tokens = tokenize("   \t\n\r\n  ")
    assert len(tokens) == 1
    assert tokens[0].type is TokenType.EOF


def test_eof_position_empty() -> None:
    t = tokenize("")[0]
    assert t.line == 1
    assert t.column == 1
    assert t.offset == 0


def test_eof_position_after_tokens() -> None:
    tokens = tokenize("42 ")
    eof = tokens[-1]
    assert eof.type is TokenType.EOF
    assert eof.line == 1
    assert eof.column == 4  # one past the space


# ---------------------------------------------------------------------------
# 2. Comments
# ---------------------------------------------------------------------------


def test_line_comment_alone() -> None:
    tokens = tokenize("// this is a comment")
    assert len(tokens) == 1
    assert tokens[0].type is TokenType.EOF


def test_line_comment_then_token() -> None:
    tokens = tokenize("// ignore me\n42")
    assert len(tokens) == 2
    assert tokens[0].type is TokenType.INT
    assert tokens[0].value == 42


def test_comment_mid_line() -> None:
    assert types("1 + // skip\n2") == [TokenType.INT, TokenType.PLUS, TokenType.INT]


def test_double_slash_not_division() -> None:
    # "/ /" is two SLASH tokens; "//" is a comment
    assert types("/ /") == [TokenType.SLASH, TokenType.SLASH]
    assert types("//nope") == []


# ---------------------------------------------------------------------------
# 3. Integer literals (INT)
# ---------------------------------------------------------------------------


def test_int_zero() -> None:
    t = tok("0")
    assert t.type is TokenType.INT
    assert t.value == 0


def test_int_single_digit() -> None:
    assert tok("7").value == 7


def test_int_multi_digit() -> None:
    assert tok("255").value == 255


def test_int_large() -> None:
    assert tok("1000000").value == 1_000_000


def test_int_position() -> None:
    t = tok("  42")
    assert t.type is TokenType.INT
    assert t.line == 1
    assert t.column == 3
    assert t.offset == 2


# ---------------------------------------------------------------------------
# 4. Hex literals (HEX)
# ---------------------------------------------------------------------------


def test_hex_lowercase_x() -> None:
    t = tok("0xff")
    assert t.type is TokenType.HEX
    assert t.value == 0xFF


def test_hex_uppercase_x() -> None:
    t = tok("0XFF")
    assert t.type is TokenType.HEX
    assert t.value == 0xFF


def test_hex_mixed_case_digits() -> None:
    assert tok("0xDeAdBeEf").value == 0xDEADBEEF


def test_hex_single_digit() -> None:
    assert tok("0x0").value == 0


def test_hex_all_digits() -> None:
    assert tok("0x0123456789abcdefABCDEF").value == 0x0123456789ABCDEFABCDEF


def test_hex_error_empty() -> None:
    with pytest.raises(LexError) as exc_info:
        tokenize("0x")
    assert "empty hex literal" in str(exc_info.value)


def test_hex_error_empty_uppercase() -> None:
    with pytest.raises(LexError):
        tokenize("0X")


def test_hex_error_position() -> None:
    with pytest.raises(LexError) as exc_info:
        tokenize("0x")
    err = exc_info.value
    assert err.line == 1
    assert err.column == 1


# ---------------------------------------------------------------------------
# 5. Identifiers
# ---------------------------------------------------------------------------


def test_ident_simple() -> None:
    t = tok("foo")
    assert t.type is TokenType.IDENT
    assert t.value == "foo"


def test_ident_underscore_prefix() -> None:
    t = tok("_x")
    assert t.type is TokenType.IDENT
    assert t.value == "_x"


def test_ident_all_underscore() -> None:
    t = tok("___")
    assert t.type is TokenType.IDENT
    assert t.value == "___"


def test_ident_with_digits() -> None:
    t = tok("abc123")
    assert t.type is TokenType.IDENT
    assert t.value == "abc123"


def test_ident_uppercase() -> None:
    t = tok("MyFunc")
    assert t.type is TokenType.IDENT
    assert t.value == "MyFunc"


def test_ident_value_is_str() -> None:
    t = tok("hello")
    assert isinstance(t.value, str)


# ---------------------------------------------------------------------------
# 6. Keywords  (value must be None for all keywords)
# ---------------------------------------------------------------------------


KEYWORDS: list[tuple[str, TokenType]] = [
    ("fn", TokenType.KW_FN),
    ("let", TokenType.KW_LET),
    ("if", TokenType.KW_IF),
    ("else", TokenType.KW_ELSE),
    ("while", TokenType.KW_WHILE),
    ("return", TokenType.KW_RETURN),
    ("in", TokenType.KW_IN),
    ("out", TokenType.KW_OUT),
    ("u8", TokenType.KW_U8),
]


@pytest.mark.parametrize("src,expected_type", KEYWORDS)
def test_keyword(src: str, expected_type: TokenType) -> None:
    t = tok(src)
    assert t.type is expected_type
    assert t.value is None


def test_keyword_prefix_is_ident() -> None:
    # "fns" starts with "fn" but is an IDENT, not KW_FN
    t = tok("fns")
    assert t.type is TokenType.IDENT
    assert t.value == "fns"


def test_keyword_not_prefix_matched() -> None:
    # "letter" starts with "let" — must be IDENT
    t = tok("letter")
    assert t.type is TokenType.IDENT


def test_u8_is_keyword() -> None:
    assert tok("u8").type is TokenType.KW_U8


def test_u8_prefix_ident() -> None:
    assert tok("u8x").type is TokenType.IDENT


# ---------------------------------------------------------------------------
# 7. One-character operators and punctuation
# ---------------------------------------------------------------------------


ONE_CHAR_CASES: list[tuple[str, TokenType]] = [
    ("+", TokenType.PLUS),
    ("-", TokenType.MINUS),
    ("*", TokenType.STAR),
    ("/", TokenType.SLASH),
    ("%", TokenType.PERCENT),
    ("&", TokenType.AMP),
    ("|", TokenType.PIPE),
    ("^", TokenType.CARET),
    ("~", TokenType.TILDE),
    ("!", TokenType.BANG),
    ("=", TokenType.EQ),
    (":", TokenType.COLON),
    ("<", TokenType.LT),
    (">", TokenType.GT),
    ("(", TokenType.LPAREN),
    (")", TokenType.RPAREN),
    ("{", TokenType.LBRACE),
    ("}", TokenType.RBRACE),
    (",", TokenType.COMMA),
    (";", TokenType.SEMI),
]


@pytest.mark.parametrize("src,expected_type", ONE_CHAR_CASES)
def test_one_char_op(src: str, expected_type: TokenType) -> None:
    t = tok(src)
    assert t.type is expected_type
    assert t.value is None


# ---------------------------------------------------------------------------
# 8. Two-character operators (maximal munch)
# ---------------------------------------------------------------------------


TWO_CHAR_CASES: list[tuple[str, TokenType]] = [
    ("<<", TokenType.SHL),
    (">>", TokenType.SHR),
    ("==", TokenType.EQ_EQ),
    ("!=", TokenType.BANG_EQ),
    ("<=", TokenType.LT_EQ),
    (">=", TokenType.GT_EQ),
    ("&&", TokenType.AMP_AMP),
    ("||", TokenType.PIPE_PIPE),
    ("->", TokenType.ARROW),
]


@pytest.mark.parametrize("src,expected_type", TWO_CHAR_CASES)
def test_two_char_op(src: str, expected_type: TokenType) -> None:
    t = tok(src)
    assert t.type is expected_type
    assert t.value is None


def test_arrow_not_minus_gt() -> None:
    # -> is a single ARROW, not MINUS + GT
    assert types("->") == [TokenType.ARROW]


def test_shl_not_lt_lt() -> None:
    assert types("<<") == [TokenType.SHL]


def test_shr_not_gt_gt() -> None:
    assert types(">>") == [TokenType.SHR]


def test_lt_followed_by_minus() -> None:
    # <- is LT then MINUS (no two-char token for <-)
    assert types("<-") == [TokenType.LT, TokenType.MINUS]


def test_minus_alone_not_arrow() -> None:
    # - not followed by > is MINUS
    assert types("- x") == [TokenType.MINUS, TokenType.IDENT]


def test_eq_alone() -> None:
    # single = is EQ, not EQ_EQ
    assert types("= x") == [TokenType.EQ, TokenType.IDENT]


def test_bang_alone() -> None:
    # ! not followed by = is BANG
    assert types("! x") == [TokenType.BANG, TokenType.IDENT]


def test_amp_alone() -> None:
    assert types("& x") == [TokenType.AMP, TokenType.IDENT]


def test_pipe_alone() -> None:
    assert types("| x") == [TokenType.PIPE, TokenType.IDENT]


# ---------------------------------------------------------------------------
# 9. Position tracking (line, column, offset)
# ---------------------------------------------------------------------------


def test_column_advances() -> None:
    tokens = tokenize("a b")
    assert tokens[0].column == 1
    assert tokens[1].column == 3


def test_line_advances_on_newline() -> None:
    tokens = tokenize("a\nb")
    assert tokens[0].line == 1
    assert tokens[1].line == 2
    assert tokens[1].column == 1


def test_column_resets_after_newline() -> None:
    tokens = tokenize("abc\nde")
    assert tokens[1].line == 2
    assert tokens[1].column == 1


def test_offset_tracks_bytes() -> None:
    tokens = tokenize("ab + cd")
    # "ab" at 0, "+" at 3, "cd" at 5
    assert tokens[0].offset == 0
    assert tokens[1].offset == 3
    assert tokens[2].offset == 5


def test_newline_col_reset() -> None:
    # After a newline the column of the NEXT character should be 1
    tokens = tokenize("x\ny")
    t_y = tokens[1]
    assert t_y.line == 2
    assert t_y.column == 1


def test_hex_position() -> None:
    t = tok("  0xFF")
    assert t.line == 1
    assert t.column == 3
    assert t.offset == 2


def test_multiline_offset() -> None:
    src = "a\nb"
    tokens = tokenize(src)
    assert tokens[0].offset == 0  # 'a'
    assert tokens[1].offset == 2  # 'b' (after '\n')


# ---------------------------------------------------------------------------
# 10. Sequences and expressions
# ---------------------------------------------------------------------------


def test_simple_addition() -> None:
    assert types("1 + 2") == [TokenType.INT, TokenType.PLUS, TokenType.INT]
    assert values("1 + 2") == [1, None, 2]


def test_comparison_chain() -> None:
    assert types("a <= b") == [TokenType.IDENT, TokenType.LT_EQ, TokenType.IDENT]


def test_function_signature() -> None:
    src = "fn add(a: u8, b: u8) -> u8 { return a + b; }"
    tys = types(src)
    assert tys[0] is TokenType.KW_FN
    assert tys[1] is TokenType.IDENT       # add
    assert tys[2] is TokenType.LPAREN
    assert TokenType.KW_U8 in tys
    assert TokenType.ARROW in tys
    assert TokenType.COLON in tys


def test_let_with_type() -> None:
    src = "let x: u8 = 10;"
    tys = types(src)
    assert tys == [
        TokenType.KW_LET,
        TokenType.IDENT,
        TokenType.COLON,
        TokenType.KW_U8,
        TokenType.EQ,
        TokenType.INT,
        TokenType.SEMI,
    ]


def test_while_loop() -> None:
    src = "while x < 10 { x = x + 1; }"
    tys = types(src)
    assert tys[0] is TokenType.KW_WHILE
    assert TokenType.LT in tys
    assert TokenType.LBRACE in tys
    assert TokenType.RBRACE in tys


def test_if_else() -> None:
    src = "if a == b { out a; } else { out b; }"
    tys = types(src)
    assert tys[0] is TokenType.KW_IF
    assert TokenType.KW_ELSE in tys
    assert TokenType.EQ_EQ in tys
    assert TokenType.KW_OUT in tys


def test_bitwise_expr() -> None:
    assert types("a & b | c ^ d") == [
        TokenType.IDENT, TokenType.AMP,
        TokenType.IDENT, TokenType.PIPE,
        TokenType.IDENT, TokenType.CARET,
        TokenType.IDENT,
    ]


def test_shift_expr() -> None:
    assert types("x << 2") == [TokenType.IDENT, TokenType.SHL, TokenType.INT]
    assert types("x >> 2") == [TokenType.IDENT, TokenType.SHR, TokenType.INT]


def test_logical_and_or() -> None:
    assert types("a && b || c") == [
        TokenType.IDENT, TokenType.AMP_AMP,
        TokenType.IDENT, TokenType.PIPE_PIPE,
        TokenType.IDENT,
    ]


def test_unary_not() -> None:
    assert types("!a") == [TokenType.BANG, TokenType.IDENT]


def test_unary_bitwise_not() -> None:
    assert types("~a") == [TokenType.TILDE, TokenType.IDENT]


def test_in_keyword() -> None:
    src = "let x = in;"
    assert TokenType.KW_IN in types(src)


def test_return_keyword() -> None:
    assert tok("return").type is TokenType.KW_RETURN


def test_comma_in_call() -> None:
    assert types("f(a, b)") == [
        TokenType.IDENT, TokenType.LPAREN,
        TokenType.IDENT, TokenType.COMMA,
        TokenType.IDENT, TokenType.RPAREN,
    ]


def test_hex_in_expression() -> None:
    tys = types("x = 0xFF;")
    assert tys == [
        TokenType.IDENT, TokenType.EQ,
        TokenType.HEX, TokenType.SEMI,
    ]
    assert values("x = 0xFF;") == ["x", None, 255, None]


def test_multiline_program() -> None:
    src = """\
fn main() -> u8 {
    let x: u8 = 0xFF;
    return x;
}
"""
    tys = types(src)
    assert TokenType.KW_FN in tys
    assert TokenType.ARROW in tys
    assert TokenType.KW_U8 in tys
    assert TokenType.KW_LET in tys
    assert TokenType.HEX in tys
    assert TokenType.KW_RETURN in tys


# ---------------------------------------------------------------------------
# 11. LexError on illegal input
# ---------------------------------------------------------------------------


def test_illegal_char_at_symbol() -> None:
    with pytest.raises(LexError) as exc_info:
        tokenize("@")
    assert "@" in str(exc_info.value)


def test_illegal_char_dollar() -> None:
    with pytest.raises(LexError):
        tokenize("$foo")


def test_illegal_char_backtick() -> None:
    with pytest.raises(LexError):
        tokenize("`")


def test_illegal_char_hash() -> None:
    with pytest.raises(LexError):
        tokenize("#")


def test_illegal_char_question() -> None:
    with pytest.raises(LexError):
        tokenize("?")


def test_illegal_char_after_valid() -> None:
    # Error should fire on the bad character, not on earlier good tokens
    with pytest.raises(LexError) as exc_info:
        tokenize("42 @")
    err = exc_info.value
    assert err.column == 4  # '@' is at column 4


def test_lex_error_line_col() -> None:
    with pytest.raises(LexError) as exc_info:
        tokenize("\n\n  @")
    err = exc_info.value
    assert err.line == 3
    assert err.column == 3


def test_lex_error_message_contains_char() -> None:
    with pytest.raises(LexError) as exc_info:
        tokenize("§")
    assert "unexpected character" in str(exc_info.value)


def test_lex_error_has_attributes() -> None:
    with pytest.raises(LexError) as exc_info:
        tokenize("@")
    err = exc_info.value
    assert hasattr(err, "line")
    assert hasattr(err, "column")
    assert err.line == 1
    assert err.column == 1


# ---------------------------------------------------------------------------
# 12. Token dataclass properties
# ---------------------------------------------------------------------------


def test_token_is_frozen() -> None:
    t = tok("42")
    with pytest.raises((AttributeError, TypeError)):
        t.type = TokenType.EOF  # type: ignore[misc]


def test_int_value_is_int() -> None:
    t = tok("42")
    assert isinstance(t.value, int)
    assert t.value == 42


def test_hex_value_is_int() -> None:
    t = tok("0xAB")
    assert isinstance(t.value, int)
    assert t.value == 0xAB


def test_ident_value_is_str_and_correct() -> None:
    t = tok("myVar")
    assert isinstance(t.value, str)
    assert t.value == "myVar"


def test_keyword_value_is_none() -> None:
    for src, _ in KEYWORDS:
        assert tok(src).value is None


def test_operator_value_is_none() -> None:
    for src, _ in ONE_CHAR_CASES + TWO_CHAR_CASES:
        assert tok(src).value is None


def test_eof_value_is_none() -> None:
    assert tokenize("")[-1].value is None


# ---------------------------------------------------------------------------
# 13. Edge: 0 not confused with hex prefix
# ---------------------------------------------------------------------------


def test_zero_not_hex() -> None:
    t = tok("0")
    assert t.type is TokenType.INT
    assert t.value == 0


def test_zero_followed_by_alpha() -> None:
    # "0a" — '0' is INT, 'a' is IDENT  (not a malformed hex)
    tys = types("0a")
    assert tys == [TokenType.INT, TokenType.IDENT]
    assert values("0a") == [0, "a"]


def test_zero_followed_by_x_then_non_hex() -> None:
    # "0xg" — 'x' triggers hex scan, 'g' is not a hex digit → LexError
    with pytest.raises(LexError):
        tokenize("0xg")


# ---------------------------------------------------------------------------
# 14. Long programs and stress tests
# ---------------------------------------------------------------------------


def test_no_trailing_newline() -> None:
    tokens = tokenize("42")
    assert tokens[-1].type is TokenType.EOF
    assert tokens[0].value == 42


def test_adjacent_tokens_no_whitespace() -> None:
    # a+b is valid: IDENT PLUS IDENT
    assert types("a+b") == [TokenType.IDENT, TokenType.PLUS, TokenType.IDENT]


def test_caret_not_xor_confused() -> None:
    assert tok("^").type is TokenType.CARET


def test_percent_modulo() -> None:
    assert tok("%").type is TokenType.PERCENT


def test_many_tokens() -> None:
    src = " ".join(str(i) for i in range(100))
    tokens = tokenize(src)
    assert len(tokens) == 101  # 100 INTs + EOF
    assert all(t.type is TokenType.INT for t in tokens[:-1])


def test_deeply_nested_braces() -> None:
    src = "{{{}}}".replace("{", "{ ").replace("}", " }")
    tys = types(src)
    assert tys.count(TokenType.LBRACE) == 3
    assert tys.count(TokenType.RBRACE) == 3


def test_comment_at_end_of_file_no_newline() -> None:
    tokens = tokenize("42 // no newline at end")
    assert tokens[0].value == 42
    assert tokens[-1].type is TokenType.EOF


def test_multiple_comments() -> None:
    src = "// first\n// second\n42"
    tokens = tokenize(src)
    assert tokens[0].value == 42
    assert len(tokens) == 2  # INT + EOF


def test_crlf_line_endings() -> None:
    tokens = tokenize("a\r\nb")
    assert tokens[0].type is TokenType.IDENT
    assert tokens[1].type is TokenType.IDENT
    assert tokens[1].line == 2


def test_gt_eq_not_two_gts() -> None:
    assert types(">=") == [TokenType.GT_EQ]
    assert types("> =") == [TokenType.GT, TokenType.EQ]


def test_lt_eq_not_two_lts() -> None:
    assert types("<=") == [TokenType.LT_EQ]
    assert types("< =") == [TokenType.LT, TokenType.EQ]
