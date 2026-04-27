"""Tests for the Nib lexer thin wrapper.

These tests verify that the grammar-driven lexer, configured with
``nib.tokens``, correctly tokenizes Nib source text.

Nib Tokenization Notes
-----------------------

Nib is a safe toy language targeting the Intel 4004 microprocessor. Its
tokenization has several interesting properties:

1. **Case-sensitive keywords**: Unlike ALGOL 60, Nib keywords are lowercase
   only. ``FN`` is a NAME (identifier), not the ``fn`` keyword. This matches
   the convention of modern systems languages like Rust and Go.

2. **Multi-character operators before single-char**: ``+%`` must be consumed
   before ``+``, ``+?`` before ``+``, ``->`` before ``-``, ``==`` before ``=``,
   ``..`` before anything starting with ``.``, etc. The grammar lists them
   first, and the lexer's first-match-wins semantics handle the rest.

3. **HEX_LIT before INT_LIT**: ``0xFF`` must lex as one hex token, not
   INT_LIT("0") followed by NAME("xFF"). Placing HEX_LIT before INT_LIT in
   the grammar ensures the full hex token is consumed first.

4. **Types are NAME tokens**: ``u4``, ``u8``, ``bcd``, and ``bool`` are NOT
   keywords — they lex as NAME tokens. The parser promotes them to type
   productions. This keeps the keyword set minimal.

5. **Line comment skipping**: ``// text`` to end of line is consumed silently
   without emitting any token. The ``//`` prefix was chosen over ``#`` to
   avoid confusion with 4004 assembly, which uses ``;`` for comments.

6. **Keyword boundary enforcement**: ``format`` is NAME, not ``for`` + NAME
   ("mat"). The full identifier token must exactly match a keyword string.
"""

from __future__ import annotations

import pytest

from nib_lexer import create_nib_lexer, tokenize_nib
from lexer import GrammarLexer, Token


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def token_types(source: str) -> list[str]:
    """Tokenize and return just the type names (excluding EOF)."""
    tokens = tokenize_nib(source)
    return [
        t.type if isinstance(t.type, str) else t.type.name
        for t in tokens
        if (t.type if isinstance(t.type, str) else t.type.name) != "EOF"
    ]


def token_values(source: str) -> list[str]:
    """Tokenize and return just the values (excluding EOF)."""
    tokens = tokenize_nib(source)
    return [
        t.value
        for t in tokens
        if (t.type if isinstance(t.type, str) else t.type.name) != "EOF"
    ]


# ---------------------------------------------------------------------------
# Factory function tests
# ---------------------------------------------------------------------------


class TestFactory:
    """Tests for the create_nib_lexer factory function."""

    def test_returns_grammar_lexer(self) -> None:
        """create_nib_lexer should return a GrammarLexer instance."""
        lexer = create_nib_lexer("fn add(a: u4) -> u4 { return a; }")
        assert isinstance(lexer, GrammarLexer)

    def test_factory_produces_tokens(self) -> None:
        """The factory-created lexer should produce valid tokens."""
        lexer = create_nib_lexer("let x: u4 = 5;")
        tokens = lexer.tokenize()
        assert len(tokens) >= 2  # at least something + EOF
        last_type = tokens[-1].type if isinstance(tokens[-1].type, str) else tokens[-1].type.name
        assert last_type == "EOF"

    def test_factory_empty_input(self) -> None:
        """Factory with empty input produces only EOF."""
        lexer = create_nib_lexer("")
        tokens = lexer.tokenize()
        assert len(tokens) == 1
        eof_name = tokens[0].type if isinstance(tokens[0].type, str) else tokens[0].type.name
        assert eof_name == "EOF"


# ---------------------------------------------------------------------------
# Keyword tests
# ---------------------------------------------------------------------------


