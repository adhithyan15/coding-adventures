package = "coding-adventures-lexer"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Tokenizer breaking source code into tokens: keywords, identifiers, numbers, operators",
    detailed = [[
        The lexer package provides two tokenizers:

        1. Lexer (hand-written) -- character-by-character tokenizer with a
           DFA-driven dispatch loop. Supports identifiers, numbers, strings
           with escape sequences, operators, delimiters, and keywords.

        2. GrammarLexer (grammar-driven) -- regex-based tokenizer driven by
           a TokenGrammar table. Supports skip patterns, type aliases,
           reserved keywords, indentation mode (INDENT/DEDENT), pattern
           groups with stackable transitions, and on-token callbacks.

        Both produce the same Token objects with type, value, line, and
        column information.
    ]],
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-grammar-tools >= 0.1.0",
    "coding-adventures-state-machine >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.lexer"] = "src/coding_adventures/lexer/init.lua",
    },
}
