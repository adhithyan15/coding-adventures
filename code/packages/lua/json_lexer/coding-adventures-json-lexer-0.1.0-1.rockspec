package = "coding-adventures-json-lexer"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "JSON lexer — tokenizes JSON text using the grammar-driven infrastructure",
    detailed = [[
        A thin wrapper around the grammar-driven GrammarLexer from the
        coding-adventures-lexer package.  Loads the shared json.tokens
        grammar file and delegates all tokenization to the GrammarLexer,
        producing a flat stream of typed tokens (STRING, NUMBER, TRUE,
        FALSE, NULL, LBRACE, RBRACE, LBRACKET, RBRACKET, COLON, COMMA,
        EOF).  Whitespace is consumed silently via json.tokens skip rules.
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
        ["coding_adventures.json_lexer"] = "src/coding_adventures/json_lexer/init.lua",
    },
}
