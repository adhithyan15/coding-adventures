"""
grammar_tools — Define and validate .tokens and .grammar file formats.

This package provides parsers and validators for two declarative file
formats used to describe programming language syntax:

- **.tokens files** define the lexical grammar (what tokens exist)
- **.grammar files** define the syntactic grammar in EBNF (how tokens
  combine into valid programs)

Together, these files provide a complete, language-agnostic description
of a programming language's surface syntax that can be used to generate
lexers and parsers for any target language.
"""

from grammar_tools.cross_validator import cross_validate
from grammar_tools.parser_grammar import (
    Alternation,
    GrammarElement,
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
from grammar_tools.token_grammar import (
    PatternGroup,
    TokenDefinition,
    TokenGrammar,
    TokenGrammarError,
    parse_token_grammar,
    validate_token_grammar,
)

__all__ = [
    # Token grammar
    "PatternGroup",
    "TokenDefinition",
    "TokenGrammar",
    "TokenGrammarError",
    "parse_token_grammar",
    "validate_token_grammar",
    # Parser grammar
    "RuleReference",
    "Literal",
    "Alternation",
    "Sequence",
    "Repetition",
    "Optional",
    "Group",
    "GrammarElement",
    "GrammarRule",
    "ParserGrammar",
    "ParserGrammarError",
    "parse_parser_grammar",
    "validate_parser_grammar",
    # Cross-validation
    "cross_validate",
]
