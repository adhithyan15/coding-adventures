"""Tests for the Ruby Lexer.

These tests verify that the grammar-driven lexer, when loaded with the
``ruby.tokens`` grammar file, correctly tokenizes Ruby source code.

The key insight being tested here is that **no new lexer code was written**.
The same ``GrammarLexer`` that handles Python handles Ruby — only the
grammar file changed. These tests prove that the grammar-driven approach
truly is language-agnostic.

Test Organization
-----------------

Tests are organized by what they verify:

1. **Basic expressions** — simple arithmetic and assignment
2. **Ruby keywords** — ``def``, ``end``, ``if``, ``puts``, ``true``, etc.
3. **Ruby-specific operators** — ``..``, ``=>``, ``!=``, ``<=``, ``>=``
4. **Method definitions** — ``def greet(name)``
5. **Strings** — including escape sequences
6. **Multi-line code** — multiple statements with newlines
7. **Position tracking** — line and column numbers
8. **Factory function** — ``create_ruby_lexer()`` works correctly
"""

from __future__ import annotations

from lexer import Token, TokenType

from ruby_lexer import create_ruby_lexer, tokenize_ruby


# ============================================================================
# Helper — makes assertions more readable
# ============================================================================


def token_types(tokens: list[Token]) -> list[str]:
    """Extract just the type names from a token list.

    This helper makes test assertions much more readable. Instead of
    checking full Token objects, we can compare a simple list of strings::

        assert token_types(tokens) == ["NAME", "EQUALS", "NUMBER", "EOF"]

    Args:
        tokens: A list of Token objects.

    Returns:
        A list of token type name strings.
    """
    return [t.type.name for t in tokens]


def token_values(tokens: list[Token]) -> list[str]:
    """Extract just the values from a token list.

    Similar to ``token_types()`` but for values. Useful when you want
    to verify that the lexer captured the right text for each token.

    Args:
        tokens: A list of Token objects.

    Returns:
        A list of token value strings.
    """
    return [t.value for t in tokens]


# ============================================================================
# Test: Basic Ruby Expressions
# ============================================================================


class TestBasicExpressions:
    """Test that simple Ruby expressions tokenize correctly.

    These tests cover the fundamental building blocks: variables, numbers,
    operators, and assignment. They overlap with Python syntax, which is
    expected — many languages share these basics.
    """

    def test_simple_assignment(self) -> None:
        """Tokenize ``x = 1 + 2`` — the simplest assignment expression.

        This is the "hello world" of lexer testing. If this works, the
        basic plumbing (grammar loading, pattern matching, token creation)
        is all functioning.
        """
        tokens = tokenize_ruby("x = 1 + 2")
        assert token_types(tokens) == [
            "NAME", "EQUALS", "NUMBER", "PLUS", "NUMBER", "EOF",
        ]
        assert token_values(tokens) == ["x", "=", "1", "+", "2", ""]

    def test_arithmetic_operators(self) -> None:
        """Tokenize ``a + b - c * d / e`` — all four arithmetic operators.

        Verifies that plus, minus, star, and slash are all recognized
        as their own distinct token types.
        """
        tokens = tokenize_ruby("a + b - c * d / e")
        assert token_types(tokens) == [
            "NAME", "PLUS", "NAME", "MINUS", "NAME",
            "STAR", "NAME", "SLASH", "NAME", "EOF",
        ]

    def test_parenthesized_expression(self) -> None:
        """Tokenize ``(1 + 2) * 3`` — parentheses for grouping.

        Parentheses override operator precedence. The lexer just sees
        them as LPAREN and RPAREN tokens; the parser handles precedence.
        """
        tokens = tokenize_ruby("(1 + 2) * 3")
        assert token_types(tokens) == [
            "LPAREN", "NUMBER", "PLUS", "NUMBER", "RPAREN",
            "STAR", "NUMBER", "EOF",
        ]

    def test_number_literals(self) -> None:
        """Tokenize various number literals: single digit, multi-digit, zero."""
        tokens = tokenize_ruby("0 1 42 100")
        numbers = [t.value for t in tokens if t.type == TokenType.NUMBER]
        assert numbers == ["0", "1", "42", "100"]

    def test_equality_operator(self) -> None:
        """Tokenize ``x == 1`` — the double-equals comparison operator.

        This tests that ``==`` is recognized as a single EQUALS_EQUALS
        token, not as two separate EQUALS tokens. The ``ruby.tokens``
        file lists ``==`` before ``=`` so that first-match-wins gives
        us the correct behavior.
        """
        tokens = tokenize_ruby("x == 1")
        assert token_types(tokens) == [
            "NAME", "EQUALS_EQUALS", "NUMBER", "EOF",
        ]