class TestKeywords:
    """Tests for Nib keyword tokenization.

    Nib keywords are intentionally minimal — reflecting the 4004's tiny
    language target. Keywords are case-sensitive and lowercase only.

    fn, let, static, const, return, for, in, if, else, true, false.
    """

    def test_fn_keyword(self) -> None:
        """'fn' is the function declaration keyword."""
        types = token_types("fn")
        assert types == ["fn"]

    def test_let_keyword(self) -> None:
        """'let' introduces a local variable declaration."""
        types = token_types("let")
        assert types == ["let"]

    def test_static_keyword(self) -> None:
        """'static' introduces a ROM-mapped static variable."""
        types = token_types("static")
        assert types == ["static"]

    def test_const_keyword(self) -> None:
        """'const' introduces a compile-time constant folded into ROM."""
        types = token_types("const")
        assert types == ["const"]

    def test_return_keyword(self) -> None:
        """'return' exits a function with an optional value."""
        types = token_types("return")
        assert types == ["return"]

    def test_for_keyword(self) -> None:
        """'for' begins a loop (only loop construct in Nib v1)."""
        types = token_types("for")
        assert types == ["for"]

    def test_in_keyword(self) -> None:
        """'in' separates the loop variable from the range."""
        types = token_types("in")
        assert types == ["in"]

    def test_if_keyword(self) -> None:
        """'if' begins a conditional."""
        types = token_types("if")
        assert types == ["if"]

    def test_else_keyword(self) -> None:
        """'else' begins the alternative branch of a conditional."""
        types = token_types("else")
        assert types == ["else"]

    def test_true_keyword(self) -> None:
        """'true' is the boolean literal for truth."""
        types = token_types("true")
        assert types == ["true"]

    def test_false_keyword(self) -> None:
        """'false' is the boolean literal for falsehood."""
        types = token_types("false")
        assert types == ["false"]

    def test_all_keywords_together(self) -> None:
        """All keywords tokenize correctly when space-separated."""
        types = token_types("fn let static const return for in if else true false")
        assert types == [
            "fn", "let", "static", "const", "return",
            "for", "in", "if", "else", "true", "false",
        ]

    def test_keywords_case_sensitive(self) -> None:
        """Uppercase versions of keywords are NAME tokens, not keywords.

        Unlike ALGOL 60, Nib is case-sensitive. 'FN' does not match 'fn'.
        This mirrors Rust, Go, and other modern systems languages.
        """
        types = token_types("FN Let STATIC")
        assert types == ["NAME", "NAME", "NAME"]

    def test_fn_uppercase_is_name(self) -> None:
        """'FN' is a NAME, not the 'fn' keyword."""
        types = token_types("FN")
        assert types == ["NAME"]

    def test_return_uppercase_is_name(self) -> None:
        """'RETURN' is a NAME, not the 'return' keyword."""
        types = token_types("RETURN")
        assert types == ["NAME"]


# ---------------------------------------------------------------------------
# Types are NAME tokens (not keywords)
# ---------------------------------------------------------------------------


class TestTypeNames:
    """Tests confirming that type names are NAME tokens, not keywords.

    In Nib, ``u4``, ``u8``, ``bcd``, and ``bool`` are not in the keyword list.
    They lex as NAME tokens. The parser promotes them to type productions in
    type-annotation context. This keeps the keyword set small and allows
    user-defined type aliases in future Nib versions.
    """

    def test_u4_is_name(self) -> None:
        """'u4' (4-bit unsigned integer) is a NAME token."""
        types = token_types("u4")
        assert types == ["NAME"]

    def test_u8_is_name(self) -> None:
        """'u8' (8-bit unsigned integer, two nibbles) is a NAME token."""
        types = token_types("u8")
        assert types == ["NAME"]

    def test_bcd_is_name(self) -> None:
        """'bcd' (binary-coded decimal) is a NAME token."""
        types = token_types("bcd")
        assert types == ["NAME"]

    def test_bool_is_name(self) -> None:
        """'bool' (boolean) is a NAME token."""
        types = token_types("bool")
        assert types == ["NAME"]

    def test_type_in_annotation(self) -> None:
        """Type names in annotations lex as NAME: 'x: u4' is NAME COLON NAME."""
        types = token_types("x: u4")
        assert types == ["NAME", "COLON", "NAME"]


# ---------------------------------------------------------------------------
# Multi-character operator tests
# ---------------------------------------------------------------------------


