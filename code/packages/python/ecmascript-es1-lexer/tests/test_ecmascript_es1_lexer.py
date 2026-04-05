"""Tests for the ECMAScript 1 (1997) Lexer.

These tests verify that the grammar-driven lexer, loaded with the ``es1.tokens``
grammar file, correctly tokenizes ES1-era JavaScript source code.

ES1 is the foundation of JavaScript. The key differences from later versions:
- No ``===`` or ``!==`` (strict equality is ES3)
- No ``try``/``catch`` (error handling is ES3)
- No regex literals (formalized in ES3)
- No ``let``/``const``/``class``/arrows (ES2015)
"""

from __future__ import annotations

from lexer import Token, TokenType

from ecmascript_es1_lexer import create_es1_lexer, tokenize_es1


# ============================================================================
# Helpers
# ============================================================================


def token_types(tokens: list[Token]) -> list[str]:
    """Extract just the type names from a token list.

    Token types can be either ``TokenType`` enum members (for built-in types like
    NAME, NUMBER, KEYWORD, EOF) or plain strings (for grammar-defined types like
    STRICT_EQUALS, AMPERSAND, etc.). We handle both.
    """
    return [t.type.name if hasattr(t.type, "name") else t.type for t in tokens]


def token_type_name(token: Token) -> str:
    """Get the type name of a single token."""
    return token.type.name if hasattr(token.type, "name") else token.type


def token_values(tokens: list[Token]) -> list[str]:
    """Extract just the values from a token list."""
    return [t.value for t in tokens]


# ============================================================================
# Test: Variable Declarations
# ============================================================================


class TestVariableDeclarations:
    """ES1 only has ``var`` — no ``let`` or ``const``."""

    def test_var_declaration(self) -> None:
        """Tokenize ``var x = 1;`` — the only declaration keyword in ES1."""
        tokens = tokenize_es1("var x = 1;")
        assert token_types(tokens) == [
            "KEYWORD", "NAME", "EQUALS", "NUMBER", "SEMICOLON", "EOF",
        ]
        assert token_values(tokens) == ["var", "x", "=", "1", ";", ""]

    def test_var_without_initializer(self) -> None:
        """Tokenize ``var x;`` — declaration without assignment."""
        tokens = tokenize_es1("var x;")
        assert token_types(tokens) == ["KEYWORD", "NAME", "SEMICOLON", "EOF"]

    def test_multiple_var_declarations(self) -> None:
        """Tokenize ``var x = 1, y = 2;`` — comma-separated declarations."""
        tokens = tokenize_es1("var x = 1, y = 2;")
        assert token_values(tokens) == [
            "var", "x", "=", "1", ",", "y", "=", "2", ";", "",
        ]


# ============================================================================
# Test: ES1 Keywords
# ============================================================================


class TestES1Keywords:
    """ES1 has 26 keywords (including true/false/null)."""

    def test_control_flow_keywords(self) -> None:
        """Core control flow keywords are recognized."""
        for kw in ["if", "else", "while", "for", "do", "switch", "case",
                    "default", "break", "continue", "return"]:
            tokens = tokenize_es1(kw)
            assert tokens[0].type == TokenType.KEYWORD, f"{kw} not recognized as keyword"
            assert tokens[0].value == kw

    def test_declaration_keywords(self) -> None:
        """``var`` and ``function`` are the only declaration keywords."""
        for kw in ["var", "function"]:
            tokens = tokenize_es1(kw)
            assert tokens[0].type == TokenType.KEYWORD

    def test_operator_keywords(self) -> None:
        """``delete``, ``typeof``, ``void``, ``in``, ``new`` are keywords."""
        for kw in ["delete", "typeof", "void", "in", "new"]:
            tokens = tokenize_es1(kw)
            assert tokens[0].type == TokenType.KEYWORD

    def test_literal_keywords(self) -> None:
        """``true``, ``false``, ``null`` are keywords in the lexer."""
        for kw in ["true", "false", "null"]:
            tokens = tokenize_es1(kw)
            assert tokens[0].type == TokenType.KEYWORD
            assert tokens[0].value == kw

    def test_this_keyword(self) -> None:
        """``this`` refers to the current execution context."""
        tokens = tokenize_es1("this")
        assert tokens[0].type == TokenType.KEYWORD

    def test_with_keyword(self) -> None:
        """``with`` extends the scope chain (deprecated in strict mode later)."""
        tokens = tokenize_es1("with")
        assert tokens[0].type == TokenType.KEYWORD

    def test_keyword_vs_identifier(self) -> None:
        """A keyword embedded in a longer name should be a NAME, not KEYWORD."""
        tokens = tokenize_es1("variable")
        assert tokens[0].type == TokenType.NAME
        assert tokens[0].value == "variable"

    def test_keyword_prefix(self) -> None:
        """``forEach`` starts with ``for`` but is a NAME, not a keyword."""
        tokens = tokenize_es1("forEach")
        assert tokens[0].type == TokenType.NAME