# ============================================================================
# Test: Ruby Keywords
# ============================================================================


class TestRubyKeywords:
    """Test that Ruby-specific keywords are recognized correctly.

    Keywords are identifiers that have special meaning in the language.
    The lexer recognizes them by matching the NAME pattern first, then
    checking against the keyword list from the ``ruby.tokens`` file.
    If the name is in the keyword list, it gets reclassified from NAME
    to KEYWORD.
    """

    def test_def_keyword(self) -> None:
        """The ``def`` keyword starts method definitions in Ruby."""
        tokens = tokenize_ruby("def")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "def"

    def test_end_keyword(self) -> None:
        """The ``end`` keyword closes blocks in Ruby (method bodies, if/else, etc.)."""
        tokens = tokenize_ruby("end")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "end"

    def test_if_else_elsif(self) -> None:
        """Ruby uses ``elsif`` (not ``elif`` like Python)."""
        tokens = tokenize_ruby("if else elsif")
        keywords = [t.value for t in tokens if t.type == TokenType.KEYWORD]
        assert keywords == ["if", "else", "elsif"]

    def test_puts_keyword(self) -> None:
        """``puts`` is Ruby's standard output method."""
        tokens = tokenize_ruby("puts")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "puts"

    def test_boolean_keywords(self) -> None:
        """Ruby uses lowercase ``true`` and ``false`` (Python uses ``True``/``False``)."""
        tokens = tokenize_ruby("true false")
        keywords = [t.value for t in tokens if t.type == TokenType.KEYWORD]
        assert keywords == ["true", "false"]

    def test_nil_keyword(self) -> None:
        """Ruby uses ``nil`` instead of Python's ``None``."""
        tokens = tokenize_ruby("nil")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "nil"

    def test_class_module_keywords(self) -> None:
        """Ruby has both ``class`` and ``module`` for defining types."""
        tokens = tokenize_ruby("class module")
        keywords = [t.value for t in tokens if t.type == TokenType.KEYWORD]
        assert keywords == ["class", "module"]

    def test_control_flow_keywords(self) -> None:
        """Ruby control flow keywords: while, for, do, unless, until."""
        tokens = tokenize_ruby("while for do unless until")
        keywords = [t.value for t in tokens if t.type == TokenType.KEYWORD]
        assert keywords == ["while", "for", "do", "unless", "until"]

    def test_keyword_vs_name(self) -> None:
        """A keyword embedded in a longer name should NOT be a keyword.

        ``define`` starts with ``def`` but is not a keyword — it is a
        regular NAME. The lexer's regex for NAME is greedy (matches as
        many word characters as possible), so ``define`` is matched as
        a single NAME token, not as ``def`` + ``ine``.
        """
        tokens = tokenize_ruby("define")
        assert tokens[0].type == TokenType.NAME
        assert tokens[0].value == "define"

    def test_exception_keywords(self) -> None:
        """Ruby exception handling keywords: begin, rescue, ensure."""
        tokens = tokenize_ruby("begin rescue ensure")
        keywords = [t.value for t in tokens if t.type == TokenType.KEYWORD]
        assert keywords == ["begin", "rescue", "ensure"]


# ============================================================================
# Test: Ruby-Specific Operators
# ============================================================================


class TestRubyOperators:
    """Test Ruby-specific operators that Python does not have.

    These operators demonstrate why the grammar-driven approach is powerful:
    adding ``..`` or ``=>`` to Ruby requires only a line in the ``.tokens``
    file. No Python code changes are needed.

    Note: These operators get token types that don't exist in the base
    ``TokenType`` enum (like ``DOT_DOT`` or ``HASH_ROCKET``). The
    ``GrammarLexer`` handles this gracefully by falling back to
    ``TokenType.NAME``. The important thing is that the *value* is
    correct — the parser can match on the value.
    """

    def test_range_operator(self) -> None:
        """The ``..`` operator creates ranges in Ruby: ``1..10``."""
        tokens = tokenize_ruby("1..10")
        assert tokens[0].value == "1"
        assert tokens[1].value == ".."
        assert tokens[2].value == "10"

    def test_hash_rocket(self) -> None:
        """The ``=>`` operator is used in Ruby hashes: ``key => value``."""
        tokens = tokenize_ruby("key => value")
        assert tokens[0].value == "key"
        assert tokens[1].value == "=>"
        assert tokens[2].value == "value"

    def test_not_equals(self) -> None:
        """The ``!=`` operator tests inequality in Ruby."""
        tokens = tokenize_ruby("x != 1")
        assert tokens[0].value == "x"
        assert tokens[1].value == "!="
        assert tokens[2].value == "1"

    def test_less_equals(self) -> None:
        """The ``<=`` operator tests less-than-or-equal in Ruby."""
        tokens = tokenize_ruby("x <= 10")
        assert tokens[0].value == "x"
        assert tokens[1].value == "<="
        assert tokens[2].value == "10"

    def test_greater_equals(self) -> None:
        """The ``>=`` operator tests greater-than-or-equal in Ruby."""
        tokens = tokenize_ruby("x >= 0")
        assert tokens[0].value == "x"
        assert tokens[1].value == ">="
        assert tokens[2].value == "0"

    def test_less_than_and_greater_than(self) -> None:
        """The ``<`` and ``>`` operators for simple comparisons."""
        tokens = tokenize_ruby("a < b")
        assert tokens[1].value == "<"

        tokens = tokenize_ruby("a > b")
        assert tokens[1].value == ">"

    def test_multi_char_before_single_char(self) -> None:
        """``==`` must be matched before ``=``, and ``>=`` before ``>``.

        This verifies that the first-match-wins ordering in ``ruby.tokens``
        is correct. Multi-character operators are listed before their
        single-character prefixes.
        """
        tokens = tokenize_ruby("x == y = z >= w > v")
        values = [t.value for t in tokens if t.value in ("==", "=", ">=", ">")]
        assert values == ["==", "=", ">=", ">"]


