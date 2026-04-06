"""Tests for the Python Lexer.

These tests verify that the grammar-driven lexer, when loaded with versioned
Python ``.tokens`` grammar files, correctly tokenizes Python source code
across multiple Python versions.

The Python lexer reuses the same ``GrammarLexer`` engine as the Starlark and
Ruby lexers — only the grammar file changes. What makes the Python lexer
unique is **versioned grammar support**: different ``.tokens`` files for
Python 2.7, 3.0, 3.6, 3.8, 3.10, and 3.12, each capturing the exact lexical
rules for that version.

Test Organization
-----------------

1. **Basic expressions** — names, numbers, operators (common to all versions)
2. **All versions** — verify that every supported version can tokenize code
3. **Version-specific keywords** — keywords that differ between versions
4. **String literals** — quoting styles, prefixes, f-strings
5. **Indentation** — INDENT/DEDENT tokens from indented blocks
6. **Operators** — multi-character operators, version-specific operators
7. **Factory function** — ``create_python_lexer()`` works correctly
8. **Error handling** — unsupported versions, invalid source
9. **Constants** — DEFAULT_VERSION and SUPPORTED_VERSIONS
"""

from __future__ import annotations

import pytest
from lexer import Token, TokenType

from python_lexer import (
    DEFAULT_VERSION,
    SUPPORTED_VERSIONS,
    create_python_lexer,
    tokenize_python,
)


# ============================================================================
# Helpers — make assertions more readable
# ============================================================================


def token_types(tokens: list[Token]) -> list[str]:
    """Extract just the type names from a token list.

    For ``TokenType`` enum members, we use ``.name`` (e.g., ``"NAME"``).
    For string types (like ``"INT"``, ``"INDENT"``), we use the string
    directly.

    Args:
        tokens: A list of Token objects.

    Returns:
        A list of token type name strings.
    """
    result = []
    for t in tokens:
        if isinstance(t.type, TokenType):
            result.append(t.type.name)
        else:
            result.append(t.type)
    return result


def token_values(tokens: list[Token]) -> list[str]:
    """Extract just the values from a token list.

    Args:
        tokens: A list of Token objects.

    Returns:
        A list of token value strings.
    """
    return [t.value for t in tokens]


# ============================================================================
# Test: Basic Expressions (all versions)
# ============================================================================


class TestBasicExpressions:
    """Test that simple Python expressions tokenize correctly.

    These tests use the default version (3.12) and cover fundamental
    building blocks that work across all Python versions: variables,
    numbers, operators, and assignment.
    """

    def test_simple_assignment(self) -> None:
        """Tokenize ``x = 1`` — the simplest assignment expression.

        This is the "hello world" of lexer testing. If this works, the
        basic plumbing (grammar loading, pattern matching, token creation)
        is all functioning correctly.
        """
        tokens = tokenize_python("x = 1\n")
        types = token_types(tokens)
        assert types[0] == "NAME"
        assert types[1] == "EQUALS"
        assert types[2] == "INT"
        assert types[3] == "NEWLINE"
        assert types[-1] == "EOF"

    def test_simple_name(self) -> None:
        """Tokenize a simple identifier (variable name)."""
        tokens = tokenize_python("x\n")
        types = token_types(tokens)
        assert types[0] == "NAME"
        assert tokens[0].value == "x"

    def test_integer_literal(self) -> None:
        """Tokenize an integer literal."""
        tokens = tokenize_python("42\n")
        types = token_types(tokens)
        assert types[0] == "INT"
        assert tokens[0].value == "42"

    def test_arithmetic_operators(self) -> None:
        """Tokenize ``a + b - c * d / e`` — four arithmetic operators."""
        tokens = tokenize_python("a + b - c * d / e\n")
        types = token_types(tokens)
        assert types == [
            "NAME", "PLUS", "NAME", "MINUS", "NAME",
            "STAR", "NAME", "SLASH", "NAME", "NEWLINE", "EOF",
        ]

    def test_parenthesized_expression(self) -> None:
        """Tokenize ``(1 + 2) * 3`` — parentheses for grouping."""
        tokens = tokenize_python("(1 + 2) * 3\n")
        types = token_types(tokens)
        assert types == [
            "LPAREN", "INT", "PLUS", "INT", "RPAREN",
            "STAR", "INT", "NEWLINE", "EOF",
        ]


