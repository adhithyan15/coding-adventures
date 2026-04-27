"""Tests for the ECMAScript 3 (1999) Lexer.

ES3 adds strict equality (===, !==), try/catch/finally/throw, regex literals,
and the instanceof operator over ES1. These tests focus on the ES3-specific
features while also verifying that all ES1 features still work.
"""

from __future__ import annotations

from lexer import Token, TokenType

from ecmascript_es3_lexer import create_es3_lexer, tokenize_es3


# ============================================================================
# Helpers
# ============================================================================


def token_types(tokens: list[Token]) -> list[str]:
    """Extract just the type names from a token list.

    Token types can be ``TokenType`` enum members or plain strings (for
    grammar-defined types). We handle both.
    """
    return [t.type.name if hasattr(t.type, "name") else t.type for t in tokens]


def token_type_name(token: Token) -> str:
    """Get the type name of a single token."""
    return token.type.name if hasattr(token.type, "name") else token.type


def token_values(tokens: list[Token]) -> list[str]:
    """Extract just the values from a token list."""
    return [t.value for t in tokens]


# ============================================================================
# Test: ES3 Strict Equality (NEW in ES3)
# ============================================================================


class TestStrictEquality:
    """ES3 adds === and !== for strict equality (no type coercion)."""

    def test_strict_equals(self) -> None:
        """``===`` tests strict equality — no type coercion."""
        tokens = tokenize_es3("x === 1")
        assert tokens[1].value == "==="
        assert token_type_name(tokens[1]) == "STRICT_EQUALS"

    def test_strict_not_equals(self) -> None:
        """``!==`` tests strict inequality."""
        tokens = tokenize_es3("x !== 1")
        assert tokens[1].value == "!=="
        assert token_type_name(tokens[1]) == "STRICT_NOT_EQUALS"

    def test_abstract_equality_still_works(self) -> None:
        """``==`` still works alongside ``===``."""
        tokens = tokenize_es3("x == 1")
        assert tokens[1].value == "=="
        assert token_type_name(tokens[1]) == "EQUALS_EQUALS"

    def test_strict_before_abstract_ordering(self) -> None:
        """``===`` must match before ``==`` (first-match-wins)."""
        tokens = tokenize_es3("a === b == c")
        assert tokens[1].value == "==="
        assert tokens[3].value == "=="


# ============================================================================
# Test: ES3 Error Handling Keywords (NEW in ES3)
# ============================================================================


class TestErrorHandlingKeywords:
    """ES3 adds try, catch, finally, throw as keywords."""

    def test_try_keyword(self) -> None:
        tokens = tokenize_es3("try")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "try"

    def test_catch_keyword(self) -> None:
        tokens = tokenize_es3("catch")
        assert tokens[0].type == TokenType.KEYWORD

    def test_finally_keyword(self) -> None:
        tokens = tokenize_es3("finally")
        assert tokens[0].type == TokenType.KEYWORD

    def test_throw_keyword(self) -> None:
        tokens = tokenize_es3("throw")
        assert tokens[0].type == TokenType.KEYWORD

    def test_instanceof_keyword(self) -> None:
        """``instanceof`` checks prototype chain membership."""
        tokens = tokenize_es3("x instanceof Array")
        assert tokens[1].type == TokenType.KEYWORD
        assert tokens[1].value == "instanceof"


# ============================================================================
# Test: ES3 Regex Literals (NEW in ES3)
# ============================================================================


class TestRegexLiterals:
    """ES3 formalizes regex literals: /pattern/flags."""

    def test_simple_regex(self) -> None:
        """A basic regex literal."""
        tokens = tokenize_es3("/hello/")
        assert token_type_name(tokens[0]) == "REGEX"
        assert tokens[0].value == "/hello/"

    def test_regex_with_flags(self) -> None:
        """Regex with global and case-insensitive flags."""
        tokens = tokenize_es3("/pattern/gi")
        assert token_type_name(tokens[0]) == "REGEX"
        assert tokens[0].value == "/pattern/gi"

    def test_regex_with_escapes(self) -> None:
        """Regex containing escaped characters."""
        tokens = tokenize_es3(r"/hello\/world/")
        assert token_type_name(tokens[0]) == "REGEX"

    def test_regex_with_character_class(self) -> None:
        """Regex with character class brackets."""
        tokens = tokenize_es3("/[a-z]+/")
        assert token_type_name(tokens[0]) == "REGEX"


# ============================================================================
# Test: ES1 Features Still Work in ES3
# ============================================================================


class TestES1Compatibility:
    """All ES1 features should work in the ES3 lexer."""

    def test_var_declaration(self) -> None:
        tokens = tokenize_es3("var x = 1;")
        assert token_types(tokens) == [
            "KEYWORD", "NAME", "EQUALS", "NUMBER", "SEMICOLON", "EOF",
        ]

    def test_all_es1_keywords(self) -> None:
        """All 26 ES1 keywords are still valid in ES3."""
        es1_keywords = [
            "break", "case", "continue", "default", "delete", "do", "else",
            "for", "function", "if", "in", "new", "return", "switch", "this",
            "typeof", "var", "void", "while", "with", "true", "false", "null",
        ]
        for kw in es1_keywords:
            tokens = tokenize_es3(kw)
            assert tokens[0].type == TokenType.KEYWORD, f"{kw} should be KEYWORD in ES3"

    def test_dollar_identifier(self) -> None:
        tokens = tokenize_es3("$el")
        assert tokens[0].type == TokenType.NAME

    def test_arithmetic(self) -> None:
        tokens = tokenize_es3("1 + 2 * 3")
        assert token_types(tokens) == [
            "NUMBER", "PLUS", "NUMBER", "STAR", "NUMBER", "EOF",
        ]

    def test_strings(self) -> None:
        tokens = tokenize_es3('"hello"')
        assert token_type_name(tokens[0]) == "STRING"


# ============================================================================
# Test: Try/Catch Pattern
# ============================================================================


class TestTryCatchPattern:
    """Test tokenization of try/catch/finally blocks."""

    def test_try_catch(self) -> None:
        source = "try { x; } catch (e) { y; }"
        tokens = tokenize_es3(source)
        values = token_values(tokens)
        assert "try" in values
        assert "catch" in values

    def test_try_catch_finally(self) -> None:
        source = "try { } catch (e) { } finally { }"
        tokens = tokenize_es3(source)
        values = token_values(tokens)
        assert "finally" in values

    def test_throw_expression(self) -> None:
        source = 'throw "error";'
        tokens = tokenize_es3(source)
        assert tokens[0].value == "throw"
        assert tokens[0].type == TokenType.KEYWORD


# ============================================================================
# Test: Factory Function
# ============================================================================


class TestCreateES3Lexer:
    """Test the ``create_es3_lexer()`` factory function."""

    def test_creates_lexer(self) -> None:
        lexer = create_es3_lexer("var x = 1;")
        assert hasattr(lexer, "tokenize")

    def test_factory_produces_same_result(self) -> None:
        source = "try { x === 1; } catch (e) { }"
        tokens_direct = tokenize_es3(source)
        tokens_factory = create_es3_lexer(source).tokenize()
        assert tokens_direct == tokens_factory


# ============================================================================
# Test: Comments
# ============================================================================


class TestES3Comments:
    def test_line_comment_skipped(self) -> None:
        tokens = tokenize_es3("x // comment")
        assert token_types(tokens) == ["NAME", "EOF"]

    def test_block_comment_skipped(self) -> None:
        tokens = tokenize_es3("x /* block */ y")
        assert token_types(tokens) == ["NAME", "NAME", "EOF"]
