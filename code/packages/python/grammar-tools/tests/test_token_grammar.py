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
    TokenDefinition,
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

    def test_missing_token_name(self) -> None:
        """A line with = but no name before it raises an error."""
        with pytest.raises(TokenGrammarError, match="Missing token name"):
            parse_token_grammar(' = /foo/')

    def test_invalid_token_name(self) -> None:
        """A token name with invalid characters raises an error."""
        with pytest.raises(TokenGrammarError, match="Invalid token name"):
            parse_token_grammar('123BAD = /foo/')

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


# ---------------------------------------------------------------------------
# Extended format: mode directive
# ---------------------------------------------------------------------------


class TestParseMode:
    """Test the mode: directive."""

    def test_mode_indentation(self) -> None:
        source = """mode: indentation
NAME = /[a-z]+/
"""
        grammar = parse_token_grammar(source)
        assert grammar.mode == "indentation"

    def test_mode_at_end(self) -> None:
        source = """NAME = /[a-z]+/
mode: indentation
"""
        grammar = parse_token_grammar(source)
        assert grammar.mode == "indentation"

    def test_no_mode(self) -> None:
        source = 'NAME = /[a-z]+/'
        grammar = parse_token_grammar(source)
        assert grammar.mode is None

    def test_mode_empty_value_raises(self) -> None:
        with pytest.raises(TokenGrammarError, match="Missing value"):
            parse_token_grammar("mode:")

    def test_mode_custom_value(self) -> None:
        grammar = parse_token_grammar("mode: custom_mode\nNAME = /[a-z]+/")
        assert grammar.mode == "custom_mode"


# ---------------------------------------------------------------------------
# Extended format: skip section
# ---------------------------------------------------------------------------


class TestParseSkip:
    """Test the skip: section for patterns that produce no tokens."""

    def test_skip_regex(self) -> None:
        source = """skip:
  COMMENT = /#[^\\n]*/
  WHITESPACE = /[ \\t]+/

NAME = /[a-z]+/
"""
        grammar = parse_token_grammar(source)
        assert len(grammar.skip_definitions) == 2
        assert grammar.skip_definitions[0].name == "COMMENT"
        assert grammar.skip_definitions[0].is_regex is True
        assert grammar.skip_definitions[1].name == "WHITESPACE"

    def test_skip_section_exits_on_non_indented(self) -> None:
        source = """skip:
  COMMENT = /#[^\\n]*/
NAME = /[a-z]+/
"""
        grammar = parse_token_grammar(source)
        assert len(grammar.skip_definitions) == 1
        assert len(grammar.definitions) == 1

    def test_skip_empty_section(self) -> None:
        source = """skip:
NAME = /[a-z]+/
"""
        grammar = parse_token_grammar(source)
        assert len(grammar.skip_definitions) == 0
        assert len(grammar.definitions) == 1

    def test_skip_bad_definition_raises(self) -> None:
        with pytest.raises(TokenGrammarError, match="Expected skip pattern"):
            parse_token_grammar("skip:\n  NOT_A_DEFINITION")

    def test_skip_incomplete_definition_raises(self) -> None:
        """A skip pattern with = but empty name or pattern raises an error."""
        with pytest.raises(TokenGrammarError, match="Incomplete skip"):
            parse_token_grammar("skip:\n  = /foo/")


# ---------------------------------------------------------------------------
# Extended format: -> TYPE alias
# ---------------------------------------------------------------------------


class TestParseAlias:
    """Test the -> TYPE alias suffix on token definitions."""

    def test_regex_with_alias(self) -> None:
        source = 'STRING_DQ = /"([^"\\\\]|\\\\.)*"/ -> STRING'
        grammar = parse_token_grammar(source)
        defn = grammar.definitions[0]
        assert defn.name == "STRING_DQ"
        assert defn.alias == "STRING"
        assert defn.is_regex is True

    def test_literal_with_alias(self) -> None:
        source = 'DOUBLE_STAR = "**" -> STAR_STAR'
        grammar = parse_token_grammar(source)
        defn = grammar.definitions[0]
        assert defn.name == "DOUBLE_STAR"
        assert defn.alias == "STAR_STAR"
        assert defn.pattern == "**"

    def test_no_alias(self) -> None:
        source = 'PLUS = "+"'
        grammar = parse_token_grammar(source)
        assert grammar.definitions[0].alias is None

    def test_alias_missing_name_raises(self) -> None:
        with pytest.raises(TokenGrammarError, match="Missing alias"):
            parse_token_grammar('FOO = /bar/ ->')

    def test_literal_alias_missing_name_raises(self) -> None:
        """Missing alias after -> on a literal pattern raises an error."""
        with pytest.raises(TokenGrammarError, match="Missing alias"):
            parse_token_grammar('FOO = "bar" ->')

    def test_unexpected_text_after_regex(self) -> None:
        """Junk text after a regex closing / raises an error."""
        with pytest.raises(TokenGrammarError, match="Unexpected text"):
            parse_token_grammar('FOO = /bar/ baz')

    def test_unexpected_text_after_literal(self) -> None:
        """Junk text after a literal closing " raises an error."""
        with pytest.raises(TokenGrammarError, match="Unexpected text"):
            parse_token_grammar('FOO = "bar" baz')

    def test_unclosed_literal(self) -> None:
        """A literal missing its closing quote raises an error."""
        with pytest.raises(TokenGrammarError, match="Unclosed literal"):
            parse_token_grammar('FOO = "bar')

    def test_unclosed_regex(self) -> None:
        """A regex with only the opening / raises an error."""
        with pytest.raises(TokenGrammarError, match="Unclosed regex"):
            parse_token_grammar("FOO = /bar")

    def test_multiple_aliases_same_target(self) -> None:
        source = """STRING_DQ = /"[^"]*"/ -> STRING
STRING_SQ = /'[^']*'/ -> STRING
"""
        grammar = parse_token_grammar(source)
        assert grammar.definitions[0].alias == "STRING"
        assert grammar.definitions[1].alias == "STRING"

    def test_token_names_includes_aliases(self) -> None:
        source = 'STRING_DQ = /"[^"]*"/ -> STRING'
        grammar = parse_token_grammar(source)
        names = grammar.token_names()
        assert "STRING_DQ" in names
        assert "STRING" in names

    def test_effective_token_names_uses_alias(self) -> None:
        source = """STRING_DQ = /"[^"]*"/ -> STRING
PLUS = "+"
"""
        grammar = parse_token_grammar(source)
        effective = grammar.effective_token_names()
        assert effective == {"STRING", "PLUS"}


