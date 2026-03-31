-- Rockspec for coding-adventures-sql-parser
-- ============================================
--
-- This rockspec declares the SQL parser as a publishable LuaRocks package.
-- It lives in the coding-adventures monorepo at:
--   code/packages/lua/sql_parser/
--
-- The package depends on:
--   - coding-adventures-sql-lexer    — tokenizes SQL source text
--   - coding-adventures-parser       — grammar-driven GrammarParser engine
--   - coding-adventures-grammar-tools — parses the .grammar specification
--   - coding-adventures-directed-graph — required by grammar_tools
--   - coding-adventures-state-machine  — required by grammar_tools / lexer

package = "coding-adventures-sql-parser"
version = "0.1.0-1"

source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}

description = {
    summary  = "SQL parser — builds AST from SQL text using grammar-driven engine",
    detailed = [[
        A grammar-driven SQL parser built on the coding-adventures parser
        infrastructure. Tokenizes SQL source with sql_lexer, loads the
        sql.grammar specification with grammar_tools, and produces an Abstract
        Syntax Tree (AST) using the GrammarParser from the parser package.
        Supports SELECT, INSERT, UPDATE, DELETE, CREATE TABLE, and DROP TABLE.
    ]],
    homepage = "https://github.com/adhithyan15/coding-adventures",
    license  = "MIT",
}

dependencies = {
    "lua >= 5.4",
    "coding-adventures-sql-lexer >= 0.1.0",
    "coding-adventures-parser >= 0.1.0",
    "coding-adventures-grammar-tools >= 0.1.0",
    "coding-adventures-directed-graph >= 0.1.0",
    "coding-adventures-state-machine >= 0.1.0",
}

build = {
    type    = "builtin",
    modules = {
        ["coding_adventures.sql_parser"] =
            "src/coding_adventures/sql_parser/init.lua",
    },
}
