-- Rockspec for coding-adventures-algol-lexer
-- ============================================
--
-- This rockspec declares the ALGOL 60 lexer as a publishable LuaRocks package.
-- It lives in the coding-adventures monorepo at:
--   code/packages/lua/algol_lexer/
--
-- The package depends on:
--   - coding-adventures-grammar-tools — parses the .tokens specification
--   - coding-adventures-lexer         — provides the GrammarLexer engine
--   - coding-adventures-directed-graph — required internally by grammar_tools
--   - coding-adventures-state-machine  — required internally by lexer / grammar_tools

package = "coding-adventures-algol-lexer"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "ALGOL 60 lexer — tokenizes ALGOL 60 source text using the grammar-driven infrastructure",
    detailed = [[
        A thin wrapper around the grammar-driven GrammarLexer from the
        coding-adventures-lexer package.  Loads the shared algol.tokens
        grammar file and delegates all tokenization to the GrammarLexer,
        producing a flat stream of typed tokens (BEGIN, END, IF, THEN, ELSE,
        FOR, DO, STEP, UNTIL, WHILE, GOTO, SWITCH, PROCEDURE, INTEGER, REAL,
        BOOLEAN, STRING, ARRAY, VALUE, TRUE, FALSE, NOT, AND, OR, IMPL, EQV,
        DIV, MOD, IDENT, INTEGER_LIT, REAL_LIT, STRING_LIT, operators,
        delimiters, and EOF).  Whitespace and comments are consumed silently.
    ]],
    homepage = "https://github.com/adhithyan15/coding-adventures",
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
        ["coding_adventures.algol_lexer"] = "src/coding_adventures/algol_lexer/init.lua",
    },
}
