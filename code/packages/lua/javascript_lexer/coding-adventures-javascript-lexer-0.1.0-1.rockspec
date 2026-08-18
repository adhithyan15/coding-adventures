package = "coding-adventures-javascript-lexer"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "JavaScript lexer — tokenizes JavaScript source using the grammar-driven infrastructure",
    detailed = [[
        A thin wrapper around the grammar-driven GrammarLexer from the
        coding-adventures-lexer package.  Loads the shared javascript.tokens
        grammar file and delegates all tokenization to the GrammarLexer,
        producing a flat stream of typed tokens (NAME, NUMBER, STRING,
        keyword tokens, operator tokens, delimiter tokens, EOF).
        Whitespace is consumed silently via javascript.tokens skip rules.
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
        ["coding_adventures.javascript_lexer"] = "src/coding_adventures/javascript_lexer/init.lua",
        ["coding_adventures.javascript_lexer._grammar_default"] = "src/coding_adventures/javascript_lexer/_grammar_default.lua",
        ["coding_adventures.javascript_lexer._grammar_es1"] = "src/coding_adventures/javascript_lexer/_grammar_es1.lua",
        ["coding_adventures.javascript_lexer._grammar_es3"] = "src/coding_adventures/javascript_lexer/_grammar_es3.lua",
        ["coding_adventures.javascript_lexer._grammar_es5"] = "src/coding_adventures/javascript_lexer/_grammar_es5.lua",
        ["coding_adventures.javascript_lexer._grammar_es2015"] = "src/coding_adventures/javascript_lexer/_grammar_es2015.lua",
        ["coding_adventures.javascript_lexer._grammar_es2016"] = "src/coding_adventures/javascript_lexer/_grammar_es2016.lua",
        ["coding_adventures.javascript_lexer._grammar_es2017"] = "src/coding_adventures/javascript_lexer/_grammar_es2017.lua",
        ["coding_adventures.javascript_lexer._grammar_es2018"] = "src/coding_adventures/javascript_lexer/_grammar_es2018.lua",
        ["coding_adventures.javascript_lexer._grammar_es2019"] = "src/coding_adventures/javascript_lexer/_grammar_es2019.lua",
        ["coding_adventures.javascript_lexer._grammar_es2020"] = "src/coding_adventures/javascript_lexer/_grammar_es2020.lua",
        ["coding_adventures.javascript_lexer._grammar_es2021"] = "src/coding_adventures/javascript_lexer/_grammar_es2021.lua",
        ["coding_adventures.javascript_lexer._grammar_es2022"] = "src/coding_adventures/javascript_lexer/_grammar_es2022.lua",
        ["coding_adventures.javascript_lexer._grammar_es2023"] = "src/coding_adventures/javascript_lexer/_grammar_es2023.lua",
        ["coding_adventures.javascript_lexer._grammar_es2024"] = "src/coding_adventures/javascript_lexer/_grammar_es2024.lua",
        ["coding_adventures.javascript_lexer._grammar_es2025"] = "src/coding_adventures/javascript_lexer/_grammar_es2025.lua",
    },
}
