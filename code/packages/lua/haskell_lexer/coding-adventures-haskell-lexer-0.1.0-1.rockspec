package = "coding-adventures-haskell-lexer"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Haskell lexer — tokenizes Haskell source using the grammar-driven infrastructure",
    detailed = [[
        A thin wrapper around the grammar-driven GrammarLexer from the
        coding-adventures-lexer package.  Loads the shared haskell/haskell<version>.tokens
        grammar file and delegates all tokenization to the GrammarLexer,
        producing a flat stream of typed tokens (NAME, NUMBER, STRING,
        keyword tokens, operator tokens, delimiter tokens, EOF).
        Whitespace is consumed silently via grammar skip rules.
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
        ["coding_adventures.haskell_lexer"] = "src/coding_adventures/haskell_lexer/init.lua",
        ["coding_adventures.haskell_lexer._grammar_1_0"] = "src/coding_adventures/haskell_lexer/_grammar_1_0.lua",
        ["coding_adventures.haskell_lexer._grammar_1_1"] = "src/coding_adventures/haskell_lexer/_grammar_1_1.lua",
        ["coding_adventures.haskell_lexer._grammar_1_2"] = "src/coding_adventures/haskell_lexer/_grammar_1_2.lua",
        ["coding_adventures.haskell_lexer._grammar_1_3"] = "src/coding_adventures/haskell_lexer/_grammar_1_3.lua",
        ["coding_adventures.haskell_lexer._grammar_1_4"] = "src/coding_adventures/haskell_lexer/_grammar_1_4.lua",
        ["coding_adventures.haskell_lexer._grammar_98"] = "src/coding_adventures/haskell_lexer/_grammar_98.lua",
        ["coding_adventures.haskell_lexer._grammar_2010"] = "src/coding_adventures/haskell_lexer/_grammar_2010.lua",
    },
}
