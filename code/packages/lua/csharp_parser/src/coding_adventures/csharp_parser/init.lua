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
-- The C# grammar is defined in `code/grammars/csharp/csharp<version>.grammar`.
-- The grammar covers a focused subset of C# that is valid across all versions:
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
-- 2. **Load grammar** — call `grammar_tools.parse_parser_grammar(content)`
--    to get a `ParserGrammar` with `.rules`.
-- 3. **Parse** — construct a `GrammarParser` (from the `parser` package)
--    and call `:parse()`.
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
--
-- # Path navigation
--
-- This file lives at:
--   code/packages/lua/csharp_parser/src/coding_adventures/csharp_parser/init.lua
--
-- Walking up 6 levels reaches `code/`, the repo root.
--
--   csharp_parser/       (1) — module dir
--   coding_adventures/   (2)
--   src/                 (3)
--   csharp_parser/       (4) — the package directory
--   lua/                 (5)
--   packages/            (6)
--   code/                → then /grammars/csharp/...

local grammar_tools  = require("coding_adventures.grammar_tools")
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
-- Path helpers
-- =========================================================================

local function get_script_dir()
    local info = debug.getinfo(1, "S")
    local src  = info.source
    if src:sub(1, 1) == "@" then
        src = src:sub(2)
    end
    src = src:gsub("\\", "/")
    local dir = src:match("(.+)/[^/]+$") or "."
    return dir
end

local function up(path, levels)
    local result = path
    for _ = 1, levels do
        result = result .. "/.."
    end
    return result
end

-- =========================================================================
-- Grammar loading
-- =========================================================================

local _grammar_cache = {}

--- Resolve the path to the correct .grammar file for a given version.
--
-- @param version string|nil  The C# version tag, or nil/empty for default (12.0).
-- @return string             Absolute path to the .grammar file.
local function resolve_grammar_path(version)
    local script_dir = get_script_dir()
    local repo_root  = up(script_dir, 6)

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

    return repo_root .. "/grammars/csharp/csharp" .. version .. ".grammar"
end

--- Load and parse the parser grammar for a specific version, with caching.
--
-- The parser grammar (`.grammar` file) defines the production rules used to
-- build the AST. It is distinct from the token grammar (`.tokens` file) used
-- by the lexer. We cache parsed grammars per-version to avoid re-reading and
-- re-parsing the file on every `parse_csharp()` call.
--
-- @param version string|nil  The C# version tag (see resolve_grammar_path).
-- @return ParserGrammar      The parsed C# parser grammar.
local function get_grammar(version)
    local key = version or DEFAULT_VERSION
    if key == "" then key = DEFAULT_VERSION end
    if _grammar_cache[key] then
        return _grammar_cache[key]
    end

    local grammar_path = resolve_grammar_path(version)

    local f, open_err = io.open(grammar_path, "r")
    if not f then
        error(
            "csharp_parser: cannot open grammar file: " .. grammar_path ..
            " (" .. (open_err or "unknown error") .. ")"
        )
    end
    local content = f:read("*all")
    f:close()

    local grammar, parse_err = grammar_tools.parse_parser_grammar(content)
    if not grammar then
        error(
            "csharp_parser: failed to parse grammar file: " ..
            (parse_err or "unknown error")
        )
    end

    _grammar_cache[key] = grammar
    return grammar
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

--- Return the cached (or freshly loaded) ParserGrammar for C#.
--
-- Useful for inspecting the production rules, verifying the start rule,
-- or passing the grammar directly to other infrastructure components.
--
-- @param version string|nil  C# version tag (see parse_csharp for valid values).
-- @return ParserGrammar      The parsed C# parser grammar.
function M.get_grammar(version)
    return get_grammar(version)
end

return M
