"""
Tests for the .grammar file parser and validator.

These tests verify that parse_parser_grammar correctly reads EBNF notation
and builds the expected AST, and that validate_parser_grammar catches
semantic issues like undefined references and unreachable rules.
"""

from __future__ import annotations

import pytest

from grammar_tools.parser_grammar import (
    Alternation,
    GrammarRule,
    Group,
    Literal,
    Optional,
    ParserGrammar,
    ParserGrammarError,
    Repetition,
    RuleReference,
    Sequence,
    parse_parser_grammar,
    validate_parser_grammar,
)


# ---------------------------------------------------------------------------
# Parsing: happy paths
# ---------------------------------------------------------------------------


class TestParseMinimal:
    """Test parsing the simplest possible .grammar files."""

    def test_single_rule_with_token_ref(self) -> None:
        """A grammar with one rule referencing a single token."""
        grammar = parse_parser_grammar("program = NUMBER ;")
        assert len(grammar.rules) == 1
        rule = grammar.rules[0]
        assert rule.name == "program"
        assert isinstance(rule.body, RuleReference)
        assert rule.body.name == "NUMBER"
        assert rule.body.is_token is True

    def test_single_rule_with_rule_ref(self) -> None:
        """A rule referencing another rule (lowercase name)."""
        grammar = parse_parser_grammar(
            "program = expression ;\nexpression = NUMBER ;"
        )
        assert len(grammar.rules) == 2
        assert isinstance(grammar.rules[0].body, RuleReference)
        assert grammar.rules[0].body.name == "expression"
        assert grammar.rules[0].body.is_token is False

    def test_sequence(self) -> None:
        """Multiple elements in a row form a Sequence."""
        grammar = parse_parser_grammar("assignment = NAME EQUALS NUMBER ;")
        rule = grammar.rules[0]
        assert isinstance(rule.body, Sequence)
        assert len(rule.body.elements) == 3
        assert rule.body.elements[0] == RuleReference("NAME", is_token=True)
        assert rule.body.elements[1] == RuleReference("EQUALS", is_token=True)
        assert rule.body.elements[2] == RuleReference("NUMBER", is_token=True)


class TestParseAlternation:
    """Test the | alternation operator."""

    def test_simple_alternation(self) -> None:
        """Two alternatives separated by |."""
        grammar = parse_parser_grammar("value = NUMBER | NAME ;")
        rule = grammar.rules[0]
        assert isinstance(rule.body, Alternation)
        assert len(rule.body.choices) == 2
        assert rule.body.choices[0] == RuleReference("NUMBER", is_token=True)
        assert rule.body.choices[1] == RuleReference("NAME", is_token=True)

    def test_three_alternatives(self) -> None:
        """Three alternatives."""
        grammar = parse_parser_grammar("value = NUMBER | NAME | STRING ;")
        rule = grammar.rules[0]
        assert isinstance(rule.body, Alternation)
        assert len(rule.body.choices) == 3


class TestParseRepetition:
    """Test the { } zero-or-more repetition."""

    def test_simple_repetition(self) -> None:
        """{ statement } becomes Repetition(RuleReference('statement'))."""
        grammar = parse_parser_grammar("program = { statement } ;")
        rule = grammar.rules[0]
        assert isinstance(rule.body, Repetition)
        assert rule.body.element == RuleReference("statement", is_token=False)

    def test_repetition_in_sequence(self) -> None:
        """Repetition used as part of a sequence."""
        grammar = parse_parser_grammar(
            "expression = term { PLUS term } ;"
        )
        rule = grammar.rules[0]
        assert isinstance(rule.body, Sequence)
        assert len(rule.body.elements) == 2
        assert isinstance(rule.body.elements[1], Repetition)