class TestMultiCharOperators:
    """Tests for Nib multi-character operators.

    These must all appear before their single-character prefixes in the
    grammar (first-match-wins semantics). Tests verify each one is lexed
    as a single atomic token.
    """

    def test_wrap_add(self) -> None:
        """'+%' is a single WRAP_ADD token, not PLUS PERCENT."""
        types = token_types("+%")
        assert types == ["WRAP_ADD"]

    def test_wrap_add_value(self) -> None:
        """WRAP_ADD token has value '+%'."""
        values = token_values("+%")
        assert values == ["+%"]

    def test_sat_add(self) -> None:
        """'+?' is a single SAT_ADD token, not PLUS QUESTION."""
        types = token_types("+?")
        assert types == ["SAT_ADD"]

    def test_sat_add_value(self) -> None:
        """SAT_ADD token has value '+?'."""
        values = token_values("+?")
        assert values == ["+?"]

    def test_range(self) -> None:
        """'..' is a single RANGE token."""
        types = token_types("..")
        assert types == ["RANGE"]

    def test_arrow(self) -> None:
        """'->' is a single ARROW token, not MINUS GT."""
        types = token_types("->")
        assert types == ["ARROW"]

    def test_eq_eq(self) -> None:
        """'==' is a single EQ_EQ token, not EQ EQ."""
        types = token_types("==")
        assert types == ["EQ_EQ"]

    def test_neq(self) -> None:
        """'!=' is a single NEQ token, not BANG EQ."""
        types = token_types("!=")
        assert types == ["NEQ"]

    def test_leq(self) -> None:
        """'<=' is a single LEQ token, not LT EQ."""
        types = token_types("<=")
        assert types == ["LEQ"]

    def test_geq(self) -> None:
        """'>=' is a single GEQ token, not GT EQ."""
        types = token_types(">=")
        assert types == ["GEQ"]

    def test_land(self) -> None:
        """'&&' is a single LAND token, not AMP AMP."""
        types = token_types("&&")
        assert types == ["LAND"]

    def test_lor(self) -> None:
        """'||' is a single LOR token, not PIPE PIPE."""
        types = token_types("||")
        assert types == ["LOR"]

    def test_all_multi_char_operators(self) -> None:
        """All multi-char operators tokenize correctly in sequence."""
        types = token_types("+% +? .. -> == != <= >= && ||")
        assert types == [
            "WRAP_ADD", "SAT_ADD", "RANGE", "ARROW",
            "EQ_EQ", "NEQ", "LEQ", "GEQ", "LAND", "LOR",
        ]


# ---------------------------------------------------------------------------
# Single-character operator disambiguation tests
# ---------------------------------------------------------------------------


class TestSingleCharOperatorDisambiguation:
    """Tests that single-char operators are not confused with multi-char ones.

    When a multi-char operator prefix appears alone (not followed by its
    continuation), the single-char token must win. E.g., '+' followed by
    a space must be PLUS, not the start of WRAP_ADD or SAT_ADD.
    """

    def test_plus_alone(self) -> None:
        """'+' alone is PLUS (not WRAP_ADD or SAT_ADD)."""
        types = token_types("+")
        assert types == ["PLUS"]

    def test_plus_before_space(self) -> None:
        """'+ 1' — the plus is PLUS, not the start of +% or +?."""
        types = token_types("+ 1")
        assert types[0] == "PLUS"

    def test_minus_alone(self) -> None:
        """'-' alone is MINUS (not ARROW)."""
        types = token_types("-")
        assert types == ["MINUS"]

    def test_minus_before_space(self) -> None:
        """'- 1' — the minus is MINUS, not the start of ->."""
        types = token_types("- 1")
        assert types[0] == "MINUS"

    def test_eq_alone(self) -> None:
        """'=' alone is EQ (assignment), not part of ==."""
        types = token_types("=")
        assert types == ["EQ"]

    def test_bang_alone(self) -> None:
        """'!' alone is BANG (logical NOT), not part of !=."""
        types = token_types("!")
        assert types == ["BANG"]

    def test_lt_alone(self) -> None:
        """'<' alone is LT, not part of <=."""
        types = token_types("<")
        assert types == ["LT"]

    def test_gt_alone(self) -> None:
        """'>' alone is GT, not part of >=."""
        types = token_types(">")
        assert types == ["GT"]

    def test_amp_alone(self) -> None:
        """'&' alone is AMP (bitwise AND), not part of &&."""
        types = token_types("&")
        assert types == ["AMP"]

    def test_pipe_alone(self) -> None:
        """'|' alone is PIPE (bitwise OR), not part of ||."""
        types = token_types("|")
        assert types == ["PIPE"]


