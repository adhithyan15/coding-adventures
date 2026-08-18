package = "coding-adventures-java-lexer"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Java lexer — tokenizes Java source using the grammar-driven infrastructure",
    detailed = [[
        A thin wrapper around the grammar-driven GrammarLexer from the
        coding-adventures-lexer package.  Loads the shared java/java<version>.tokens
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
        ["coding_adventures.java_lexer"] = "src/coding_adventures/java_lexer/init.lua",
        ["coding_adventures.java_lexer._grammar_1_0"] = "src/coding_adventures/java_lexer/_grammar_1_0.lua",
        ["coding_adventures.java_lexer._grammar_1_1"] = "src/coding_adventures/java_lexer/_grammar_1_1.lua",
        ["coding_adventures.java_lexer._grammar_1_4"] = "src/coding_adventures/java_lexer/_grammar_1_4.lua",
        ["coding_adventures.java_lexer._grammar_5"] = "src/coding_adventures/java_lexer/_grammar_5.lua",
        ["coding_adventures.java_lexer._grammar_7"] = "src/coding_adventures/java_lexer/_grammar_7.lua",
        ["coding_adventures.java_lexer._grammar_8"] = "src/coding_adventures/java_lexer/_grammar_8.lua",
        ["coding_adventures.java_lexer._grammar_10"] = "src/coding_adventures/java_lexer/_grammar_10.lua",
        ["coding_adventures.java_lexer._grammar_14"] = "src/coding_adventures/java_lexer/_grammar_14.lua",
        ["coding_adventures.java_lexer._grammar_17"] = "src/coding_adventures/java_lexer/_grammar_17.lua",
        ["coding_adventures.java_lexer._grammar_21"] = "src/coding_adventures/java_lexer/_grammar_21.lua",
    },
}