class TestParseOptional:
    """Test the [ ] optional notation."""

    def test_simple_optional(self) -> None:
        """[ ELSE block ] becomes Optional(Sequence(...))."""
        grammar = parse_parser_grammar("if_stmt = IF expression [ ELSE block ] ;")
        rule = grammar.rules[0]
        assert isinstance(rule.body, Sequence)
        # The optional is the last element in the sequence.
        opt = rule.body.elements[2]
        assert isinstance(opt, Optional)

    def test_optional_single_element(self) -> None:
        """[ SEMICOLON ] with a single element."""
        grammar = parse_parser_grammar("stmt = expression [ SEMICOLON ] ;")
        rule = grammar.rules[0]
        assert isinstance(rule.body, Sequence)
        assert isinstance(rule.body.elements[1], Optional)
        assert rule.body.elements[1].element == RuleReference(
            "SEMICOLON", is_token=True
        )


class TestParseGrouping:
    """Test the ( ) grouping notation."""

    def test_grouped_alternation(self) -> None:
        """( PLUS | MINUS ) groups an alternation."""
        grammar = parse_parser_grammar(
            "expression = term { ( PLUS | MINUS ) term } ;"
        )
        rule = grammar.rules[0]
        assert isinstance(rule.body, Sequence)
        rep = rule.body.elements[1]
        assert isinstance(rep, Repetition)
        # Inside the repetition: sequence of (group, term).
        inner = rep.element
        assert isinstance(inner, Sequence)
        grp = inner.elements[0]
        assert isinstance(grp, Group)
        assert isinstance(grp.element, Alternation)

    def test_simple_group(self) -> None:
        """( expression ) is a Group wrapping a RuleReference."""
        grammar = parse_parser_grammar("factor = ( expression ) ;")
        rule = grammar.rules[0]
        assert isinstance(rule.body, Group)
        assert rule.body.element == RuleReference("expression", is_token=False)


class TestParseLiteral:
    """Test literal string matches in grammar rules."""

    def test_literal_in_rule(self) -> None:
        """A quoted string becomes a Literal node."""
        grammar = parse_parser_grammar('stmt = "return" expression ;')
        rule = grammar.rules[0]
        assert isinstance(rule.body, Sequence)
        assert isinstance(rule.body.elements[0], Literal)
        assert rule.body.elements[0].value == "return"


class TestParseRecursive:
    """Test grammars with recursive rule references."""

    def test_direct_recursion(self) -> None:
        """A rule that references itself."""
        grammar = parse_parser_grammar(
            "expression = NUMBER | expression PLUS expression ;"
        )
        rule = grammar.rules[0]
        assert isinstance(rule.body, Alternation)

    def test_mutual_recursion(self) -> None:
        """Rules that reference each other."""
        source = """
expression = term { PLUS term } ;
term = factor { STAR factor } ;
factor = NUMBER | LPAREN expression RPAREN ;
"""
        grammar = parse_parser_grammar(source)
        assert len(grammar.rules) == 3


class TestParseCommentsAndBlanks:
    """Comments and blank lines are properly ignored."""

    def test_comments_ignored(self) -> None:
        source = """# This is a comment
program = { statement } ;
# Another comment
statement = NUMBER ;
"""
        grammar = parse_parser_grammar(source)
        assert len(grammar.rules) == 2

    def test_blank_lines_ignored(self) -> None:
        source = """
program = { statement } ;

statement = NUMBER ;

"""
        grammar = parse_parser_grammar(source)
        assert len(grammar.rules) == 2


# ---------------------------------------------------------------------------
# Parsing: error cases
# ---------------------------------------------------------------------------


