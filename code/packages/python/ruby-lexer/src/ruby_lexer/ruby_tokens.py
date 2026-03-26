# AUTO-GENERATED FILE - DO NOT EDIT
from grammar_tools.token_grammar import TokenGrammar, TokenDefinition, PatternGroup

RubyTokens = TokenGrammar(
    version=1,
    case_insensitive=False,
    case_sensitive=True,
    keywords=["if", "else", "elsif", "end", "while", "for", "do", "def", "return", "class", "module", "require", "puts", "true", "false", "nil", "and", "or", "not", "then", "unless", "until", "yield", "begin", "rescue", "ensure"],
    definitions=[
        TokenDefinition(name="NAME", pattern="[a-zA-Z_][a-zA-Z0-9_]*", is_regex=True, line_number=23),
        TokenDefinition(name="NUMBER", pattern="[0-9]+", is_regex=True, line_number=24),
        TokenDefinition(name="STRING", pattern="\"([^\"\\\\]|\\\\.)*\"", is_regex=True, line_number=25),
        TokenDefinition(name="EQUALS_EQUALS", pattern="==", is_regex=False, line_number=28),
        TokenDefinition(name="DOT_DOT", pattern="..", is_regex=False, line_number=29),
        TokenDefinition(name="HASH_ROCKET", pattern="=>", is_regex=False, line_number=30),
        TokenDefinition(name="NOT_EQUALS", pattern="!=", is_regex=False, line_number=31),
        TokenDefinition(name="LESS_EQUALS", pattern="<=", is_regex=False, line_number=32),
        TokenDefinition(name="GREATER_EQUALS", pattern=">=", is_regex=False, line_number=33),
        TokenDefinition(name="EQUALS", pattern="=", is_regex=False, line_number=36),
        TokenDefinition(name="PLUS", pattern="+", is_regex=False, line_number=37),
        TokenDefinition(name="MINUS", pattern="-", is_regex=False, line_number=38),
        TokenDefinition(name="STAR", pattern="*", is_regex=False, line_number=39),
        TokenDefinition(name="SLASH", pattern="/", is_regex=False, line_number=40),
        TokenDefinition(name="LESS_THAN", pattern="<", is_regex=False, line_number=43),
        TokenDefinition(name="GREATER_THAN", pattern=">", is_regex=False, line_number=44),
        TokenDefinition(name="LPAREN", pattern="(", is_regex=False, line_number=47),
        TokenDefinition(name="RPAREN", pattern=")", is_regex=False, line_number=48),
        TokenDefinition(name="COMMA", pattern=",", is_regex=False, line_number=49),
        TokenDefinition(name="COLON", pattern=":", is_regex=False, line_number=50),
    ],
)