# ---------------------------------------------------------------------------
# Extended format: reserved section
# ---------------------------------------------------------------------------


class TestParseReserved:
    """Test the reserved: section for forbidden identifiers."""

    def test_reserved_keywords(self) -> None:
        source = """NAME = /[a-z]+/

reserved:
  class
  import
  while
"""
        grammar = parse_token_grammar(source)
        assert grammar.reserved_keywords == ["class", "import", "while"]

    def test_reserved_empty(self) -> None:
        source = """reserved:
NAME = /[a-z]+/
"""
        grammar = parse_token_grammar(source)
        assert grammar.reserved_keywords == []

    def test_keywords_and_reserved_coexist(self) -> None:
        source = """NAME = /[a-z]+/
keywords:
  if
  else
reserved:
  class
  while
"""
        grammar = parse_token_grammar(source)
        assert grammar.keywords == ["if", "else"]
        assert grammar.reserved_keywords == ["class", "while"]


# ---------------------------------------------------------------------------
# Extended format: validation
# ---------------------------------------------------------------------------


class TestExtendedValidation:
    """Test validation of extended features."""

    def test_valid_mode_no_issues(self) -> None:
        source = "mode: indentation\nNAME = /[a-z]+/"
        grammar = parse_token_grammar(source)
        issues = validate_token_grammar(grammar)
        assert not any("mode" in i.lower() for i in issues)

    def test_invalid_mode(self) -> None:
        source = "mode: foobar\nNAME = /[a-z]+/"
        grammar = parse_token_grammar(source)
        issues = validate_token_grammar(grammar)
        assert any("Unknown lexer mode" in i for i in issues)

    def test_alias_non_uppercase_flagged(self) -> None:
        source = 'FOO = /bar/ -> string'
        grammar = parse_token_grammar(source)
        issues = validate_token_grammar(grammar)
        assert any("Alias" in i and "UPPER_CASE" in i for i in issues)

    def test_skip_invalid_regex_flagged(self) -> None:
        source = "skip:\n  BAD = /[invalid/"
        grammar = parse_token_grammar(source)
        issues = validate_token_grammar(grammar)
        assert any("Invalid regex" in i for i in issues)

    def test_empty_pattern_flagged(self) -> None:
        """An empty pattern (constructed programmatically) is flagged."""
        grammar = TokenGrammar(definitions=[
            TokenDefinition(
                name="EMPTY", pattern="", is_regex=False, line_number=1,
            )
        ])
        issues = validate_token_grammar(grammar)
        assert any("Empty pattern" in i for i in issues)

    def test_duplicate_skip_names_flagged(self) -> None:
        """Duplicate skip pattern names are flagged."""
        grammar = TokenGrammar(skip_definitions=[
            TokenDefinition(name="WS", pattern="[ \\t]+", is_regex=True, line_number=1),
            TokenDefinition(name="WS", pattern="\\s+", is_regex=True, line_number=2),
        ])
        issues = validate_token_grammar(grammar)
        assert any("Duplicate" in i for i in issues)


# ---------------------------------------------------------------------------
# Full Starlark example
# ---------------------------------------------------------------------------


class TestStarlarkTokens:
    """Test parsing the actual starlark.tokens file."""

    def test_parse_starlark_tokens(self) -> None:
        """The full starlark.tokens file parses without errors."""
        import os
        # tests/ → grammar-tools/ → python/ → packages/ → code/
        code_dir = os.path.dirname(os.path.dirname(os.path.dirname(
            os.path.dirname(os.path.dirname(
                os.path.abspath(__file__))))))
        tokens_path = os.path.join(code_dir, "grammars", "starlark.tokens")
        if not os.path.exists(tokens_path):
            pytest.skip("starlark.tokens not found")
        with open(tokens_path) as f:
            source = f.read()
        grammar = parse_token_grammar(source)

        assert grammar.mode == "indentation"
        assert len(grammar.definitions) > 40
        assert len(grammar.keywords) == 18
        assert len(grammar.reserved_keywords) == 18
        assert len(grammar.skip_definitions) == 2
        assert sum(1 for d in grammar.definitions if d.alias) > 5

        issues = validate_token_grammar(grammar)
        assert issues == []