class TestParseErrors:
    """Test that malformed .grammar files produce clear errors."""

    def test_missing_semicolon(self) -> None:
        """A rule without a trailing ; raises an error."""
        with pytest.raises(ParserGrammarError, match="Expected SEMI"):
            parse_parser_grammar("program = NUMBER")

    def test_unexpected_character(self) -> None:
        """An unexpected character raises an error."""
        with pytest.raises(ParserGrammarError, match="Unexpected character"):
            parse_parser_grammar("program = NUMBER @ ;")

    def test_unterminated_string(self) -> None:
        """A string literal without closing quote raises an error."""
        with pytest.raises(ParserGrammarError, match="Unterminated"):
            parse_parser_grammar('program = "hello ;')

    def test_unmatched_brace(self) -> None:
        """An unmatched { raises an error."""
        with pytest.raises(ParserGrammarError):
            parse_parser_grammar("program = { statement ;")

    def test_error_includes_line_number(self) -> None:
        """Errors include the correct line number."""
        source = """program = { statement } ;
bad_rule = ;
"""
        with pytest.raises(ParserGrammarError) as exc_info:
            parse_parser_grammar(source)
        assert exc_info.value.line_number == 2


# ---------------------------------------------------------------------------
# Query methods
# ---------------------------------------------------------------------------


class TestQueryMethods:
    """Test rule_names(), token_references(), and rule_references()."""

    @pytest.fixture()
    def sample_grammar(self) -> ParserGrammar:
        source = """
expression = term { ( PLUS | MINUS ) term } ;
term       = factor { ( STAR | SLASH ) factor } ;
factor     = NUMBER | NAME | LPAREN expression RPAREN ;
"""
        return parse_parser_grammar(source)

    def test_rule_names(self, sample_grammar: ParserGrammar) -> None:
        assert sample_grammar.rule_names() == {"expression", "term", "factor"}

    def test_token_references(self, sample_grammar: ParserGrammar) -> None:
        assert sample_grammar.token_references() == {
            "PLUS", "MINUS", "STAR", "SLASH", "NUMBER", "NAME", "LPAREN", "RPAREN",
        }

    def test_rule_references(self, sample_grammar: ParserGrammar) -> None:
        assert sample_grammar.rule_references() == {"term", "factor", "expression"}


# ---------------------------------------------------------------------------
# Validation
# ---------------------------------------------------------------------------


class TestValidation:
    """Test the validate_parser_grammar function."""

    def test_valid_grammar_no_issues(self) -> None:
        """A well-formed grammar produces no warnings."""
        source = """
program    = { statement } ;
statement  = expression NEWLINE ;
expression = NUMBER ;
"""
        grammar = parse_parser_grammar(source)
        issues = validate_parser_grammar(
            grammar, token_names={"NUMBER", "NEWLINE"}
        )
        assert issues == []

    def test_undefined_rule_reference(self) -> None:
        """Referencing a rule that does not exist is flagged."""
        source = "program = undefined_rule ;"
        grammar = parse_parser_grammar(source)
        issues = validate_parser_grammar(grammar)
        assert any("Undefined rule" in i for i in issues)
        assert any("undefined_rule" in i for i in issues)

    def test_undefined_token_reference(self) -> None:
        """Referencing a token not in token_names is flagged."""
        source = "program = MISSING_TOKEN ;"
        grammar = parse_parser_grammar(source)
        issues = validate_parser_grammar(grammar, token_names={"NUMBER"})
        assert any("Undefined token" in i for i in issues)
        assert any("MISSING_TOKEN" in i for i in issues)

    def test_no_token_check_without_token_names(self) -> None:
        """Without token_names, token references are not checked."""
        source = "program = ANYTHING ;"
        grammar = parse_parser_grammar(source)
        issues = validate_parser_grammar(grammar, token_names=None)
        assert not any("Undefined token" in i for i in issues)

    def test_duplicate_rule_name(self) -> None:
        """Duplicate rule names are flagged."""
        source = """
program = NUMBER ;
program = NAME ;
"""
        grammar = parse_parser_grammar(source)
        issues = validate_parser_grammar(grammar)
        assert any("Duplicate" in i for i in issues)

    def test_non_lowercase_rule_name(self) -> None:
        """Rule names that aren't lowercase are flagged."""
        source = "Program = NUMBER ;"
        grammar = parse_parser_grammar(source)
        issues = validate_parser_grammar(grammar)
        assert any("lowercase" in i for i in issues)

    def test_unreachable_rule(self) -> None:
        """A rule defined but never referenced is flagged as unreachable."""
        source = """
program = NUMBER ;
orphan  = NAME ;
"""
        grammar = parse_parser_grammar(source)
        issues = validate_parser_grammar(grammar)
        assert any("unreachable" in i for i in issues)
        assert any("orphan" in i for i in issues)

    def test_start_rule_not_flagged_unreachable(self) -> None:
        """The first rule (start symbol) is never flagged as unreachable."""
        source = "program = NUMBER ;"
        grammar = parse_parser_grammar(source)
        issues = validate_parser_grammar(grammar)
        assert not any("unreachable" in i for i in issues)

    def test_referenced_rule_not_unreachable(self) -> None:
        """A rule referenced by another rule is not unreachable."""
        source = """
program = expression ;
expression = NUMBER ;
"""
        grammar = parse_parser_grammar(source)
        issues = validate_parser_grammar(grammar)
        assert not any("unreachable" in i for i in issues)


