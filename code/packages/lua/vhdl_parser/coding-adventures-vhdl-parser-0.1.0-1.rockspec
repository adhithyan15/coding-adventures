-- Rockspec for coding-adventures-vhdl-parser
-- ============================================
--
-- This rockspec declares the VHDL parser as a publishable LuaRocks package.
-- It lives in the coding-adventures monorepo at:
--   code/packages/lua/vhdl_parser/
--
-- The package depends on:
--   - coding-adventures-vhdl-lexer    — tokenizes VHDL source text
--   - coding-adventures-parser        — grammar-driven GrammarParser engine
--   - coding-adventures-grammar-tools — parses the .grammar specification
--   - coding-adventures-directed-graph — required by grammar_tools
--   - coding-adventures-state-machine  — required by grammar_tools / lexer

package = "coding-adventures-vhdl-parser"
version = "0.1.0-1"

source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}

description = {
    summary  = "VHDL parser — builds AST from VHDL HDL using grammar-driven engine",
    detailed = [[
        A grammar-driven VHDL (IEEE 1076-2008) parser built on the coding-adventures
        parser infrastructure. Tokenizes VHDL source with vhdl_lexer, loads the
        vhdl.grammar specification with grammar_tools, and produces an Abstract
        Syntax Tree (AST) using the GrammarParser from the parser package.
        Covers the synthesizable subset: design files with context clauses (library/use),
        entity declarations with generics and ports, architecture bodies with signal
        declarations, concurrent statements (processes, signal assignments, component
        instantiations, generate statements), sequential statements (if/case/loop),
        and full VHDL expression grammar with keyword operators (and, or, xor, etc.).
    ]],
    homepage = "https://github.com/adhithyan15/coding-adventures",
    license  = "MIT",
}

dependencies = {
    "lua >= 5.4",
    "coding-adventures-vhdl-lexer >= 0.1.0",
    "coding-adventures-parser >= 0.1.0",
    "coding-adventures-grammar-tools >= 0.1.0",
    "coding-adventures-directed-graph >= 0.1.0",
    "coding-adventures-state-machine >= 0.1.0",
}

build = {
    type    = "builtin",
    modules = {
        ["coding_adventures.vhdl_parser"] =
            "src/coding_adventures/vhdl_parser/init.lua",
    },
}