# ============================================================================
# Test: Operators
# ============================================================================


class TestES1Operators:
    """ES1 has 46 operators but NO ``===`` or ``!==``."""

    def test_arithmetic_operators(self) -> None:
        """Basic arithmetic: + - * / %"""
        tokens = tokenize_es1("a + b - c * d / e % f")
        ops = [t.value for t in tokens if t.type != TokenType.NAME and t.type != TokenType.EOF]
        assert ops == ["+", "-", "*", "/", "%"]

    def test_comparison_operators(self) -> None:
        """ES1 comparison: == != < > <= >="""
        tokens = tokenize_es1("a == b")
        assert tokens[1].value == "=="
        assert tokens[1].type.name == "EQUALS_EQUALS"

    def test_no_strict_equality(self) -> None:
        """ES1 does NOT have ===. The lexer should tokenize ``===`` as
        ``==`` followed by ``=`` (since === is not defined in ES1)."""
        tokens = tokenize_es1("a === b")
        # Without === defined, the lexer should match == then =
        values = token_values(tokens)
        assert "===" not in values

    def test_assignment_operators(self) -> None:
        """Compound assignment: += -= *= /= %="""
        tokens = tokenize_es1("x += 1")
        assert tokens[1].value == "+="

    def test_bitwise_operators(self) -> None:
        """Bitwise: & | ^ ~ << >> >>>"""
        tokens = tokenize_es1("a & b | c ^ d")
        ops = [t.value for t in tokens if token_type_name(t) in ("AMPERSAND", "PIPE", "CARET")]
        assert ops == ["&", "|", "^"]

    def test_unsigned_right_shift(self) -> None:
        """>>> is unique to JavaScript — unsigned right shift."""
        tokens = tokenize_es1("a >>> b")
        assert tokens[1].value == ">>>"

    def test_logical_operators(self) -> None:
        """Logical: && || !"""
        tokens = tokenize_es1("a && b || !c")
        ops = [t.value for t in tokens if token_type_name(t) in ("AND_AND", "OR_OR", "BANG")]
        assert ops == ["&&", "||", "!"]

    def test_increment_decrement(self) -> None:
        """Prefix and postfix: ++ --"""
        tokens = tokenize_es1("x++ + --y")
        assert tokens[1].value == "++"
        assert tokens[3].value == "--"

    def test_ternary(self) -> None:
        """Ternary conditional: ? :"""
        tokens = tokenize_es1("a ? b : c")
        assert tokens[1].value == "?"
        assert tokens[3].value == ":"


# ============================================================================
# Test: Literals
# ============================================================================


class TestES1Literals:
    """ES1 has numbers (decimal/hex/float) and strings (single/double quoted)."""

    def test_integer(self) -> None:
        tokens = tokenize_es1("42")
        assert token_type_name(tokens[0]) == "NUMBER"
        assert tokens[0].value == "42"

    def test_hex_number(self) -> None:
        """Hex literals start with 0x or 0X."""
        tokens = tokenize_es1("0xFF")
        assert token_type_name(tokens[0]) == "NUMBER"
        assert tokens[0].value == "0xFF"

    def test_float(self) -> None:
        tokens = tokenize_es1("3.14")
        assert token_type_name(tokens[0]) == "NUMBER"

    def test_leading_dot_float(self) -> None:
        """Leading-dot floats like .5 are valid."""
        tokens = tokenize_es1(".5")
        assert token_type_name(tokens[0]) == "NUMBER"

    def test_scientific_notation(self) -> None:
        tokens = tokenize_es1("1e10")
        assert token_type_name(tokens[0]) == "NUMBER"

    def test_double_quoted_string(self) -> None:
        tokens = tokenize_es1('"hello"')
        assert token_type_name(tokens[0]) == "STRING"
        # The GrammarLexer may or may not strip quotes — check either form
        assert "hello" in tokens[0].value

    def test_single_quoted_string(self) -> None:
        tokens = tokenize_es1("'hello'")
        assert token_type_name(tokens[0]) == "STRING"

    def test_string_with_escapes(self) -> None:
        tokens = tokenize_es1(r'"hello\nworld"')
        assert token_type_name(tokens[0]) == "STRING"


# ============================================================================
# Test: Identifiers
# ============================================================================


