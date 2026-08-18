# AUTO-GENERATED FILE — DO NOT EDIT
# ruff: noqa: E501, F401
# Source: twig.tokens
# Regenerate with: grammar-tools compile-tokens <source.tokens>
#
# This file embeds a TokenGrammar as native Python data structures.
# Downstream packages import TOKEN_GRAMMAR directly instead of
# reading and parsing the .tokens file at runtime.

from grammar_tools.token_grammar import ModeTransition, PatternGroup, TokenDefinition, TokenGrammar, TransitionAction

# fmt: off  # noqa: E501 — generated code may have long lines

TOKEN_GRAMMAR = TokenGrammar(
    version=1,
    case_insensitive=False,
    case_sensitive=True,
    definitions=[
        TokenDefinition(
            name='LPAREN',
            pattern='(',
            is_regex=False,
            line_number=26,
            alias=None,
        ),
        TokenDefinition(
            name='RPAREN',
            pattern=')',
            is_regex=False,
            line_number=27,
            alias=None,
        ),
        TokenDefinition(
            name='QUOTE',
            pattern="'",
            is_regex=False,
            line_number=28,
            alias=None,
        ),
        TokenDefinition(
            name='COLON',
            pattern=':',
            is_regex=False,
            line_number=36,
            alias=None,
        ),
        TokenDefinition(
            name='ARROW',
            pattern='->',
            is_regex=False,
            line_number=55,
            alias=None,
        ),
        TokenDefinition(
            name='BOOL_TRUE',
            pattern='#t',
            is_regex=False,
            line_number=63,
            alias=None,
        ),
        TokenDefinition(
            name='BOOL_FALSE',
            pattern='#f',
            is_regex=False,
            line_number=64,
            alias=None,
        ),
        TokenDefinition(
            name='STRING',
            pattern='"([^"\\\\]|\\\\.)*"',
            is_regex=True,
            line_number=78,
            alias=None,
        ),
        TokenDefinition(
            name='INTEGER',
            pattern='-?[0-9]+',
            is_regex=True,
            line_number=89,
            alias=None,
        ),
        TokenDefinition(
            name='NAME',
            pattern='[A-Za-z+\\-*/=<>!?_$][A-Za-z+\\-*/=<>!?_$0-9]*',
            is_regex=True,
            line_number=102,
            alias=None,
        ),
    ],
    keywords=['define', 'lambda', 'let', 'if', 'begin', 'quote', 'nil', 'module', 'export', 'import', 'typed', 'type', 'record', 'union', 'match', 'let*'],
    mode=None,
    escape_mode=None,
    skip_definitions=[
        TokenDefinition(
            name='WHITESPACE',
            pattern='[ \\t\\r\\n]+',
            is_regex=True,
            line_number=159,
            alias=None,
        ),
        TokenDefinition(
            name='COMMENT',
            pattern=';[^\\n]*',
            is_regex=True,
            line_number=160,
            alias=None,
        ),
    ],
    reserved_keywords=[],
    error_definitions=[],
    groups={},
    start_mode=None,
    transitions=[],
    layout_keywords=[],
)
