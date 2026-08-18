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
        ["coding_adventures.csharp_lexer._grammar_1_0"] = "src/coding_adventures/csharp_lexer/_grammar_1_0.lua",
        ["coding_adventures.csharp_lexer._grammar_2_0"] = "src/coding_adventures/csharp_lexer/_grammar_2_0.lua",
        ["coding_adventures.csharp_lexer._grammar_3_0"] = "src/coding_adventures/csharp_lexer/_grammar_3_0.lua",
        ["coding_adventures.csharp_lexer._grammar_4_0"] = "src/coding_adventures/csharp_lexer/_grammar_4_0.lua",
        ["coding_adventures.csharp_lexer._grammar_5_0"] = "src/coding_adventures/csharp_lexer/_grammar_5_0.lua",
        ["coding_adventures.csharp_lexer._grammar_6_0"] = "src/coding_adventures/csharp_lexer/_grammar_6_0.lua",
        ["coding_adventures.csharp_lexer._grammar_7_0"] = "src/coding_adventures/csharp_lexer/_grammar_7_0.lua",
        ["coding_adventures.csharp_lexer._grammar_8_0"] = "src/coding_adventures/csharp_lexer/_grammar_8_0.lua",
        ["coding_adventures.csharp_lexer._grammar_9_0"] = "src/coding_adventures/csharp_lexer/_grammar_9_0.lua",
        ["coding_adventures.csharp_lexer._grammar_10_0"] = "src/coding_adventures/csharp_lexer/_grammar_10_0.lua",
        ["coding_adventures.csharp_lexer._grammar_11_0"] = "src/coding_adventures/csharp_lexer/_grammar_11_0.lua",
        ["coding_adventures.csharp_lexer._grammar_12_0"] = "src/coding_adventures/csharp_lexer/_grammar_12_0.lua",
    },
}