# ---------------------------------------------------------------------------
# Arithmetic operator tests
# ---------------------------------------------------------------------------


class TestArithmeticOperators:
    """Tests for Nib arithmetic operators."""

    def test_plus(self) -> None:
        """'+' is PLUS."""
        types = token_types("+")
        assert types == ["PLUS"]

    def test_minus(self) -> None:
        """'-' is MINUS."""
        types = token_types("-")
        assert types == ["MINUS"]

    def test_star(self) -> None:
        """'*' is STAR (reserved for future v2 — 4004 has no multiply)."""
        types = token_types("*")
        assert types == ["STAR"]

    def test_slash(self) -> None:
        """'/' is SLASH (reserved for future v2 — 4004 has no divide)."""
        types = token_types("/")
        assert types == ["SLASH"]

    def test_all_arithmetic_operators(self) -> None:
        """All arithmetic operators tokenize correctly."""
        types = token_types("+ - * /")
        assert types == ["PLUS", "MINUS", "STAR", "SLASH"]


# ---------------------------------------------------------------------------
# Bitwise operator tests
# ---------------------------------------------------------------------------


class TestBitwiseOperators:
    """Tests for Nib bitwise operators.

    The 4004 has hardware instructions for bitwise AND (ANL), OR (ORL),
    and XOR (XRL). These map directly to Nib's &, |, and ^ operators.
    Bitwise NOT maps to the 4004's CMA (complement accumulator) instruction.
    """

    def test_amp(self) -> None:
        """'&' is AMP (bitwise AND, maps to 4004 ANL instruction)."""
        types = token_types("&")
        assert types == ["AMP"]

    def test_pipe(self) -> None:
        """'|' is PIPE (bitwise OR, maps to 4004 ORL instruction)."""
        types = token_types("|")
        assert types == ["PIPE"]

    def test_caret(self) -> None:
        """'^' is CARET (bitwise XOR, maps to 4004 XRL instruction)."""
        types = token_types("^")
        assert types == ["CARET"]

    def test_tilde(self) -> None:
        """'~' is TILDE (bitwise NOT, maps to 4004 CMA instruction)."""
        types = token_types("~")
        assert types == ["TILDE"]

    def test_all_bitwise_operators(self) -> None:
        """All bitwise operators tokenize correctly."""
        types = token_types("& | ^ ~")
        assert types == ["AMP", "PIPE", "CARET", "TILDE"]


# ---------------------------------------------------------------------------
# Literal tests
# ---------------------------------------------------------------------------


class TestHexLiteral:
    """Tests for hexadecimal integer literals.

    HEX_LIT must come before INT_LIT in the grammar. If INT_LIT came first,
    '0xFF' would lex as INT_LIT("0") then NAME("xFF"), which is wrong.

    Hex literals are crucial for 4004 programming: nibble masks (0xF),
    port addresses, ROM addresses, and hardware register values are all
    naturally expressed in hex.
    """

    def test_hex_single_digit(self) -> None:
        """'0xA' is a single HEX_LIT token."""
        types = token_types("0xA")
        assert types == ["HEX_LIT"]

    def test_hex_single_digit_value(self) -> None:
        """HEX_LIT value preserves the original text."""
        values = token_values("0xA")
        assert values == ["0xA"]

    def test_hex_uppercase_digits(self) -> None:
        """'0xFF' is HEX_LIT (uppercase hex digits A-F accepted)."""
        types = token_types("0xFF")
        assert types == ["HEX_LIT"]

    def test_hex_lowercase_digits(self) -> None:
        """'0xff' is HEX_LIT (lowercase hex digits a-f accepted)."""
        types = token_types("0xff")
        assert types == ["HEX_LIT"]

    def test_hex_mixed_case(self) -> None:
        """'0xAbCd' is HEX_LIT (mixed case accepted)."""
        types = token_types("0xAbCd")
        assert types == ["HEX_LIT"]

    def test_hex_nibble_mask(self) -> None:
        """'0xF' is HEX_LIT — the classic nibble mask for the 4004."""
        types = token_types("0xF")
        values = token_values("0xF")
        assert types == ["HEX_LIT"]
        assert values == ["0xF"]

    def test_hex_before_int_for_0xff(self) -> None:
        """'0xFF' must lex as HEX_LIT, not INT_LIT('0') + NAME('xFF')."""
        types = token_types("0xFF")
        assert types == ["HEX_LIT"]  # not ["INT_LIT", "NAME"]

    def test_hex_zero(self) -> None:
        """'0x0' is HEX_LIT."""
        types = token_types("0x0")
        assert types == ["HEX_LIT"]

    def test_hex_multi_digit(self) -> None:
        """'0x1F' is a single HEX_LIT."""
        types = token_types("0x1F")
        assert types == ["HEX_LIT"]


