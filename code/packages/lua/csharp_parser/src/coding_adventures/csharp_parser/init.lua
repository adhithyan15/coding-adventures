-- csharp_parser — Builds an AST from C# text using the grammar-driven engine
-- ============================================================================
--
-- This package is part of the coding-adventures monorepo, a ground-up
-- implementation of the computing stack from transistors to operating systems.
-- It sits in the language-tooling layer alongside sql_parser, json_parser, and
-- java_parser, above the lexer, grammar_tools, and csharp_lexer packages.
--
-- # What does a C# parser do?
--
-- A lexer breaks raw C# source into a flat token stream:
--
--   'int x = 5;'
--   →  INT("int") NAME("x") EQUALS("=") NUMBER("5") SEMICOLON(";") EOF
--
-- A parser takes that flat stream and builds a tree that captures the
-- *structure* of the program:
--
--   program
--   └── statement
--       └── var_declaration
--           ├── INT      "int"
--           ├── NAME     "x"
--           ├── EQUALS   "="
--           ├── expression
--           │   └── term
--           │       └── factor
--           │           └── NUMBER  "5"
--           └── SEMICOLON  ";"
--
-- This tree is called an Abstract Syntax Tree (AST). Downstream tools
-- (evaluators, transpilers, linters, IDE analyzers) walk the AST rather
-- than re-parsing the source every time.
--
-- # C# grammar
--
-- The C# grammar covers a focused subset of C# that is valid across all
-- versions:
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
-- 1. **Tokenize** — call `csharp_lexer.tokenize_csharp(source, version)` to
--    get the flat token stream.
-- 2. **Load grammar** — select the precompiled `ParserGrammar` module for
--    the requested version (see "Why precompiled grammars?" below) and call
--    its `parser_grammar()` constructor.
-- 3. **Parse** — construct a `GrammarParser` (from the `parser` package)
--    and call `:parse()`.
--
-- # Why precompiled grammars?
--
-- Earlier versions of this module read `code/grammars/csharp/csharp<version>.grammar`
-- off disk at runtime, using a path that walked outside this package's own
-- directory into the monorepo. That works when running from a checkout of
-- the monorepo, but a published LuaRocks package does not include
-- `code/grammars/` — installing this rock and calling `parse_csharp` would
-- raise a file-not-found error.
--
-- Instead, each supported C# version's grammar is compiled ahead of time
-- (via `grammar-tools compile-grammar`) into a `_grammar_<version>.lua`
-- sibling module that embeds the parsed `ParserGrammar` as native Lua data.
-- Those modules ship with this package like any other source file, so the
-- rock is self-contained.
--
-- # Operator precedence
--
-- The grammar encodes C# operator precedence through rule layering:
--
--   expression  → handles + and - (lowest precedence)
--   term        → handles * and / (higher precedence)
--   factor      → literals, names, parenthesized expressions (highest)
--
-- This means `1 + 2 * 3` naturally parses as `1 + (2 * 3)` — the `*` binds
-- tighter because `term` nests inside `expression`.

local csharp_lexer   = require("coding_adventures.csharp_lexer")
local parser_pkg     = require("coding_adventures.parser")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Valid C# versions
-- =========================================================================
--
-- Must mirror the set in csharp_lexer — if the lexer can't tokenize a
-- version, the parser can't parse it either.

local VALID_CSHARP_VERSIONS = {
    ["1.0"]  = true, ["2.0"]  = true, ["3.0"]  = true,
    ["4.0"]  = true, ["5.0"]  = true, ["6.0"]  = true,
    ["7.0"]  = true, ["8.0"]  = true, ["9.0"]  = true,
    ["10.0"] = true, ["11.0"] = true, ["12.0"] = true,
}

local DEFAULT_VERSION = "12.0"

-- =========================================================================
-- Precompiled grammars
-- =========================================================================
--
-- Each entry maps a version string to the `require`d module produced by
-- `grammar-tools compile-grammar code/grammars/csharp/csharp<version>.grammar`.
-- The module exposes a `parser_grammar()` constructor; we call it lazily
-- (and cache the result) so grammar construction only happens for versions
-- actually used.