# ============================================================================
# Test: All Versions Load and Tokenize
# ============================================================================


class TestAllVersions:
    """Test that every supported Python version can tokenize basic code.

    Each version has its own ``.tokens`` grammar file. This test suite
    verifies that all of them load without errors and can handle a simple
    assignment expression.
    """

    @pytest.mark.parametrize("version", SUPPORTED_VERSIONS)
    def test_version_tokenizes_assignment(self, version: str) -> None:
        """Every version should be able to tokenize ``x = 1``.

        This is a smoke test: if the grammar file loads and the basic
        tokenization pipeline works, the version is functional.
        """
        tokens = tokenize_python("x = 1\n", version=version)
        types = token_types(tokens)
        assert types[0] == "NAME"
        assert tokens[0].value == "x"
        assert "EQUALS" in types
        assert "INT" in types
        assert types[-1] == "EOF"

    @pytest.mark.parametrize("version", SUPPORTED_VERSIONS)
    def test_version_tokenizes_function_def(self, version: str) -> None:
        """Every version should handle a simple function definition.

        ``def`` has been a keyword since Python 1.0, so all versions
        must recognize it.
        """
        tokens = tokenize_python("def f():\n    return 1\n", version=version)
        types = token_types(tokens)
        # def should be a KEYWORD in all versions
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "def"
        # INDENT/DEDENT should be present (all versions use indentation mode)
        assert "INDENT" in types
        assert "DEDENT" in types


# ============================================================================
# Test: Version-Specific Keywords
# ============================================================================


class TestVersionSpecificKeywords:
    """Test keywords that differ between Python versions.

    The Python language has changed its keyword set over the years. Some
    notable changes:

    - ``print`` was a keyword in 2.7 but became a regular name in 3.0+
    - ``exec`` was a keyword in 2.7 but became a regular name in 3.0+
    - ``async``/``await`` became keywords in 3.6+
    - ``match``/``case`` became soft keywords in 3.10+ (they are still
      valid identifiers but are listed in the soft_keywords section)
    """

    def test_print_keyword_in_27(self) -> None:
        """``print`` is a keyword in Python 2.7.

        In Python 2.7, ``print`` is a statement keyword:
        ``print "hello"`` is valid syntax.
        """
        tokens = tokenize_python("print\n", version="2.7")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "print"

    def test_print_not_keyword_in_3x(self) -> None:
        """``print`` is NOT a keyword in Python 3.x — it is a regular name.

        In Python 3.0+, ``print`` is a built-in function, not a keyword.
        It should tokenize as a NAME, not a KEYWORD.
        """
        tokens = tokenize_python("print\n", version="3.0")
        assert tokens[0].type == TokenType.NAME
        assert tokens[0].value == "print"

    def test_exec_keyword_in_27(self) -> None:
        """``exec`` is a keyword in Python 2.7."""
        tokens = tokenize_python("exec\n", version="2.7")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "exec"

    def test_exec_not_keyword_in_3x(self) -> None:
        """``exec`` is NOT a keyword in Python 3.x."""
        tokens = tokenize_python("exec\n", version="3.0")
        assert tokens[0].type == TokenType.NAME
        assert tokens[0].value == "exec"

    def test_def_keyword_all_versions(self) -> None:
        """``def`` is a keyword in all Python versions."""
        for version in SUPPORTED_VERSIONS:
            tokens = tokenize_python("def\n", version=version)
            assert tokens[0].type == TokenType.KEYWORD
            assert tokens[0].value == "def"

    def test_class_keyword_all_versions(self) -> None:
        """``class`` is a keyword in all Python versions."""
        for version in SUPPORTED_VERSIONS:
            tokens = tokenize_python("class\n", version=version)
            assert tokens[0].type == TokenType.KEYWORD
            assert tokens[0].value == "class"

    def test_if_keyword_all_versions(self) -> None:
        """``if`` is a keyword in all Python versions."""
        for version in SUPPORTED_VERSIONS:
            tokens = tokenize_python("if\n", version=version)
            assert tokens[0].type == TokenType.KEYWORD
            assert tokens[0].value == "if"


