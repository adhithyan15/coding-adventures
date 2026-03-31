"""
Tests for the Grammar-Driven Lexer
====================================

These tests verify that ``GrammarLexer`` correctly tokenizes source code
using token definitions from a ``.tokens`` file. The critical property
we are testing is **interchangeability**: for all well-formed inputs, the
``GrammarLexer`` must produce *identical* token output to the hand-written
``Lexer``.

We test in three layers:

1. **Standalone tests** — verify GrammarLexer behavior on its own
2. **Comparison tests** — verify GrammarLexer matches the hand-written Lexer
3. **Custom grammar tests** — verify GrammarLexer works with minimal/custom
   grammars (not loaded from a file)

The python.tokens grammar file is loaded from the ``code/grammars/`` directory,
which is the canonical source of token definitions for this project.
"""

from __future__ import annotations

from pathlib import Path

import pytest
from grammar_tools import TokenDefinition, TokenGrammar, parse_token_grammar

from lexer.grammar_lexer import GrammarLexer, LexerContext
from lexer.tokenizer import Lexer, LexerConfig, LexerError, Token, TokenType

# ---------------------------------------------------------------------------
# Fixtures — load the python.tokens grammar once for all tests
# ---------------------------------------------------------------------------

GRAMMARS_DIR = Path(__file__).parent.parent.parent.parent.parent / "grammars"


@pytest.fixture()
def python_grammar() -> TokenGrammar:
    """Load and parse the python.tokens grammar file.

    This is the same grammar file that would be used in production —
    we are testing with real data, not synthetic test fixtures.
    """
    tokens_path = GRAMMARS_DIR / "python.tokens"
    assert tokens_path.exists(), f"Grammar file not found: {tokens_path}"
    return parse_token_grammar(tokens_path.read_text())


@pytest.fixture()
def python_config(python_grammar: TokenGrammar) -> LexerConfig:
    """Create a LexerConfig with the same keywords as python.tokens.

    This lets us run the hand-written Lexer with the same keyword set
    as the GrammarLexer, so their outputs should match exactly.
    """
    return LexerConfig(keywords=python_grammar.keywords)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def grammar_tokenize(
    source: str,
    grammar: TokenGrammar,
) -> list[Token]:
    """Convenience wrapper — tokenize a string with GrammarLexer."""
    return GrammarLexer(source, grammar).tokenize()


def hand_tokenize(
    source: str,
    config: LexerConfig | None = None,
) -> list[Token]:
    """Convenience wrapper — tokenize a string with the hand-written Lexer."""
    return Lexer(source, config).tokenize()


# ============================================================================
# Standalone tests — GrammarLexer behavior
# ============================================================================


class TestGrammarLexerBasics:
    """Test fundamental GrammarLexer behavior using the python.tokens grammar."""

    def test_simple_assignment(self, python_grammar: TokenGrammar) -> None:
        """The canonical test case: x = 1 + 2"""
        tokens = grammar_tokenize("x = 1 + 2", python_grammar)
        expected_types = [
            TokenType.NAME,
            TokenType.EQUALS,
            TokenType.NUMBER,
            TokenType.PLUS,
            TokenType.NUMBER,
            TokenType.EOF,
        ]
        expected_values = ["x", "=", "1", "+", "2", ""]
        assert [t.type for t in tokens] == expected_types
        assert [t.value for t in tokens] == expected_values

    def test_arithmetic_expression(self, python_grammar: TokenGrammar) -> None:
        """Verify precedence-sensitive operators: 1 + 2 * 3"""
        tokens = grammar_tokenize("1 + 2 * 3", python_grammar)
        expected_types = [
            TokenType.NUMBER,
            TokenType.PLUS,
            TokenType.NUMBER,
            TokenType.STAR,
            TokenType.NUMBER,
            TokenType.EOF,
        ]
        assert [t.type for t in tokens] == expected_types
        assert [t.value for t in tokens] == ["1", "+", "2", "*", "3", ""]

    def test_string_literal(self, python_grammar: TokenGrammar) -> None:
        """Verify string handling — quotes stripped, value preserved."""
        tokens = grammar_tokenize('"Hello, World!"', python_grammar)
        assert tokens[0].type == TokenType.STRING
        assert tokens[0].value == "Hello, World!"
        assert tokens[1].type == TokenType.EOF

    def test_empty_string(self, python_grammar: TokenGrammar) -> None:
        """Empty string literal should produce an empty value."""
        tokens = grammar_tokenize('""', python_grammar)
        assert tokens[0].type == TokenType.STRING
        assert tokens[0].value == ""

    def test_string_escape_newline(self, python_grammar: TokenGrammar) -> None:
        r"""Escape sequence \n should become a real newline."""
        tokens = grammar_tokenize(r'"hello\nworld"', python_grammar)
        assert tokens[0].value == "hello\nworld"

    def test_string_escape_tab(self, python_grammar: TokenGrammar) -> None:
        r"""Escape sequence \t should become a real tab."""
        tokens = grammar_tokenize(r'"col1\tcol2"', python_grammar)
        assert tokens[0].value == "col1\tcol2"

    def test_string_escape_backslash(self, python_grammar: TokenGrammar) -> None:
        r"""Escape sequence \\ should become a single backslash."""
        tokens = grammar_tokenize(r'"path\\to\\file"', python_grammar)
        assert tokens[0].value == "path\\to\\file"

    def test_string_escape_quote(self, python_grammar: TokenGrammar) -> None:
        r"""Escape sequence \" should become a literal quote."""
        tokens = grammar_tokenize(r'"He said \"hi\""', python_grammar)
        assert tokens[0].value == 'He said "hi"'

    def test_unknown_escape(self, python_grammar: TokenGrammar) -> None:
        r"""Unknown escape sequences pass through the escaped character."""
        tokens = grammar_tokenize(r'"hello\xworld"', python_grammar)
        assert tokens[0].value == "helloxworld"

    def test_multiline_input(self, python_grammar: TokenGrammar) -> None:
        """NEWLINE tokens should appear between lines."""
        tokens = grammar_tokenize("x = 1\ny = 2", python_grammar)
        types = [t.type for t in tokens]
        assert types == [
            TokenType.NAME,
            TokenType.EQUALS,
            TokenType.NUMBER,
            TokenType.NEWLINE,
            TokenType.NAME,
            TokenType.EQUALS,
            TokenType.NUMBER,
            TokenType.EOF,
        ]

    def test_blank_lines(self, python_grammar: TokenGrammar) -> None:
        """Consecutive newlines should produce consecutive NEWLINE tokens."""
        tokens = grammar_tokenize("x\n\ny", python_grammar)
        types = [t.type for t in tokens]
        assert types == [
            TokenType.NAME,
            TokenType.NEWLINE,
            TokenType.NEWLINE,
            TokenType.NAME,
            TokenType.EOF,
        ]

    def test_empty_input(self, python_grammar: TokenGrammar) -> None:
        """Empty input should produce only an EOF token."""
        tokens = grammar_tokenize("", python_grammar)
        assert len(tokens) == 1
        assert tokens[0].type == TokenType.EOF

    def test_only_whitespace(self, python_grammar: TokenGrammar) -> None:
        """Whitespace-only input should produce only an EOF token."""
        tokens = grammar_tokenize("   \t  ", python_grammar)
        assert len(tokens) == 1
        assert tokens[0].type == TokenType.EOF

    def test_equals_vs_equals_equals(self, python_grammar: TokenGrammar) -> None:
        """The grammar-driven lexer must distinguish = from == correctly.

        This works because python.tokens defines EQUALS_EQUALS ("==")
        before EQUALS ("="), so the longer match wins.
        """
        tokens = grammar_tokenize("a = b == c", python_grammar)
        types = [t.type for t in tokens]
        assert types == [
            TokenType.NAME,
            TokenType.EQUALS,
            TokenType.NAME,
            TokenType.EQUALS_EQUALS,
            TokenType.NAME,
            TokenType.EOF,
        ]

    def test_function_call_style(self, python_grammar: TokenGrammar) -> None:
        """Parentheses, commas, and names."""
        tokens = grammar_tokenize("print(x, y)", python_grammar)
        types = [t.type for t in tokens]
        assert types == [
            TokenType.NAME,
            TokenType.LPAREN,
            TokenType.NAME,
            TokenType.COMMA,
            TokenType.NAME,
            TokenType.RPAREN,
            TokenType.EOF,
        ]

    def test_no_spaces(self, python_grammar: TokenGrammar) -> None:
        """Tokens should be recognized even without spaces."""
        tokens = grammar_tokenize("x=1+2", python_grammar)
        types = [t.type for t in tokens]
        assert types == [
            TokenType.NAME,
            TokenType.EQUALS,
            TokenType.NUMBER,
            TokenType.PLUS,
            TokenType.NUMBER,
            TokenType.EOF,
        ]

    def test_position_tracking(self, python_grammar: TokenGrammar) -> None:
        """Line and column numbers should be tracked correctly."""
        tokens = grammar_tokenize("x = 1", python_grammar)
        assert tokens[0].line == 1
        assert tokens[0].column == 1  # x
        assert tokens[1].column == 3  # =
        assert tokens[2].column == 5  # 1

    def test_position_tracking_multiline(self, python_grammar: TokenGrammar) -> None:
        """Position tracking across line boundaries."""
        tokens = grammar_tokenize("abc\nde = 1", python_grammar)
        # abc: line 1, col 1
        assert tokens[0] == Token(TokenType.NAME, "abc", 1, 1)
        # de: line 2, col 1
        de_token = [t for t in tokens if t.value == "de"][0]
        assert de_token.line == 2
        assert de_token.column == 1

    def test_eof_position(self, python_grammar: TokenGrammar) -> None:
        """EOF token should be at the position after the last character."""
        tokens = grammar_tokenize("ab", python_grammar)
        eof = tokens[-1]
        assert eof.type == TokenType.EOF
        assert eof.line == 1
        assert eof.column == 3