# ============================================================================
# Test: Method Definitions
# ============================================================================


class TestMethodDefinitions:
    """Test tokenization of Ruby method definitions.

    Ruby methods use ``def`` ... ``end`` blocks. The lexer's job is just
    to recognize the individual tokens — the parser will assemble them
    into a method definition AST node.
    """

    def test_simple_method_def(self) -> None:
        """Tokenize ``def greet(name)`` — a method with one parameter."""
        tokens = tokenize_ruby("def greet(name)")
        assert token_types(tokens) == [
            "KEYWORD", "NAME", "LPAREN", "NAME", "RPAREN", "EOF",
        ]
        assert tokens[0].value == "def"
        assert tokens[1].value == "greet"
        assert tokens[3].value == "name"

    def test_method_no_params(self) -> None:
        """Tokenize ``def hello()`` — a method with no parameters."""
        tokens = tokenize_ruby("def hello()")
        assert token_types(tokens) == [
            "KEYWORD", "NAME", "LPAREN", "RPAREN", "EOF",
        ]

    def test_method_multiple_params(self) -> None:
        """Tokenize ``def add(a, b)`` — a method with two parameters."""
        tokens = tokenize_ruby("def add(a, b)")
        assert token_types(tokens) == [
            "KEYWORD", "NAME", "LPAREN", "NAME", "COMMA",
            "NAME", "RPAREN", "EOF",
        ]


# ============================================================================
# Test: String Literals
# ============================================================================


class TestStringLiterals:
    """Test tokenization of double-quoted string literals.

    Ruby supports several string quoting styles (double-quoted, single-
    quoted, heredocs, etc.), but our grammar currently handles double-
    quoted strings — the most common form.
    """

    def test_simple_string(self) -> None:
        """Tokenize a simple string literal."""
        tokens = tokenize_ruby('"hello"')
        assert tokens[0].type == TokenType.STRING
        assert tokens[0].value == "hello"

    def test_string_with_spaces(self) -> None:
        """Tokenize a string containing spaces."""
        tokens = tokenize_ruby('"hello world"')
        assert tokens[0].type == TokenType.STRING
        assert tokens[0].value == "hello world"

    def test_string_with_escape_newline(self) -> None:
        r"""Tokenize a string with ``\n`` escape sequence."""
        tokens = tokenize_ruby(r'"hello\nworld"')
        assert tokens[0].type == TokenType.STRING
        assert tokens[0].value == "hello\nworld"

    def test_string_with_escape_tab(self) -> None:
        r"""Tokenize a string with ``\t`` escape sequence."""
        tokens = tokenize_ruby(r'"col1\tcol2"')
        assert tokens[0].type == TokenType.STRING
        assert tokens[0].value == "col1\tcol2"

    def test_string_with_escaped_quote(self) -> None:
        r"""Tokenize a string containing an escaped double quote."""
        tokens = tokenize_ruby(r'"say \"hi\""')
        assert tokens[0].type == TokenType.STRING
        assert tokens[0].value == 'say "hi"'

    def test_string_in_expression(self) -> None:
        """Tokenize ``puts("hello")`` — a string used as a method argument."""
        tokens = tokenize_ruby('puts("hello")')
        assert token_types(tokens) == [
            "KEYWORD", "LPAREN", "STRING", "RPAREN", "EOF",
        ]
        assert tokens[2].value == "hello"


# ============================================================================
# Test: Multi-line Ruby Code
# ============================================================================


