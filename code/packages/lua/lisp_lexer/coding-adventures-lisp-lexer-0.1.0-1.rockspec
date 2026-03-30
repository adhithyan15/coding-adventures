package = "coding-adventures-lisp-lexer"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Lisp tokenizer — tokenizes Lisp/Scheme source using the grammar-driven infrastructure",
    detailed = [[
        A thin wrapper around the grammar-driven GrammarLexer from the
        coding-adventures-lexer package.  Loads the shared lisp.tokens
        grammar file and delegates all tokenization to the GrammarLexer,
        producing a flat stream of typed tokens (NUMBER, SYMBOL, STRING,
        LPAREN, RPAREN, QUOTE, DOT, EOF).  Whitespace and line comments
        (;...) are consumed silently via lisp.tokens skip rules.
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
        ["coding_adventures.lisp_lexer"] = "src/coding_adventures/lisp_lexer/init.lua",
    },
}