# ============================================================================
# Keyword tests
# ============================================================================


class TestGrammarLexerKeywords:
    """Test that keywords from the .tokens file are recognized correctly."""

    def test_keyword_if(self, python_grammar: TokenGrammar) -> None:
        """The word 'if' should be classified as KEYWORD, not NAME."""
        tokens = grammar_tokenize("if x == 1", python_grammar)
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "if"

    def test_keyword_def(self, python_grammar: TokenGrammar) -> None:
        tokens = grammar_tokenize("def foo", python_grammar)
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "def"

    def test_non_keyword_stays_name(self, python_grammar: TokenGrammar) -> None:
        """Words that look like keywords but are not should remain NAME."""
        tokens = grammar_tokenize("iffy", python_grammar)
        assert tokens[0].type == TokenType.NAME
        assert tokens[0].value == "iffy"

    def test_all_python_keywords_recognized(
        self,
        python_grammar: TokenGrammar,
    ) -> None:
        """Every keyword listed in python.tokens should be recognized."""
        for keyword in python_grammar.keywords:
            tokens = grammar_tokenize(keyword, python_grammar)
            assert tokens[0].type == TokenType.KEYWORD, (
                f"Expected {keyword!r} to be KEYWORD, got {tokens[0].type}"
            )
            assert tokens[0].value == keyword


# ============================================================================
# Error tests
# ============================================================================


class TestGrammarLexerErrors:
    """Test that the GrammarLexer raises appropriate errors."""

    def test_unexpected_character(self, python_grammar: TokenGrammar) -> None:
        """Characters not in any pattern should raise LexerError."""
        with pytest.raises(LexerError, match="Unexpected character"):
            grammar_tokenize("@", python_grammar)

    def test_unexpected_character_hash(self, python_grammar: TokenGrammar) -> None:
        with pytest.raises(LexerError, match="Unexpected character"):
            grammar_tokenize("#", python_grammar)

    def test_error_position(self, python_grammar: TokenGrammar) -> None:
        """The error should report the correct position."""
        try:
            grammar_tokenize("x = @", python_grammar)
        except LexerError as e:
            assert e.line == 1
            assert e.column == 5

    def test_error_on_second_line(self, python_grammar: TokenGrammar) -> None:
        """Errors on the second line should have the correct line number."""
        try:
            grammar_tokenize("x = 1\n@", python_grammar)
        except LexerError as e:
            assert e.line == 2
            assert e.column == 1


# ============================================================================
# Comparison tests — GrammarLexer vs. hand-written Lexer
# ============================================================================


class TestGrammarLexerMatchesHandWritten:
    """Verify that GrammarLexer and Lexer produce identical output.

    This is the most important test class. If both lexers produce the same
    tokens for the same input, they are truly interchangeable. We test a
    variety of inputs to build confidence.
    """

    # A collection of source strings to test with both lexers.
    # Each entry is a source string that both lexers should handle identically.
    COMPARISON_INPUTS: list[str] = [
        # Simple expressions
        "x = 1 + 2",
        "1 + 2 * 3",
        "a + b - c",
        "x == 1",
        "a = b == c",
        # Operators without spaces
        "x=1+2",
        "+-*/",
        # Strings
        '"Hello, World!"',
        '""',
        '"abc 123"',
        # Parentheses and delimiters
        "print(x, y)",
        "(1 + 2)",
        "key: value",
        # Multi-line
        "x = 1\ny = 2",
        "a = 1\nb = 2\nc = a + b",
        "x\n\ny",
        # Whitespace variations
        "  x   =   1  ",
        "\tx",
        "x\r= 1",
        # Edge cases
        "",
        "   \t  ",
        "\n\n",
        "x",
        "_",
        "_foo",
        "var1",
        "hello_world_123",
        "0",
        "42",
        "1000",
        # Mixed
        'x = "hello"',
        '"a" "b"',
    ]

    @pytest.mark.parametrize("source", COMPARISON_INPUTS)
    def test_tokens_match(
        self,
        source: str,
        python_grammar: TokenGrammar,
        python_config: LexerConfig,
    ) -> None:
        """Both lexers should produce identical token lists."""
        grammar_tokens = grammar_tokenize(source, python_grammar)
        hand_tokens = hand_tokenize(source, python_config)
        assert grammar_tokens == hand_tokens, (
            f"Mismatch for input {source!r}:\n"
            f"  Grammar: {grammar_tokens}\n"
            f"  Hand:    {hand_tokens}"
        )

    def test_keyword_expression_matches(
        self,
        python_grammar: TokenGrammar,
        python_config: LexerConfig,
    ) -> None:
        """Both lexers should classify keywords identically."""
        source = "if x == 1"
        grammar_tokens = grammar_tokenize(source, python_grammar)
        hand_tokens = hand_tokenize(source, python_config)
        assert grammar_tokens == hand_tokens

    def test_string_escapes_match(
        self,
        python_grammar: TokenGrammar,
        python_config: LexerConfig,
    ) -> None:
        r"""Both lexers should handle escape sequences identically."""
        test_cases = [
            r'"hello\nworld"',
            r'"col1\tcol2"',
            r'"path\\to\\file"',
            r'"He said \"hi\""',
            r'"hello\xworld"',
        ]
        for source in test_cases:
            grammar_tokens = grammar_tokenize(source, python_grammar)
            hand_tokens = hand_tokenize(source, python_config)
            assert grammar_tokens == hand_tokens, (
                f"Mismatch for input {source!r}:\n"
                f"  Grammar: {grammar_tokens}\n"
                f"  Hand:    {hand_tokens}"
            )

    def test_return_keyword_in_expression(
        self,
        python_grammar: TokenGrammar,
        python_config: LexerConfig,
    ) -> None:
        """Both lexers should handle 'return x + 1' identically."""
        source = "return x + 1"
        grammar_tokens = grammar_tokenize(source, python_grammar)
        hand_tokens = hand_tokenize(source, python_config)
        assert grammar_tokens == hand_tokens


# ============================================================================
# Custom grammar tests — build a TokenGrammar programmatically
# ============================================================================