class TestIntLiteral:
    """Tests for decimal integer literals.

    INT_LIT must come after HEX_LIT in the grammar to prevent '0xFF' from
    splitting at the leading '0'. Decimal integers are simple: one or more
    decimal digits.
    """

    def test_single_digit(self) -> None:
        """A single digit is INT_LIT."""
        types = token_types("5")
        assert types == ["INT_LIT"]

    def test_multi_digit(self) -> None:
        """Multiple digits form one INT_LIT."""
        types = token_types("42")
        values = token_values("42")
        assert types == ["INT_LIT"]
        assert values == ["42"]

    def test_zero(self) -> None:
        """'0' is INT_LIT."""
        types = token_types("0")
        assert types == ["INT_LIT"]

    def test_decimal_not_hex(self) -> None:
        """'10' is INT_LIT (decimal ten), not anything hex-related."""
        types = token_types("10")
        assert types == ["INT_LIT"]

    def test_max_u4(self) -> None:
        """'15' is INT_LIT (maximum u4 value on the 4004)."""
        types = token_types("15")
        assert types == ["INT_LIT"]

    def test_multiple_integers(self) -> None:
        """Multiple integer literals are separate tokens."""
        types = token_types("1 2 3")
        assert types == ["INT_LIT", "INT_LIT", "INT_LIT"]


# ---------------------------------------------------------------------------
# Identifier (NAME) tests
# ---------------------------------------------------------------------------


class TestIdentifiers:
    """Tests for Nib identifier tokenization.

    Nib identifiers start with a letter or underscore, followed by letters,
    digits, or underscores. The underscore support (unlike ALGOL 60) is
    essential for embedded code naming conventions: ``carry_out``,
    ``nibble_high``, ``bcd_digit_2``.
    """

    def test_simple_name(self) -> None:
        """A simple identifier is NAME."""
        types = token_types("counter")
        assert types == ["NAME"]

    def test_name_with_underscore(self) -> None:
        """An identifier with underscore is NAME."""
        types = token_types("my_var")
        assert types == ["NAME"]

    def test_name_starts_underscore(self) -> None:
        """An identifier starting with underscore is NAME."""
        types = token_types("_hidden")
        assert types == ["NAME"]

    def test_single_letter(self) -> None:
        """A single letter is NAME."""
        types = token_types("x")
        assert types == ["NAME"]

    def test_name_with_digits(self) -> None:
        """An identifier with digits is NAME."""
        types = token_types("carry_out2")
        assert types == ["NAME"]

    def test_embedded_naming_convention(self) -> None:
        """Embedded-style identifier with underscores is NAME."""
        types = token_types("nibble_high")
        assert types == ["NAME"]

    def test_multiple_names(self) -> None:
        """Multiple identifiers are separate NAME tokens."""
        types = token_types("a b c")
        assert types == ["NAME", "NAME", "NAME"]


# ---------------------------------------------------------------------------
# Keyword boundary enforcement tests
# ---------------------------------------------------------------------------


