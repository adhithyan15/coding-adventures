-- Rockspec for coding-adventures-dartmouth-basic-parser
-- =======================================================
--
-- This rockspec declares the Dartmouth BASIC parser as a publishable
-- LuaRocks package. It lives in the coding-adventures monorepo at:
--   code/packages/lua/dartmouth_basic_parser/
--
-- The package depends on:
--   - coding-adventures-dartmouth-basic-lexer — tokenizes BASIC source text
--   - coding-adventures-parser       — grammar-driven GrammarParser engine
--   - coding-adventures-grammar-tools — parses the .grammar specification
--   - coding-adventures-directed-graph — required by grammar_tools
--   - coding-adventures-state-machine  — required by grammar_tools / lexer

package = "coding-adventures-dartmouth-basic-parser"
version = "0.1.0-1"

source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}

description = {
    summary  = "Dartmouth BASIC parser — builds AST from BASIC source using grammar-driven engine",
    detailed = [[
        A grammar-driven 1964 Dartmouth BASIC parser built on the coding-adventures
        parser infrastructure. Tokenizes BASIC source with dartmouth_basic_lexer,
        loads the dartmouth_basic.grammar specification with grammar_tools, and
        produces an Abstract Syntax Tree (AST) using the GrammarParser from the
        parser package. Supports all 17 statement types from the 1964 Dartmouth
        BASIC specification.
    ]],
    homepage = "https://github.com/adhithyan15/coding-adventures",
    license  = "MIT",
}

dependencies = {
    "lua >= 5.4",
    "coding-adventures-dartmouth-basic-lexer >= 0.1.0",
    "coding-adventures-parser >= 0.1.0",
    "coding-adventures-grammar-tools >= 0.1.0",
    "coding-adventures-directed-graph >= 0.1.0",
    "coding-adventures-state-machine >= 0.1.0",
}

build = {
    type    = "builtin",
    modules = {
        ["coding_adventures.dartmouth_basic_parser"] =
            "src/coding_adventures/dartmouth_basic_parser/init.lua",
    },
}
