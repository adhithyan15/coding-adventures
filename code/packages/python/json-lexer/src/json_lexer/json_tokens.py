# AUTO-GENERATED FILE - DO NOT EDIT
from grammar_tools.token_grammar import TokenGrammar, TokenDefinition, PatternGroup

JsonTokens = TokenGrammar(
    version=1,
    case_insensitive=False,
    case_sensitive=True,
    definitions=[
        TokenDefinition(name="STRING", pattern="\"([^\"\\\\]|\\\\[\"\\\\\\x2fbfnrt]|\\\\u[0-9a-fA-F]{4})*\"", is_regex=True, line_number=25),
        TokenDefinition(name="NUMBER", pattern="-?(0|[1-9][0-9]*)(\\.[0-9]+)?([eE][+-]?[0-9]+)?", is_regex=True, line_number=31),
        TokenDefinition(name="TRUE", pattern="true", is_regex=False, line_number=35),
        TokenDefinition(name="FALSE", pattern="false", is_regex=False, line_number=36),
        TokenDefinition(name="NULL", pattern="null", is_regex=False, line_number=37),
        TokenDefinition(name="LBRACE", pattern="{", is_regex=False, line_number=43),
        TokenDefinition(name="RBRACE", pattern="}", is_regex=False, line_number=44),
        TokenDefinition(name="LBRACKET", pattern="[", is_regex=False, line_number=45),
        TokenDefinition(name="RBRACKET", pattern="]", is_regex=False, line_number=46),
        TokenDefinition(name="COLON", pattern=":", is_regex=False, line_number=47),
        TokenDefinition(name="COMMA", pattern=",", is_regex=False, line_number=48),
    ],
    skip_definitions=[
        TokenDefinition(name="WHITESPACE", pattern="[ \\t\\r\\n]+", is_regex=True, line_number=59),
    ],
)
