-- Rockspec for coding-adventures-toml-parser
-- ============================================
--
-- This rockspec declares the TOML parser as a publishable LuaRocks package.
-- It lives in the coding-adventures monorepo at:
--   code/packages/lua/toml_parser/

package = "coding-adventures-toml-parser"
version = "0.1.0-1"

source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}

description = {
    summary  = "TOML parser — builds AST from TOML text using grammar-driven engine",
    detailed = [[
        A grammar-driven TOML parser built on the coding-adventures parser
        infrastructure. Tokenizes TOML source with toml_lexer, loads the
        toml.grammar specification with grammar_tools, and produces an Abstract
        Syntax Tree (AST) using the GrammarParser from the parser package.
    ]],
    homepage = "https://github.com/adhithyan15/coding-adventures",
    license  = "MIT",
}

dependencies = {
    "lua >= 5.4",
    "coding-adventures-toml-lexer >= 0.1.0",
    "coding-adventures-parser >= 0.1.0",
    "coding-adventures-grammar-tools >= 0.1.0",
    "coding-adventures-directed-graph >= 0.1.0",
    "coding-adventures-state-machine >= 0.1.0",
}

build = {
    type    = "builtin",
    modules = {
        ["coding_adventures.toml_parser"] =
            "src/coding_adventures/toml_parser/init.lua",
    },
}
