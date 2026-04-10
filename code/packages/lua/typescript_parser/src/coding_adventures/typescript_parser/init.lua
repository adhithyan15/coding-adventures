-- typescript_parser -- Builds an AST from TypeScript text using the grammar-driven engine
-- ==========================================================================================
--
-- This package is part of the coding-adventures monorepo, a ground-up
-- implementation of the computing stack from transistors to operating systems.
-- It sits in the language-tooling layer alongside javascript_parser, json_parser,
-- and sql_parser, above the lexer, grammar_tools, and typescript_lexer packages.
--
-- # What does a TypeScript parser do?
--
-- A lexer breaks raw TypeScript source into a flat token stream:
--
--   'let x: number = 5;'
--   →  KEYWORD("let") NAME("x") COLON(":") KEYWORD("number") EQUALS("=")
--      NUMBER("5") SEMICOLON(";") EOF
--
-- A parser takes that flat stream and builds a tree that captures the
-- *structure* of the program:
--
--   program
--   └── statement
--       └── var_declaration
--           ├── KEYWORD  "let"
--           ├── NAME     "x"
--           ├── EQUALS   "="
--           ├── expression
--           │   └── term
--           │       └── factor
--           │           └── NUMBER  "5"
--           └── SEMICOLON  ";"
--
-- This tree is called an Abstract Syntax Tree (AST). Downstream tools
-- (evaluators, transpilers, type-checkers) walk the AST rather than re-parsing.
--
-- # TypeScript grammar
--
-- The TypeScript grammar is defined in `code/grammars/typescript.grammar`.
-- For the subset we support, the grammar is identical to JavaScript's simple
-- grammar — the key difference is that the lexer uses typescript.tokens
-- which recognizes more keywords (interface, type, enum, abstract, etc.).
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
-- # TypeScript vs JavaScript
--
-- TypeScript is a strict superset of JavaScript. Every valid JavaScript
-- program is also valid TypeScript. The additions are:
--   - Type annotations:  `let x: number = 1`
--   - Interfaces:        `interface Foo { bar: string }`
--   - Generics:          `Array<number>`
--   - Access modifiers:  `public`, `private`, `protected`
--   - Enums:             `enum Color { Red, Green, Blue }`
--   - Type aliases:      `type Pair = [string, number]`
--
-- For the parser grammar subset we implement, TypeScript and JavaScript share
-- the same grammar rules. The TypeScript-specific keywords appear as KEYWORD
-- tokens during lexing, so they are handled by the existing grammar.
--
-- # Architecture
--
-- 1. **Tokenize** — call `typescript_lexer.tokenize(source)` to get a token list.
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
-- The grammar encodes operator precedence through rule layering:
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
--   code/packages/lua/typescript_parser/src/coding_adventures/typescript_parser/init.lua
--
-- `debug.getinfo(1, "S").source` gives the absolute path (prefixed with "@").
-- Stripping the prefix and walking up 6 levels reaches `code/`, the repo root.
--
-- Directory structure from script_dir upward:
--   typescript_parser/  (1)  ← this module's dir
--   coding_adventures/  (2)
--   src/                (3)
--   typescript_parser/  (4)  ← the package directory
--   lua/                (5)
--   packages/           (6)
--   code/               → then /grammars/typescript.grammar

local grammar_tools     = require("coding_adventures.grammar_tools")
local typescript_lexer  = require("coding_adventures.typescript_lexer")
local parser_pkg        = require("coding_adventures.parser")

local M = {}
M.VERSION = "0.2.0"

-- =========================================================================
-- Valid TypeScript versions
-- =========================================================================
--
-- Each version maps to a grammar file under code/grammars/typescript/.
-- The parser grammar (.grammar) and the token grammar (.tokens) are paired.
--
-- When version is nil or "" → loads code/grammars/typescript.grammar (generic)
-- When version is "ts5.0"   → loads code/grammars/typescript/ts5.0.grammar
--
-- Recognized versions: ts1.0, ts2.0, ts3.0, ts4.0, ts5.0, ts5.8

local VALID_TS_VERSIONS = {
    ["ts1.0"] = true, ["ts2.0"] = true, ["ts3.0"] = true,
    ["ts4.0"] = true, ["ts5.0"] = true, ["ts5.8"] = true,
}

-- =========================================================================
-- Path helpers
-- =========================================================================
--
-- These helpers mirror the pattern used by javascript_parser, json_parser,
-- and sql_parser.  We navigate up 6 levels to reach `code/`, then descend
-- into `grammars/typescript.grammar`.

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
    -- Security: Do not pass the dir string to io.popen (shell injection risk).
    -- Instead, use os.getenv to resolve relative paths -- no subprocess or
    -- shell invocation is involved. The previously removed pattern
    --   io.popen("cd '" .. dir .. "' 2>/dev/null && pwd")
    -- was unsafe because dir could contain shell metacharacters.
    -- Fixed: 2026-04-10 security review.
    if dir:sub(1, 1) ~= "/" and dir:sub(2, 2) ~= ":" then
        local cwd = os.getenv("PWD") or os.getenv("CD") or ""
        if cwd ~= "" then
            dir = cwd:gsub("\\\\", "/"):gsub("%c+$", "") .. "/" .. dir
            -- Normalise .. and . segments so dirname-based traversal works
            -- correctly when the source was loaded via a relative package.path
            -- entry (e.g. "../src/?.lua" from a tests/ subdirectory).
            local is_abs = dir:sub(1, 1) == "/"
            local parts = {}
            for seg in dir:gmatch("[^/]+") do
                if seg == ".." then
                    if #parts > 0 then table.remove(parts) end
                elseif seg ~= "." then
                    table.insert(parts, seg)
                end
            end
            dir = (is_abs and "/" or "") .. table.concat(parts, "/")
        end
    end
    return dir
