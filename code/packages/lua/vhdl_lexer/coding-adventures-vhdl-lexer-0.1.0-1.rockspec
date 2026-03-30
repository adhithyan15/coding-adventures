package = "coding-adventures-vhdl-lexer"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "VHDL lexer — tokenizes VHDL source using the grammar-driven infrastructure",
    detailed = [[
        A thin wrapper around the grammar-driven GrammarLexer from the
        coding-adventures-lexer package.  Loads the shared vhdl.tokens
        grammar file and delegates all tokenization to the GrammarLexer,
        producing a flat stream of typed tokens (NAME, keyword tokens,
        number/literal tokens, operator tokens, delimiter tokens, EOF).
        VHDL is case-insensitive; the grammar lowercases all input before
        matching. Whitespace and comments are consumed silently.
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
        ["coding_adventures.vhdl_lexer"] = "src/coding_adventures/vhdl_lexer/init.lua",
    },
}
