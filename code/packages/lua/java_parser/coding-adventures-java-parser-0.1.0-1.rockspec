-- Rockspec for coding-adventures-java-parser
-- ============================================
--
-- This rockspec declares the Java parser as a publishable LuaRocks package.
-- It lives in the coding-adventures monorepo at:
--   code/packages/lua/java_parser/

package = "coding-adventures-java-parser"
version = "0.1.0-1"

source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}

description = {
    summary  = "Java parser — builds AST from Java text using grammar-driven engine",
    detailed = [[
        A grammar-driven Java parser built on the coding-adventures parser
        infrastructure. Tokenizes Java source with java_lexer, loads the
        java<version>.grammar specification with grammar_tools, and produces an
        Abstract Syntax Tree (AST) using the GrammarParser from the parser package.
        Handles variable declarations, assignments, arithmetic expressions with
        correct operator precedence, and parenthesized groups.
    ]],
    homepage = "https://github.com/adhithyan15/coding-adventures",
    license  = "MIT",
}

dependencies = {
    "lua >= 5.4",
    "coding-adventures-java-lexer >= 0.1.0",
    "coding-adventures-parser >= 0.1.0",
    "coding-adventures-grammar-tools >= 0.1.0",
    "coding-adventures-directed-graph >= 0.1.0",
    "coding-adventures-state-machine >= 0.1.0",
}

build = {
    type    = "builtin",
    modules = {
        ["coding_adventures.java_parser"] =
            "src/coding_adventures/java_parser/init.lua",
        ["coding_adventures.java_parser._grammar_1_0"] = "src/coding_adventures/java_parser/_grammar_1_0.lua",
        ["coding_adventures.java_parser._grammar_1_1"] = "src/coding_adventures/java_parser/_grammar_1_1.lua",
        ["coding_adventures.java_parser._grammar_1_4"] = "src/coding_adventures/java_parser/_grammar_1_4.lua",
        ["coding_adventures.java_parser._grammar_5"] = "src/coding_adventures/java_parser/_grammar_5.lua",
        ["coding_adventures.java_parser._grammar_7"] = "src/coding_adventures/java_parser/_grammar_7.lua",
        ["coding_adventures.java_parser._grammar_8"] = "src/coding_adventures/java_parser/_grammar_8.lua",
        ["coding_adventures.java_parser._grammar_10"] = "src/coding_adventures/java_parser/_grammar_10.lua",
        ["coding_adventures.java_parser._grammar_14"] = "src/coding_adventures/java_parser/_grammar_14.lua",
        ["coding_adventures.java_parser._grammar_17"] = "src/coding_adventures/java_parser/_grammar_17.lua",
        ["coding_adventures.java_parser._grammar_21"] = "src/coding_adventures/java_parser/_grammar_21.lua",
    },
}
