# AUTO-GENERATED FILE - DO NOT EDIT
from grammar_tools.token_grammar import TokenGrammar, TokenDefinition, PatternGroup

LispTokens = TokenGrammar(
    version=1,
    case_insensitive=False,
    case_sensitive=True,
    definitions=[
        TokenDefinition(name="NUMBER", pattern="-?[0-9]+", is_regex=True, line_number=6),
        TokenDefinition(name="SYMBOL", pattern="[a-zA-Z_+\\-*\\/=<>!?&][a-zA-Z0-9_+\\-*\\/=<>!?&]*", is_regex=True, line_number=7),
        TokenDefinition(name="STRING", pattern="\"([^\"\\\\]|\\\\.)*\"", is_regex=True, line_number=8),
        TokenDefinition(name="LPAREN", pattern="(", is_regex=False, line_number=9),
        TokenDefinition(name="RPAREN", pattern=")", is_regex=False, line_number=10),
        TokenDefinition(name="QUOTE", pattern="'", is_regex=False, line_number=11),
        TokenDefinition(name="DOT", pattern=".", is_regex=False, line_number=12),
    ],
    skip_definitions=[
        TokenDefinition(name="WHITESPACE", pattern="[ \\t\\r\\n]+", is_regex=True, line_number=3),
        TokenDefinition(name="COMMENT", pattern=";[^\\n]*", is_regex=True, line_number=4),
    ],
)
