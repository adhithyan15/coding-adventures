package = "coding-adventures-starlark-lexer"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Starlark lexer — tokenizes Starlark source using the grammar-driven infrastructure",
    detailed = [[
        A thin wrapper around the grammar-driven GrammarLexer from the
        coding-adventures-lexer package.  Loads the shared starlark.tokens
        grammar file and delegates all tokenization to the GrammarLexer,
        producing a flat stream of typed tokens (NAME, INT, FLOAT, STRING,
        keyword tokens, operator tokens, delimiter tokens, and indentation
        tokens INDENT/DEDENT/NEWLINE, EOF).
        Whitespace and comments are consumed silently via starlark.tokens
        skip rules. Indentation tracking follows Python-style rules as
        specified by mode: indentation in starlark.tokens.
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
        ["coding_adventures.starlark_lexer"] = "src/coding_adventures/starlark_lexer/init.lua",
    },
}
