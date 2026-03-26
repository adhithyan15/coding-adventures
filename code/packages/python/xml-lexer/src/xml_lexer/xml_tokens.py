# AUTO-GENERATED FILE - DO NOT EDIT
from grammar_tools.token_grammar import TokenGrammar, TokenDefinition, PatternGroup

XmlTokens = TokenGrammar(
    version=1,
    case_insensitive=False,
    case_sensitive=True,
    escape_mode="none",
    definitions=[
        TokenDefinition(name="TEXT", pattern="[^<&]+", is_regex=True, line_number=57),
        TokenDefinition(name="ENTITY_REF", pattern="&[a-zA-Z][a-zA-Z0-9]*;", is_regex=True, line_number=58),
        TokenDefinition(name="CHAR_REF", pattern="&#[0-9]+;|&#x[0-9a-fA-F]+;", is_regex=True, line_number=59),
        TokenDefinition(name="COMMENT_START", pattern="<!--", is_regex=False, line_number=60),
        TokenDefinition(name="CDATA_START", pattern="<![CDATA[", is_regex=False, line_number=61),
        TokenDefinition(name="PI_START", pattern="<?", is_regex=False, line_number=62),
        TokenDefinition(name="CLOSE_TAG_START", pattern="</", is_regex=False, line_number=63),
        TokenDefinition(name="OPEN_TAG_START", pattern="<", is_regex=False, line_number=64),
    ],
    skip_definitions=[
        TokenDefinition(name="WHITESPACE", pattern="[ \\t\\r\\n]+", is_regex=True, line_number=42),
    ],
    groups={
        "tag": PatternGroup(
            name="tag",
            definitions=[
                TokenDefinition(name="TAG_NAME", pattern="[a-zA-Z_][a-zA-Z0-9_:.-]*", is_regex=True, line_number=79),
                TokenDefinition(name="ATTR_EQUALS", pattern="=", is_regex=False, line_number=80),
                TokenDefinition(name="ATTR_VALUE_DQ", pattern="\"[^\"]*\"", is_regex=True, line_number=81, alias="ATTR_VALUE"),
                TokenDefinition(name="ATTR_VALUE_SQ", pattern="'[^']*'", is_regex=True, line_number=82, alias="ATTR_VALUE"),
                TokenDefinition(name="TAG_CLOSE", pattern=">", is_regex=False, line_number=83),
                TokenDefinition(name="SELF_CLOSE", pattern="/>", is_regex=False, line_number=84),
                TokenDefinition(name="SLASH", pattern="/", is_regex=False, line_number=85),
            ],
        ),
        "comment": PatternGroup(
            name="comment",
            definitions=[
                TokenDefinition(name="COMMENT_TEXT", pattern="([^-]|-(?!->))+", is_regex=True, line_number=99),
                TokenDefinition(name="COMMENT_END", pattern="-->", is_regex=False, line_number=100),
            ],
        ),
        "cdata": PatternGroup(
            name="cdata",
            definitions=[
                TokenDefinition(name="CDATA_TEXT", pattern="([^\\]]|\\](?!\\]>))+", is_regex=True, line_number=113),
                TokenDefinition(name="CDATA_END", pattern="]]>", is_regex=False, line_number=114),
            ],
        ),
        "pi": PatternGroup(
            name="pi",
            definitions=[
                TokenDefinition(name="PI_TARGET", pattern="[a-zA-Z_][a-zA-Z0-9_:.-]*", is_regex=True, line_number=128),
                TokenDefinition(name="PI_TEXT", pattern="([^?]|\\?(?!>))+", is_regex=True, line_number=129),
                TokenDefinition(name="PI_END", pattern="?>", is_regex=False, line_number=130),
            ],
        ),
    },
)
