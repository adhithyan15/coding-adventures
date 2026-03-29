-- Rockspec for coding-adventures-verilog-parser
-- ===============================================
--
-- This rockspec declares the Verilog parser as a publishable LuaRocks package.
-- It lives in the coding-adventures monorepo at:
--   code/packages/lua/verilog_parser/
--
-- The package depends on:
--   - coding-adventures-verilog-lexer — tokenizes Verilog source text
--   - coding-adventures-parser        — grammar-driven GrammarParser engine
--   - coding-adventures-grammar-tools — parses the .grammar specification
--   - coding-adventures-directed-graph — required by grammar_tools
--   - coding-adventures-state-machine  — required by grammar_tools / lexer

package = "coding-adventures-verilog-parser"
version = "0.1.0-1"

source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}

description = {
    summary  = "Verilog parser — builds AST from Verilog HDL using grammar-driven engine",
    detailed = [[
        A grammar-driven Verilog (IEEE 1364-2005) parser built on the coding-adventures
        parser infrastructure. Tokenizes Verilog source with verilog_lexer, loads the
        verilog.grammar specification with grammar_tools, and produces an Abstract
        Syntax Tree (AST) using the GrammarParser from the parser package.
        Covers the synthesizable subset: module declarations, port lists, wire/reg
        declarations, continuous assignments, always blocks, if/case/for statements,
        module instantiation, generate blocks, functions, tasks, and full expression
        grammar with operator precedence (ternary, logical, bitwise, arithmetic).
    ]],
    homepage = "https://github.com/adhithyan15/coding-adventures",
    license  = "MIT",
}

dependencies = {
    "lua >= 5.4",
    "coding-adventures-verilog-lexer >= 0.1.0",
    "coding-adventures-parser >= 0.1.0",
    "coding-adventures-grammar-tools >= 0.1.0",
    "coding-adventures-directed-graph >= 0.1.0",
    "coding-adventures-state-machine >= 0.1.0",
}

build = {
    type    = "builtin",
    modules = {
        ["coding_adventures.verilog_parser"] =
            "src/coding_adventures/verilog_parser/init.lua",
    },
}