class TestCustomGrammar:
    """Test GrammarLexer with custom/minimal grammars built in code.

    This proves that the GrammarLexer works with any TokenGrammar, not
    just one loaded from a file.
    """

    def test_minimal_grammar_numbers_only(self) -> None:
        """A grammar that only recognizes numbers."""
        grammar = TokenGrammar(
            definitions=[
                TokenDefinition(
                    name="NUMBER",
                    pattern="[0-9]+",
                    is_regex=True,
                    line_number=1,
                ),
            ],
            keywords=[],
        )
        tokens = GrammarLexer("42", grammar).tokenize()
        assert tokens[0] == Token(TokenType.NUMBER, "42", 1, 1)
        assert tokens[1].type == TokenType.EOF

    def test_minimal_grammar_names_and_equals(self) -> None:
        """A grammar with names and a literal = operator."""
        grammar = TokenGrammar(
            definitions=[
                TokenDefinition(
                    name="NAME",
                    pattern="[a-zA-Z_][a-zA-Z0-9_]*",
                    is_regex=True,
                    line_number=1,
                ),
                TokenDefinition(
                    name="EQUALS",
                    pattern="=",
                    is_regex=False,
                    line_number=2,
                ),
            ],
            keywords=[],
        )
        tokens = GrammarLexer("x = y", grammar).tokenize()
        assert [t.type for t in tokens] == [
            TokenType.NAME,
            TokenType.EQUALS,
            TokenType.NAME,
            TokenType.EOF,
        ]

    def test_custom_grammar_with_keywords(self) -> None:
        """A grammar with a custom keyword list."""
        grammar = TokenGrammar(
            definitions=[
                TokenDefinition(
                    name="NAME",
                    pattern="[a-zA-Z_][a-zA-Z0-9_]*",
                    is_regex=True,
                    line_number=1,
                ),
            ],
            keywords=["let", "var"],
        )
        tokens = GrammarLexer("let x", grammar).tokenize()
        assert tokens[0].type == TokenType.KEYWORD
        assert tokens[0].value == "let"
        assert tokens[1].type == TokenType.NAME
        assert tokens[1].value == "x"

    def test_custom_grammar_unknown_token_name(self) -> None:
        """Token names not in TokenType use the string name as type.

        Extended grammars (like Starlark) define token names beyond the
        basic TokenType enum. When no enum member matches, the token name
        string is used as the type directly. This allows the grammar-driven
        lexer to support arbitrary token names without modifying the enum.
        """
        grammar = TokenGrammar(
            definitions=[
                TokenDefinition(
                    name="IDENTIFIER",
                    pattern="[a-zA-Z]+",
                    is_regex=True,
                    line_number=1,
                ),
            ],
            keywords=[],
        )
        tokens = GrammarLexer("hello", grammar).tokenize()
        # "IDENTIFIER" is not in TokenType, so it becomes a string type.
        assert tokens[0].type == "IDENTIFIER"
        assert tokens[0].value == "hello"

    def test_literal_pattern_escapes_special_chars(self) -> None:
        """Literal patterns should escape regex-special characters.

        The pattern "+" should match a literal +, not act as a regex
        quantifier. The GrammarLexer uses re.escape() for literal patterns.
        """
        grammar = TokenGrammar(
            definitions=[
                TokenDefinition(
                    name="PLUS",
                    pattern="+",
                    is_regex=False,
                    line_number=1,
                ),
            ],
            keywords=[],
        )
        tokens = GrammarLexer("+", grammar).tokenize()
        assert tokens[0] == Token(TokenType.PLUS, "+", 1, 1)

    def test_first_match_wins_ordering(self) -> None:
        """When two patterns could match, the first one defined should win.

        We define a literal "==" before "=" and verify that "==" is matched
        as a single token, not as two "=" tokens.
        """
        grammar = TokenGrammar(
            definitions=[
                TokenDefinition(
                    name="EQUALS_EQUALS",
                    pattern="==",
                    is_regex=False,
                    line_number=1,
                ),
                TokenDefinition(
                    name="EQUALS",
                    pattern="=",
                    is_regex=False,
                    line_number=2,
                ),
            ],
            keywords=[],
        )
        tokens = GrammarLexer("==", grammar).tokenize()
        assert tokens[0].type == TokenType.EQUALS_EQUALS
        assert tokens[0].value == "=="
        assert tokens[1].type == TokenType.EOF

    def test_newline_handling_with_custom_grammar(self) -> None:
        """Newlines should produce NEWLINE tokens regardless of grammar."""
        grammar = TokenGrammar(
            definitions=[
                TokenDefinition(
                    name="NUMBER",
                    pattern="[0-9]+",
                    is_regex=True,
                    line_number=1,
                ),
            ],
            keywords=[],
        )
        tokens = GrammarLexer("1\n2", grammar).tokenize()
        types = [t.type for t in tokens]
        assert types == [
            TokenType.NUMBER,
            TokenType.NEWLINE,
            TokenType.NUMBER,
            TokenType.EOF,
        ]

    def test_error_with_custom_grammar(self) -> None:
        """Unrecognized characters should raise LexerError."""
        grammar = TokenGrammar(
            definitions=[
                TokenDefinition(
                    name="NUMBER",
                    pattern="[0-9]+",
                    is_regex=True,
                    line_number=1,
                ),
            ],
            keywords=[],
        )
        with pytest.raises(LexerError, match="Unexpected character"):
            GrammarLexer("abc", grammar).tokenize()


# ============================================================================
# Skip patterns tests
# ============================================================================


class TestSkipPatterns:
    """Test the skip: section — patterns consumed without emitting tokens.

    Skip patterns replace the hardcoded whitespace-skipping logic when
    present. They are typically used for comments and inline whitespace
    in extended grammars like Starlark.
    """

    def test_skip_whitespace(self) -> None:
        """Explicit whitespace skip pattern replaces default behavior."""
        grammar = TokenGrammar(
            definitions=[
                TokenDefinition(
                    name="NUMBER", pattern="[0-9]+",
                    is_regex=True, line_number=1,
                ),
                TokenDefinition(
                    name="PLUS", pattern="+",
                    is_regex=False, line_number=2,
                ),
            ],
            skip_definitions=[
                TokenDefinition(
                    name="WS", pattern="[ \\t]+",
                    is_regex=True, line_number=10,
                ),
            ],
        )
        tokens = GrammarLexer("1 + 2", grammar).tokenize()
        types = [t.type for t in tokens]
        assert types == [
            TokenType.NUMBER, TokenType.PLUS,
            TokenType.NUMBER, TokenType.EOF,
        ]

    def test_skip_comments(self) -> None:
        """Comment skip pattern consumes # to end of line."""
        grammar = TokenGrammar(
            definitions=[
                TokenDefinition(
                    name="NAME", pattern="[a-zA-Z_][a-zA-Z0-9_]*",
                    is_regex=True, line_number=1,
                ),
                TokenDefinition(
                    name="EQUALS", pattern="=",
                    is_regex=False, line_number=2,
                ),
                TokenDefinition(
                    name="NUMBER", pattern="[0-9]+",
                    is_regex=True, line_number=3,
                ),
            ],
            skip_definitions=[
                TokenDefinition(
                    name="WS", pattern="[ \\t]+",
                    is_regex=True, line_number=10,
                ),
                TokenDefinition(
                    name="COMMENT", pattern="#[^\\n]*",
                    is_regex=True, line_number=11,
                ),
            ],
        )
        tokens = GrammarLexer("x = 1 # assign", grammar).tokenize()
        values = [t.value for t in tokens if t.type != TokenType.EOF]
        assert values == ["x", "=", "1"]

    def test_skip_multiple_patterns(self) -> None:
        """Multiple skip patterns are all tried."""
        grammar = TokenGrammar(
            definitions=[
                TokenDefinition(
                    name="NUMBER", pattern="[0-9]+",
                    is_regex=True, line_number=1,
                ),
            ],
            skip_definitions=[
                TokenDefinition(
                    name="WS", pattern="[ \\t]+",
                    is_regex=True, line_number=10,
                ),
                TokenDefinition(
                    name="COMMENT", pattern="#[^\\n]*",
                    is_regex=True, line_number=11,
                ),
            ],
        )
        tokens = GrammarLexer("42 # a number", grammar).tokenize()
        assert tokens[0].type == TokenType.NUMBER
        assert tokens[0].value == "42"
        assert tokens[1].type == TokenType.EOF


# ============================================================================
# Alias tests
# ============================================================================


