-- Rockspec for coding-adventures-csharp-parser
-- ==============================================
--
-- This rockspec declares the C# parser as a publishable LuaRocks package.
-- It lives in the coding-adventures monorepo at:
--   code/packages/lua/csharp_parser/

package = "coding-adventures-csharp-parser"
version = "0.1.0-1"

source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}

description = {
    summary  = "C# parser — builds AST from C# text using grammar-driven engine",
    detailed = [[
        A grammar-driven C# parser built on the coding-adventures parser
        infrastructure. Tokenizes C# source with csharp_lexer, loads the
        csharp<version>.grammar specification with grammar_tools, and produces an
        Abstract Syntax Tree (AST) using the GrammarParser from the parser package.
        Handles variable declarations, assignments, arithmetic expressions with
        correct operator precedence, and parenthesized groups.
        Supports all 12 C# language versions (1.0 through 12.0).
    ]],
    homepage = "https://github.com/adhithyan15/coding-adventures",
    license  = "MIT",
}

dependencies = {
    "lua >= 5.4",
    "coding-adventures-csharp-lexer >= 0.1.0",
    "coding-adventures-parser >= 0.1.0",
    "coding-adventures-grammar-tools >= 0.1.0",
    "coding-adventures-directed-graph >= 0.1.0",
    "coding-adventures-state-machine >= 0.1.0",
}

build = {
    type    = "builtin",
    modules = {
        ["coding_adventures.csharp_parser"] =
            "src/coding_adventures/csharp_parser/init.lua",
    },
}