# ============================================================================
# Test: String Literals
# ============================================================================


class TestStringLiterals:
    """Test tokenization of string literals in Python.

    Python supports multiple quoting styles: double-quoted, single-quoted,
    triple-double-quoted, and triple-single-quoted. All variants should
    emit the same ``STRING`` token type thanks to ``-> STRING`` aliases
    in the ``.tokens`` file.
    """

    def test_double_quoted_string(self) -> None:
        """Tokenize a simple double-quoted string."""
        tokens = tokenize_python('"hello"\n')
        assert tokens[0].type == TokenType.STRING
        assert tokens[0].value == "hello"

    def test_single_quoted_string(self) -> None:
        """Tokenize a single-quoted string."""
        tokens = tokenize_python("'hello'\n")
        assert tokens[0].type == TokenType.STRING

    def test_string_in_expression(self) -> None:
        """Tokenize a string used in an expression."""
        tokens = tokenize_python('x = "hello"\n')
        types = token_types(tokens)
        assert "STRING" in types


# ============================================================================
# Test: Indentation
# ============================================================================


class TestIndentation:
    """Test that indented blocks produce INDENT and DEDENT tokens.

    Python uses significant indentation to define block structure. The
    lexer emits INDENT when indentation increases, DEDENT when it
    decreases, and NEWLINE at logical line boundaries.
    """

    def test_simple_function_body(self) -> None:
        """A function definition with one indented line."""
        tokens = tokenize_python("def f():\n    return 1\n")
        types = token_types(tokens)
        assert "INDENT" in types
        assert "DEDENT" in types

    def test_nested_indentation(self) -> None:
        """Two levels of indentation (nested blocks)."""
        tokens = tokenize_python("if x:\n    if y:\n        z\n")
        types = token_types(tokens)
        assert types.count("INDENT") == 2
        assert types.count("DEDENT") == 2

    def test_bracket_suppression(self) -> None:
        """INDENT/DEDENT are suppressed inside brackets."""
        tokens = tokenize_python("f(\n  1,\n  2\n)\n")
        types = token_types(tokens)
        assert "INDENT" not in types


# ============================================================================
# Test: Multi-Character Operators
# ============================================================================


class TestOperators:
    """Test multi-character operators in Python."""

    def test_equality(self) -> None:
        """``==`` comparison operator."""
        tokens = tokenize_python("x == y\n")
        types = token_types(tokens)
        assert "EQUALS_EQUALS" in types

    def test_inequality(self) -> None:
        """``!=`` comparison operator."""
        tokens = tokenize_python("x != y\n")
        types = token_types(tokens)
        assert "NOT_EQUALS" in types

    def test_exponentiation(self) -> None:
        """``**`` exponentiation operator."""
        tokens = tokenize_python("2 ** 10\n")
        types = token_types(tokens)
        assert "DOUBLE_STAR" in types

    def test_floor_division(self) -> None:
        """``//`` floor division operator."""
        tokens = tokenize_python("7 // 2\n")
        types = token_types(tokens)
        assert "FLOOR_DIV" in types

    def test_augmented_assignment(self) -> None:
        """``+=`` augmented assignment."""
        tokens = tokenize_python("x += 1\n")
        types = token_types(tokens)
        assert "PLUS_EQUALS" in types


# ============================================================================
# Test: Factory Function
# ============================================================================


class TestCreatePythonLexer:
    """Test the ``create_python_lexer()`` factory function."""

    def test_creates_lexer(self) -> None:
        """The factory function should return a GrammarLexer instance."""
        lexer = create_python_lexer("x = 1\n")
        assert hasattr(lexer, "tokenize")

    def test_factory_produces_same_result(self) -> None:
        """The factory should produce the same tokens as tokenize_python()."""
        source = "def greet(name):\n    return name\n"
        tokens_direct = tokenize_python(source)
        tokens_factory = create_python_lexer(source).tokenize()
        assert tokens_direct == tokens_factory

    def test_factory_with_version(self) -> None:
        """The factory should accept a version parameter."""
        lexer = create_python_lexer("x = 1\n", version="2.7")
        tokens = lexer.tokenize()
        types = token_types(tokens)
        assert types[0] == "NAME"