# ---------------------------------------------------------------------------
# Magic comments
# ---------------------------------------------------------------------------


class TestMagicComments:
    """Tests for magic comment directives (# @key value) in .grammar files.

    Magic comments follow the same convention as in .tokens files. They are
    comment lines of the form ``# @key value`` that carry metadata about
    the grammar file without affecting EBNF parsing.

    Recognised directives for .grammar files:
        ``# @version N`` — sets ParserGrammar.version to the integer N.

    All other ``@key`` directives are silently ignored for forward
    compatibility.
    """

    def test_version_is_set(self) -> None:
        """# @version 1 sets grammar.version to 1."""
        source = "# @version 1\nprogram = NUMBER ;\n"
        grammar = parse_parser_grammar(source)
        assert grammar.version == 1

    def test_version_larger_number(self) -> None:
        """# @version 5 sets grammar.version to 5."""
        source = "# @version 5\nprogram = NUMBER ;\n"
        grammar = parse_parser_grammar(source)
        assert grammar.version == 5

    def test_version_default_zero(self) -> None:
        """Without # @version, grammar.version defaults to 0."""
        source = "program = NUMBER ;\n"
        grammar = parse_parser_grammar(source)
        assert grammar.version == 0

    def test_unknown_magic_key_silently_ignored(self) -> None:
        """An unrecognised @key does not raise an error."""
        source = "# @future_directive foo\nprogram = NUMBER ;\n"
        grammar = parse_parser_grammar(source)
        assert grammar.version == 0
        assert len(grammar.rules) == 1

    def test_magic_comment_mixed_with_normal_comments(self) -> None:
        """Magic comments and ordinary comments coexist peacefully."""
        source = (
            "# Ordinary comment\n"
            "# @version 3\n"
            "# Another ordinary comment\n"
            "program = NUMBER ;\n"
        )
        grammar = parse_parser_grammar(source)
        assert grammar.version == 3
        assert len(grammar.rules) == 1

    def test_magic_comment_does_not_affect_rules(self) -> None:
        """Magic comments do not accidentally consume or corrupt EBNF rules."""
        source = (
            "# @version 2\n"
            "program = { statement } ;\n"
            "statement = NUMBER ;\n"
        )
        grammar = parse_parser_grammar(source)
        assert grammar.version == 2
        assert len(grammar.rules) == 2
        assert grammar.rules[0].name == "program"
        assert grammar.rules[1].name == "statement"

    def test_version_non_integer_silently_ignored(self) -> None:
        """A non-integer @version value is silently ignored (version stays 0)."""
        source = "# @version bad\nprogram = NUMBER ;\n"
        grammar = parse_parser_grammar(source)
        assert grammar.version == 0

    def test_magic_comment_extra_whitespace(self) -> None:
        """Extra whitespace around the @ key and value is handled correctly."""
        source = "#  @version   9\nprogram = NUMBER ;\n"
        grammar = parse_parser_grammar(source)
        assert grammar.version == 9