class TestAliases:
    """Test the -> TYPE alias feature.

    When a token definition has an alias, the emitted token uses the alias
    as its type. This lets the parser grammar reference a single type
    (e.g., STRING) while the tokens file defines multiple patterns
    (STRING_DQ and STRING_SQ) that all emit STRING.
    """

    def test_regex_alias(self) -> None:
        """A regex definition with alias emits the alias type."""
        grammar = TokenGrammar(
            definitions=[
                TokenDefinition(
                    name="INT_DEC", pattern="[0-9]+",
                    is_regex=True, line_number=1, alias="INT",
                ),
            ],
        )
        tokens = GrammarLexer("42", grammar).tokenize()
        # INT is not in TokenType, so it becomes a string type.
        assert tokens[0].type == "INT"
        assert tokens[0].value == "42"

    def test_multiple_definitions_same_alias(self) -> None:
        """Multiple definitions can alias to the same type."""
        grammar = TokenGrammar(
            definitions=[
                TokenDefinition(
                    name="STRING_DQ", pattern='"[^"]*"',
                    is_regex=True, line_number=1, alias="STRING",
                ),
                TokenDefinition(
                    name="STRING_SQ", pattern="'[^']*'",
                    is_regex=True, line_number=2, alias="STRING",
                ),
            ],
            skip_definitions=[
                TokenDefinition(
                    name="WS", pattern="[ \\t]+",
                    is_regex=True, line_number=10,
                ),
            ],
        )
        tokens = GrammarLexer('"hello" \'world\'', grammar).tokenize()
        # Both should emit STRING type (mapped to TokenType.STRING)
        assert tokens[0].type == TokenType.STRING
        assert tokens[1].type == TokenType.STRING

    def test_alias_with_enum_member(self) -> None:
        """An alias that matches a TokenType enum member uses the enum."""
        grammar = TokenGrammar(
            definitions=[
                TokenDefinition(
                    name="INT_LIT", pattern="[0-9]+",
                    is_regex=True, line_number=1, alias="NUMBER",
                ),
            ],
        )
        tokens = GrammarLexer("42", grammar).tokenize()
        assert tokens[0].type == TokenType.NUMBER

    def test_alias_string_escape_processing(self) -> None:
        """String definitions with aliases still get escape processing."""
        grammar = TokenGrammar(
            definitions=[
                TokenDefinition(
                    name="STRING_DQ", pattern='"([^"\\\\\\\\]|\\\\\\\\.)*"',
                    is_regex=True, line_number=1, alias="STRING",
                ),
            ],
        )
        tokens = GrammarLexer('"hello"', grammar).tokenize()
        assert tokens[0].value == "hello"  # quotes stripped


# ============================================================================
# Reserved keywords tests
# ============================================================================


class TestReservedKeywords:
    """Test the reserved: section — identifiers that cause lex errors.

    In Starlark, words like 'class' and 'import' are reserved. They
    cannot be used as identifiers at all — the lexer raises an error
    immediately rather than letting the parser produce a confusing error.
    """

    def test_reserved_keyword_raises(self) -> None:
        """Using a reserved keyword raises LexerError."""
        grammar = TokenGrammar(
            definitions=[
                TokenDefinition(
                    name="NAME", pattern="[a-zA-Z_][a-zA-Z0-9_]*",
                    is_regex=True, line_number=1,
                ),
            ],
            reserved_keywords=["class", "import"],
        )
        with pytest.raises(LexerError, match="Reserved keyword 'class'"):
            GrammarLexer("class", grammar).tokenize()

    def test_reserved_keyword_import(self) -> None:
        """Another reserved keyword raises LexerError."""
        grammar = TokenGrammar(
            definitions=[
                TokenDefinition(
                    name="NAME", pattern="[a-zA-Z_][a-zA-Z0-9_]*",
                    is_regex=True, line_number=1,
                ),
            ],
            reserved_keywords=["class", "import"],
        )
        with pytest.raises(LexerError, match="Reserved keyword 'import'"):
            GrammarLexer("import", grammar).tokenize()

    def test_non_reserved_word_passes(self) -> None:
        """A non-reserved identifier tokenizes normally."""
        grammar = TokenGrammar(
            definitions=[
                TokenDefinition(
                    name="NAME", pattern="[a-zA-Z_][a-zA-Z0-9_]*",
                    is_regex=True, line_number=1,
                ),
            ],
            reserved_keywords=["class", "import"],
        )
        tokens = GrammarLexer("hello", grammar).tokenize()
        assert tokens[0].type == TokenType.NAME
        assert tokens[0].value == "hello"

    def test_reserved_vs_regular_keywords(self) -> None:
        """Reserved and regular keywords work independently.

        Regular keywords are reclassified to KEYWORD type. Reserved keywords
        raise errors. They don't interfere with each other.
        """
        grammar = TokenGrammar(
            definitions=[
                TokenDefinition(
                    name="NAME", pattern="[a-zA-Z_][a-zA-Z0-9_]*",
                    is_regex=True, line_number=1,
                ),
            ],
            keywords=["if", "else"],
            reserved_keywords=["class"],
        )
        # Regular keyword works
        tokens = GrammarLexer("if", grammar).tokenize()
        assert tokens[0].type == TokenType.KEYWORD
        # Reserved keyword errors
        with pytest.raises(LexerError, match="Reserved keyword 'class'"):
            GrammarLexer("class", grammar).tokenize()


# ============================================================================
# Indentation mode tests
# ============================================================================


