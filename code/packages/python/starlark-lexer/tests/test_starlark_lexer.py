"""Tests for the Starlark Lexer.

These tests verify that the grammar-driven lexer, when loaded with the
``starlark.tokens`` grammar file, correctly tokenizes Starlark source code.

The key insight being tested here is that **no new lexer code was written**.
The same ``GrammarLexer`` that handles Python and Ruby handles Starlark — only
the grammar file changed. These tests prove that the grammar-driven approach
truly is language-agnostic.

However, Starlark exercises **three features** that the Ruby lexer does not:

1. **Indentation mode** — The lexer emits INDENT, DEDENT, and NEWLINE tokens
   based on leading whitespace changes. This is how Starlark (like Python)
   handles block structure without curly braces.

2. **Reserved keywords** — Certain Python keywords (``class``, ``import``,
   ``while``, etc.) are illegal in Starlark. The lexer raises an error
   immediately if it encounters one, rather than producing a confusing
   parse error later.

3. **Type aliases** — Multiple token patterns emit the same type. For
   example, ``STRING_DQ``, ``STRING_SQ``, ``STRING_TRIPLE_DQ``, and all
   the prefixed variants all emit ``STRING``. The tests verify that the
   alias resolution works correctly.

Test Organization
-----------------

Tests are organized by what they verify:

1. **Basic expressions** — names, numbers, operators
2. **Starlark keywords** — ``def``, ``return``, ``if``, ``for``, ``pass``, etc.
3. **Reserved keywords** — ``class``, ``import``, etc. cause errors
4. **String literals** — double-quoted, with escape sequences
5. **Indentation** — INDENT/DEDENT tokens from indented blocks
6. **Bracket suppression** — INDENT/DEDENT/NEWLINE suppressed inside brackets
7. **Multi-character operators** — ``==``, ``!=``, ``**``, ``//``, ``+=``, etc.
8. **Comment skipping** — comments are not tokenized
9. **Factory function** — ``create_starlark_lexer()`` works correctly
10. **Position tracking** — line and column numbers
"""

from __future__ import annotations

import pytest
from lexer import LexerError, Token, TokenType

from starlark_lexer import create_starlark_lexer, tokenize_starlark


# ============================================================================
# Helpers — make assertions more readable
# ============================================================================


