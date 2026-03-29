-- sql_lexer -- Tokenizes SQL text using the grammar-driven infrastructure
-- =========================================================================
--
-- This package is part of the coding-adventures monorepo. It is a thin
-- wrapper around the grammar-driven `GrammarLexer` from the `lexer` package,
-- loading the `sql.tokens` grammar file to configure the tokenizer.
--
-- # What is SQL tokenization?
--
-- Given the input:  SELECT * FROM users WHERE id = 1
--
-- The lexer produces a flat stream of typed tokens:
--
--   Token(SELECT,  "SELECT",  1:1)
--   Token(STAR,    "*",       1:8)
--   Token(FROM,    "FROM",    1:10)
--   Token(NAME,    "users",   1:15)
--   Token(WHERE,   "WHERE",   1:21)
--   Token(NAME,    "id",      1:27)
--   Token(EQUALS,  "=",       1:30)
--   Token(NUMBER,  "1",       1:32)
--   Token(EOF,     "",        1:33)
--
-- Whitespace and comments are silently consumed (the `sql.tokens` grammar
-- declares them as skip patterns). The parser never sees whitespace tokens.
--
-- # SQL-specific lexer concerns
--
-- **Case-insensitive keywords** — The `sql.tokens` grammar has
-- `@case_insensitive true`, meaning keyword literals like "SELECT" match
-- `select`, `SELECT`, or `Select`. The grammar tools infrastructure handles
-- case folding when building the GrammarLexer.
--
-- **Keywords vs identifiers** — The grammar lists keywords (SELECT, FROM,
-- WHERE, etc.) that must match before the generic `NAME` pattern. The
-- GrammarLexer tries definitions in order, so keywords take priority.
--
-- **Operator ordering** — Longer operators must come before shorter ones:
--   `<=` before `<`,  `>=` before `>`,  `!=` before nothing.
-- The grammar handles this via ordering.
--
-- **NEQ_ANSI alias** — `<>` (ANSI SQL inequality) is aliased to NOT_EQUALS
-- so a parser only needs to handle one token type for both `!=` and `<>`.
--
-- **STRING alias** — `STRING_SQ` (single-quoted) and `QUOTED_ID` (backtick-
-- quoted identifier) are aliased to STRING and NAME respectively so the
-- grammar can reference a single type.
--
-- # Architecture
--
-- This module:
--   1. Locates the shared `sql.tokens` grammar file in `code/grammars/`.
--   2. Reads and parses it once (cached) using `grammar_tools.parse_token_grammar`.
--   3. Constructs a `GrammarLexer` from the `lexer` package for each call.
--   4. Returns the flat token list.
--
-- # Path navigation
--
-- The source file lives at:
--   code/packages/lua/sql_lexer/src/coding_adventures/sql_lexer/init.lua
--
-- `debug.getinfo(1, "S").source` gives the absolute path to this file.
-- We strip the leading `@` Lua adds to source paths, then walk up 6
-- directory levels to reach the repo root (`code/`), then descend into
-- `grammars/sql.tokens`.
--
-- Directory structure from script_dir upward:
--   sql_lexer/           (1) — coding_adventures/sql_lexer/
--   coding_adventures/   (2)
--   src/                 (3)
--   sql_lexer/           (4) — the package directory
--   lua/                 (5)
--   packages/            (6)
--   code/                → then /grammars/sql.tokens

local grammar_tools = require("coding_adventures.grammar_tools")
local lexer_pkg     = require("coding_adventures.lexer")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Path helpers
-- =========================================================================

--- Return the directory portion of a file path (without trailing slash).
-- For example:  "/a/b/c/init.lua"  →  "/a/b/c"
-- @param path string The full file path.
-- @return string     The directory portion.
local function dirname(path)
    return path:match("(.+)/[^/]+$") or "."
end

--- Return the absolute directory of this source file.
-- Lua embeds the source path in the chunk debug info with a leading "@".
-- We strip that prefix to get the real filesystem path.
-- @return string Absolute directory of this init.lua file.
local function get_script_dir()
    local info = debug.getinfo(1, "S")
    local src  = info.source
    if src:sub(1, 1) == "@" then
        src = src:sub(2)
    end
    -- Extract the directory portion of the source path (may be relative
    -- and may contain .. when busted uses ../src in package.path).
    local dir = src:match("(.+)/[^/]+$") or "."
    -- Resolve to an absolute normalised path. Using 'cd dir && pwd' correctly
    -- resolves any .. components -- unlike string-based dirname traversal.
    -- Skip on Windows drive paths (C:\...) and fall back to the raw string.
    if dir:sub(2, 2) ~= ":" then
        local f = io.popen("cd '" .. dir .. "' 2>/dev/null && pwd")
        local resolved = f and f:read("*l")
        if f then f:close() end
        if resolved and resolved ~= "" then
            return resolved
        end
    end
    return dir
