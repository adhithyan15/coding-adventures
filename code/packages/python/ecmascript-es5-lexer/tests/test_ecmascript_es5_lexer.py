"""Tests for the ECMAScript 5 (2009) Lexer.

ES5 adds the ``debugger`` keyword (promoted from future-reserved in ES3).
The token set is otherwise nearly identical to ES3. The real ES5 innovations
(strict mode, property descriptors, JSON) are semantic, not lexical.
"""

from __future__ import annotations

from lexer import Token, TokenType

from ecmascript_es5_lexer import create_es5_lexer, tokenize_es5


# ============================================================================
# Helpers
# ============================================================================


def token_types(tokens: list[Token]) -> list[str]:
    return [t.type.name if hasattr(t.type, "name") else t.type for t in tokens]


def token_type_name(token: Token) -> str:
    return token.type.name if hasattr(token.type, "name") else token.type


def token_values(tokens: list[Token]) -> list[str]:
    return [t.value for t in tokens]


# ============================================================================
# Test: ES5 Debugger Keyword (NEW in ES5)
# ============================================================================


class TestDebuggerKeyword:
    """ES5 promotes ``debugger`` from future-reserved to a real keyword."""

    def test_debugger_is_keyword(self) -> None:
        tokens = tokenize_es5("debugger")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "debugger"

    def test_debugger_statement(self) -> None:
        """``debugger;`` is a complete statement in ES5."""
        tokens = tokenize_es5("debugger;")
        assert token_types(tokens) == ["KEYWORD", "SEMICOLON", "EOF"]


# ============================================================================
# Test: All ES3 Features Still Work
# ============================================================================


class TestES3Compatibility:
    """All ES3 features should work in the ES5 lexer."""

    def test_strict_equality(self) -> None:
        tokens = tokenize_es5("x === y")
        assert tokens[1].value == "==="

    def test_strict_inequality(self) -> None:
        tokens = tokenize_es5("x !== y")
        assert tokens[1].value == "!=="

    def test_try_catch_keywords(self) -> None:
        for kw in ["try", "catch", "finally", "throw"]:
            tokens = tokenize_es5(kw)
            assert tokens[0].type == TokenType.KEYWORD

    def test_instanceof(self) -> None:
        tokens = tokenize_es5("x instanceof Array")
        assert tokens[1].type == TokenType.KEYWORD
        assert tokens[1].value == "instanceof"

    def test_regex_literal(self) -> None:
        tokens = tokenize_es5("/pattern/gi")
        assert token_type_name(tokens[0]) == "REGEX"


# ============================================================================
# Test: All ES1 Features Still Work
# ============================================================================


class TestES1Compatibility:
    def test_var_declaration(self) -> None:
        tokens = tokenize_es5("var x = 1;")
        assert token_types(tokens) == [
            "KEYWORD", "NAME", "EQUALS", "NUMBER", "SEMICOLON", "EOF",
        ]

    def test_all_es1_keywords(self) -> None:
        es1_keywords = [
            "break", "case", "continue", "default", "delete", "do", "else",
            "for", "function", "if", "in", "new", "return", "switch", "this",
            "typeof", "var", "void", "while", "with", "true", "false", "null",
        ]
        for kw in es1_keywords:
            tokens = tokenize_es5(kw)
            assert tokens[0].type == TokenType.KEYWORD

    def test_arithmetic(self) -> None:
        tokens = tokenize_es5("1 + 2 * 3")
        assert token_types(tokens) == [
            "NUMBER", "PLUS", "NUMBER", "STAR", "NUMBER", "EOF",
        ]

    def test_dollar_identifier(self) -> None:
        tokens = tokenize_es5("$")
        assert tokens[0].type == TokenType.NAME

    def test_hex_number(self) -> None:
        tokens = tokenize_es5("0xFF")
        assert token_type_name(tokens[0]) == "NUMBER"

    def test_strings(self) -> None:
        tokens = tokenize_es5("'hello'")
        assert token_type_name(tokens[0]) == "STRING"


# ============================================================================
# Test: Full Keyword Set (ES5 = ES3 keywords + debugger)
# ============================================================================


class TestES5FullKeywordSet:
    """ES5 has all ES3 keywords plus ``debugger``."""

    def test_complete_keyword_list(self) -> None:
        es5_keywords = [
            "break", "case", "catch", "continue", "debugger", "default",
            "delete", "do", "else", "finally", "for", "function", "if",
            "in", "instanceof", "new", "return", "switch", "this", "throw",
            "try", "typeof", "var", "void", "while", "with",
            "true", "false", "null",
        ]
        for kw in es5_keywords:
            tokens = tokenize_es5(kw)
            assert tokens[0].type == TokenType.KEYWORD, f"{kw} not keyword in ES5"


# ============================================================================
# Test: Getter/Setter Pattern (lexer sees NAME tokens, grammar handles context)
# ============================================================================


class TestGetterSetterTokenization:
    """In ES5, ``get`` and ``set`` are NOT keywords — they are NAMEs.
    The grammar handles the contextual interpretation."""

    def test_get_is_name(self) -> None:
        tokens = tokenize_es5("get")
        assert tokens[0].type == TokenType.NAME

    def test_set_is_name(self) -> None:
        tokens = tokenize_es5("set")
        assert tokens[0].type == TokenType.NAME

    def test_getter_pattern_tokens(self) -> None:
        """``get name() {}`` tokenizes as NAME NAME LPAREN RPAREN LBRACE RBRACE."""
        tokens = tokenize_es5("get name() {}")
        assert token_types(tokens) == [
            "NAME", "NAME", "LPAREN", "RPAREN", "LBRACE", "RBRACE", "EOF",
        ]


# ============================================================================
# Test: Real-world ES5 Patterns
# ============================================================================


class TestES5RealWorldPatterns:
    def test_strict_mode_directive(self) -> None:
        """The "use strict" directive is just a string literal to the lexer."""
        tokens = tokenize_es5('"use strict";')
        assert token_type_name(tokens[0]) == "STRING"
        assert "use strict" in tokens[0].value

    def test_property_access_chain(self) -> None:
        tokens = tokenize_es5("a.b.c")
        assert token_types(tokens) == [
            "NAME", "DOT", "NAME", "DOT", "NAME", "EOF",
        ]

    def test_function_call(self) -> None:
        tokens = tokenize_es5("foo(1, 2)")
        assert token_types(tokens) == [
            "NAME", "LPAREN", "NUMBER", "COMMA", "NUMBER", "RPAREN", "EOF",
        ]


# ============================================================================
# Test: Factory Function
# ============================================================================


class TestCreateES5Lexer:
    def test_creates_lexer(self) -> None:
        lexer = create_es5_lexer("debugger;")
        assert hasattr(lexer, "tokenize")

    def test_factory_produces_same_result(self) -> None:
        source = "debugger; var x = 1;"
        tokens_direct = tokenize_es5(source)
        tokens_factory = create_es5_lexer(source).tokenize()
        assert tokens_direct == tokens_factory


# ============================================================================
# Test: Comments
# ============================================================================


class TestES5Comments:
    def test_line_comment_skipped(self) -> None:
        tokens = tokenize_es5("x // comment")
        assert token_types(tokens) == ["NAME", "EOF"]

    def test_block_comment_skipped(self) -> None:
        tokens = tokenize_es5("x /* block */ y")
        assert token_types(tokens) == ["NAME", "NAME", "EOF"]