# ============================================================================
# Test: Error Handling
# ============================================================================


class TestErrorHandling:
    """Test error cases: unsupported versions and invalid input."""

    def test_unsupported_version_raises(self) -> None:
        """Requesting an unsupported version should raise ValueError."""
        with pytest.raises(ValueError, match="Unsupported Python version"):
            tokenize_python("x = 1\n", version="4.0")

    def test_unsupported_version_message(self) -> None:
        """The error message should list supported versions."""
        with pytest.raises(ValueError, match="Supported versions"):
            tokenize_python("x = 1\n", version="1.0")


# ============================================================================
# Test: Constants
# ============================================================================


class TestConstants:
    """Test that the exported constants have correct values."""

    def test_default_version(self) -> None:
        """DEFAULT_VERSION should be '3.12'."""
        assert DEFAULT_VERSION == "3.12"

    def test_supported_versions_list(self) -> None:
        """SUPPORTED_VERSIONS should contain all expected versions."""
        assert SUPPORTED_VERSIONS == ["2.7", "3.0", "3.6", "3.8", "3.10", "3.12"]

    def test_supported_versions_includes_default(self) -> None:
        """DEFAULT_VERSION should be in SUPPORTED_VERSIONS."""
        assert DEFAULT_VERSION in SUPPORTED_VERSIONS


# ============================================================================
# Test: Comment Skipping
# ============================================================================


class TestCommentSkipping:
    """Test that comments are skipped during tokenization."""

    def test_inline_comment(self) -> None:
        """A comment after a statement should be stripped."""
        tokens = tokenize_python("x = 1  # assign x\n")
        types = token_types(tokens)
        assert "COMMENT" not in types
        assert types == ["NAME", "EQUALS", "INT", "NEWLINE", "EOF"]

    def test_full_line_comment(self) -> None:
        """A comment-only line should produce no content tokens."""
        tokens = tokenize_python("# this is a comment\nx = 1\n")
        name_tokens = [t for t in tokens if t.type == TokenType.NAME]
        assert len(name_tokens) == 1
        assert name_tokens[0].value == "x"


# ============================================================================
# Test: Position Tracking
# ============================================================================


class TestPositionTracking:
    """Test that the lexer correctly tracks line and column numbers."""

    def test_first_token_position(self) -> None:
        """The first token should be at line 1, column 1."""
        tokens = tokenize_python("x = 1\n")
        assert tokens[0].line == 1
        assert tokens[0].column == 1

    def test_column_tracking(self) -> None:
        """Tokens on the same line should have increasing column numbers."""
        tokens = tokenize_python("x = 1\n")
        assert tokens[0].column == 1  # x
        assert tokens[1].column == 3  # =
        assert tokens[2].column == 5  # 1

    def test_line_tracking_across_lines(self) -> None:
        """Tokens on different lines should have different line numbers."""
        tokens = tokenize_python("x = 1\ny = 2\n")
        y_token = [t for t in tokens if t.value == "y"][0]
        assert y_token.line == 2


# ============================================================================
# Test: Grammar Caching
# ============================================================================


class TestGrammarCaching:
    """Test that grammar caching works correctly.

    The lexer caches parsed grammars per version to avoid re-reading
    and re-parsing the .tokens file on every call.
    """

    def test_second_call_uses_cache(self) -> None:
        """Calling tokenize_python twice with the same version should work.

        This verifies that the caching mechanism does not break anything.
        Both calls should produce identical results.
        """
        tokens1 = tokenize_python("x = 1\n", version="3.12")
        tokens2 = tokenize_python("x = 1\n", version="3.12")
        assert token_types(tokens1) == token_types(tokens2)

    def test_different_versions_different_results(self) -> None:
        """Different versions may produce different results for the same input.

        ``print`` is a keyword in 2.7 but a name in 3.0+, so tokenizing
        ``print`` should give different token types.
        """
        tokens_27 = tokenize_python("print\n", version="2.7")
        tokens_30 = tokenize_python("print\n", version="3.0")
        assert tokens_27[0].type != tokens_30[0].type