class TestIndentationMode:
    """Test Python-style INDENT/DEDENT/NEWLINE tokenization.

    In indentation mode (``mode: indentation``), the lexer:
    - Maintains an indentation stack
    - Emits INDENT when indentation increases
    - Emits DEDENT when indentation decreases
    - Emits NEWLINE at end of logical lines
    - Suppresses NEWLINE/INDENT/DEDENT inside brackets
    - Rejects tab characters in leading whitespace
    """

    @pytest.fixture()
    def indent_grammar(self) -> TokenGrammar:
        """A minimal grammar with indentation mode enabled."""
        return TokenGrammar(
            definitions=[
                TokenDefinition(
                    name="NAME", pattern="[a-zA-Z_][a-zA-Z0-9_]*",
                    is_regex=True, line_number=1,
                ),
                TokenDefinition(
                    name="NUMBER", pattern="[0-9]+",
                    is_regex=True, line_number=2,
                ),
                TokenDefinition(
                    name="COLON", pattern=":",
                    is_regex=False, line_number=3,
                ),
                TokenDefinition(
                    name="EQUALS", pattern="=",
                    is_regex=False, line_number=4,
                ),
                TokenDefinition(
                    name="PLUS", pattern="+",
                    is_regex=False, line_number=5,
                ),
                TokenDefinition(
                    name="LPAREN", pattern="(",
                    is_regex=False, line_number=6,
                ),
                TokenDefinition(
                    name="RPAREN", pattern=")",
                    is_regex=False, line_number=7,
                ),
                TokenDefinition(
                    name="COMMA", pattern=",",
                    is_regex=False, line_number=8,
                ),
                TokenDefinition(
                    name="LBRACKET", pattern="[",
                    is_regex=False, line_number=9,
                ),
                TokenDefinition(
                    name="RBRACKET", pattern="]",
                    is_regex=False, line_number=10,
                ),
            ],
            keywords=["def", "return", "if", "pass"],
            mode="indentation",
            skip_definitions=[
                TokenDefinition(
                    name="WS", pattern="[ \\t]+",
                    is_regex=True, line_number=20,
                ),
                TokenDefinition(
                    name="COMMENT", pattern="#[^\\n]*",
                    is_regex=True, line_number=21,
                ),
            ],
        )

    def test_simple_indent_dedent(self, indent_grammar: TokenGrammar) -> None:
        """A simple function with one level of indentation."""
        source = "def f:\n    x = 1\n"
        tokens = GrammarLexer(source, indent_grammar).tokenize()
        types = [t.type for t in tokens]
        # def f : NEWLINE INDENT x = 1 NEWLINE DEDENT EOF
        # The final \n in the source produces the NEWLINE before DEDENT.
        # The DEDENT is emitted at EOF cleanup, then EOF itself.
        assert types == [
            TokenType.KEYWORD,  # def
            TokenType.NAME,     # f
            TokenType.COLON,    # :
            "NEWLINE",
            "INDENT",
            TokenType.NAME,     # x
            TokenType.EQUALS,   # =
            TokenType.NUMBER,   # 1
            "NEWLINE",
            "DEDENT",
            "EOF",
        ]

    def test_nested_indent(self, indent_grammar: TokenGrammar) -> None:
        """Two levels of indentation."""
        source = "if x:\n    if y:\n        z\n"
        tokens = GrammarLexer(source, indent_grammar).tokenize()
        types = [t.type for t in tokens]
        assert types == [
            TokenType.KEYWORD,  # if
            TokenType.NAME,     # x
            TokenType.COLON,    # :
            "NEWLINE",
            "INDENT",
            TokenType.KEYWORD,  # if
            TokenType.NAME,     # y
            TokenType.COLON,    # :
            "NEWLINE",
            "INDENT",
            TokenType.NAME,     # z
            "NEWLINE",
            "DEDENT",
            "DEDENT",
            "EOF",
        ]

    def test_multiple_dedents(self, indent_grammar: TokenGrammar) -> None:
        """Dedenting back to top level from two levels deep."""
        source = "a:\n    b:\n        c\nd\n"
        tokens = GrammarLexer(source, indent_grammar).tokenize()
        types = [t.type for t in tokens]
        # a : NEWLINE INDENT b : NEWLINE INDENT c NEWLINE DEDENT DEDENT d NEWLINE EOF
        assert "DEDENT" in types
        dedent_count = types.count("DEDENT")
        assert dedent_count == 2

    def test_eof_emits_remaining_dedents(self, indent_grammar: TokenGrammar) -> None:
        """EOF should emit DEDENT for remaining indent levels."""
        source = "a:\n    b"
        tokens = GrammarLexer(source, indent_grammar).tokenize()
        types = [t.type for t in tokens]
        # a : NEWLINE INDENT b NEWLINE DEDENT NEWLINE EOF
        assert "INDENT" in types
        assert "DEDENT" in types
        assert types[-1] == "EOF"

    def test_blank_lines_ignored(self, indent_grammar: TokenGrammar) -> None:
        """Blank lines don't affect indentation or emit NEWLINE."""
        source = "a:\n    b\n\n    c\n"
        tokens = GrammarLexer(source, indent_grammar).tokenize()
        # The blank line between b and c should not cause DEDENT/INDENT
        types = [t.type for t in tokens]
        indent_count = types.count("INDENT")
        assert indent_count == 1  # Only one INDENT (for the block)

    def test_comment_lines_ignored(self, indent_grammar: TokenGrammar) -> None:
        """Comment-only lines don't affect indentation."""
        source = "a:\n    b\n    # comment\n    c\n"
        tokens = GrammarLexer(source, indent_grammar).tokenize()
        types = [t.type for t in tokens]
        indent_count = types.count("INDENT")
        assert indent_count == 1

    def test_implicit_line_joining_parens(self, indent_grammar: TokenGrammar) -> None:
        """Inside parentheses, newlines are suppressed."""
        source = "f(\n    a,\n    b\n)\n"
        tokens = GrammarLexer(source, indent_grammar).tokenize()
        types = [t.type for t in tokens]
        # No INDENT/DEDENT inside the parens
        assert "INDENT" not in types

    def test_implicit_line_joining_brackets(
        self, indent_grammar: TokenGrammar,
    ) -> None:
        """Inside square brackets, newlines are suppressed."""
        source = "x = [\n    1,\n    2\n]\n"
        tokens = GrammarLexer(source, indent_grammar).tokenize()
        types = [t.type for t in tokens]
        assert "INDENT" not in types

    def test_tab_in_indentation_raises(self, indent_grammar: TokenGrammar) -> None:
        """Tab characters in leading whitespace are rejected."""
        source = "a:\n\tb\n"
        with pytest.raises(LexerError, match="Tab character"):
            GrammarLexer(source, indent_grammar).tokenize()

    def test_inconsistent_dedent_raises(self, indent_grammar: TokenGrammar) -> None:
        """Inconsistent indentation raises LexerError."""
        source = "a:\n    b:\n        c\n   d\n"  # 3 spaces doesn't match
        with pytest.raises(LexerError, match="Indentation level"):
            GrammarLexer(source, indent_grammar).tokenize()

    def test_empty_input_indentation_mode(
        self, indent_grammar: TokenGrammar,
    ) -> None:
        """Empty input produces NEWLINE + EOF in indentation mode."""
        tokens = GrammarLexer("", indent_grammar).tokenize()
        # No tokens to emit, but we always get EOF
        assert tokens[-1].type == "EOF"

    def test_single_line_no_indent(self, indent_grammar: TokenGrammar) -> None:
        """A single line at top level — no INDENT/DEDENT needed."""
        source = "x = 1\n"
        tokens = GrammarLexer(source, indent_grammar).tokenize()
        types = [t.type for t in tokens]
        assert "INDENT" not in types
        assert "DEDENT" not in types

    def test_pass_statement_in_block(self, indent_grammar: TokenGrammar) -> None:
        """A real-ish Starlark-like snippet."""
        source = "def f:\n    pass\n"
        tokens = GrammarLexer(source, indent_grammar).tokenize()
        synthetic = ("NEWLINE", "INDENT", "DEDENT", "EOF")
        values = [t.value for t in tokens if t.type not in synthetic]
        assert values == ["def", "f", ":", "pass"]


# ============================================================================
# Starlark integration test
# ============================================================================


class TestStarlarkLexer:
    """Test lexing with the real starlark.tokens file.

    This is the ultimate integration test: can the grammar-driven lexer
    handle a real-world grammar with all extensions active?
    """

    @pytest.fixture()
    def starlark_grammar(self) -> TokenGrammar:
        """Load the starlark.tokens grammar file."""
        tokens_path = GRAMMARS_DIR / "starlark.tokens"
        if not tokens_path.exists():
            pytest.skip("starlark.tokens not found")
        return parse_token_grammar(tokens_path.read_text())

    def test_simple_assignment(self, starlark_grammar: TokenGrammar) -> None:
        """Lex a simple Starlark assignment."""
        tokens = GrammarLexer("x = 1\n", starlark_grammar).tokenize()
        # Filter out NEWLINE/EOF to check the core tokens
        core = [(t.type, t.value) for t in tokens
                if t.type not in ("NEWLINE", "EOF")]
        # NAME maps to TokenType.NAME (has an enum member), but INT is a string
        assert (TokenType.NAME, "x") in core
        assert (TokenType.EQUALS, "=") in core
        assert ("INT", "1") in core

    def test_function_definition(self, starlark_grammar: TokenGrammar) -> None:
        """Lex a Starlark function definition with indent/dedent."""
        source = "def add(x, y):\n    return x + y\n"
        tokens = GrammarLexer(source, starlark_grammar).tokenize()
        types = [t.type for t in tokens]
        # Should have INDENT and DEDENT
        assert "INDENT" in types
        assert "DEDENT" in types
        # 'def' and 'return' are keywords (mapped to TokenType.KEYWORD)
        keyword_tokens = [t for t in tokens if t.type == TokenType.KEYWORD]
        keyword_values = [t.value for t in keyword_tokens]
        assert "def" in keyword_values
        assert "return" in keyword_values

    def test_reserved_keyword_rejected(
        self, starlark_grammar: TokenGrammar,
    ) -> None:
        """Reserved keywords like 'class' cause lex errors in Starlark."""
        with pytest.raises(LexerError, match="Reserved keyword 'class'"):
            GrammarLexer("class Foo:\n    pass\n", starlark_grammar).tokenize()

    def test_string_literals(self, starlark_grammar: TokenGrammar) -> None:
        """Lex Starlark string literals."""
        source = 'x = "hello"\n'
        tokens = GrammarLexer(source, starlark_grammar).tokenize()
        string_tokens = [t for t in tokens if t.type == TokenType.STRING]
        assert len(string_tokens) == 1
        assert string_tokens[0].value == "hello"

    def test_comment_skipped(self, starlark_grammar: TokenGrammar) -> None:
        """Comments are consumed by skip patterns."""
        source = "x = 1  # a comment\n"
        tokens = GrammarLexer(source, starlark_grammar).tokenize()
        # No token should contain "comment"
        values = [t.value for t in tokens]
        assert not any("comment" in v for v in values)

    def test_multiline_with_brackets(
        self, starlark_grammar: TokenGrammar,
    ) -> None:
        """Implicit line joining inside brackets."""
        source = "x = [\n    1,\n    2,\n]\n"
        tokens = GrammarLexer(source, starlark_grammar).tokenize()
        types = [t.type for t in tokens]
        # No INDENT/DEDENT inside brackets
        # After the brackets close, there should be no indentation artifacts
        assert "INDENT" not in types