class TestMultiLine:
    """Test tokenization of multi-line Ruby source code.

    Real Ruby programs span many lines. The lexer must correctly emit
    NEWLINE tokens between lines and track position (line/column)
    accurately.
    """

    def test_two_assignments(self) -> None:
        """Tokenize two lines of assignments."""
        source = "x = 1\ny = 2"
        tokens = tokenize_ruby(source)
        assert token_types(tokens) == [
            "NAME", "EQUALS", "NUMBER", "NEWLINE",
            "NAME", "EQUALS", "NUMBER", "EOF",
        ]

    def test_method_body(self) -> None:
        """Tokenize a simple Ruby method definition with a body.

        This tests multiple lines with keywords, expressions, and the
        ``end`` keyword that closes the method block.
        """
        source = "def add(a, b)\na + b\nend"
        tokens = tokenize_ruby(source)

        # Verify key tokens are present
        types = token_types(tokens)
        assert types[0] == "KEYWORD"  # def
        assert "NEWLINE" in types
        assert types[-2] == "KEYWORD"  # end

    def test_blank_lines(self) -> None:
        """Blank lines produce NEWLINE tokens but no other tokens."""
        source = "x = 1\n\ny = 2"
        tokens = tokenize_ruby(source)
        newline_count = sum(1 for t in tokens if t.type == TokenType.NEWLINE)
        assert newline_count == 2  # one after x=1, one blank line

    def test_if_else_end_block(self) -> None:
        """Tokenize a Ruby if/else/end block."""
        source = "if x\nputs(x)\nelse\nputs(y)\nend"
        tokens = tokenize_ruby(source)

        keywords = [t.value for t in tokens if t.type == TokenType.KEYWORD]
        assert "if" in keywords
        assert "puts" in keywords
        assert "else" in keywords
        assert "end" in keywords


# ============================================================================
# Test: Position Tracking
# ============================================================================


class TestPositionTracking:
    """Test that the lexer correctly tracks line and column numbers.

    Position tracking is crucial for error reporting. When the parser
    finds a syntax error, it needs to tell the user exactly where the
    problem is. That information comes from the lexer's position data.
    """

    def test_first_token_position(self) -> None:
        """The first token should be at line 1, column 1."""
        tokens = tokenize_ruby("x = 1")
        assert tokens[0].line == 1
        assert tokens[0].column == 1

    def test_column_tracking(self) -> None:
        """Tokens on the same line should have increasing column numbers."""
        tokens = tokenize_ruby("x = 1")
        # x is at column 1
        assert tokens[0].column == 1
        # = is at column 3 (after "x ")
        assert tokens[1].column == 3
        # 1 is at column 5 (after "x = ")
        assert tokens[2].column == 5

    def test_line_tracking(self) -> None:
        """Tokens on different lines should have different line numbers."""
        tokens = tokenize_ruby("x = 1\ny = 2")
        # First line tokens
        assert tokens[0].line == 1  # x
        # Second line tokens (after NEWLINE)
        y_token = [t for t in tokens if t.value == "y"][0]
        assert y_token.line == 2

    def test_multiline_positions(self) -> None:
        """Position tracking across three lines."""
        source = "a = 1\nb = 2\nc = 3"
        tokens = tokenize_ruby(source)

        a_token = tokens[0]
        assert a_token.line == 1 and a_token.column == 1

        # Find 'b' on line 2
        b_token = [t for t in tokens if t.value == "b"][0]
        assert b_token.line == 2 and b_token.column == 1

        # Find 'c' on line 3
        c_token = [t for t in tokens if t.value == "c"][0]
        assert c_token.line == 3 and c_token.column == 1


# ============================================================================
# Test: Factory Function
# ============================================================================


class TestCreateRubyLexer:
    """Test the ``create_ruby_lexer()`` factory function.

    While ``tokenize_ruby()`` is the simpler interface, ``create_ruby_lexer()``
    gives access to the ``GrammarLexer`` object itself, which is useful for
    advanced use cases.
    """

    def test_creates_lexer(self) -> None:
        """The factory function should return a GrammarLexer instance."""
        lexer = create_ruby_lexer("x = 1")
        # Verify it has a tokenize method
        assert hasattr(lexer, "tokenize")

    def test_factory_produces_same_result(self) -> None:
        """The factory function should produce the same tokens as tokenize_ruby()."""
        source = "def greet(name)\nputs(name)\nend"
        tokens_direct = tokenize_ruby(source)
        tokens_factory = create_ruby_lexer(source).tokenize()
        assert tokens_direct == tokens_factory

    def test_factory_with_operators(self) -> None:
        """Verify the factory works with Ruby-specific operators."""
        lexer = create_ruby_lexer("1..10")
        tokens = lexer.tokenize()
        assert tokens[1].value == ".."
