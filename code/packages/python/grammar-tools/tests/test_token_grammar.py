"""
Tests for the .tokens file parser and validator.

These tests verify that parse_token_grammar correctly reads the declarative
token definition format, and that validate_token_grammar catches common
mistakes. Each test focuses on one aspect of the format: regex patterns,
literal patterns, comments, keywords, error handling, and validation.
"""

from __future__ import annotations

import pytest

from grammar_tools.token_grammar import (
    TokenGrammar,
    TokenGrammarError,
    parse_token_grammar,
    validate_token_grammar,
)


# ---------------------------------------------------------------------------
# Parsing: happy paths
# ---------------------------------------------------------------------------


class TestParseMinimal:
    """Test parsing the simplest possible .tokens files."""

    def test_single_regex_token(self) -> None:
        """A file with one regex-based token definition."""
        source = 'NUMBER = /[0-9]+/'
        grammar = parse_token_grammar(source)
        assert len(grammar.definitions) == 1
        defn = grammar.definitions[0]
        assert defn.name == "NUMBER"
        assert defn.pattern == "[0-9]+"
        assert defn.is_regex is True
        assert defn.line_number == 1

    def test_single_literal_token(self) -> None:
        """A file with one literal-string token definition."""
        source = 'PLUS = "+"'
        grammar = parse_token_grammar(source)
        assert len(grammar.definitions) == 1
        defn = grammar.definitions[0]
        assert defn.name == "PLUS"
        assert defn.pattern == "+"
        assert defn.is_regex is False

    def test_multiple_tokens(self) -> None:
        """Multiple token definitions are parsed in order."""
        source = """NUMBER = /[0-9]+/
PLUS   = "+"
MINUS  = "-"
"""
        grammar = parse_token_grammar(source)
        assert len(grammar.definitions) == 3
        assert [d.name for d in grammar.definitions] == [
            "NUMBER",
            "PLUS",
            "MINUS",
        ]


class TestParseKeywords:
    """Test the keywords: section."""

    def test_keywords_section(self) -> None:
        """Keywords are parsed from indented lines after 'keywords:'."""
        source = """NAME = /[a-zA-Z_][a-zA-Z0-9_]*/

keywords:
  if
  else
  while
"""
        grammar = parse_token_grammar(source)
        assert grammar.keywords == ["if", "else", "while"]

    def test_keywords_with_tabs(self) -> None:
        """Keywords indented with tabs should also work."""
        source = "NAME = /[a-z]+/\nkeywords:\n\tif\n\telse"
        grammar = parse_token_grammar(source)
        assert grammar.keywords == ["if", "else"]

    def test_no_keywords_section(self) -> None:
        """A file without keywords: section has an empty keywords list."""
        source = 'NUMBER = /[0-9]+/'
        grammar = parse_token_grammar(source)
        assert grammar.keywords == []


class TestParseCommentsAndBlanks:
    """Test that comments and blank lines are properly ignored."""

    def test_comments_ignored(self) -> None:
        """Lines starting with # are skipped."""
        source = """# This is a comment
NUMBER = /[0-9]+/
# Another comment
PLUS   = "+"
"""
        grammar = parse_token_grammar(source)
        assert len(grammar.definitions) == 2

    def test_blank_lines_ignored(self) -> None:
        """Empty and whitespace-only lines are skipped."""
        source = """
NUMBER = /[0-9]+/

PLUS   = "+"

"""
        grammar = parse_token_grammar(source)
        assert len(grammar.definitions) == 2

    def test_comments_in_keywords(self) -> None:
        """Comments inside the keywords section are skipped."""
        source = """NAME = /[a-z]+/
keywords:
  # this is a comment
  if
  else
"""
        grammar = parse_token_grammar(source)
        assert grammar.keywords == ["if", "else"]


class TestParseRegexVsLiteral:
    """Test that regex and literal patterns are distinguished correctly."""

    def test_regex_pattern(self) -> None:
        """Regex patterns are delimited by /slashes/."""
        grammar = parse_token_grammar('NAME = /[a-zA-Z_][a-zA-Z0-9_]*/')
        assert grammar.definitions[0].is_regex is True
        assert grammar.definitions[0].pattern == "[a-zA-Z_][a-zA-Z0-9_]*"

    def test_literal_pattern(self) -> None:
        """Literal patterns are delimited by "quotes"."""
        grammar = parse_token_grammar('EQUALS = "="')
        assert grammar.definitions[0].is_regex is False
        assert grammar.definitions[0].pattern == "="

    def test_literal_with_multiple_chars(self) -> None:
        """Multi-character literals work correctly."""
        grammar = parse_token_grammar('EQUALS_EQUALS = "=="')
        assert grammar.definitions[0].pattern == "=="