# ===========================================================================
# Pattern Groups & Callback Hooks Tests
# ===========================================================================
#
# These tests verify the pattern group and on-token callback functionality
# added in F04. The tests are organized into three categories:
#
# 1. **LexerContext unit tests** — push/pop/emit/suppress in isolation
# 2. **Group stack tests** — correct group transitions during tokenization
# 3. **Backward compatibility** — no groups + no callback = identical behavior
# ===========================================================================


def _token_type_name(t: Token) -> str:
    """Normalize a token type to its string name.

    Token types can be either a string (e.g., "TAG_NAME") or a
    TokenType enum member (e.g., TokenType.EQUALS). This helper
    normalizes both to a plain string for easy assertion.
    """
    if isinstance(t.type, str):
        return t.type
    return t.type.name


def _make_group_grammar() -> TokenGrammar:
    """Create a grammar with pattern groups for testing.

    This simulates a simplified XML-like grammar:
    - Default group: TEXT and OPEN_TAG
    - tag group: TAG_NAME, EQUALS, TAG_CLOSE

    The grammar uses skip patterns for whitespace and no keywords.
    """
    source = """\
escapes: none

skip:
  WS = /[ \\t\\r\\n]+/

TEXT      = /[^<]+/
OPEN_TAG  = "<"

group tag:
  TAG_NAME  = /[a-zA-Z_][a-zA-Z0-9_]*/
  EQUALS    = "="
  VALUE     = /"[^"]*"/
  TAG_CLOSE = ">"
"""
    return parse_token_grammar(source)


class TestLexerContext:
    """Unit tests for the LexerContext API.

    These tests verify that each LexerContext method correctly records
    actions without immediately mutating lexer state. The actions are
    applied by the tokenizer's main loop after the callback returns.
    """

    def test_push_group_records_action(self) -> None:
        """push_group() records a push action."""
        grammar = _make_group_grammar()
        lexer = GrammarLexer("x", grammar)
        ctx = LexerContext(lexer, "x", 1)
        ctx.push_group("tag")
        assert ctx._group_actions == [("push", "tag")]

    def test_push_unknown_group_raises(self) -> None:
        """push_group() with unknown name raises ValueError."""
        grammar = _make_group_grammar()
        lexer = GrammarLexer("x", grammar)
        ctx = LexerContext(lexer, "x", 1)
        with pytest.raises(ValueError, match="Unknown pattern group"):
            ctx.push_group("nonexistent")

    def test_pop_group_records_action(self) -> None:
        """pop_group() records a pop action."""
        grammar = _make_group_grammar()
        lexer = GrammarLexer("x", grammar)
        ctx = LexerContext(lexer, "x", 1)
        ctx.pop_group()
        assert ctx._group_actions == [("pop", "")]

    def test_active_group_reads_stack(self) -> None:
        """active_group() returns the top of the lexer's group stack."""
        grammar = _make_group_grammar()
        lexer = GrammarLexer("x", grammar)
        ctx = LexerContext(lexer, "x", 1)
        assert ctx.active_group() == "default"

    def test_group_stack_depth(self) -> None:
        """group_stack_depth() returns the length of the group stack."""
        grammar = _make_group_grammar()
        lexer = GrammarLexer("x", grammar)
        ctx = LexerContext(lexer, "x", 1)
        assert ctx.group_stack_depth() == 1

    def test_emit_appends_token(self) -> None:
        """emit() appends a synthetic token to the emitted list."""
        grammar = _make_group_grammar()
        lexer = GrammarLexer("x", grammar)
        ctx = LexerContext(lexer, "x", 1)
        synthetic = Token(type="SYNTHETIC", value="!", line=1, column=1)
        ctx.emit(synthetic)
        assert ctx._emitted == [synthetic]

    def test_suppress_sets_flag(self) -> None:
        """suppress() sets the suppressed flag."""
        grammar = _make_group_grammar()
        lexer = GrammarLexer("x", grammar)
        ctx = LexerContext(lexer, "x", 1)
        assert ctx._suppressed is False
        ctx.suppress()
        assert ctx._suppressed is True

    def test_peek_reads_source(self) -> None:
        """peek() reads characters from the source after the token."""
        grammar = _make_group_grammar()
        lexer = GrammarLexer("hello", grammar)
        # Suppose token ended at position 3 (consumed "hel")
        ctx = LexerContext(lexer, "hello", 3)
        assert ctx.peek(1) == "l"
        assert ctx.peek(2) == "o"
        assert ctx.peek(3) == ""  # past EOF

    def test_peek_str_reads_source(self) -> None:
        """peek_str() reads a substring from the source after the token."""
        grammar = _make_group_grammar()
        lexer = GrammarLexer("hello world", grammar)
        ctx = LexerContext(lexer, "hello world", 5)
        assert ctx.peek_str(6) == " world"

    def test_set_skip_enabled(self) -> None:
        """set_skip_enabled() records the new skip state."""
        grammar = _make_group_grammar()
        lexer = GrammarLexer("x", grammar)
        ctx = LexerContext(lexer, "x", 1)
        assert ctx._skip_enabled is None  # no change by default
        ctx.set_skip_enabled(False)
        assert ctx._skip_enabled is False

    def test_multiple_pushes(self) -> None:
        """Multiple push_group() calls are recorded in order."""
        grammar = _make_group_grammar()
        lexer = GrammarLexer("x", grammar)
        ctx = LexerContext(lexer, "x", 1)
        ctx.push_group("tag")
        ctx.push_group("tag")
        assert ctx._group_actions == [("push", "tag"), ("push", "tag")]


