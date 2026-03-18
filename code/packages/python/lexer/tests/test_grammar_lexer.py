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

from grammar_tools import TokenGrammar, TokenDefinition, parse_token_grammar

from lexer.grammar_lexer import GrammarLexer
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
        """Token names not in TokenType should fall back to NAME."""
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
        # "IDENTIFIER" is not in TokenType, so it falls back to NAME.
        assert tokens[0].type == TokenType.NAME
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
