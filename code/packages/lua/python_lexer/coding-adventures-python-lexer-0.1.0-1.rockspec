package = "coding-adventures-python-lexer"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Python lexer — tokenizes Python source using the grammar-driven infrastructure",
    detailed = [[
        A thin wrapper around the grammar-driven GrammarLexer from the
        coding-adventures-lexer package.  Loads the shared python.tokens
        grammar file and delegates all tokenization to the GrammarLexer,
        producing a flat stream of typed tokens (NAME, NUMBER, STRING,
        keyword tokens, operator tokens, delimiter tokens, EOF).
        Whitespace is consumed silently via python.tokens skip rules.
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
        ["coding_adventures.python_lexer"] = "src/coding_adventures/python_lexer/init.lua",
        ["coding_adventures.python_lexer._grammar_2_7"] = "src/coding_adventures/python_lexer/_grammar_2_7.lua",
        ["coding_adventures.python_lexer._grammar_3_0"] = "src/coding_adventures/python_lexer/_grammar_3_0.lua",
        ["coding_adventures.python_lexer._grammar_3_6"] = "src/coding_adventures/python_lexer/_grammar_3_6.lua",
        ["coding_adventures.python_lexer._grammar_3_8"] = "src/coding_adventures/python_lexer/_grammar_3_8.lua",
        ["coding_adventures.python_lexer._grammar_3_10"] = "src/coding_adventures/python_lexer/_grammar_3_10.lua",
        ["coding_adventures.python_lexer._grammar_3_12"] = "src/coding_adventures/python_lexer/_grammar_3_12.lua",
    },
}
