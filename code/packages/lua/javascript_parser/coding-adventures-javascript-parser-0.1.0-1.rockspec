-- Rockspec for coding-adventures-javascript-parser
-- ==================================================
--
-- This rockspec declares the JavaScript parser as a publishable LuaRocks package.
-- It lives in the coding-adventures monorepo at:
--   code/packages/lua/javascript_parser/
--
-- The package depends on:
--   - coding-adventures-javascript-lexer — tokenizes JavaScript source text
--   - coding-adventures-parser           — grammar-driven GrammarParser engine
--   - coding-adventures-grammar-tools    — parses the .grammar specification
--   - coding-adventures-directed-graph   — required by grammar_tools
--   - coding-adventures-state-machine    — required by grammar_tools / lexer

package = "coding-adventures-javascript-parser"
version = "0.1.0-1"

source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}

description = {
    summary  = "JavaScript parser — builds AST from JavaScript text using grammar-driven engine",
    detailed = [[
        A grammar-driven JavaScript parser built on the coding-adventures parser
        infrastructure. Tokenizes JavaScript source with javascript_lexer, loads the
        javascript.grammar specification with grammar_tools, and produces an Abstract
        Syntax Tree (AST) using the GrammarParser from the parser package.
        Handles variable declarations (var/let/const), assignments, arithmetic
        expressions with correct operator precedence, and parenthesized groups.
    ]],
    homepage = "https://github.com/adhithyan15/coding-adventures",
    license  = "MIT",
}

dependencies = {
    "lua >= 5.4",
    "coding-adventures-javascript-lexer >= 0.1.0",
    "coding-adventures-parser >= 0.1.0",
    "coding-adventures-grammar-tools >= 0.1.0",
    "coding-adventures-directed-graph >= 0.1.0",
    "coding-adventures-state-machine >= 0.1.0",
}

build = {
    type    = "builtin",
    modules = {
        ["coding_adventures.javascript_parser"] =
            "src/coding_adventures/javascript_parser/init.lua",
        ["coding_adventures.javascript_parser._grammar_default"] =
            "src/coding_adventures/javascript_parser/_grammar_default.lua",
        ["coding_adventures.javascript_parser._grammar_es1"] =
            "src/coding_adventures/javascript_parser/_grammar_es1.lua",
        ["coding_adventures.javascript_parser._grammar_es3"] =
            "src/coding_adventures/javascript_parser/_grammar_es3.lua",
        ["coding_adventures.javascript_parser._grammar_es5"] =
            "src/coding_adventures/javascript_parser/_grammar_es5.lua",
        ["coding_adventures.javascript_parser._grammar_es2015"] =
            "src/coding_adventures/javascript_parser/_grammar_es2015.lua",
        ["coding_adventures.javascript_parser._grammar_es2016"] =
            "src/coding_adventures/javascript_parser/_grammar_es2016.lua",
        ["coding_adventures.javascript_parser._grammar_es2017"] =
            "src/coding_adventures/javascript_parser/_grammar_es2017.lua",
        ["coding_adventures.javascript_parser._grammar_es2018"] =
            "src/coding_adventures/javascript_parser/_grammar_es2018.lua",
        ["coding_adventures.javascript_parser._grammar_es2019"] =
            "src/coding_adventures/javascript_parser/_grammar_es2019.lua",
        ["coding_adventures.javascript_parser._grammar_es2020"] =
            "src/coding_adventures/javascript_parser/_grammar_es2020.lua",
        ["coding_adventures.javascript_parser._grammar_es2021"] =
            "src/coding_adventures/javascript_parser/_grammar_es2021.lua",
        ["coding_adventures.javascript_parser._grammar_es2022"] =
            "src/coding_adventures/javascript_parser/_grammar_es2022.lua",
        ["coding_adventures.javascript_parser._grammar_es2023"] =
            "src/coding_adventures/javascript_parser/_grammar_es2023.lua",
        ["coding_adventures.javascript_parser._grammar_es2024"] =
            "src/coding_adventures/javascript_parser/_grammar_es2024.lua",
        ["coding_adventures.javascript_parser._grammar_es2025"] =
            "src/coding_adventures/javascript_parser/_grammar_es2025.lua",
    },
}