class TestKeywordBoundary:
    """Tests that keywords only fire on exact full-token matches.

    The keyword match fires only when the entire identifier token matches the
    keyword string exactly. Prefixes and suffixes do not qualify.

        fn      → 'fn' keyword
        fns     → NAME  (full token 'fns' ≠ keyword 'fn')
        for     → 'for' keyword
        format  → NAME  (full token 'format' ≠ keyword 'for')
        in      → 'in' keyword
        input   → NAME  (full token 'input' ≠ keyword 'in')
        return  → 'return' keyword
        returns → NAME
    """

    def test_fns_is_name(self) -> None:
        """'fns' is NAME, not 'fn' + 's'."""
        types = token_types("fns")
        assert types == ["NAME"]

    def test_format_is_name(self) -> None:
        """'format' is NAME, not 'for' + 'mat'."""
        types = token_types("format")
        assert types == ["NAME"]

    def test_input_is_name(self) -> None:
        """'input' is NAME, not 'in' + 'put'."""
        types = token_types("input")
        assert types == ["NAME"]

    def test_returns_is_name(self) -> None:
        """'returns' is NAME, not 'return' + 's'."""
        types = token_types("returns")
        assert types == ["NAME"]

    def test_static_var_is_name(self) -> None:
        """'static_var' is NAME, not 'static' + '_var'."""
        types = token_types("static_var")
        assert types == ["NAME"]

    def test_ifelse_is_name(self) -> None:
        """'ifelse' is NAME, not 'if' + 'else'."""
        types = token_types("ifelse")
        assert types == ["NAME"]

    def test_fn_alone_is_keyword(self) -> None:
        """Confirm 'fn' alone is keyword but 'fns' is NAME."""
        assert token_types("fn") == ["fn"]
        assert token_types("fns") == ["NAME"]

    def test_for_alone_is_keyword(self) -> None:
        """Confirm 'for' alone is keyword but 'format' is NAME."""
        assert token_types("for") == ["for"]
        assert token_types("format") == ["NAME"]

    def test_truthy_is_name(self) -> None:
        """'truthy' is NAME, not 'true' + 'hy'."""
        types = token_types("truthy")
        assert types == ["NAME"]

    def test_falsetto_is_name(self) -> None:
        """'falsetto' is NAME, not 'false' + 'tto'."""
        types = token_types("falsetto")
        assert types == ["NAME"]


# ---------------------------------------------------------------------------
# Delimiter tests
# ---------------------------------------------------------------------------


class TestDelimiters:
    """Tests for Nib delimiter tokens."""

    def test_lbrace_rbrace(self) -> None:
        """Braces delimit function and control-flow bodies."""
        types = token_types("{}")
        assert types == ["LBRACE", "RBRACE"]

    def test_lparen_rparen(self) -> None:
        """Parentheses for grouping and parameter lists."""
        types = token_types("()")
        assert types == ["LPAREN", "RPAREN"]

    def test_colon(self) -> None:
        """Colon is the type annotation separator: 'x: u4'."""
        types = token_types(":")
        assert types == ["COLON"]

    def test_semicolon(self) -> None:
        """Semicolon terminates every statement."""
        types = token_types(";")
        assert types == ["SEMICOLON"]

    def test_comma(self) -> None:
        """Comma separates arguments and parameters."""
        types = token_types(",")
        assert types == ["COMMA"]

    def test_all_delimiters(self) -> None:
        """All delimiter tokens together."""
        types = token_types("{ } ( ) : ; ,")
        assert types == ["LBRACE", "RBRACE", "LPAREN", "RPAREN", "COLON", "SEMICOLON", "COMMA"]


# ---------------------------------------------------------------------------
# Comment skipping tests
# ---------------------------------------------------------------------------


