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
-- The Java grammar is defined in `code/grammars/java/java<version>.grammar`.
-- The grammar covers a focused subset:
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
-- 2. **Load grammar** — call `grammar_tools.parse_parser_grammar(content)`
--    to get a `ParserGrammar` with `.rules`.
-- 3. **Parse** — construct a `GrammarParser` (from the `parser` package)
--    and call `:parse()`.
--
-- # Operator precedence
--
-- The grammar encodes Java operator precedence through rule layering:
--
--   expression  → handles + and - (lowest precedence)
--   term        → handles * and / (higher precedence)
--   factor      → literals, names, parenthesized expressions (highest)
--
-- # Path navigation
--
-- This file lives at:
--   code/packages/lua/java_parser/src/coding_adventures/java_parser/init.lua
--
-- Walking up 6 levels reaches `code/`, the repo root.

local grammar_tools = require("coding_adventures.grammar_tools")
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
-- Path helpers
-- =========================================================================

local function dirname(path)
    return path:match("(.+)/[^/]+$") or "."
end

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
local function resolve_grammar_path(version)
    local script_dir = get_script_dir()
    local repo_root  = up(script_dir, 6)

    if not version or version == "" then
        version = DEFAULT_VERSION
    end

    if not VALID_JAVA_VERSIONS[version] then
        error(
            "java_parser: unknown Java version '" .. version .. "'. " ..
            "Valid values are: 1.0, 1.1, 1.4, 5, 7, 8, 10, 14, 17, 21, or nil/\"\" for default (21)."
        )
    end

    return repo_root .. "/grammars/java/java" .. version .. ".grammar"
end

--- Load and parse the grammar for a specific version, with per-version caching.
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
            "java_parser: cannot open grammar file: " .. grammar_path ..
            " (" .. (open_err or "unknown error") .. ")"
        )
    end
    local content = f:read("*all")
    f:close()

    local grammar, parse_err = grammar_tools.parse_parser_grammar(content)
    if not grammar then
        error(
            "java_parser: failed to parse grammar file: " ..
            (parse_err or "unknown error")
        )
    end

    _grammar_cache[key] = grammar
    return grammar
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

--- Return the cached (or freshly loaded) ParserGrammar for Java.
--
-- @param version string|nil  Java version tag (see parse for valid values).
-- @return ParserGrammar      The parsed Java parser grammar.
function M.get_grammar(version)
    return get_grammar(version)
end

return M
