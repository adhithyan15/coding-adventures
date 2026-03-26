# AUTO-GENERATED FILE - DO NOT EDIT
from grammar_tools.token_grammar import TokenGrammar, TokenDefinition, PatternGroup

SqlTokens = TokenGrammar(
    version=1,
    case_insensitive=True,
    case_sensitive=True,
    keywords=["SELECT", "FROM", "WHERE", "GROUP", "BY", "HAVING", "ORDER", "LIMIT", "OFFSET", "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE", "CREATE", "DROP", "TABLE", "IF", "EXISTS", "NOT", "AND", "OR", "NULL", "IS", "IN", "BETWEEN", "LIKE", "AS", "DISTINCT", "ALL", "UNION", "INTERSECT", "EXCEPT", "JOIN", "INNER", "LEFT", "RIGHT", "OUTER", "CROSS", "FULL", "ON", "ASC", "DESC", "TRUE", "FALSE", "CASE", "WHEN", "THEN", "ELSE", "END", "PRIMARY", "KEY", "UNIQUE", "DEFAULT"],
    definitions=[
        TokenDefinition(name="NAME", pattern="[a-zA-Z_][a-zA-Z0-9_]*", is_regex=True, line_number=12),
        TokenDefinition(name="NUMBER", pattern="[0-9]+(\\.[0-9]+)?", is_regex=True, line_number=13),
        TokenDefinition(name="STRING_SQ", pattern="'([^'\\\\]|\\\\.)*'", is_regex=True, line_number=14, alias="STRING"),
        TokenDefinition(name="QUOTED_ID", pattern="`[^`]+`", is_regex=True, line_number=15, alias="NAME"),
        TokenDefinition(name="LESS_EQUALS", pattern="<=", is_regex=False, line_number=17),
        TokenDefinition(name="GREATER_EQUALS", pattern=">=", is_regex=False, line_number=18),
        TokenDefinition(name="NOT_EQUALS", pattern="!=", is_regex=False, line_number=19),
        TokenDefinition(name="NEQ_ANSI", pattern="<>", is_regex=False, line_number=20, alias="NOT_EQUALS"),
        TokenDefinition(name="EQUALS", pattern="=", is_regex=False, line_number=22),
        TokenDefinition(name="LESS_THAN", pattern="<", is_regex=False, line_number=23),
        TokenDefinition(name="GREATER_THAN", pattern=">", is_regex=False, line_number=24),
        TokenDefinition(name="PLUS", pattern="+", is_regex=False, line_number=25),
        TokenDefinition(name="MINUS", pattern="-", is_regex=False, line_number=26),
        TokenDefinition(name="STAR", pattern="*", is_regex=False, line_number=27),
        TokenDefinition(name="SLASH", pattern="/", is_regex=False, line_number=28),
        TokenDefinition(name="PERCENT", pattern="%", is_regex=False, line_number=29),
        TokenDefinition(name="LPAREN", pattern="(", is_regex=False, line_number=31),
        TokenDefinition(name="RPAREN", pattern=")", is_regex=False, line_number=32),
        TokenDefinition(name="COMMA", pattern=",", is_regex=False, line_number=33),
        TokenDefinition(name="SEMICOLON", pattern=";", is_regex=False, line_number=34),
        TokenDefinition(name="DOT", pattern=".", is_regex=False, line_number=35),
    ],
    skip_definitions=[
        TokenDefinition(name="WHITESPACE", pattern="[ \\t\\r\\n]+", is_regex=True, line_number=95),
        TokenDefinition(name="LINE_COMMENT", pattern="--[^\\n]*", is_regex=True, line_number=96),
        TokenDefinition(name="BLOCK_COMMENT", pattern="\\x2f\\*([^*]|\\*[^\\x2f])*\\*\\x2f", is_regex=True, line_number=97),
    ],
)