def token_types(tokens: list[Token]) -> list[str]:
    """Extract just the type names from a token list.

    This helper makes test assertions much more readable. Instead of
    checking full Token objects, we can compare a simple list of strings::

        assert token_types(tokens) == ["NAME", "EQUALS", "INT", "NEWLINE", "EOF"]

    For ``TokenType`` enum members, we use ``.name`` (e.g., ``"NAME"``).
    For string types (like ``"INT"``, ``"INDENT"``, ``"DEDENT"``), we use
    the string directly. This is necessary because the Starlark grammar
    defines token types (INT, FLOAT, INDENT, etc.) that are beyond the
    base ``TokenType`` enum.

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
            # String token type (INT, FLOAT, INDENT, DEDENT, NEWLINE, EOF, etc.)
            result.append(t.type)
    return result


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
# Test: Basic Starlark Expressions
# ============================================================================


class TestBasicExpressions:
    """Test that simple Starlark expressions tokenize correctly.

    These tests cover the fundamental building blocks: variables, numbers,
    operators, and assignment. Starlark's basic syntax is identical to
    Python's, so these tests look similar to what you would write for a
    Python lexer.

    Note: Because Starlark uses indentation mode, every token stream ends
    with a ``NEWLINE`` token (for the final logical line) followed by
    ``EOF``. This is different from the Ruby lexer, which does not produce
    trailing NEWLINE tokens.
    """

    def test_simple_name(self) -> None:
        """Tokenize a simple identifier (variable name).

        In Starlark, identifiers follow the same rules as Python:
        start with a letter or underscore, followed by letters, digits,
        or underscores.
        """
        tokens = tokenize_starlark("x\n")
        types = token_types(tokens)
        assert types[0] == "NAME"
        assert tokens[0].value == "x"

    def test_integer_literal(self) -> None:
        """Tokenize an integer literal.

        Starlark uses ``INT`` as the token type for integers (not ``NUMBER``
        like the base lexer). This is because Starlark distinguishes between
        integers and floats at the lexer level.
        """
        tokens = tokenize_starlark("42\n")
        types = token_types(tokens)
        assert types[0] == "INT"
        assert tokens[0].value == "42"

    def test_simple_assignment(self) -> None:
        """Tokenize ``x = 1`` — the simplest assignment expression.

        This is the "hello world" of lexer testing. If this works, the
        basic plumbing (grammar loading, pattern matching, token creation)
        is all functioning.
        """
        tokens = tokenize_starlark("x = 1\n")
        types = token_types(tokens)
        assert types[0] == "NAME"
        assert types[1] == "EQUALS"
        assert types[2] == "INT"
        assert types[3] == "NEWLINE"
        assert types[-1] == "EOF"

    def test_arithmetic_operators(self) -> None:
        """Tokenize ``a + b - c * d / e`` — all four arithmetic operators.

        Verifies that plus, minus, star, and slash are all recognized
        as their own distinct token types.
        """
        tokens = tokenize_starlark("a + b - c * d / e\n")
        types = token_types(tokens)
        assert types == [
            "NAME", "PLUS", "NAME", "MINUS", "NAME",
            "STAR", "NAME", "SLASH", "NAME", "NEWLINE", "EOF",
        ]

    def test_parenthesized_expression(self) -> None:
        """Tokenize ``(1 + 2) * 3`` — parentheses for grouping.

        Parentheses override operator precedence. The lexer just sees
        them as LPAREN and RPAREN tokens; the parser handles precedence.
        """
        tokens = tokenize_starlark("(1 + 2) * 3\n")
        types = token_types(tokens)
        assert types == [
            "LPAREN", "INT", "PLUS", "INT", "RPAREN",
            "STAR", "INT", "NEWLINE", "EOF",
        ]


# ============================================================================
# Test: Starlark Keywords
# ============================================================================


class TestStarlarkKeywords:
    """Test that Starlark keywords are recognized correctly.

    Starlark has a specific set of keywords that are a strict subset of
    Python's keywords. Notable absences include ``class``, ``while``,
    ``try``, ``except``, ``import``, and ``with`` — these are reserved
    (see TestReservedKeywords) and cause errors if used.

    The keywords that Starlark DOES support are:
    ``and``, ``break``, ``continue``, ``def``, ``elif``, ``else``,
    ``for``, ``if``, ``in``, ``lambda``, ``load``, ``not``, ``or``,
    ``pass``, ``return``, ``True``, ``False``, ``None``
    """

    def test_def_keyword(self) -> None:
        """The ``def`` keyword starts function definitions in Starlark."""
        tokens = tokenize_starlark("def\n")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "def"

    def test_return_keyword(self) -> None:
        """The ``return`` keyword exits a function, optionally with a value."""
        tokens = tokenize_starlark("return\n")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "return"

    def test_if_elif_else(self) -> None:
        """Starlark uses ``elif`` (like Python, unlike Ruby's ``elsif``)."""
        tokens = tokenize_starlark("if elif else\n")
        keywords = [t.value for t in tokens if t.type == TokenType.KEYWORD]
        assert keywords == ["if", "elif", "else"]

    def test_for_in(self) -> None:
        """``for`` and ``in`` are both keywords used in for-loops.

        Example: ``for item in items:``
        """
        tokens = tokenize_starlark("for in\n")
        keywords = [t.value for t in tokens if t.type == TokenType.KEYWORD]
        assert keywords == ["for", "in"]

    def test_pass_keyword(self) -> None:
        """``pass`` is a no-op placeholder for empty function bodies.

        Example: ``def todo(): pass``
        """
        tokens = tokenize_starlark("pass\n")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "pass"

    def test_boolean_and_none_keywords(self) -> None:
        """Starlark uses ``True``, ``False``, and ``None`` (capitalized, like Python)."""
        tokens = tokenize_starlark("True False None\n")
        keywords = [t.value for t in tokens if t.type == TokenType.KEYWORD]
        assert keywords == ["True", "False", "None"]

    def test_logical_operators(self) -> None:
        """``and``, ``or``, and ``not`` are keyword-based logical operators."""
        tokens = tokenize_starlark("and or not\n")
        keywords = [t.value for t in tokens if t.type == TokenType.KEYWORD]
        assert keywords == ["and", "or", "not"]

    def test_loop_control_keywords(self) -> None:
        """``break`` and ``continue`` control for-loop iteration."""
        tokens = tokenize_starlark("break continue\n")
        keywords = [t.value for t in tokens if t.type == TokenType.KEYWORD]
        assert keywords == ["break", "continue"]

    def test_lambda_keyword(self) -> None:
        """``lambda`` creates anonymous single-expression functions."""
        tokens = tokenize_starlark("lambda\n")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "lambda"

    def test_load_keyword(self) -> None:
        """``load`` imports symbols from other Starlark files.

        This is Starlark's alternative to Python's ``import`` statement.
        Example: ``load("//rules:python.bzl", "py_library")``
        """
        tokens = tokenize_starlark("load\n")
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "load"

    def test_keyword_vs_name(self) -> None:
        """A keyword embedded in a longer name should NOT be a keyword.

        ``define`` starts with ``def`` but is not a keyword — it is a
        regular NAME. The lexer's regex for NAME is greedy (matches as
        many word characters as possible), so ``define`` is matched as
        a single NAME token, not as ``def`` + ``ine``.
        """
        tokens = tokenize_starlark("define\n")
        assert tokens[0].type == TokenType.NAME
        assert tokens[0].value == "define"


# ============================================================================
# Test: Reserved Keywords
# ============================================================================


class TestReservedKeywords:
    """Test that reserved keywords cause lex errors.

    Starlark intentionally removes certain Python constructs to keep the
    language deterministic and simple. The following Python keywords are
    **reserved** in Starlark — they cannot be used as identifiers, and
    encountering one is an immediate syntax error:

    ``as``, ``assert``, ``async``, ``await``, ``class``, ``del``,
    ``except``, ``finally``, ``from``, ``global``, ``import``, ``is``,
    ``nonlocal``, ``raise``, ``try``, ``while``, ``with``, ``yield``

    This is a deliberate design choice: if someone writes ``class Foo:``
    in a BUILD file, they should get a clear "reserved keyword" error
    rather than a confusing parse error.
    """

    def test_class_reserved(self) -> None:
        """``class`` is reserved — Starlark has no class definitions."""
        with pytest.raises(LexerError, match="Reserved keyword 'class'"):
            tokenize_starlark("class\n")

    def test_import_reserved(self) -> None:
        """``import`` is reserved — Starlark uses ``load()`` instead."""
        with pytest.raises(LexerError, match="Reserved keyword 'import'"):
            tokenize_starlark("import\n")

    def test_while_reserved(self) -> None:
        """``while`` is reserved — Starlark only has ``for`` loops.

        This is intentional: for-loops over finite collections guarantee
        termination, making BUILD file evaluation decidable.
        """
        with pytest.raises(LexerError, match="Reserved keyword 'while'"):
            tokenize_starlark("while\n")

    def test_try_reserved(self) -> None:
        """``try`` is reserved — Starlark has no exception handling."""
        with pytest.raises(LexerError, match="Reserved keyword 'try'"):
            tokenize_starlark("try\n")

    def test_raise_reserved(self) -> None:
        """``raise`` is reserved — errors in Starlark are always fatal."""
        with pytest.raises(LexerError, match="Reserved keyword 'raise'"):
            tokenize_starlark("raise\n")

    def test_yield_reserved(self) -> None:
        """``yield`` is reserved — Starlark has no generators."""
        with pytest.raises(LexerError, match="Reserved keyword 'yield'"):
            tokenize_starlark("yield\n")


# ============================================================================
# Test: String Literals
# ============================================================================


class TestStringLiterals:
    """Test tokenization of string literals in Starlark.

    Starlark supports four quoting styles: double-quoted, single-quoted,
    triple-double-quoted, and triple-single-quoted. All variants emit the
    same ``STRING`` token type thanks to the ``-> STRING`` alias in the
    ``.tokens`` file.

    The lexer also handles optional ``r``/``b`` prefixes for raw strings
    and byte strings.
    """

    def test_double_quoted_string(self) -> None:
        """Tokenize a simple double-quoted string literal."""
        tokens = tokenize_starlark('"hello"\n')
        assert tokens[0].type == TokenType.STRING
        assert tokens[0].value == "hello"

    def test_single_quoted_string(self) -> None:
        """Tokenize a single-quoted string literal.

        Starlark (like Python) allows both ``'...'`` and ``"..."``
        for strings. Both produce the same STRING token type.
        """
        tokens = tokenize_starlark("'hello'\n")
        assert tokens[0].type == TokenType.STRING

    def test_string_with_escape_newline(self) -> None:
        r"""Tokenize a string with ``\n`` escape sequence."""
        tokens = tokenize_starlark(r'"hello\nworld"' + "\n")
        assert tokens[0].type == TokenType.STRING

    def test_string_in_expression(self) -> None:
        """Tokenize ``print("hello")`` — a string used as a function argument."""
        tokens = tokenize_starlark('print("hello")\n')
        types = token_types(tokens)
        assert "STRING" in types


# ============================================================================
# Test: Indentation — INDENT/DEDENT Tokens
# ============================================================================


class TestIndentation:
    """Test that indented blocks produce INDENT and DEDENT tokens.

    Starlark, like Python, uses significant indentation to define block
    structure. When the lexer encounters a line that is more indented than
    the previous line, it emits an ``INDENT`` token. When indentation
    decreases, it emits one or more ``DEDENT`` tokens to close the
    open blocks.

    The indentation algorithm maintains a stack of indentation levels:

    1. Start with stack ``[0]`` (top-level, no indentation).
    2. At each line start:
       - If indent > top of stack → push, emit INDENT
       - If indent < top of stack → pop until match, emit DEDENT for each pop
       - If indent == top of stack → no change
    3. At EOF → emit DEDENT for each remaining level on the stack.
    """

    def test_simple_function_body(self) -> None:
        """A function definition with one indented line.

        Source::

            def f():
                return 1

        Expected tokens: KEYWORD(def) NAME(f) LPAREN RPAREN COLON
        NEWLINE INDENT KEYWORD(return) INT(1) NEWLINE DEDENT EOF
        """
        tokens = tokenize_starlark("def f():\n    return 1\n")
        types = token_types(tokens)

        # Verify the INDENT/DEDENT sandwich structure
        assert "INDENT" in types
        assert "DEDENT" in types

        # INDENT should come after the NEWLINE that follows the colon
        indent_idx = types.index("INDENT")
        assert types[indent_idx - 1] == "NEWLINE"

        # DEDENT should come after the body's NEWLINE
        dedent_idx = types.index("DEDENT")
        assert types[dedent_idx - 1] == "NEWLINE"

    def test_nested_indentation(self) -> None:
        """Two levels of indentation (nested blocks).

        Source::

            if x:
                if y:
                    z

        This should produce two INDENT tokens (one for each level) and
        two DEDENT tokens at the end.
        """
        tokens = tokenize_starlark("if x:\n    if y:\n        z\n")
        types = token_types(tokens)

        indent_count = types.count("INDENT")
        dedent_count = types.count("DEDENT")
        assert indent_count == 2
        assert dedent_count == 2

    def test_dedent_at_eof(self) -> None:
        """Unclosed blocks should get DEDENT tokens at EOF.

        If the source ends while still indented, the lexer must emit
        enough DEDENT tokens to close all open blocks before emitting EOF.
        """
        tokens = tokenize_starlark("if x:\n    y\n")
        types = token_types(tokens)
        assert "INDENT" in types
        assert "DEDENT" in types
        # EOF must be the last token
        assert types[-1] == "EOF"


# ============================================================================
# Test: Bracket Suppression
# ============================================================================


class TestBracketSuppression:
    """Test that INDENT/DEDENT/NEWLINE are suppressed inside brackets.

    Starlark (and Python) allow multi-line expressions inside parentheses,
    square brackets, and curly braces without worrying about indentation.
    This is essential for BUILD file readability::

        cc_library(
            name = "foo",
            srcs = ["foo.cc"],
        )

    Inside the ``()``, the indentation changes are ignored. The lexer
    tracks bracket depth and suppresses INDENT/DEDENT/NEWLINE tokens when
    the depth is greater than zero.
    """

    def test_parenthesized_multiline(self) -> None:
        """Multi-line function call with parentheses.

        Source::

            f(
              1,
              2
            )

        Inside the parentheses, the indentation of ``1,`` and ``2`` should
        NOT produce INDENT/DEDENT tokens. This is the bracket suppression
        behavior.
        """
        tokens = tokenize_starlark("f(\n  1,\n  2\n)\n")
        types = token_types(tokens)
        assert "INDENT" not in types

    def test_square_bracket_multiline(self) -> None:
        """Multi-line list literal with square brackets.

        Source::

            [
              1,
              2,
            ]

        Square brackets also suppress indentation tokens.
        """
        tokens = tokenize_starlark("[\n  1,\n  2,\n]\n")
        types = token_types(tokens)
        assert "INDENT" not in types

    def test_curly_brace_multiline(self) -> None:
        """Multi-line dictionary literal with curly braces.

        Source::

            {
              "a": 1,
              "b": 2,
            }

        Curly braces also suppress indentation tokens.
        """
        tokens = tokenize_starlark('{\n  "a": 1,\n  "b": 2,\n}\n')
        types = token_types(tokens)
        assert "INDENT" not in types


# ============================================================================
# Test: Multi-Character Operators
# ============================================================================


class TestMultiCharOperators:
    """Test multi-character operators unique to or important in Starlark.

    Starlark supports a rich set of operators from Python, including
    exponentiation (``**``), floor division (``//``), augmented assignment
    (``+=``, ``-=``, etc.), and bitwise shifts (``<<``, ``>>``).

    These tests verify that multi-character operators are matched as single
    tokens, not split into separate single-character tokens. This depends
    on the first-match-wins ordering in ``starlark.tokens``: ``**`` is
    listed before ``*``, ``==`` before ``=``, etc.
    """

    def test_equality_and_inequality(self) -> None:
        """``==`` and ``!=`` comparison operators."""
        tokens = tokenize_starlark("x == y\n")
        types = token_types(tokens)
        assert "EQUALS_EQUALS" in types

        tokens = tokenize_starlark("x != y\n")
        types = token_types(tokens)
        assert "NOT_EQUALS" in types

    def test_comparison_operators(self) -> None:
        """``<=`` and ``>=`` comparison operators."""
        tokens = tokenize_starlark("x <= y\n")
        types = token_types(tokens)
        assert "LESS_EQUALS" in types

        tokens = tokenize_starlark("x >= y\n")
        types = token_types(tokens)
        assert "GREATER_EQUALS" in types

    def test_exponentiation(self) -> None:
        """``**`` is the exponentiation operator: ``2 ** 10`` = 1024."""
        tokens = tokenize_starlark("2 ** 10\n")
        types = token_types(tokens)
        assert "DOUBLE_STAR" in types

    def test_floor_division(self) -> None:
        """``//`` is floor division: ``7 // 2`` = 3."""
        tokens = tokenize_starlark("7 // 2\n")
        types = token_types(tokens)
        assert "FLOOR_DIV" in types

    def test_augmented_assignment_plus_equals(self) -> None:
        """``+=`` augmented assignment operator."""
        tokens = tokenize_starlark("x += 1\n")
        types = token_types(tokens)
        assert "PLUS_EQUALS" in types

    def test_augmented_assignment_star_equals(self) -> None:
        """``*=`` augmented assignment operator."""
        tokens = tokenize_starlark("x *= 2\n")
        types = token_types(tokens)
        assert "STAR_EQUALS" in types

    def test_shift_operators(self) -> None:
        """``<<`` and ``>>`` bitwise shift operators."""
        tokens = tokenize_starlark("x << 1\n")
        types = token_types(tokens)
        assert "LEFT_SHIFT" in types

        tokens = tokenize_starlark("x >> 1\n")
        types = token_types(tokens)
        assert "RIGHT_SHIFT" in types

    def test_three_char_operators(self) -> None:
        """Three-character operators: ``**=``, ``//=``, ``<<=``, ``>>=``."""
        tokens = tokenize_starlark("x **= 2\n")
        types = token_types(tokens)
        assert "DOUBLE_STAR_EQUALS" in types

        tokens = tokenize_starlark("x //= 2\n")
        types = token_types(tokens)
        assert "FLOOR_DIV_EQUALS" in types

    def test_multi_char_before_single_char(self) -> None:
        """``==`` must be matched before ``=``, and ``**`` before ``*``.

        This verifies that the first-match-wins ordering in ``starlark.tokens``
        is correct. Multi-character operators are listed before their
        single-character prefixes.
        """
        tokens = tokenize_starlark("x == y\n")
        eq_tokens = [t for t in tokens if t.value == "=="]
        assert len(eq_tokens) == 1

        tokens = tokenize_starlark("x ** 2\n")
        star_tokens = [t for t in tokens if t.value == "**"]
        assert len(star_tokens) == 1


# ============================================================================
# Test: Comment Skipping
# ============================================================================


class TestCommentSkipping:
    """Test that comments are skipped during tokenization.

    In Starlark (as in Python), comments start with ``#`` and extend to
    the end of the line. They are matched by the ``skip:`` section in
    ``starlark.tokens`` and produce no tokens.

    This is important for BUILD files, which are heavily commented::

        cc_library(
            name = "foo",
            srcs = ["foo.cc"],  # The main source file
        )
    """

    def test_inline_comment(self) -> None:
        """A comment after a statement should be stripped.

        ``x = 1  # assign x`` should produce the same tokens as ``x = 1``.
        """
        tokens = tokenize_starlark("x = 1  # assign x\n")
        types = token_types(tokens)
        # The comment should not appear as a token
        assert "COMMENT" not in types
        # The actual tokens should be: NAME EQUALS INT NEWLINE EOF
        assert types == ["NAME", "EQUALS", "INT", "NEWLINE", "EOF"]

    def test_full_line_comment(self) -> None:
        """A line that is only a comment should produce no content tokens.

        The comment line might produce a NEWLINE, but no NAME/INT/etc.
        """
        tokens = tokenize_starlark("# this is a comment\nx = 1\n")
        # Should still have the x = 1 tokens
        name_tokens = [t for t in tokens if t.type == TokenType.NAME]
        assert len(name_tokens) == 1
        assert name_tokens[0].value == "x"


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
        tokens = tokenize_starlark("x = 1\n")
        assert tokens[0].line == 1
        assert tokens[0].column == 1

    def test_column_tracking(self) -> None:
        """Tokens on the same line should have increasing column numbers."""
        tokens = tokenize_starlark("x = 1\n")
        # x is at column 1
        assert tokens[0].column == 1
        # = is at column 3 (after "x ")
        assert tokens[1].column == 3
        # 1 is at column 5 (after "x = ")
        assert tokens[2].column == 5

    def test_line_tracking_across_lines(self) -> None:
        """Tokens on different lines should have different line numbers."""
        tokens = tokenize_starlark("x = 1\ny = 2\n")
        # Find 'y' on line 2
        y_token = [t for t in tokens if t.value == "y"][0]
        assert y_token.line == 2


# ============================================================================
# Test: Factory Function
# ============================================================================


class TestCreateStarlarkLexer:
    """Test the ``create_starlark_lexer()`` factory function.

    While ``tokenize_starlark()`` is the simpler interface,
    ``create_starlark_lexer()`` gives access to the ``GrammarLexer``
    object itself, which is useful for advanced use cases like
    inspecting internal state or integrating with custom pipelines.
    """

    def test_creates_lexer(self) -> None:
        """The factory function should return a GrammarLexer instance."""
        lexer = create_starlark_lexer("x = 1\n")
        # Verify it has a tokenize method
        assert hasattr(lexer, "tokenize")

    def test_factory_produces_same_result(self) -> None:
        """The factory function should produce the same tokens as tokenize_starlark()."""
        source = "def greet(name):\n    return name\n"
        tokens_direct = tokenize_starlark(source)
        tokens_factory = create_starlark_lexer(source).tokenize()
        assert tokens_direct == tokens_factory

    def test_factory_with_operators(self) -> None:
        """Verify the factory works with Starlark-specific operators."""
        lexer = create_starlark_lexer("2 ** 10\n")
        tokens = lexer.tokenize()
        types = token_types(tokens)
        assert "DOUBLE_STAR" in types


# ============================================================================
# Test: Float Literals
# ============================================================================


class TestFloatLiterals:
    """Test tokenization of floating-point number literals.

    Starlark supports float literals like ``3.14``, ``.5``, ``1e10``.
    These are tokenized as FLOAT tokens, distinct from INT tokens.
    """

    def test_decimal_float(self) -> None:
        """Tokenize ``3.14`` — a standard decimal float."""
        tokens = tokenize_starlark("3.14\n")
        types = token_types(tokens)
        assert types[0] == "FLOAT"
        assert tokens[0].value == "3.14"

    def test_scientific_notation(self) -> None:
        """Tokenize ``1e10`` — scientific notation."""
        tokens = tokenize_starlark("1e10\n")
        types = token_types(tokens)
        assert types[0] == "FLOAT"
