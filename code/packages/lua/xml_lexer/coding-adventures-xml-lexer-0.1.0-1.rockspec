package = "coding-adventures-xml-lexer"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "XML lexer — tokenizes XML text using the grammar-driven infrastructure",
    detailed = [[
        A thin wrapper around the grammar-driven GrammarLexer from the
        coding-adventures-lexer package.  Loads the shared xml.tokens
        grammar file and registers group-switching callbacks to handle
        XML's context-sensitive lexical rules (tags, attributes, comments,
        CDATA sections, processing instructions).

        Token types emitted: TEXT, ENTITY_REF, CHAR_REF, COMMENT_START,
        CDATA_START, PI_START, CLOSE_TAG_START, OPEN_TAG_START, TAG_NAME,
        ATTR_EQUALS, ATTR_VALUE, TAG_CLOSE, SELF_CLOSE, SLASH,
        COMMENT_TEXT, COMMENT_END, CDATA_TEXT, CDATA_END, PI_TARGET,
        PI_TEXT, PI_END, EOF.
    ]],
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-grammar-tools >= 0.1.0",
    "coding-adventures-lexer >= 0.1.0",
    "coding-adventures-directed-graph >= 0.1.0",
    "coding-adventures-state-machine >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.xml_lexer"] = "src/coding_adventures/xml_lexer/init.lua",
    },
}