class TestPatternGroupTokenization:
    """Tests for pattern group switching during tokenization.

    These tests verify that the lexer correctly switches between
    pattern groups based on callback actions, producing the right
    tokens in the right order.
    """

    def test_no_callback_uses_default_group(self) -> None:
        """Without a callback, only default group patterns are used."""
        grammar = _make_group_grammar()
        tokens = GrammarLexer("hello", grammar).tokenize()
        # TEXT pattern matches in default group
        assert tokens[0].type == "TEXT"
        assert tokens[0].value == "hello"

    def test_callback_push_pop_group(self) -> None:
        """Callback can push/pop groups to switch pattern sets.

        Simulates: <div> where < triggers push("tag"), > triggers pop().
        """
        grammar = _make_group_grammar()

        def on_token(token: Token, ctx: LexerContext) -> None:
            if token.type == "OPEN_TAG":
                ctx.push_group("tag")
            elif token.type == "TAG_CLOSE":
                ctx.pop_group()

        lexer = GrammarLexer('<div>hello', grammar)
        lexer.set_on_token(on_token)
        tokens = lexer.tokenize()

        types = [
            (_token_type_name(t), t.value)
            for t in tokens if _token_type_name(t) != "EOF"
        ]
        assert types == [
            ("OPEN_TAG", "<"),
            ("TAG_NAME", "div"),
            ("TAG_CLOSE", ">"),
            ("TEXT", "hello"),
        ]

    def test_callback_with_attributes(self) -> None:
        """Callback handles tag with attributes.

        Simulates: <div class="main"> where the tag group lexes
        TAG_NAME, EQUALS, and VALUE tokens.
        """
        grammar = _make_group_grammar()

        def on_token(token: Token, ctx: LexerContext) -> None:
            if token.type == "OPEN_TAG":
                ctx.push_group("tag")
            elif token.type == "TAG_CLOSE":
                ctx.pop_group()

        lexer = GrammarLexer('<div class="main">', grammar)
        lexer.set_on_token(on_token)
        tokens = lexer.tokenize()

        types = [
            (_token_type_name(t), t.value)
            for t in tokens if _token_type_name(t) != "EOF"
        ]
        assert types == [
            ("OPEN_TAG", "<"),
            ("TAG_NAME", "div"),
            ("TAG_NAME", "class"),
            ("EQUALS", "="),
            ("VALUE", '"main"'),
            ("TAG_CLOSE", ">"),
        ]

    def test_nested_tags(self) -> None:
        """Group stack handles nested structures.

        Simulates: <a>text<b>inner</b></a> with push/pop on < and >.
        """
        # Need a grammar with CLOSE_TAG_START for </
        source = """\
escapes: none

skip:
  WS = /[ \\t\\r\\n]+/

TEXT             = /[^<]+/
CLOSE_TAG_START  = "</"
OPEN_TAG         = "<"

group tag:
  TAG_NAME  = /[a-zA-Z_][a-zA-Z0-9_]*/
  TAG_CLOSE = ">"
  SLASH     = "/"
"""
        grammar = parse_token_grammar(source)

        def on_token(token: Token, ctx: LexerContext) -> None:
            if token.type in ("OPEN_TAG", "CLOSE_TAG_START"):
                ctx.push_group("tag")
            elif token.type == "TAG_CLOSE":
                ctx.pop_group()

        lexer = GrammarLexer("<a>text<b>inner</b></a>", grammar)
        lexer.set_on_token(on_token)
        tokens = lexer.tokenize()

        types = [
            (_token_type_name(t), t.value)
            for t in tokens if _token_type_name(t) != "EOF"
        ]
        assert types == [
            ("OPEN_TAG", "<"),
            ("TAG_NAME", "a"),
            ("TAG_CLOSE", ">"),
            ("TEXT", "text"),
            ("OPEN_TAG", "<"),
            ("TAG_NAME", "b"),
            ("TAG_CLOSE", ">"),
            ("TEXT", "inner"),
            ("CLOSE_TAG_START", "</"),
            ("TAG_NAME", "b"),
            ("TAG_CLOSE", ">"),
            ("CLOSE_TAG_START", "</"),
            ("TAG_NAME", "a"),
            ("TAG_CLOSE", ">"),
        ]

    def test_suppress_token(self) -> None:
        """Callback can suppress tokens (remove from output)."""
        grammar = _make_group_grammar()

        def on_token(token: Token, ctx: LexerContext) -> None:
            # Suppress the OPEN_TAG token
            if token.type == "OPEN_TAG":
                ctx.suppress()

        lexer = GrammarLexer("<hello", grammar)
        lexer.set_on_token(on_token)
        tokens = lexer.tokenize()

        types = [_token_type_name(t) for t in tokens if _token_type_name(t) != "EOF"]
        # OPEN_TAG was suppressed, only TEXT remains
        assert types == ["TEXT"]

    def test_emit_synthetic_token(self) -> None:
        """Callback can emit synthetic tokens after the current one."""
        grammar = _make_group_grammar()

        def on_token(token: Token, ctx: LexerContext) -> None:
            if token.type == "OPEN_TAG":
                ctx.emit(Token(
                    type="MARKER",
                    value="[start]",
                    line=token.line,
                    column=token.column,
                ))

        lexer = GrammarLexer("<hello", grammar)
        lexer.set_on_token(on_token)
        tokens = lexer.tokenize()

        types = [
            (_token_type_name(t), t.value)
            for t in tokens if _token_type_name(t) != "EOF"
        ]
        assert types == [
            ("OPEN_TAG", "<"),
            ("MARKER", "[start]"),
            ("TEXT", "hello"),
        ]

    def test_suppress_and_emit(self) -> None:
        """Suppress + emit = token replacement.

        The current token is swallowed, but emitted tokens still output.
        This enables token rewriting (e.g., replacing OPEN_TAG with a
        different token type).
        """
        grammar = _make_group_grammar()

        def on_token(token: Token, ctx: LexerContext) -> None:
            if token.type == "OPEN_TAG":
                ctx.suppress()
                ctx.emit(Token(
                    type="REPLACED",
                    value="<",
                    line=token.line,
                    column=token.column,
                ))

        lexer = GrammarLexer("<hello", grammar)
        lexer.set_on_token(on_token)
        tokens = lexer.tokenize()

        types = [
            (_token_type_name(t), t.value)
            for t in tokens if _token_type_name(t) != "EOF"
        ]
        assert types == [
            ("REPLACED", "<"),
            ("TEXT", "hello"),
        ]

    def test_pop_at_bottom_is_noop(self) -> None:
        """Popping when only default remains is a no-op (no crash)."""
        grammar = _make_group_grammar()

        def on_token(token: Token, ctx: LexerContext) -> None:
            ctx.pop_group()  # Should be safe even at the bottom

        lexer = GrammarLexer("hello", grammar)
        lexer.set_on_token(on_token)
        tokens = lexer.tokenize()

        # Should still produce TEXT token without crashing
        assert tokens[0].type == "TEXT"

    def test_set_skip_enabled_false(self) -> None:
        """Callback can disable skip patterns for significant whitespace.

        When skip is disabled, whitespace that would normally be consumed
        silently instead causes a match failure (unless the active group
        has a pattern that matches whitespace).
        """
        # Grammar with a group that captures whitespace as a token
        source = """\
escapes: none

skip:
  WS = /[ \\t]+/

TEXT      = /[^<]+/
START     = "<!"

group raw:
  RAW_TEXT = /[^>]+/
  END      = ">"
"""
        grammar = parse_token_grammar(source)

        def on_token(token: Token, ctx: LexerContext) -> None:
            if token.type == "START":
                ctx.push_group("raw")
                ctx.set_skip_enabled(False)
            elif token.type == "END":
                ctx.pop_group()
                ctx.set_skip_enabled(True)

        # The space in "hello world" should be preserved (not skipped)
        # because skip is disabled while in the raw group.
        lexer = GrammarLexer("<! hello world >after", grammar)
        lexer.set_on_token(on_token)
        tokens = lexer.tokenize()

        types = [
            (_token_type_name(t), t.value)
            for t in tokens if _token_type_name(t) != "EOF"
        ]
        assert types == [
            ("START", "<!"),
            ("RAW_TEXT", " hello world "),
            ("END", ">"),
            ("TEXT", "after"),
        ]

    def test_no_groups_backward_compat(self) -> None:
        """A grammar with no groups behaves identically to before.

        This verifies backward compatibility: no groups + no callback
        = same behavior as the original GrammarLexer.
        """
        source = """\
NAME   = /[a-zA-Z_][a-zA-Z0-9_]*/
NUMBER = /[0-9]+/
PLUS   = "+"
"""
        grammar = parse_token_grammar(source)
        tokens = GrammarLexer("x + 1", grammar).tokenize()

        types = [(_token_type_name(t), t.value) for t in tokens
                 if _token_type_name(t) not in ("NEWLINE", "EOF")]
        assert types == [
            ("NAME", "x"),
            ("PLUS", "+"),
            ("NUMBER", "1"),
        ]

    def test_clear_callback(self) -> None:
        """Passing None to set_on_token clears the callback."""
        grammar = _make_group_grammar()
        called = []

        def on_token(token: Token, ctx: LexerContext) -> None:
            called.append(token.type)

        lexer = GrammarLexer("hello", grammar)
        lexer.set_on_token(on_token)
        lexer.set_on_token(None)
        lexer.tokenize()

        assert called == []

    def test_group_stack_resets_between_calls(self) -> None:
        """The group stack resets when tokenize() is called again.

        This ensures the lexer can be reused for multiple tokenize()
        calls without group state leaking between them.
        """
        grammar = _make_group_grammar()

        def on_token(token: Token, ctx: LexerContext) -> None:
            if token.type == "OPEN_TAG":
                ctx.push_group("tag")

        lexer = GrammarLexer("<div", grammar)
        lexer.set_on_token(on_token)

        # First call: pushes "tag" group
        tokens1 = lexer.tokenize()
        assert any(t.type == "TAG_NAME" for t in tokens1)

        # Second call: should start fresh from "default"
        lexer._source = "<div"
        lexer._pos = 0
        lexer._line = 1
        lexer._column = 1
        tokens2 = lexer.tokenize()
        assert any(t.type == "TAG_NAME" for t in tokens2)

    def test_multiple_push_pop_sequence(self) -> None:
        """Multiple push/pop in one callback are applied in order."""
        grammar = _make_group_grammar()

        def on_token(token: Token, ctx: LexerContext) -> None:
            if token.type == "OPEN_TAG":
                # Push tag, then immediately push tag again (stacking)
                ctx.push_group("tag")
                ctx.push_group("tag")

        lexer = GrammarLexer("<div", grammar)
        lexer.set_on_token(on_token)
        lexer.tokenize()

        # After OPEN_TAG, stack should be ["default", "tag", "tag"]
        # The test verifies no crash occurs with multiple pushes.
        # We re-tokenize to confirm the stack handles double-push.
        lexer._source = "<div"
        lexer._pos = 0
        lexer._line = 1
        lexer._column = 1
        tokens = lexer.tokenize()
        assert any(_token_type_name(t) == "TAG_NAME" for t in tokens)


