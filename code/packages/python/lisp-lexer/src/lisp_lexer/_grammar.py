# AUTO-GENERATED FILE — DO NOT EDIT
# Source: lisp.tokens
# Regenerate with: grammar-tools compile-tokens <source.tokens>
#
# This file embeds a TokenGrammar as native Python data structures.
# Downstream packages import TOKEN_GRAMMAR directly instead of
# reading and parsing the .tokens file at runtime.

from grammar_tools.token_grammar import PatternGroup, TokenDefinition, TokenGrammar

# fmt: off  # noqa: E501 — generated code may have long lines

TOKEN_GRAMMAR = TokenGrammar(
    version=1,
    case_insensitive=False,
    case_sensitive=True,
    definitions=[
        TokenDefinition(
            name='NUMBER',
            pattern='-?[0-9]+',
            is_regex=True,
            line_number=11,
            alias=None,
        ),
        TokenDefinition(
            name='SYMBOL',
            pattern='[a-zA-Z_+\\-*\\/=<>!?&][a-zA-Z0-9_+\\-*\\/=<>!?&]*',
            is_regex=True,
            line_number=12,
            alias=None,
        ),
        TokenDefinition(
            name='STRING',
            pattern='"([^"\\\\]|\\\\.)*"',
            is_regex=True,
            line_number=13,
            alias=None,
        ),
        TokenDefinition(
            name='LPAREN',
            pattern='(',
            is_regex=False,
            line_number=14,
            alias=None,
        ),
        TokenDefinition(
            name='RPAREN',
            pattern=')',
            is_regex=False,
            line_number=15,
            alias=None,
        ),
        TokenDefinition(
            name='QUOTE',
            pattern="'",
            is_regex=False,
            line_number=16,
            alias=None,
        ),
        TokenDefinition(
            name='DOT',
            pattern='.',
            is_regex=False,
            line_number=17,
            alias=None,
        ),
    ],
    keywords=[],
    mode=None,
    escape_mode='none',
    skip_definitions=[
        TokenDefinition(
            name='WHITESPACE',
            pattern='[ \\t\\r\\n]+',
            is_regex=True,
            line_number=8,
            alias=None,
        ),
        TokenDefinition(
            name='COMMENT',
            pattern=';[^\\n]*',
            is_regex=True,
            line_number=9,
            alias=None,
        ),
    ],
    reserved_keywords=[],
    error_definitions=[],
    groups={},
    layout_keywords=[],
)
