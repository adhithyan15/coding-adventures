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