# ---------------------------------------------------------------------------
# Case-insensitive keyword support
# ---------------------------------------------------------------------------
#
# When a grammar is created with ``# @case_insensitive true``, keyword
# matching must ignore case.  The rules are:
#
# 1. Keywords in the .tokens ``keywords:`` section are stored uppercase.
# 2. When a NAME token is matched, ``value.upper()`` is compared against the
#    keyword set.
# 3. If the value IS a keyword, the emitted token has ``value = value.upper()``
#    (i.e., normalised to uppercase) regardless of how it appeared in the
#    source.
# 4. Without ``# @case_insensitive true`` the behaviour is unchanged —
#    keyword matching is exact and values are emitted as-is.
#
# We build a minimal grammar inline (no .tokens file needed) to keep the
# tests self-contained and fast.


def _make_case_insensitive_grammar() -> TokenGrammar:
    """Build a minimal grammar that enables case-insensitive keyword matching.

    Grammar source (as it would appear in a .tokens file)::

        # @case_insensitive true
        NAME   = /[a-zA-Z_][a-zA-Z0-9_]*/
        WS     = /[ \\t]+/

        skip:
          WS

        keywords:
          select
          from
    """
    grammar_source = (
        "# @case_insensitive true\n"
        "NAME   = /[a-zA-Z_][a-zA-Z0-9_]*/\n"
        "\n"
        "skip:\n"
        "  WS = /[ \\t]+/\n"
        "\n"
        "keywords:\n"
        "  select\n"
        "  from\n"
    )
    return parse_token_grammar(grammar_source)


def _make_case_sensitive_grammar() -> TokenGrammar:
    """Build a minimal grammar WITHOUT case-insensitive mode.

    The keyword ``select`` is stored exactly as-is.  Only the exact string
    ``"select"`` should be reclassified as KEYWORD — ``"SELECT"`` and
    ``"Select"`` remain NAME tokens.
    """
    grammar_source = (
        "NAME   = /[a-zA-Z_][a-zA-Z0-9_]*/\n"
        "\n"
        "skip:\n"
        "  WS = /[ \\t]+/\n"
        "\n"
        "keywords:\n"
        "  select\n"
    )
    return parse_token_grammar(grammar_source)


class TestCaseInsensitiveKeywords:
    """Tests for ``# @case_insensitive true`` keyword handling in GrammarLexer.

    All five variants required by the specification are covered:
      1. Lowercase input → uppercase KEYWORD
      2. Uppercase input → uppercase KEYWORD
      3. Mixed-case input → uppercase KEYWORD
      4. Case-sensitive grammar — exact-match only
      5. Non-keyword identifier in case-insensitive grammar → NAME, original case
    """

    def test_lowercase_input_emits_uppercase_keyword(self) -> None:
        """``"select"`` → KEYWORD with value ``"SELECT"`` (case-insensitive grammar).

        Even though the source contains a lowercase ``select``, the lexer
        must reclassify it as a keyword AND normalise the value to uppercase
        because ``case_insensitive`` is True.
        """
        grammar = _make_case_insensitive_grammar()
        tokens = GrammarLexer("select", grammar).tokenize()

        # Filter out the EOF sentinel.
        non_eof = [t for t in tokens if _token_type_name(t) != "EOF"]
        assert len(non_eof) == 1

        tok = non_eof[0]
        assert _token_type_name(tok) == "KEYWORD", (
            f"Expected KEYWORD, got {_token_type_name(tok)!r}"
        )
        assert tok.value == "SELECT", (
            f"Expected value 'SELECT', got {tok.value!r}"
        )

    def test_uppercase_input_emits_uppercase_keyword(self) -> None:
        """``"SELECT"`` → KEYWORD with value ``"SELECT"`` (case-insensitive grammar).

        The source already uses uppercase; the lexer should still match it
        as a keyword and emit the normalised uppercase value.
        """
        grammar = _make_case_insensitive_grammar()
        tokens = GrammarLexer("SELECT", grammar).tokenize()

        non_eof = [t for t in tokens if _token_type_name(t) != "EOF"]
        assert len(non_eof) == 1

        tok = non_eof[0]
        assert _token_type_name(tok) == "KEYWORD"
        assert tok.value == "SELECT"

    def test_mixed_case_input_emits_uppercase_keyword(self) -> None:
        """``"Select"`` → KEYWORD with value ``"SELECT"`` (case-insensitive grammar).

        Mixed-case input that happens to match a keyword (case-insensitively)
        must also be normalised to uppercase.
        """
        grammar = _make_case_insensitive_grammar()
        tokens = GrammarLexer("Select", grammar).tokenize()

        non_eof = [t for t in tokens if _token_type_name(t) != "EOF"]
        assert len(non_eof) == 1

        tok = non_eof[0]
        assert _token_type_name(tok) == "KEYWORD"
        assert tok.value == "SELECT"

    def test_case_sensitive_grammar_exact_match_only(self) -> None:
        """``"select"`` → KEYWORD in case-sensitive grammar; other casings → NAME.

        Without ``# @case_insensitive true``, keyword matching is exact.
        Only the string ``"select"`` is in the keyword set.  ``"SELECT"``
        and ``"Select"`` must remain NAME tokens and their values must be
        preserved verbatim.
        """
        grammar = _make_case_sensitive_grammar()

        # Exact match — should be KEYWORD with original value.
        tokens_lower = GrammarLexer("select", grammar).tokenize()
        non_eof_lower = [t for t in tokens_lower if _token_type_name(t) != "EOF"]
        assert len(non_eof_lower) == 1
        assert _token_type_name(non_eof_lower[0]) == "KEYWORD"
        assert non_eof_lower[0].value == "select"

        # Uppercase — NOT a keyword; value preserved verbatim.
        tokens_upper = GrammarLexer("SELECT", grammar).tokenize()
        non_eof_upper = [t for t in tokens_upper if _token_type_name(t) != "EOF"]
        assert len(non_eof_upper) == 1
        assert _token_type_name(non_eof_upper[0]) == "NAME", (
            "Case-sensitive grammar must not match 'SELECT' as a keyword"
        )
        assert non_eof_upper[0].value == "SELECT"

        # Mixed case — NOT a keyword; value preserved verbatim.
        tokens_mixed = GrammarLexer("Select", grammar).tokenize()
        non_eof_mixed = [t for t in tokens_mixed if _token_type_name(t) != "EOF"]
        assert len(non_eof_mixed) == 1
        assert _token_type_name(non_eof_mixed[0]) == "NAME"
        assert non_eof_mixed[0].value == "Select"

    def test_non_keyword_identifier_preserves_original_case(self) -> None:
        """Non-keyword identifiers keep their original casing in case-insensitive mode.

        The grammar has ``select`` and ``from`` as keywords.  The identifier
        ``myTable`` is not a keyword; it should be emitted as a NAME token
        with its original mixed-case value untouched.
        """
        grammar = _make_case_insensitive_grammar()
        tokens = GrammarLexer("myTable", grammar).tokenize()

        non_eof = [t for t in tokens if _token_type_name(t) != "EOF"]
        assert len(non_eof) == 1

        tok = non_eof[0]
        assert _token_type_name(tok) == "NAME", (
            f"Non-keyword identifier should remain NAME, got {_token_type_name(tok)!r}"
        )
        assert tok.value == "myTable", (
            f"Non-keyword value must preserve original case, got {tok.value!r}"
        )

    def test_string_literal_preserves_original_case(self) -> None:
        """STRING values keep their original casing in case-insensitive mode."""
        grammar = parse_token_grammar(
            "# @case_insensitive true\n"
            "STRING = /'[^']*'/\n"
        )
        tokens = GrammarLexer("'Ada'", grammar).tokenize()

        non_eof = [t for t in tokens if _token_type_name(t) != "EOF"]
        assert len(non_eof) == 1

        tok = non_eof[0]
        assert _token_type_name(tok) == "STRING"
        assert tok.value == "Ada"
