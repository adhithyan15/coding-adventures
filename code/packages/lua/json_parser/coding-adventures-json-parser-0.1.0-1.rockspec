-- Rockspec for coding-adventures-json-parser
-- ============================================
--
-- This rockspec declares the JSON parser as a publishable LuaRocks package.
-- It lives in the coding-adventures monorepo at:
--   code/packages/lua/json_parser/
--
-- The package depends on:
--   - coding-adventures-json-lexer   — tokenizes JSON source text
--   - coding-adventures-parser       — grammar-driven GrammarParser engine
--   - coding-adventures-grammar-tools — parses the .grammar specification
--   - coding-adventures-directed-graph — required by grammar_tools
--   - coding-adventures-state-machine  — required by grammar_tools / lexer

package = "coding-adventures-json-parser"
version = "0.1.0-1"

source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}

description = {
    summary  = "JSON parser — builds AST from JSON text using grammar-driven engine",
    detailed = [[
        A grammar-driven JSON parser built on the coding-adventures parser
        infrastructure. Tokenizes JSON source with json_lexer, loads the
        json.grammar specification with grammar_tools, and produces an Abstract
        Syntax Tree (AST) using the GrammarParser from the parser package.
    ]],
    homepage = "https://github.com/adhithyan15/coding-adventures",
    license  = "MIT",
}

dependencies = {
    "lua >= 5.4",
    "coding-adventures-json-lexer >= 0.1.0",
    "coding-adventures-parser >= 0.1.0",
    "coding-adventures-grammar-tools >= 0.1.0",
    "coding-adventures-directed-graph >= 0.1.0",
    "coding-adventures-state-machine >= 0.1.0",
}

build = {
    type    = "builtin",
    modules = {
        ["coding_adventures.json_parser"] =
            "src/coding_adventures/json_parser/init.lua",
    },
}
