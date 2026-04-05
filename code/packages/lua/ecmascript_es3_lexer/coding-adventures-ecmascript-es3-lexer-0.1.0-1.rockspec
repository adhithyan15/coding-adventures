package = "coding-adventures-ecmascript-es3-lexer"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "ECMAScript 3 (1999) lexer — tokenizes ES3 source using the grammar-driven infrastructure",
    detailed = [[
        A thin wrapper around the grammar-driven GrammarLexer from the
        coding-adventures-lexer package.  Loads the shared ecmascript/es3.tokens
        grammar file and delegates all tokenization to the GrammarLexer,
        producing a flat stream of typed tokens.  ES3 adds strict equality
        (===, !==), try/catch/finally/throw, instanceof, and regex literals
        over ES1.
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
        ["coding_adventures.ecmascript_es3_lexer"] = "src/coding_adventures/ecmascript_es3_lexer/init.lua",
    },
}