end

--- Walk up `levels` directory levels from `path`.
-- Each call to this function strips one path component.
-- For example: up("/a/b/c", 2) → "/a"
-- @param path   string  Starting directory.
-- @param levels number  How many levels to climb.
-- @return string        Resulting directory.
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
-- The grammar is read from disk exactly once and cached in a module-level
-- variable. Subsequent calls to `tokenize` reuse the cached grammar.
-- This avoids repeated file I/O and repeated regex compilation.

local _grammar_cache = nil

--- Load and parse the `sql.tokens` grammar, with caching.
-- On the first call, opens and parses the file. On subsequent calls,
-- returns the cached TokenGrammar object immediately.
-- @return TokenGrammar  The parsed SQL token grammar.
local function get_grammar()
    if _grammar_cache then
        return _grammar_cache
    end

    -- Navigate from this file's directory up to the repo root.
    -- init.lua is 3 dirs inside the package (src/coding_adventures/sql_lexer/).
    -- The package itself is 3 more dirs inside the repo (packages/lua/sql_lexer/).
    -- Total: 6 levels up lands us at `code/`, the repo root.
    local script_dir  = get_script_dir()
    local repo_root   = up(script_dir, 6)
    local tokens_path = repo_root .. "/grammars/sql.tokens"

    local f, open_err = io.open(tokens_path, "r")
    if not f then
        error(
            "sql_lexer: cannot open grammar file: " .. tokens_path ..
            " (" .. (open_err or "unknown error") .. ")"
        )
    end
    local content = f:read("*all")
    f:close()

    local grammar, parse_err = grammar_tools.parse_token_grammar(content)
    if not grammar then
        error("sql_lexer: failed to parse sql.tokens: " .. (parse_err or "unknown error"))
    end

    _grammar_cache = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Tokenize a SQL source string.
--
-- Loads the `sql.tokens` grammar (cached after first call) and feeds the
-- source to a `GrammarLexer`. Returns the complete flat token list,
-- including a terminal `EOF` token.
--
-- Keywords are case-insensitive: "select", "SELECT", and "Select" all
-- produce a SELECT token. The token value preserves the original source
-- casing.
--
-- SQL token types produced:
--
--   NAME           — identifiers: column names, table names, aliases
--   NUMBER         — integer and decimal literals: 42, 3.14
--   STRING         — single-quoted string literals: 'hello'
--                    (aliased from STRING_SQ in grammar)
--   SELECT, FROM, WHERE, GROUP, BY, HAVING, ORDER, LIMIT, OFFSET
--   INSERT, INTO, VALUES, UPDATE, SET, DELETE
--   CREATE, DROP, TABLE, IF, EXISTS
--   NOT, AND, OR, NULL, IS, IN, BETWEEN, LIKE, AS, DISTINCT
--   ALL, UNION, INTERSECT, EXCEPT
--   JOIN, INNER, LEFT, RIGHT, OUTER, CROSS, FULL, ON
--   ASC, DESC, TRUE, FALSE
--   CASE, WHEN, THEN, ELSE, END
--   PRIMARY, KEY, UNIQUE, DEFAULT
--   LESS_EQUALS, GREATER_EQUALS, NOT_EQUALS  — <=, >=, != (and <>)
--   EQUALS, LESS_THAN, GREATER_THAN          — =, <, >
--   PLUS, MINUS, STAR, SLASH, PERCENT        — arithmetic operators
--   LPAREN, RPAREN, COMMA, SEMICOLON, DOT    — delimiters
--   EOF                                      — end of input
--
-- Whitespace, line comments (-- ...), and block comments (/* ... */) are
-- consumed silently via the skip patterns in `sql.tokens`.
--
-- @param source string  The SQL text to tokenize.
-- @return table         Array of Token objects (type, value, line, col).
-- @error                Raises an error on unexpected characters.
--
-- Example:
--
--   local sql_lexer = require("coding_adventures.sql_lexer")
--   local tokens = sql_lexer.tokenize("SELECT * FROM users")
--   -- tokens[1].type  → "SELECT"
--   -- tokens[1].value → "SELECT"
--   -- tokens[2].type  → "STAR"
--   -- tokens[3].type  → "FROM"
--   -- tokens[4].type  → "NAME"
--   -- tokens[4].value → "users"
function M.tokenize(source)
    local grammar = get_grammar()
    local gl      = lexer_pkg.GrammarLexer.new(source, grammar)
    return gl:tokenize()
end

--- Return the cached (or freshly loaded) TokenGrammar for SQL.
--
-- Exposed for callers that want to inspect or reuse the grammar object
-- directly — for example, to build a custom GrammarLexer with callbacks.
--
-- @return TokenGrammar  The parsed SQL token grammar.
function M.get_grammar()
    return get_grammar()
end

return M
