-- java_parser -- Builds an AST from Java text using the grammar-driven engine
-- ============================================================================
--
-- This package is part of the coding-adventures monorepo, a ground-up
-- implementation of the computing stack from transistors to operating systems.
-- It sits in the language-tooling layer alongside sql_parser, json_parser, and
-- toml_parser, above the lexer, grammar_tools, and java_lexer packages.
--
-- # What does a Java parser do?
--
-- A lexer breaks raw Java source into a flat token stream:
--
--   'int x = 5;'
--   →  NAME("int") NAME("x") EQUALS("=") NUMBER("5") SEMICOLON(";") EOF
--
-- A parser takes that flat stream and builds a tree that captures the
-- *structure* of the program:
--
--   program
--   └── statement
--       └── var_declaration
--           ├── NAME     "int"
--           ├── NAME     "x"
--           ├── EQUALS   "="
--           ├── expression
--           │   └── term
--           │       └── factor
--           │           └── NUMBER  "5"
--           └── SEMICOLON  ";"
--
-- This tree is called an Abstract Syntax Tree (AST). Downstream tools
-- (evaluators, transpilers, linters) walk the AST rather than re-parsing.
--
-- # Java grammar
--
-- The Java grammar covers a focused subset:
--
--   program        = { statement } ;
--   statement      = var_declaration | assignment | expression_stmt ;
--   var_declaration = NAME NAME EQUALS expression SEMICOLON ;
--   assignment     = NAME EQUALS expression SEMICOLON ;
--   expression_stmt = expression SEMICOLON ;
--   expression     = term { ( PLUS | MINUS ) term } ;
--   term           = factor { ( STAR | SLASH ) factor } ;
--   factor         = NUMBER | STRING | NAME | LPAREN expression RPAREN ;
--
-- # Architecture
--
-- 1. **Tokenize** — call `java_lexer.tokenize(source, version)` to get tokens.
-- 2. **Load grammar** — select the precompiled `ParserGrammar` module for
--    the requested version (see "Why precompiled grammars?" below) and call
--    its `parser_grammar()` constructor.
-- 3. **Parse** — construct a `GrammarParser` (from the `parser` package)
--    and call `:parse()`.
--
-- # Why precompiled grammars?
--
-- Earlier versions of this module read `code/grammars/java/java<version>.grammar`
-- off disk at runtime, using a path that walked outside this package's own
-- directory into the monorepo. That works when running from a checkout of
-- the monorepo, but a published LuaRocks package does not include
-- `code/grammars/` — installing this rock and calling `parse` would raise a
-- file-not-found error.
--
-- Instead, each supported Java version's grammar is compiled ahead of time
-- (via `grammar-tools compile-grammar`) into a `_grammar_<version>.lua`
-- sibling module that embeds the parsed `ParserGrammar` as native Lua data.
-- Those modules ship with this package like any other source file, so the
-- rock is self-contained.
--
-- # Operator precedence
--
-- The grammar encodes Java operator precedence through rule layering:
--
--   expression  → handles + and - (lowest precedence)
--   term        → handles * and / (higher precedence)
--   factor      → literals, names, parenthesized expressions (highest)

local java_lexer    = require("coding_adventures.java_lexer")
local parser_pkg    = require("coding_adventures.parser")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Valid Java versions
-- =========================================================================

local VALID_JAVA_VERSIONS = {
    ["1.0"] = true, ["1.1"] = true, ["1.4"] = true,
    ["5"]   = true, ["7"]   = true, ["8"]   = true,
    ["10"]  = true, ["14"]  = true, ["17"]  = true,
    ["21"]  = true,
}

local DEFAULT_VERSION = "21"

-- =========================================================================
-- Precompiled grammars
-- =========================================================================
--
-- Each entry maps a version string to the `require`d module produced by
-- `grammar-tools compile-grammar code/grammars/java/java<version>.grammar`.
-- The module exposes a `parser_grammar()` constructor; we call it lazily
-- (and cache the result) so grammar construction only happens for versions
-- actually used.

local compiled_grammars = {
    ["1.0"] = require("coding_adventures.java_parser._grammar_1_0"),
    ["1.1"] = require("coding_adventures.java_parser._grammar_1_1"),
    ["1.4"] = require("coding_adventures.java_parser._grammar_1_4"),
    ["5"]   = require("coding_adventures.java_parser._grammar_5"),
    ["7"]   = require("coding_adventures.java_parser._grammar_7"),
    ["8"]   = require("coding_adventures.java_parser._grammar_8"),
    ["10"]  = require("coding_adventures.java_parser._grammar_10"),
    ["14"]  = require("coding_adventures.java_parser._grammar_14"),
    ["17"]  = require("coding_adventures.java_parser._grammar_17"),
    ["21"]  = require("coding_adventures.java_parser._grammar_21"),
}

local _grammar_cache = {}

--- Resolve a version string (or nil/"") to a valid, known version.
--
-- @param version string|nil  The Java version tag, or nil/empty for default (21).
-- @return string             A key present in VALID_JAVA_VERSIONS.
local function resolve_version(version)
    if not version or version == "" then
        version = DEFAULT_VERSION
    end

    if not VALID_JAVA_VERSIONS[version] then
        error(
            "java_parser: unknown Java version '" .. version .. "'. " ..
            "Valid values are: 1.0, 1.1, 1.4, 5, 7, 8, 10, 14, 17, 21, or nil/\"\" for default (21)."
        )
    end

    return version
end

--- Build (or return the cached) `ParserGrammar` for a specific version.
--
-- @param version string|nil  The Java version tag (see resolve_version).
-- @return ParserGrammar      The Java parser grammar.
local function get_grammar(version)
    local key = resolve_version(version)
    if _grammar_cache[key] == nil then
        _grammar_cache[key] = compiled_grammars[key].parser_grammar()
    end
    return _grammar_cache[key]
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Parse a Java source string and return the root ASTNode.
--
-- @param source  string       The Java text to parse.
-- @param version string|nil   Java version: "1.0", "1.1", "1.4", "5", "7",
--                             "8", "10", "14", "17", "21", or nil/"" for
--                             default (21).
-- @return ASTNode             Root of the AST.
-- @error                      Raises an error on lexer or parser failure.
function M.parse(source, version)
    local tokens = java_lexer.tokenize(source, version)
    local grammar = get_grammar(version)
    local gp = parser_pkg.GrammarParser.new(tokens, grammar)
    local ast, err = gp:parse()
    if not ast then
        error("java_parser: " .. (err or "parse failed"))
    end
    return ast
end

--- Create a GrammarParser for a Java source string without immediately parsing.
--
-- @param source  string       The Java text to tokenize.
-- @param version string|nil   Java version tag (see parse for valid values).
-- @return GrammarParser       An initialized parser, ready to call `:parse()`.
function M.create_parser(source, version)
    local tokens = java_lexer.tokenize(source, version)
    local grammar = get_grammar(version)
    return parser_pkg.GrammarParser.new(tokens, grammar)
end

--- Return the cached (or freshly built) ParserGrammar for Java.
--
-- @param version string|nil  Java version tag (see parse for valid values).
-- @return ParserGrammar      The Java parser grammar.
function M.get_grammar(version)
    return get_grammar(version)
end

return M
