"""
Tests for the grammar compiler (compiler.py).

The compiler transforms in-memory TokenGrammar and ParserGrammar objects into
Python source code. Tests here verify:

1. The generated code is valid Python (exec-able without errors).
2. Loading the generated code recreates an equivalent grammar object.
3. All grammar features are round-tripped correctly (aliases, skip patterns,
   error patterns, groups, keywords, mode, escape_mode).
4. Edge cases: empty grammars, special characters in patterns, etc.

Round-trip fidelity
-------------------

A round-trip test works like this:

    original = parse_token_grammar(source)
    code = compile_token_grammar(original)
    namespace = {}
    exec(code, namespace)
    loaded = namespace["TOKEN_GRAMMAR"]
    assert loaded == original

If the compiler is correct, the loaded grammar is indistinguishable from the
one that came out of the parser.
"""

from __future__ import annotations

import textwrap
from typing import Any

from grammar_tools.compiler import compile_parser_grammar, compile_token_grammar
from grammar_tools.parser_grammar import (
    Alternation,
    Literal,
    ParserGrammar,
    Repetition,
    Sequence,
    parse_parser_grammar,
)
from grammar_tools.token_grammar import TokenGrammar, parse_token_grammar

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _exec_token_grammar(code: str) -> TokenGrammar:
    """Execute generated code and return the TOKEN_GRAMMAR it defines."""
    namespace: dict[str, Any] = {}
    exec(code, namespace)  # noqa: S102 — test-only execution of generated code
    return namespace["TOKEN_GRAMMAR"]


def _exec_parser_grammar(code: str) -> ParserGrammar:
    """Execute generated code and return the PARSER_GRAMMAR it defines."""
    namespace: dict[str, Any] = {}
    exec(code, namespace)  # noqa: S102
    return namespace["PARSER_GRAMMAR"]


# ---------------------------------------------------------------------------
# compile_token_grammar — output structure
# ---------------------------------------------------------------------------


class TestCompileTokenGrammarOutput:
    """Tests that verify the structure of the generated token grammar code."""

    def test_header_contains_do_not_edit(self) -> None:
        """The generated file should have a DO NOT EDIT header."""
        grammar = TokenGrammar()
        code = compile_token_grammar(grammar)
        assert "DO NOT EDIT" in code
        assert "# ruff: noqa: E501, F401" in code

    def test_header_contains_source_when_given(self) -> None:
        """Source filename appears in header when provided."""
        grammar = TokenGrammar()
        code = compile_token_grammar(grammar, "my_language.tokens")
        assert "my_language.tokens" in code

    def test_header_omits_source_when_empty(self) -> None:
        """No stray '# Source:' line when source_file is empty."""
        grammar = TokenGrammar()
        code = compile_token_grammar(grammar, "")
        assert "# Source:" not in code

    def test_imports_token_grammar_types(self) -> None:
        """Generated code imports the necessary types."""
        grammar = TokenGrammar()
        code = compile_token_grammar(grammar)
        assert "from grammar_tools.token_grammar import" in code
        assert "TokenGrammar" in code
        assert "TokenDefinition" in code
        assert "PatternGroup" in code

    def test_defines_token_grammar_constant(self) -> None:
        """Generated code assigns to TOKEN_GRAMMAR."""
        grammar = TokenGrammar()
        code = compile_token_grammar(grammar)
        assert "TOKEN_GRAMMAR = TokenGrammar(" in code


# ---------------------------------------------------------------------------
# compile_token_grammar — round-trip tests
# ---------------------------------------------------------------------------


