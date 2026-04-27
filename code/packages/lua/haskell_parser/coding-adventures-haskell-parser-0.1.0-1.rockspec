-- Rockspec for coding-adventures-haskell-parser
-- ============================================
--
-- This rockspec declares the Haskell parser as a publishable LuaRocks package.
-- It lives in the coding-adventures monorepo at:
--   code/packages/lua/haskell_parser/

package = "coding-adventures-haskell-parser"
version = "0.1.0-1"

source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}

description = {
    summary  = "Haskell parser — builds AST from Haskell text using grammar-driven engine",
    detailed = [[
        A grammar-driven Haskell parser built on the coding-adventures parser
        infrastructure. Tokenizes Haskell source with haskell_lexer, loads the
        haskell<version>.grammar specification with grammar_tools, and produces an
        Abstract Syntax Tree (AST) using the GrammarParser from the parser package.
        Handles variable declarations, assignments, arithmetic expressions with
        correct operator precedence, and parenthesized groups.
    ]],
    homepage = "https://github.com/adhithyan15/coding-adventures",
    license  = "MIT",
}

dependencies = {
    "lua >= 5.4",
    "coding-adventures-haskell-lexer >= 0.1.0",
    "coding-adventures-parser >= 0.1.0",
    "coding-adventures-grammar-tools >= 0.1.0",
    "coding-adventures-directed-graph >= 0.1.0",
    "coding-adventures-state-machine >= 0.1.0",
}

build = {
    type    = "builtin",
    modules = {
        ["coding_adventures.haskell_parser"] =
            "src/coding_adventures/haskell_parser/init.lua",
    },
}
