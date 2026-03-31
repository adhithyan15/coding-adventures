-- Rockspec for coding-adventures-typescript-parser
-- ==================================================
--
-- This rockspec declares the TypeScript parser as a publishable LuaRocks package.
-- It lives in the coding-adventures monorepo at:
--   code/packages/lua/typescript_parser/
--
-- The package depends on:
--   - coding-adventures-typescript-lexer — tokenizes TypeScript source text
--   - coding-adventures-parser           — grammar-driven GrammarParser engine
--   - coding-adventures-grammar-tools    — parses the .grammar specification
--   - coding-adventures-directed-graph   — required by grammar_tools
--   - coding-adventures-state-machine    — required by grammar_tools / lexer

package = "coding-adventures-typescript-parser"
version = "0.1.0-1"

source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}

description = {
    summary  = "TypeScript parser — builds AST from TypeScript text using grammar-driven engine",
    detailed = [[
        A grammar-driven TypeScript parser built on the coding-adventures parser
        infrastructure. Tokenizes TypeScript source with typescript_lexer, loads the
        typescript.grammar specification with grammar_tools, and produces an Abstract
        Syntax Tree (AST) using the GrammarParser from the parser package.
        Handles variable declarations (var/let/const), assignments, arithmetic
        expressions with correct operator precedence, and parenthesized groups.
        TypeScript-specific keywords (interface, type, enum, abstract, etc.) are
        recognized during lexing and handled as KEYWORD tokens in the grammar.
    ]],
    homepage = "https://github.com/adhithyan15/coding-adventures",
    license  = "MIT",
}

dependencies = {
    "lua >= 5.4",
    "coding-adventures-typescript-lexer >= 0.1.0",
    "coding-adventures-parser >= 0.1.0",
    "coding-adventures-grammar-tools >= 0.1.0",
    "coding-adventures-directed-graph >= 0.1.0",
    "coding-adventures-state-machine >= 0.1.0",
}

build = {
    type    = "builtin",
    modules = {
        ["coding_adventures.typescript_parser"] =
            "src/coding_adventures/typescript_parser/init.lua",
    },
}
