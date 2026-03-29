package = "coding-adventures-sql-lexer"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "SQL lexer — tokenizes SQL text using the grammar-driven infrastructure",
    detailed = [[
        A thin wrapper around the grammar-driven GrammarLexer from the
        coding-adventures-lexer package.  Loads the shared sql.tokens
        grammar file and delegates all tokenization to the GrammarLexer,
        producing a flat stream of typed tokens.  Keywords are
        case-insensitive (SELECT, select, and Select all produce a SELECT
        token).  Whitespace and SQL comments (-- and /* */) are consumed
        silently via sql.tokens skip rules.
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
        ["coding_adventures.sql_lexer"] = "src/coding_adventures/sql_lexer/init.lua",
    },
}