class TestCompileTokenGrammarRoundTrip:
    """Verify that compiling then loading recreates the original grammar."""

    def test_empty_grammar(self) -> None:
        """An empty TokenGrammar round-trips cleanly."""
        original = TokenGrammar()
        code = compile_token_grammar(original)
        loaded = _exec_token_grammar(code)
        assert loaded == original

    def test_single_regex_token(self) -> None:
        """One regex definition round-trips."""
        original = parse_token_grammar("NUMBER = /[0-9]+/")
        code = compile_token_grammar(original)
        loaded = _exec_token_grammar(code)
        assert loaded == original

    def test_single_literal_token(self) -> None:
        """One literal definition round-trips."""
        original = parse_token_grammar('PLUS = "+"')
        code = compile_token_grammar(original)
        loaded = _exec_token_grammar(code)
        assert loaded == original

    def test_alias(self) -> None:
        """Alias definitions round-trip, preserving both name and alias."""
        original = parse_token_grammar('STRING_DQ = /"[^"]*"/ -> STRING')
        code = compile_token_grammar(original)
        loaded = _exec_token_grammar(code)
        assert loaded == original
        assert loaded.definitions[0].alias == "STRING"

    def test_keywords(self) -> None:
        """Keyword lists round-trip."""
        source = textwrap.dedent("""\
            NAME = /[a-z]+/
            keywords:
              if
              else
              while
        """)
        original = parse_token_grammar(source)
        code = compile_token_grammar(original)
        loaded = _exec_token_grammar(code)
        assert loaded == original
        assert loaded.keywords == ["if", "else", "while"]

    def test_skip_definitions(self) -> None:
        """Skip pattern definitions round-trip."""
        source = textwrap.dedent("""\
            NAME = /[a-z]+/
            skip:
              WHITESPACE = /[ \\t]+/
              COMMENT = /#.*/
        """)
        original = parse_token_grammar(source)
        code = compile_token_grammar(original)
        loaded = _exec_token_grammar(code)
        assert loaded == original
        assert len(loaded.skip_definitions) == 2

    def test_error_definitions(self) -> None:
        """Error recovery pattern definitions round-trip."""
        source = textwrap.dedent("""\
            STRING = /"[^"]*"/
            errors:
              BAD_STRING = /"[^"\\n]*/
        """)
        original = parse_token_grammar(source)
        code = compile_token_grammar(original)
        loaded = _exec_token_grammar(code)
        assert loaded == original

    def test_mode_indentation(self) -> None:
        """Mode directive round-trips."""
        source = textwrap.dedent("""\
            mode: indentation
            NAME = /[a-z]+/
        """)
        original = parse_token_grammar(source)
        code = compile_token_grammar(original)
        loaded = _exec_token_grammar(code)
        assert loaded == original
        assert loaded.mode == "indentation"

    def test_escape_mode_none(self) -> None:
        """Escape mode directive round-trips."""
        source = textwrap.dedent("""\
            escapes: none
            STRING = /"[^"]*"/
        """)
        original = parse_token_grammar(source)
        code = compile_token_grammar(original)
        loaded = _exec_token_grammar(code)
        assert loaded == original
        assert loaded.escape_mode == "none"

    def test_case_insensitive(self) -> None:
        """case_insensitive flag round-trips."""
        source = "# @case_insensitive true\nNAME = /[a-z]+/"
        original = parse_token_grammar(source)
        code = compile_token_grammar(original)
        loaded = _exec_token_grammar(code)
        assert loaded == original
        assert loaded.case_insensitive is True

    def test_pattern_groups(self) -> None:
        """Pattern groups round-trip fully."""
        source = textwrap.dedent("""\
            TEXT = /[^<]+/
            group tag:
              ATTR = /[a-z]+/
              EQ = "="
        """)
        original = parse_token_grammar(source)
        code = compile_token_grammar(original)
        loaded = _exec_token_grammar(code)
        assert loaded == original
        assert "tag" in loaded.groups
        assert len(loaded.groups["tag"].definitions) == 2

    def test_version(self) -> None:
        """Version field round-trips."""
        source = "# @version 3\nNAME = /[a-z]+/"
        original = parse_token_grammar(source)
        code = compile_token_grammar(original)
        loaded = _exec_token_grammar(code)
        assert loaded == original
        assert loaded.version == 3

    def test_special_regex_characters(self) -> None:
        """Patterns with backslashes and special chars round-trip safely."""
        # This pattern has \\t (tab), \\n (newline), \\u (unicode),
        # single quotes, and backslashes — all need careful escaping.
        source = r'STRING = /"([^"\\]|\\["\\/bfnrt]|\\u[0-9a-fA-F]{4})*"/'
        original = parse_token_grammar(source)
        code = compile_token_grammar(original)
        loaded = _exec_token_grammar(code)
        assert loaded == original

    def test_json_tokens_full_round_trip(self) -> None:
        """Full round-trip on the JSON token grammar."""
        source = textwrap.dedent(r"""
            STRING   = /"([^"\\]|\\["\\\x2fbfnrt]|\\u[0-9a-fA-F]{4})*"/
            NUMBER   = /-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?/
            TRUE     = "true"
            FALSE    = "false"
            NULL     = "null"
            LBRACE   = "{"
            RBRACE   = "}"
            LBRACKET = "["
            RBRACKET = "]"
            COLON    = ":"
            COMMA    = ","
            skip:
              WHITESPACE = /[ \t\r\n]+/
        """).strip()
        original = parse_token_grammar(source)
        code = compile_token_grammar(original)
        loaded = _exec_token_grammar(code)
        assert loaded == original
        assert len(loaded.definitions) == 11
        assert len(loaded.skip_definitions) == 1


# ---------------------------------------------------------------------------
# compile_parser_grammar — output structure
# ---------------------------------------------------------------------------


