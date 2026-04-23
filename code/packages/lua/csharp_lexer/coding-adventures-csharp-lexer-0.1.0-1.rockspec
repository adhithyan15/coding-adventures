package = "coding-adventures-csharp-lexer"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "C# lexer — tokenizes C# source using the grammar-driven infrastructure",
    detailed = [[
        A thin wrapper around the grammar-driven GrammarLexer from the
        coding-adventures-lexer package.  Loads the shared csharp/csharp<version>.tokens
        grammar file and delegates all tokenization to the GrammarLexer,
        producing a flat stream of typed tokens (NAME, NUMBER, STRING,
        keyword tokens, operator tokens, delimiter tokens, EOF).
        Whitespace is consumed silently via grammar skip rules.
        Supports all 12 C# language versions (1.0 through 12.0).
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
        ["coding_adventures.csharp_lexer"] = "src/coding_adventures/csharp_lexer/init.lua",
    },
}