end

--- Walk up `levels` directory levels from `path`.
-- @param path   string  Starting directory.
-- @param levels number  How many levels to climb.
-- @return string
local function up(path, levels)
    local result = path
    for _ = 1, levels do
        result = dirname(result)
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
-- @param version string|nil  TypeScript version tag, or nil/"" for generic.
-- @return string             Absolute path to the parser grammar file.
local function resolve_grammar_path(version)
    local script_dir = get_script_dir()
    local repo_root  = up(script_dir, 6)

    if not version or version == "" then
        return repo_root .. "/grammars/typescript.grammar"
    end

    if not VALID_TS_VERSIONS[version] then
        error(
            "typescript_parser: unknown TypeScript version '" .. version .. "'. " ..
            "Valid values are: ts1.0, ts2.0, ts3.0, ts4.0, ts5.0, ts5.8, or nil/\"\" for generic."
        )
    end

    return repo_root .. "/grammars/typescript/" .. version .. ".grammar"
end

--- Load and parse the grammar for a specific version, with per-version caching.
--
-- On the first call for a given version, opens the file, parses it with
-- `grammar_tools.parse_parser_grammar`, and stores the result in _grammar_cache.
-- On subsequent calls for the same version, returns the cached object immediately.
--
-- @param version string|nil  TypeScript version tag (see resolve_grammar_path).
-- @return ParserGrammar      The parsed TypeScript parser grammar.
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
            "typescript_parser: cannot open grammar file: " .. grammar_path ..
            " (" .. (open_err or "unknown error") .. ")"
        )
    end
    local content = f:read("*all")
    f:close()

    local grammar, parse_err = grammar_tools.parse_parser_grammar(content)
    if not grammar then
        error(
            "typescript_parser: failed to parse grammar file: " ..
            (parse_err or "unknown error")
        )
    end

    _grammar_cache[key] = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Parse a TypeScript source string and return the root ASTNode.
--
-- Internally:
--   1. Tokenizes `source` using `typescript_lexer.tokenize`.
--   2. Loads the TypeScript parser grammar (cached after the first call).
--   3. Runs the grammar-driven `GrammarParser` on the token stream.
--   4. Returns the AST root on success, or raises an error on failure.
--
-- The root node will have `rule_name == "program"` (the first rule in the
-- TypeScript grammar).
--
-- The grammar supports:
--   - var/let/const declarations: `let x = 5;`
--   - Assignments: `x = 10;`
--   - Arithmetic: `1 + 2 * 3` (correct precedence via term/factor layering)
--   - Parenthesized expressions: `(a + b) * c`
--   - Expression statements
--
-- @param source  string       The TypeScript text to parse.
-- @param version string|nil   TypeScript version: "ts1.0", "ts2.0", "ts3.0",
--                             "ts4.0", "ts5.0", "ts5.8", or nil/"" for generic.
-- @return ASTNode             Root of the AST.
-- @error                      Raises an error on lexer or parser failure.
--
-- Example (generic):
--
--   local typescript_parser = require("coding_adventures.typescript_parser")
--   local ast = typescript_parser.parse("let x = 5;")
--   -- ast.rule_name  → "program"
--
-- Example (versioned):
--
--   local ast = typescript_parser.parse("let x: number = 5;", "ts5.0")
function M.parse(source, version)
    local tokens = typescript_lexer.tokenize(source, version)
    local grammar = get_grammar(version)
    local gp = parser_pkg.GrammarParser.new(tokens, grammar)
    local ast, err = gp:parse()
    if not ast then
        error("typescript_parser: " .. (err or "parse failed"))
    end
    return ast
end

--- Create a GrammarParser for a TypeScript source string without immediately parsing.
--
-- Use this when you want to control parsing yourself — for example, to
-- use trace mode or to inspect the token stream before parsing.
--
-- @param source  string       The TypeScript text to tokenize.
-- @param version string|nil   TypeScript version tag (see parse for valid values).
-- @return GrammarParser       An initialized parser, ready to call `:parse()`.
--
-- Example:
--
--   local p = typescript_parser.create_parser("let x = 1;", "ts5.8")
--   local ast, err = p:parse()
function M.create_parser(source, version)
    local tokens = typescript_lexer.tokenize(source, version)
    local grammar = get_grammar(version)
    return parser_pkg.GrammarParser.new(tokens, grammar)
end

--- Return the cached (or freshly loaded) ParserGrammar for TypeScript.
--
-- Exposed so callers can inspect the grammar rules directly — for example,
-- to enumerate rule names or check the grammar structure.
--
-- @param version string|nil  TypeScript version tag (see parse for valid values).
-- @return ParserGrammar      The parsed TypeScript parser grammar.
function M.get_grammar(version)
    return get_grammar(version)
end

return M