class TestCommentSkipping:
    """Tests for Nib line comment syntax.

    Nib uses C++/Java/Rust-style ``//`` line comments. Everything from ``//``
    to the end of the line is consumed without emitting any token. The newline
    itself is consumed by the WHITESPACE skip rule (or possibly by the comment
    pattern itself — either way, the next line's tokens are produced correctly).
    """

    def test_comment_produces_no_tokens(self) -> None:
        """A comment line produces no tokens."""
        types = token_types("// this is a comment")
        assert types == []

    def test_code_after_comment_on_next_line(self) -> None:
        """Tokens on the next line after a comment are correctly emitted."""
        types = token_types("// comment\n42")
        assert types == ["INT_LIT"]

    def test_comment_after_code_on_same_line(self) -> None:
        """A comment after code on the same line is skipped."""
        types = token_types("42 // set x to 42")
        assert types == ["INT_LIT"]

    def test_comment_between_statements(self) -> None:
        """A comment between two statements does not affect either."""
        types = token_types("let x: u4 = 5;\n// comment\nlet y: u4 = 6;")
        assert "INT_LIT" in types
        assert "let" in types

    def test_empty_comment(self) -> None:
        """An empty comment '//' produces no tokens."""
        types = token_types("//")
        assert types == []

    def test_multiple_comment_lines(self) -> None:
        """Multiple consecutive comment lines produce no tokens."""
        types = token_types("// line one\n// line two\n// line three")
        assert types == []


# ---------------------------------------------------------------------------
# Whitespace tests
# ---------------------------------------------------------------------------


class TestWhitespace:
    """Tests for Nib whitespace handling.

    Nib is free-format: whitespace is insignificant between tokens. Spaces,
    tabs, carriage returns, and newlines are all skipped.
    """

    def test_spaces_skipped(self) -> None:
        """Spaces between tokens are ignored."""
        types = token_types("  42  ")
        assert types == ["INT_LIT"]

    def test_tabs_skipped(self) -> None:
        """Tabs between tokens are ignored."""
        types = token_types("\t42\t")
        assert types == ["INT_LIT"]

    def test_newlines_skipped(self) -> None:
        """Newlines between tokens are ignored."""
        types = token_types("\n42\n")
        assert types == ["INT_LIT"]

    def test_mixed_whitespace(self) -> None:
        """Mixed whitespace types are all ignored."""
        types = token_types("  \t\r\n  let  \t  x  \r\n  ")
        assert types == ["let", "NAME"]

    def test_no_space_needed_between_tokens(self) -> None:
        """Tokens can be adjacent with no whitespace if unambiguous."""
        types = token_types("42+1")
        assert types == ["INT_LIT", "PLUS", "INT_LIT"]


# ---------------------------------------------------------------------------
# EOF token tests
# ---------------------------------------------------------------------------


class TestEOF:
    """Tests for the EOF token.

    The token list always ends with an EOF token, even for empty input.
    """

    def test_always_ends_with_eof(self) -> None:
        """Token list always ends with EOF."""
        tokens = tokenize_nib("let x: u4 = 5;")
        last = tokens[-1]
        eof_name = last.type if isinstance(last.type, str) else last.type.name
        assert eof_name == "EOF"

    def test_empty_input_has_eof(self) -> None:
        """Empty input produces exactly one EOF token."""
        tokens = tokenize_nib("")
        assert len(tokens) == 1
        eof_name = tokens[0].type if isinstance(tokens[0].type, str) else tokens[0].type.name
        assert eof_name == "EOF"

    def test_whitespace_only_has_eof(self) -> None:
        """Input with only whitespace produces just EOF."""
        tokens = tokenize_nib("   \t\n  ")
        assert len(tokens) == 1
        eof_name = tokens[0].type if isinstance(tokens[0].type, str) else tokens[0].type.name
        assert eof_name == "EOF"

    def test_int_lit_present(self) -> None:
        """'42' produces INT_LIT token followed by EOF."""
        tokens = tokenize_nib("42")
        assert len(tokens) == 2
        last_name = tokens[-1].type if isinstance(tokens[-1].type, str) else tokens[-1].type.name
        assert last_name == "EOF"


# ---------------------------------------------------------------------------
# Complete statement tests
# ---------------------------------------------------------------------------


