-- javascript_parser -- Builds an AST from JavaScript text using the grammar-driven engine
-- ========================================================================================
--
-- This package is part of the coding-adventures monorepo, a ground-up
-- implementation of the computing stack from transistors to operating systems.
-- It sits in the language-tooling layer alongside sql_parser, json_parser, and
-- toml_parser, above the lexer, grammar_tools, and javascript_lexer packages.
--
-- # What does a JavaScript parser do?
--
-- A lexer breaks raw JavaScript source into a flat token stream:
--
--   'var x = 5;'
--   →  KEYWORD("var") NAME("x") EQUALS("=") NUMBER("5") SEMICOLON(";") EOF
--
-- A parser takes that flat stream and builds a tree that captures the
-- *structure* of the program:
--
--   program
--   └── statement
--       └── var_declaration
--           ├── KEYWORD  "var"
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
-- # JavaScript grammar
--
-- The JavaScript grammar is defined in `code/grammars/javascript.grammar`.
-- The grammar covers a focused subset:
--
--   program        = { statement } ;
--   statement      = var_declaration | assignment | expression_stmt ;
--   var_declaration = KEYWORD NAME EQUALS expression SEMICOLON ;
--   assignment     = NAME EQUALS expression SEMICOLON ;
--   expression_stmt = expression SEMICOLON ;
--   expression     = term { ( PLUS | MINUS ) term } ;
--   term           = factor { ( STAR | SLASH ) factor } ;
--   factor         = NUMBER | STRING | NAME | KEYWORD | LPAREN expression RPAREN ;
--
-- This grammar handles:
--   - Variable declarations:  var x = 5;  let y = "hello";  const z = true;
--   - Assignments:            x = 10;
--   - Arithmetic expressions: 1 + 2 * 3  (respects precedence via term/factor)
--   - Parenthesized groups:   (a + b) * c
--   - Expression statements:  f(x);   (as NAME LPAREN … RPAREN — lexed as NAME)
--
-- # Architecture
--
-- 1. **Tokenize** — call `javascript_lexer.tokenize(source)` to get a token list.
-- 2. **Load grammar** — call `grammar_tools.parse_parser_grammar(content)`
--    to get a `ParserGrammar` with `.rules`.
-- 3. **Parse** — construct a `GrammarParser` (from the `parser` package)
--    and call `:parse()`.  The engine interprets the grammar rules against
--    the token stream, producing an AST.
--
-- # GrammarParser and ASTNode
--
-- `GrammarParser.new(tokens, grammar)` returns a parser instance.
-- Calling `:parse()` returns either:
--   (node, nil)    — success; `node` is the root ASTNode
--   (nil, errmsg)  — failure; `errmsg` is a human-readable error string
--
-- ASTNode fields:
--   node.rule_name  — which grammar rule produced this node ("program", …)
--   node.children   — array of child ASTNodes and/or Token tables
--   node:is_leaf()  — true when the node wraps exactly one token
--   node:token()    — the wrapped token (only valid when is_leaf() is true)
--
-- # Operator precedence
--
-- The grammar encodes JavaScript operator precedence through rule layering:
--
--   expression  → handles + and - (lowest precedence)
--   term        → handles * and / (higher precedence)
--   factor      → literals, names, parenthesized expressions (highest)
--
-- This means "1 + 2 * 3" correctly parses as "1 + (2 * 3)":
--
--   expression
--   ├── term → factor → NUMBER "1"
--   ├── PLUS "+"
--   └── term
--       ├── factor → NUMBER "2"
--       ├── STAR "*"
--       └── factor → NUMBER "3"
--
-- # Path navigation
--
-- This file lives at:
--   code/packages/lua/javascript_parser/src/coding_adventures/javascript_parser/init.lua
--
-- `debug.getinfo(1, "S").source` gives the absolute path (prefixed with "@").
-- Stripping the prefix and walking up 6 levels reaches `code/`, the repo root.
--
-- Directory structure from script_dir upward:
--   javascript_parser/  (1)
--   coding_adventures/  (2)
--   src/                (3)
--   javascript_parser/  (4) — the package directory
--   lua/                (5)
--   packages/           (6)
--   code/               → then /grammars/javascript.grammar