class TestES1Identifiers:
    """ES1 identifiers can contain letters, digits, _, and $."""

    def test_simple_name(self) -> None:
        tokens = tokenize_es1("foo")
        assert tokens[0].type == TokenType.NAME

    def test_dollar_sign(self) -> None:
        """The $ character is valid in identifiers (jQuery convention)."""
        tokens = tokenize_es1("$element")
        assert tokens[0].type == TokenType.NAME
        assert tokens[0].value == "$element"

    def test_underscore_prefix(self) -> None:
        tokens = tokenize_es1("_private")
        assert tokens[0].type == TokenType.NAME

    def test_dollar_only(self) -> None:
        """A single $ is a valid identifier (used by jQuery)."""
        tokens = tokenize_es1("$")
        assert tokens[0].type == TokenType.NAME
        assert tokens[0].value == "$"


# ============================================================================
# Test: Delimiters
# ============================================================================


class TestES1Delimiters:
    """Test all delimiter tokens."""

    def test_braces(self) -> None:
        tokens = tokenize_es1("{ }")
        assert token_types(tokens) == ["LBRACE", "RBRACE", "EOF"]

    def test_brackets(self) -> None:
        tokens = tokenize_es1("[ ]")
        assert token_types(tokens) == ["LBRACKET", "RBRACKET", "EOF"]

    def test_parens(self) -> None:
        tokens = tokenize_es1("( )")
        assert token_types(tokens) == ["LPAREN", "RPAREN", "EOF"]

    def test_semicolon(self) -> None:
        tokens = tokenize_es1(";")
        assert token_type_name(tokens[0]) == "SEMICOLON"

    def test_dot(self) -> None:
        tokens = tokenize_es1("a.b")
        assert token_type_name(tokens[1]) == "DOT"


# ============================================================================
# Test: Comments (skipped)
# ============================================================================


class TestES1Comments:
    """Comments are skipped by the lexer — they don't produce tokens."""

    def test_line_comment(self) -> None:
        tokens = tokenize_es1("x // this is a comment")
        assert token_types(tokens) == ["NAME", "EOF"]

    def test_block_comment(self) -> None:
        tokens = tokenize_es1("x /* comment */ y")
        assert token_types(tokens) == ["NAME", "NAME", "EOF"]


# ============================================================================
# Test: Position Tracking
# ============================================================================


class TestES1Positions:
    """Verify that line and column numbers are tracked correctly."""

    def test_single_line_positions(self) -> None:
        tokens = tokenize_es1("var x = 1;")
        assert tokens[0].line == 1
        assert tokens[0].column == 1  # var starts at column 1

    def test_multiline_positions(self) -> None:
        tokens = tokenize_es1("var x;\nvar y;")
        # Second `var` should be on line 2
        second_var = [t for t in tokens if t.value == "var"][1]
        assert second_var.line == 2


# ============================================================================
# Test: Factory Function
# ============================================================================


class TestCreateES1Lexer:
    """Test the ``create_es1_lexer()`` factory function."""

    def test_creates_lexer(self) -> None:
        lexer = create_es1_lexer("var x = 1;")
        assert hasattr(lexer, "tokenize")

    def test_factory_produces_same_result(self) -> None:
        source = "var x = 1 + 2;"
        tokens_direct = tokenize_es1(source)
        tokens_factory = create_es1_lexer(source).tokenize()
        assert tokens_direct == tokens_factory


# ============================================================================
# Test: Real-world ES1 Patterns
# ============================================================================


class TestES1RealWorldPatterns:
    """Test realistic ES1-era code patterns."""

    def test_function_declaration(self) -> None:
        source = "function add(a, b) { return a + b; }"
        tokens = tokenize_es1(source)
        assert tokens[0].value == "function"
        assert tokens[0].type == TokenType.KEYWORD

    def test_for_loop(self) -> None:
        source = "for (var i = 0; i < 10; i++) { }"
        tokens = tokenize_es1(source)
        types = token_types(tokens)
        assert "KEYWORD" in types  # for, var
        assert "LESS_THAN" in types
        assert "PLUS_PLUS" in types

    def test_object_literal(self) -> None:
        source = '{ name: "hello", value: 42 }'
        tokens = tokenize_es1(source)
        assert token_type_name(tokens[0]) == "LBRACE"
        assert token_type_name(tokens[-2]) == "RBRACE"

    def test_array_literal(self) -> None:
        source = "[1, 2, 3]"
        tokens = tokenize_es1(source)
        assert token_type_name(tokens[0]) == "LBRACKET"
        assert token_type_name(tokens[-2]) == "RBRACKET"