class TestCompileParserGrammarOutput:
    """Tests that verify the structure of the generated parser grammar code."""

    def test_header_contains_do_not_edit(self) -> None:
        """The generated file should have a DO NOT EDIT header."""
        grammar = ParserGrammar()
        code = compile_parser_grammar(grammar)
        assert "DO NOT EDIT" in code
        assert "# ruff: noqa: E501, F401" in code

    def test_imports_parser_grammar_types(self) -> None:
        """Generated code imports the necessary types."""
        grammar = ParserGrammar()
        code = compile_parser_grammar(grammar)
        assert "from grammar_tools.parser_grammar import" in code
        assert "ParserGrammar" in code
        assert "GrammarRule" in code
        assert "RuleReference" in code
        assert "Sequence" in code
        assert "Alternation" in code
        assert "Repetition" in code
        assert "Optional" in code
        assert "Group" in code
        assert "Literal" in code

    def test_defines_parser_grammar_constant(self) -> None:
        """Generated code assigns to PARSER_GRAMMAR."""
        grammar = ParserGrammar()
        code = compile_parser_grammar(grammar)
        assert "PARSER_GRAMMAR = ParserGrammar(" in code


# ---------------------------------------------------------------------------
# compile_parser_grammar — round-trip tests
# ---------------------------------------------------------------------------


class TestCompileParserGrammarRoundTrip:
    """Verify that compiling then loading recreates the original parser grammar."""

    def test_empty_grammar(self) -> None:
        """An empty ParserGrammar round-trips cleanly."""
        original = ParserGrammar()
        code = compile_parser_grammar(original)
        loaded = _exec_parser_grammar(code)
        assert loaded == original

    def test_single_token_reference(self) -> None:
        """A rule with a single token reference round-trips."""
        original = parse_parser_grammar("value = NUMBER ;")
        code = compile_parser_grammar(original)
        loaded = _exec_parser_grammar(code)
        assert loaded == original

    def test_single_rule_reference(self) -> None:
        """A rule with a single rule reference round-trips."""
        original = parse_parser_grammar("start = expr ;\nexpr = NUMBER ;")
        code = compile_parser_grammar(original)
        loaded = _exec_parser_grammar(code)
        assert loaded == original

    def test_literal(self) -> None:
        """Literal elements round-trip."""
        original = parse_parser_grammar('start = "hello" ;')
        code = compile_parser_grammar(original)
        loaded = _exec_parser_grammar(code)
        assert loaded == original
        assert isinstance(loaded.rules[0].body, Literal)
        assert loaded.rules[0].body.value == "hello"

    def test_alternation(self) -> None:
        """Alternation round-trips."""
        original = parse_parser_grammar("value = A | B | C ;")
        code = compile_parser_grammar(original)
        loaded = _exec_parser_grammar(code)
        assert loaded == original
        assert isinstance(loaded.rules[0].body, Alternation)
        assert len(loaded.rules[0].body.choices) == 3

    def test_sequence(self) -> None:
        """Sequence round-trips."""
        original = parse_parser_grammar("pair = KEY COLON value ;")
        code = compile_parser_grammar(original)
        loaded = _exec_parser_grammar(code)
        assert loaded == original
        assert isinstance(loaded.rules[0].body, Sequence)

    def test_repetition(self) -> None:
        """Repetition (zero-or-more) round-trips."""
        original = parse_parser_grammar("stmts = { stmt } ;")
        code = compile_parser_grammar(original)
        loaded = _exec_parser_grammar(code)
        assert loaded == original
        assert isinstance(loaded.rules[0].body, Repetition)

    def test_optional(self) -> None:
        """Optional (zero-or-one) round-trips."""
        original = parse_parser_grammar("expr = NUMBER [ PLUS NUMBER ] ;")
        code = compile_parser_grammar(original)
        loaded = _exec_parser_grammar(code)
        assert loaded == original

    def test_group(self) -> None:
        """Group (explicit parentheses) round-trips."""
        original = parse_parser_grammar("term = NUMBER { ( PLUS | MINUS ) NUMBER } ;")
        code = compile_parser_grammar(original)
        loaded = _exec_parser_grammar(code)
        assert loaded == original

    def test_version(self) -> None:
        """Version field round-trips."""
        source = "# @version 2\nvalue = NUMBER ;"
        original = parse_parser_grammar(source)
        code = compile_parser_grammar(original)
        loaded = _exec_parser_grammar(code)
        assert loaded == original
        assert loaded.version == 2

    def test_json_grammar_full_round_trip(self) -> None:
        """Full round-trip on the JSON parser grammar."""
        source = textwrap.dedent("""\
            value    = object | array | STRING | NUMBER | TRUE | FALSE | NULL ;
            object   = LBRACE [ pair { COMMA pair } ] RBRACE ;
            pair     = STRING COLON value ;
            array    = LBRACKET [ value { COMMA value } ] RBRACKET ;
        """)
        original = parse_parser_grammar(source)
        code = compile_parser_grammar(original)
        loaded = _exec_parser_grammar(code)
        assert loaded == original
        assert len(loaded.rules) == 4
        assert loaded.rules[0].name == "value"
