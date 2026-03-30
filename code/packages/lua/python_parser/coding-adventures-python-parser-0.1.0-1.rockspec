-- Rockspec for coding-adventures-python-parser
-- ==============================================
--
-- This rockspec declares the Python parser as a publishable LuaRocks package.
-- It lives in the coding-adventures monorepo at:
--   code/packages/lua/python_parser/
--
-- The package depends on:
--   - coding-adventures-python-lexer  — tokenizes Python source text
--   - coding-adventures-parser        — grammar-driven GrammarParser engine
--   - coding-adventures-grammar-tools — parses the .grammar specification
--   - coding-adventures-directed-graph — required by grammar_tools
--   - coding-adventures-state-machine  — required by grammar_tools / lexer

package = "coding-adventures-python-parser"
version = "0.1.0-1"

source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}

description = {
    summary  = "Python parser — builds AST from Python text using grammar-driven engine",
    detailed = [[
        A grammar-driven Python parser built on the coding-adventures parser
        infrastructure. Tokenizes Python source with python_lexer, loads the
        python.grammar specification with grammar_tools, and produces an Abstract
        Syntax Tree (AST) using the GrammarParser from the parser package.
        Handles assignments, arithmetic expressions with correct operator precedence,
        parenthesized groups, and expression statements.
    ]],
    homepage = "https://github.com/adhithyan15/coding-adventures",
    license  = "MIT",
}

dependencies = {
    "lua >= 5.4",
    "coding-adventures-python-lexer >= 0.1.0",
    "coding-adventures-parser >= 0.1.0",
    "coding-adventures-grammar-tools >= 0.1.0",
    "coding-adventures-directed-graph >= 0.1.0",
    "coding-adventures-state-machine >= 0.1.0",
}

build = {
    type    = "builtin",
    modules = {
        ["coding_adventures.python_parser"] =
            "src/coding_adventures/python_parser/init.lua",
    },
}
