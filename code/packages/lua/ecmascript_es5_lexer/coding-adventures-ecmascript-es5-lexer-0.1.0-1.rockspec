package = "coding-adventures-ecmascript-es5-lexer"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "ECMAScript 5 (2009) lexer — tokenizes ES5 source using the grammar-driven infrastructure",
    detailed = [[
        A thin wrapper around the grammar-driven GrammarLexer from the
        coding-adventures-lexer package.  Loads the shared ecmascript/es5.tokens
        grammar file and delegates all tokenization to the GrammarLexer.
        ES5 adds the debugger keyword over ES3 while keeping the same
        operator and literal set.
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
        ["coding_adventures.ecmascript_es5_lexer"] = "src/coding_adventures/ecmascript_es5_lexer/init.lua",
    },
}