local grammar_tools    = require("coding_adventures.grammar_tools")
local javascript_lexer = require("coding_adventures.javascript_lexer")
local parser_pkg       = require("coding_adventures.parser")

local M = {}
M.VERSION = "0.2.0"

-- =========================================================================
-- Valid ECMAScript / JavaScript versions
-- =========================================================================
--
-- Each version maps to grammar files under code/grammars/ecmascript/.
--
-- When version is nil or "" → loads code/grammars/javascript.grammar (generic)
-- When version is "es2015"  → loads code/grammars/ecmascript/es2015.grammar
--
-- Recognized versions: es1, es3, es5, es2015..es2025

local VALID_JS_VERSIONS = {
    ["es1"]    = true, ["es3"]    = true, ["es5"]    = true,
    ["es2015"] = true, ["es2016"] = true, ["es2017"] = true,
    ["es2018"] = true, ["es2019"] = true, ["es2020"] = true,
    ["es2021"] = true, ["es2022"] = true, ["es2023"] = true,
    ["es2024"] = true, ["es2025"] = true,
}

-- =========================================================================
-- Path helpers
-- =========================================================================
--
-- These helpers mirror the pattern used by json_parser, toml_parser, and
-- sql_parser.  We navigate up 6 levels to reach `code/`, then descend
-- into `grammars/javascript.grammar`.

--- Return the directory portion of a file path (no trailing slash).
-- Example:  "/a/b/c/init.lua"  →  "/a/b/c"
-- @param path string
-- @return string
local function dirname(path)
    return path:match("(.+)/[^/]+$") or "."
end

--- Return the absolute directory of this source file.
-- Lua prepends "@" to the source path in debug info — we strip it.
-- When busted runs tests with a relative path containing ".." the
-- dirname-only approach produces a path that collapses to "." after
-- up() steps, so the grammar file cannot be found.  We resolve to an
-- absolute path via "cd <dir> && pwd" to give up() an absolute anchor.
-- @return string Absolute directory of this init.lua file.
local function get_script_dir()
    local info = debug.getinfo(1, "S")
    local src  = info.source
    if src:sub(1, 1) == "@" then
        src = src:sub(2)
    end
    -- Normalize Windows backslashes to forward slashes for cross-platform
    -- path handling (on Linux/macOS this is a no-op).
    src = src:gsub("\\", "/")
    local dir = src:match("(.+)/[^/]+$") or "."
    return dir
end

--- Walk up `levels` directory levels from `path`.
-- @param path   string  Starting directory.
-- @param levels number  How many levels to climb.
-- @return string
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
--
-- The parser grammar is loaded from disk once and cached.  Repeated calls
-- to `parse()` or `create_parser()` reuse the cached grammar, avoiding
-- repeated file I/O and repeated rule compilation.

-- Cache keyed by version string (or "" for generic).
local _grammar_cache = {}

--- Resolve the path to the correct .grammar file for a given version.
--
-- @param version string|nil  ECMAScript version tag, or nil/"" for generic.
-- @return string             Absolute path to the parser grammar file.
local function resolve_grammar_path(version)
    local script_dir = get_script_dir()
    local repo_root  = up(script_dir, 6)

    if not version or version == "" then
        return repo_root .. "/grammars/javascript.grammar"
    end

    if not VALID_JS_VERSIONS[version] then
        error(
            "javascript_parser: unknown ECMAScript version '" .. version .. "'. " ..
            "Valid values are: es1, es3, es5, es2015..es2025, or nil/\"\" for generic."
        )
    end

    return repo_root .. "/grammars/ecmascript/" .. version .. ".grammar"
