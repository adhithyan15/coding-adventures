# AUTO-GENERATED FILE — DO NOT EDIT
# Source: python.tokens
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
            name='NAME',
            pattern='[a-zA-Z_][a-zA-Z0-9_]*',
            is_regex=True,
            line_number=13,
            alias=None,
        ),
        TokenDefinition(
            name='NUMBER',
            pattern='[0-9]+',
            is_regex=True,
            line_number=14,
            alias=None,
        ),
        TokenDefinition(
            name='STRING',
            pattern='"([^"\\\\]|\\\\.)*"',
            is_regex=True,
            line_number=15,
            alias=None,
        ),
        TokenDefinition(
            name='EQUALS_EQUALS',
            pattern='==',
            is_regex=False,
            line_number=18,
            alias=None,
        ),
        TokenDefinition(
            name='EQUALS',
            pattern='=',
            is_regex=False,
            line_number=21,
            alias=None,
        ),
        TokenDefinition(
            name='PLUS',
            pattern='+',
            is_regex=False,
            line_number=22,
            alias=None,
        ),
        TokenDefinition(
            name='MINUS',
            pattern='-',
            is_regex=False,
            line_number=23,
            alias=None,
        ),
        TokenDefinition(
            name='STAR',
            pattern='*',
            is_regex=False,
            line_number=24,
            alias=None,
        ),
        TokenDefinition(
            name='SLASH',
            pattern='/',
            is_regex=False,
            line_number=25,
            alias=None,
        ),
        TokenDefinition(
            name='LPAREN',
            pattern='(',
            is_regex=False,
            line_number=28,
            alias=None,
        ),
        TokenDefinition(
            name='RPAREN',
            pattern=')',
            is_regex=False,
            line_number=29,
            alias=None,
        ),
        TokenDefinition(
            name='COMMA',
            pattern=',',
            is_regex=False,
            line_number=30,
            alias=None,
        ),
        TokenDefinition(
            name='COLON',
            pattern=':',
            is_regex=False,
            line_number=31,
            alias=None,
        ),
    ],
    keywords=['if', 'else', 'elif', 'while', 'for', 'def', 'return', 'class', 'import', 'from', 'as', 'True', 'False', 'None'],
    mode=None,
    escape_mode=None,
    skip_definitions=[],
    reserved_keywords=[],
    error_definitions=[],
    groups={},
    layout_keywords=[],
)
