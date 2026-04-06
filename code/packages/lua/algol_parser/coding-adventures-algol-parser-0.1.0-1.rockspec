-- Rockspec for coding-adventures-algol-parser
-- =============================================
--
-- This rockspec declares the ALGOL 60 parser as a publishable LuaRocks package.
-- It lives in the coding-adventures monorepo at:
--   code/packages/lua/algol_parser/
--
-- The package depends on:
--   - coding-adventures-algol-lexer    — tokenizes ALGOL 60 source text
--   - coding-adventures-parser         — grammar-driven GrammarParser engine
--   - coding-adventures-grammar-tools  — parses the .grammar specification
--   - coding-adventures-directed-graph — required by grammar_tools
--   - coding-adventures-state-machine  — required by grammar_tools / lexer

package = "coding-adventures-algol-parser"
version = "0.1.0-1"

source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}

description = {
    summary  = "ALGOL 60 parser — builds AST from ALGOL 60 text using grammar-driven engine",
    detailed = [[
        A grammar-driven ALGOL 60 parser built on the coding-adventures parser
        infrastructure. Tokenizes ALGOL 60 source with algol_lexer, loads the
        algol.grammar specification with grammar_tools, and produces an Abstract
        Syntax Tree (AST) using the GrammarParser from the parser package.
        Supports the full ALGOL 60 grammar: blocks, declarations (type, array,
        switch, procedure), all statement forms (assignment, conditional, for,
        goto, compound), and expressions (arithmetic, boolean, designational).
    ]],
    homepage = "https://github.com/adhithyan15/coding-adventures",
    license  = "MIT",
}

dependencies = {
    "lua >= 5.4",
    "coding-adventures-algol-lexer >= 0.1.0",
    "coding-adventures-parser >= 0.1.0",
    "coding-adventures-grammar-tools >= 0.1.0",
    "coding-adventures-directed-graph >= 0.1.0",
    "coding-adventures-state-machine >= 0.1.0",
}

build = {
    type    = "builtin",
    modules = {
        ["coding_adventures.algol_parser"] =
            "src/coding_adventures/algol_parser/init.lua",
    },
}