class TestCompleteStatements:
    """Tests for complete Nib statements and constructs.

    These integration-style tests verify that real Nib snippets tokenize
    into the correct token sequences end-to-end.
    """

    def test_let_statement(self) -> None:
        """'let x: u4 = 5;' tokenizes correctly."""
        types = token_types("let x: u4 = 5;")
        assert types == ["let", "NAME", "COLON", "NAME", "EQ", "INT_LIT", "SEMICOLON"]

    def test_let_with_hex(self) -> None:
        """'let mask: u4 = 0xF;' — hex literal in declaration."""
        types = token_types("let mask: u4 = 0xF;")
        assert types == ["let", "NAME", "COLON", "NAME", "EQ", "HEX_LIT", "SEMICOLON"]

    def test_fn_signature(self) -> None:
        """Function signature tokenizes correctly."""
        types = token_types("fn add(a: u4, b: u4) -> u4")
        assert types == [
            "fn", "NAME", "LPAREN",
            "NAME", "COLON", "NAME", "COMMA",
            "NAME", "COLON", "NAME",
            "RPAREN", "ARROW", "NAME",
        ]

    def test_return_statement(self) -> None:
        """'return a +% b;' — wrapping addition in a return."""
        types = token_types("return a +% b;")
        assert types == ["return", "NAME", "WRAP_ADD", "NAME", "SEMICOLON"]

    def test_sat_add_expression(self) -> None:
        """'x +? 1' — saturating addition expression."""
        types = token_types("x +? 1")
        assert types == ["NAME", "SAT_ADD", "INT_LIT"]

    def test_for_loop(self) -> None:
        """'for i: u4 in 0..8' — for loop header."""
        types = token_types("for i: u4 in 0..8")
        assert types == [
            "for", "NAME", "COLON", "NAME", "in",
            "INT_LIT", "RANGE", "INT_LIT",
        ]

    def test_if_condition(self) -> None:
        """'if x == 0' — if with equality test."""
        types = token_types("if x == 0")
        assert types == ["if", "NAME", "EQ_EQ", "INT_LIT"]

    def test_if_else(self) -> None:
        """'if a != b { } else { }' — if-else structure."""
        types = token_types("if a != b { } else { }")
        assert types == [
            "if", "NAME", "NEQ", "NAME",
            "LBRACE", "RBRACE",
            "else", "LBRACE", "RBRACE",
        ]

    def test_comparison_operators(self) -> None:
        """All comparison operators tokenize correctly in sequence."""
        types = token_types("a < b && c > d || e <= f || g >= h")
        assert types == [
            "NAME", "LT", "NAME", "LAND",
            "NAME", "GT", "NAME", "LOR",
            "NAME", "LEQ", "NAME", "LOR",
            "NAME", "GEQ", "NAME",
        ]

    def test_boolean_literals(self) -> None:
        """'true' and 'false' boolean literals in an expression."""
        types = token_types("let flag: bool = true;")
        assert types == ["let", "NAME", "COLON", "NAME", "EQ", "true", "SEMICOLON"]

    def test_const_declaration(self) -> None:
        """'const MAX: u4 = 0xF;' — const with hex literal."""
        types = token_types("const MAX: u4 = 0xF;")
        assert types == ["const", "NAME", "COLON", "NAME", "EQ", "HEX_LIT", "SEMICOLON"]

    def test_static_declaration(self) -> None:
        """'static counter: u8 = 0;' — static RAM variable."""
        types = token_types("static counter: u8 = 0;")
        assert types == ["static", "NAME", "COLON", "NAME", "EQ", "INT_LIT", "SEMICOLON"]

    def test_bitwise_expression(self) -> None:
        """'a & 0xF | b ^ c' — bitwise operators."""
        types = token_types("a & 0xF | b ^ c")
        assert types == [
            "NAME", "AMP", "HEX_LIT",
            "PIPE", "NAME",
            "CARET", "NAME",
        ]

    def test_not_expression(self) -> None:
        """'!flag' — logical not of a boolean."""
        types = token_types("!flag")
        assert types == ["BANG", "NAME"]

    def test_bitwise_not_expression(self) -> None:
        """'~nibble' — bitwise complement of a nibble."""
        types = token_types("~nibble")
        assert types == ["TILDE", "NAME"]

    def test_full_function(self) -> None:
        """A minimal complete Nib function tokenizes correctly."""
        source = "fn clamp(x: u4) -> u4 { return x +? 0xF; }"
        types = token_types(source)
        assert types == [
            "fn", "NAME", "LPAREN",
            "NAME", "COLON", "NAME",
            "RPAREN", "ARROW", "NAME",
            "LBRACE",
            "return", "NAME", "SAT_ADD", "HEX_LIT", "SEMICOLON",
            "RBRACE",
        ]
