# AUTO-GENERATED FILE — DO NOT EDIT
# Source: json.tokens
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
            name='STRING',
            pattern='"([^"\\\\]|\\\\["\\\\\\x2fbfnrt]|\\\\u[0-9a-fA-F]{4})*"',
            is_regex=True,
            line_number=30,
            alias=None,
        ),
        TokenDefinition(
            name='NUMBER',
            pattern='-?[0-9]+\\.?[0-9]*[eE]?[-+]?[0-9]*',
            is_regex=True,
            line_number=37,
            alias=None,
        ),
        TokenDefinition(
            name='TRUE',
            pattern='true',
            is_regex=False,
            line_number=41,
            alias=None,
        ),
        TokenDefinition(
            name='FALSE',
            pattern='false',
            is_regex=False,
            line_number=42,
            alias=None,
        ),
        TokenDefinition(
            name='NULL',
            pattern='null',
            is_regex=False,
            line_number=43,
            alias=None,
        ),
        TokenDefinition(
            name='LBRACE',
            pattern='{',
            is_regex=False,
            line_number=49,
            alias=None,
        ),
        TokenDefinition(
            name='RBRACE',
            pattern='}',
            is_regex=False,
            line_number=50,
            alias=None,
        ),
        TokenDefinition(
            name='LBRACKET',
            pattern='[',
            is_regex=False,
            line_number=51,
            alias=None,
        ),
        TokenDefinition(
            name='RBRACKET',
            pattern=']',
            is_regex=False,
            line_number=52,
            alias=None,
        ),
        TokenDefinition(
            name='COLON',
            pattern=':',
            is_regex=False,
            line_number=53,
            alias=None,
        ),
        TokenDefinition(
            name='COMMA',
            pattern=',',
            is_regex=False,
            line_number=54,
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
            line_number=65,
            alias=None,
        ),
    ],
    reserved_keywords=[],
    error_definitions=[],
    groups={},
    layout_keywords=[],
)