class TestTokenNames:
    """Test the token_names() method."""

    def test_returns_all_names(self) -> None:
        source = """NUMBER = /[0-9]+/
PLUS   = "+"
NAME   = /[a-z]+/
"""
        grammar = parse_token_grammar(source)
        assert grammar.token_names() == {"NUMBER", "PLUS", "NAME"}

    def test_empty_grammar(self) -> None:
        grammar = parse_token_grammar("")
        assert grammar.token_names() == set()


# ---------------------------------------------------------------------------
# Parsing: error cases
# ---------------------------------------------------------------------------


class TestParseErrors:
    """Test that malformed .tokens files produce clear errors."""

    def test_duplicate_token_name_parses_ok(self) -> None:
        """Duplicate names are not a parse error (caught by validator)."""
        source = """NUMBER = /[0-9]+/
NUMBER = /[0-9]+\\.?[0-9]*/
"""
        grammar = parse_token_grammar(source)
        assert len(grammar.definitions) == 2

    def test_missing_pattern(self) -> None:
        """A line with name and = but no pattern raises an error."""
        with pytest.raises(TokenGrammarError, match="Missing pattern"):
            parse_token_grammar("NUMBER =")

    def test_malformed_line_no_equals(self) -> None:
        """A line without = raises an error."""
        with pytest.raises(TokenGrammarError, match="Expected token definition"):
            parse_token_grammar("NUMBER /[0-9]+/")

    def test_invalid_pattern_delimiters(self) -> None:
        """A pattern that is neither /regex/ nor \"literal\" raises an error."""
        with pytest.raises(TokenGrammarError, match="must be /regex/ or"):
            parse_token_grammar("NUMBER = [0-9]+")

    def test_empty_regex_pattern(self) -> None:
        """An empty regex // raises an error."""
        with pytest.raises(TokenGrammarError, match="Empty regex"):
            parse_token_grammar("NUMBER = //")

    def test_empty_literal_pattern(self) -> None:
        """An empty literal \"\" raises an error."""
        with pytest.raises(TokenGrammarError, match="Empty literal"):
            parse_token_grammar('EMPTY = ""')

    def test_error_includes_line_number(self) -> None:
        """Error messages include the correct line number."""
        source = """# comment
NUMBER = /[0-9]+/
BADLINE
"""
        with pytest.raises(TokenGrammarError) as exc_info:
            parse_token_grammar(source)
        assert exc_info.value.line_number == 3


# ---------------------------------------------------------------------------
# Validation
# ---------------------------------------------------------------------------


class TestValidation:
    """Test the validate_token_grammar function."""

    def test_valid_grammar_no_issues(self) -> None:
        """A well-formed grammar produces no warnings."""
        source = """NUMBER = /[0-9]+/
PLUS   = "+"
NAME   = /[a-zA-Z_][a-zA-Z0-9_]*/
"""
        grammar = parse_token_grammar(source)
        issues = validate_token_grammar(grammar)
        assert issues == []

    def test_duplicate_names(self) -> None:
        """Duplicate token names are flagged."""
        source = """NUMBER = /[0-9]+/
NUMBER = /[0-9]+\\.?[0-9]*/
"""
        grammar = parse_token_grammar(source)
        issues = validate_token_grammar(grammar)
        assert any("Duplicate" in i for i in issues)

    def test_invalid_regex(self) -> None:
        """An invalid regex pattern is flagged."""
        source = 'BAD = /[invalid/'
        grammar = parse_token_grammar(source)
        issues = validate_token_grammar(grammar)
        assert any("Invalid regex" in i for i in issues)

    def test_non_uppercase_name(self) -> None:
        """Token names that aren't UPPER_CASE are flagged."""
        source = 'number = /[0-9]+/'
        grammar = parse_token_grammar(source)
        issues = validate_token_grammar(grammar)
        assert any("UPPER_CASE" in i for i in issues)

    def test_mixed_case_name(self) -> None:
        """Mixed case names like 'Number' are flagged."""
        source = 'Number = /[0-9]+/'
        grammar = parse_token_grammar(source)
        issues = validate_token_grammar(grammar)
        assert any("UPPER_CASE" in i for i in issues)


class TestFullExample:
    """Test parsing the complete example from the specification."""

    def test_full_tokens_file(self) -> None:
        source = """# Token definitions for a simple expression language

NAME        = /[a-zA-Z_][a-zA-Z0-9_]*/
NUMBER      = /[0-9]+/
STRING      = /"([^"\\\\]|\\\\.)*"/

EQUALS_EQUALS = "=="
EQUALS      = "="
PLUS        = "+"
MINUS       = "-"
STAR        = "*"
SLASH       = "/"
LPAREN      = "("
RPAREN      = ")"
COMMA       = ","
COLON       = ":"

# Keywords section
keywords:
  if
  else
  def
  return
  while
  for
  True
  False
  None
"""
        grammar = parse_token_grammar(source)
        assert len(grammar.definitions) == 13
        assert grammar.keywords == [
            "if", "else", "def", "return", "while", "for",
            "True", "False", "None",
        ]
        issues = validate_token_grammar(grammar)
        assert issues == []