local compiled_grammars = {
    ["1.0"]  = require("coding_adventures.csharp_parser._grammar_1_0"),
    ["2.0"]  = require("coding_adventures.csharp_parser._grammar_2_0"),
    ["3.0"]  = require("coding_adventures.csharp_parser._grammar_3_0"),
    ["4.0"]  = require("coding_adventures.csharp_parser._grammar_4_0"),
    ["5.0"]  = require("coding_adventures.csharp_parser._grammar_5_0"),
    ["6.0"]  = require("coding_adventures.csharp_parser._grammar_6_0"),
    ["7.0"]  = require("coding_adventures.csharp_parser._grammar_7_0"),
    ["8.0"]  = require("coding_adventures.csharp_parser._grammar_8_0"),
    ["9.0"]  = require("coding_adventures.csharp_parser._grammar_9_0"),
    ["10.0"] = require("coding_adventures.csharp_parser._grammar_10_0"),
    ["11.0"] = require("coding_adventures.csharp_parser._grammar_11_0"),
    ["12.0"] = require("coding_adventures.csharp_parser._grammar_12_0"),
}

local _grammar_cache = {}

--- Resolve a version string (or nil/"") to a valid, known version.
--
-- @param version string|nil  The C# version tag, or nil/empty for default (12.0).
-- @return string             A key present in VALID_CSHARP_VERSIONS.
local function resolve_version(version)
    if not version or version == "" then
        version = DEFAULT_VERSION
    end

    if not VALID_CSHARP_VERSIONS[version] then
        error(
            "csharp_parser: unknown C# version '" .. version .. "'. " ..
            "Valid values are: 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, " ..
            "9.0, 10.0, 11.0, 12.0, or nil/\"\" for default (12.0)."
        )
    end

    return version
end

--- Build (or return the cached) `ParserGrammar` for a specific version.
--
-- @param version string|nil  The C# version tag (see resolve_version).
-- @return ParserGrammar      The C# parser grammar.
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

--- Parse a C# source string and return the root ASTNode.
--
-- @param source  string       The C# text to parse.
-- @param version string|nil   C# version: "1.0", "2.0", "3.0", "4.0", "5.0",
--                             "6.0", "7.0", "8.0", "9.0", "10.0", "11.0",
--                             "12.0", or nil/"" for default (12.0).
-- @return ASTNode             Root of the AST (rule_name == "program").
-- @error                      Raises an error on lexer or parser failure.
--
-- Example (default):
--
--   local csharp_parser = require("coding_adventures.csharp_parser")
--   local ast = csharp_parser.parse_csharp("int x = 1 + 2;")
--   print(ast.rule_name)  -- "program"
--
-- Example (versioned):
--
--   local ast = csharp_parser.parse_csharp("int x = 1;", "8.0")
function M.parse_csharp(source, version)
    local tokens = csharp_lexer.tokenize_csharp(source, version)
    local grammar = get_grammar(version)
    local gp = parser_pkg.GrammarParser.new(tokens, grammar)
    local ast, err = gp:parse()
    if not ast then
        error("csharp_parser: " .. (err or "parse failed"))
    end
    return ast
end

--- Create a GrammarParser for a C# source string without immediately parsing.
--
-- This is useful when you need the parser object directly — for example,
-- to drive parsing incrementally, to inspect the grammar, or to compose the
-- parser with other tools.
--
-- @param source  string       The C# text to tokenize.
-- @param version string|nil   C# version tag (see parse_csharp for valid values).
-- @return GrammarParser       An initialized parser, ready to call `:parse()`.
function M.create_csharp_parser(source, version)
    local tokens = csharp_lexer.tokenize_csharp(source, version)
    local grammar = get_grammar(version)
    return parser_pkg.GrammarParser.new(tokens, grammar)
end

--- Return the cached (or freshly built) ParserGrammar for C#.
--
-- Useful for inspecting the production rules, verifying the start rule,
-- or passing the grammar directly to other infrastructure components.
--
-- @param version string|nil  C# version tag (see parse_csharp for valid values).
-- @return ParserGrammar      The C# parser grammar.
function M.get_grammar(version)
    return get_grammar(version)
end

return M
