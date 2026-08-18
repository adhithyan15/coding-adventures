# AUTO-GENERATED FILE — DO NOT EDIT
# ruff: noqa: E501, F401
# Source: brainfuck.tokens
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
            name='RIGHT',
            pattern='>',
            is_regex=False,
            line_number=23,
            alias=None,
        ),
        TokenDefinition(
            name='LEFT',
            pattern='<',
            is_regex=False,
            line_number=24,
            alias=None,
        ),
        TokenDefinition(
            name='INC',
            pattern='+',
            is_regex=False,
            line_number=29,
            alias=None,
        ),
        TokenDefinition(
            name='DEC',
            pattern='-',
            is_regex=False,
            line_number=30,
            alias=None,
        ),
        TokenDefinition(
            name='OUTPUT',
            pattern='.',
            is_regex=False,
            line_number=35,
            alias=None,
        ),
        TokenDefinition(
            name='INPUT',
            pattern=',',
            is_regex=False,
            line_number=36,
            alias=None,
        ),
        TokenDefinition(
            name='LOOP_START',
            pattern='[',
            is_regex=False,
            line_number=41,
            alias=None,
        ),
        TokenDefinition(
            name='LOOP_END',
            pattern=']',
            is_regex=False,
            line_number=42,
            alias=None,
        ),
    ],
    keywords=[],
    mode=None,
    escape_mode=None,
    skip_definitions=[
        TokenDefinition(
            name='WHITESPACE',
            pattern='[ \\t\\r\\n]+',
            is_regex=True,
            line_number=65,
            alias=None,
        ),
        TokenDefinition(
            name='COMMENT',
            pattern='[^><+\\-.,\\[\\] \\t\\r\\n]+',
            is_regex=True,
            line_number=66,
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