end

--- Load and parse the grammar for a specific version, with per-version caching.
--
-- @param version string|nil  ECMAScript version tag (see resolve_grammar_path).
-- @return ParserGrammar      The parsed JavaScript parser grammar.
-- @error                     Raises an error if the file cannot be opened or parsed.
local function get_grammar(version)
    local key = version or ""
    if _grammar_cache[key] then
        return _grammar_cache[key]
    end

    local grammar_path = resolve_grammar_path(version)

    local f, open_err = io.open(grammar_path, "r")
    if not f then
        error(
            "javascript_parser: cannot open grammar file: " .. grammar_path ..
            " (" .. (open_err or "unknown error") .. ")"
        )
    end
    local content = f:read("*all")
    f:close()

    local grammar, parse_err = grammar_tools.parse_parser_grammar(content)
    if not grammar then
        error(
            "javascript_parser: failed to parse grammar file: " ..
            (parse_err or "unknown error")
        )
    end

    _grammar_cache[key] = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Parse a JavaScript source string and return the root ASTNode.
--
-- Internally:
--   1. Tokenizes `source` using `javascript_lexer.tokenize`.
--   2. Loads the JavaScript parser grammar (cached after the first call).
--   3. Runs the grammar-driven `GrammarParser` on the token stream.
--   4. Returns the AST root on success, or raises an error on failure.
--
-- The root node will have `rule_name == "program"` (the first rule in the
-- JavaScript grammar).
--
-- The grammar supports:
--   - var/let/const declarations: `var x = 5;`
--   - Assignments: `x = 10;`
--   - Arithmetic: `1 + 2 * 3` (correct precedence via term/factor layering)
--   - Parenthesized expressions: `(a + b) * c`
--   - Expression statements
--
-- @param source  string       The JavaScript text to parse.
-- @param version string|nil   ECMAScript version: "es1", "es3", "es5",
--                             "es2015".."es2025", or nil/"" for generic.
-- @return ASTNode             Root of the AST.
-- @error                      Raises an error on lexer or parser failure.
--
-- Example (generic):
--
--   local javascript_parser = require("coding_adventures.javascript_parser")
--   local ast = javascript_parser.parse("var x = 5;")
--   -- ast.rule_name  → "program"
--
-- Example (versioned):
--
--   local ast = javascript_parser.parse("var x = 5;", "es2015")
function M.parse(source, version)
    local tokens = javascript_lexer.tokenize(source, version)
    local grammar = get_grammar(version)
    local gp = parser_pkg.GrammarParser.new(tokens, grammar)
    local ast, err = gp:parse()
    if not ast then
        error("javascript_parser: " .. (err or "parse failed"))
    end
    return ast
end

--- Create a GrammarParser for a JavaScript source string without immediately parsing.
--
-- Use this when you want to control parsing yourself — for example, to
-- use trace mode or to inspect the token stream before parsing.
--
-- @param source  string       The JavaScript text to tokenize.
-- @param version string|nil   ECMAScript version tag (see parse for valid values).
-- @return GrammarParser       An initialized parser, ready to call `:parse()`.
--
-- Example:
--
--   local p = javascript_parser.create_parser("var x = 1;", "es5")
--   local ast, err = p:parse()
function M.create_parser(source, version)
    local tokens = javascript_lexer.tokenize(source, version)
    local grammar = get_grammar(version)
    return parser_pkg.GrammarParser.new(tokens, grammar)
end

--- Return the cached (or freshly loaded) ParserGrammar for JavaScript.
--
-- Exposed so callers can inspect the grammar rules directly — for example,
-- to enumerate rule names or check the grammar structure.
--
-- @param version string|nil  ECMAScript version tag (see parse for valid values).
-- @return ParserGrammar      The parsed JavaScript parser grammar.
function M.get_grammar(version)
    return get_grammar(version)
end

return M
