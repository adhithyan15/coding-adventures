"""
Tests for cross-validation between .tokens and .grammar files.

Cross-validation ensures that the two grammar files are consistent with
each other: every token referenced in the grammar is defined in the tokens
file, and unused tokens are reported as warnings.
"""

from __future__ import annotations

from grammar_tools.cross_validator import cross_validate
from grammar_tools.parser_grammar import parse_parser_grammar
from grammar_tools.token_grammar import parse_token_grammar


class TestCrossValidateHappy:
    """Test cases where the grammars are consistent."""

    def test_all_references_resolve(self) -> None:
        """When every token used in the grammar is defined, no errors."""
        tokens = parse_token_grammar("""
NUMBER = /[0-9]+/
PLUS   = "+"
NAME   = /[a-zA-Z]+/
LPAREN = "("
RPAREN = ")"
""")
        grammar = parse_parser_grammar("""
expression = term { PLUS term } ;
term       = NUMBER | NAME | LPAREN expression RPAREN ;
""")
        issues = cross_validate(tokens, grammar)
        # No errors — all references resolve.
        errors = [i for i in issues if i.startswith("Error")]
        assert errors == []

    def test_no_unused_warnings_when_all_used(self) -> None:
        """When every token is used, no unused warnings."""
        tokens = parse_token_grammar("""
NUMBER = /[0-9]+/
PLUS   = "+"
""")
        grammar = parse_parser_grammar("""
expression = NUMBER { PLUS NUMBER } ;
""")
        issues = cross_validate(tokens, grammar)
        assert issues == []


class TestCrossValidateErrors:
    """Test cases where the grammars are inconsistent."""

    def test_missing_token_reference(self) -> None:
        """A token referenced in the grammar but not in .tokens is an error."""
        tokens = parse_token_grammar("""
NUMBER = /[0-9]+/
""")
        grammar = parse_parser_grammar("""
expression = NUMBER PLUS NUMBER ;
""")
        issues = cross_validate(tokens, grammar)
        errors = [i for i in issues if i.startswith("Error")]
        assert len(errors) == 1
        assert "PLUS" in errors[0]

    def test_unused_token_warning(self) -> None:
        """A token defined in .tokens but not used in the grammar is a warning."""
        tokens = parse_token_grammar("""
NUMBER = /[0-9]+/
PLUS   = "+"
MINUS  = "-"
""")
        grammar = parse_parser_grammar("""
expression = NUMBER { PLUS NUMBER } ;
""")
        issues = cross_validate(tokens, grammar)
        warnings = [i for i in issues if i.startswith("Warning")]
        assert len(warnings) == 1
        assert "MINUS" in warnings[0]

    def test_multiple_issues(self) -> None:
        """Multiple errors and warnings can be reported at once."""
        tokens = parse_token_grammar("""
NUMBER = /[0-9]+/
UNUSED_A = "a"
UNUSED_B = "b"
""")
        grammar = parse_parser_grammar("""
expression = NUMBER PLUS MINUS ;
""")
        issues = cross_validate(tokens, grammar)
        errors = [i for i in issues if i.startswith("Error")]
        warnings = [i for i in issues if i.startswith("Warning")]
        # Missing: PLUS, MINUS
        assert len(errors) == 2
        # Unused: UNUSED_A, UNUSED_B
        assert len(warnings) == 2

    def test_empty_grammars(self) -> None:
        """Empty grammars produce no issues."""
        tokens = parse_token_grammar("")
        grammar = parse_parser_grammar("")
        issues = cross_validate(tokens, grammar)
        assert issues == []


class TestCrossValidateIndentationMode:
    """Test cross-validation with indentation mode implicit tokens."""

    def test_indent_dedent_newline_implicit(self) -> None:
        """INDENT, DEDENT, NEWLINE are valid when mode is indentation."""
        tokens = parse_token_grammar("""
mode: indentation
NAME = /[a-z]+/
COLON = ":"
""")
        grammar = parse_parser_grammar("""
block = NAME COLON NEWLINE INDENT NAME NEWLINE DEDENT ;
""")
        issues = cross_validate(tokens, grammar)
        errors = [i for i in issues if i.startswith("Error")]
        assert errors == []

    def test_indent_dedent_not_implicit_without_mode(self) -> None:
        """Without indentation mode, INDENT/DEDENT are errors but NEWLINE is valid.

        NEWLINE is always a valid synthetic token because the lexer emits it
        whenever a bare newline is encountered and no skip pattern consumed it.
        This is important for newline-sensitive formats like TOML and Starlark.
        INDENT/DEDENT are only valid in indentation mode.
        """
        tokens = parse_token_grammar("""
NAME = /[a-z]+/
COLON = ":"
""")
        grammar = parse_parser_grammar("""
block = NAME COLON NEWLINE INDENT NAME DEDENT ;
""")
        issues = cross_validate(tokens, grammar)
        errors = [i for i in issues if i.startswith("Error")]
        assert len(errors) == 2
        error_text = " ".join(errors)
        assert "INDENT" in error_text
        assert "DEDENT" in error_text
        assert "NEWLINE" not in error_text

    def test_eof_always_implicit(self) -> None:
        """EOF is always valid even without indentation mode."""
        tokens = parse_token_grammar("NAME = /[a-z]+/")
        grammar = parse_parser_grammar("file = NAME EOF ;")
        issues = cross_validate(tokens, grammar)
        errors = [i for i in issues if i.startswith("Error")]
        assert errors == []


class TestCrossValidateAliases:
    """Test cross-validation with aliased token definitions."""

    def test_alias_counts_as_used(self) -> None:
        """A definition with alias=STRING is used when grammar references STRING."""
        tokens = parse_token_grammar("""
STRING_DQ = /"[^"]*"/ -> STRING
NAME = /[a-z]+/
""")
        grammar = parse_parser_grammar("""
expr = STRING | NAME ;
""")
        issues = cross_validate(tokens, grammar)
        warnings = [i for i in issues if i.startswith("Warning")]
        # STRING_DQ should NOT be warned as unused — its alias STRING is used
        assert not any("STRING_DQ" in w for w in warnings)

    def test_original_name_also_resolves(self) -> None:
        """Referencing the original name (not alias) also counts as defined."""
        tokens = parse_token_grammar("""
STRING_DQ = /"[^"]*"/ -> STRING
""")
        grammar = parse_parser_grammar("""
expr = STRING_DQ ;
""")
        issues = cross_validate(tokens, grammar)
        errors = [i for i in issues if i.startswith("Error")]
        assert errors == []

    def test_unreferenced_aliased_token_warns(self) -> None:
        """An aliased token neither referenced by name nor alias is unused."""
        tokens = parse_token_grammar("""
STRING_DQ = /"[^"]*"/ -> STRING
NAME = /[a-z]+/
""")
        grammar = parse_parser_grammar("""
expr = NAME ;
""")
        issues = cross_validate(tokens, grammar)
        warnings = [i for i in issues if i.startswith("Warning")]
        assert any("STRING_DQ" in w for w in warnings)
