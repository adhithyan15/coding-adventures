package = "coding-adventures-toml-lexer"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "TOML lexer — tokenizes TOML text using the grammar-driven infrastructure",
    detailed = [[
        A thin wrapper around the grammar-driven GrammarLexer from the
        coding-adventures-lexer package.  Loads the shared toml.tokens
        grammar file and delegates all tokenization to the GrammarLexer,
        producing a flat stream of typed tokens including BARE_KEY,
        BASIC_STRING, LITERAL_STRING, ML_BASIC_STRING, ML_LITERAL_STRING,
        INTEGER, FLOAT, TRUE, FALSE, OFFSET_DATETIME, LOCAL_DATETIME,
        LOCAL_DATE, LOCAL_TIME, EQUALS, DOT, COMMA, LBRACKET, RBRACKET,
        LBRACE, RBRACE, and EOF.  Horizontal whitespace and comments are
        consumed silently; newlines are significant in TOML.
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
        ["coding_adventures.toml_lexer"] = "src/coding_adventures/toml_lexer/init.lua",
    },
}
